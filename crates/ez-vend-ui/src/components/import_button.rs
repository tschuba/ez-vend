use std::rc::Rc;

use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Event, File as WebFile, FileReader, ProgressEvent};

use crate::components::{
    use_toast, Button, ButtonSize, ButtonVariant, Icon, LuUpload, Modal, ModalSize,
};
use crate::error_logging::{current_route, stack_trace, use_error_logger, ErrorLogDraft};
use crate::selected_booth_context::use_booth_list_version;
use crate::state::use_app_state;
use crate::t;
use crate::utils::current_device_info;
use ez_vend_storage::export::{
    BoothCandidate, BoothResolution, ConflictStrategy, ImportAnalysis, ImportError, ImportPayload,
    ImportSummary, ImportValidator, ValidationFailure,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum ImportPreview {
    Full {
        booths: usize,
        vendors: usize,
        purchases: usize,
    },
    Booth {
        description: String,
        vendors: usize,
        purchases: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum AnalysisState {
    Pending,
    Available(ImportAnalysis),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
struct ImportCandidate {
    file_name: String,
    preview: Option<ImportPreview>,
    parsed_data: Option<ImportPayload>,
    validation_failures: Vec<ValidationFailure>,
    structure_error: Option<String>,
    analysis: AnalysisState,
}

impl ImportCandidate {
    fn is_ready(&self) -> bool {
        self.preview.is_some() && self.parsed_data.is_some()
    }
}

/// Operator's decision for one Ambiguous or Archived wizard step
#[derive(Clone, Debug, PartialEq)]
enum WizardChoice {
    UseCandidate(BoothId),
    #[allow(dead_code)]
    ImportAsNew,
    Skip,
    /// Merge canonical into other locally first, then import under canonical
    Advanced {
        canonical: BoothId,
        other: BoothId,
    },
}

use domain::models::BoothId;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportResultItem {
    file_name: String,
    success: bool,
    message: String,
}

#[component]
pub fn ImportButton(
    #[prop(optional)] variant: Option<ButtonVariant>,
    #[prop(optional)] size: Option<ButtonSize>,
    #[prop(optional)] class: Option<String>,
) -> impl IntoView {
    let app_state = use_app_state();
    let booth_list_version = use_booth_list_version();
    let toast = use_toast();
    let log_error = use_error_logger();
    let input_ref = create_node_ref::<html::Input>();
    let (is_reading, set_is_reading) = signal(false);
    let (is_importing, set_is_importing) = signal(false);
    let (show_modal, set_show_modal) = signal(false);
    let (selected_file_names, set_selected_file_names) = signal(Vec::<String>::new());
    let (candidates, set_candidates) = signal(Vec::<ImportCandidate>::new());
    let (conflict_strategy, set_conflict_strategy) = signal(ConflictStrategy::Merge);
    let (import_progress, set_import_progress) = signal(None::<(usize, usize)>);
    let (import_results, set_import_results) = signal(Vec::<ImportResultItem>::new());
    // Wizard state
    let (wizard_mode, set_wizard_mode) = signal(false);
    let (wizard_step, set_wizard_step) = signal(0usize);
    let (wizard_decisions, set_wizard_decisions) =
        signal(std::collections::HashMap::<String, WizardChoice>::new());
    let (archived_confirmed, set_archived_confirmed) = signal(false);

    let validator = Rc::new(ImportValidator::new());

    let open_file_picker = move || {
        if is_reading.get_untracked() || is_importing.get_untracked() {
            return;
        }

        if let Some(input) = input_ref.get() {
            input.set_value("");
            input.click();
        }
    };

    let reset_results = {
        move || {
            set_candidates.set(Vec::new());
            set_selected_file_names.set(Vec::new());
            set_import_progress.set(None);
            set_import_results.set(Vec::new());
        }
    };

    let log_error_for_file_change = log_error.clone();
    let on_file_change = {
        let validator = Rc::clone(&validator);
        move |_ev: Event| {
            let Some(input) = input_ref.get() else {
                return;
            };

            let Some(files) = input.files() else {
                return;
            };

            if files.length() == 0 {
                return;
            }

            reset_results();
            set_is_reading.set(true);

            let file_names = (0..files.length())
                .filter_map(|index| files.get(index))
                .map(|file| file.name())
                .collect::<Vec<_>>();
            set_selected_file_names.set(file_names);

            let candidates_signal = set_candidates;
            let show_modal_signal = set_show_modal;
            let reading_signal = set_is_reading;
            let log_error = log_error_for_file_change.clone();
            let validator_for_parse = Rc::clone(&validator);

            spawn_local(async move {
                let mut parsed_candidates = Vec::new();
                for index in 0..files.length() {
                    let Some(file) = files.get(index) else {
                        continue;
                    };

                    let candidate =
                        read_and_parse_file(file, Rc::clone(&validator_for_parse)).await;
                    parsed_candidates.push(candidate);
                }

                reading_signal.set(false);
                if parsed_candidates.is_empty() {
                    log_error(ErrorLogDraft {
                        error_type: "import_read_failed".to_string(),
                        error_message: t!("backup.import_failed")(),
                        stack_trace: stack_trace(),
                        user_action: Some("read import files".to_string()),
                        route: current_route(),
                        vendor_id: None,
                        purchase_id: None,
                        details: vec!["no import candidates produced".to_string()],
                    });
                    toast.error(t!("backup.import_failed")());
                    return;
                }

                // Run analysis on parseable candidates
                let analyzed = if let Some(Ok(state)) = app_state.get() {
                    let mut out = parsed_candidates.clone();
                    for candidate in &mut out {
                        if let Some(payload) = &candidate.parsed_data {
                            let result: Result<ImportAnalysis, _> =
                                state.import_service.analyze_import(payload).await;
                            candidate.analysis = match result {
                                Ok(analysis) => AnalysisState::Available(analysis),
                                Err(e) => AnalysisState::Failed(e.to_string()),
                            };
                        }
                    }
                    out
                } else {
                    parsed_candidates
                };

                // Detect wizard mode: any Ambiguous in any analysis
                let needs_wizard = analyzed.iter().any(|c| {
                    if let AnalysisState::Available(ref a) = c.analysis {
                        a.has_ambiguous()
                    } else {
                        false
                    }
                });

                set_wizard_mode.set(needs_wizard);
                if needs_wizard {
                    set_wizard_step.set(0);
                    set_wizard_decisions.set(std::collections::HashMap::new());
                }
                set_archived_confirmed.set(false);
                candidates_signal.set(analyzed);
                show_modal_signal.set(true);
            });
        }
    };

    let close_modal = move || {
        set_show_modal.set(false);
        set_candidates.set(Vec::new());
        set_selected_file_names.set(Vec::new());
        set_import_progress.set(None);
        set_import_results.set(Vec::new());
        set_conflict_strategy.set(ConflictStrategy::Merge);
        set_wizard_mode.set(false);
        set_wizard_step.set(0);
        set_wizard_decisions.set(std::collections::HashMap::new());
        set_archived_confirmed.set(false);
    };

    let close_modal_action = StoredValue::new_local(close_modal);

    let handle_apply_import = move || {
        if is_importing.get_untracked() {
            return;
        }

        let state_result = app_state.get();
        let decisions = wizard_decisions.get_untracked();
        let strategy = conflict_strategy.get_untracked();

        // Apply wizard decisions to payload: rewrite booth IDs for UseCandidate, remove Skipped
        let ready_candidates = candidates
            .get_untracked()
            .into_iter()
            .filter_map(|candidate| {
                candidate.parsed_data.clone().map(|payload| {
                    let adjusted = apply_wizard_decisions_to_payload(payload, &decisions);
                    (candidate.file_name, adjusted)
                })
            })
            .collect::<Vec<_>>();

        if ready_candidates.is_empty() {
            return;
        }

        set_is_importing.set(true);
        set_import_progress.set(Some((0, ready_candidates.len())));
        set_import_results.set(Vec::new());
        let log_error = log_error.clone();

        // Collect Advanced merges to run first (before import)
        let advanced_merges: Vec<(BoothId, BoothId)> = decisions
            .values()
            .filter_map(|d| {
                if let WizardChoice::Advanced { canonical, other } = d {
                    Some((*canonical, *other))
                } else {
                    None
                }
            })
            .collect();

        spawn_local(async move {
            let state = match state_result {
                Some(Ok(state)) => state,
                Some(Err(error)) => {
                    set_is_importing.set(false);
                    set_import_progress.set(None);
                    toast.error(format!("{}: {}", t!("backup.import_apply_failed")(), error));
                    return;
                }
                None => {
                    set_is_importing.set(false);
                    set_import_progress.set(None);
                    toast.error(format!(
                        "{}: {}",
                        t!("backup.import_apply_failed")(),
                        t!("common.loading")()
                    ));
                    return;
                }
            };

            // Run Advanced merges first, independently from the import transaction
            for (canonical_id, other_id) in &advanced_merges {
                if let Err(e) = state
                    .merge_service
                    .merge_booths(canonical_id, other_id)
                    .await
                {
                    set_is_importing.set(false);
                    set_import_progress.set(None);
                    toast.error(format!("{}: {}", t!("backup.import_apply_failed")(), e));
                    return;
                }
            }

            let mut combined = ImportSummary::default();
            let mut result_items = Vec::new();

            for (index, (file_name, payload)) in ready_candidates.iter().cloned().enumerate() {
                set_import_progress.set(Some((index + 1, ready_candidates.len())));

                let result = match payload {
                    ImportPayload::Full(data) => {
                        state.import_service.import_all(data, strategy).await
                    }
                    ImportPayload::Booth(data) => {
                        state
                            .import_service
                            .import_booth_backup_restoring_archived(
                                data,
                                strategy,
                                current_device_info(),
                            )
                            .await
                    }
                };

                match result {
                    Ok(summary) => {
                        combined.booths_imported += summary.booths_imported;
                        combined.vendors_imported += summary.vendors_imported;
                        combined.purchases_imported += summary.purchases_imported;
                        combined.conflicts_resolved += summary.conflicts_resolved;
                        combined.skipped_records.extend(summary.skipped_records);
                        result_items.push(ImportResultItem {
                            file_name,
                            success: true,
                            message: t!("backup.import_file_success")(),
                        });
                    }
                    Err(err) => {
                        log_error(ErrorLogDraft {
                            error_type: "import_apply_failed".to_string(),
                            error_message: err.to_string(),
                            stack_trace: stack_trace(),
                            user_action: Some("apply import".to_string()),
                            route: current_route(),
                            vendor_id: None,
                            purchase_id: None,
                            details: vec![format!("file={file_name}")],
                        });
                        result_items.push(ImportResultItem {
                            file_name,
                            success: false,
                            message: err.to_string(),
                        });
                    }
                }
            }

            set_is_importing.set(false);
            set_import_progress.set(None);
            set_import_results.set(result_items.clone());

            let success_count = result_items.iter().filter(|item| item.success).count();
            let failure_count = result_items.len().saturating_sub(success_count);

            if combined.booths_imported > 0
                || combined.vendors_imported > 0
                || combined.purchases_imported > 0
                || combined.conflicts_resolved > 0
            {
                booth_list_version.update(|version| *version += 1);
            }

            if failure_count == 0 {
                let message = t!("backup.import_apply_success")()
                    .replace("{booths}", &combined.booths_imported.to_string())
                    .replace("{vendors}", &combined.vendors_imported.to_string())
                    .replace("{purchases}", &combined.purchases_imported.to_string())
                    .replace("{resolved}", &combined.conflicts_resolved.to_string())
                    .replace("{skipped}", &combined.skipped_records.len().to_string());
                toast.success(message);
                close_modal_action.with_value(|close| close());
            } else {
                let message = t!("backup.import_partial_success")()
                    .replace("{success}", &success_count.to_string())
                    .replace("{failed}", &failure_count.to_string());
                toast.error(message);
            }
        });
    };

    let on_strategy_change = move |ev: Event| {
        let value = event_target_value(&ev);
        let strategy = match value.as_str() {
            "skip" => ConflictStrategy::Skip,
            "replace" => ConflictStrategy::Replace,
            _ => ConflictStrategy::Merge,
        };
        set_conflict_strategy.set(strategy);
    };

    let on_strategy_change_action = StoredValue::new_local(on_strategy_change);
    let handle_apply_import_action = StoredValue::new_local(handle_apply_import);

    let ready_count = Signal::derive(move || {
        candidates
            .get()
            .iter()
            .filter(|item| item.is_ready())
            .count()
    });
    let invalid_count = Signal::derive(move || {
        candidates
            .get()
            .iter()
            .filter(|item| !item.is_ready())
            .count()
    });
    let has_ready_candidates = Signal::derive(move || ready_count.get() > 0);

    view! {
        <>
            <input
                node_ref=input_ref
                type="file"
                accept="application/json,.json"
                class="hidden"
                multiple
                on:change=on_file_change
            />

            <Button
                on_click=Box::new(open_file_picker)
                variant=variant.unwrap_or(ButtonVariant::Secondary)
                size=size.unwrap_or(ButtonSize::Medium)
                class=class.unwrap_or_default()
                disabled=is_reading.get() || is_importing.get()
                title=t!("backup.import")()
            >
                <Icon icon=LuUpload class="h-4 w-4 shrink-0" />
                <span class="hidden sm:inline">
                    {move || if is_reading.get() || is_importing.get() {
                        t!("backup.import_in_progress")()
                    } else {
                        t!("backup.import")()
                    }}
                </span>
                <span class="sr-only sm:hidden">
                    {move || if is_reading.get() || is_importing.get() {
                        t!("backup.import_in_progress")()
                    } else {
                        t!("backup.import")()
                    }}
                </span>
            </Button>

            <Modal
                show=Signal::derive(move || show_modal.get())
                on_close=move || close_modal_action.with_value(|close| close())
                title=Signal::derive(move || t!("backup.import_review_title")())
                size=ModalSize::Large
                action_bar=
                    view! {
                        <div class="contents">
                            <Button
                                variant=ButtonVariant::Secondary
                                on_click=Box::new(move || close_modal_action.with_value(|close| close()))
                            >
                                {t!("common.close")}
                            </Button>
                            <Show when=move || has_ready_candidates.get()>
                                {move || {
                                    let needs_archived_confirm = candidates.get().iter().any(|c| {
                                        if let AnalysisState::Available(ref a) = c.analysis {
                                            a.has_archived_single()
                                        } else { false }
                                    });
                                    let on_final_wizard_step = if wizard_mode.get() {
                                        let ambiguous_count = candidates.get().iter()
                                            .filter_map(|c| if let AnalysisState::Available(ref a) = c.analysis { Some(a.booths.iter().filter(|b| matches!(b.resolution, BoothResolution::Ambiguous(_))).count()) } else { None })
                                            .sum::<usize>();
                                        wizard_step.get() > ambiguous_count
                                    } else { true };
                                    let all_decided = if wizard_mode.get() {
                                        let decisions = wizard_decisions.get();
                                        candidates.get().iter().all(|c| {
                                            if let AnalysisState::Available(ref a) = c.analysis {
                                                a.booths.iter().filter(|b| matches!(b.resolution, BoothResolution::Ambiguous(_))).all(|b| decisions.contains_key(&b.incoming_id.to_string()))
                                            } else { true }
                                        })
                                    } else { true };
                                    let apply_disabled = is_importing.get()
                                        || (needs_archived_confirm && !wizard_mode.get() && !archived_confirmed.get())
                                        || (wizard_mode.get() && (!on_final_wizard_step || !all_decided));
                                    view! {
                                        <Button
                                            on_click=Box::new(move || handle_apply_import_action.with_value(|handler| handler()))
                                            disabled=apply_disabled
                                        >
                                            {move || if is_importing.get() {
                                                t!("backup.import_in_progress")()
                                            } else {
                                                t!("backup.import_apply")()
                                            }}
                                        </Button>
                                    }
                                }}
                            </Show>
                        </div>
                    }
                    .into_any()
            >
                <div class="space-y-4 text-gray-700">
                    <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3 text-sm">
                        <span class="font-medium text-gray-900">{t!("backup.selected_files")}</span>
                        <span>{move || format!(" {}", selected_file_names.get().join(", "))}</span>
                    </div>

                    <div class="grid gap-3 sm:grid-cols-3">
                        <div class="rounded-lg border border-green-200 bg-green-50 px-4 py-3 text-sm text-green-900">
                            <p class="font-semibold">{t!("backup.import_ready_count")}</p>
                            <p>{move || ready_count.get().to_string()}</p>
                        </div>
                        <div class="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-900">
                            <p class="font-semibold">{t!("backup.import_blocked_count")}</p>
                            <p>{move || invalid_count.get().to_string()}</p>
                        </div>
                        <Show when=move || import_progress.get().is_some()>
                            {move || import_progress.get().map(|(current, total)| view! {
                                <div class="rounded-lg border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-900">
                                    <p class="font-semibold">{t!("backup.import_progress_label")}</p>
                                    <p>{t!("backup.import_progress_status")()
                                        .replace("{current}", &current.to_string())
                                        .replace("{total}", &total.to_string())}</p>
                                </div>
                            })}
                        </Show>
                    </div>

                    <Show when=move || has_ready_candidates.get()>
                        <div class="space-y-2 rounded-lg border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-900">
                            <p class="font-medium">{t!("backup.import_strategy_label")}</p>
                            <div class="space-y-1">
                                {[
                                    ("merge",   ConflictStrategy::Merge,   t!("backup.strategy_merge")(),   t!("backup.strategy_merge_desc")()),
                                    ("skip",    ConflictStrategy::Skip,    t!("backup.strategy_skip")(),    t!("backup.strategy_skip_desc")()),
                                    ("replace", ConflictStrategy::Replace, t!("backup.strategy_replace")(), t!("backup.strategy_replace_desc")()),
                                ].into_iter().map(|(val, strat, label, desc)| {
                                    let is_selected = move || conflict_strategy.get() == strat;
                                    view! {
                                        <label class="flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 hover:bg-blue-100">
                                            <input
                                                type="radio"
                                                name="conflict_strategy"
                                                value=val
                                                class="mt-0.5 accent-blue-600"
                                                prop:checked=is_selected
                                                on:change=move |ev| on_strategy_change_action.with_value(|handler| handler(ev))
                                            />
                                            <div>
                                                <p class="font-medium leading-snug">{label}</p>
                                                <p class="text-xs text-blue-700 leading-snug">{desc}</p>
                                            </div>
                                        </label>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                            <p class="text-xs text-blue-700">{t!("backup.import_apply_ready")}</p>
                        </div>
                    </Show>

                    // Mode 1: archived restore confirmation
                    <Show when=move || {
                        !wizard_mode.get() && candidates.get().iter().any(|c| {
                            if let AnalysisState::Available(ref a) = c.analysis { a.has_archived_single() } else { false }
                        })
                    }>
                        <div class="rounded-lg border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-900 space-y-2">
                            <p class="font-medium">{t!("backup.wizard_archived_title")}</p>
                            <p class="text-xs">{t!("backup.wizard_archived_body")}</p>
                            <ul class="list-inside list-disc text-xs space-y-0.5">
                                {move || candidates.get().iter().filter_map(|c| {
                                    if let AnalysisState::Available(ref a) = c.analysis {
                                        let names: Vec<String> = a.booths.iter().filter_map(|b| {
                                            if let BoothResolution::Single(ref cand) = b.resolution {
                                                if cand.is_archived { Some(b.incoming_description.clone()) } else { None }
                                            } else { None }
                                        }).collect();
                                        if !names.is_empty() { Some(names) } else { None }
                                    } else { None }
                                }).flatten().map(|name| view! { <li>{name}</li> }).collect::<Vec<_>>()}
                            </ul>
                            <label class="flex cursor-pointer items-center gap-2 pt-1">
                                <input
                                    type="checkbox"
                                    class="accent-amber-600"
                                    prop:checked=move || archived_confirmed.get()
                                    on:change=move |ev| {
                                        use wasm_bindgen::JsCast;
                                        if let Ok(cb) = ev.target().unwrap().dyn_into::<web_sys::HtmlInputElement>() {
                                            set_archived_confirmed.set(cb.checked());
                                        }
                                    }
                                />
                                <span class="text-xs font-medium">{t!("backup.wizard_archived_confirm")}</span>
                            </label>
                        </div>
                    </Show>

                    // Mode 2: wizard steps
                    <Show when=move || wizard_mode.get()>
                        {move || {
                            let all_candidates = candidates.get();
                            let ambiguous_booths: Vec<(String, Vec<BoothCandidate>)> = all_candidates.iter().flat_map(|c| {
                                if let AnalysisState::Available(ref a) = c.analysis {
                                    a.booths.iter().filter_map(|b| {
                                        if let BoothResolution::Ambiguous(ref cands) = b.resolution {
                                            Some((b.incoming_id.to_string(), cands.clone()))
                                        } else { None }
                                    }).collect::<Vec<_>>()
                                } else { vec![] }
                            }).collect();
                            let total_steps = ambiguous_booths.len();
                            let step = wizard_step.get();
                            let decisions = wizard_decisions.get();

                            if step == 0 {
                                // Step 0: overview
                                view! {
                                    <div class="rounded-lg border border-blue-200 bg-blue-50 px-4 py-4 text-sm text-blue-900 space-y-2">
                                        <p class="font-semibold">{t!("backup.wizard_overview_title")}</p>
                                        <p>
                                            {
                                                let auto_count: usize = all_candidates.iter()
                                                    .map(|c| if let AnalysisState::Available(ref a) = c.analysis { a.booths.len() } else { 0 })
                                                    .sum::<usize>()
                                                    .saturating_sub(total_steps);
                                                format!("{} event{} need your attention before import can proceed. {} other event{} will be handled automatically.",
                                                    total_steps, if total_steps == 1 { "" } else { "s" },
                                                    auto_count, if auto_count == 1 { "" } else { "s" }
                                                )
                                            }
                                        </p>
                                        <button
                                            type="button"
                                            class="mt-2 rounded-lg bg-blue-600 px-4 py-2 text-xs font-medium text-white hover:bg-blue-700 focus:outline-none"
                                            on:click=move |_| set_wizard_step.set(1)
                                        >
                                            {t!("backup.wizard_overview_review")}
                                        </button>
                                    </div>
                                }.into_any()
                            } else if step <= total_steps {
                                let (booth_id_str, candidates_for_step) = ambiguous_booths[step - 1].clone();
                                let booth_name = candidates_for_step.first().map(|c| c.description.clone()).unwrap_or_default();
                                let already_decided = decisions.get(&booth_id_str).cloned();
                                view! {
                                    <div class="rounded-lg border border-gray-200 bg-white p-4 space-y-3 text-sm">
                                        <p class="text-xs text-gray-500">
                                            {t!("backup.wizard_step_label")()
                                                .replace("{step}", &step.to_string())
                                                .replace("{total}", &total_steps.to_string())}
                                        </p>
                                        <p class="font-semibold text-gray-900">{t!("backup.wizard_step_question")}</p>
                                        <p class="text-gray-700">{booth_name.clone()}</p>
                                        <div class="space-y-2">
                                            {candidates_for_step.iter().map(|cand| {
                                                let cand_id = cand.id;
                                                let cand_id_str = cand.id.to_string();
                                                let bid_str = booth_id_str.clone();
                                                let is_selected = already_decided.as_ref().map(|d| *d == WizardChoice::UseCandidate(cand_id)).unwrap_or(false);
                                                view! {
                                                    <label class="flex cursor-pointer items-start gap-2 rounded-md border border-gray-200 p-3 hover:bg-gray-50">
                                                        <input
                                                            type="radio"
                                                            name=format!("wizard_{}", booth_id_str)
                                                            value=cand_id_str
                                                            class="mt-0.5 accent-blue-600"
                                                            prop:checked=move || is_selected
                                                            on:change=move |_| {
                                                                let mut d = wizard_decisions.get_untracked();
                                                                d.insert(bid_str.clone(), WizardChoice::UseCandidate(cand_id));
                                                                set_wizard_decisions.set(d);
                                                            }
                                                        />
                                                        <div>
                                                            <p class="font-medium text-gray-800">{format!("{} vendors · {} purchases", cand.vendor_count, cand.purchase_count)}</p>
                                                            <p class="text-xs text-gray-500">{t!("backup.wizard_candidate_source")}</p>
                                                        </div>
                                                    </label>
                                                }
                                            }).collect::<Vec<_>>()}
                                            {
                                                let bid_str = booth_id_str.clone();
                                                let is_skip = already_decided.as_ref().map(|d| *d == WizardChoice::Skip).unwrap_or(false);
                                                view! {
                                                    <label class="flex cursor-pointer items-center gap-2 rounded-md border border-gray-200 p-3 hover:bg-gray-50">
                                                        <input
                                                            type="radio"
                                                            name=format!("wizard_{}", booth_id_str)
                                                            value="__skip__"
                                                            class="accent-blue-600"
                                                            prop:checked=move || is_skip
                                                            on:change=move |_| {
                                                                let mut d = wizard_decisions.get_untracked();
                                                                d.insert(bid_str.clone(), WizardChoice::Skip);
                                                                set_wizard_decisions.set(d);
                                                            }
                                                        />
                                                        <span class="text-sm text-gray-700">{t!("backup.wizard_skip_option")}</span>
                                                    </label>
                                                }
                                            }

                                            // Advanced option — only when 2 candidates
                                            {if candidates_for_step.len() == 2 {
                                                let cand0 = candidates_for_step[0].id;
                                                let cand1 = candidates_for_step[1].id;
                                                let bid_str = booth_id_str.clone();
                                                // Canonical = more purchases
                                                let canonical_id = cand0; // write phase handles canonical selection
                                                let other_id = cand1;
                                                let is_advanced = already_decided.as_ref().map(|d| {
                                                    matches!(d, WizardChoice::Advanced { .. })
                                                }).unwrap_or(false);
                                                view! {
                                                    <div class="mt-2 border-t border-gray-200 pt-3">
                                                        <label class="flex cursor-pointer items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 hover:bg-amber-100">
                                                            <input
                                                                type="radio"
                                                                name=format!("wizard_{}", booth_id_str)
                                                                value="__advanced__"
                                                                class="mt-0.5 accent-amber-600"
                                                                prop:checked=move || is_advanced
                                                                on:change=move |_| {
                                                                    let mut d = wizard_decisions.get_untracked();
                                                                    d.insert(bid_str.clone(), WizardChoice::Advanced { canonical: canonical_id, other: other_id });
                                                                    set_wizard_decisions.set(d);
                                                                }
                                                            />
                                                            <div>
                                                                <p class="font-medium text-amber-900 text-sm">{t!("backup.wizard_advanced_label")}</p>
                                                                <p class="text-xs text-amber-700 mt-0.5">{t!("backup.wizard_advanced_desc")}</p>
                                                            </div>
                                                        </label>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <span /> }.into_any()
                                            }}
                                        </div>
                                        <div class="flex gap-2 pt-2">
                                            <button
                                                type="button"
                                                class="rounded-lg border border-gray-300 px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-gray-100"
                                                on:click=move |_| set_wizard_step.update(|s| { if *s > 0 { *s -= 1; } })
                                            >
                                                {t!("backup.wizard_back")}
                                            </button>
                                            <button
                                                type="button"
                                                class="rounded-lg bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
                                                on:click=move |_| set_wizard_step.update(|s| *s += 1)
                                            >
                                                {t!("backup.wizard_next")}
                                            </button>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                // Final step: summary
                                let unresolvable: Vec<String> = all_candidates.iter().flat_map(|c| {
                                    if let AnalysisState::Available(ref a) = c.analysis {
                                        a.booths.iter().filter_map(|b| {
                                            if matches!(b.resolution, BoothResolution::UnresolvableAmbiguous) {
                                                Some(b.incoming_description.clone())
                                            } else { None }
                                        }).collect::<Vec<_>>()
                                    } else { vec![] }
                                }).collect();
                                view! {
                                    <div class="space-y-3 text-sm">
                                        <p class="font-semibold text-gray-900">{t!("backup.wizard_summary_title")}</p>
                                        <ul class="space-y-1 text-gray-700">
                                            {ambiguous_booths.iter().map(|(bid, cands)| {
                                                let name = cands.first().map(|c| c.description.clone()).unwrap_or_default();
                                                let decision_str = match decisions.get(bid) {
                                                    Some(WizardChoice::UseCandidate(_)) => t!("backup.wizard_decision_merge")(),
                                                    Some(WizardChoice::Advanced { .. }) => t!("backup.wizard_advanced_label")(),
                                                    Some(WizardChoice::Skip) => t!("backup.wizard_decision_skip")(),
                                                    Some(WizardChoice::ImportAsNew) => t!("backup.wizard_decision_new")(),
                                                    None => t!("backup.wizard_decision_none")(),
                                                };
                                                view! { <li><span class="font-medium">{name}</span>": "{decision_str}</li> }
                                            }).collect::<Vec<_>>()}
                                        </ul>
                                        {
                                            let unresolvable2 = unresolvable.clone();
                                            if !unresolvable.is_empty() {
                                                view! {
                                                    <div class="rounded-md border border-amber-200 bg-amber-50 p-3 text-xs text-amber-800">
                                                        <p class="font-semibold mb-1">{t!("backup.wizard_unresolvable_title")}</p>
                                                        {unresolvable2.into_iter().map(|name| view! { <p>{name}" — "{t!("backup.wizard_unresolvable_hint")}</p> }).collect::<Vec<_>>()}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <span /> }.into_any()
                                            }
                                        }
                                        <button
                                            type="button"
                                            class="rounded-lg border border-gray-300 px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-gray-100"
                                            on:click=move |_| set_wizard_step.update(|s| { if *s > 0 { *s -= 1; } })
                                        >
                                            {t!("backup.wizard_back")}
                                        </button>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </Show>

                    <div class="space-y-3 max-h-[28rem] overflow-y-auto pr-1">
                        <For
                            each=move || candidates.get()
                            key=|candidate| candidate.file_name.clone()
                            children=move |candidate| {
                                let file_name = candidate.file_name;
                                let preview = candidate.preview;
                                let validation_failures = candidate.validation_failures;
                                let structure_error = candidate.structure_error;
                                let is_ready = preview.is_some();
                                let preview_view = match preview.clone() {
                                    Some(ImportPreview::Full {
                                        booths,
                                        vendors,
                                        purchases,
                                    }) => Some(
                                        view! {
                                            <p class="mt-2 text-sm text-gray-600">
                                                {t!("backup.import_counts")()
                                                    .replace("{booths}", &booths.to_string())
                                                    .replace("{vendors}", &vendors.to_string())
                                                    .replace("{purchases}", &purchases.to_string())}
                                            </p>
                                        }
                                            .into_any(),
                                    ),
                                    Some(ImportPreview::Booth {
                                        description,
                                        vendors,
                                        purchases,
                                    }) => Some(
                                        view! {
                                            <div class="mt-2 space-y-1 text-sm text-gray-600">
                                                <p>{t!("backup.import_valid_booth")().replace("{description}", &description)}</p>
                                                <p>
                                                    {t!("backup.import_booth_counts")()
                                                        .replace("{vendors}", &vendors.to_string())
                                                        .replace("{purchases}", &purchases.to_string())}
                                                </p>
                                            </div>
                                        }
                                            .into_any(),
                                    ),
                                    None => None,
                                };
                                let validation_view = if validation_failures.is_empty() {
                                    None
                                } else {
                                    Some(
                                        view! {
                                            <div class="mt-3 space-y-2 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
                                                <p class="font-medium">{t!("backup.import_validation_summary")}</p>
                                                <For
                                                    each=move || validation_failures.clone()
                                                    key=|failure| format!("{}:{}:{}", failure.record_type, failure.record_id, failure.reason)
                                                    children=move |failure| {
                                                        view! {
                                                            <div class="rounded border border-red-100 bg-white px-3 py-2">
                                                                <p class="font-medium">{format!("{} {}", failure.record_type, failure.record_id)}</p>
                                                                <p>{failure.reason}</p>
                                                            </div>
                                                        }
                                                    }
                                                />
                                            </div>
                                        }
                                            .into_any(),
                                    )
                                };
                                let structure_view = structure_error.map(|message| {
                                    view! {
                                        <div class="mt-3 rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-900">
                                            <p class="font-medium">{t!("backup.import_structure_summary")}</p>
                                            <pre class="mt-2 whitespace-pre-wrap">{message}</pre>
                                        </div>
                                    }
                                        .into_any()
                                });
                                view! {
                                    <div class="rounded-lg border border-gray-200 bg-white px-4 py-3 shadow-sm">
                                        <div class="flex items-start justify-between gap-3">
                                            <div>
                                                <p class="font-medium text-gray-900">{file_name.clone()}</p>
                                                <Show when=move || is_ready>
                                                    <p class="text-sm text-green-700">{t!("backup.import_ready_label")}</p>
                                                </Show>
                                                <Show when=move || !is_ready>
                                                    <p class="text-sm text-red-700">{t!("backup.import_invalid_label")}</p>
                                                </Show>
                                            </div>
                                        </div>

                                        {preview_view}
                                        {validation_view}
                                        {structure_view}
                                    </div>
                                }
                            }
                        />
                    </div>

                    <Show when=move || !import_results.get().is_empty()>
                        <div class="space-y-3 rounded-lg border border-gray-200 bg-gray-50 px-4 py-3 text-sm text-gray-800">
                            <p class="font-medium text-gray-900">{t!("backup.import_results_label")}</p>
                            <For
                                each=move || import_results.get()
                                key=|item| item.file_name.clone()
                                children=move |item| {
                                    let status_class = if item.success {
                                        "text-green-700"
                                    } else {
                                        "text-red-700"
                                    };
                                    view! {
                                        <div class="rounded border border-gray-200 bg-white px-3 py-2">
                                            <p class="font-medium text-gray-900">{item.file_name}</p>
                                            <p class=status_class>{item.message}</p>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </Show>
                </div>
            </Modal>
        </>
    }
}

async fn read_and_parse_file(file: WebFile, validator: Rc<ImportValidator>) -> ImportCandidate {
    let file_name = file.name();

    match read_file_as_text(&file).await {
        Ok(contents) => match parse_import_data(&validator, &contents) {
            Ok((preview, parsed_data)) => ImportCandidate {
                file_name,
                preview: Some(preview),
                parsed_data: Some(parsed_data),
                validation_failures: Vec::new(),
                structure_error: None,
                analysis: AnalysisState::Pending,
            },
            Err(ImportError::ValidationFailed { failures }) => ImportCandidate {
                file_name,
                preview: None,
                parsed_data: None,
                validation_failures: failures,
                structure_error: None,
                analysis: AnalysisState::Pending,
            },
            Err(other) => ImportCandidate {
                file_name,
                preview: None,
                parsed_data: None,
                validation_failures: Vec::new(),
                structure_error: Some(format_import_error(&other)),
                analysis: AnalysisState::Pending,
            },
        },
        Err(message) => ImportCandidate {
            file_name,
            preview: None,
            parsed_data: None,
            validation_failures: Vec::new(),
            structure_error: Some(message),
            analysis: AnalysisState::Pending,
        },
    }
}

fn parse_import_data(
    validator: &ImportValidator,
    contents: &str,
) -> Result<(ImportPreview, ImportPayload), ImportError> {
    let booth_validation = validator.validate_booth_backup(contents).map(|data| {
        let preview = ImportPreview::Booth {
            description: data.booth.description.clone(),
            vendors: data.vendors.len(),
            purchases: data.purchases.len(),
        };
        (preview, ImportPayload::Booth(data))
    });

    match booth_validation {
        Ok(booth) => Ok(booth),
        Err(ImportError::InvalidJson(_)) | Err(ImportError::InvalidStructure(_)) => {
            // Only fall back when the payload does not match the booth backup shape.
            // Once a file parses as a booth backup, checksum/version/validation errors
            // should be shown directly instead of reinterpreting the same payload as a
            // different backup type.
            validator.validate_backup(contents).map(|data| {
                let preview = ImportPreview::Full {
                    booths: data.booths.len(),
                    vendors: data.vendors.len(),
                    purchases: data.purchases.len(),
                };
                (preview, ImportPayload::Full(data))
            })
        }
        Err(other) => Err(other),
    }
}

fn format_import_error(error: &ImportError) -> String {
    match error {
        ImportError::UnsupportedVersion { found, supported } => t!("backup.import_version_error")()
            .replace("{found}", &found.to_string())
            .replace("{supported}", &supported.to_string()),
        ImportError::OrphanedRecords { details } => details.join("\n"),
        ImportError::InvalidStructure(message) | ImportError::InvalidJson(message) => {
            message.clone()
        }
        ImportError::ChecksumMismatch { expected, actual } => t!("backup.checksum_failed")()
            .replace(
                "{detail}",
                &t!("backup.checksum_mismatch_detail")()
                    .replace("{expected}", expected)
                    .replace("{actual}", actual),
            ),
        _ => error.to_string(),
    }
}

/// Transform a payload based on wizard decisions before passing to import_all.
/// - UseCandidate(id): rewrite booth.id (and subordinate booth_ids) to the chosen canonical id
/// - Skip: remove the booth and its subordinates from the payload
/// - ImportAsNew: leave as-is
fn apply_wizard_decisions_to_payload(
    payload: ImportPayload,
    decisions: &std::collections::HashMap<String, WizardChoice>,
) -> ImportPayload {
    use ez_vend_storage::export::BoothBackupData;

    if decisions.is_empty() {
        return payload;
    }

    match payload {
        ImportPayload::Full(mut data) => {
            let mut keep_booth_ids: std::collections::HashMap<BoothId, BoothId> =
                std::collections::HashMap::new(); // original → canonical
            let mut skip_ids: std::collections::HashSet<BoothId> = std::collections::HashSet::new();

            data.booths.retain(|booth| {
                let key = booth.id.to_string();
                match decisions.get(&key) {
                    Some(WizardChoice::Skip) => {
                        skip_ids.insert(booth.id);
                        false
                    }
                    Some(WizardChoice::UseCandidate(canonical_id)) => {
                        keep_booth_ids.insert(booth.id, *canonical_id);
                        true
                    }
                    Some(WizardChoice::Advanced { canonical, .. }) => {
                        keep_booth_ids.insert(booth.id, *canonical);
                        true
                    }
                    _ => true,
                }
            });

            // Rewrite IDs on retained booths
            for booth in &mut data.booths {
                if let Some(&canonical) = keep_booth_ids.get(&booth.id) {
                    booth.id = canonical;
                }
            }

            // Remap vendors and purchases; drop those belonging to skipped booths
            data.vendors.retain_mut(|v| {
                if skip_ids.contains(&v.booth_id) {
                    return false;
                }
                if let Some(&canonical) = keep_booth_ids.get(&v.booth_id) {
                    v.booth_id = canonical;
                }
                true
            });

            data.purchases.retain_mut(|p| {
                if skip_ids.contains(&p.booth_id) {
                    return false;
                }
                if let Some(&canonical) = keep_booth_ids.get(&p.booth_id) {
                    p.booth_id = canonical;
                }
                true
            });

            ImportPayload::Full(data)
        }
        ImportPayload::Booth(mut data) => {
            let key = data.booth.id.to_string();
            match decisions.get(&key) {
                Some(WizardChoice::Skip) => {
                    // Return an empty payload — caller will produce zero imports
                    ImportPayload::Booth(BoothBackupData {
                        vendors: vec![],
                        purchases: vec![],
                        ..data
                    })
                }
                Some(WizardChoice::UseCandidate(canonical_id))
                | Some(WizardChoice::Advanced {
                    canonical: canonical_id,
                    ..
                }) => {
                    let old_id = data.booth.id;
                    data.booth.id = *canonical_id;
                    for v in &mut data.vendors {
                        if v.booth_id == old_id {
                            v.booth_id = *canonical_id;
                        }
                    }
                    for p in &mut data.purchases {
                        if p.booth_id == old_id {
                            p.booth_id = *canonical_id;
                        }
                    }
                    ImportPayload::Booth(data)
                }
                _ => ImportPayload::Booth(data),
            }
        }
    }
}

async fn read_file_as_text(file: &WebFile) -> Result<String, String> {
    let reader =
        FileReader::new().map_err(|err| format!("{}: {:?}", t!("backup.import_failed")(), err))?;

    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let reader_for_load = reader.clone();
        let resolve_fn = resolve.clone();
        let reject_for_load = reject.clone();

        let onload =
            Closure::once(Box::new(
                move |_event: ProgressEvent| match reader_for_load.result() {
                    Ok(result) => {
                        if let Some(contents) = result.as_string() {
                            let _ = resolve_fn.call1(&JsValue::NULL, &JsValue::from_str(&contents));
                        } else {
                            let _ = reject_for_load.call1(
                                &JsValue::NULL,
                                &JsValue::from_str(&t!("backup.import_invalid_encoding")()),
                            );
                        }
                    }
                    Err(_) => {
                        let _ = reject_for_load.call1(
                            &JsValue::NULL,
                            &JsValue::from_str(&t!("backup.import_failed")()),
                        );
                    }
                },
            ) as Box<dyn FnOnce(_)>);

        let reject_for_error = reject.clone();
        let onerror = Closure::once(Box::new(move |_event: ProgressEvent| {
            let _ = reject_for_error.call1(
                &JsValue::NULL,
                &JsValue::from_str(&t!("backup.import_failed")()),
            );
        }) as Box<dyn FnOnce(_)>);

        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        if let Err(err) = reader.read_as_text(file) {
            let _ = reject.call1(
                &JsValue::NULL,
                &JsValue::from_str(&format!("{}: {:?}", t!("backup.import_failed")(), err)),
            );
        }

        onload.forget();
        onerror.forget();
    });

    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|err| {
            err.as_string()
                .unwrap_or_else(|| t!("backup.import_failed")())
        })?
        .as_string()
        .ok_or_else(|| t!("backup.import_invalid_encoding")())
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};
    use domain::{Booth, FeeConfig};
    use ez_vend_storage::export::{
        BackupData, BoothBackupData, DeviceInfo, ImportError, ImportValidator,
        BACKUP_FORMAT_VERSION,
    };
    use rust_decimal_macros::dec;

    use super::{parse_import_data, ImportPayload, ImportPreview};

    fn sample_booth() -> Booth {
        Booth::new(
            "Spring Market 2026".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 29).unwrap(),
            FeeConfig {
                participation_fee: dec!(10.00),
                sales_fee_percent: dec!(15.00),
                rounding_step: dec!(0.50),
            },
        )
        .unwrap()
    }

    #[test]
    fn booth_backup_with_empty_lists_is_detected_as_booth_backup() {
        let validator = ImportValidator::new();
        let booth = sample_booth();
        let mut backup = BoothBackupData::new(booth.clone(), "test-version");
        backup.created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();
        backup.device_info = Some(DeviceInfo {
            identifier: "marias-laptop".to_string(),
            platform: "Windows".to_string(),
            browser: "Chrome".to_string(),
        });

        let contents = serde_json::to_string(&backup).unwrap();
        let result = parse_import_data(&validator, &contents).unwrap();

        match result {
            (
                ImportPreview::Booth {
                    description,
                    vendors,
                    purchases,
                },
                ImportPayload::Booth(data),
            ) => {
                assert_eq!(description, booth.description);
                assert_eq!(vendors, 0);
                assert_eq!(purchases, 0);
                assert_eq!(data.booth.id, booth.id);
                assert_eq!(
                    data.device_info.unwrap().identifier,
                    "marias-laptop".to_string()
                );
            }
            other => panic!("expected booth backup parse result, got {other:?}"),
        }
    }

    #[test]
    fn full_backup_with_empty_lists_is_detected_as_full_backup() {
        let validator = ImportValidator::new();
        let booth = sample_booth();
        let backup = BackupData {
            version: BACKUP_FORMAT_VERSION,
            created_at: Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap(),
            app_version: "test-version".to_string(),
            checksum: None,
            device_info: Some(DeviceInfo {
                identifier: "marias-laptop".to_string(),
                platform: "Windows".to_string(),
                browser: "Chrome".to_string(),
            }),
            booths: vec![booth.clone()],
            vendors: Vec::new(),
            purchases: Vec::new(),
            metadata: Default::default(),
        };

        let contents = serde_json::to_string(&backup).unwrap();
        let result = parse_import_data(&validator, &contents).unwrap();

        match result {
            (
                ImportPreview::Full {
                    booths,
                    vendors,
                    purchases,
                },
                ImportPayload::Full(data),
            ) => {
                assert_eq!(booths, 1);
                assert_eq!(vendors, 0);
                assert_eq!(purchases, 0);
                assert_eq!(data.booths[0].id, booth.id);
                assert_eq!(
                    data.device_info.unwrap().identifier,
                    "marias-laptop".to_string()
                );
            }
            other => panic!("expected full backup parse result, got {other:?}"),
        }
    }

    #[test]
    fn booth_backup_checksum_mismatch_is_reported_without_fallback() {
        let validator = ImportValidator::new();
        let booth = sample_booth();
        let mut backup = BoothBackupData::new(booth, "test-version");
        backup.created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();
        backup.checksum = Some("wrong".to_string());

        let contents = serde_json::to_string(&backup).unwrap();
        let error = parse_import_data(&validator, &contents).unwrap_err();

        assert!(matches!(error, ImportError::ChecksumMismatch { .. }));
    }
}

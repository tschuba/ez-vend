#![allow(clippy::clone_on_copy)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::let_unit_value)]

use crate::booth_ordering::{sort_booths, split_booths};
use crate::components::*;
use crate::error_translator::translate_domain_error;
use crate::formatting::{format_date_with_contextual_year as format_display_date, format_datetime};
use crate::i18n::{translate_with_params, use_locale};
use crate::selected_booth_context::use_selected_booth;
use crate::state::*;
use crate::t;
use domain::models::booth::Booth;
use domain::models::{BoothId, BoothSummary, BoothType, Vendor};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use web_sys::window;

fn confirmation_token_from_booth(booth_id: &BoothId) -> String {
    confirmation_token_from_id_str(&booth_id.as_str())
}

fn confirmation_token_from_id_str(id: &str) -> String {
    let sanitized: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();

    const FALLBACK_TOKEN: &str = "DELETE";

    if sanitized.len() < 4 {
        return FALLBACK_TOKEN.to_string();
    }

    let len = sanitized.len();
    let start = len.saturating_sub(4);
    sanitized[start..].to_uppercase()
}

fn focus_and_select_input(input_ref: &NodeRef<html::Input>) {
    if let Some(input) = input_ref.get() {
        let _ = input.focus();
        let _ = input.select();
    }
}

const SHOW_ARCHIVED_SECTION_STORAGE_KEY: &str = "ez-vend-show-archived-section";

fn load_show_archived_section_preference() -> bool {
    window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| {
            storage
                .get_item(SHOW_ARCHIVED_SECTION_STORAGE_KEY)
                .ok()
                .flatten()
        })
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(true)
}

fn persist_show_archived_section_preference(show_archived_section: bool) {
    if let Some(storage) = window().and_then(|window| window.local_storage().ok().flatten()) {
        let _ = storage.set_item(
            SHOW_ARCHIVED_SECTION_STORAGE_KEY,
            &show_archived_section.to_string(),
        );
    }
}

#[component]
pub fn BoothListPage() -> impl IntoView {
    let app_state = use_app_state();
    let locale = use_locale();
    let selected_booth = use_selected_booth();
    let booth_list_version = crate::selected_booth_context::use_booth_list_version();
    let (booths, set_booths) = signal(Vec::<Booth>::new());
    let (show_archive_modal, set_show_archive_modal) = signal(false);
    let (archiving_booth, set_archiving_booth) = signal(None::<Booth>);
    let (show_archived_section, set_show_archived_section) =
        signal(load_show_archived_section_preference());
    let (booth_search_query, set_booth_search_query) = signal(String::new());
    let (show_create_modal, set_show_create_modal) = signal(false);
    let (show_edit_modal, set_show_edit_modal) = signal(false);
    let (show_copy_modal, set_show_copy_modal) = signal(false);
    let (show_switch_modal, set_show_switch_modal) = signal(false);
    let (show_delete_confirm, set_show_delete_confirm) = signal(false);
    let (editing_booth, set_editing_booth) = signal(None::<Booth>);
    let (copying_booth, set_copying_booth) = signal(None::<Booth>);
    let (switch_target_booth, set_switch_target_booth) = signal(None::<Booth>);
    let (deleting_booth, set_deleting_booth) = signal(None::<Booth>);
    let (is_checking_delete_requirements, set_is_checking_delete_requirements) = signal(false);
    let (delete_confirmation_token, set_delete_confirmation_token) = signal(String::new());
    let (delete_confirmation_input, set_delete_confirmation_input) = signal(String::new());
    let delete_confirmation_ref: NodeRef<html::Input> = NodeRef::new();
    let (is_loading, set_is_loading) = signal(true);
    let (vendor_counts, set_vendor_counts) =
        signal(std::collections::HashMap::<BoothId, usize>::new());
    let (purchase_counts, set_purchase_counts) =
        signal(std::collections::HashMap::<BoothId, usize>::new());
    let (product_group_counts, set_product_group_counts) =
        signal(std::collections::HashMap::<BoothId, usize>::new());
    let create_step = RwSignal::new(0_usize);
    let selected_create_type = RwSignal::new(BoothType::ThirdPartySale);
    let initial_edit_tab = RwSignal::new(0_usize);
    let open_edit_after_switch = RwSignal::new(false);
    let (duplicate_groups, set_duplicate_groups) = signal(Vec::<Vec<Booth>>::new());
    let (show_dedup_modal, set_show_dedup_modal) = signal(false);
    let (dedup_detail, set_dedup_detail) = signal(Vec::<(Vec<Booth>, Vec<Vec<Vendor>>)>::new());
    let (is_merging, set_is_merging) = signal(false);
    let (expanded_booth_id, set_expanded_booth_id) = signal(None::<BoothId>);
    let (expanded_booth_summary, set_expanded_booth_summary) = signal(None::<BoothSummary>);
    let (is_loading_report, set_is_loading_report) = signal(false);
    let deletion_token_matches = Memo::new(move |_| {
        let required = delete_confirmation_token.get().trim().to_uppercase();

        if required.is_empty() {
            return false;
        }

        let entered = delete_confirmation_input.get().trim().to_uppercase();
        entered == required
    });

    let toast = use_toast();

    let refresh_booths = move |state: AppState| {
        spawn_local(async move {
            match state.booth_repository.find_all().await {
                Ok(loaded_booths) => {
                    let mut sorted = loaded_booths;
                    sort_booths(&mut sorted);
                    set_booths.set(sorted);
                }
                Err(_) => {
                    toast.error(&t!("booth.errors.load_failed")());
                }
            }
        });
    };

    let format_date =
        move |date: chrono::NaiveDate| -> String { format_display_date(date, locale.get()) };

    let filtered_booths = Signal::derive(move || {
        let query = booth_search_query.get().trim().to_lowercase();

        if query.is_empty() {
            return booths.get();
        }

        booths
            .get()
            .into_iter()
            .filter(|booth| booth.description.to_lowercase().contains(&query))
            .collect::<Vec<_>>()
    });

    let booth_sections = Signal::derive(move || split_booths(&filtered_booths.get()));

    Effect::new(move |_| {
        persist_show_archived_section_preference(show_archived_section.get());
    });

    let close_report_modal = move || {
        set_expanded_booth_id.set(None);
        set_expanded_booth_summary.set(None);
        set_is_loading_report.set(false);
    };

    Effect::new(move |_| {
        let _ = booth_list_version.get();
        let state_result = app_state.get();

        if let Some(Ok(state)) = state_result {
            spawn_local(async move {
                match state.booth_repository.find_all().await {
                    Ok(loaded_booths) => {
                        let mut sorted = loaded_booths;
                        sort_booths(&mut sorted);
                        set_booths.set(sorted);
                        set_is_loading.set(false);
                    }
                    Err(_) => {
                        toast.error(&t!("booth.errors.load_failed")());
                        set_is_loading.set(false);
                    }
                }
                if let Ok(all_vendors) = state.vendor_repository.find_all().await {
                    let mut counts = std::collections::HashMap::<BoothId, usize>::new();
                    for vendor in all_vendors {
                        *counts.entry(vendor.booth_id).or_insert(0) += 1;
                    }
                    set_vendor_counts.set(counts);
                }
                if let Ok(all_purchases) = state.purchase_repository.find_all().await {
                    let mut counts = std::collections::HashMap::<BoothId, usize>::new();
                    for purchase in all_purchases {
                        *counts.entry(purchase.booth_id).or_insert(0) += 1;
                    }
                    set_purchase_counts.set(counts);
                }
                {
                    let mut pgc = std::collections::HashMap::<BoothId, usize>::new();
                    for booth in booths.get_untracked() {
                        if booth.booth_type == BoothType::DirectSale {
                            let count = state
                                .product_group_repository
                                .find_by_booth(&booth.id)
                                .await
                                .map(|v| v.len())
                                .unwrap_or(0);
                            pgc.insert(booth.id, count);
                        }
                    }
                    set_product_group_counts.set(pgc);
                }
                if let Ok(groups) = state.booth_repository.find_duplicate_groups().await {
                    set_duplicate_groups.set(groups);
                }
            });
        } else if let Some(Err(e)) = state_result {
            toast.error(&t!("booth.errors_detail.init_failed")().replace("{error}", &e));
            set_is_loading.set(false);
        }
    });

    Effect::new(move |_| {
        let state_result = app_state.get();
        let booth_id = expanded_booth_id.get();
        let expanded_booth = booth_id
            .and_then(|booth_id| booths.get().into_iter().find(|booth| booth.id == booth_id));

        if let Some(booth) = expanded_booth {
            if booth.is_archived() {
                set_expanded_booth_summary.set(None);
                set_is_loading_report.set(false);
                return;
            }
        }

        if let (Some(Ok(state)), Some(booth_id)) = (state_result, booth_id) {
            set_is_loading_report.set(true);
            let booth_id_for_request = booth_id.clone();
            spawn_local(async move {
                match state
                    .report_service
                    .generate_booth_summary(&booth_id_for_request, None)
                    .await
                {
                    Ok(summary) => {
                        if expanded_booth_id.get_untracked() == Some(booth_id_for_request) {
                            set_expanded_booth_summary.set(Some(summary));
                        }
                    }
                    Err(e) => {
                        if expanded_booth_id.get_untracked() == Some(booth_id_for_request) {
                            let error_msg = translate_with_params(
                                "report.errors.generate_failed",
                                std::collections::HashMap::from([(
                                    "error",
                                    format_error_message(&e),
                                )]),
                            );
                            toast.error(&error_msg);
                            close_report_modal();
                        }
                    }
                }

                if expanded_booth_id.get_untracked() == Some(booth_id_for_request) {
                    set_is_loading_report.set(false);
                }
            });
        } else if booth_id.is_none() {
            set_expanded_booth_summary.set(None);
            set_is_loading_report.set(false);
        }
    });

    let handle_create_booth = move |data: BoothFormData| {
        let state_result = app_state.get();
        let locale = use_locale().get();

        spawn_local(async move {
            if let Some(Ok(state)) = state_result {
                match data.to_booth(locale) {
                    Ok(booth) => match state
                        .booth_service
                        .create_configured_booth(booth.clone())
                        .await
                    {
                        Ok(_) => {
                            let is_direct_sale = booth.booth_type == BoothType::DirectSale;
                            match state.booth_repository.find_all().await {
                                Ok(mut loaded_booths) => {
                                    if loaded_booths.len() == 1 {
                                        if !booth.is_archived() {
                                            selected_booth.set(Some(booth.clone()));
                                        }
                                        toast.success(
                                            &t!("booth.success.created_and_selected")()
                                                .replace("{description}", &booth.description),
                                        );
                                        if is_direct_sale {
                                            set_editing_booth.set(Some(booth.clone()));
                                            initial_edit_tab.set(1);
                                            set_show_edit_modal.set(true);
                                        }
                                    } else {
                                        set_switch_target_booth.set(Some(booth.clone()));
                                        if is_direct_sale {
                                            open_edit_after_switch.set(true);
                                        }
                                        set_show_switch_modal.set(true);
                                        toast.success(
                                            &t!("booth.success.created")()
                                                .replace("{description}", &booth.description),
                                        );
                                    }

                                    sort_booths(&mut loaded_booths);
                                    set_booths.set(loaded_booths);
                                }
                                Err(_) => {
                                    toast.error(&t!("booth.errors.load_failed")());
                                }
                            }

                            create_step.set(0);
                            set_show_create_modal.set(false);
                            booth_list_version.update(|v| *v += 1);
                        }
                        Err(err) => {
                            toast.error(&translate_domain_error(&err));
                        }
                    },
                    Err(err) => {
                        toast.error(&translate_domain_error(&err));
                    }
                }
            }
        });
    };

    let handle_edit_booth = move |data: BoothFormData| {
        let state_result = app_state.get();
        let booth_to_edit = editing_booth.get();
        let locale = use_locale().get();

        spawn_local(async move {
            if let Some(Ok(state)) = state_result {
                if let Some(mut booth) = booth_to_edit {
                    match data.update_booth(&mut booth, locale) {
                        Ok(_) => match state.booth_service.update_booth(booth.clone()).await {
                            Ok(_) => {
                                toast.success(
                                    &t!("booth.success.updated")()
                                        .replace("{description}", &booth.description),
                                );
                                set_show_edit_modal.set(false);
                                set_editing_booth.set(None);
                                booth_list_version.update(|v| *v += 1);
                                refresh_booths(state.clone());
                            }
                            Err(err) => {
                                toast.error(&translate_domain_error(&err));
                            }
                        },
                        Err(err) => {
                            toast.error(&translate_domain_error(&err));
                        }
                    }
                }
            }
        });
    };

    let handle_copy_booth = move |data: CopyBoothFormData| {
        let state_result = app_state.get();
        let booth_to_copy = copying_booth.get();

        spawn_local(async move {
            if let Some(Ok(state)) = state_result {
                if let Some(source_booth) = booth_to_copy {
                    let Some(new_date) = data.parse_date() else {
                        toast.error(&t!("validation.date_invalid")());
                        return;
                    };

                    match state
                        .booth_service
                        .copy_booth(source_booth.id, data.description.clone(), new_date)
                        .await
                    {
                        Ok(copied) => {
                            match state.booth_repository.find_all().await {
                                Ok(mut loaded_booths) => {
                                    if loaded_booths.len() == 1 {
                                        if !copied.is_archived() {
                                            selected_booth.set(Some(copied.clone()));
                                        }
                                        toast.success(
                                            &t!("booth.success.copied_and_selected")()
                                                .replace("{description}", &copied.description),
                                        );
                                    } else {
                                        set_switch_target_booth.set(Some(copied.clone()));
                                        set_show_switch_modal.set(true);
                                        toast.success(
                                            &t!("booth.success.copied")()
                                                .replace("{description}", &copied.description),
                                        );
                                    }

                                    sort_booths(&mut loaded_booths);
                                    set_booths.set(loaded_booths);
                                }
                                Err(_) => {
                                    toast.error(&t!("booth.errors.load_failed")());
                                }
                            }

                            set_show_copy_modal.set(false);
                            set_copying_booth.set(None);
                            booth_list_version.update(|v| *v += 1);
                        }
                        Err(err) => {
                            toast.error(&translate_domain_error(&err));
                        }
                    }
                }
            }
        });
    };

    let handle_delete_booth = move || {
        let state_result = app_state.get();
        let booth_to_delete = deleting_booth.get();

        spawn_local(async move {
            if let Some(Ok(state)) = state_result {
                if let Some(booth) = booth_to_delete {
                    match state
                        .archive_service
                        .delete_booth_with_cascade(&booth.id)
                        .await
                    {
                        Ok(_) => {
                            if expanded_booth_id.get_untracked() == Some(booth.id) {
                                set_expanded_booth_id.set(None);
                                set_expanded_booth_summary.set(None);
                                set_is_loading_report.set(false);
                            }

                            if let Some(current_booth) = selected_booth.get() {
                                if current_booth.id == booth.id {
                                    selected_booth.set(None);
                                }
                            }

                            toast.success(
                                &t!("booth.success.deleted")()
                                    .replace("{description}", &booth.description),
                            );
                            set_show_delete_confirm.set(false);
                            set_delete_confirmation_token.set(String::new());
                            set_delete_confirmation_input.set(String::new());
                            set_deleting_booth.set(None);
                            booth_list_version.update(|v| *v += 1);

                            match state.booth_repository.find_all().await {
                                Ok(loaded_booths) => {
                                    let mut sorted = loaded_booths;
                                    sort_booths(&mut sorted);
                                    set_booths.set(sorted);
                                }
                                Err(_) => {
                                    toast.error(&t!("booth.errors.load_failed")());
                                }
                            }
                        }
                        Err(_) => {
                            toast.error(&t!("booth.errors_detail.delete_failed")());
                        }
                    }
                }
            }
        });
    };

    let translations = crate::i18n::use_translations();
    let create_booth_title = move || translations.with(|t| t.get("booth.create"));
    let edit_booth_title = move || translations.with(|t| t.get("booth.edit"));

    let close_switch_modal = move || {
        if open_edit_after_switch.get_untracked() {
            if let Some(booth) = switch_target_booth.get_untracked() {
                set_editing_booth.set(Some(booth));
                initial_edit_tab.set(1);
                set_show_edit_modal.set(true);
            }
            open_edit_after_switch.set(false);
        }
        set_show_switch_modal.set(false);
        set_switch_target_booth.set(None);
    };

    let handle_switch_to_booth = move || {
        if let Some(booth) = switch_target_booth.get() {
            if booth.is_archived() {
                toast.error(&t!("archive.cannot_select")());
                close_switch_modal();
                return;
            }

            let description = booth.description.clone();
            selected_booth.set(Some(booth));
            toast.success(&t!("booth.success.selected")().replace("{description}", &description));
            close_switch_modal();
        }
    };

    let switch_message = move || {
        switch_target_booth
            .get()
            .map(|booth| {
                t!("booth.switch_modal.message")().replace("{description}", &booth.description)
            })
            .unwrap_or_else(|| t!("booth.switch_modal.title")())
    };

    let close_delete_modals = move || {
        set_show_delete_confirm.set(false);
        set_delete_confirmation_token.set(String::new());
        set_delete_confirmation_input.set(String::new());
        set_deleting_booth.set(None);
        set_is_checking_delete_requirements.set(false);
    };

    let prompt_delete_booth = move |booth: Booth| {
        let state_result = app_state.get();
        set_deleting_booth.set(Some(booth.clone()));
        set_is_checking_delete_requirements.set(true);
        set_show_delete_confirm.set(false);
        set_delete_confirmation_token.set(String::new());
        set_delete_confirmation_input.set(String::new());

        spawn_local(async move {
            let Some(Ok(state)) = state_result else {
                set_is_checking_delete_requirements.set(false);
                return;
            };

            match state.purchase_repository.find_by_booth(&booth.id).await {
                Ok(purchases) => {
                    set_is_checking_delete_requirements.set(false);

                    if purchases.is_empty() {
                        set_show_delete_confirm.set(true);
                    } else {
                        set_delete_confirmation_token.set(confirmation_token_from_booth(&booth.id));
                        set_delete_confirmation_input.set(String::new());
                        set_timeout(
                            move || focus_and_select_input(&delete_confirmation_ref),
                            std::time::Duration::from_millis(0),
                        );
                    }
                }
                Err(_) => {
                    set_is_checking_delete_requirements.set(false);
                    set_deleting_booth.set(None);
                    toast.error(&t!("booth.errors_detail.delete_failed")());
                }
            }
        });
    };

    let delete_message = move || {
        deleting_booth
            .get()
            .map(|b| t!("booth.delete_confirm_message")().replace("{description}", &b.description))
            .unwrap_or_else(|| t!("booth.delete_confirm")())
    };

    let report_modal_title = move || {
        expanded_booth_id.get().and_then(|booth_id| {
            booths
                .get()
                .into_iter()
                .find(|booth| booth.id == booth_id)
                .map(|booth| {
                    format!(
                        "{} - {}",
                        booth.description,
                        if booth.is_archived() {
                            t!("archive.summary_title")()
                        } else {
                            t!("report.booth_summary_report")()
                        }
                    )
                })
        })
    };

    let handle_print = move |_| {
        set_timeout(
            move || {
                if let Some(window) = window() {
                    let _ = window.print();
                }
            },
            std::time::Duration::from_millis(100),
        );
    };

    let print_header_action = move || {
        view! {
            <button
                type="button"
                on:click=handle_print
                class="inline-flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                title={t!("report.print_report")()}
                aria-label={t!("report.print_report")()}
            >
                <Icon icon=LuPrinter class="h-4 w-4" />
                <span>{t!("report.print_report")}</span>
            </button>
        }
            .into_any()
    };

    let close_archive_modal = move || {
        set_show_archive_modal.set(false);
        set_archiving_booth.set(None);
    };

    let handle_archive_completed = move || {
        if let Some(booth) = archiving_booth.get() {
            if let Some(selected) = selected_booth.get() {
                if selected.id == booth.id {
                    selected_booth.set(None);
                }
            }
        }
        close_archive_modal();
        if let Some(Ok(state)) = app_state.get() {
            refresh_booths(state);
        }
    };

    let open_archive_modal = move |booth: Booth| {
        set_archiving_booth.set(Some(booth));
        set_show_archive_modal.set(true);
    };

    let active_booth_cards = move || {
        let (active_booths, _) = booth_sections.get();
        let counts = vendor_counts.get();
        let pgc = product_group_counts.get();
        active_booths
            .into_iter()
            .map(|booth| {
                let vc = *counts.get(&booth.id).unwrap_or(&0);
                let pgcount = *pgc.get(&booth.id).unwrap_or(&0);
                booth_card_view(
                    booth,
                    false,
                    vc,
                    pgcount,
                    format_date,
                    set_copying_booth,
                    set_show_copy_modal,
                    set_editing_booth,
                    set_show_edit_modal,
                    set_expanded_booth_summary,
                    set_expanded_booth_id,
                    prompt_delete_booth,
                    open_archive_modal,
                    initial_edit_tab,
                )
            })
            .collect_view()
    };

    let archived_booth_cards = move || {
        let (_, archived_booths) = booth_sections.get();
        archived_booths
            .into_iter()
            .map(|booth| {
                let vc = booth
                    .archived_summary
                    .as_ref()
                    .map(|s| s.vendor_count)
                    .unwrap_or(0);
                booth_card_view(
                    booth,
                    true,
                    vc,
                    0,
                    format_date,
                    set_copying_booth,
                    set_show_copy_modal,
                    set_editing_booth,
                    set_show_edit_modal,
                    set_expanded_booth_summary,
                    set_expanded_booth_id,
                    prompt_delete_booth,
                    open_archive_modal,
                    initial_edit_tab,
                )
            })
            .collect_view()
    };

    // Open the dedup modal: load vendor lists per group for the diff view
    let open_dedup_modal = move || {
        let state_result = app_state.get();
        if let Some(Ok(state)) = state_result {
            let groups = duplicate_groups.get();
            spawn_local(async move {
                let mut detail: Vec<(Vec<Booth>, Vec<Vec<Vendor>>)> = Vec::new();
                for group in groups {
                    let mut vendor_lists: Vec<Vec<Vendor>> = Vec::new();
                    for booth in &group {
                        let vendors = state
                            .vendor_repository
                            .find_by_booth(&booth.id)
                            .await
                            .unwrap_or_default();
                        vendor_lists.push(vendors);
                    }
                    detail.push((group, vendor_lists));
                }
                set_dedup_detail.set(detail);
                set_show_dedup_modal.set(true);
            });
        }
    };

    let handle_merge_group = move |canonical_id: BoothId, other_id: BoothId| {
        let state_result = app_state.get();
        if let Some(Ok(state)) = state_result {
            set_is_merging.set(true);
            spawn_local(async move {
                match state
                    .merge_service
                    .merge_booths(&canonical_id, &other_id)
                    .await
                {
                    Ok(()) => {
                        // Reload booths and duplicate groups
                        let mut booths_after =
                            state.booth_repository.find_all().await.unwrap_or_default();
                        sort_booths(&mut booths_after);
                        set_booths.set(booths_after);

                        if let Ok(groups) = state.booth_repository.find_duplicate_groups().await {
                            set_duplicate_groups.set(groups.clone());
                            if groups.is_empty() {
                                set_show_dedup_modal.set(false);
                            } else {
                                // Refresh modal detail
                                let mut detail: Vec<(Vec<Booth>, Vec<Vec<Vendor>>)> = Vec::new();
                                for group in groups {
                                    let mut vendor_lists: Vec<Vec<Vendor>> = Vec::new();
                                    for booth in &group {
                                        let vendors = state
                                            .vendor_repository
                                            .find_by_booth(&booth.id)
                                            .await
                                            .unwrap_or_default();
                                        vendor_lists.push(vendors);
                                    }
                                    detail.push((group, vendor_lists));
                                }
                                set_dedup_detail.set(detail);
                            }
                        }

                        // Refresh counts
                        if let Ok(all_vendors) = state.vendor_repository.find_all().await {
                            let mut vc = HashMap::<BoothId, usize>::new();
                            for v in all_vendors {
                                *vc.entry(v.booth_id).or_insert(0) += 1;
                            }
                            set_vendor_counts.set(vc);
                        }
                        if let Ok(all_purchases) = state.purchase_repository.find_all().await {
                            let mut pc = HashMap::<BoothId, usize>::new();
                            for p in all_purchases {
                                *pc.entry(p.booth_id).or_insert(0) += 1;
                            }
                            set_purchase_counts.set(pc);
                        }
                    }
                    Err(_) => {
                        toast.error("Merge failed. Please try again.");
                    }
                }
                set_is_merging.set(false);
            });
        }
    };

    view! {
        <>
            <div class="print:hidden">
                <div class="fixed left-0 right-0 top-36 z-20 bg-gray-50">
                    <div class="mx-auto max-w-7xl px-4 py-3 sm:px-6 lg:px-8">
                        <div class="space-y-4">

                            <Show when=move || !booths.get().is_empty()>
                                <div class="max-w-md">
                                    <input
                                        type="search"
                                        class="w-full rounded-lg border border-gray-300 px-4 py-2 text-sm text-gray-900 shadow-sm focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                        placeholder=t!("common.search_placeholder")()
                                        aria-label=t!("common.search_placeholder")()
                                        prop:value=move || booth_search_query.get()
                                        on:input=move |ev| {
                                            set_booth_search_query.set(event_target_value(&ev));
                                        }
                                    />
                                </div>
                            </Show>
                        </div>
                    </div>
                </div>

                <Container>
                    <div class="pb-40 pt-20 sm:pt-16">

                        <Show
                            when=move || is_loading.get()
                            fallback=move || {
                                view! {
                                    <Show
                                        when=move || booths.get().is_empty()
                                        fallback=move || {
                                            view! {
                                                <div class="space-y-8">

                                                    <Show
                                                        when=move || {
                                                            booth_search_query.get().trim().is_empty()
                                                                || !booth_sections.get().0.is_empty()
                                                                || !booth_sections.get().1.is_empty()
                                                        }
                                                        fallback=move || {
                                                            view! {
                                                                <Card>
                                                                    <div class="px-6 py-12 text-center text-gray-600">
                                                                        {t!("common.no_results")()}
                                                                    </div>
                                                                </Card>
                                                            }
                                                        }
                                                    >
                                                        <div class="space-y-8">
                                                            <Show when=move || !duplicate_groups.get().is_empty()>
                                                                <div class="flex items-center justify-between gap-3 rounded-lg border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-900">
                                                                    <div class="flex items-center gap-2">
                                                                        <Icon icon=LuAlertTriangle class="h-4 w-4 shrink-0 text-amber-600" />
                                                                        <span>
                                                                            {move || {
                                                                                let n = duplicate_groups.get().len();
                                                                                format!("{} event{} share the same name and date. Review duplicates?", n, if n == 1 { "" } else { "s" })
                                                                            }}
                                                                        </span>
                                                                    </div>
                                                                    <button
                                                                        type="button"
                                                                        class="shrink-0 rounded-md border border-amber-400 bg-amber-100 px-3 py-1.5 text-xs font-medium text-amber-900 transition-colors hover:bg-amber-200 focus:outline-none focus:ring-2 focus:ring-amber-500"
                                                                        on:click=move |_| open_dedup_modal()
                                                                    >
                                                                        "Review duplicates"
                                                                    </button>
                                                                </div>
                                                            </Show>

                                                            <section class="space-y-4">
                                                                <div class="flex items-center justify-between gap-4">
                                                                    <div>
                                                                        <h2 class="text-lg font-semibold text-slate-900">{t!("booth.active_section_title")()}</h2>
                                                                        <p class="text-sm text-gray-600">{t!("booth.active_section_description")()}</p>
                                                                    </div>
                                                                </div>
                                                                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
                                                                    {active_booth_cards()}
                                                                </div>
                                                            </section>

                                                            <Show when=move || !booth_sections.get().1.is_empty()>
                                                                <section class="space-y-4 border-t border-gray-200 pt-8">
                                                                    <div class="flex items-center justify-between gap-4">
                                                                        <div>
                                                                            <h2 class="text-lg font-semibold text-slate-900">{t!("booth.archived_section_title")()}</h2>
                                                                            <p class="text-sm text-gray-600">{t!("booth.archived_section_description")()}</p>
                                                                        </div>
                                                                        <Button
                                                                            variant=ButtonVariant::Secondary
                                                                            on_click=Box::new(move || set_show_archived_section.update(|value| *value = !*value))
                                                                        >
                                                                            {move || if show_archived_section.get() {
                                                                                t!("archive.hide_archived")()
                                                                            } else {
                                                                                t!("archive.show_archived")()
                                                                            }}
                                                                        </Button>
                                                                    </div>
                                                                    <Show when=move || show_archived_section.get()>
                                                                        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
                                                                            {archived_booth_cards()}
                                                                        </div>
                                                                    </Show>
                                                                </section>
                                                            </Show>
                                                        </div>
                                                    </Show>
                                                </div>
                                            }
                                        }
                                    >
                                        <Card>
                                            <div class="px-6 py-12 text-center">
                                                <p class="mb-4 text-lg font-semibold text-gray-700">
                                                    {t!("booth.empty_state.heading")}
                                                </p>
                                                <p class="mb-3 text-gray-600">
                                                    <span class="font-medium text-gray-800">
                                                        {t!("booth.empty_state.requirement")}
                                                    </span>
                                                    " "
                                                    {t!("booth.empty_state.features")}
                                                </p>
                                                <p class="mb-6 text-gray-600">
                                                    {t!("booth.empty_state.get_started")}
                                                </p>
                                                <Button
                                                    on_click=Box::new(move || set_show_create_modal.set(true))
                                                >
                                                    {t!("booth.create")}
                                                </Button>

                                                <div class="my-8 flex items-center gap-3 text-sm text-gray-400">
                                                    <div class="h-px flex-1 bg-gray-200"></div>
                                                    <span class="font-medium uppercase tracking-wide">
                                                        {t!("booth.empty_state.or")}
                                                    </span>
                                                    <div class="h-px flex-1 bg-gray-200"></div>
                                                </div>

                                                <div class="mx-auto max-w-xl">
                                                    <p class="mb-4 text-lg font-semibold text-gray-700">
                                                        {t!("booth.empty_state.migration_heading")}
                                                    </p>
                                                    <p class="mb-6 text-gray-600">
                                                        {t!("booth.empty_state.migration_body")}
                                                    </p>
                                                    <Button
                                                        on_click=Box::new(move || {
                                                            if let Some(window) = window() {
                                                                let _ = window
                                                                    .location()
                                                                    .set_href("/settings?tab=migration");
                                                            }
                                                        })
                                                        variant=ButtonVariant::Secondary
                                                    >
                                                        {t!("booth.empty_state.migration_button")}
                                                    </Button>
                                                </div>
                                            </div>
                                        </Card>
                                    </Show>
                                }
                            }
                        >
                            <Card>
                                <div class="text-center py-12">
                                    <p class="text-gray-600">{t!("booth.loading_message")}</p>
                                </div>
                            </Card>
                        </Show>
                    </div>

                    <Show when=move || expanded_booth_id.get().is_some()>
                        {move || {
                            let modal_title = report_modal_title().unwrap_or_else(|| t!("report.booth_summary_report")());

                            view! {
                                <Modal
                                    show=Signal::derive(move || expanded_booth_id.get().is_some())
                                    on_close=close_report_modal
                                    title=Signal::derive(move || modal_title.clone())
                                    header_actions=print_header_action()
                                    size=ModalSize::XLarge
                                >
                                    <div class="space-y-6">
                                        <Show
                                            when=move || is_loading_report.get()
                                            fallback=move || {
                                                view! {
                                                    {move || {
                                                        expanded_booth_id.get().and_then(|booth_id| {
                                                            booths
                                                                .get()
                                                                .into_iter()
                                                                .find(|booth| booth.id == booth_id)
                                                                .map(|booth| {
                                                                    if booth.is_archived() {
                                                                        view! {
                                                                            <ArchivedBoothSummaryDisplay booth=booth />
                                                                        }
                                                                        .into_any()
                                                                    } else {
                                                                        view! {
                                                                            <Show when=move || expanded_booth_summary.get().is_some()>
                                                                                {move || {
                                                                                    expanded_booth_summary.get().map(|summary| {
                                                                                        view! {
                                                                                            <BoothSummaryDisplay summary=summary />
                                                                                        }
                                                                                    })
                                                                                }}
                                                                            </Show>
                                                                        }
                                                                        .into_any()
                                                                    }
                                                                })
                                                        })
                                                    }}
                                                }
                                            }
                                        >
                                            <div class="flex items-center justify-center py-12">
                                                <div class="text-center">
                                                    <div class="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
                                                    <p class="mt-4 text-gray-600">{t!("report.loading")}</p>
                                                </div>
                                            </div>
                                        </Show>
                                    </div>
                                </Modal>
                            }
                        }}
                    </Show>

                    <div class="fixed bottom-28 right-6 z-50 print:hidden">
                        <button
                            type="button"
                            class="inline-flex h-14 items-center gap-2 rounded-full bg-teal-600 px-5 text-white shadow-xl transition-all hover:scale-105 hover:bg-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-500 focus:ring-offset-2"
                            on:click=move |_| set_show_create_modal.set(true)
                            title={t!("booth.create")()}
                            aria-label={t!("booth.create_aria_label")()}
                        >
                            <Icon icon=LuPlus class="h-5 w-5 shrink-0" />
                            <span class="hidden sm:inline">{t!("booth.create")}</span>
                        </button>
                    </div>

                    {move || archiving_booth.get().map(|booth| {
                        view! {
                            <ArchiveWizard
                                booth=booth
                                show=Signal::derive(move || show_archive_modal.get())
                                on_close=close_archive_modal
                                on_archived=handle_archive_completed
                            />
                        }
                    })}

                    <Modal
                        show=show_create_modal
                        on_close=move || {
                            create_step.set(0);
                            set_show_create_modal.set(false);
                        }
                        title=Signal::derive(create_booth_title)
                        size=ModalSize::XLarge
                        action_bar=
                            view! {
                                <div class="contents">
                                    <Button
                                        on_click=Box::new(move || {
                                            create_step.set(0);
                                            set_show_create_modal.set(false);
                                        })
                                        variant=ButtonVariant::Secondary
                                    >
                                        {t!("common.cancel")()}
                                    </Button>
                                    <Show when=move || create_step.get() == 1>
                                        <Button
                                            variant=ButtonVariant::Primary
                                            button_type="submit".to_string()
                                            form="create-booth-form".to_string()
                                        >
                                            {t!("booth.save_button")()}
                                        </Button>
                                    </Show>
                                </div>
                            }
                            .into_any()
                    >
                        {move || {
                            if !show_create_modal.get() {
                                return None;
                            }
                            Some(view! {
                                {move || {
                                    if create_step.get() == 0 {
                                        view! {
                                            <div class="space-y-4">
                                                <p class="text-sm text-gray-600">
                                                    {t!("booth.create_select_type_title")()}
                                                </p>
                                                <div class="grid grid-cols-2 gap-4">
                                                    <button
                                                        type="button"
                                                        class="rounded-lg border-2 border-gray-200 p-6 text-left transition-colors hover:border-blue-400 hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                                        on:click=move |_| {
                                                            selected_create_type.set(BoothType::DirectSale);
                                                            create_step.set(1);
                                                        }
                                                    >
                                                        <p class="text-base font-semibold text-gray-900">
                                                            {t!("booth.type_direct_sale")()}
                                                        </p>
                                                        <p class="mt-1 text-sm text-gray-500">
                                                            {t!("booth.type_direct_sale_description")()}
                                                        </p>
                                                    </button>
                                                    <button
                                                        type="button"
                                                        class="rounded-lg border-2 border-gray-200 p-6 text-left transition-colors hover:border-blue-400 hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                                        on:click=move |_| {
                                                            selected_create_type.set(BoothType::ThirdPartySale);
                                                            create_step.set(1);
                                                        }
                                                    >
                                                        <p class="text-base font-semibold text-gray-900">
                                                            {t!("booth.type_third_party_sale")()}
                                                        </p>
                                                        <p class="mt-1 text-sm text-gray-500">
                                                            {t!("booth.type_third_party_sale_description")()}
                                                        </p>
                                                    </button>
                                                </div>
                                            </div>
                                        }.into_any()
                                    } else {
                                        let current_locale = locale.get();
                                        let mut initial_data = BoothFormData::default_with_locale(current_locale);
                                        initial_data.booth_type = selected_create_type.get_untracked();
                                        view! {
                                            <BoothForm
                                                form_id="create-booth-form".to_string()
                                                autofocus_description=true
                                                initial_data=initial_data
                                                on_submit=handle_create_booth
                                            />
                                        }.into_any()
                                    }
                                }}
                            })
                        }}
                    </Modal>

                    <Modal
                        show=show_edit_modal
                        on_close=move || {
                            set_show_edit_modal.set(false);
                            set_editing_booth.set(None);
                        }
                        title=Signal::derive(edit_booth_title)
                        size=ModalSize::XLarge
                        action_bar=
                            view! {
                                <div class="contents">
                                    <Button
                                        on_click=Box::new(move || {
                                            set_show_edit_modal.set(false);
                                            set_editing_booth.set(None);
                                        })
                                        variant=ButtonVariant::Secondary
                                    >
                                        {t!("common.cancel")()}
                                    </Button>
                                    <Button
                                        variant=ButtonVariant::Primary
                                        button_type="submit".to_string()
                                        form="edit-booth-form".to_string()
                                    >
                                        {t!("booth.save_button")()}
                                    </Button>
                                </div>
                            }
                            .into_any()
                    >
                        {move || editing_booth.get().map(|booth| {
                            let current_locale = locale.get();
                            let initial_data = BoothFormData::from_booth(&booth, current_locale);
                            let tab = initial_edit_tab.get_untracked();
                            initial_edit_tab.set(0);
                            let pc = *purchase_counts.get_untracked().get(&booth.id).unwrap_or(&0);
                            view! {
                                <BoothForm
                                    form_id="edit-booth-form".to_string()
                                    initial_data=initial_data
                                    on_submit=handle_edit_booth
                                    initial_tab=tab
                                    booth_type_locked={pc > 0}
                                />
                            }
                        })}
                    </Modal>

                    <Modal
                        show=show_copy_modal
                        on_close=move || {
                            set_show_copy_modal.set(false);
                            set_copying_booth.set(None);
                        }
                        title=Signal::derive(move || t!("booth.copy_title")())
                        size=ModalSize::Medium
                        action_bar=
                            view! {
                                <div class="contents">
                                    <Button
                                        on_click=Box::new(move || {
                                            set_show_copy_modal.set(false);
                                            set_copying_booth.set(None);
                                        })
                                        variant=ButtonVariant::Secondary
                                    >
                                        {t!("common.cancel")()}
                                    </Button>
                                    <Button
                                        variant=ButtonVariant::Primary
                                        button_type="submit".to_string()
                                        form="copy-booth-form".to_string()
                                    >
                                        {t!("booth.copy_confirm")()}
                                    </Button>
                                </div>
                            }
                            .into_any()
                    >
                        {move || copying_booth.get().map(|booth| {
                            view! {
                                <CopyBoothDialog
                                    source_booth=booth
                                    form_id="copy-booth-form".to_string()
                                    autofocus_description=true
                                    on_submit=handle_copy_booth
                                />
                            }
                        })}
                    </Modal>

                    <ConfirmModal
                        show=show_switch_modal
                        on_close=close_switch_modal
                        on_confirm=handle_switch_to_booth
                        title=Signal::derive(move || t!("booth.switch_modal.title")())
                        message=Signal::derive(switch_message)
                        confirm_text=Signal::derive(move || t!("booth.switch_modal.confirm")())
                        cancel_text=Signal::derive(move || t!("booth.switch_modal.cancel")())
                    />

                    <ConfirmModal
                        show=show_delete_confirm
                        on_close=close_delete_modals
                        on_confirm=handle_delete_booth
                        title=Signal::derive(move || t!("booth.delete")())
                        message=Signal::derive(delete_message)
                        confirm_text=Signal::derive(move || t!("common.delete")())
                        cancel_text=Signal::derive(move || t!("common.cancel")())
                        is_destructive=true
                    />

                    <Modal
                        show=Signal::derive(move || {
                            is_checking_delete_requirements.get()
                                || (!delete_confirmation_token.get().is_empty()
                                    && deleting_booth.get().is_some())
                        })
                        on_close=close_delete_modals
                        title=Signal::derive(move || {
                            if is_checking_delete_requirements.get() {
                                t!("booth.delete")()
                            } else {
                                t!("booth.delete_modal.title")()
                            }
                        })
                        size=ModalSize::Medium
                        action_bar=
                            view! {
                                <div class="contents">
                                    <Show when=move || !is_checking_delete_requirements.get()>
                                        <Button
                                            variant=ButtonVariant::Secondary
                                            on_click=Box::new(close_delete_modals)
                                        >
                                            {t!("common.cancel")}
                                        </Button>
                                        <Button
                                            variant=ButtonVariant::Danger
                                            disabled=!deletion_token_matches.get()
                                            on_click=Box::new(handle_delete_booth)
                                        >
                                            {t!("booth.delete_modal.confirm")}
                                        </Button>
                                    </Show>
                                </div>
                            }
                            .into_any()
                    >
                        <Show
                            when=move || is_checking_delete_requirements.get()
                            fallback=move || {
                                view! {
                                    <Show when=move || deleting_booth.get().is_some()>
                                        <div class="space-y-4">
                                            <p class="text-gray-700">
                                                {move || {
                                                    translate_with_params(
                                                        "booth.delete_modal.instructions",
                                                        HashMap::from([(
                                                            "token",
                                                            delete_confirmation_token.get(),
                                                        )]),
                                                    )
                                                }}
                                            </p>
                                            <input
                                                class="w-full rounded-lg border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-red-500"
                                                placeholder=t!("booth.delete_modal.placeholder")()
                                                value=move || delete_confirmation_input.get()
                                                node_ref=delete_confirmation_ref
                                                on:input=move |ev| {
                                                    set_delete_confirmation_input
                                                        .set(event_target_value(&ev));
                                                }
                                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                    if ev.key() == "Enter" && deletion_token_matches.get() {
                                                        ev.prevent_default();
                                                        handle_delete_booth();
                                                    }
                                                }
                                            />
                                        </div>
                                    </Show>
                                }
                            }
                        >
                            <div class="flex items-center justify-center py-12">
                                <div class="text-center">
                                    <div class="inline-block h-12 w-12 animate-spin rounded-full border-b-2 border-blue-600"></div>
                                    <p class="mt-4 text-gray-600">{t!("common.loading")}</p>
                                </div>
                            </div>
                        </Show>
                    </Modal>
                </Container>
            </div>

            <div class="hidden print:block">
                <Show when=move || expanded_booth_summary.get().is_some()>
                    {move || {
                        expanded_booth_summary.get().and_then(|summary| {
                            expanded_booth_id.get().and_then(|booth_id| {
                                booths
                                    .get()
                                    .into_iter()
                                    .find(|booth| booth.id == booth_id)
                                    .map(|booth| {
                                        view! {
                                            <PrintBoothSummary
                                                summary=summary
                                                booth_name=booth.description.clone()
                                                booth_date=booth.date
                                            />
                                        }
                                    })
                            })
                        })
                    }}
                </Show>

                <Show when=move || expanded_booth_id.get().is_some() && expanded_booth_summary.get().is_none()>
                    {move || {
                        expanded_booth_id.get().and_then(|booth_id| {
                            booths
                                .get()
                                .into_iter()
                                .find(|booth| booth.id == booth_id && booth.is_archived())
                                .map(|booth| {
                                    view! {
                                        <PrintArchivedBoothSummary booth=booth />
                                    }
                                })
                        })
                    }}
                </Show>
            </div>

            {dedup_modal_view(
                show_dedup_modal,
                set_show_dedup_modal,
                dedup_detail,
                purchase_counts,
                is_merging,
                handle_merge_group,
                format_date,
            )}
        </>
    }
}

#[allow(clippy::too_many_arguments)]
fn dedup_modal_view(
    show: ReadSignal<bool>,
    set_show: WriteSignal<bool>,
    detail: ReadSignal<Vec<(Vec<Booth>, Vec<Vec<Vendor>>)>>,
    purchase_counts: ReadSignal<HashMap<BoothId, usize>>,
    is_merging: ReadSignal<bool>,
    handle_merge: impl Fn(BoothId, BoothId) + Copy + Send + Sync + 'static,
    format_date: impl Fn(chrono::NaiveDate) -> String + Copy + Send + Sync + 'static,
) -> AnyView {
    let locale = use_locale();
    view! {
        <Show when=move || show.get()>
            <div
                class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
                on:click=move |ev| {
                    use wasm_bindgen::JsCast;
                    if ev.target().and_then(|t| t.dyn_into::<web_sys::EventTarget>().ok())
                        == ev.current_target().and_then(|t| t.dyn_into::<web_sys::EventTarget>().ok()) {
                        set_show.set(false);
                    }
                }
            >
                <div class="max-h-[85vh] w-full max-w-2xl overflow-y-auto rounded-xl bg-white shadow-2xl">
                    <div class="flex items-center justify-between border-b border-gray-200 px-6 py-4">
                        <h2 class="text-lg font-semibold text-gray-900">"Duplicate events"</h2>
                        <button
                            type="button"
                            class="rounded-lg p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
                            on:click=move |_| set_show.set(false)
                        >
                            <Icon icon=LuX class="h-5 w-5" />
                        </button>
                    </div>
                    <div class="divide-y divide-gray-100 px-6 py-4">
                        {move || {
                            let groups = detail.get();
                            let pc = purchase_counts.get();
                            groups.into_iter().map(|(group, vendor_lists)| {
                                let canonical_idx = group
                                    .iter()
                                    .enumerate()
                                    .max_by_key(|(_, b)| {
                                        let count = *pc.get(&b.id).unwrap_or(&0);
                                        (count, std::cmp::Reverse(b.created_at))
                                    })
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                let canonical_id = group[canonical_idx].id;
                                let other_ids: Vec<BoothId> = group
                                    .iter()
                                    .enumerate()
                                    .filter(|(i, _)| *i != canonical_idx)
                                    .map(|(_, b)| b.id)
                                    .collect();
                                let vendor_id_sets: Vec<std::collections::HashSet<String>> = vendor_lists
                                    .iter()
                                    .map(|vs| vs.iter().map(|v| v.vendor_id.to_string()).collect())
                                    .collect();

                                let shared: Vec<String> = if vendor_id_sets.len() >= 2 {
                                    let set0 = &vendor_id_sets[0];
                                    set0.iter()
                                        .filter(|id| vendor_id_sets[1..].iter().all(|s| s.contains(*id)))
                                        .cloned()
                                        .collect()
                                } else { vec![] };
                                let unique_per_side: Vec<Vec<String>> = vendor_id_sets
                                    .iter()
                                    .enumerate()
                                    .map(|(i, s)| {
                                        s.iter()
                                            .filter(|id| !vendor_id_sets.iter().enumerate()
                                                .any(|(j, other)| j != i && other.contains(*id)))
                                            .cloned()
                                            .collect()
                                    })
                                    .collect();

                                let event_name = group[0].description.clone();
                                let event_date = group[0].date;
                                let delete_count = group.len() - 1;

                                let candidate_cards: Vec<AnyView> = group.iter().enumerate().map(|(i, booth)| {
                                    let vc = vendor_id_sets[i].len();
                                    let pcount = *pc.get(&booth.id).unwrap_or(&0);
                                    let is_canonical = i == canonical_idx;
                                    let border_class = if is_canonical { "border-blue-300 bg-blue-50" } else { "border-gray-200 bg-gray-50" };
                                    let label = if is_canonical { "Will survive" } else { "Will be deleted" };
                                    let updated = format_datetime(booth.updated_at, locale.get());
                                    view! {
                                        <div class=format!("rounded-lg border p-3 text-sm {}", border_class)>
                                            <div class="flex items-center justify-between">
                                                <span class="font-medium text-gray-700">{label}</span>
                                                {if is_canonical {
                                                    view! { <span class="rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">"canonical"</span> }.into_any()
                                                } else {
                                                    view! { <span /> }.into_any()
                                                }}
                                            </div>
                                            <p class="mt-1 text-gray-600">
                                                {format!("{} vendor{} · {} purchase{}", vc, if vc == 1 { "" } else { "s" }, pcount, if pcount == 1 { "" } else { "s" })}
                                            </p>
                                            <p class="text-xs text-gray-400">{"Updated: "}{updated}</p>
                                        </div>
                                    }.into_any()
                                }).collect();

                                let diff_view: AnyView = if !shared.is_empty() || unique_per_side.iter().any(|u| !u.is_empty()) {
                                    let shared_str = if shared.is_empty() { String::new() } else { format!("Shared: {}", shared.join(", ")) };
                                    let unique_strs: Vec<String> = unique_per_side.iter().enumerate()
                                        .filter(|(_, u)| !u.is_empty())
                                        .map(|(i, u)| format!("Only in booth {}: {}", i + 1, u.join(", ")))
                                        .collect();
                                    view! {
                                        <div class="mb-3 rounded-md bg-gray-50 p-3 text-xs text-gray-600 space-y-1">
                                            {if shared_str.is_empty() { view! { <span /> }.into_any() } else { view! { <p class="font-medium">{shared_str}</p> }.into_any() }}
                                            {unique_strs.into_iter().map(|s| view! { <p>{s}</p> }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                };

                                let other_ids_for_button = other_ids.clone();
                                view! {
                                    <div class="py-5">
                                        <p class="mb-3 font-medium text-gray-900">
                                            {event_name} " — " {format_date(event_date)}
                                        </p>
                                        <div class="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2">
                                            {candidate_cards}
                                        </div>
                                        {diff_view}
                                        <div class="space-y-2">
                                            <p class="text-xs text-amber-700">
                                                {format!("{} event{} will be permanently deleted. This cannot be undone.",
                                                    delete_count, if delete_count == 1 { "" } else { "s" })}
                                            </p>
                                            <button
                                                type="button"
                                                disabled=move || is_merging.get()
                                                class="rounded-lg bg-amber-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-amber-700 disabled:opacity-50 focus:outline-none focus:ring-2 focus:ring-amber-500"
                                                on:click=move |_| {
                                                    for other_id in &other_ids_for_button {
                                                        handle_merge(canonical_id, *other_id);
                                                    }
                                                }
                                            >
                                                {move || if is_merging.get() { "Merging…" } else { "Merge into one event" }}
                                            </button>
                                        </div>
                                    </div>
                                }.into_any()
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                </div>
            </div>
        </Show>
    }.into_any()
}

fn booth_card_view(
    booth: Booth,
    is_archived: bool,
    vendor_count: usize,
    product_group_count: usize,
    format_date: impl Fn(chrono::NaiveDate) -> String + Copy + Send + Sync + 'static,
    set_copying_booth: WriteSignal<Option<Booth>>,
    set_show_copy_modal: WriteSignal<bool>,
    set_editing_booth: WriteSignal<Option<Booth>>,
    set_show_edit_modal: WriteSignal<bool>,
    set_expanded_booth_summary: WriteSignal<Option<BoothSummary>>,
    set_expanded_booth_id: WriteSignal<Option<BoothId>>,
    prompt_delete_booth: impl Fn(Booth) + Copy + Send + Sync + 'static,
    open_archive_modal: impl Fn(Booth) + Copy + Send + Sync + 'static,
    initial_edit_tab: RwSignal<usize>,
) -> AnyView {
    let booth_description = StoredValue::new_local(booth.description.clone());
    let booth_date = booth.date;
    let booth_updated_at = booth.updated_at;
    let booth_archived_at = booth.archived_at;
    let booth_id = booth.id;
    let booth_id_for_report = booth.id;
    let booth_type = booth.booth_type;
    let is_direct_sale = booth_type == BoothType::DirectSale;
    let show_no_products_badge = is_direct_sale && product_group_count == 0 && !is_archived;
    let locale = use_locale();
    let archived_timestamp = Memo::new(move |_| {
        booth_archived_at.map(|timestamp| {
            t!("archive.archived_at_label")() + ": " + &format_datetime(timestamp, locale.get())
        })
    });
    let booth_for_edit = StoredValue::new_local(booth.clone());
    let booth_for_copy = StoredValue::new_local(booth.clone());
    let booth_for_delete = StoredValue::new_local(booth.clone());
    let booth_for_archive = StoredValue::new_local(booth.clone());
    let booth_for_products = StoredValue::new_local(booth.clone());
    let archived_class = if is_archived {
        "border-slate-300 bg-slate-50"
    } else {
        "border-gray-200 bg-white hover:-translate-y-0.5 hover:border-blue-300 hover:shadow-lg"
    };
    let title_text_class = if is_archived {
        "text-gray-500"
    } else {
        "text-gray-900"
    };
    let date_text_class = if is_archived {
        "text-gray-400"
    } else {
        "text-gray-600"
    };

    view! {
        <article
            class=format!(
                "booth-card group relative h-full rounded-lg p-6 shadow-md transition-all duration-200 {}",
                archived_class
            )
            aria_label=booth_description.get_value()
            role="region"
        >
            <div class="absolute right-4 top-4">
                <DropdownMenu
                    trigger=view! {
                        <Button
                            variant=ButtonVariant::Secondary
                            class="h-12 w-12 p-0".to_string()
                            title=t!("booth.actions_menu")()
                            aria_label=t!("booth.actions_menu_aria")()
                        >
                            <Icon icon=LuMoreVertical class="h-7 w-7" />
                        </Button>
                    }.into_any()
                >
                    <ExportButton scope=ExportScope::Booth(booth_id) menu_item=true />
                    <DropdownMenuItem
                        on_click=Callback::new(move |_| {
                            set_copying_booth.set(Some(booth_for_copy.get_value()));
                            set_show_copy_modal.set(true);
                        })
                    >
                        {t!("booth.copy_button")()}
                    </DropdownMenuItem>
                    <Show when=move || !is_archived>
                        <DropdownMenuItem
                            on_click=Callback::new(move |_| {
                                initial_edit_tab.set(0);
                                set_editing_booth.set(Some(booth_for_edit.get_value()));
                                set_show_edit_modal.set(true);
                            })
                        >
                            {t!("booth.edit_button")()}
                        </DropdownMenuItem>
                    </Show>
                    {if is_direct_sale && !is_archived {
                        view! {
                            <DropdownMenuItem
                                on_click=Callback::new(move |_| {
                                    initial_edit_tab.set(1);
                                    set_editing_booth.set(Some(booth_for_products.get_value()));
                                    set_show_edit_modal.set(true);
                                })
                            >
                                {t!("booth.configure_products")()}
                            </DropdownMenuItem>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    <DropdownMenuItem
                        on_click=Callback::new(move |_| {
                            set_expanded_booth_summary.set(None);
                            set_expanded_booth_id.set(Some(booth_id_for_report));
                        })
                    >
                        {move || if is_archived { t!("archive.view_summary")() } else { t!("booth.view_report")() }}
                    </DropdownMenuItem>
                    <Show when=move || !is_archived>
                        <DropdownMenuItem
                            on_click=Callback::new(move |_| open_archive_modal(booth_for_archive.get_value()))
                            class="text-amber-700 hover:bg-amber-50 hover:text-amber-800 focus:bg-amber-50 focus:text-amber-800".to_string()
                        >
                            {t!("archive.archive_button")()}
                        </DropdownMenuItem>
                    </Show>
                    <Show when=move || !is_archived>
                        <div class="my-1 h-px bg-gray-200"></div>
                        <DropdownMenuItem
                            on_click=Callback::new(move |_| prompt_delete_booth(booth_for_delete.get_value()))
                            class="text-red-600 hover:bg-red-50 hover:text-red-700 focus:bg-red-50 focus:text-red-700".to_string()
                        >
                            {t!("booth.delete_button")()}
                        </DropdownMenuItem>
                    </Show>
                </DropdownMenu>
            </div>
            <div class="flex h-full flex-col gap-4">
                <div class="flex gap-4 pr-16">
                    <div class="min-w-0 flex-1 space-y-3">
                        <div class="flex flex-wrap items-center gap-2">
                            <h3 class=format!("text-lg font-semibold {}", title_text_class)>{booth_description.get_value()}</h3>
                            <Show when=move || is_archived>
                                <span class="rounded-full bg-slate-200 px-2.5 py-1 text-xs font-medium uppercase tracking-wide text-slate-700">{t!("archive.archived_badge")()}</span>
                            </Show>
                            {if is_direct_sale {
                                view! {
                                    <span class="rounded-full bg-blue-100 px-2.5 py-1 text-xs font-medium text-blue-700">
                                        {t!("booth.type_direct_sale")()}
                                    </span>
                                }.into_any()
                            } else {
                                ().into_any()
                            }}
                            {if show_no_products_badge {
                                view! {
                                    <span class="rounded-full bg-amber-100 px-2.5 py-1 text-xs font-medium text-amber-700">
                                        {t!("booth.no_products_badge")()}
                                    </span>
                                }.into_any()
                            } else {
                                ().into_any()
                            }}
                        </div>
                        <div class="min-w-0 flex-1 space-y-1">
                            <p class=date_text_class>{t!("booth.date_prefix")} " " {move || format_date(booth_date)}</p>
                            <p class="text-xs text-gray-500">
                                {format!("{} vendors", vendor_count)}
                                " · "
                                {move || format_datetime(booth_updated_at, locale.get())}
                            </p>
                            <Show when=move || is_archived>
                                <Show when=move || archived_timestamp.get().is_some()>
                                    <p class="text-xs text-slate-500">{move || archived_timestamp.get().unwrap_or_default()}</p>
                                </Show>
                            </Show>
                        </div>
                    </div>
                </div>
            </div>
        </article>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::confirmation_token_from_id_str;

    #[test]
    fn confirmation_token_uses_last_four_alphanumeric_characters() {
        assert_eq!(
            confirmation_token_from_id_str("12345678-1234-1234-1234-1234567890ab"),
            "90AB"
        );
    }

    #[test]
    fn confirmation_token_falls_back_when_id_has_too_few_alphanumeric_characters() {
        assert_eq!(confirmation_token_from_id_str("--"), "DELETE");
        assert_eq!(confirmation_token_from_id_str("a-1"), "DELETE");
    }
}

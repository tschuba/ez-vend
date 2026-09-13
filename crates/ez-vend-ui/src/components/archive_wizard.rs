use leptos::html;
use leptos::*;

use crate::components::{
    use_toast, Button, ButtonVariant, Modal, ModalSize, StorageStatusRefreshContext,
};
use crate::selected_booth_context::use_booth_list_version;
use crate::state::use_app_state;
use crate::t;
use crate::utils::{current_device_info, download_text_file};
use domain::Booth;
use ez_vend_storage::{record_backup_completed, ArchivePreview, ExportRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArchiveStep {
    Review,
    Export,
    Confirm,
    Done,
}

fn confirmation_token_from_id_str(id: &str) -> String {
    let sanitized: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();

    if sanitized.len() < 4 {
        return "ARCHIVE".to_string();
    }

    let len = sanitized.len();
    sanitized[len.saturating_sub(4)..].to_uppercase()
}

#[component]
pub fn ArchiveWizard(
    booth: Booth,
    #[prop(into)] show: Signal<bool>,
    on_close: impl Fn() + Clone + 'static,
    on_archived: impl Fn() + Clone + 'static,
) -> impl IntoView {
    let app_state = use_app_state();
    let booth_list_version = use_booth_list_version();
    let toast = use_toast();
    let on_archived = StoredValue::new(on_archived);
    let on_close = StoredValue::new(on_close);
    let (step, set_step) = create_signal(ArchiveStep::Review);
    let (preview, set_preview) = create_signal(None::<ArchivePreview>);
    let (export_record, set_export_record) = create_signal(None::<ExportRecord>);
    let (token_input, set_token_input) = create_signal(String::new());
    let (is_busy, set_is_busy) = create_signal(false);
    let token_ref = create_node_ref::<html::Input>();

    create_effect(move |_| {
        if !show.get() {
            return;
        }

        let state = app_state.get();
        let booth_id = booth.id;
        set_is_busy.set(true);
        set_preview.set(None);
        set_export_record.set(None);
        set_step.set(ArchiveStep::Review);
        set_token_input.set(String::new());

        spawn_local(async move {
            let result = match state {
                Some(Ok(state)) => state.archive_service.generate_preview(&booth_id).await,
                Some(Err(error)) => Err(ez_vend_storage::StorageError::TransactionError(error)),
                None => Err(ez_vend_storage::StorageError::TransactionError(t!(
                    "common.loading"
                )(
                ))),
            };

            set_is_busy.set(false);

            match result {
                Ok(preview) => set_preview.set(Some(preview)),
                Err(error) => toast.error(error.to_string()),
            }
        });
    });

    let confirmation_token = store_value(confirmation_token_from_id_str(&booth.id.as_str()));
    let token_matches = Signal::derive(move || {
        token_input.get().trim().to_uppercase() == confirmation_token.get_value()
    });

    let create_export = {
        let booth_id = booth.id;
        move || {
            if is_busy.get_untracked() {
                return;
            }

            let state_result = app_state.get();
            set_is_busy.set(true);

            spawn_local(async move {
                let result = async move {
                    let state = match state_result {
                        Some(Ok(state)) => state,
                        Some(Err(error)) => return Err(error),
                        None => return Err(t!("common.loading")()),
                    };

                    let device_info = current_device_info();
                    let mut data = state
                        .export_service
                        .export_booth(&booth_id)
                        .await
                        .map_err(|error| error.to_string())?;
                    data.device_info = Some(device_info.clone());
                    let serialized = state
                        .export_service
                        .serialize_booth_backup_with_device_identifier(
                            &data,
                            Some(device_info.identifier.as_str()),
                        )
                        .map_err(|error| error.to_string())?;

                    download_text_file(
                        &serialized.file_name,
                        &serialized.json,
                        "application/json;charset=utf-8",
                    )
                    .map_err(|error| error.to_string())?;

                    record_backup_completed(&state.database, chrono::Utc::now())
                        .await
                        .map_err(|error| error.to_string())?;

                    state
                        .export_service
                        .record_booth_export(&booth_id, &serialized)
                        .await
                        .map_err(|error| error.to_string())
                }
                .await;

                set_is_busy.set(false);

                match result {
                    Ok(record) => {
                        set_export_record.set(record);
                        set_step.set(ArchiveStep::Confirm);
                        toast.success(t!("archive.export_success")());
                        set_timeout(
                            move || {
                                if let Some(input) = token_ref.get() {
                                    let _ = input.focus();
                                    input.select();
                                }
                            },
                            std::time::Duration::from_millis(0),
                        );
                    }
                    Err(error) => {
                        toast.error(t!("archive.export_failed")().replace("{error}", &error));
                    }
                }
            });
        }
    };

    let archive_booth = {
        let booth_id = booth.id;
        move || {
            if is_busy.get_untracked() || !token_matches.get_untracked() {
                return;
            }

            let Some(export_record) = export_record.get_untracked() else {
                return;
            };

            let state_result = app_state.get();
            let db = state_result
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .map(|s| s.database.clone());
            set_is_busy.set(true);

            spawn_local(async move {
                let result = async move {
                    let state = match state_result {
                        Some(Ok(state)) => state,
                        Some(Err(error)) => return Err(error),
                        None => return Err(t!("common.loading")()),
                    };

                    state
                        .archive_service
                        .archive_booth(&booth_id, export_record, current_device_info())
                        .await
                        .map_err(|error| error.to_string())
                }
                .await;

                set_is_busy.set(false);

                match result {
                    Ok(_) => {
                        booth_list_version.update(|value| *value += 1);
                        set_step.set(ArchiveStep::Done);
                        toast.success(t!("archive.archive_success")());
                        on_archived.with_value(|callback| callback());

                        // Archiving is always preceded by an export in this wizard,
                        // so the data is in a known-good state. Mark last_backup_at
                        // so the footer shows green instead of amber.
                        if let Some(db) = db {
                            spawn_local(async move {
                                let _ = record_backup_completed(&db, chrono::Utc::now()).await;
                            });
                        }
                        if let Some(ctx) = use_context::<StorageStatusRefreshContext>() {
                            ctx.0.update(|n| *n += 1);
                        }
                    }
                    Err(error) => {
                        toast.error(t!("archive.archive_failed")().replace("{error}", &error));
                    }
                }
            });
        }
    };

    let close_modal = {
        move || {
            set_step.set(ArchiveStep::Review);
            set_preview.set(None);
            set_export_record.set(None);
            set_token_input.set(String::new());
            on_close.with_value(|callback| callback());
        }
    };

    let step_title = Signal::derive(move || match step.get() {
        ArchiveStep::Review => t!("archive.review_title")(),
        ArchiveStep::Export => t!("archive.export_title")(),
        ArchiveStep::Confirm => t!("archive.confirm_title")(),
        ArchiveStep::Done => t!("archive.done_title")(),
    });

    view! {
        <Modal
            show=show
            on_close=close_modal
            title=step_title
            size=ModalSize::Large
            action_bar=view! {
                <div class="contents">
                    <Button variant=ButtonVariant::Secondary on_click=Box::new(close_modal)>
                        {t!("common.cancel")()}
                    </Button>
                    {move || match step.get() {
                        ArchiveStep::Review => view! {
                            <Button
                                disabled=Signal::derive(move || preview.get().is_none() || is_busy.get())
                                on_click=Box::new(move || set_step.set(ArchiveStep::Export))
                            >
                                {t!("common.next")()}
                            </Button>
                        }.into_view(),
                        ArchiveStep::Export => view! {
                            <Button
                                disabled=is_busy
                                on_click=Box::new(create_export)
                            >
                                {move || if is_busy.get() { t!("backup.export_in_progress")() } else { t!("archive.create_export")() }}
                            </Button>
                        }.into_view(),
                        ArchiveStep::Confirm => view! {
                            <Button
                                variant=ButtonVariant::Danger
                                disabled=Signal::derive(move || !token_matches.get() || is_busy.get())
                                on_click=Box::new(archive_booth)
                            >
                                {move || if is_busy.get() { t!("archive.processing")() } else { t!("archive.confirm_action")() }}
                            </Button>
                        }.into_view(),
                        ArchiveStep::Done => view! {
                            <Button on_click=Box::new(move || on_close.with_value(|callback| callback()))>{t!("common.close")()}</Button>
                        }.into_view(),
                    }}
                </div>
            }.into_view()
        >
            <Show when=move || preview.get().is_some() fallback=move || view! {
                <div class="flex items-center justify-center py-12">
                    <div class="text-center">
                        <div class="inline-block h-12 w-12 animate-spin rounded-full border-b-2 border-blue-600"></div>
                        <p class="mt-4 text-gray-600">{t!("common.loading")()}</p>
                    </div>
                </div>
            }>
                {move || preview.get().map(|preview| {
                    let confirmation_token_text = confirmation_token.get_value();
                    view! {
                    <div class="space-y-5 text-sm text-gray-700">
                        <Show when=move || preview.size_warning.is_some()>
                            <div class="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-amber-900">
                                <p class="font-semibold">{t!("archive.size_warning_title")()}</p>
                                <p>{t!("archive.size_warning_body")()
                                    .replace("{vendors}", &preview.summary.vendor_count.to_string())
                                    .replace("{purchases}", &preview.summary.purchase_count.to_string())}</p>
                            </div>
                        </Show>

                        <Show when=move || step.get() == ArchiveStep::Review>
                            <div class="space-y-4">
                                <p>{t!("archive.review_intro")()}</p>
                                <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
                                    <div class="rounded-lg bg-gray-50 px-4 py-3"><p class="text-xs uppercase text-gray-500">{t!("report.sales_total")()}</p><p class="text-lg font-semibold">{crate::formatting::format_currency(preview.summary.total_revenue, crate::i18n::use_locale().get())}</p></div>
                                    <div class="rounded-lg bg-gray-50 px-4 py-3"><p class="text-xs uppercase text-gray-500">{t!("report.purchase_count")()}</p><p class="text-lg font-semibold">{preview.summary.purchase_count}</p></div>
                                    <div class="rounded-lg bg-gray-50 px-4 py-3"><p class="text-xs uppercase text-gray-500">{t!("report.items")()}</p><p class="text-lg font-semibold">{preview.summary.item_count}</p></div>
                                    <div class="rounded-lg bg-gray-50 px-4 py-3"><p class="text-xs uppercase text-gray-500">{t!("report.vendors_count")()}</p><p class="text-lg font-semibold">{preview.summary.vendor_count}</p></div>
                                </div>
                                <div class="grid gap-4 sm:grid-cols-2">
                                    <div class="rounded-lg border border-red-200 bg-red-50 px-4 py-3">
                                        <p class="font-semibold text-red-900">{t!("archive.will_delete")()}</p>
                                        <ul class="mt-2 space-y-1 text-red-900">
                                            <li>{t!("archive.delete_transactions")()}</li>
                                            <li>{t!("archive.delete_vendors")()}</li>
                                        </ul>
                                    </div>
                                    <div class="rounded-lg border border-green-200 bg-green-50 px-4 py-3">
                                        <p class="font-semibold text-green-900">{t!("archive.will_keep")()}</p>
                                        <ul class="mt-2 space-y-1 text-green-900">
                                            <li>{t!("archive.keep_booth")()}</li>
                                            <li>{t!("archive.keep_summary")()}</li>
                                        </ul>
                                    </div>
                                </div>
                            </div>
                        </Show>

                        <Show when=move || step.get() == ArchiveStep::Export>
                            <div class="space-y-4">
                                <p>{t!("archive.export_intro")()}</p>
                                <div class="rounded-lg border border-blue-200 bg-blue-50 px-4 py-3 text-blue-900">
                                    <p>{t!("archive.export_required")()}</p>
                                </div>
                            </div>
                        </Show>

                        <Show when=move || step.get() == ArchiveStep::Confirm>
                            <div class="space-y-4">
                                <div class="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-red-900">
                                    <p class="font-semibold">{t!("archive.irreversible_title")()}</p>
                                    <p>{t!("archive.irreversible_body")()}</p>
                                </div>
                                <p>{t!("archive.confirm_with_token")().replace("{token}", &confirmation_token_text)}</p>
                                <input
                                    node_ref=token_ref
                                    class="w-full rounded-lg border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-red-500"
                                    placeholder=t!("archive.token_placeholder")()
                                    value=move || token_input.get()
                                    on:input=move |ev| set_token_input.set(event_target_value(&ev))
                                />
                            </div>
                        </Show>

                        <Show when=move || step.get() == ArchiveStep::Done>
                            <div class="space-y-3">
                                <div class="rounded-lg border border-green-200 bg-green-50 px-4 py-3 text-green-900">
                                    <p class="font-semibold">{t!("archive.done_message")()}</p>
                                </div>
                            </div>
                        </Show>
                    </div>
                }})}
            </Show>
        </Modal>
    }
}

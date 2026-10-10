use crate::components::{use_toast, Icon, LuDownload, Modal, ModalSize, SpinnerIcon};
use crate::error_logging::{current_route, stack_trace, use_error_logger, ErrorLogDraft};
use crate::state::use_app_state;
use crate::state::AppState;
use crate::t;
use crate::utils::{
    current_device_info, download_text_file, share_json_file, supports_native_share_with_files,
};
use domain::BoothId;
use ez_vend_storage::record_backup_completed;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExportScope {
    All,
    Booth(BoothId),
}

#[component]
pub fn ExportButton(scope: ExportScope) -> impl IntoView {
    let app_state = use_app_state();
    let toast = use_toast();
    let log_error = use_error_logger();
    let (is_exporting, set_is_exporting) = signal(false);
    let (show_modal, set_show_modal) = signal(false);
    let share_supported = supports_native_share_with_files();

    let trigger_label = move || t!("backup.export_button")();

    let log_error_for_export = log_error.clone();
    let handle_export = move || {
        if is_exporting.get_untracked() {
            return;
        }

        let log_error = log_error_for_export.clone();
        start_export(scope, app_state, toast, set_is_exporting, false, log_error);
    };

    let log_error_for_share = log_error.clone();
    let handle_share = move || {
        if is_exporting.get_untracked() || !share_supported {
            return;
        }

        let log_error = log_error_for_share.clone();
        start_export(scope, app_state, toast, set_is_exporting, true, log_error);
    };

    let handle_export_action = StoredValue::new_local(handle_export);
    let handle_share_action = StoredValue::new_local(handle_share);

    let handle_trigger_click = move |_| {
        if !share_supported {
            handle_export_action.with_value(|handler| handler());
        } else {
            set_show_modal.set(true);
        }
    };

    view! {
        <button
            type="button"
            on:click=handle_trigger_click
            disabled=move || is_exporting.get()
            class="inline-flex h-11 w-11 items-center justify-center rounded-lg text-gray-600 transition-colors hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 touch-manipulation"
            title={trigger_label()}
            aria-label={trigger_label()}
        >
            {move || if is_exporting.get() {
                view! { <SpinnerIcon class="h-5 w-5 animate-spin".to_string() /> }.into_any()
            } else {
                view! { <Icon icon=LuDownload class="h-5 w-5" /> }.into_any()
            }}
        </button>
        <Modal
            show=show_modal
            on_close=move || set_show_modal.set(false)
            title=Signal::derive(move || t!("backup.export_modal_title")())
            size=ModalSize::Medium
        >
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <button
                    type="button"
                    class="rounded-lg border-2 border-gray-200 p-6 text-left transition-colors hover:border-blue-400 hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    on:click=move |_| {
                        set_show_modal.set(false);
                        handle_export_action.with_value(|handler| handler());
                    }
                >
                    <p class="text-base font-semibold text-gray-900">{t!("backup.export_booth")()}</p>
                    <p class="mt-1 text-sm text-gray-500">{t!("backup.export_booth_description")()}</p>
                </button>
                <Show when=move || share_supported>
                    <button
                        type="button"
                        class="rounded-lg border-2 border-gray-200 p-6 text-left transition-colors hover:border-blue-400 hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        on:click=move |_| {
                            set_show_modal.set(false);
                            handle_share_action.with_value(|handler| handler());
                        }
                    >
                        <p class="text-base font-semibold text-gray-900">{t!("backup.share_booth")()}</p>
                        <p class="mt-1 text-sm text-gray-500">{t!("backup.share_booth_description")()}</p>
                    </button>
                </Show>
            </div>
        </Modal>
    }
}

fn start_export(
    scope: ExportScope,
    app_state: LocalResource<Result<AppState, String>>,
    toast: crate::components::ToastContext,
    set_is_exporting: WriteSignal<bool>,
    share_after_export: bool,
    log_error: impl Fn(ErrorLogDraft) + Clone + 'static,
) {
    let state_result = app_state.get();
    set_is_exporting.set(true);

    spawn_local(async move {
        let result: Result<(), String> = async move {
            let state = match state_result {
                Some(Ok(state)) => state,
                Some(Err(error)) => return Err(error),
                None => return Err(t!("common.loading")()),
            };

            let device_info = current_device_info();
            let serialized = match scope {
                ExportScope::All => {
                    let mut data = state
                        .export_service
                        .export_all()
                        .await
                        .map_err(|err| err.to_string())?;
                    data.device_info = Some(device_info.clone());
                    state
                        .export_service
                        .serialize_full_backup_with_device_identifier(
                            &data,
                            Some(device_info.identifier.as_str()),
                        )
                        .map_err(|err| err.to_string())?
                }
                ExportScope::Booth(booth_id) => {
                    let mut data = state
                        .export_service
                        .export_booth(&booth_id)
                        .await
                        .map_err(|err| err.to_string())?;
                    data.device_info = Some(device_info.clone());
                    state
                        .export_service
                        .serialize_booth_backup_with_device_identifier(
                            &data,
                            Some(device_info.identifier.as_str()),
                        )
                        .map_err(|err| err.to_string())?
                }
            };

            if share_after_export {
                share_json_file(
                    &serialized.file_name,
                    &serialized.json,
                    &t!("backup.share_booth")(),
                )
                .await
                .map_err(|err| err.to_string())?;
            } else {
                download_text_file(
                    &serialized.file_name,
                    &serialized.json,
                    "application/json;charset=utf-8",
                )
                .map_err(|err| err.to_string())?;
            }

            record_backup_completed(&state.database, chrono::Utc::now())
                .await
                .map_err(|err| err.to_string())?;

            Ok::<(), String>(())
        }
        .await;

        set_is_exporting.set(false);

        // Notify the footer to reload diagnostics so it reflects the new backup
        if let Some(ctx) = use_context::<crate::components::StorageStatusRefreshContext>() {
            ctx.0.update(|n| *n += 1);
        }

        match result {
            Ok(()) => {
                let key = if share_after_export {
                    t!("backup.share_success")()
                } else {
                    match scope {
                        ExportScope::All => t!("backup.export_success_all")(),
                        ExportScope::Booth(_) => t!("backup.export_success_booth")(),
                    }
                };
                toast.success(key);
            }
            Err(error) => {
                log_error(ErrorLogDraft {
                    error_type: if share_after_export {
                        "share_failed".to_string()
                    } else {
                        "export_failed".to_string()
                    },
                    error_message: error.clone(),
                    stack_trace: stack_trace(),
                    user_action: Some(if share_after_export {
                        "share backup".to_string()
                    } else {
                        "export backup".to_string()
                    }),
                    route: current_route(),
                    vendor_id: None,
                    purchase_id: None,
                    details: vec![format!(
                        "scope={}",
                        match scope {
                            ExportScope::All => "all",
                            ExportScope::Booth(_) => "booth",
                        }
                    )],
                });
                let message = if share_after_export {
                    t!("backup.share_failed")()
                } else {
                    t!("backup.export_failed")()
                };
                toast.error(format!("{message}: {error}"));
            }
        }
    });
}

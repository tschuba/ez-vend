#![allow(clippy::clone_on_copy)]
#![allow(clippy::unnecessary_map_or)]

use chrono::Utc;
use leptos::leptos_dom::helpers::{window_event_listener_untyped, WindowListenerHandle};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

use crate::components::*;
use crate::error_logging::{recent_error_cutoff, stack_trace, use_error_logger, ErrorLogDraft};
use crate::selected_booth_context::use_selected_booth;
use crate::settings_support::{
    build_error_log_print_html, diagnostics_export_filename, diagnostics_last_backup_label,
    error_log_print_filename, format_error_log_text, format_error_log_timestamp,
    integrity_issue_count, DiagnosticsBoothSummary, DiagnosticsExportData, APP_VERSION, DOCS_URL,
    REPOSITORY_URL,
};
use crate::state::use_app_state;
use crate::t;
use crate::utils::{
    copy_text_to_clipboard, current_device_info, download_text_file, open_print_window_html,
    reset_device_identifier, save_device_identifier, validate_device_identifier,
};
use ez_vend_storage::{
    ArchiveAuditEvent, ArchiveAuditEventType, ErrorLogEntry, IntegrityStatus, StorageDiagnostics,
};

const SETTINGS_TAB_GENERAL_ID: &str = "general";
const SETTINGS_TAB_DIAGNOSTICS_ID: &str = "diagnostics";
const SETTINGS_TAB_MIGRATION_ID: &str = "migration";

fn settings_tab_index_from_id(tab_id: &str) -> usize {
    match tab_id {
        SETTINGS_TAB_DIAGNOSTICS_ID => 1,
        SETTINGS_TAB_MIGRATION_ID => 2,
        _ => 0,
    }
}

fn settings_tab_index_from_hash(hash: &str) -> usize {
    settings_tab_index_from_id(hash.trim_start_matches('#'))
}

fn settings_tab_hash(index: usize) -> &'static str {
    match index {
        1 => "#diagnostics",
        2 => "#migration",
        _ => "#general",
    }
}

fn settings_tab_id(index: usize) -> &'static str {
    match index {
        1 => SETTINGS_TAB_DIAGNOSTICS_ID,
        2 => SETTINGS_TAB_MIGRATION_ID,
        _ => SETTINGS_TAB_GENERAL_ID,
    }
}

fn settings_tab_index_from_location() -> usize {
    let Some(window) = web_sys::window() else {
        return 0;
    };

    if let Ok(search) = window.location().search() {
        if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
            if let Some(tab) = params.get("tab") {
                return settings_tab_index_from_id(&tab);
            }
        }
    }

    window
        .location()
        .hash()
        .ok()
        .map(|hash| settings_tab_index_from_hash(&hash))
        .unwrap_or(0)
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let app_state = use_app_state();
    let toast = use_toast();
    let log_error = use_error_logger();
    let selected_booth = use_selected_booth();
    let device_info = current_device_info();
    let platform = device_info.platform.clone();
    let browser = device_info.browser.clone();
    let initial_identifier = device_info.identifier;

    let (saved_identifier, set_saved_identifier) = signal(initial_identifier.clone());
    let device_identifier = RwSignal::new(initial_identifier);
    let (validation_error, set_validation_error) = signal(None::<String>);
    let (storage_diagnostics, set_storage_diagnostics) = signal(None::<StorageDiagnostics>);
    let (integrity_status, set_integrity_status) = signal(None::<IntegrityStatus>);
    let (is_loading_diagnostics, set_is_loading_diagnostics) = signal(true);
    let (is_running_integrity_check, set_is_running_integrity_check) = signal(false);
    let (error_log_entries, set_error_log_entries) = signal(Vec::<ErrorLogEntry>::new());
    let (archive_audit_events, set_archive_audit_events) = signal(Vec::<ArchiveAuditEvent>::new());
    let (recent_error_count, set_recent_error_count) = signal(0_usize);
    let (is_loading_error_log, set_is_loading_error_log) = signal(true);
    let (is_loading_archive_history, set_is_loading_archive_history) = signal(true);
    let (is_clearing_error_log, set_is_clearing_error_log) = signal(false);
    let (is_exporting_diagnostics, set_is_exporting_diagnostics) = signal(false);
    let (show_clear_error_log_confirm, set_show_clear_error_log_confirm) = signal(false);
    let (expanded_error_ids, set_expanded_error_ids) = signal(Vec::<u32>::new());
    let initial_tab = settings_tab_index_from_location();
    let active_tab = RwSignal::new(initial_tab);

    // ponytail: StoredValue keeps the handle alive for the component lifetime; plain let would drop immediately
    let _hashchange_handle =
        StoredValue::new_local(window_event_listener_untyped("hashchange", move |_| {
            active_tab.set(settings_tab_index_from_location());
        }));

    Effect::new(move |_| {
        let active_index = active_tab.get();
        let hash = settings_tab_hash(active_index);
        let tab_id = settings_tab_id(active_index);

        if let Some(window) = web_sys::window() {
            if window
                .location()
                .pathname()
                .ok()
                .map_or(false, |p| p.ends_with("/settings"))
            {
                if let Ok(current_href) = window.location().href() {
                    if let Ok(url) = web_sys::Url::new(&current_href) {
                        url.search_params().set("tab", tab_id);
                        url.set_hash(hash);

                        if let Ok(history) = window.history() {
                            let _ = history.replace_state_with_url(
                                &JsValue::NULL,
                                "",
                                Some(&url.href()),
                            );
                        }
                    }
                }
            }
        }
    });

    {
        let app_state = app_state.clone();
        let toast = toast.clone();
        Effect::new(move |_| match app_state.get() {
            Some(Ok(state)) => {
                set_is_loading_diagnostics.set(true);
                let toast = toast.clone();
                spawn_local(async move {
                    let result = state.load_storage_diagnostics().await;
                    set_is_loading_diagnostics.set(false);

                    match result {
                        Ok(diagnostics) => set_storage_diagnostics.set(Some(diagnostics)),
                        Err(error) => toast.error(format!("{}: {error}", t!("common.error")())),
                    }
                });
            }
            Some(Err(error)) => {
                set_is_loading_diagnostics.set(false);
                toast.error(format!("{}: {error}", t!("common.error")()));
            }
            None => {}
        });
    }

    {
        let app_state = app_state.clone();
        let toast = toast.clone();
        Effect::new(move |_| match app_state.get() {
            Some(Ok(state)) => {
                set_is_loading_archive_history.set(true);
                let toast = toast.clone();
                spawn_local(async move {
                    let result = state.archive_service.list_audit_events().await;
                    set_is_loading_archive_history.set(false);

                    match result {
                        Ok(events) => set_archive_audit_events.set(events),
                        Err(error) => {
                            toast.error(format!("{}: {error}", t!("common.error")()));
                        }
                    }
                });
            }
            Some(Err(error)) => {
                set_is_loading_archive_history.set(false);
                toast.error(format!("{}: {error}", t!("common.error")()));
            }
            None => {}
        });
    }

    {
        let app_state = app_state.clone();
        let toast = toast.clone();
        Effect::new(move |_| match app_state.get() {
            Some(Ok(state)) => {
                set_is_loading_error_log.set(true);
                let toast = toast.clone();
                spawn_local(async move {
                    let cutoff = recent_error_cutoff();
                    let entries = state.get_recent_errors(20).await;
                    let count = state.count_errors_since(cutoff).await;

                    set_is_loading_error_log.set(false);

                    match (entries, count) {
                        (Ok(entries), Ok(count)) => {
                            set_error_log_entries.set(entries);
                            set_recent_error_count.set(count);
                        }
                        (Err(error), _) | (_, Err(error)) => {
                            toast.error(format!("{}: {error}", t!("common.error")()));
                        }
                    }
                });
            }
            Some(Err(error)) => {
                set_is_loading_error_log.set(false);
                toast.error(format!("{}: {error}", t!("common.error")()));
            }
            None => {}
        });
    }

    Effect::new(move |_| {
        let value = device_identifier.get();
        let message = match validate_device_identifier(&value) {
            Ok(()) => None,
            Err(_) => Some(t!("settings.device_validation_error")()),
        };
        set_validation_error.set(message);
    });

    let is_dirty = Signal::derive(move || device_identifier.get() != saved_identifier.get());
    let can_save = Signal::derive(move || is_dirty.get() && validation_error.get().is_none());
    let is_error_log_actions_disabled =
        Signal::derive(move || is_loading_error_log.get() || error_log_entries.get().is_empty());

    let handle_save = {
        let toast = toast.clone();
        move || {
            let value = device_identifier.get_untracked();
            match save_device_identifier(value) {
                Ok(saved_value) => {
                    set_saved_identifier.set(saved_value.clone());
                    device_identifier.set(saved_value);
                    toast.success(t!("settings.device_save_success")());
                }
                Err(error) => {
                    toast.error(format!("{}: {error}", t!("common.error")()));
                }
            }
        }
    };

    let handle_reset = {
        let toast = toast.clone();
        move || match reset_device_identifier() {
            Ok(generated) => {
                set_saved_identifier.set(generated.clone());
                device_identifier.set(generated);
                toast.success(t!("settings.device_reset_success")());
            }
            Err(error) => {
                toast.error(format!("{}: {error}", t!("common.error")()));
            }
        }
    };

    let handle_run_integrity_check = {
        let app_state = app_state.clone();
        let toast = toast.clone();
        move || {
            if is_running_integrity_check.get_untracked() {
                return;
            }

            let state = match app_state.get() {
                Some(Ok(state)) => state,
                Some(Err(error)) => {
                    toast.error(format!("{}: {error}", t!("common.error")()));
                    return;
                }
                None => return,
            };

            set_is_running_integrity_check.set(true);
            let toast = toast.clone();

            spawn_local(async move {
                let result = state.run_integrity_check().await;

                set_is_running_integrity_check.set(false);

                match result {
                    Ok(status) => {
                        let issue_count = integrity_issue_count(&status);
                        if issue_count == 0 {
                            toast.success(t!("settings.integrity_status_healthy")());
                        } else {
                            toast.error(
                                t!("settings.integrity_status_issues_found")()
                                    .replace("{count}", &issue_count.to_string()),
                            );
                        }
                        set_integrity_status.set(Some(status));
                    }
                    Err(error) => {
                        toast.error(
                            t!("settings.integrity_check_failed")().replace("{error}", &error),
                        );
                    }
                }
            });
        }
    };

    let last_backup_label = Signal::derive(move || {
        storage_diagnostics
            .get()
            .as_ref()
            .and_then(diagnostics_last_backup_label)
            .unwrap_or_else(|| t!("settings.last_backup_never")())
    });

    let database_version_label = Signal::derive(move || {
        storage_diagnostics
            .get()
            .map(|diagnostics| format!("v{}", diagnostics.database_version))
            .unwrap_or_else(|| "-".to_string())
    });

    let toggle_error_details = move |id: u32| {
        set_expanded_error_ids.update(|ids| {
            if ids.contains(&id) {
                ids.retain(|existing| *existing != id);
            } else {
                ids.push(id);
            }
        });
    };

    let refresh_error_log = {
        let app_state = app_state.clone();
        let toast = toast.clone();
        move || {
            let state = match app_state.get() {
                Some(Ok(state)) => state,
                Some(Err(error)) => {
                    toast.error(format!("{}: {error}", t!("common.error")()));
                    return;
                }
                None => return,
            };

            let toast = toast.clone();
            set_is_loading_error_log.set(true);
            spawn_local(async move {
                let cutoff = recent_error_cutoff();
                let entries = state.get_recent_errors(20).await;
                let count = state.count_errors_since(cutoff).await;

                set_is_loading_error_log.set(false);

                match (entries, count) {
                    (Ok(entries), Ok(count)) => {
                        set_error_log_entries.set(entries);
                        set_recent_error_count.set(count);
                    }
                    (Err(error), _) | (_, Err(error)) => {
                        toast.error(format!("{}: {error}", t!("common.error")()));
                    }
                }
            });
        }
    };

    let handle_confirm_clear_error_log = {
        let app_state = app_state.clone();
        let toast = toast.clone();
        let refresh_error_log = refresh_error_log.clone();
        let log_error_for_clear = log_error.clone();
        move || {
            if is_clearing_error_log.get_untracked() {
                return;
            }

            let state = match app_state.get() {
                Some(Ok(state)) => state,
                Some(Err(error)) => {
                    toast.error(format!("{}: {error}", t!("common.error")()));
                    return;
                }
                None => return,
            };

            set_is_clearing_error_log.set(true);

            let toast = toast.clone();
            let refresh_error_log = refresh_error_log.clone();
            let log_error = log_error_for_clear.clone();
            spawn_local(async move {
                let result = state.clear_error_log().await;

                set_is_clearing_error_log.set(false);

                match result {
                    Ok(()) => {
                        toast.success(t!("settings.error_log_clear_success")());
                        set_error_log_entries.set(Vec::new());
                        set_recent_error_count.set(0);
                        set_expanded_error_ids.set(Vec::new());
                        refresh_error_log();
                    }
                    Err(error) => {
                        log_error(ErrorLogDraft {
                            error_type: "error_log_clear_failed".to_string(),
                            error_message: error.clone(),
                            stack_trace: stack_trace(),
                            user_action: Some("clear error log".to_string()),
                            route: crate::error_logging::current_route(),
                            vendor_id: None,
                            purchase_id: None,
                            details: Vec::new(),
                        });
                        toast.error(
                            t!("settings.error_log_clear_failed")().replace("{error}", &error),
                        );
                    }
                }
            });
        }
    };

    let handle_prompt_clear_error_log = move || {
        if is_clearing_error_log.get_untracked() {
            return;
        }

        set_show_clear_error_log_confirm.set(true);
    };

    let handle_export_diagnostics = {
        let app_state = app_state.clone();
        let toast = toast.clone();
        let log_error_for_export = log_error.clone();
        move || {
            if is_exporting_diagnostics.get_untracked() {
                return;
            }

            let state = match app_state.get() {
                Some(Ok(state)) => state,
                Some(Err(error)) => {
                    toast.error(format!("{}: {error}", t!("common.error")()));
                    return;
                }
                None => return,
            };

            set_is_exporting_diagnostics.set(true);
            let toast = toast.clone();
            let selected_booth = selected_booth.get_untracked();
            let device_info = current_device_info();
            let integrity_snapshot = integrity_status
                .get_untracked()
                .unwrap_or(IntegrityStatus::Healthy);
            let diagnostics_snapshot = storage_diagnostics.get_untracked();
            let log_error = log_error_for_export.clone();

            spawn_local(async move {
                let result: Result<(), String> = async move {
                    let exported_at = Utc::now();
                    let all_errors = state.get_recent_errors(100).await?;
                    let booth_summary = if let Some(booth) = selected_booth.clone() {
                        DiagnosticsBoothSummary {
                            active_booth_id: Some(booth.id.as_str()),
                            active_booth_description: Some(booth.description),
                            vendor_count: state
                                .vendor_repository
                                .find_by_booth(&booth.id)
                                .await
                                .map_err(|err| err.to_string())?
                                .len(),
                            purchase_count: state
                                .purchase_repository
                                .find_by_booth(&booth.id)
                                .await
                                .map_err(|err| err.to_string())?
                                .len(),
                        }
                    } else {
                        DiagnosticsBoothSummary {
                            active_booth_id: None,
                            active_booth_description: None,
                            vendor_count: 0,
                            purchase_count: 0,
                        }
                    };

                    let payload = DiagnosticsExportData {
                        app_version: APP_VERSION.to_string(),
                        database_version: diagnostics_snapshot
                            .as_ref()
                            .map(|diagnostics| diagnostics.database_version)
                            .unwrap_or_default(),
                        exported_at,
                        device_identifier: device_info.identifier,
                        device_platform: device_info.platform,
                        device_browser: device_info.browser,
                        session_id: state.session_id.clone(),
                        last_backup_at: diagnostics_snapshot
                            .and_then(|diagnostics| diagnostics.last_backup_at),
                        integrity_status: integrity_snapshot,
                        recent_error_count: recent_error_count.get_untracked(),
                        errors: all_errors,
                        archive_audit_events: state
                            .archive_service
                            .list_audit_events()
                            .await
                            .map_err(|err| err.to_string())?,
                        booth_summary,
                    };

                    let json = serde_json::to_string_pretty(&payload).map_err(|error| {
                        format!("failed to serialize diagnostics export: {error}")
                    })?;
                    download_text_file(
                        &diagnostics_export_filename(exported_at),
                        &json,
                        "application/json;charset=utf-8",
                    )
                }
                .await;

                set_is_exporting_diagnostics.set(false);

                match result {
                    Ok(()) => toast.success(t!("settings.error_log_export_success")()),
                    Err(error) => {
                        log_error(ErrorLogDraft {
                            error_type: "diagnostics_export_failed".to_string(),
                            error_message: error.clone(),
                            stack_trace: stack_trace(),
                            user_action: Some("export diagnostics".to_string()),
                            route: crate::error_logging::current_route(),
                            vendor_id: None,
                            purchase_id: None,
                            details: Vec::new(),
                        });
                        toast.error(
                            t!("settings.error_log_export_failed")().replace("{error}", &error),
                        );
                    }
                }
            });
        }
    };

    let handle_copy_error_log = {
        let toast = toast.clone();
        move || {
            let text = format_error_log_text(&error_log_entries.get_untracked());
            spawn_local(async move {
                match copy_text_to_clipboard(&text).await {
                    Ok(()) => toast.success(t!("settings.error_log_copy_success")()),
                    Err(error) => toast
                        .error(t!("settings.error_log_copy_failed")().replace("{error}", &error)),
                }
            });
        }
    };

    let handle_print_error_log = {
        let toast = toast.clone();
        move || {
            let entries = error_log_entries.get_untracked();
            let html =
                build_error_log_print_html(&entries, &t!("settings.error_log_section_title")());
            if let Err(error) = open_print_window_html(&html) {
                let fallback_text = format_error_log_text(&entries);
                if let Err(download_error) = download_text_file(
                    &error_log_print_filename(Utc::now()),
                    &fallback_text,
                    "text/plain;charset=utf-8",
                ) {
                    toast.error(
                        t!("settings.error_log_print_failed")()
                            .replace("{error}", &format!("{error}; {download_error}")),
                    );
                } else {
                    toast.info(t!("settings.error_log_print_downloaded")());
                }
            }
        }
    };

    let on_save = Callback::new(move |_| handle_save());
    let on_reset = Callback::new(move |_| handle_reset());
    let on_run_integrity_check = Callback::new(move |_| handle_run_integrity_check());
    let on_export_diagnostics = Callback::new(move |_| handle_export_diagnostics());
    let on_clear_error_log = Callback::new(move |_| handle_prompt_clear_error_log());
    let on_print_error_log = Callback::new(move |_| handle_print_error_log());
    let on_copy_error_log = Callback::new(move |_| handle_copy_error_log());
    let close_clear_error_log_confirm = move || {
        set_show_clear_error_log_confirm.set(false);
    };
    let on_confirm_clear_error_log = move || {
        handle_confirm_clear_error_log();
    };
    let general_tab_view = move || {
        view! {
            <div class="space-y-6">
                <div class="space-y-2">
                    <h3 class="text-lg font-semibold text-slate-900">{t!("settings.device_section_title")}</h3>
                    <p class="text-sm text-gray-600">{t!("settings.device_help_text")}</p>
                </div>

                <div class="grid gap-4 sm:grid-cols-2">
                    <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
                        <p class="text-sm font-medium text-gray-700">{t!("settings.device_current_label")}</p>
                        <p class="mt-1 font-mono text-sm text-slate-900 break-all">{move || saved_identifier.get()}</p>
                    </div>
                    <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
                        <p class="text-sm font-medium text-gray-700">{t!("settings.device_detected_label")}</p>
                        <p class="mt-1 text-sm text-slate-900">
                            {t!("settings.device_platform_info")()
                                .replace("{platform}", &platform)
                                .replace("{browser}", &browser)}
                        </p>
                    </div>
                </div>

                <div class="space-y-2">
                    <Input
                        value=device_identifier
                        label=t!("settings.device_edit_label")()
                        error=validation_error
                        placeholder="marias-laptop".to_string()
                        aria_label=t!("settings.device_edit_label")()
                    />
                    <p class="text-sm text-gray-500">{t!("settings.device_format_help")}</p>
                </div>

                <div class="flex flex-wrap gap-3">
                    <Button
                        on_click=Box::new(move || on_save.run(()))
                        disabled=Signal::derive(move || !can_save.get())
                    >
                        {t!("settings.device_save")}
                    </Button>
                    <Button
                        on_click=Box::new(move || on_reset.run(()))
                        variant=ButtonVariant::Secondary
                    >
                        {t!("settings.device_reset")}
                    </Button>
                </div>

                <div class="space-y-2 pt-2">
                    <h3 class="text-lg font-semibold text-slate-900">{t!("settings.app_info_section_title")}</h3>
                    <p class="text-sm text-gray-600">{t!("settings.app_info_help_text")}</p>
                </div>

                <div class="grid gap-4 sm:grid-cols-2">
                    <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
                        <p class="text-sm font-medium text-gray-700">{t!("settings.app_version_label")}</p>
                        <p class="mt-1 font-mono text-sm text-slate-900">{APP_VERSION}</p>
                    </div>
                    <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
                        <p class="text-sm font-medium text-gray-700">{t!("settings.database_version_label")}</p>
                        <p class="mt-1 font-mono text-sm text-slate-900">{move || database_version_label.get()}</p>
                    </div>
                </div>

                <div class="flex flex-wrap gap-3 pt-1 text-sm">
                    <a
                        href=REPOSITORY_URL
                        target="_blank"
                        rel="noreferrer"
                        class="font-medium text-blue-600 hover:text-blue-700"
                    >
                        {t!("settings.repository_link")}
                    </a>
                    <a
                        href=DOCS_URL
                        target="_blank"
                        rel="noreferrer"
                        class="font-medium text-blue-600 hover:text-blue-700"
                    >
                        {t!("settings.docs_link")}
                    </a>
                </div>
            </div>
        }
    };

    view! {
        <Container class="mt-6 pb-24" aria_label=t!("settings.title")() as_landmark=true>
            <div class="mx-auto max-w-3xl">
                <Card title_view={t!("settings.title").into_any()}>
                    <ConfirmModal
                        show=show_clear_error_log_confirm
                        on_close=close_clear_error_log_confirm
                        on_confirm=on_confirm_clear_error_log
                        title=Signal::derive(|| t!("settings.error_log_clear_confirm_title")())
                        message=Signal::derive(|| t!("settings.error_log_clear_confirm_message")())
                        confirm_text=Signal::derive(|| t!("settings.error_log_clear_confirm_button")())
                        cancel_text=Signal::derive(|| t!("settings.error_log_clear_cancel_button")())
                        is_destructive=true
                    />
                    <TabGroup
                        tabs=vec![
                            TabItem {
                                id: SETTINGS_TAB_GENERAL_ID.to_string(),
                                label: t!("settings.tabs.general")(),
                                has_error: Signal::derive(|| false),
                            },
                            TabItem {
                                id: SETTINGS_TAB_DIAGNOSTICS_ID.to_string(),
                                label: t!("settings.tabs.diagnostics")(),
                                has_error: Signal::derive(|| false),
                            },
                            TabItem {
                                id: SETTINGS_TAB_MIGRATION_ID.to_string(),
                                label: t!("settings.tabs.migration")(),
                                has_error: Signal::derive(|| false),
                            },
                        ]
                        active_tab=active_tab
                        children=Box::new(move |tab_index| match tab_index {
                            0 => general_tab_view().into_any(),
                            1 => view! {
                                <div class="space-y-8">
                                    <section class="space-y-4">
                                        <div class="space-y-2">
                                            <h3 class="text-lg font-semibold text-slate-900">{t!("settings.storage_health_section_title")}</h3>
                                            <p class="text-sm text-gray-600">{t!("settings.storage_health_help_text")}</p>
                                        </div>

                                        <div class="grid gap-4 sm:grid-cols-2">
                                            <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
                                                <p class="text-sm font-medium text-gray-700">{t!("settings.last_backup_label")}</p>
                                                <p class="mt-1 text-sm text-slate-900">
                                                    {move || if is_loading_diagnostics.get() {
                                                        t!("common.loading")()
                                                    } else {
                                                        last_backup_label.get()
                                                    }}
                                                </p>
                                            </div>
                                            <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
                                                <p class="text-sm font-medium text-gray-700">{t!("settings.integrity_status_label")}</p>
                                                <p class="mt-1 text-sm text-slate-900">
                                                    {move || match integrity_status.get() {
                                                        Some(IntegrityStatus::Healthy) => t!("settings.integrity_status_healthy")(),
                                                        Some(IntegrityStatus::IssuesFound { ref issues }) => t!("settings.integrity_status_issues_found")().replace("{count}", &issues.len().to_string()),
                                                        None => t!("settings.integrity_not_checked")(),
                                                    }}
                                                </p>
                                            </div>
                                        </div>

                                        <div class="flex flex-wrap gap-3">
                                            <Button
                                                on_click=Box::new(move || on_run_integrity_check.run(()))
                                                variant=ButtonVariant::Secondary
                                                disabled=is_running_integrity_check
                                            >
                                                {move || if is_running_integrity_check.get() {
                                                    t!("settings.integrity_check_running")()
                                                } else {
                                                    t!("settings.integrity_check_button")()
                                                }}
                                            </Button>
                                        </div>

                                        <Show when=move || matches!(integrity_status.get(), Some(IntegrityStatus::IssuesFound { .. }))>
                                            <div class="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3">
                                                <p class="text-sm font-medium text-amber-900">{t!("settings.integrity_status_issues_detail_title")}</p>
                                                <ul class="mt-2 space-y-1 text-sm text-amber-900">
                                                    {move || match integrity_status.get() {
                                                        Some(IntegrityStatus::IssuesFound { issues }) => issues
                                                            .into_iter()
                                                            .map(|issue| view! { <li class="list-disc ml-5">{issue}</li> })
                                                            .collect_view()
                                                            .into_any(),
                                                        _ => ().into_any(),
                                                    }}
                                                </ul>
                                            </div>
                                        </Show>
                                    </section>

                                    <section class="space-y-4 border-t border-gray-200 pt-8">
                                        <div class="space-y-2">
                                            <h3 class="text-lg font-semibold text-slate-900">{t!("settings.archive_history_section_title")}</h3>
                                            <p class="text-sm text-gray-600">{t!("settings.archive_history_help_text")}</p>
                                        </div>

                                        <Show
                                            when=move || !is_loading_archive_history.get()
                                            fallback=move || {
                                                view! {
                                                    <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3 text-sm text-gray-600">
                                                        {t!("common.loading")()}
                                                    </div>
                                                }
                                            }
                                        >
                                            <Show
                                                when=move || !archive_audit_events.get().is_empty()
                                                fallback=move || {
                                                    view! {
                                                        <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3 text-sm text-gray-600">
                                                            {t!("settings.archive_history_empty")()}
                                                        </div>
                                                    }
                                                }
                                            >
                                                <div class="overflow-x-auto rounded-lg border border-gray-200">
                                                    <table class="min-w-full divide-y divide-gray-200 bg-white text-sm">
                                                        <thead class="bg-gray-50 text-left text-xs font-medium uppercase tracking-wider text-gray-500">
                                                            <tr>
                                                                <th class="px-4 py-3">{t!("settings.archive_history_timestamp_label")()}</th>
                                                                <th class="px-4 py-3">{t!("settings.archive_history_event_label")()}</th>
                                                                <th class="px-4 py-3">{t!("settings.archive_history_booth_label")()}</th>
                                                                <th class="px-4 py-3">{t!("settings.archive_history_device_label")()}</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="divide-y divide-gray-200">
                                                            <For
                                                                each=move || archive_audit_events.get()
                                                                key=|entry| entry.id.clone()
                                                                children=move |entry| {
                                                                    let action_label = match entry.event_type {
                                                                        ArchiveAuditEventType::Archived => {
                                                                            t!("settings.archive_history_event_archived")()
                                                                        }
                                                                        ArchiveAuditEventType::Restored => {
                                                                            t!("settings.archive_history_event_restored")()
                                                                        }
                                                                    };

                                                                    view! {
                                                                        <tr>
                                                                            <td class="px-4 py-3 text-slate-700">{format_error_log_timestamp(entry.timestamp)}</td>
                                                                            <td class="px-4 py-3 font-medium text-slate-900">{action_label}</td>
                                                                            <td class="px-4 py-3 text-slate-700">{entry.booth_description}</td>
                                                                            <td class="px-4 py-3 text-slate-700">{entry.device_info.identifier}</td>
                                                                        </tr>
                                                                    }
                                                                }
                                                            />
                                                        </tbody>
                                                    </table>
                                                </div>
                                            </Show>
                                        </Show>
                                    </section>

                                    <section class="space-y-4 border-t border-gray-200 pt-8">
                                        <div class="space-y-2">
                                            <h3 class="text-lg font-semibold text-slate-900">{t!("settings.error_log_section_title")}</h3>
                                            <p class="text-sm text-gray-600">{t!("settings.error_log_help_text")}</p>
                                        </div>

                                        <div class="grid gap-4 sm:grid-cols-2">
                                            <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
                                                <p class="text-sm font-medium text-gray-700">{t!("settings.error_log_recent_count_label")}</p>
                                                <p class="mt-1 text-sm text-slate-900">
                                                    {move || if is_loading_error_log.get() {
                                                        t!("common.loading")()
                                                    } else {
                                                        t!("settings.error_log_recent_count_value")()
                                                            .replace("{count}", &recent_error_count.get().to_string())
                                                    }}
                                                </p>
                                            </div>
                                            <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3">
                                                <p class="text-sm font-medium text-gray-700">{t!("settings.error_log_entries_label")}</p>
                                                <p class="mt-1 text-sm text-slate-900">{move || error_log_entries.get().len().to_string()}</p>
                                            </div>
                                        </div>

                                        <div class="flex flex-wrap gap-3">
                                            <Button
                                                on_click=Box::new(move || on_export_diagnostics.run(()))
                                                variant=ButtonVariant::Secondary
                                                disabled=is_exporting_diagnostics
                                            >
                                                {move || if is_exporting_diagnostics.get() {
                                                    t!("settings.error_log_export_running")()
                                                } else {
                                                    t!("settings.error_log_export_button")()
                                                }}
                                            </Button>
                                            <Button
                                                on_click=Box::new(move || on_clear_error_log.run(()))
                                                variant=ButtonVariant::Secondary
                                                disabled=is_clearing_error_log
                                            >
                                                {move || if is_clearing_error_log.get() {
                                                    t!("settings.error_log_clear_running")()
                                                } else {
                                                    t!("settings.error_log_clear_button")()
                                                }}
                                            </Button>
                                            <DropdownMenu
                                                trigger={view! {
                                                    <Button variant=ButtonVariant::Secondary>
                                                        {t!("settings.error_log_more_actions_button")}
                                                    </Button>
                                                }.into_any()}
                                                align="right".to_string()
                                                >
                                                <DropdownMenuItem
                                                    on_click=Callback::new(move |_| on_print_error_log.run(()))
                                                    disabled=is_error_log_actions_disabled
                                                >
                                                    {t!("settings.error_log_print_button")}
                                                </DropdownMenuItem>
                                                <DropdownMenuItem
                                                    on_click=Callback::new(move |_| on_copy_error_log.run(()))
                                                    disabled=is_error_log_actions_disabled
                                                >
                                                    {t!("settings.error_log_copy_button")}
                                                </DropdownMenuItem>
                                            </DropdownMenu>
                                        </div>

                                        <Show
                                            when=move || !error_log_entries.get().is_empty()
                                            fallback=move || {
                                                view! {
                                                    <div class="rounded-lg border border-gray-200 bg-gray-50 px-4 py-3 text-sm text-gray-600">
                                                        {t!("settings.error_log_empty")}
                                                    </div>
                                                }
                                            }
                                        >
                                            <div class="space-y-3">
                                                <For
                                                    each=move || error_log_entries.get()
                                                    key=|entry| entry.id.unwrap_or_default()
                                                    children=move |entry| {
                                                        let entry_id = entry.id.unwrap_or_default();
                                                        let is_expanded = Signal::derive(move || expanded_error_ids.get().contains(&entry_id));
                                                        let timestamp = format_error_log_timestamp(entry.timestamp);
                                                        let details = entry.context.clone().map(|context| context.details).unwrap_or_default();
                                                        let route = entry.context.as_ref().and_then(|context| context.route.clone());
                                                        let action = entry.context.as_ref().and_then(|context| context.user_action.clone());
                                                        let booth_id = entry.context.as_ref().and_then(|context| context.booth_id.clone());
                                                        let vendor_id = entry.context.as_ref().and_then(|context| context.vendor_id.clone());
                                                        let purchase_id = entry.context.as_ref().and_then(|context| context.purchase_id.clone());
                                                        let stack_trace_text = entry.stack_trace.clone();
                                                        let has_route = route.is_some();
                                                        let has_action = action.is_some();
                                                        let has_booth_id = booth_id.is_some();
                                                        let has_vendor_id = vendor_id.is_some();
                                                        let has_purchase_id = purchase_id.is_some();
                                                        let has_stack_trace = stack_trace_text.is_some();
                                                        let route_text = format!(
                                                            "{}: {}",
                                                            t!("settings.error_log_route_label")(),
                                                            route.clone().unwrap_or_default()
                                                        );
                                                        let action_text = format!(
                                                            "{}: {}",
                                                            t!("settings.error_log_action_label")(),
                                                            action.clone().unwrap_or_default()
                                                        );
                                                        let booth_text = format!(
                                                            "{}: {}",
                                                            t!("settings.error_log_booth_label")(),
                                                            booth_id.clone().unwrap_or_default()
                                                        );
                                                        let vendor_text = format!(
                                                            "{}: {}",
                                                            t!("settings.error_log_vendor_label")(),
                                                            vendor_id.clone().unwrap_or_default()
                                                        );
                                                        let purchase_text = format!(
                                                            "{}: {}",
                                                            t!("settings.error_log_purchase_label")(),
                                                            purchase_id.clone().unwrap_or_default()
                                                        );
                                                        let details_items = details
                                                            .iter()
                                                            .cloned()
                                                            .map(|detail| view! { <li class="list-disc ml-5">{detail}</li> })
                                                            .collect_view();
                                                        view! {
                                                            <div class="rounded-lg border border-gray-200 bg-white px-4 py-3 shadow-sm">
                                                                <button
                                                                    type="button"
                                                                    class="flex w-full items-start justify-between gap-4 text-left"
                                                                    on:click=move |_| toggle_error_details(entry_id)
                                                                >
                                                                    <div class="space-y-1">
                                                                        <p class="text-sm font-semibold text-slate-900">{entry.error_type.clone()}</p>
                                                                        <p class="text-sm text-gray-700">{entry.error_message.clone()}</p>
                                                                        <p class="text-xs text-gray-500">{timestamp.clone()}</p>
                                                                    </div>
                                                                    <span class="text-xs font-medium text-blue-600">
                                                                        {move || if is_expanded.get() {
                                                                            t!("settings.error_log_hide_details")()
                                                                        } else {
                                                                            t!("settings.error_log_show_details")()
                                                                        }}
                                                                    </span>
                                                                </button>

                                                                <Show when=move || is_expanded.get()>
                                                                    <div class="mt-3 space-y-2 border-t border-gray-100 pt-3 text-sm text-gray-700">
                                                                        <p><span class="font-medium text-slate-900">{t!("settings.error_log_session_label")}</span>{format!(": {}", entry.session_id)}</p>
                                                                        <p><span class="font-medium text-slate-900">{t!("settings.error_log_device_label")}</span>{format!(": {} / {} / {}", entry.device_info.identifier, entry.device_info.platform, entry.device_info.browser)}</p>
                                                                        {if has_route {
                                                                            Some(view! { <p>{route_text.clone()}</p> })
                                                                        } else {
                                                                            None
                                                                        }}
                                                                        {if has_action {
                                                                            Some(view! { <p>{action_text.clone()}</p> })
                                                                        } else {
                                                                            None
                                                                        }}
                                                                        {if has_booth_id {
                                                                            Some(view! { <p>{booth_text.clone()}</p> })
                                                                        } else {
                                                                            None
                                                                        }}
                                                                        {if has_vendor_id {
                                                                            Some(view! { <p>{vendor_text.clone()}</p> })
                                                                        } else {
                                                                            None
                                                                        }}
                                                                        {if has_purchase_id {
                                                                            Some(view! { <p>{purchase_text.clone()}</p> })
                                                                        } else {
                                                                            None
                                                                        }}
                                                                        {if details.is_empty() {
                                                                            None
                                                                        } else {
                                                                            Some(view! {
                                                                                <div>
                                                                                    <p class="font-medium text-slate-900">{t!("settings.error_log_details_label")}</p>
                                                                                    <ul class="mt-1 space-y-1">{details_items.clone()}</ul>
                                                                                </div>
                                                                            })
                                                                        }}
                                                                        {if has_stack_trace {
                                                                            Some(view! {
                                                                                <div>
                                                                                    <p class="font-medium text-slate-900">{t!("settings.error_log_stack_trace_label")}</p>
                                                                                    <pre class="mt-1 whitespace-pre-wrap rounded-lg bg-slate-50 p-3 text-xs text-slate-800">{stack_trace_text.clone().unwrap_or_default()}</pre>
                                                                                </div>
                                                                            })
                                                                        } else {
                                                                            None
                                                                        }}
                                                                    </div>
                                                                </Show>
                                                            </div>
                                                        }
                                                    }
                                                />
                                            </div>
                                        </Show>
                                    </section>
                                </div>
                            }
                            .into_any(),
                            2 => view! {
                                <MigrationWizard />
                            }
                            .into_any(),
                            _ => general_tab_view().into_any(),
                        })
                    />
                </Card>
            </div>
        </Container>
    }
}

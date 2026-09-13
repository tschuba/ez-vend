use chrono::{DateTime, Utc};
use leptos::*;
use wasm_bindgen_futures::spawn_local;

use crate::state::use_app_state;
use crate::t;
use crate::utils::detect_browser;

use super::use_toast;
use crate::error_logging::{current_route, stack_trace, use_error_logger, ErrorLogDraft};
use crate::utils::{current_device_info, download_text_file};
use ez_vend_storage::record_backup_completed;

/// Context signal that any component (e.g. ExportButton) can increment to
/// trigger the footer to reload its storage diagnostics.
#[derive(Clone, Copy)]
pub struct StorageStatusRefreshContext(pub RwSignal<u32>);

fn is_safari() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .map(|ua| detect_browser(&ua) == "Safari")
        .unwrap_or(false)
}

/// Returns a short human-readable relative age string for the given timestamp.
/// Examples: "vor wenigen Minuten", "vor 2 Stunden", "vor 3 Tagen"
fn format_backup_age(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if minutes < 2 {
        t!("backup.status_age_just_now")()
    } else if hours < 1 {
        t!("backup.status_age_minutes")().replace("{n}", &minutes.to_string())
    } else if days < 1 {
        t!("backup.status_age_hours")().replace("{n}", &hours.to_string())
    } else {
        t!("backup.status_age_days")().replace("{n}", &days.to_string())
    }
}

/// The smart footer storage status component. Displays one of four states:
///
/// - Green: backup is current and no unsaved changes (non-Safari)
/// - Amber: unsaved changes since last backup
/// - Amber: backup overdue (> 30 days for non-Safari, > 3 days for Safari)
/// - Amber: always shown on Safari with explanation text
///
/// Accepts a refresh signal via `StorageStatusRefreshContext` — increment it
/// to force a diagnostics reload (e.g. after a successful backup export).
#[component]
pub fn StorageIndicator() -> impl IntoView {
    let app_state = use_app_state();
    let refresh_context = use_context::<StorageStatusRefreshContext>();

    let (diagnostics, set_diagnostics) = create_signal(None::<ez_vend_storage::StorageDiagnostics>);
    let safari = is_safari();

    create_effect(move |_| {
        // Subscribe to refresh signal if provided
        if let Some(ctx) = refresh_context {
            let _ = ctx.0.get();
        }
        if let Some(Ok(state)) = app_state.get() {
            spawn_local(async move {
                if let Ok(d) = state.load_storage_diagnostics().await {
                    set_diagnostics.set(Some(d));
                }
            });
        }
    });

    // 30-second fallback poll — catches edge cases where the write callback
    // was not registered (e.g. component remount before AppState resolved).
    if let Some(ctx) = refresh_context {
        set_interval(
            move || ctx.0.update(|n| *n += 1),
            std::time::Duration::from_secs(30),
        );
    }

    let backups_href = format!("{}/booths", crate::base_path());

    view! {
        {move || {
            let Some(diag) = diagnostics.get() else {
                // Loading state — show the plain green pill
                return view! {
                    <div
                        role="status"
                        aria-live="polite"
                        class="flex flex-col items-center gap-2 text-center sm:flex-row sm:justify-center sm:text-left"
                    >
                        <div class="inline-flex items-center gap-2 rounded-full border border-sky-200 bg-sky-50 px-3 py-1 text-xs font-medium uppercase tracking-wide text-sky-800">
                            <span class="h-2 w-2 rounded-full bg-sky-500"></span>
                            <span>{t!("backup.status_ok_label")}</span>
                        </div>
                    </div>
                }.into_view();
            };

            let last_backup_at = diag.last_backup_at;
            let last_modified_at = diag.last_modified_at;

            // Determine which state to show
            let has_changes = match (last_modified_at, last_backup_at) {
                (Some(modified), Some(backup)) => modified > backup,
                (Some(_), None) => true,   // modified but never backed up
                (None, None) => false,     // fresh install, nothing written yet
                (None, Some(_)) => false,  // no writes since tracking started
            };

            let safari_overdue = safari && match last_backup_at {
                Some(at) => Utc::now().signed_duration_since(at).num_days() >= 3,
                None => true,
            };

            let backup_overdue = !safari && match last_backup_at {
                Some(at) => Utc::now().signed_duration_since(at).num_days() >= 30,
                None => true,
            };

            let last_backup_text = match last_backup_at {
                Some(at) => format!(
                    "{} {}",
                    t!("backup.status_last_backup")(),
                    format_backup_age(at)
                ),
                None => t!("backup.status_never")(),
            };

            let backups_href = backups_href.clone();

            if !has_changes && !safari_overdue && !backup_overdue {
                // ── State 1: Green ────────────────────────────────────────────
                view! {
                    <div
                        role="status"
                        aria-live="polite"
                        class="flex flex-col items-center gap-2 text-center sm:flex-row sm:justify-center sm:text-left"
                    >
                        <div class="group relative">
                            <button
                                class="inline-flex cursor-default items-center gap-2 rounded-full border border-green-200 bg-green-50 px-3 py-1 text-xs font-medium uppercase tracking-wide text-green-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-green-400"
                                tabindex="0"
                                aria-describedby="storage-tooltip"
                            >
                                <span class="h-2 w-2 rounded-full bg-green-500"></span>
                                <span>{t!("backup.status_ok_label")}</span>
                                <span class="ml-0.5 text-[10px] text-green-600 opacity-60">"ⓘ"</span>
                            </button>
                            // Tooltip (shown on hover/focus via group-hover/group-focus-within)
                            <div
                                id="storage-tooltip"
                                role="tooltip"
                                class="absolute bottom-full left-1/2 z-50 mb-2 hidden w-72 -translate-x-1/2 rounded-lg bg-gray-900 px-4 py-3 text-left text-xs text-gray-100 shadow-lg group-hover:block group-focus-within:block"
                            >
                                <p class="mb-1 font-semibold text-white">{t!("backup.tooltip_title")}</p>
                                <p class="leading-relaxed text-gray-300">{t!("backup.tooltip_body")}</p>
                                // Caret
                                <div class="absolute -bottom-1 left-1/2 -translate-x-1/2 border-4 border-transparent border-t-gray-900"></div>
                            </div>
                        </div>
                        <p class="text-sm text-gray-600">
                            {last_backup_text}
                            " · "
                            <a
                                href={backups_href}
                                class="font-medium text-blue-600 hover:text-blue-700"
                            >
                                {t!("backup.storage_indicator_link")}
                            </a>
                        </p>
                    </div>
                }.into_view()
            } else {
                // ── States 2/3/4: Amber ───────────────────────────────────────
                let (pill_label, detail_text) = if safari_overdue {
                    (
                        t!("backup.status_warning_label")(),
                        format!("{} · {}", t!("backup.status_safari")(), last_backup_text),
                    )
                } else if has_changes {
                    (
                        t!("backup.status_warning_label")(),
                        t!("backup.status_changes")(),
                    )
                } else {
                    (
                        t!("backup.status_overdue_label")(),
                        last_backup_text.clone(),
                    )
                };

                let app_state = use_app_state();
                let toast = use_toast();
                let log_error = use_error_logger();
                let (is_exporting, set_is_exporting) = create_signal(false);

                let handle_export = move |_| {
                    if is_exporting.get_untracked() {
                        return;
                    }
                    let state_result = app_state.get();
                    set_is_exporting.set(true);
                    let toast = toast;
                    let log_error = log_error.clone();
                    spawn_local(async move {
                        let result: Result<(), String> = async {
                            let state = match state_result {
                                Some(Ok(s)) => s,
                                Some(Err(e)) => return Err(e),
                                None => return Err(t!("common.loading")()),
                            };
                            let device_info = current_device_info();
                            let mut data = state
                                .export_service
                                .export_all()
                                .await
                                .map_err(|e| e.to_string())?;
                            data.device_info = Some(device_info.clone());
                            let serialized = state
                                .export_service
                                .serialize_full_backup_with_device_identifier(
                                    &data,
                                    Some(device_info.identifier.as_str()),
                                )
                                .map_err(|e| e.to_string())?;
                            download_text_file(
                                &serialized.file_name,
                                &serialized.json,
                                "application/json;charset=utf-8",
                            )
                            .map_err(|e| e.to_string())?;
                            record_backup_completed(&state.database, chrono::Utc::now())
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok(())
                        }
                        .await;
                        set_is_exporting.set(false);
                        if let Some(ctx) = use_context::<StorageStatusRefreshContext>() {
                            ctx.0.update(|n| *n += 1);
                        }
                        match result {
                            Ok(()) => { toast.success(t!("backup.export_success_all")()); }
                            Err(error) => {
                                log_error(ErrorLogDraft {
                                    error_type: "export_failed".to_string(),
                                    error_message: error.clone(),
                                    stack_trace: stack_trace(),
                                    user_action: Some("export backup from footer".to_string()),
                                    route: current_route(),
                                    vendor_id: None,
                                    purchase_id: None,
                                    details: vec!["scope=all".to_string()],
                                });
                                toast.error(format!("{}: {error}", t!("backup.export_failed")()));
                            }
                        }
                    });
                };

                view! {
                    <div
                        role="status"
                        aria-live="polite"
                        class="flex flex-col items-center gap-2 sm:flex-row sm:justify-center"
                    >
                        <div class="inline-flex items-center gap-1.5 rounded-full border border-amber-300 bg-amber-100 px-3 py-1 text-xs font-bold uppercase tracking-wide text-amber-800">
                            <span class="text-amber-600">"⚠"</span>
                            <span>{pill_label}</span>
                        </div>
                        <span class="text-sm text-amber-900">{detail_text}</span>
                        <div class="flex items-center gap-2">
                            <a
                                href={backups_href}
                                class="text-sm font-medium text-amber-800 underline hover:text-amber-900"
                            >
                                {t!("backup.storage_indicator_link")}
                            </a>
                            <button
                                on:click=handle_export
                                disabled=move || is_exporting.get()
                                class="inline-flex items-center rounded border border-amber-400 bg-amber-200 px-2.5 py-1 text-xs font-semibold text-amber-900 transition-colors hover:bg-amber-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber-500 disabled:opacity-60"
                            >
                                {move || if is_exporting.get() {
                                    t!("backup.export_in_progress")()
                                } else {
                                    t!("backup.create_backup")()
                                }}
                            </button>
                        </div>
                    </div>
                }.into_view()
            }
        }}
    }
}

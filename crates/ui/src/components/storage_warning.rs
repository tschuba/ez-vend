use std::sync::Arc;

use chrono::{DateTime, Utc};
use leptos::leptos_dom::helpers::{window_event_listener_untyped, WindowListenerHandle};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::state::use_app_state;
use crate::t;
use crate::utils::detect_browser;

use super::button::{Button, ButtonVariant};
use super::use_toast;
use crate::error_logging::{current_route, stack_trace, use_error_logger, ErrorLogDraft};
use crate::utils::{current_device_info, download_text_file};
use ez_vend_storage::record_backup_completed;

/// Context signal that any component (e.g. ExportButton) can increment to
/// trigger the footer to reload its storage diagnostics.
#[derive(Clone, Copy)]
pub struct StorageStatusRefreshContext(pub RwSignal<u32>);

const BACKUP_OVERDUE_DAYS: i64 = 30;
const SAFARI_BACKUP_OVERDUE_DAYS: i64 = 3;

/// Which of the four footer states applies. Safari overdue takes precedence
/// over unsaved changes, which take precedence over plain overdue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackupStatus {
    Ok,
    Changes,
    Overdue,
    SafariOverdue,
}

pub(crate) fn backup_status(
    last_modified_at: Option<DateTime<Utc>>,
    last_backup_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    safari: bool,
) -> BackupStatus {
    let has_changes = match (last_modified_at, last_backup_at) {
        (Some(modified), Some(backup)) => modified > backup,
        (Some(_), None) => true,
        (None, _) => false,
    };
    let overdue_after = |days: i64| match last_backup_at {
        Some(at) => now.signed_duration_since(at).num_days() >= days,
        None => true,
    };

    if safari && overdue_after(SAFARI_BACKUP_OVERDUE_DAYS) {
        BackupStatus::SafariOverdue
    } else if has_changes {
        BackupStatus::Changes
    } else if !safari && overdue_after(BACKUP_OVERDUE_DAYS) {
        BackupStatus::Overdue
    } else {
        BackupStatus::Ok
    }
}

fn blur_active_element() {
    let active = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
    if let Some(element) = active {
        let _ = element.blur();
    }
}

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

/// The smart footer storage status component.
///
/// The pill shows one of four states (see [`BackupStatus`]) and opens a
/// non-modal popup with an explanation and an immediate export button.
/// The popup is visible while pinned (`open`), hovered with a mouse, or
/// reached via keyboard focus.
///
/// Accepts a refresh signal via `StorageStatusRefreshContext` — increment it
/// to force a diagnostics reload (e.g. after a successful backup export).
#[component]
pub fn StorageIndicator() -> impl IntoView {
    let app_state = use_app_state();
    let toast = use_toast();
    let log_error = use_error_logger();
    let refresh_context = use_context::<StorageStatusRefreshContext>();

    let (diagnostics, set_diagnostics) = signal(None::<ez_vend_storage::StorageDiagnostics>);
    let safari = is_safari();

    let open = RwSignal::new(false);
    let hovered = RwSignal::new(false);
    let (is_exporting, set_is_exporting) = signal(false);
    let root_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
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

    // Memo keeps the popup DOM (and its open/exporting state) stable across
    // polls that return unchanged diagnostics.
    let view_state = Memo::new(move |_| {
        diagnostics.get().map(|d| {
            (
                backup_status(d.last_modified_at, d.last_backup_at, Utc::now(), safari),
                d.last_backup_at,
            )
        })
    });

    let close_popup = move || {
        open.set(false);
        hovered.set(false);
        blur_active_element();
    };

    // Escape always closes; a pointerdown outside only matters while pinned.
    Effect::<LocalStorage>::new(move |previous: Option<Vec<WindowListenerHandle>>| {
        drop(previous);
        let mut handles = vec![window_event_listener_untyped("keydown", move |event| {
            let event: web_sys::KeyboardEvent = event.unchecked_into();
            if event.key() == "Escape" {
                close_popup();
            }
        })];
        if open.get() {
            handles.push(window_event_listener_untyped("pointerdown", move |event| {
                let inside = event
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                    .zip(root_ref.get_untracked())
                    .is_some_and(|(target, root)| root.contains(Some(&target)));
                if !inside {
                    open.set(false);
                }
            }));
        }
        handles
    });

    let handle_export = Arc::new(move || {
        if is_exporting.get_untracked() {
            return;
        }
        let state_result = app_state.get();
        set_is_exporting.set(true);
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
            if let Some(ctx) = refresh_context {
                ctx.0.update(|n| *n += 1);
            }
            match result {
                Ok(()) => {
                    toast.success(t!("backup.export_success_all")());
                    close_popup();
                }
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
    });

    let toggle_popup = move |_| {
        if open.get_untracked() {
            close_popup();
        } else {
            open.set(true);
        }
    };

    // Age text re-evaluates with every diagnostics poll so it does not go stale.
    let age_text = move |last_backup_at: Option<DateTime<Utc>>| {
        diagnostics.with(|_| ());
        last_backup_at
            .map(format_backup_age)
            .unwrap_or_else(|| t!("backup.popup_never")())
    };

    view! {
        {move || {
            let Some((status, last_backup_at)) = view_state.get() else {
                // Loading state — plain pill, no popup
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
                }.into_any();
            };

            let (pill_classes, pill_label) = match status {
                BackupStatus::Ok => (
                    "border-green-200 bg-green-50 font-medium text-green-800 focus-visible:ring-green-400",
                    t!("backup.status_ok_label")(),
                ),
                BackupStatus::Overdue => (
                    "border-amber-300 bg-amber-100 font-bold text-amber-800 focus-visible:ring-amber-400",
                    t!("backup.status_overdue_label")(),
                ),
                BackupStatus::Changes | BackupStatus::SafariOverdue => (
                    "border-amber-300 bg-amber-100 font-bold text-amber-800 focus-visible:ring-amber-400",
                    t!("backup.status_warning_label")(),
                ),
            };
            let (title, body, export_label, export_variant) = match status {
                BackupStatus::Ok => (
                    t!("backup.popup_title_ok")(),
                    t!("backup.popup_body_ok")(),
                    t!("backup.popup_export")(),
                    ButtonVariant::Secondary,
                ),
                BackupStatus::Changes => (
                    t!("backup.popup_title_changes")(),
                    t!("backup.popup_body_changes")(),
                    t!("backup.popup_export")(),
                    ButtonVariant::Primary,
                ),
                BackupStatus::Overdue => (
                    t!("backup.popup_title_overdue")(),
                    t!("backup.popup_body_overdue")(),
                    t!("backup.popup_export")(),
                    ButtonVariant::Primary,
                ),
                BackupStatus::SafariOverdue => (
                    t!("backup.popup_title_safari")(),
                    t!("backup.popup_body_safari")(),
                    t!("backup.popup_export")(),
                    ButtonVariant::Primary,
                ),
            };
            let dot = if status == BackupStatus::Ok {
                view! { <span class="h-2 w-2 rounded-full bg-green-500"></span> }.into_any()
            } else {
                view! { <span class="text-amber-600">"⚠"</span> }.into_any()
            };
            let on_export = {
                let handle_export = Arc::clone(&handle_export);
                Box::new(move || handle_export()) as Box<dyn Fn()>
            };

            view! {
                <div
                    node_ref=root_ref
                    class="group relative"
                    on:pointerenter=move |event: web_sys::PointerEvent| {
                        // Touch has no real hover; only a mouse shows the popup on hover.
                        if event.pointer_type() == "mouse" {
                            hovered.set(true);
                        }
                    }
                    on:pointerleave=move |_| hovered.set(false)
                >
                    <button
                        type="button"
                        class=format!(
                            "inline-flex cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1 text-xs uppercase tracking-wide focus-visible:outline-none focus-visible:ring-2 {pill_classes}"
                        )
                        aria-expanded=move || (open.get() || hovered.get()).to_string()
                        aria-controls="storage-status-popup"
                        on:click=toggle_popup
                    >
                        {dot}
                        <span>{pill_label}</span>
                        <span class="ml-0.5 text-[10px] opacity-60">"ⓘ"</span>
                    </button>
                    // Wrapper has no gap to the pill (pb-2 instead of mb-2), so the
                    // mouse can travel from pill to popup without leaving the group.
                    <div class=move || format!(
                        "absolute bottom-full left-0 z-50 pb-2 {}",
                        if open.get() || hovered.get() {
                            "block"
                        } else {
                            "hidden group-has-[:focus-visible]:block"
                        }
                    )>
                        <div
                            id="storage-status-popup"
                            role="dialog"
                            aria-labelledby="storage-status-popup-title"
                            class="relative w-72 max-w-[calc(100vw-2rem)] rounded-lg border border-gray-200 bg-white p-4 text-left text-sm text-gray-700 shadow-lg"
                        >
                            <div class="flex items-start justify-between gap-2">
                                <p id="storage-status-popup-title" class="font-semibold text-gray-900">
                                    {title}
                                </p>
                                <button
                                    type="button"
                                    class="-mr-3 -mt-3 flex h-11 w-11 shrink-0 touch-manipulation items-center justify-center rounded-md text-gray-500 hover:bg-gray-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
                                    aria-label=t!("backup.popup_close")
                                    on:click=move |_| close_popup()
                                >
                                    "✕"
                                </button>
                            </div>
                            <p class="mt-1 leading-relaxed">
                                {move || body.replace("{age}", &age_text(last_backup_at))}
                            </p>
                            <Button
                                variant=export_variant
                                full_width=true
                                class="mt-3 min-h-[44px] touch-manipulation".to_string()
                                disabled=is_exporting
                                on_click=on_export
                            >
                                {move || {
                                    if is_exporting.get() {
                                        t!("backup.export_in_progress")()
                                    } else {
                                        export_label.clone()
                                    }
                                }}
                            </Button>
                            // Caret
                            <div class="absolute -bottom-1.5 left-6 h-3 w-3 rotate-45 border-b border-r border-gray-200 bg-white"></div>
                        </div>
                    </div>
                </div>
            }.into_any()
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn now() -> DateTime<Utc> {
        "2026-06-15T12:00:00Z".parse().unwrap()
    }

    fn ago(days: i64) -> Option<DateTime<Utc>> {
        Some(now() - Duration::days(days) - Duration::minutes(1))
    }

    #[test]
    fn fresh_install_without_backup_is_overdue() {
        assert_eq!(
            backup_status(None, None, now(), false),
            BackupStatus::Overdue
        );
    }

    #[test]
    fn recent_backup_without_changes_is_ok() {
        assert_eq!(
            backup_status(ago(5), ago(5), now(), false),
            BackupStatus::Ok
        );
    }

    #[test]
    fn modification_after_backup_is_changes() {
        assert_eq!(
            backup_status(ago(1), ago(5), now(), false),
            BackupStatus::Changes
        );
    }

    #[test]
    fn modification_without_any_backup_is_changes() {
        assert_eq!(
            backup_status(ago(1), None, now(), false),
            BackupStatus::Changes
        );
    }

    #[test]
    fn backup_older_than_30_days_is_overdue_on_non_safari() {
        assert_eq!(backup_status(None, ago(29), now(), false), BackupStatus::Ok);
        assert_eq!(
            backup_status(None, ago(30), now(), false),
            BackupStatus::Overdue
        );
    }

    #[test]
    fn safari_backup_older_than_3_days_is_safari_overdue() {
        assert_eq!(backup_status(None, ago(2), now(), true), BackupStatus::Ok);
        assert_eq!(
            backup_status(None, ago(3), now(), true),
            BackupStatus::SafariOverdue
        );
    }

    #[test]
    fn safari_without_backup_is_safari_overdue() {
        assert_eq!(
            backup_status(None, None, now(), true),
            BackupStatus::SafariOverdue
        );
    }

    #[test]
    fn safari_overdue_takes_precedence_over_changes() {
        assert_eq!(
            backup_status(ago(1), ago(10), now(), true),
            BackupStatus::SafariOverdue
        );
    }

    #[test]
    fn safari_with_recent_backup_and_changes_is_changes() {
        assert_eq!(
            backup_status(ago(0), ago(1), now(), true),
            BackupStatus::Changes
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use super::*;
    use crate::components::ToastProvider;
    use crate::i18n::provide_i18n;
    use crate::state::provide_app_state;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    async fn sleep_ms(ms: i32) {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                .unwrap();
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }

    fn popup_wrapper_class() -> Option<String> {
        let document = web_sys::window()?.document()?;
        let popup = document.get_element_by_id("storage-status-popup")?;
        Some(popup.parent_element()?.class_name())
    }

    #[wasm_bindgen_test]
    async fn popup_opens_on_click_and_closes_on_escape() {
        let _mount = leptos::mount::mount_to_body(|| {
            provide_i18n();
            provide_context(provide_app_state(None));
            view! { <ToastProvider><StorageIndicator /></ToastProvider> }
        });

        // Wait for diagnostics to load; until then there is no popup (loading state).
        let mut waited = 0;
        while popup_wrapper_class().is_none() && waited < 5000 {
            sleep_ms(50).await;
            waited += 50;
        }
        let closed = popup_wrapper_class().expect("popup rendered after diagnostics loaded");
        assert!(closed.contains("hidden"), "popup starts closed: {closed}");

        let document = web_sys::window().unwrap().document().unwrap();
        let pill = document
            .query_selector("button[aria-controls='storage-status-popup']")
            .unwrap()
            .expect("pill button")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        pill.click();
        sleep_ms(50).await;
        let opened = popup_wrapper_class().unwrap();
        assert!(opened.split(' ').any(|c| c == "block"), "opened: {opened}");
        assert_eq!(pill.get_attribute("aria-expanded").as_deref(), Some("true"));

        js_sys::eval("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))")
            .unwrap();
        sleep_ms(50).await;
        let after_escape = popup_wrapper_class().unwrap();
        assert!(
            after_escape.split(' ').any(|c| c == "hidden"),
            "closed after Escape: {after_escape}"
        );
    }
}

use crate::selected_booth_context::use_selected_booth;
use crate::state::{use_app_state, AppState};
use crate::utils::current_device_info;
use chrono::{Duration, Utc};
use ez_vend_storage::{ErrorLogContext, ErrorLogDeviceInfo, ErrorLogEntry};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone, Debug, Default)]
pub struct ErrorLogDraft {
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub user_action: Option<String>,
    pub route: Option<String>,
    pub vendor_id: Option<String>,
    pub purchase_id: Option<String>,
    pub details: Vec<String>,
}

pub fn log_error_with_state(state: AppState, draft: ErrorLogDraft, booth_id: Option<String>) {
    spawn_local(async move {
        let device_info = current_device_info();
        let entry = ErrorLogEntry {
            id: None,
            timestamp: Utc::now(),
            session_id: state.session_id.clone(),
            error_type: draft.error_type,
            error_message: draft.error_message,
            stack_trace: draft.stack_trace,
            context: Some(ErrorLogContext {
                booth_id,
                vendor_id: draft.vendor_id,
                purchase_id: draft.purchase_id,
                route: draft.route,
                user_action: draft.user_action,
                details: draft.details,
            }),
            device_info: ErrorLogDeviceInfo {
                identifier: device_info.identifier,
                platform: device_info.platform,
                browser: device_info.browser,
            },
        };

        if let Err(error) = state.log_error(&entry).await {
            log::error!("Failed to persist error log entry: {}", error);
        }
    });
}

pub fn use_error_logger() -> impl Fn(ErrorLogDraft) + Clone {
    let app_state = use_app_state();
    let selected_booth = use_selected_booth();

    move |draft: ErrorLogDraft| {
        let state_result = app_state.get();
        let booth_id = selected_booth
            .get_untracked()
            .map(|booth| booth.id.as_str());

        match state_result {
            Some(Ok(state)) => log_error_with_state(state, draft, booth_id),
            Some(Err(error)) => {
                log::error!("Failed to access app state for error logging: {}", error);
            }
            None => {
                log::warn!("App state not ready while attempting to log an error");
            }
        }
    }
}

pub fn stack_trace() -> Option<String> {
    let error = js_sys::Error::new("");
    js_sys::Reflect::get(error.as_ref(), &wasm_bindgen::JsValue::from_str("stack"))
        .ok()
        .and_then(|value| value.as_string())
        .filter(|stack: &String| !stack.trim().is_empty())
}

pub fn current_route() -> Option<String> {
    web_sys::window().and_then(|window| window.location().pathname().ok())
}

pub fn recent_error_cutoff() -> chrono::DateTime<Utc> {
    Utc::now() - Duration::hours(24)
}

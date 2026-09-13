#![allow(clippy::clone_on_copy)]
#![allow(clippy::needless_borrows_for_generic_args)]

use crate::components::{Icon, LuCopy, LuX};
use crate::{t, utils::copy_text_to_clipboard};
use leptos::*;
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;

/// Toast notification type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastType {
    fn icon(&self) -> &'static str {
        match self {
            ToastType::Success => "✓",
            ToastType::Error => "✕",
            ToastType::Warning => "⚠",
            ToastType::Info => "ℹ",
        }
    }

    fn bg_color(&self) -> &'static str {
        match self {
            ToastType::Success => "bg-green-600",
            ToastType::Error => "bg-red-600",
            ToastType::Warning => "bg-yellow-600",
            ToastType::Info => "bg-blue-600",
        }
    }
}

/// Individual toast notification
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub full_message: Option<String>,
    pub toast_type: ToastType,
    pub duration_ms: u32,
}

/// Toast context for managing notifications
#[derive(Clone, Copy)]
pub struct ToastContext {
    toasts: RwSignal<Vec<Toast>>,
    next_id: RwSignal<usize>,
}

impl ToastContext {
    pub fn new() -> Self {
        Self {
            toasts: create_rw_signal(Vec::new()),
            next_id: create_rw_signal(0),
        }
    }

    /// Show a toast notification
    pub fn show(&self, message: String, toast_type: ToastType, duration_ms: u32) {
        self.show_with_full(message, None, toast_type, duration_ms);
    }

    /// Show a toast notification with a full message for copying
    pub fn show_with_full(
        &self,
        message: String,
        full_message: Option<String>,
        toast_type: ToastType,
        duration_ms: u32,
    ) {
        let id = self.next_id.get();
        self.next_id.update(|n| *n += 1);

        let toast = Toast {
            id,
            message,
            full_message,
            toast_type,
            duration_ms,
        };

        self.toasts.update(|toasts| toasts.push(toast.clone()));

        // Auto-remove after duration
        let toasts = self.toasts;
        set_timeout(
            move || {
                toasts.update(|t| t.retain(|item| item.id != id));
            },
            Duration::from_millis(duration_ms as u64),
        );
    }

    /// Show success toast
    pub fn success(&self, message: impl Into<String>) {
        self.show(message.into(), ToastType::Success, 3000);
    }

    /// Show error toast
    pub fn error(&self, message: impl Into<String>) {
        self.show(message.into(), ToastType::Error, 5000);
    }

    /// Show error toast with full message for copying
    #[allow(dead_code)]
    pub fn error_with_full(&self, message: impl Into<String>, full_message: impl Into<String>) {
        self.show_with_full(
            message.into(),
            Some(full_message.into()),
            ToastType::Error,
            5000,
        );
    }

    /// Show warning toast
    pub fn warning(&self, message: impl Into<String>) {
        self.show(message.into(), ToastType::Warning, 4000);
    }

    /// Show info toast
    pub fn info(&self, message: impl Into<String>) {
        self.show(message.into(), ToastType::Info, 3000);
    }

    /// Manually dismiss a toast
    pub fn dismiss(&self, id: usize) {
        self.toasts.update(|toasts| toasts.retain(|t| t.id != id));
    }

    /// Get current toasts
    pub fn get_toasts(&self) -> Signal<Vec<Toast>> {
        self.toasts.into()
    }
}

/// Provider component for toast context
#[component]
pub fn ToastProvider(children: Children) -> impl IntoView {
    let toast_context = ToastContext::new();
    provide_context(toast_context.clone());

    view! {
        {children()}
        <ToastContainer />
    }
}

/// Hook to use toast context
pub fn use_toast() -> ToastContext {
    expect_context::<ToastContext>()
}

/// Container component that displays toasts
#[component]
fn ToastContainer() -> impl IntoView {
    let toast_context = use_toast();
    let toasts = toast_context.get_toasts();

    view! {
        <div
            class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none print:hidden"
            role="region"
            aria-label="Notifications"
            aria-live="polite"
        >
            <For
                each=move || toasts.get()
                key=|toast| toast.id
                children=move |toast: Toast| {
                    let toast_context = use_toast();
                    let toast_id = toast.id;

                    view! {
                        <ToastItem
                            toast=toast
                            on_dismiss=move || toast_context.dismiss(toast_id)
                        />
                    }
                }
            />
        </div>
    }
}

/// Individual toast item component
#[component]
fn ToastItem(toast: Toast, on_dismiss: impl Fn() + 'static) -> impl IntoView {
    let bg_color = toast.toast_type.bg_color();
    let icon = toast.toast_type.icon();

    let is_error = toast.toast_type == ToastType::Error;

    view! {
        <div
            class=format!(
                "relative flex items-start gap-3 {} text-white pl-4 pr-16 py-3 rounded-lg shadow-lg max-w-sm pointer-events-auto animate-slide-in",
                bg_color
            )
            role="alert"
        >
            <span class="text-xl font-bold mt-0.5 flex-shrink-0">{icon}</span>
            <p class="flex-1 text-sm pr-2">{toast.message.clone()}</p>
            <div class="absolute top-2 right-2 flex items-center gap-1">
                {is_error.then(|| view! {
                    <button
                        type="button"
                        class="text-white hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-white rounded p-1"
                        on:click=move |_| {
                            let message_to_copy =
                                toast.full_message.clone().unwrap_or_else(|| toast.message.clone());
                            spawn_local(async move {
                                match copy_text_to_clipboard(&message_to_copy).await {
                                    Ok(()) => use_toast().info(&t!("components.toast.copied")()),
                                    Err(error) => {
                                        log::warn!("Failed to copy toast message: {error}");
                                        use_toast().error(&t!("components.toast.copy_failed")());
                                    }
                                }
                            });
                        }
                        aria-label=t!("components.toast.copy")()
                    >
                        <Icon icon=LuCopy class="w-4 h-4" />
                    </button>
                })}
                <button
                    type="button"
                    class="text-white hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-white rounded p-1"
                    on:click=move |_| on_dismiss()
                    aria-label="Dismiss notification"
                >
                    <Icon icon=LuX class="w-4 h-4" />
                </button>
            </div>
        </div>
    }
}

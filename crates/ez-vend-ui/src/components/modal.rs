#![allow(clippy::clone_on_copy)]

use leptos::leptos_dom::helpers::{window_event_listener_untyped, WindowListenerHandle};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::{Icon, LuX};

/// Modal size variants
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModalSize {
    Small,
    Medium,
    Large,
    XLarge,
    ExtraLarge,
    FullScreen,
}

impl ModalSize {
    fn max_width_class(&self) -> &'static str {
        match self {
            ModalSize::Small => "max-w-sm",
            ModalSize::Medium => "max-w-md",
            ModalSize::Large => "max-w-2xl",
            ModalSize::XLarge => "max-w-3xl",
            ModalSize::ExtraLarge => "max-w-4xl",
            ModalSize::FullScreen => "max-w-full",
        }
    }
}

/// Modal component properties
#[component]
pub fn Modal(
    /// Whether the modal is open
    #[prop(into)]
    show: Signal<bool>,
    /// Callback when modal should close
    on_close: impl Fn() + 'static + Clone,
    /// Modal title
    #[prop(optional, into)]
    title: Option<Signal<String>>,
    /// Optional actions rendered in the header
    #[prop(optional)]
    header_actions: Option<AnyView>,
    /// Modal size
    #[prop(default = ModalSize::Medium)]
    size: ModalSize,
    /// Whether to show close button
    #[prop(default = true)]
    show_close_button: bool,
    /// Whether clicking overlay closes modal
    #[prop(default = true)]
    close_on_overlay_click: bool,
    /// Optional actions rendered in a fixed footer
    #[prop(optional)]
    action_bar: Option<AnyView>,
    /// Child content
    children: Children,
) -> impl IntoView {
    let on_close_stored = StoredValue::new_local(on_close.clone());
    let title_stored = StoredValue::new_local(title);
    let has_header_actions = header_actions.is_some();
    let has_title = title_stored.get_value().is_some();
    let has_action_bar = action_bar.is_some();
    let children_view = children();

    // Close on Escape key; listener lives for the component's lifetime, guarded by show check.
    // ponytail: Effect::<LocalStorage>::new avoids Send+Sync on on_close
    Effect::<LocalStorage>::new(move |prev: Option<WindowListenerHandle>| {
        drop(prev);
        let on_close_esc = on_close.clone();
        window_event_listener_untyped("keydown", move |event| {
            if !show.get_untracked() {
                return;
            }
            let event: web_sys::KeyboardEvent = event.unchecked_into();
            if event.key() == "Escape" {
                on_close_esc();
            }
        })
    });

    let overlay_click = move |_| {
        if close_on_overlay_click {
            on_close_stored.with_value(|f| f());
        }
    };

    let content_click = move |e: web_sys::MouseEvent| {
        e.stop_propagation();
    };

    let close_button_click = move |_| {
        on_close_stored.with_value(|f| f());
    };

    // ponytail: CSS-based show/hide instead of <Show> so non-Clone AnyView props render once
    view! {
        <div
            class=move || if show.get() {
                "fixed inset-0 z-[60] overflow-y-auto print:hidden"
            } else {
                "hidden"
            }
            aria-labelledby="modal-title"
            role="dialog"
            aria-modal=move || show.get().to_string()
        >
            // Overlay
            <div
                class="fixed inset-0 bg-black bg-opacity-50 transition-opacity"
                on:click=overlay_click
            ></div>

            // Modal container
            <div class="flex min-h-full items-center justify-center p-4">
                <div
                    class=format!(
                        "relative bg-white rounded-lg shadow-xl overflow-hidden {} w-full max-h-[calc(100vh-2rem)] flex flex-col transform transition-all",
                        size.max_width_class()
                    )
                    on:click=content_click
                >
                    // Header (static — rendered once)
                    {(has_title || has_header_actions || show_close_button).then(|| view! {
                        <div class="flex items-center justify-between p-4 border-b border-gray-200">
                            {has_title.then(|| view! {
                                <h3
                                    id="modal-title"
                                    class="text-lg font-semibold text-gray-900"
                                >
                                    {move || {
                                        title_stored
                                            .get_value()
                                            .map(|title| title.get())
                                            .unwrap_or_default()
                                    }}
                                </h3>
                            })}
                            <div class="flex items-center gap-2">
                                {header_actions}
                                {show_close_button.then(|| view! {
                                    <button
                                        type="button"
                                        class="text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 rounded"
                                        on:click=close_button_click
                                        aria-label="Close modal"
                                    >
                                        <Icon icon=LuX class="w-6 h-6" />
                                    </button>
                                })}
                            </div>
                        </div>
                    })}

                    // Body
                    <div class="min-h-0 flex-1 overflow-y-auto p-4">
                        {children_view}
                    </div>

                    {has_action_bar.then(|| view! {
                        <div class="border-t border-gray-200 bg-white p-4">
                            <div class="flex justify-end gap-2">
                                {action_bar}
                            </div>
                        </div>
                    })}
                </div>
            </div>
        </div>
    }
}

/// Confirmation modal with actions
#[component]
pub fn ConfirmModal(
    /// Whether the modal is open
    #[prop(into)]
    show: Signal<bool>,
    /// Callback when modal should close
    on_close: impl Fn() + 'static + Clone,
    /// Callback when confirmed
    on_confirm: impl Fn() + 'static + Clone,
    /// Modal title
    #[prop(into)]
    title: Signal<String>,
    /// Confirmation message (can be a signal or string)
    #[prop(into)]
    message: Signal<String>,
    /// Confirm button text
    #[prop(default = Signal::derive(|| "Confirm".to_string()), into)]
    confirm_text: Signal<String>,
    /// Cancel button text
    #[prop(default = Signal::derive(|| "Cancel".to_string()), into)]
    cancel_text: Signal<String>,
    /// Whether confirm action is destructive (uses danger styling)
    #[prop(default = false)]
    is_destructive: bool,
) -> impl IntoView {
    let on_close_for_confirm = on_close.clone();
    let on_close_for_cancel = on_close.clone();
    let on_close_for_modal = on_close.clone();

    let on_confirm_click = move |_| {
        on_confirm();
        on_close_for_confirm();
    };

    let on_cancel_click = move |_| {
        on_close_for_cancel();
    };

    let confirm_button_class = if is_destructive {
        "px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500"
    } else {
        "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
    };

    view! {
        <Modal
            show=show
            on_close=on_close_for_modal
            title=title
            size=ModalSize::Small
            action_bar=
                view! {
                    <div class="contents">
                        <button
                            type="button"
                            class="px-4 py-2 bg-gray-200 text-gray-800 rounded hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-500"
                            on:click=on_cancel_click
                        >
                            {move || cancel_text.get()}
                        </button>
                        <button
                            type="button"
                            class=confirm_button_class
                            on:click=on_confirm_click
                        >
                            {move || confirm_text.get()}
                        </button>
                    </div>
                }
                .into_any()
        >
            <div class="space-y-4">
                <p class="text-gray-700">{move || message.get()}</p>
            </div>
        </Modal>
    }
}

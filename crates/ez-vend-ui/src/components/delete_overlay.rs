use leptos::*;

use crate::components::{Icon, LuTrash2};

/// Shared red confirmation overlay used for two-click destructive actions.
#[component]
pub fn DeleteOverlay(
    #[prop(into)] prompt: String,
    #[prop(into, optional)] aria_label: Option<String>,
    #[prop(into)] on_click: Callback<ev::MouseEvent>,
) -> impl IntoView {
    let aria_prompt = aria_label.unwrap_or_else(|| prompt.clone());
    view! {
        <div
            class="absolute inset-0 rounded-lg cursor-pointer z-10 transition-all pointer-events-auto"
            style="background: rgba(220, 38, 38, 0.7); backdrop-filter: blur(2px);"
            role="alertdialog"
            aria-label=aria_prompt
            on:click=move |ev| {
                ev.stop_propagation();
                on_click.call(ev);
            }
        >
            <div class="flex items-center justify-center gap-3 h-full">
                <Icon icon=LuTrash2 class="w-8 h-8 text-white flex-shrink-0" />

                <p class="text-white text-base font-semibold text-center">
                    {prompt}
                </p>
            </div>
        </div>
    }
}

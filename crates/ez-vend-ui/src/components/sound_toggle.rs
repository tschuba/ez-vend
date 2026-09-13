use crate::components::{Icon, LuVolume2, LuVolumeX};
use crate::t;
use leptos::*;

#[component]
pub fn SoundToggle(enabled: Signal<bool>, on_toggle: Callback<()>) -> impl IntoView {
    view! {
        <button
            type="button"
            class=move || {
                if enabled.get() {
                    "inline-flex items-center rounded-full border border-amber-300 bg-amber-50 px-4 py-2 text-sm font-semibold text-amber-900 shadow-sm transition-colors hover:bg-amber-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                } else {
                    "inline-flex items-center rounded-full border border-slate-200 bg-white/80 px-4 py-2 text-sm font-semibold text-slate-700 shadow-sm backdrop-blur transition-colors hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                }
            }
            aria-label=move || {
                if enabled.get() {
                    t!("checkout.error_sound_disable")()
                } else {
                    t!("checkout.error_sound_enable")()
                }
            }
            aria-pressed=move || if enabled.get() { "true" } else { "false" }
            title=move || {
                if enabled.get() {
                    t!("checkout.error_sound_disable")()
                } else {
                    t!("checkout.error_sound_enable")()
                }
            }
            on:click=move |_| on_toggle.call(())
        >
            <Show
                when=move || enabled.get()
                fallback=move || view! { <Icon icon=LuVolumeX class="h-5 w-5" /> }
            >
                <Icon icon=LuVolume2 class="h-5 w-5" />
            </Show>
        </button>
    }
}

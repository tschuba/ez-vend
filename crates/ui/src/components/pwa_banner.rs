use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::t;

fn detect_ios_not_standalone() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    matches!(
        js_sys::Reflect::get(navigator.as_ref(), &JsValue::from_str("standalone")),
        Ok(v) if v == JsValue::FALSE
    )
}

fn pwa_dismissed() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item("ez-vend-pwa-dismissed").ok().flatten())
        .is_some()
}

#[component]
pub fn PwaBanner(banner_visible: RwSignal<bool>) -> impl IntoView {
    banner_visible.set(detect_ios_not_standalone() && !pwa_dismissed());

    let dismiss = move |_| {
        if let Some(Ok(Some(ls))) = web_sys::window().map(|w| w.local_storage()) {
            let _ = ls.set_item("ez-vend-pwa-dismissed", "1");
        }
        banner_visible.set(false);
    };

    view! {
        <Show when=move || banner_visible.get()>
            <div class="border-b border-gray-100 bg-amber-50 px-4 py-2">
                <div class="relative flex items-start justify-center text-center text-sm">
                    <p class="text-amber-900">
                        <strong>{t!("pwa.title")}</strong>
                        " "
                        {t!("pwa.message")}
                    </p>
                    <button
                        on:click=dismiss
                        aria-label=t!("pwa.dismiss")
                        class="absolute right-0 shrink-0 text-amber-700 hover:text-amber-900 focus:outline-none"
                    >
                        "✕"
                    </button>
                </div>
            </div>
        </Show>
    }
}

use leptos::*;
use wasm_bindgen::prelude::*;

/// Entry point for the WASM application
#[wasm_bindgen(start)]
pub fn main() {
    // Set up console error panic hook for better debugging
    console_error_panic_hook::set_once();

    // Initialize WASM logger for console logging
    wasm_logger::init(wasm_logger::Config::default());

    // Mount the application
    leptos::mount_to_body(|| {
        view! { <ez_vend_ui::SelectedBoothProvider>
            <ez_vend_ui::App />
        </ez_vend_ui::SelectedBoothProvider> }
    });
}

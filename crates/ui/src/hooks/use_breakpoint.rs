use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::window;

/// Matches the `lg:` breakpoint (Tailwind default) used across the app's
/// desktop/mobile layout splits (e.g. the register card column-layout
/// classes in checkout, the vendor list master-detail layout).
pub const DESKTOP_BREAKPOINT_QUERY: &str = "(min-width: 1024px)";

/// Tracks whether `query` currently matches, via `window().match_media(...)`
/// plus a `MediaQueryList` change listener, so the signal stays in sync as
/// the viewport is resized.
pub fn use_media_query(query: &'static str) -> Signal<bool> {
    let initial = window()
        .and_then(|w| w.match_media(query).ok())
        .flatten()
        .map(|mql| mql.matches())
        .unwrap_or(false);
    let (matches, set_matches) = signal(initial);
    Effect::new(move |_| {
        let Some(mql) = window().and_then(|w| w.match_media(query).ok()).flatten() else {
            return;
        };
        set_matches.set(mql.matches());
        let callback = Closure::wrap(Box::new(move |event: web_sys::MediaQueryListEvent| {
            set_matches.set(event.matches());
        }) as Box<dyn Fn(_)>);
        mql.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())
            .expect("MediaQueryList addEventListener should not fail");
        callback.forget();
    });
    matches.into()
}

/// Whether the viewport is currently at or above the shared `lg:` breakpoint.
pub fn use_desktop_breakpoint() -> Signal<bool> {
    use_media_query(DESKTOP_BREAKPOINT_QUERY)
}

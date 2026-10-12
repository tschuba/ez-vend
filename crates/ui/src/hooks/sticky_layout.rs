use leptos::leptos_dom::helpers::{window_event_listener_untyped, WindowListenerHandle};
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::window;

const REM_PX: f64 = 16.0;

/// Measures the fixed `#app-header` and `#app-footer` elements' heights so
/// sticky panels below them (checkout's register card, the vendor list's
/// detail panel) can position themselves without overlapping - neither
/// element has a constant height (wrapped nav, PWA install banner, the
/// backup-warning pill in `StorageIndicator`, ...).
///
/// Header height is re-measured on `resize` (layout wrap changes only ever
/// come from viewport width changes). Footer height uses a `ResizeObserver`
/// instead, since it can also change from async content loads that never
/// fire a `resize` event.
pub fn use_header_footer_heights() -> (Signal<f64>, Signal<f64>) {
    let (header_height, set_header_height) = signal(56.0_f64);
    let measure_header_height = move || {
        if let Some(header) = window()
            .and_then(|w| w.document())
            .and_then(|document| document.get_element_by_id("app-header"))
        {
            set_header_height.set(header.get_bounding_client_rect().height());
        }
    };
    Effect::<LocalStorage>::new(move |prev: Option<WindowListenerHandle>| {
        drop(prev);
        measure_header_height();
        window_event_listener_untyped("resize", move |_| measure_header_height())
    });

    // Guessed baseline (border + copyright row, no banner/pill) so the
    // consumer isn't briefly wrong before this effect's first measurement.
    let (footer_height, set_footer_height) = signal(40.0_f64);
    Effect::new(move |_| {
        let Some(footer) = window()
            .and_then(|w| w.document())
            .and_then(|document| document.get_element_by_id("app-footer"))
        else {
            return;
        };
        set_footer_height.set(footer.get_bounding_client_rect().height());

        let callback = Closure::wrap(Box::new(move |entries: web_sys::js_sys::Array| {
            if let Some(entry) = entries.get(0).dyn_ref::<web_sys::ResizeObserverEntry>() {
                set_footer_height.set(entry.target().get_bounding_client_rect().height());
            }
        }) as Box<dyn Fn(_)>);
        let observer = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref())
            .expect("ResizeObserver construction should not fail");
        observer.observe(&footer);
        callback.forget();
        // ResizeObserver isn't kept alive by the observed element - only by
        // JS references to the observer itself, so it must be leaked here
        // too or it becomes eligible for GC and observation silently stops.
        std::mem::forget(observer);
    });

    (header_height.into(), footer_height.into())
}

/// The sticky/full-height `style` attribute for an element positioned below
/// the (measured) header and clipped above the (measured) footer -
/// `top_margin_rem` is the gap kept above the element's top edge, below the
/// header; `vertical_margin_rem` is the total vertical margin subtracted
/// from its available height (below the header, above the footer).
pub fn sticky_offset_style(
    header_height: f64,
    footer_height: f64,
    top_margin_rem: f64,
    vertical_margin_rem: f64,
) -> String {
    format!(
        "top: calc({header_height}px + {top_margin_rem}rem); height: calc(100dvh - {header_height}px - {footer_height}px - {vertical_margin_rem}rem);",
    )
}

/// Vertical space left for a sticky element once the header, footer and
/// layout margin are subtracted from the viewport - the same area
/// `sticky_offset_style` actually lays the element into.
pub fn sticky_available_height(
    viewport_height: f64,
    header_height: f64,
    footer_height: f64,
    vertical_margin_rem: f64,
) -> f64 {
    (viewport_height - header_height - footer_height - REM_PX * vertical_margin_rem).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticky_available_height_subtracts_header_and_footer_and_margin() {
        assert_eq!(sticky_available_height(800.0, 64.0, 40.0, 3.0), 648.0);
    }

    #[test]
    fn sticky_available_height_never_goes_negative() {
        assert_eq!(sticky_available_height(200.0, 300.0, 300.0, 3.0), 0.0);
    }

    #[test]
    fn sticky_offset_style_reserves_space_for_header_and_footer() {
        let style = sticky_offset_style(64.0, 40.0, 1.5, 3.0);
        assert_eq!(
            style,
            "top: calc(64px + 1.5rem); height: calc(100dvh - 64px - 40px - 3rem);"
        );
    }
}

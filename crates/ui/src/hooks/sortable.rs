use leptos::prelude::*;
use std::cell::Cell;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::PointerEvent;

/// Mouse/pen: the pointer must travel this far (px) before a drag starts;
/// anything less is a tap.
const DRAG_THRESHOLD_PX: f64 = 8.0;
/// Touch: finger must rest this long (ms) to arm a drag, so a normal swipe
/// still scrolls the page.
const LONG_PRESS_MS: i32 = 300;
/// Touch: moving further than this before the long-press fires means the
/// user is scrolling, not dragging.
const TOUCH_SLOP_PX: f64 = 10.0;
/// Auto-scroll kicks in within this distance (px) of the viewport top/bottom.
const EDGE_ZONE_PX: f64 = 60.0;
/// Auto-scroll speed (px per 16 ms tick) at the very edge.
const MAX_SCROLL_STEP_PX: f64 = 14.0;

thread_local! {
    /// True while a sort drag is armed; read by the document-level
    /// `touchmove` guard so the browser doesn't pan (and cancel the pointer)
    /// mid-drag.
    static DRAG_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static GUARD_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

/// iOS Safari only honours `preventDefault` on `touchmove` when a
/// non-passive listener already exists before the touch starts, so install
/// one once, for good.
fn install_touchmove_guard() {
    if GUARD_INSTALLED.with(|c| c.replace(true)) {
        return;
    }
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let cb = Closure::wrap(Box::new(|e: web_sys::Event| {
        if DRAG_ACTIVE.with(|c| c.get()) && e.cancelable() {
            e.prevent_default();
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_passive(false);
    let _ = doc.add_event_listener_with_callback_and_add_event_listener_options(
        "touchmove",
        cb.as_ref().unchecked_ref(),
        &opts,
    );
    cb.forget();
}

fn exceeds(dx: f64, dy: f64, limit: f64) -> bool {
    dx * dx + dy * dy >= limit * limit
}

/// Signed px to scroll per tick for a pointer at `y` in a viewport of
/// `height`; zero outside the edge zones.
fn edge_scroll_step(y: f64, height: f64) -> f64 {
    if y < EDGE_ZONE_PX {
        -((EDGE_ZONE_PX - y.max(0.0)) / EDGE_ZONE_PX) * MAX_SCROLL_STEP_PX
    } else if y > height - EDGE_ZONE_PX {
        ((y.min(height) - (height - EDGE_ZONE_PX)) / EDGE_ZONE_PX) * MAX_SCROLL_STEP_PX
    } else {
        0.0
    }
}

type Key = (String, usize);

/// Per-gesture bookkeeping that doesn't need to be reactive.
#[derive(Clone, Default)]
struct Gesture {
    active: bool,
    touch: bool,
    pointer_id: i32,
    start: (f64, f64),
    last: (f64, f64),
    /// Pointer position inside the dragged item at the moment of arming.
    grab: (f64, f64),
    key: Option<Key>,
    handler_el: Option<web_sys::Element>,
    item_el: Option<web_sys::Element>,
    long_press_timer: Option<i32>,
    scroll_interval: Option<i32>,
}

/// Whole-item drag-and-drop sort state, shared across every list in `scope`.
///
/// Items carry `data-sort-scope` (matched to `scope`), `data-sort-group`
/// (list/group id; may be absent for a single flat list, then `""`) and
/// `data-sort-index` (position within that group). The pointer handlers go
/// on the element the user touches; the element carrying the `data-sort-*`
/// attributes (found via `closest`) is the one that follows the pointer.
///
/// A tap (movement under the threshold) is left alone so the item's own
/// `on:click` fires; after a drag, `take_click_suppressed` tells that
/// handler to ignore the trailing click.
#[derive(Clone, Copy)]
pub struct SortableState {
    dragging: RwSignal<Option<Key>>,
    drag_over: RwSignal<Option<Key>>,
    /// Touch long-press in progress (not yet armed).
    pending: RwSignal<Option<Key>>,
    /// Current ghost translation of the dragged item, in px.
    offset: RwSignal<(f64, f64)>,
    gesture: StoredValue<Gesture, LocalStorage>,
    suppress_click_until: StoredValue<f64>,
    scope: &'static str,
    /// Class pair applied to the drop target: vertical lists get a top
    /// border, grids a bar in the gap left of the tile.
    marker: &'static str,
}

const ROW_MARKER: &str = "border-t-2 border-blue-400";
const TILE_MARKER: &str = "relative before:content-[''] before:absolute before:-left-1.5 before:top-0 before:bottom-0 before:w-1 before:rounded before:bg-blue-500";

fn new_state(scope: &'static str, marker: &'static str) -> SortableState {
    install_touchmove_guard();
    SortableState {
        dragging: RwSignal::new(None),
        drag_over: RwSignal::new(None),
        pending: RwSignal::new(None),
        offset: RwSignal::new((0.0, 0.0)),
        gesture: StoredValue::new_local(Gesture::default()),
        suppress_click_until: StoredValue::new(0.0),
        scope,
        marker,
    }
}

/// Sort state for a vertical list of rows (drop marker = top border).
/// Use group `""` for every item.
pub fn use_sortable(scope: &'static str) -> SortableState {
    new_state(scope, ROW_MARKER)
}

/// Sort state for tile grids spread over several groups, letting items move
/// between groups (drop marker = bar left of the target tile).
pub fn use_grouped_sortable(scope: &'static str) -> SortableState {
    new_state(scope, TILE_MARKER)
}

impl SortableState {
    /// Whether a drag is armed (not merely a pending long-press).
    pub fn is_dragging(self) -> impl Fn() -> bool {
        move || self.dragging.get().is_some()
    }

    /// True once right after a drag ended: the item's `on:click` should
    /// return early so a drop doesn't also open the edit modal.
    pub fn take_click_suppressed(self) -> bool {
        self.suppress_click_until
            .get_value()
            .gt(&js_sys::Date::now())
    }

    /// Reactive class string for the item at `(group, index)`; `base` is the
    /// always-applied classes. Covers the hold-to-drag pulse, the lifted
    /// ghost and the drop marker.
    pub fn item_classes(
        self,
        group: String,
        index: usize,
        base: &'static str,
    ) -> impl Fn() -> String {
        move || {
            let key = Some((group.clone(), index));
            let mut s = base.to_string();
            if self.dragging.get() == key {
                s.push_str(" relative z-50 shadow-xl ring-2 ring-blue-400 pointer-events-none");
            } else if self.pending.get() == key {
                s.push_str(" animate-pulse");
            } else if self.drag_over.get() == key {
                s.push(' ');
                s.push_str(self.marker);
            }
            s
        }
    }

    /// Reactive inline style: the ghost transform while this item is dragged.
    pub fn item_style(self, group: String, index: usize) -> impl Fn() -> String {
        move || {
            if self.dragging.get() == Some((group.clone(), index)) {
                let (dx, dy) = self.offset.get();
                format!("transform: translate({dx}px, {dy}px) scale(1.05);")
            } else {
                String::new()
            }
        }
    }

    /// Attach to `on:pointerdown` of the touchable element. `group`/`index`
    /// are closures so the current position is read at event time.
    pub fn on_pointerdown(
        self,
        group: impl Fn() -> String + 'static,
        index: impl Fn() -> usize + 'static,
    ) -> impl Fn(PointerEvent) {
        move |e: PointerEvent| {
            if !e.is_primary() || e.button() > 0 || self.dragging.get_untracked().is_some() {
                return;
            }
            let target = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok());
            // Buttons etc. inside the item keep their own behaviour.
            if target
                .and_then(|t| {
                    t.closest("button, a, input, textarea, select")
                        .ok()
                        .flatten()
                })
                .is_some()
            {
                return;
            }
            let Some(handler_el) = e
                .current_target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            else {
                return;
            };
            let item_el = handler_el
                .closest(&format!("[data-sort-scope=\"{}\"]", self.scope))
                .ok()
                .flatten();
            let key = (group(), index());
            let touch = e.pointer_type() == "touch";
            let at = (f64::from(e.client_x()), f64::from(e.client_y()));

            self.clear_timers();
            self.gesture.set_value(Gesture {
                active: true,
                touch,
                pointer_id: e.pointer_id(),
                start: at,
                last: at,
                key: Some(key.clone()),
                handler_el: Some(handler_el),
                item_el,
                ..Gesture::default()
            });

            if touch {
                self.pending.set(Some(key));
                let cb = Closure::once_into_js(move || self.arm());
                let id = web_sys::window().and_then(|w| {
                    w.set_timeout_with_callback_and_timeout_and_arguments_0(
                        cb.unchecked_ref(),
                        LONG_PRESS_MS,
                    )
                    .ok()
                });
                self.gesture.update_value(|g| g.long_press_timer = id);
            }
        }
    }

    /// Attach to `on:pointermove` of the same element.
    pub fn on_pointermove(self) -> impl Fn(PointerEvent) {
        move |e: PointerEvent| {
            let g = self.gesture.get_value();
            if !g.active || g.pointer_id != e.pointer_id() {
                return;
            }
            let at = (f64::from(e.client_x()), f64::from(e.client_y()));
            self.gesture.update_value(|g| g.last = at);
            if self.dragging.get_untracked().is_none() {
                let (dx, dy) = (at.0 - g.start.0, at.1 - g.start.1);
                if g.touch {
                    // User is scrolling — let go before the long-press fires.
                    if exceeds(dx, dy, TOUCH_SLOP_PX) {
                        self.reset();
                    }
                    return;
                }
                if !exceeds(dx, dy, DRAG_THRESHOLD_PX) {
                    return;
                }
                // Armed on this very move: fall through so a fast flick
                // still hit-tests the position it landed on.
                self.arm();
            }
            self.update_drag();
        }
    }

    /// Attach to `on:pointerup`. Calls `on_move(from_group, from_index,
    /// to_group, to_index)` when the drop changes the position; indices are
    /// "insert before this item" positions (`len` = append).
    pub fn on_pointerup(
        self,
        on_move: impl Fn(String, usize, String, usize) + Clone + 'static,
    ) -> impl Fn(PointerEvent) {
        move |_| {
            let from = self.dragging.get_untracked();
            let to = self.drag_over.get_untracked();
            let was_dragging = from.is_some();
            self.reset();
            if was_dragging {
                self.suppress_click_until
                    .set_value(js_sys::Date::now() + 300.0);
            }
            if let (Some((fg, fi)), Some((tg, ti))) = (from, to) {
                // Dropping onto itself or right before its successor is a no-op.
                if !(fg == tg && (ti == fi || ti == fi + 1)) {
                    on_move(fg, fi, tg, ti);
                }
            }
        }
    }

    /// Attach to `on:pointercancel` to abandon the gesture without moving.
    pub fn on_pointercancel(self) -> impl Fn(PointerEvent) {
        move |_| self.reset()
    }

    /// Start the drag: capture the pointer, lift the item, start auto-scroll.
    fn arm(self) {
        let g = self.gesture.get_value();
        let Some(key) = g.key.clone() else { return };
        if !g.active {
            return;
        }
        if let Some(el) = &g.handler_el {
            let _ = el.set_pointer_capture(g.pointer_id);
        }
        if let Some(item) = &g.item_el {
            let r = item.get_bounding_client_rect();
            self.gesture
                .update_value(|g| g.grab = (g.last.0 - r.left(), g.last.1 - r.top()));
        }
        DRAG_ACTIVE.with(|c| c.set(true));
        self.pending.set(None);
        self.offset.set((0.0, 0.0));
        self.dragging.set(Some(key.clone()));
        self.drag_over.set(Some(key));
        if let Some(w) = web_sys::window() {
            // ponytail: not every device has it (iOS Safari doesn't); the visual lift is the real cue
            let _ = w.navigator().vibrate_with_duration(10);
            let cb = Closure::wrap(Box::new(move || self.scroll_tick()) as Box<dyn FnMut()>);
            let id = w
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    16,
                )
                .ok();
            // ponytail: one tiny leaked closure per drag; hoist if drags ever get frequent
            cb.forget();
            self.gesture.update_value(|g| g.scroll_interval = id);
        }
    }

    /// Re-position the ghost under the pointer and hit-test the drop target.
    fn update_drag(self) {
        let g = self.gesture.get_value();
        if let Some(item) = &g.item_el {
            // Untransformed position = current rect minus the translate we
            // applied, so layout shifts and scrolling can't make it drift.
            let r = item.get_bounding_client_rect();
            let (ox, oy) = self.offset.get_untracked();
            self.offset.set((
                g.last.0 - g.grab.0 - (r.left() - ox),
                g.last.1 - g.grab.1 - (r.top() - oy),
            ));
        }
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let elements = doc.elements_from_point(g.last.0 as f32, g.last.1 as f32);
        for i in 0..elements.length() {
            let Ok(el) = elements.get(i).dyn_into::<web_sys::Element>() else {
                continue;
            };
            if el.get_attribute("data-sort-scope").as_deref() != Some(self.scope) {
                continue;
            }
            let group = el.get_attribute("data-sort-group").unwrap_or_default();
            if let Some(idx) = el
                .get_attribute("data-sort-index")
                .and_then(|s| s.parse::<usize>().ok())
            {
                self.drag_over.set(Some((group, idx)));
                return;
            }
        }
    }

    fn scroll_tick(self) {
        if self.dragging.get_untracked().is_none() {
            return;
        }
        let Some(w) = web_sys::window() else { return };
        let height = w
            .inner_height()
            .ok()
            .and_then(|h| h.as_f64())
            .unwrap_or_default();
        let step = edge_scroll_step(self.gesture.get_value().last.1, height);
        if step != 0.0 {
            w.scroll_by_with_x_and_y(0.0, step);
            self.update_drag();
        }
    }

    fn clear_timers(self) {
        let g = self.gesture.get_value();
        if let Some(w) = web_sys::window() {
            if let Some(id) = g.long_press_timer {
                w.clear_timeout_with_handle(id);
            }
            if let Some(id) = g.scroll_interval {
                w.clear_interval_with_handle(id);
            }
        }
    }

    /// Drop all gesture state without reordering.
    fn reset(self) {
        self.clear_timers();
        self.gesture.set_value(Gesture::default());
        DRAG_ACTIVE.with(|c| c.set(false));
        self.pending.set(None);
        self.dragging.set(None);
        self.drag_over.set(None);
        self.offset.set((0.0, 0.0));
    }
}

/// Reorder a Vec by moving item at `from` to appear before item at `to`;
/// `to == items.len()` moves it to the end.
pub fn apply_reorder<T: Clone>(mut items: Vec<T>, from: usize, to: usize) -> Vec<T> {
    if from >= items.len() || to > items.len() || from == to {
        return items;
    }
    let item = items.remove(from);
    let insert_at = if to > from { to - 1 } else { to };
    items.insert(insert_at, item);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_reorder_moves_item_to_the_end_when_to_equals_len() {
        assert_eq!(
            apply_reorder(vec!['a', 'b', 'c'], 0, 3),
            vec!['b', 'c', 'a']
        );
    }

    #[test]
    fn apply_reorder_inserts_before_the_target() {
        assert_eq!(
            apply_reorder(vec!['a', 'b', 'c'], 0, 2),
            vec!['b', 'a', 'c']
        );
        assert_eq!(
            apply_reorder(vec!['a', 'b', 'c'], 2, 0),
            vec!['c', 'a', 'b']
        );
    }

    #[test]
    fn apply_reorder_ignores_out_of_range_and_noop_moves() {
        assert_eq!(apply_reorder(vec!['a', 'b'], 0, 3), vec!['a', 'b']);
        assert_eq!(apply_reorder(vec!['a', 'b'], 2, 0), vec!['a', 'b']);
        assert_eq!(apply_reorder(vec!['a', 'b'], 1, 1), vec!['a', 'b']);
    }

    #[test]
    fn movement_below_the_threshold_is_a_tap() {
        assert!(!exceeds(3.0, 4.0, DRAG_THRESHOLD_PX));
        assert!(exceeds(6.0, 6.0, DRAG_THRESHOLD_PX));
    }

    #[test]
    fn edge_scroll_is_zero_in_the_middle_and_signed_at_the_edges() {
        assert_eq!(edge_scroll_step(300.0, 800.0), 0.0);
        assert!(edge_scroll_step(10.0, 800.0) < 0.0);
        assert!(edge_scroll_step(790.0, 800.0) > 0.0);
        assert_eq!(edge_scroll_step(0.0, 800.0), -MAX_SCROLL_STEP_PX);
    }
}

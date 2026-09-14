use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::PointerEvent;

// elements_from_point returns js_sys::Array; each item is JsValue -> cast to Element

/// Drag-and-drop sort state. Create one instance per sortable list with `use_sortable()`.
/// Attach the returned handler methods to your drag-handle elements and list item wrappers.
#[derive(Clone, Copy)]
pub struct SortableState {
    dragging_idx: RwSignal<Option<usize>>,
    drag_over_idx: RwSignal<Option<usize>>,
    scope: &'static str,
}

/// Creates a new `SortableState` for a sortable list.
///
/// `scope` is matched against `data-sort-scope` on item wrappers so that
/// nested lists (e.g. groups containing products) don't interfere with each
/// other during hit-testing.
pub fn use_sortable(scope: &'static str) -> SortableState {
    SortableState {
        dragging_idx: RwSignal::new(None),
        drag_over_idx: RwSignal::new(None),
        scope,
    }
}

impl SortableState {
    #[allow(dead_code)]
    pub fn dragging_index(self) -> ReadSignal<Option<usize>> {
        self.dragging_idx.read_only()
    }

    #[allow(dead_code)]
    pub fn drag_over_index(self) -> ReadSignal<Option<usize>> {
        self.drag_over_idx.read_only()
    }

    /// Returns a reactive class string for the item wrapper at `index`.
    /// `base` is the always-applied classes.
    pub fn item_classes(self, index: usize, base: &'static str) -> impl Fn() -> String {
        move || {
            let dragging = self.dragging_idx.get();
            let over = self.drag_over_idx.get();
            let is_dragging = dragging == Some(index);
            let is_target = over == Some(index) && !is_dragging;
            let mut s = base.to_string();
            if is_dragging {
                s.push_str(" opacity-40 pointer-events-none");
            }
            if is_target {
                s.push_str(" border-t-2 border-blue-400");
            }
            s
        }
    }

    /// Attach to `on:pointerdown` of the ⠿ handle element for item at `index`.
    /// Captures the pointer so all subsequent events go to this element.
    /// `index` is a closure so the current position is read at event time, not at render time.
    pub fn on_handle_pointerdown(
        self,
        index: impl Fn() -> usize + 'static,
    ) -> impl Fn(PointerEvent) {
        move |e: PointerEvent| {
            e.prevent_default();
            if let Some(el) = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            {
                let _ = el.set_pointer_capture(e.pointer_id());
            }
            let idx = index();
            self.dragging_idx.set(Some(idx));
            self.drag_over_idx.set(Some(idx));
        }
    }

    /// Attach to `on:pointermove` of the same ⠿ handle element.
    /// Hit-tests via `elements_from_point` against `data-sort-index` attributes.
    pub fn on_handle_pointermove(self) -> impl Fn(PointerEvent) {
        move |e: PointerEvent| {
            if self.dragging_idx.get_untracked().is_none() {
                return;
            }
            let doc = match web_sys::window().and_then(|w| w.document()) {
                Some(d) => d,
                None => return,
            };
            let elements = doc.elements_from_point(e.client_x() as f32, e.client_y() as f32);
            for i in 0..elements.length() {
                if let Ok(el) = elements.get(i).dyn_into::<web_sys::Element>() {
                    if el.get_attribute("data-sort-scope").as_deref() == Some(self.scope) {
                        if let Some(idx_str) = el.get_attribute("data-sort-index") {
                            if let Ok(idx) = idx_str.parse::<usize>() {
                                self.drag_over_idx.set(Some(idx));
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Attach to `on:pointerup` of the ⠿ handle element.
    /// Calls `on_reorder(from, to)` when the drag ends at a different position.
    /// `from` and `to` are indices into the displayed list.
    pub fn on_handle_pointerup(
        self,
        on_reorder: impl Fn(usize, usize) + Clone + 'static,
    ) -> impl Fn(PointerEvent) {
        move |_| {
            let from = match self.dragging_idx.get_untracked() {
                Some(i) => i,
                None => return,
            };
            let to = self.drag_over_idx.get_untracked().unwrap_or(from);
            self.dragging_idx.set(None);
            self.drag_over_idx.set(None);
            if from != to {
                on_reorder(from, to);
            }
        }
    }

    /// Attach to `on:pointercancel` to reset state without reordering.
    pub fn on_handle_pointercancel(self) -> impl Fn(PointerEvent) {
        move |_| {
            self.dragging_idx.set(None);
            self.drag_over_idx.set(None);
        }
    }
}

/// Reorder a Vec by moving item at `from` to appear before item at `to`.
/// Returns the reordered Vec with updated `sort_order` fields (index × 10).
pub fn apply_reorder<T: Clone>(mut items: Vec<T>, from: usize, to: usize) -> Vec<T> {
    if from >= items.len() || to >= items.len() || from == to {
        return items;
    }
    let item = items.remove(from);
    let insert_at = if to > from { to - 1 } else { to };
    items.insert(insert_at, item);
    items
}

use leptos::ev;
use leptos::html;
use leptos::leptos_dom::helpers::{window_event_listener_untyped, WindowListenerHandle};
use leptos::portal::Portal;
use leptos::prelude::*;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};

const CLOSE_ALL_EVENT: &str = "dropdown-menu-close-all";
const OPEN_EVENT: &str = "dropdown-menu-open";
const OPEN_MENU_ID_KEY: &str = "__ez_vend_open_dropdown_menu_id";
const VIEWPORT_PADDING: f64 = 8.0;
const MIN_MENU_WIDTH: f64 = 192.0;
const MIN_MENU_HEIGHT: f64 = 160.0;

fn call_dom_rect(target: &JsValue) -> Option<JsValue> {
    js_sys::Reflect::get(target, &JsValue::from_str("getBoundingClientRect"))
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
        .and_then(|function| function.call0(target).ok())
}

fn read_rect_value(rect: &JsValue, key: &str, fallback: f64) -> f64 {
    js_sys::Reflect::get(rect, &JsValue::from_str(key))
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(fallback)
}

fn dispatch_window_event(name: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(event) = web_sys::Event::new(name) {
            let _ = window.dispatch_event(&event);
        }
    }
}

fn set_open_menu_id(window: &web_sys::Window, menu_id: &str) {
    let _ = js_sys::Reflect::set(
        window,
        &JsValue::from_str(OPEN_MENU_ID_KEY),
        &JsValue::from_str(menu_id),
    );
}

fn get_open_menu_id(window: &web_sys::Window) -> Option<String> {
    js_sys::Reflect::get(window, &JsValue::from_str(OPEN_MENU_ID_KEY))
        .ok()
        .and_then(|value| value.as_string())
}

fn clear_open_menu_id(window: &web_sys::Window, menu_id: &str) {
    if get_open_menu_id(window).as_deref() == Some(menu_id) {
        let _ = js_sys::Reflect::delete_property(window, &JsValue::from_str(OPEN_MENU_ID_KEY));
    }
}

#[component]
pub fn DropdownMenu(
    trigger: AnyView,
    children: ChildrenFn,
    #[prop(default = "right".to_string())] align: String,
    #[prop(default = true)] close_on_item_click: bool,
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] menu_class: Option<String>,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    let (menu_style, set_menu_style) = signal(String::new());
    let menu_id = format!("dropdown-menu-{}", js_sys::Math::random());
    let menu_id_stored = StoredValue::new_local(menu_id.clone());
    let trigger_ref: NodeRef<html::Div> = NodeRef::new();
    let menu_ref: NodeRef<html::Div> = NodeRef::new();
    let container_class = class.unwrap_or_default();
    let menu_class = menu_class.unwrap_or_default();
    let align_right = align != "left";
    let menu_classes = StoredValue::new_local(format!(
        "fixed z-50 min-w-[12rem] overflow-y-auto rounded-lg border border-gray-200 bg-white py-1 shadow-xl {}",
        menu_class
    ));

    let update_menu_position: Rc<dyn Fn()> = Rc::new(move || {
        if let Some(trigger) = trigger_ref.get() {
            let Some(window) = web_sys::window() else {
                return;
            };

            let Some(rect) = call_dom_rect(trigger.as_ref()) else {
                return;
            };

            let viewport_width = window
                .inner_width()
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0);
            let viewport_height = window
                .inner_height()
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0);
            let left = read_rect_value(&rect, "left", VIEWPORT_PADDING);
            let top = read_rect_value(&rect, "top", VIEWPORT_PADDING);
            let right = read_rect_value(&rect, "right", left);
            let bottom = read_rect_value(&rect, "bottom", VIEWPORT_PADDING);

            let (menu_width, menu_height) = if let Some(menu) = menu_ref.get() {
                if let Some(menu_rect) = call_dom_rect(menu.as_ref()) {
                    let width = read_rect_value(&menu_rect, "width", MIN_MENU_WIDTH);
                    let height = read_rect_value(&menu_rect, "height", 0.0);
                    (width, height)
                } else {
                    (MIN_MENU_WIDTH, 0.0)
                }
            } else {
                (MIN_MENU_WIDTH, 0.0)
            };

            let desired_left = if align_right {
                right - menu_width
            } else {
                left
            };
            let max_left = (viewport_width - menu_width - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
            let clamped_left = desired_left.clamp(VIEWPORT_PADDING, max_left);

            // Prefer opening upward only when there is more usable space above the trigger.
            let space_below = (viewport_height - bottom - VIEWPORT_PADDING).max(0.0);
            let space_above = (top - VIEWPORT_PADDING).max(0.0);
            let should_open_upward = menu_height > 0.0 && space_above > space_below;

            let desired_top = if should_open_upward {
                top - menu_height - VIEWPORT_PADDING
            } else {
                bottom + VIEWPORT_PADDING
            };
            let max_top = (viewport_height - menu_height - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
            let clamped_top = if menu_height > 0.0 {
                desired_top.clamp(VIEWPORT_PADDING, max_top)
            } else if should_open_upward {
                top.max(VIEWPORT_PADDING)
            } else {
                desired_top.max(VIEWPORT_PADDING)
            };
            let max_height =
                (viewport_height - clamped_top - VIEWPORT_PADDING).max(MIN_MENU_HEIGHT);

            set_menu_style.set(format!(
                "left: {}px; top: {}px; max-width: {}px; max-height: {}px;",
                clamped_left,
                clamped_top,
                (viewport_width - (VIEWPORT_PADDING * 2.0)).max(MIN_MENU_WIDTH),
                max_height
            ));
        }
    });

    // Effect 1: listen for close-all and open events for the lifetime of the component.
    // Handles returned by window_event_listener_untyped are Send+Sync and remove themselves on drop.
    Effect::new({
        let menu_id = menu_id.clone();
        move |prev: Option<(WindowListenerHandle, WindowListenerHandle)>| {
            drop(prev);
            let h1 = window_event_listener_untyped(CLOSE_ALL_EVENT, move |_| {
                set_is_open.set(false);
            });
            let menu_id_c = menu_id.clone();
            let h2 = window_event_listener_untyped(OPEN_EVENT, move |_| {
                if let Some(window) = web_sys::window() {
                    if get_open_menu_id(&window).as_deref() != Some(menu_id_c.as_str()) {
                        set_is_open.set(false);
                    }
                }
            });
            (h1, h2)
        }
    });

    // Effect 2: while the menu is open, respond to keydown / resize / scroll.
    // ponytail: LocalStorage because update_menu_position is Rc<dyn Fn()> which is !Send
    Effect::<LocalStorage>::new(move |prev: Option<Vec<WindowListenerHandle>>| {
        drop(prev); // drops previous handles, removing old listeners

        if !is_open.get() {
            return vec![];
        }

        update_menu_position();
        {
            let update_menu_position = Rc::clone(&update_menu_position);
            // Measure once more after mount so fixed positioning can use the real menu size.
            set_timeout(
                move || update_menu_position(),
                std::time::Duration::from_millis(0),
            );
        }

        let h_key = window_event_listener_untyped("keydown", move |event| {
            let event: web_sys::KeyboardEvent = event.unchecked_into();
            if event.key() == "Escape" {
                set_is_open.set(false);
            }
        });

        let pos = Rc::clone(&update_menu_position);
        let h_resize = window_event_listener_untyped("resize", move |_| pos());

        let pos = Rc::clone(&update_menu_position);
        let h_scroll = window_event_listener_untyped("scroll", move |_| pos());

        vec![h_key, h_resize, h_scroll]
    });

    // ponytail: StoredValue is Copy, preventing FnOnce when children is captured in Show's ChildrenFn
    let children_stored = StoredValue::new_local(children);

    let toggle_menu = {
        let menu_id = menu_id.clone();
        move |event: ev::MouseEvent| {
            event.stop_propagation();
            let was_open = is_open.get_untracked();

            if let Some(window) = web_sys::window() {
                dispatch_window_event(CLOSE_ALL_EVENT);

                if !was_open {
                    set_open_menu_id(&window, &menu_id);
                    dispatch_window_event(OPEN_EVENT);
                }
            }

            if !was_open {
                set_is_open.set(true);
            }
        }
    };

    view! {
        <div node_ref=trigger_ref class=format!("relative {container_class}")>
            <div on:click=toggle_menu aria-expanded=move || is_open.get()>
                {trigger}
            </div>

            <Show when=move || is_open.get()>
                <Portal>
                    <>
                        <div
                            class="fixed inset-0 z-40 print:hidden"
                            on:click={
                                move |_| {
                                    if let Some(window) = web_sys::window() {
                                        clear_open_menu_id(&window, &menu_id_stored.get_value());
                                    }
                                    set_is_open.set(false);
                                }
                            }
                        ></div>

                        <div
                            node_ref=menu_ref
                            class=move || format!("{} print:hidden", menu_classes.get_value())
                            style=move || menu_style.get()
                            on:click={
                                move |event: ev::MouseEvent| {
                                    event.stop_propagation();
                                    if close_on_item_click {
                                        if let Some(window) = web_sys::window() {
                                            clear_open_menu_id(&window, &menu_id_stored.get_value());
                                        }
                                        set_is_open.set(false);
                                    }
                                }
                            }
                        >
                            {move || children_stored.with_value(|c| c())}
                        </div>
                    </>
                </Portal>
            </Show>
        </div>
    }
}

#[component]
pub fn DropdownMenuItem(
    on_click: Callback<ev::MouseEvent>,
    #[prop(optional)] icon: Option<AnyView>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let children = children();
    let additional_class = class.unwrap_or_default();

    view! {
        <button
            type="button"
            class=move || format!(
                "flex w-full items-center gap-3 px-4 py-2.5 text-left text-sm text-gray-700 transition-colors focus:outline-none {} {}",
                if disabled.get() {
                    "cursor-not-allowed opacity-50"
                } else {
                    "hover:bg-gray-50 focus:bg-gray-50"
                },
                additional_class
            )
            disabled=move || disabled.get()
            aria-disabled=move || if disabled.get() { Some("true") } else { None }
            on:click=move |event| {
                if !disabled.get() {
                    on_click.run(event)
                }
            }
        >
            {icon.map(|icon| view! { <span class="shrink-0 opacity-70">{icon}</span> })}
            <span class="flex-1">{children}</span>
        </button>
    }
}

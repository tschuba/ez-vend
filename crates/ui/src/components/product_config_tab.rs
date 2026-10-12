#![allow(clippy::clone_on_copy)]

use crate::components::{
    color_accent_class,
    icons::{Icon, LuChevronDown, LuChevronUp},
    use_toast, Input, Modal, NumberInput, PRODUCT_GRID_CLASS,
};
use crate::formatting::{format_currency, format_decimal_for_input, parse_decimal_input};
use crate::hooks::sortable::{apply_reorder, use_grouped_sortable, use_sortable};
use crate::i18n::{translate_with_params, use_locale};
use crate::state::use_app_state;
use crate::t;
use crate::utils::format_error_message;
use domain::models::shared::{BoothId, ProductGroupId, ProductId};
use domain::models::{Product, ProductGroup, TailwindColor};
use leptos::portal::Portal;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use wasm_bindgen::{JsCast, JsValue};

const VENDING_EMOJIS: &[&str] = &[
    "🥤", "☕", "🍺", "🍻", "🧃", "🍵", "🍕", "🍔", "🌭", "🍟", "🥪", "🍱", "🍣", "🍜", "🌮", "🍰",
    "🎂", "🍩", "🧁", "🍫", "🍬", "🍭", "🍦", "🎪", "🛍️",
];

#[component]
fn EmojiPicker(value: RwSignal<String>) -> impl IntoView {
    let open = RwSignal::new(false);
    let btn_ref: NodeRef<leptos::html::Button> = NodeRef::new();
    // (top, left) of the popup in viewport coordinates
    let pos: RwSignal<(f64, f64)> = RwSignal::new((0.0, 0.0));

    view! {
        <button
            node_ref=btn_ref
            type="button"
            class="px-3 py-1.5 min-h-[44px] rounded-full border border-gray-300 bg-white text-sm text-gray-700 hover:bg-gray-50 cursor-pointer whitespace-nowrap"
            on:click=move |e| {
                e.stop_propagation();
                if let Some(el) = btn_ref.get() {
                    let el_js: &JsValue = el.as_ref();
                    if let Some(rect) = js_sys::Reflect::get(el_js, &JsValue::from_str("getBoundingClientRect"))
                        .ok()
                        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
                        .and_then(|f| f.call0(el_js).ok())
                    {
                        let bottom = js_sys::Reflect::get(&rect, &JsValue::from_str("bottom")).ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let left = js_sys::Reflect::get(&rect, &JsValue::from_str("left")).ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                        pos.set((bottom, left));
                    }
                }
                open.update(|o| *o = !*o);
            }
        >
            {move || {
                let v = value.get();
                if v.is_empty() { "+ Emoji".to_string() } else { v }
            }}
        </button>
        <Show when=move || open.get()>
            <Portal>
                <div
                    class="fixed inset-0 z-[65]"
                    on:click=move |_| open.set(false)
                />
                <div
                    class="fixed z-[70] rounded-lg border border-gray-200 bg-white p-2 shadow-xl"
                    style=move || {
                        let (top, left) = pos.get();
                        format!("top: {}px; left: {}px;", top + 4.0, left)
                    }
                >
                    <div class="grid grid-cols-5 gap-1 mb-1">
                        {VENDING_EMOJIS.iter().map(|&e| {
                            view! {
                                <button
                                    type="button"
                                    class="w-9 h-9 text-xl rounded hover:bg-gray-100 flex items-center justify-center"
                                    on:click=move |_| { value.set(e.to_string()); open.set(false); }
                                >{e}</button>
                            }
                        }).collect_view()}
                    </div>
                    <button
                        type="button"
                        class="w-full text-xs text-gray-400 hover:text-gray-600 py-1 text-center"
                        on:click=move |_| { value.set(String::new()); open.set(false); }
                    >"✕ Kein Emoji"</button>
                </div>
            </Portal>
        </Show>
    }
}

const ALL_COLORS: [TailwindColor; 8] = [
    TailwindColor::Red,
    TailwindColor::Orange,
    TailwindColor::Amber,
    TailwindColor::Green,
    TailwindColor::Teal,
    TailwindColor::Blue,
    TailwindColor::Violet,
    TailwindColor::Pink,
];

fn color_bg(c: TailwindColor) -> &'static str {
    match c {
        TailwindColor::Red => "bg-red-500",
        TailwindColor::Orange => "bg-orange-500",
        TailwindColor::Amber => "bg-amber-500",
        TailwindColor::Green => "bg-green-500",
        TailwindColor::Teal => "bg-teal-500",
        TailwindColor::Blue => "bg-blue-500",
        TailwindColor::Violet => "bg-violet-500",
        TailwindColor::Pink => "bg-pink-500",
    }
}

/// Blank input → no stock limit; otherwise the entered non-negative integer.
/// `Err` means the (non-blank) input couldn't be parsed.
fn parse_stock_input(value: &str) -> Result<Option<u32>, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed.parse::<u32>().map(Some).map_err(|_| ())
    }
}

/// Move a product from `(from_group, from_idx)` to `(to_group, to_idx)` —
/// same-group reorder or a move to a different group — and renumber
/// `sort_order` (step 10) for every product in the affected group(s).
/// `from_idx`/`to_idx` are positions within each group's `sort_order`-sorted
/// list. Returns the full product list unchanged if `from_idx` is out of
/// range for the source group.
fn move_product(
    all: Vec<Product>,
    from_group: ProductGroupId,
    from_idx: usize,
    to_group: ProductGroupId,
    to_idx: usize,
) -> Vec<Product> {
    if from_group == to_group {
        let mut group_ps: Vec<Product> = all
            .iter()
            .filter(|p| p.product_group_id == from_group)
            .cloned()
            .collect();
        group_ps.sort_by_key(|p| p.sort_order);
        group_ps = apply_reorder(group_ps, from_idx, to_idx);
        for (i, p) in group_ps.iter_mut().enumerate() {
            p.sort_order = (i * 10) as u32;
        }
        let mut updated: Vec<Product> = all
            .into_iter()
            .filter(|p| p.product_group_id != from_group)
            .collect();
        updated.extend(group_ps);
        updated.sort_by_key(|p| p.sort_order);
        updated
    } else {
        let mut source: Vec<Product> = all
            .iter()
            .filter(|p| p.product_group_id == from_group)
            .cloned()
            .collect();
        source.sort_by_key(|p| p.sort_order);
        if from_idx >= source.len() {
            return all;
        }
        let mut moved = source.remove(from_idx);
        moved.product_group_id = to_group;
        for (i, p) in source.iter_mut().enumerate() {
            p.sort_order = (i * 10) as u32;
        }

        let mut target: Vec<Product> = all
            .iter()
            .filter(|p| p.product_group_id == to_group)
            .cloned()
            .collect();
        target.sort_by_key(|p| p.sort_order);
        let insert_at = to_idx.min(target.len());
        target.insert(insert_at, moved);
        for (i, p) in target.iter_mut().enumerate() {
            p.sort_order = (i * 10) as u32;
        }

        let mut updated: Vec<Product> = all
            .into_iter()
            .filter(|p| p.product_group_id != from_group && p.product_group_id != to_group)
            .collect();
        updated.extend(source);
        updated.extend(target);
        updated.sort_by_key(|p| p.sort_order);
        updated
    }
}

fn color_picker_button_class(c: TailwindColor, selected: TailwindColor) -> String {
    let bg = color_bg(c);
    let ring = if c == selected {
        " ring-2 ring-offset-1 ring-gray-700"
    } else {
        ""
    };
    // ponytail: w-9 (36px) — not 44px, but 8 circles × 44px = 352px minimum which breaks mobile layout
    format!("w-9 h-9 rounded-full {bg}{ring} cursor-pointer")
}

const TILE_BASE: &str = "relative border-l-4 bg-white border border-gray-200 rounded-lg px-3 py-2 min-h-[64px] text-sm text-gray-800 flex flex-col items-center justify-center text-center select-none w-full touch-manipulation cursor-grab [-webkit-touch-callout:none]";
const ADD_TILE_BASE: &str = "w-full min-h-[64px] rounded-lg border-2 border-dashed border-gray-300 text-2xl leading-none text-gray-400 hover:bg-gray-100 flex items-center justify-center select-none touch-manipulation";

#[component]
pub fn ProductConfigTab(
    booth_id: BoothId,
    /// Write total product count back to parent (for warning banner)
    total_products_out: RwSignal<usize>,
) -> impl IntoView {
    let app_state = use_app_state();
    let locale = use_locale();
    let toast = use_toast();

    let groups: RwSignal<Vec<ProductGroup>> = RwSignal::new(vec![]);
    let products: RwSignal<Vec<Product>> = RwSignal::new(vec![]);
    let locked_ids: RwSignal<HashSet<ProductId>> = RwSignal::new(HashSet::new());
    let data_version = RwSignal::new(0_u32);

    // Keep parent informed of total product count
    Effect::new(move |_| {
        total_products_out.set(products.get().len());
    });

    // Load groups + products + locked product ids
    Effect::new(move |_| {
        let _ = data_version.get();
        if let Some(Ok(state)) = app_state.get() {
            spawn_local(async move {
                if let Ok(mut gs) = state
                    .product_group_repository
                    .find_by_booth(&booth_id)
                    .await
                {
                    gs.sort_by_key(|g| g.sort_order);
                    groups.set(gs);
                }
                if let Ok(mut ps) = state.product_repository.find_by_booth(&booth_id).await {
                    ps.sort_by_key(|p| p.sort_order);
                    let count = ps.len();
                    products.set(ps);
                    total_products_out.set(count);
                }
                if let Ok(purchases) = state.purchase_repository.find_by_booth(&booth_id).await {
                    let locked: HashSet<ProductId> = purchases
                        .iter()
                        .flat_map(|p| p.items.iter())
                        .filter_map(|item| item.product_id)
                        .collect();
                    locked_ids.set(locked);
                }
            });
        }
    });

    // ── Group modal state (editing_group None while open = "new group") ───────
    let group_modal_open = RwSignal::new(false);
    let editing_group: RwSignal<Option<ProductGroupId>> = RwSignal::new(None);
    let group_name = RwSignal::new(String::new());
    let group_emoji = RwSignal::new(String::new());
    let group_color: RwSignal<TailwindColor> = RwSignal::new(TailwindColor::Blue);
    let group_delete_armed = RwSignal::new(false);

    // ── Product modal state (editing_product None while open = "new product
    // in adding_product_to") ─────────────────────────────────────────────────
    let product_modal_open = RwSignal::new(false);
    let editing_product: RwSignal<Option<ProductId>> = RwSignal::new(None);
    let adding_product_to: RwSignal<Option<ProductGroupId>> = RwSignal::new(None);
    let product_name = RwSignal::new(String::new());
    let product_price = RwSignal::new(String::new());
    let product_stock = RwSignal::new(String::new());
    let product_delete_armed = RwSignal::new(false);

    // ── Group reordering: drag (whole header) + up/down buttons ──────────────
    let group_dnd = use_sortable("group");

    let save_group_order = move |from: usize, to: usize| {
        let mut gs = groups.get_untracked();
        gs = apply_reorder(gs, from, to);
        for (i, g) in gs.iter_mut().enumerate() {
            g.sort_order = (i * 10) as u32;
        }
        groups.set(gs.clone());
        if let Some(Ok(state)) = app_state.get_untracked() {
            spawn_local(async move {
                for g in &gs {
                    let _ = state.product_group_repository.save(g).await;
                }
            });
        }
    };

    // `apply_reorder` inserts *before* `to`, so one step down is `from + 2`.
    let move_group_up = move |group_id: ProductGroupId| {
        if let Some(i) = groups.get_untracked().iter().position(|g| g.id == group_id) {
            if i > 0 {
                save_group_order(i, i - 1);
            }
        }
    };
    let move_group_down = move |group_id: ProductGroupId| {
        if let Some(i) = groups.get_untracked().iter().position(|g| g.id == group_id) {
            save_group_order(i, i + 2);
        }
    };

    // ── Product drag (shared across all groups so a drag can cross group
    // boundaries; group ids travel as strings through data attributes) ───────
    let product_dnd = use_grouped_sortable("product");

    let move_product_between_groups =
        move |from_group: String, from_idx: usize, to_group: String, to_idx: usize| {
            let (Ok(from_group), Ok(to_group)) =
                (Uuid::parse_str(&from_group), Uuid::parse_str(&to_group))
            else {
                return;
            };
            let from_group = ProductGroupId::from_uuid(from_group);
            let to_group = ProductGroupId::from_uuid(to_group);

            let previous = products.get_untracked();
            let updated = move_product(previous.clone(), from_group, from_idx, to_group, to_idx);
            products.set(updated.clone());
            let to_save: Vec<Product> = updated
                .into_iter()
                .filter(|p| p.product_group_id == from_group || p.product_group_id == to_group)
                .collect();
            if let Some(Ok(state)) = app_state.get_untracked() {
                spawn_local(async move {
                    if let Err(e) = state.product_repository.save_many(&to_save).await {
                        products.set(previous);
                        toast.error(translate_with_params(
                            "product.move_failed",
                            HashMap::from([("error", format_error_message(&e))]),
                        ));
                    }
                });
            }
        };

    // ── Group modal actions ───────────────────────────────────────────────────
    let close_group_modal = move || {
        group_modal_open.set(false);
        group_delete_armed.set(false);
    };

    let open_new_group = move || {
        group_name.set(String::new());
        group_emoji.set(String::new());
        group_color.set(TailwindColor::Blue);
        group_delete_armed.set(false);
        editing_group.set(None);
        group_modal_open.set(true);
    };

    let open_edit_group = move |group_id: ProductGroupId| {
        if let Some(g) = groups
            .get_untracked()
            .into_iter()
            .find(|g| g.id == group_id)
        {
            group_name.set(g.name.clone());
            group_emoji.set(g.emoji.clone().unwrap_or_default());
            group_color.set(g.color);
            group_delete_armed.set(false);
            editing_group.set(Some(group_id));
            group_modal_open.set(true);
        }
    };

    let save_group = move || {
        let name = group_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let emoji = {
            let e = group_emoji.get_untracked().trim().to_string();
            if e.is_empty() {
                None
            } else {
                Some(e)
            }
        };
        let color = group_color.get_untracked();

        let group = match editing_group.get_untracked() {
            Some(group_id) => {
                let Some(mut group) = groups
                    .get_untracked()
                    .into_iter()
                    .find(|g| g.id == group_id)
                else {
                    return;
                };
                group.name = name;
                group.emoji = emoji;
                group.color = color;
                groups.update(|gs| {
                    if let Some(g) = gs.iter_mut().find(|g| g.id == group_id) {
                        *g = group.clone();
                    }
                });
                group
            }
            None => {
                let group = ProductGroup {
                    id: ProductGroupId::new(),
                    booth_id,
                    name,
                    color,
                    emoji,
                    sort_order: (groups.get_untracked().len() * 10) as u32,
                };
                groups.update(|gs| gs.push(group.clone()));
                group
            }
        };
        close_group_modal();
        if let Some(Ok(state)) = app_state.get_untracked() {
            spawn_local(async move {
                let _ = state.product_group_repository.save(&group).await;
            });
        }
    };

    let delete_group = move |group_id: ProductGroupId| {
        let has_products = products
            .get_untracked()
            .iter()
            .any(|p| p.product_group_id == group_id);
        if has_products {
            return;
        }
        close_group_modal();
        groups.update(|gs| gs.retain(|g| g.id != group_id));
        if let Some(Ok(state)) = app_state.get_untracked() {
            spawn_local(async move {
                let _ = state
                    .product_group_repository
                    .delete(&booth_id, &group_id)
                    .await;
            });
        }
    };

    // ── Product modal actions ─────────────────────────────────────────────────
    let close_product_modal = move || {
        product_modal_open.set(false);
        product_delete_armed.set(false);
    };

    let open_new_product = move |group_id: ProductGroupId| {
        product_name.set(String::new());
        product_price.set(String::new());
        product_stock.set(String::new());
        product_delete_armed.set(false);
        editing_product.set(None);
        adding_product_to.set(Some(group_id));
        product_modal_open.set(true);
    };

    let open_edit_product = move |product_id: ProductId| {
        if let Some(p) = products
            .get_untracked()
            .into_iter()
            .find(|p| p.id == product_id)
        {
            product_name.set(p.name.clone());
            product_price.set(format_decimal_for_input(p.price, locale.get_untracked(), 2));
            product_stock.set(p.initial_stock.map(|s| s.to_string()).unwrap_or_default());
            product_delete_armed.set(false);
            adding_product_to.set(None);
            editing_product.set(Some(product_id));
            product_modal_open.set(true);
        }
    };

    let save_product = move || {
        let name = product_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let Ok(price) = parse_decimal_input(&product_price.get_untracked()) else {
            return;
        };
        let Ok(initial_stock) = parse_stock_input(&product_stock.get_untracked()) else {
            return;
        };

        let product = if let Some(product_id) = editing_product.get_untracked() {
            products.update(|ps| {
                if let Some(p) = ps.iter_mut().find(|p| p.id == product_id) {
                    p.name = name;
                    p.price = price;
                    p.initial_stock = initial_stock;
                }
            });
            products
                .get_untracked()
                .into_iter()
                .find(|p| p.id == product_id)
        } else if let Some(group_id) = adding_product_to.get_untracked() {
            let next_order = products
                .get_untracked()
                .iter()
                .filter(|p| p.product_group_id == group_id)
                .count()
                * 10;
            let product = Product {
                id: ProductId::new(),
                booth_id,
                product_group_id: group_id,
                name,
                price,
                sort_order: next_order as u32,
                initial_stock,
            };
            products.update(|ps| ps.push(product.clone()));
            Some(product)
        } else {
            None
        };
        close_product_modal();
        if let (Some(product), Some(Ok(state))) = (product, app_state.get_untracked()) {
            spawn_local(async move {
                let _ = state.product_repository.save(&product).await;
            });
        }
    };

    let delete_product = move |product_id: ProductId| {
        if locked_ids.get_untracked().contains(&product_id) {
            return;
        }
        close_product_modal();
        products.update(|ps| ps.retain(|p| p.id != product_id));
        if let Some(Ok(state)) = app_state.get_untracked() {
            spawn_local(async move {
                let _ = state
                    .product_repository
                    .delete(&booth_id, &product_id)
                    .await;
            });
        }
    };

    let group_modal_title = Signal::derive(move || {
        if editing_group.get().is_some() {
            t!("product.edit_group")()
        } else {
            t!("product.add_group")()
        }
    });
    let product_modal_title = Signal::derive(move || {
        if editing_product.get().is_some() {
            t!("product.edit_product")()
        } else {
            t!("product.add_product")()
        }
    });

    // Delete lives only in the edit modals. A blocked delete shows its reason
    // as visible text (a `title` tooltip never appears on touch devices).
    let group_delete_blocked = move || {
        editing_group
            .get()
            .is_some_and(|gid| products.get().iter().any(|p| p.product_group_id == gid))
    };
    let product_delete_blocked = move || {
        editing_product
            .get()
            .is_some_and(|pid| locked_ids.get().contains(&pid))
    };

    view! {
        <div class="space-y-4">
            // Empty state
            {move || {
                if products.get().is_empty() && groups.get().is_empty() {
                    view! {
                        <p class="text-center text-sm text-gray-500 py-4">
                            {t!("product.no_products_hint")()}
                        </p>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Groups
            <For
                each=move || groups.get()
                key=|g| g.id
                children=move |group| {
                    let group_id = group.id;
                    let group_id_str = group_id.to_string();

                    let g_idx = Signal::derive(move || {
                        groups.get().iter().position(|g| g.id == group_id).unwrap_or(0)
                    });
                    let is_first = Signal::derive(move || g_idx.get() == 0);
                    let is_last = Signal::derive(move || g_idx.get() + 1 >= groups.get().len());

                    // Read fresh values from the signal so display updates after save
                    let group_name_display = move || {
                        groups.get().iter().find(|g| g.id == group_id)
                            .map(|g| g.name.clone())
                            .unwrap_or_default()
                    };
                    let group_emoji_display = move || {
                        groups.get().iter().find(|g| g.id == group_id).and_then(|g| g.emoji.clone())
                    };
                    let group_color_value = move || {
                        groups.get().iter().find(|g| g.id == group_id)
                            .map(|g| g.color)
                            .unwrap_or(group.color)
                    };
                    let product_count = move || {
                        products.get().iter().filter(|p| p.product_group_id == group_id).count()
                    };

                    view! {
                        <div
                            data-sort-index=move || g_idx.get().to_string()
                            data-sort-scope="group"
                            class=move || group_dnd.item_classes(
                                String::new(),
                                g_idx.get(),
                                "rounded-lg border border-gray-200 bg-gray-50",
                            )()
                            style=move || group_dnd.item_style(String::new(), g_idx.get())()
                        >
                            // Group header: the whole row is the drag surface;
                            // a tap (no drag) opens the edit modal.
                            <div
                                class="flex items-center cursor-grab select-none touch-manipulation [-webkit-touch-callout:none]"
                                on:pointerdown=group_dnd.on_pointerdown(
                                    String::new,
                                    move || g_idx.get_untracked(),
                                )
                                on:pointermove=group_dnd.on_pointermove()
                                on:pointerup=group_dnd.on_pointerup(
                                    move |_, from, _, to| save_group_order(from, to),
                                )
                                on:pointercancel=group_dnd.on_pointercancel()
                                on:contextmenu=|e| e.prevent_default()
                                on:click=move |_| {
                                    if group_dnd.take_click_suppressed() {
                                        return;
                                    }
                                    open_edit_group(group_id);
                                }
                            >
                                // Visual drag hint only; the whole row is the handle
                                <span class="text-gray-400 text-lg leading-none px-3 py-3 pointer-events-none">"⠿"</span>
                                <span class=move || format!("w-3 h-3 rounded-full flex-shrink-0 {}", color_bg(group_color_value())) />
                                <span class="flex-1 ml-2 flex items-center gap-1">
                                    <span class="text-sm font-medium text-gray-900">{group_name_display}</span>
                                    {move || group_emoji_display().map(|e| view! { <span>{e}</span> })}
                                </span>
                                <button
                                    type="button"
                                    class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-500 hover:text-gray-900 disabled:opacity-30 disabled:cursor-not-allowed"
                                    aria-label=t!("product.move_group_up")()
                                    disabled=move || is_first.get()
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        move_group_up(group_id);
                                    }
                                ><Icon icon=LuChevronUp class="w-5 h-5" /></button>
                                <button
                                    type="button"
                                    class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-500 hover:text-gray-900 disabled:opacity-30 disabled:cursor-not-allowed"
                                    aria-label=t!("product.move_group_down")()
                                    disabled=move || is_last.get()
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        move_group_down(group_id);
                                    }
                                ><Icon icon=LuChevronDown class="w-5 h-5" /></button>
                            </div>

                            // Product tiles. Collapsed while a group is being
                            // dragged so the ghost is a slim row and every
                            // group fits on screen as a drop target.
                            <div class=move || if group_dnd.is_dragging()() {
                                "hidden"
                            } else {
                                "border-t border-gray-200 p-3"
                            }>
                                <div class=PRODUCT_GRID_CLASS>
                                    <For
                                        each=move || {
                                            let mut ps: Vec<Product> = products.get()
                                                .into_iter()
                                                .filter(|p| p.product_group_id == group_id)
                                                .collect();
                                            ps.sort_by_key(|p| p.sort_order);
                                            ps
                                        }
                                        key=|p| p.id
                                        children={
                                            let group_id_str = group_id_str.clone();
                                            move |product| {
                                                let product_id = product.id;
                                                let group_for_attr = group_id_str.clone();
                                                let group_for_class = group_id_str.clone();
                                                let group_for_style = group_id_str.clone();
                                                let group_for_down = group_id_str.clone();
                                                // Position is read reactively: keyed children
                                                // aren't re-created when a reorder shifts them.
                                                let local_idx = Signal::derive(move || {
                                                    let mut ps: Vec<(u32, ProductId)> = products.get()
                                                        .iter()
                                                        .filter(|p| p.product_group_id == group_id)
                                                        .map(|p| (p.sort_order, p.id))
                                                        .collect();
                                                    ps.sort_by_key(|(order, _)| *order);
                                                    ps.iter().position(|(_, id)| *id == product_id).unwrap_or(0)
                                                });
                                                let tile_classes = move || {
                                                    format!(
                                                        "{} {}",
                                                        color_accent_class(group_color_value()),
                                                        product_dnd.item_classes(
                                                            group_for_class.clone(),
                                                            local_idx.get(),
                                                            TILE_BASE,
                                                        )()
                                                    )
                                                };
                                                let current = move || {
                                                    products.get().into_iter().find(|p| p.id == product_id)
                                                };

                                                view! {
                                                    <div
                                                        role="button"
                                                        tabindex="0"
                                                        data-sort-index=move || local_idx.get().to_string()
                                                        data-sort-group=group_for_attr
                                                        data-sort-scope="product"
                                                        class=tile_classes
                                                        style=move || product_dnd.item_style(group_for_style.clone(), local_idx.get())()
                                                        on:pointerdown=product_dnd.on_pointerdown(
                                                            move || group_for_down.clone(),
                                                            move || local_idx.get_untracked(),
                                                        )
                                                        on:pointermove=product_dnd.on_pointermove()
                                                        on:pointerup=product_dnd.on_pointerup(move_product_between_groups)
                                                        on:pointercancel=product_dnd.on_pointercancel()
                                                        on:contextmenu=|e| e.prevent_default()
                                                        on:click=move |_| {
                                                            if product_dnd.take_click_suppressed() {
                                                                return;
                                                            }
                                                            open_edit_product(product_id);
                                                        }
                                                        on:keydown=move |e| {
                                                            if e.key() == "Enter" || e.key() == " " {
                                                                e.prevent_default();
                                                                open_edit_product(product_id);
                                                            }
                                                        }
                                                    >
                                                        // Visual drag hint only (no handlers)
                                                        <span class="absolute top-1 right-1 text-xs leading-none text-gray-300 pointer-events-none">"⠿"</span>
                                                        <span class="font-semibold leading-tight">
                                                            {move || current().map(|p| p.name).unwrap_or_default()}
                                                        </span>
                                                        <span class="text-xs opacity-70 leading-tight tabular-nums">
                                                            {move || current()
                                                                .map(|p| format_currency(p.price, locale.get()))
                                                                .unwrap_or_default()}
                                                        </span>
                                                        {move || current().and_then(|p| p.initial_stock).map(|s| view! {
                                                            <span class="text-[10px] text-gray-500 tabular-nums leading-tight">
                                                                {format!("{}: {}", t!("product.product_stock_label")(), s)}
                                                            </span>
                                                        })}
                                                    </div>
                                                }
                                            }
                                        }
                                    />

                                    // "+" tile: tap creates a product; as a drop target it
                                    // means "append to this group" (never a drag source).
                                    <button
                                        type="button"
                                        aria-label=t!("product.add_product")()
                                        data-sort-index=move || product_count().to_string()
                                        data-sort-group=group_id_str.clone()
                                        data-sort-scope="product"
                                        class={
                                            let g = group_id_str.clone();
                                            move || product_dnd.item_classes(g.clone(), product_count(), ADD_TILE_BASE)()
                                        }
                                        on:click=move |_| open_new_product(group_id)
                                    >"+"</button>
                                </div>
                            </div>
                        </div>
                    }
                }
            />

            // Add new group
            <button
                type="button"
                class="flex min-h-[44px] items-center text-sm font-medium text-blue-600 hover:text-blue-800"
                on:click=move |_| open_new_group()
            >"+ "{t!("product.add_group")()}</button>

            // ── Group modal ────────────────────────────────────────────────────
            <Modal
                show=group_modal_open
                on_close=close_group_modal
                title=group_modal_title
                close_on_overlay_click=false
                action_bar=view! {
                    <div class="contents">
                        {move || editing_group.get().map(|group_id| {
                            if group_delete_blocked() {
                                view! {
                                    <button
                                        type="button"
                                        disabled=true
                                        class="mr-auto px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-gray-400 cursor-not-allowed"
                                    >{t!("common.delete")()}</button>
                                }.into_any()
                            } else if group_delete_armed.get() {
                                view! {
                                    <button
                                        type="button"
                                        class="mr-auto px-3 py-2 min-h-[44px] rounded-md bg-red-600 text-sm font-medium text-white hover:bg-red-700"
                                        on:click=move |_| delete_group(group_id)
                                    >{t!("product.group_delete_confirm")()}</button>
                                }.into_any()
                            } else {
                                view! {
                                    <button
                                        type="button"
                                        class="mr-auto px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-red-600 hover:bg-red-50"
                                        on:click=move |_| group_delete_armed.set(true)
                                    >{t!("common.delete")()}</button>
                                }.into_any()
                            }
                        })}
                        <button
                            type="button"
                            class="px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-gray-600 hover:bg-gray-200"
                            on:click=move |_| close_group_modal()
                        >{t!("common.cancel")()}</button>
                        <button
                            type="button"
                            class="px-3 py-2 min-h-[44px] rounded-md bg-blue-600 text-sm font-medium text-white hover:bg-blue-700"
                            on:click=move |_| save_group()
                        >{t!("common.save")()}</button>
                    </div>
                }.into_any()
            >
                <div class="space-y-3">
                    <Input
                        value=group_name
                        label=t!("product.group_name_label")()
                        placeholder=t!("product.group_name_placeholder")()
                        aria_label=t!("product.group_name_label")()
                    />
                    <div class="flex flex-wrap items-center gap-2">
                        <EmojiPicker value=group_emoji />
                        <div class="flex gap-1 flex-wrap">
                            {ALL_COLORS.iter().map(|&c| {
                                let cls = move || color_picker_button_class(c, group_color.get());
                                view! {
                                    <button
                                        type="button"
                                        class=cls
                                        aria-label=t!("product.group_color_label")()
                                        on:click=move |_| group_color.set(c)
                                    />
                                }
                            }).collect_view()}
                        </div>
                    </div>
                    <Show when=group_delete_blocked>
                        <p class="text-sm text-gray-500">{t!("product.group_has_products")()}</p>
                    </Show>
                </div>
            </Modal>

            // ── Product modal ──────────────────────────────────────────────────
            <Modal
                show=product_modal_open
                on_close=close_product_modal
                title=product_modal_title
                close_on_overlay_click=false
                action_bar=view! {
                    <div class="contents">
                        {move || editing_product.get().map(|product_id| {
                            if product_delete_blocked() {
                                view! {
                                    <button
                                        type="button"
                                        disabled=true
                                        class="mr-auto px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-gray-400 cursor-not-allowed"
                                    >{t!("common.delete")()}</button>
                                }.into_any()
                            } else if product_delete_armed.get() {
                                view! {
                                    <button
                                        type="button"
                                        class="mr-auto px-3 py-2 min-h-[44px] rounded-md bg-red-600 text-sm font-medium text-white hover:bg-red-700"
                                        on:click=move |_| delete_product(product_id)
                                    >{t!("product.product_delete_confirm")()}</button>
                                }.into_any()
                            } else {
                                view! {
                                    <button
                                        type="button"
                                        class="mr-auto px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-red-600 hover:bg-red-50"
                                        on:click=move |_| product_delete_armed.set(true)
                                    >{t!("common.delete")()}</button>
                                }.into_any()
                            }
                        })}
                        <button
                            type="button"
                            class="px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-gray-600 hover:bg-gray-200"
                            on:click=move |_| close_product_modal()
                        >{t!("common.cancel")()}</button>
                        <button
                            type="button"
                            class="px-3 py-2 min-h-[44px] rounded-md bg-blue-600 text-sm font-medium text-white hover:bg-blue-700"
                            on:click=move |_| save_product()
                        >{t!("common.save")()}</button>
                    </div>
                }.into_any()
            >
                <div class="space-y-3">
                    <Input
                        value=product_name
                        label=t!("product.product_name_label")()
                        placeholder=t!("product.product_name_placeholder")()
                        aria_label=t!("product.product_name_label")()
                    />
                    <div class="grid grid-cols-2 gap-3">
                        <NumberInput
                            value=product_price
                            label=t!("product.product_price_label")()
                            placeholder=t!("common.placeholders.decimal_zero")()
                        />
                        <NumberInput
                            value=product_stock
                            label=t!("product.product_stock_label")()
                            placeholder=t!("product.product_stock_placeholder")()
                        />
                    </div>
                    <Show when=product_delete_blocked>
                        <p class="text-sm text-gray-500">{t!("product.locked_hint")()}</p>
                    </Show>
                </div>
            </Modal>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn product(group: ProductGroupId, sort_order: u32) -> Product {
        Product {
            id: ProductId::new(),
            booth_id: BoothId::new(),
            product_group_id: group,
            name: "Test".to_string(),
            price: Decimal::ONE,
            sort_order,
            initial_stock: None,
        }
    }

    #[test]
    fn reorders_within_the_same_group() {
        let group = ProductGroupId::new();
        let a = product(group, 0);
        let b = product(group, 10);
        let c = product(group, 20);
        let all = vec![a.clone(), b.clone(), c.clone()];

        // Drag "a" (index 0) to land after "b" (index 1 pre-removal, i.e. before "c").
        let result = move_product(all, group, 0, group, 2);

        let ordered: Vec<_> = result.into_iter().map(|p| p.id).collect();
        assert_eq!(ordered, vec![b.id, a.id, c.id]);
    }

    #[test]
    fn moves_a_product_into_another_group_at_the_drop_position() {
        let source_group = ProductGroupId::new();
        let target_group = ProductGroupId::new();
        let moved = product(source_group, 0);
        let stays_in_source = product(source_group, 10);
        let x = product(target_group, 0);
        let y = product(target_group, 10);
        let all = vec![moved.clone(), stays_in_source.clone(), x.clone(), y.clone()];

        // Drop "moved" between x (index 0) and y (index 1) in the target group.
        let result = move_product(all, source_group, 0, target_group, 1);

        let moved_product = result.iter().find(|p| p.id == moved.id).unwrap();
        assert_eq!(moved_product.product_group_id, target_group);

        let mut target_ordered: Vec<_> = result
            .iter()
            .filter(|p| p.product_group_id == target_group)
            .collect();
        target_ordered.sort_by_key(|p| p.sort_order);
        let target_ids: Vec<_> = target_ordered.iter().map(|p| p.id).collect();
        assert_eq!(target_ids, vec![x.id, moved.id, y.id]);

        let source_ordered: Vec<_> = result
            .iter()
            .filter(|p| p.product_group_id == source_group)
            .collect();
        assert_eq!(source_ordered.len(), 1);
        assert_eq!(source_ordered[0].id, stays_in_source.id);
        assert_eq!(source_ordered[0].sort_order, 0);
    }

    #[test]
    fn moving_into_an_empty_group_lands_at_index_zero() {
        let source_group = ProductGroupId::new();
        let target_group = ProductGroupId::new();
        let moved = product(source_group, 0);
        let all = vec![moved.clone()];

        let result = move_product(all, source_group, 0, target_group, 0);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].product_group_id, target_group);
        assert_eq!(result[0].sort_order, 0);
    }

    #[test]
    fn dragging_onto_the_trailing_slot_appends_to_the_group() {
        let group = ProductGroupId::new();
        let a = product(group, 0);
        let b = product(group, 10);
        let c = product(group, 20);
        let all = vec![a.clone(), b.clone(), c.clone()];

        // The "+" tile sits at index == len and means "append".
        let result = move_product(all, group, 0, group, 3);

        let ordered: Vec<_> = result.into_iter().map(|p| p.id).collect();
        assert_eq!(ordered, vec![b.id, c.id, a.id]);
    }

    #[test]
    fn moving_across_groups_onto_the_trailing_slot_appends() {
        let source_group = ProductGroupId::new();
        let target_group = ProductGroupId::new();
        let moved = product(source_group, 0);
        let x = product(target_group, 0);
        let all = vec![moved.clone(), x.clone()];

        let result = move_product(all, source_group, 0, target_group, 1);

        let mut target: Vec<_> = result
            .iter()
            .filter(|p| p.product_group_id == target_group)
            .collect();
        target.sort_by_key(|p| p.sort_order);
        let ids: Vec<_> = target.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![x.id, moved.id]);
    }
}

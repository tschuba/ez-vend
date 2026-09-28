#![allow(clippy::clone_on_copy)]

use crate::components::{
    icons::{Icon, LuTrash2},
    use_toast, Input, NumberInput,
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

    // ── Group CRUD state ──────────────────────────────────────────────────────
    let editing_group: RwSignal<Option<ProductGroupId>> = RwSignal::new(None);
    let new_group_open = RwSignal::new(false);

    let edit_group_name = RwSignal::new(String::new());
    let edit_group_emoji = RwSignal::new(String::new());
    let edit_group_color: RwSignal<TailwindColor> = RwSignal::new(TailwindColor::Blue);

    let new_group_name = RwSignal::new(String::new());
    let new_group_emoji = RwSignal::new(String::new());
    let new_group_color: RwSignal<TailwindColor> = RwSignal::new(TailwindColor::Blue);

    // ponytail: plain RwSignal (Copy) instead of TwoStepDeleteController
    let armed_group_delete: RwSignal<Option<ProductGroupId>> = RwSignal::new(None);

    // ── Product CRUD state ────────────────────────────────────────────────────
    let editing_product: RwSignal<Option<ProductId>> = RwSignal::new(None);
    let adding_product_to: RwSignal<Option<ProductGroupId>> = RwSignal::new(None);

    let edit_product_name = RwSignal::new(String::new());
    let edit_product_price = RwSignal::new(String::new());
    let edit_product_stock = RwSignal::new(String::new());

    let new_product_name = RwSignal::new(String::new());
    let new_product_price = RwSignal::new(String::new());
    let new_product_stock = RwSignal::new(String::new());

    let armed_product_delete: RwSignal<Option<ProductId>> = RwSignal::new(None);

    // ── Group DnD ────────────────────────────────────────────────────────────
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

    // ── Product DnD (shared across all groups so a drag can cross group
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

    // ── Helpers ───────────────────────────────────────────────────────────────
    let open_edit_group = move |group_id: ProductGroupId| {
        if let Some(g) = groups
            .get_untracked()
            .into_iter()
            .find(|g| g.id == group_id)
        {
            edit_group_name.set(g.name.clone());
            edit_group_emoji.set(g.emoji.clone().unwrap_or_default());
            edit_group_color.set(g.color);
            editing_group.set(Some(group_id));
            new_group_open.set(false);
        }
    };

    let save_edit_group = move |group_id: ProductGroupId| {
        let name = edit_group_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        if let Some(mut group) = groups
            .get_untracked()
            .into_iter()
            .find(|g| g.id == group_id)
        {
            group.name = name;
            group.emoji = {
                let e = edit_group_emoji.get_untracked().trim().to_string();
                if e.is_empty() {
                    None
                } else {
                    Some(e)
                }
            };
            group.color = edit_group_color.get_untracked();
            editing_group.set(None);
            groups.update(|gs| {
                if let Some(g) = gs.iter_mut().find(|g| g.id == group_id) {
                    g.name = group.name.clone();
                    g.emoji = group.emoji.clone();
                    g.color = group.color;
                }
            });
            if let Some(Ok(state)) = app_state.get_untracked() {
                spawn_local(async move {
                    let _ = state.product_group_repository.save(&group).await;
                });
            }
        }
    };

    let save_new_group = move || {
        let name = new_group_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let next_order = groups.get_untracked().len() * 10;
        let group = ProductGroup {
            id: ProductGroupId::new(),
            booth_id,
            name,
            color: new_group_color.get_untracked(),
            emoji: {
                let e = new_group_emoji.get_untracked().trim().to_string();
                if e.is_empty() {
                    None
                } else {
                    Some(e)
                }
            },
            sort_order: next_order as u32,
        };
        new_group_name.set(String::new());
        new_group_emoji.set(String::new());
        new_group_color.set(TailwindColor::Blue);
        new_group_open.set(false);
        groups.update(|gs| gs.push(group.clone()));
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

    let save_new_product = move |group_id: ProductGroupId| {
        let name = new_product_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let price = match parse_decimal_input(&new_product_price.get_untracked()) {
            Ok(p) => p,
            Err(_) => return,
        };
        let initial_stock = match parse_stock_input(&new_product_stock.get_untracked()) {
            Ok(s) => s,
            Err(_) => return,
        };
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
        new_product_name.set(String::new());
        new_product_price.set(String::new());
        new_product_stock.set(String::new());
        adding_product_to.set(None);
        products.update(|ps| ps.push(product.clone()));
        if let Some(Ok(state)) = app_state.get_untracked() {
            spawn_local(async move {
                let _ = state.product_repository.save(&product).await;
            });
        }
    };

    let save_edit_product = move |product_id: ProductId| {
        let name = edit_product_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let price = match parse_decimal_input(&edit_product_price.get_untracked()) {
            Ok(p) => p,
            Err(_) => return,
        };
        let initial_stock = match parse_stock_input(&edit_product_stock.get_untracked()) {
            Ok(s) => s,
            Err(_) => return,
        };
        editing_product.set(None);
        products.update(|ps| {
            if let Some(p) = ps.iter_mut().find(|p| p.id == product_id) {
                p.name = name;
                p.price = price;
                p.initial_stock = initial_stock;
            }
        });
        if let Some(product) = products
            .get_untracked()
            .into_iter()
            .find(|p| p.id == product_id)
        {
            if let Some(Ok(state)) = app_state.get_untracked() {
                spawn_local(async move {
                    let _ = state.product_repository.save(&product).await;
                });
            }
        }
    };

    let delete_product = move |product_id: ProductId| {
        if locked_ids.get_untracked().contains(&product_id) {
            return;
        }
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

                    view! {
                        // Group row
                        <div
                            data-sort-index=move || g_idx.get().to_string()
                            data-sort-scope="group"
                            class=move || group_dnd.item_classes(g_idx.get(), "rounded-lg border border-gray-200 bg-gray-50")
                        >
                            // Group header
                            {move || {
                                if editing_group.get() == Some(group_id) {
                                    // Edit form row
                                    view! {
                                        <div class="flex flex-wrap items-center gap-2 p-3">
                                            <Input
                                                value=edit_group_name
                                                placeholder=t!("product.group_name_placeholder")()
                                                aria_label=t!("product.group_name_label")()
                                            />
                                            <EmojiPicker value=edit_group_emoji />
                                            // Color picker
                                            <div class="flex gap-1 flex-wrap">
                                                {ALL_COLORS.iter().map(|&c| {
                                                    let cls = move || color_picker_button_class(c, edit_group_color.get());
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class=cls
                                                            on:click=move |_| edit_group_color.set(c)
                                                        />
                                                    }
                                                }).collect_view()}
                                            </div>
                                            <button
                                                type="button"
                                                class="px-3 py-2 min-h-[44px] rounded-md bg-blue-600 text-sm font-medium text-white hover:bg-blue-700"
                                                on:click=move |_| save_edit_group(group_id)
                                            >{t!("common.save")()}</button>
                                            <button
                                                type="button"
                                                class="px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-gray-600 hover:bg-gray-200"
                                                on:click=move |_| editing_group.set(None)
                                            >{t!("common.cancel")()}</button>
                                        </div>
                                    }.into_any()
                                } else {
                                    // Read fresh values from signal so display updates after save
                                    let current = groups.get();
                                    let g = current.iter().find(|g| g.id == group_id);
                                    let display_emoji = g.and_then(|g| g.emoji.clone());
                                    let display_name = g.map(|g| g.name.clone()).unwrap_or_else(|| group.name.clone());
                                    let display_color = g.map(|g| g.color).unwrap_or(group.color);
                                    let has_products = move || {
                                        products.get().iter().any(|p| p.product_group_id == group_id)
                                    };
                                    // Tap anywhere on the group header (except delete zone) → edit
                                    view! {
                                        <div
                                            class="flex items-center cursor-pointer"
                                            on:click=move |_| open_edit_group(group_id)
                                        >
                                            // DnD handle — wider hit area, stops click from bubbling to edit
                                            <span
                                                class="cursor-grab touch-none select-none text-gray-400 text-lg leading-none px-3 py-3"
                                                on:pointerdown=group_dnd.on_handle_pointerdown(move || g_idx.get_untracked())
                                                on:pointermove=group_dnd.on_handle_pointermove()
                                                on:pointerup=group_dnd.on_handle_pointerup(save_group_order)
                                                on:pointercancel=group_dnd.on_handle_pointercancel()
                                                on:click=|e| e.stop_propagation()
                                            >"⠿"</span>
                                            <span class=format!("w-3 h-3 rounded-full flex-shrink-0 {}", color_bg(display_color)) />
                                            <span class="flex-1 ml-2 flex items-center gap-1">
                                                <span class="text-sm font-medium text-gray-900">{display_name}</span>
                                                {display_emoji.map(|e| view! { <span>{e}</span> }.into_any())}
                                            </span>
                                            // Delete button — stops propagation so it doesn't trigger group edit
                                            {move || {
                                                if armed_group_delete.get() == Some(group_id) {
                                                    view! {
                                                        <div class="flex items-center gap-2 mr-2">
                                                            <button
                                                                type="button"
                                                                class="px-3 py-1 rounded-md bg-red-600 text-xs font-medium text-white hover:bg-red-700"
                                                                on:click=move |e| {
                                                                    e.stop_propagation();
                                                                    delete_group(group_id);
                                                                    armed_group_delete.set(None);
                                                                }
                                                            >{t!("product.group_delete_confirm")()}</button>
                                                            <button
                                                                type="button"
                                                                class="px-3 py-1 rounded-md bg-gray-100 text-xs text-gray-600 hover:bg-gray-200"
                                                                on:click=move |e| {
                                                                    e.stop_propagation();
                                                                    armed_group_delete.set(None);
                                                                }
                                                            >{t!("common.cancel")()}</button>
                                                        </div>
                                                    }.into_any()
                                                } else if has_products() {
                                                    view! {
                                                        <span
                                                            class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-300 cursor-not-allowed"
                                                            title=t!("product.group_has_products")()
                                                        ><Icon icon=LuTrash2 class="w-5 h-5" /></span>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-400 hover:text-red-600"
                                                            title=t!("common.delete")()
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                armed_group_delete.set(Some(group_id));
                                                            }
                                                        ><Icon icon=LuTrash2 class="w-5 h-5" /></button>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any()
                                }
                            }}

                            // Products list for this group
                            <div class="border-t border-gray-200 px-3 pb-2 pt-1 space-y-1">
                                <For
                                    each=move || {
                                        let mut ps: Vec<(usize, Product)> = products.get()
                                            .into_iter()
                                            .filter(|p| p.product_group_id == group_id)
                                            .enumerate()
                                            .collect();
                                        ps.sort_by_key(|(_, p)| p.sort_order);
                                        ps
                                    }
                                    key=|(_, p)| p.id
                                    children={
                                        let group_id_str = group_id_str.clone();
                                        move |(local_idx, product)| {
                                        let product_id = product.id;
                                        let group_id_for_attr = group_id_str.clone();
                                        let group_id_for_class = group_id_str.clone();
                                        let group_id_for_content = group_id_str.clone();
                                        // Clone for outer tap-to-edit handler; product itself moves into reactive closure
                                        let pfe = product.clone();

                                        view! {
                                            <div
                                                data-sort-index=local_idx.to_string()
                                                data-sort-group=group_id_for_attr
                                                data-sort-scope="product"
                                                class=move || product_dnd.item_classes(group_id_for_class.clone(), local_idx, "flex items-center gap-2 py-1")
                                                // Tap anywhere (except handle / delete) → edit
                                                on:click=move |_| {
                                                    if editing_product.get_untracked().is_none()
                                                        && armed_product_delete.get_untracked().is_none()
                                                    {
                                                        if let Some(p) = products.get_untracked().into_iter().find(|p| p.id == product_id) {
                                                            edit_product_name.set(p.name.clone());
                                                            edit_product_price.set(
                                                                format_decimal_for_input(p.price, locale.get_untracked(), 2)
                                                            );
                                                            edit_product_stock.set(
                                                                p.initial_stock.map(|s| s.to_string()).unwrap_or_default()
                                                            );
                                                            adding_product_to.set(None);
                                                            editing_product.set(Some(product_id));
                                                        }
                                                    }
                                                }
                                            >
                                                {move || {
                                                    if editing_product.get() == Some(product_id) {
                                                        view! {
                                                            <div class="flex flex-wrap items-end gap-2 flex-1">
                                                                <div class="flex-1 min-w-32">
                                                                    <Input
                                                                        value=edit_product_name
                                                                        label=t!("product.product_name_label")()
                                                                        placeholder=t!("product.product_name_placeholder")()
                                                                        aria_label=t!("product.product_name_label")()
                                                                    />
                                                                </div>
                                                                <div class="w-28">
                                                                    <NumberInput
                                                                        value=edit_product_price
                                                                        label=t!("product.product_price_label")()
                                                                        placeholder=t!("common.placeholders.decimal_zero")()
                                                                    />
                                                                </div>
                                                                <div class="w-24">
                                                                    <NumberInput
                                                                        value=edit_product_stock
                                                                        label=t!("product.product_stock_label")()
                                                                        placeholder=t!("product.product_stock_placeholder")()
                                                                    />
                                                                </div>
                                                                <button
                                                                    type="button"
                                                                    class="px-3 py-2 min-h-[44px] rounded-md bg-blue-600 text-sm font-medium text-white hover:bg-blue-700"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        save_edit_product(product_id);
                                                                    }
                                                                >{t!("common.save")()}</button>
                                                                <button
                                                                    type="button"
                                                                    class="px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-gray-600 hover:bg-gray-200"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        editing_product.set(None);
                                                                    }
                                                                >{t!("common.cancel")()}</button>
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        let locked = locked_ids.get().contains(&product_id);
                                                        let armed = armed_product_delete.get() == Some(product_id);
                                                        let (display_name, display_price, display_stock) = products.get()
                                                            .into_iter()
                                                            .find(|p| p.id == product_id)
                                                            .map(|p| (p.name.clone(), p.price, p.initial_stock))
                                                            .unwrap_or_else(|| (pfe.name.clone(), pfe.price, pfe.initial_stock));
                                                        if locked {
                                                            view! {
                                                                <span class="cursor-grab touch-none select-none text-gray-300 leading-none px-3 py-3"
                                                                    on:pointerdown=product_dnd.on_handle_pointerdown(
                                                                        { let g = group_id_for_content.clone(); move || g.clone() },
                                                                        move || local_idx,
                                                                    )
                                                                    on:pointermove=product_dnd.on_handle_pointermove()
                                                                    on:pointerup=product_dnd.on_handle_pointerup(move_product_between_groups)
                                                                    on:pointercancel=product_dnd.on_handle_pointercancel()
                                                                    on:click=|e| e.stop_propagation()
                                                                >"⠿"</span>
                                                                <span class="flex-1 text-sm text-gray-800">{display_name.clone()}</span>
                                                                {display_stock.map(|s| view! {
                                                                    <span class="text-xs text-gray-500 tabular-nums mr-1">
                                                                        {format!("{}: {}", t!("product.product_stock_label")(), s)}
                                                                    </span>
                                                                }.into_any())}
                                                                <span class="text-sm text-gray-600 tabular-nums">
                                                                    {format_currency(display_price, locale.get_untracked())}
                                                                </span>
                                                                <span
                                                                    class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-400"
                                                                    title=t!("product.locked_hint")()
                                                                >"🔒"</span>
                                                            }.into_any()
                                                        } else if armed {
                                                            view! {
                                                                <span class="cursor-grab touch-none select-none text-gray-300 leading-none px-3 py-3"
                                                                    on:pointerdown=product_dnd.on_handle_pointerdown(
                                                                        { let g = group_id_for_content.clone(); move || g.clone() },
                                                                        move || local_idx,
                                                                    )
                                                                    on:pointermove=product_dnd.on_handle_pointermove()
                                                                    on:pointerup=product_dnd.on_handle_pointerup(move_product_between_groups)
                                                                    on:pointercancel=product_dnd.on_handle_pointercancel()
                                                                    on:click=|e| e.stop_propagation()
                                                                >"⠿"</span>
                                                                <span class="flex-1 text-sm text-gray-800">{display_name.clone()}</span>
                                                                {display_stock.map(|s| view! {
                                                                    <span class="text-xs text-gray-500 tabular-nums mr-1">
                                                                        {format!("{}: {}", t!("product.product_stock_label")(), s)}
                                                                    </span>
                                                                }.into_any())}
                                                                <span class="text-sm text-gray-600 tabular-nums">
                                                                    {format_currency(display_price, locale.get_untracked())}
                                                                </span>
                                                                <button type="button"
                                                                    class="px-3 py-2 min-h-[44px] rounded-md bg-red-600 text-xs font-medium text-white hover:bg-red-700"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        delete_product(product_id);
                                                                        armed_product_delete.set(None);
                                                                    }
                                                                >{t!("product.product_delete_confirm")()}</button>
                                                                <button type="button"
                                                                    class="px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-xs text-gray-600 hover:bg-gray-200"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        armed_product_delete.set(None);
                                                                    }
                                                                >{t!("common.cancel")()}</button>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <span class="cursor-grab touch-none select-none text-gray-300 leading-none px-3 py-3"
                                                                    on:pointerdown=product_dnd.on_handle_pointerdown(
                                                                        { let g = group_id_for_content.clone(); move || g.clone() },
                                                                        move || local_idx,
                                                                    )
                                                                    on:pointermove=product_dnd.on_handle_pointermove()
                                                                    on:pointerup=product_dnd.on_handle_pointerup(move_product_between_groups)
                                                                    on:pointercancel=product_dnd.on_handle_pointercancel()
                                                                    on:click=|e| e.stop_propagation()
                                                                >"⠿"</span>
                                                                <span class="flex-1 text-sm text-gray-800">{display_name.clone()}</span>
                                                                {display_stock.map(|s| view! {
                                                                    <span class="text-xs text-gray-500 tabular-nums mr-1">
                                                                        {format!("{}: {}", t!("product.product_stock_label")(), s)}
                                                                    </span>
                                                                }.into_any())}
                                                                <span class="text-sm text-gray-600 tabular-nums">
                                                                    {format_currency(display_price, locale.get_untracked())}
                                                                </span>
                                                                <button type="button"
                                                                    class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-400 hover:text-red-600"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        armed_product_delete.set(Some(product_id));
                                                                    }
                                                                ><Icon icon=LuTrash2 class="w-5 h-5" /></button>
                                                            }.into_any()
                                                        }
                                                    }
                                                }}
                                            </div>
                                        }
                                    }
                                    }
                                />

                                // Empty-group drop target: only rendered while a product is
                                // being dragged, so an empty group still has a droppable
                                // element for hit-testing (a real product row wouldn't exist
                                // for elements_from_point to match against otherwise).
                                {move || {
                                    let has_products = products.get().iter().any(|p| p.product_group_id == group_id);
                                    if !has_products && product_dnd.is_dragging()() {
                                        view! {
                                            <div
                                                data-sort-index="0"
                                                data-sort-group=group_id_str.clone()
                                                data-sort-scope="product"
                                                class={
                                                    let g = group_id_str.clone();
                                                    move || product_dnd.item_classes(g.clone(), 0, "rounded border border-dashed border-gray-300 py-2 text-center text-xs text-gray-400")
                                                }
                                            >
                                                {t!("product.empty_group_drop_hint")()}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }
                                }}

                                // Add product row / form
                                {move || if adding_product_to.get() == Some(group_id) {
                                    view! {
                                        <div class="flex flex-wrap items-end gap-2 pt-1">
                                            <div class="flex-1 min-w-32">
                                                <Input
                                                    value=new_product_name
                                                    label=t!("product.product_name_label")()
                                                    placeholder=t!("product.product_name_placeholder")()
                                                    aria_label=t!("product.product_name_label")()
                                                    autofocus=true
                                                />
                                            </div>
                                            <div class="w-28">
                                                <NumberInput
                                                    value=new_product_price
                                                    label=t!("product.product_price_label")()
                                                    placeholder=t!("common.placeholders.decimal_zero")()
                                                />
                                            </div>
                                            <div class="w-24">
                                                <NumberInput
                                                    value=new_product_stock
                                                    label=t!("product.product_stock_label")()
                                                    placeholder=t!("product.product_stock_placeholder")()
                                                />
                                            </div>
                                            <button
                                                type="button"
                                                class="px-3 py-2 min-h-[44px] rounded-md bg-blue-600 text-sm font-medium text-white hover:bg-blue-700"
                                                on:click=move |_| save_new_product(group_id)
                                            >{t!("common.save")()}</button>
                                            <button
                                                type="button"
                                                class="px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-gray-600 hover:bg-gray-200"
                                                on:click=move |_| adding_product_to.set(None)
                                            >{t!("common.cancel")()}</button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <button
                                            type="button"
                                            class="mt-1 flex w-full min-h-[44px] items-center text-sm text-blue-600 hover:text-blue-800"
                                            on:click=move |_| {
                                                new_product_name.set(String::new());
                                                new_product_price.set(String::new());
                                                new_product_stock.set(String::new());
                                                editing_product.set(None);
                                                adding_product_to.set(Some(group_id));
                                            }
                                        >"+ "{t!("product.add_product")()}</button>
                                    }.into_any()
                                }}
                            </div>
                        </div>
                    }
                }
            />

            // Add new group row / form
            {move || if new_group_open.get() {
                view! {
                    <div class="rounded-lg border border-blue-200 bg-blue-50 p-3 flex flex-wrap items-center gap-2">
                        <Input
                            value=new_group_name
                            placeholder=t!("product.group_name_placeholder")()
                            aria_label=t!("product.group_name_label")()
                            autofocus=true
                        />
                        <EmojiPicker value=new_group_emoji />
                        <div class="flex gap-1 flex-wrap">
                            {ALL_COLORS.iter().map(|&c| {
                                let cls = move || color_picker_button_class(c, new_group_color.get());
                                view! {
                                    <button
                                        type="button"
                                        class=cls
                                        on:click=move |_| new_group_color.set(c)
                                    />
                                }
                            }).collect_view()}
                        </div>
                        <button
                            type="button"
                            class="px-3 py-2 min-h-[44px] rounded-md bg-blue-600 text-sm font-medium text-white hover:bg-blue-700"
                            on:click=move |_| save_new_group()
                        >{t!("common.save")()}</button>
                        <button
                            type="button"
                            class="px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-sm text-gray-600 hover:bg-gray-200"
                            on:click=move |_| new_group_open.set(false)
                        >{t!("common.cancel")()}</button>
                    </div>
                }.into_any()
            } else {
                view! {
                    <button
                        type="button"
                        class="flex min-h-[44px] items-center text-sm font-medium text-blue-600 hover:text-blue-800"
                        on:click=move |_| {
                            editing_group.set(None);
                            new_group_open.set(true);
                        }
                    >"+ "{t!("product.add_group")()}</button>
                }.into_any()
            }}
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
}

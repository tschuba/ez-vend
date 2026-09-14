#![allow(clippy::clone_on_copy)]

use crate::components::{Input, NumberInput};
use crate::formatting::{format_currency, format_decimal_for_input, parse_decimal_input};
use crate::hooks::sortable::{apply_reorder, use_sortable};
use crate::i18n::use_locale;
use crate::state::use_app_state;
use crate::t;
use domain::models::shared::{BoothId, ProductGroupId, ProductId};
use domain::models::{Product, ProductGroup, TailwindColor};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashSet;

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

    let new_product_name = RwSignal::new(String::new());
    let new_product_price = RwSignal::new(String::new());

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

    // ── Helpers ───────────────────────────────────────────────────────────────
    let open_edit_group = move |group: &ProductGroup| {
        edit_group_name.set(group.name.clone());
        edit_group_emoji.set(group.emoji.clone().unwrap_or_default());
        edit_group_color.set(group.color);
        editing_group.set(Some(group.id));
        new_group_open.set(false);
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
        };
        new_product_name.set(String::new());
        new_product_price.set(String::new());
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
        editing_product.set(None);
        products.update(|ps| {
            if let Some(p) = ps.iter_mut().find(|p| p.id == product_id) {
                p.name = name;
                p.price = price;
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
                    let product_dnd = use_sortable("product");

                    let save_product_order = move |from: usize, to: usize| {
                        let all = products.get_untracked();
                        let mut group_ps: Vec<Product> = all
                            .iter()
                            .filter(|p| p.product_group_id == group_id)
                            .cloned()
                            .collect();
                        group_ps.sort_by_key(|p| p.sort_order);
                        group_ps = apply_reorder(group_ps, from, to);
                        for (i, p) in group_ps.iter_mut().enumerate() {
                            p.sort_order = (i * 10) as u32;
                        }
                        // Optimistic update
                        let mut updated_all: Vec<Product> = all
                            .into_iter()
                            .filter(|p| p.product_group_id != group_id)
                            .collect();
                        updated_all.extend(group_ps.iter().cloned());
                        updated_all.sort_by_key(|p| p.sort_order);
                        products.set(updated_all);
                        if let Some(Ok(state)) = app_state.get_untracked() {
                            spawn_local(async move {
                                for p in &group_ps {
                                    let _ = state.product_repository.save(p).await;
                                }
                            });
                        }
                    };

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
                                            <input
                                                type="text"
                                                class="w-14 rounded border border-gray-300 px-2 py-1.5 text-sm"
                                                placeholder=t!("product.group_emoji_placeholder")()
                                                prop:value=move || edit_group_emoji.get()
                                                on:input=move |ev| edit_group_emoji.set(event_target_value(&ev))
                                            />
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
                                    let group_clone = group.clone();
                                    let has_products = move || {
                                        products.get().iter().any(|p| p.product_group_id == group_id)
                                    };
                                    // Tap anywhere on the group header (except delete zone) → edit
                                    view! {
                                        <div
                                            class="flex items-center cursor-pointer"
                                            on:click=move |_| open_edit_group(&group_clone)
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
                                            // Color dot
                                            <span class=format!("w-3 h-3 rounded-full flex-shrink-0 {}", color_bg(group.color)) />
                                            // Emoji + Name
                                            {group.emoji.as_deref().map(|e| view! { <span class="ml-1">{e.to_string()}</span> })}
                                            <span class="flex-1 ml-2 text-sm font-medium text-gray-900">{group.name.clone()}</span>
                                            // Delete button — stops propagation so it doesn't trigger group edit
                                            {move || {
                                                if armed_group_delete.get() == Some(group_id) {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="px-3 py-2 min-h-[44px] rounded-md bg-red-600 text-xs font-medium text-white hover:bg-red-700"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                delete_group(group_id);
                                                                armed_group_delete.set(None);
                                                            }
                                                        >{t!("product.group_delete_confirm")()}</button>
                                                        <button
                                                            type="button"
                                                            class="px-3 py-2 min-h-[44px] rounded-md bg-gray-100 text-xs text-gray-600 hover:bg-gray-200 mr-2"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                armed_group_delete.set(None);
                                                            }
                                                        >{t!("common.cancel")()}</button>
                                                    }.into_any()
                                                } else if has_products() {
                                                    view! {
                                                        <span
                                                            class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-300 cursor-not-allowed"
                                                            title=t!("product.group_has_products")()
                                                        >"🗑"</span>
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
                                                        >"🗑"</button>
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
                                    children=move |(local_idx, product)| {
                                        let product_id = product.id;
                                        // Clone for outer tap-to-edit handler; product itself moves into reactive closure
                                        let pfe = product.clone();

                                        view! {
                                            <div
                                                data-sort-index=local_idx.to_string()
                                                data-sort-scope="product"
                                                class=move || product_dnd.item_classes(local_idx, "flex items-center gap-2 py-1")
                                                // Tap anywhere (except handle / delete) → edit
                                                on:click=move |_| {
                                                    if editing_product.get_untracked().is_none()
                                                        && armed_product_delete.get_untracked().is_none()
                                                        && !locked_ids.get_untracked().contains(&product_id)
                                                    {
                                                        edit_product_name.set(pfe.name.clone());
                                                        edit_product_price.set(
                                                            format_decimal_for_input(pfe.price, locale.get_untracked(), 2)
                                                        );
                                                        adding_product_to.set(None);
                                                        editing_product.set(Some(product_id));
                                                    }
                                                }
                                            >
                                                {move || {
                                                    if editing_product.get() == Some(product_id) {
                                                        view! {
                                                            <div class="flex flex-wrap items-center gap-2 flex-1">
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
                                                        if locked {
                                                            view! {
                                                                <span class="cursor-grab touch-none select-none text-gray-300 leading-none px-3 py-3"
                                                                    on:pointerdown=product_dnd.on_handle_pointerdown(move || local_idx)
                                                                    on:pointermove=product_dnd.on_handle_pointermove()
                                                                    on:pointerup=product_dnd.on_handle_pointerup(save_product_order)
                                                                    on:pointercancel=product_dnd.on_handle_pointercancel()
                                                                    on:click=|e| e.stop_propagation()
                                                                >"⠿"</span>
                                                                <span class="flex-1 text-sm text-gray-800">{product.name.clone()}</span>
                                                                <span class="text-sm text-gray-600 tabular-nums">
                                                                    {format_currency(product.price, locale.get_untracked())}
                                                                </span>
                                                                <span
                                                                    class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-400"
                                                                    title=t!("product.locked_hint")()
                                                                >"🔒"</span>
                                                            }.into_any()
                                                        } else if armed {
                                                            view! {
                                                                <span class="cursor-grab touch-none select-none text-gray-300 leading-none px-3 py-3"
                                                                    on:pointerdown=product_dnd.on_handle_pointerdown(move || local_idx)
                                                                    on:pointermove=product_dnd.on_handle_pointermove()
                                                                    on:pointerup=product_dnd.on_handle_pointerup(save_product_order)
                                                                    on:pointercancel=product_dnd.on_handle_pointercancel()
                                                                    on:click=|e| e.stop_propagation()
                                                                >"⠿"</span>
                                                                <span class="flex-1 text-sm text-gray-800">{product.name.clone()}</span>
                                                                <span class="text-sm text-gray-600 tabular-nums">
                                                                    {format_currency(product.price, locale.get_untracked())}
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
                                                                    on:pointerdown=product_dnd.on_handle_pointerdown(move || local_idx)
                                                                    on:pointermove=product_dnd.on_handle_pointermove()
                                                                    on:pointerup=product_dnd.on_handle_pointerup(save_product_order)
                                                                    on:pointercancel=product_dnd.on_handle_pointercancel()
                                                                    on:click=|e| e.stop_propagation()
                                                                >"⠿"</span>
                                                                <span class="flex-1 text-sm text-gray-800">{product.name.clone()}</span>
                                                                <span class="text-sm text-gray-600 tabular-nums">
                                                                    {format_currency(product.price, locale.get_untracked())}
                                                                </span>
                                                                <button type="button"
                                                                    class="flex items-center justify-center min-w-[44px] min-h-[44px] text-gray-400 hover:text-red-600"
                                                                    on:click=move |e| {
                                                                        e.stop_propagation();
                                                                        armed_product_delete.set(Some(product_id));
                                                                    }
                                                                >"🗑"</button>
                                                            }.into_any()
                                                        }
                                                    }
                                                }}
                                            </div>
                                        }
                                    }
                                />

                                // Add product row / form
                                {move || if adding_product_to.get() == Some(group_id) {
                                    view! {
                                        <div class="flex flex-wrap items-center gap-2 pt-1">
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
                        <input
                            type="text"
                            class="w-14 rounded border border-gray-300 px-2 py-1.5 text-sm"
                            placeholder=t!("product.group_emoji_placeholder")()
                            prop:value=move || new_group_emoji.get()
                            on:input=move |ev| new_group_emoji.set(event_target_value(&ev))
                        />
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

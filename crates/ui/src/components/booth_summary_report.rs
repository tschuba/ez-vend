use crate::components::pagination::Pagination;
use crate::formatting::{format_currency, format_percentage_smart};
use crate::i18n::{translate_with_params, use_locale, Locale};
use crate::t;
use chrono::{Datelike, Local, NaiveDate};
use domain::models::{BoothSummary, BoothType, TailwindColor, VendorBoothSummary};
use leptos::prelude::*;
use std::collections::HashMap;

fn configured_participation_fee_label(
    fee: rust_decimal::Decimal,
    locale: crate::i18n::Locale,
) -> String {
    let mut params = HashMap::new();
    params.insert("fee", format_currency(fee, locale));
    translate_with_params("report.total_participation_fees_with_config", params)
}

fn configured_sales_fee_label(
    percent: rust_decimal::Decimal,
    locale: crate::i18n::Locale,
) -> String {
    let mut params = HashMap::new();
    params.insert("percent", format_percentage_smart(percent, locale));
    translate_with_params("report.total_sales_fees_with_config", params)
}

fn format_booth_date(date: NaiveDate, locale: Locale) -> String {
    match locale {
        Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => {
            let month_name = match date.month() {
                1 => "Januar",
                2 => "Februar",
                3 => "März",
                4 => "April",
                5 => "Mai",
                6 => "Juni",
                7 => "Juli",
                8 => "August",
                9 => "September",
                10 => "Oktober",
                11 => "November",
                12 => "Dezember",
                _ => "",
            };

            format!("{}. {} {}", date.day(), month_name, date.year())
        }
        Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => {
            date.format("%B %d, %Y").to_string()
        }
    }
}

fn color_left_border(c: TailwindColor) -> &'static str {
    match c {
        TailwindColor::Red => "border-l-4 border-red-500",
        TailwindColor::Orange => "border-l-4 border-orange-500",
        TailwindColor::Amber => "border-l-4 border-amber-500",
        TailwindColor::Green => "border-l-4 border-green-500",
        TailwindColor::Teal => "border-l-4 border-teal-500",
        TailwindColor::Blue => "border-l-4 border-blue-500",
        TailwindColor::Violet => "border-l-4 border-violet-500",
        TailwindColor::Pink => "border-l-4 border-pink-500",
    }
}

fn color_dot(c: TailwindColor) -> &'static str {
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

fn vendor_row_view(vs: &VendorBoothSummary, locale: RwSignal<crate::i18n::Locale>) -> AnyView {
    let vendor_id_str = vs.vendor_id.to_string();
    let net_payout = vs.net_payout;
    let gross_sales = vs.gross_sales;
    let fees_due = vs.fees_due;
    let item_count = vs.item_count;

    view! {
        <tr class="hover:bg-gray-50">
            <td class="px-4 py-3 text-sm font-medium text-gray-900">{vendor_id_str}</td>
            <td class="px-4 py-3 text-sm font-semibold text-gray-900 text-right">
                {move || format_currency(net_payout, locale.get())}
            </td>
            <td class="px-4 py-3 text-sm text-gray-700 text-right">
                {move || format_currency(gross_sales, locale.get())}
            </td>
            <td class="px-4 py-3 text-sm text-gray-700 text-right">
                {move || format_currency(fees_due, locale.get())}
            </td>
            <td class="px-4 py-3 text-sm text-gray-700 text-right">{item_count}</td>
        </tr>
    }
    .into_any()
}

#[component]
pub fn BoothSummaryDisplay(summary: BoothSummary) -> impl IntoView {
    let locale = use_locale();
    let is_direct_sale = summary.booth_type == BoothType::DirectSale;
    let total_revenue = summary.total_revenue;
    let total_purchases = summary.total_purchases;
    let total_items = summary.total_items;
    let unique_vendors = summary.unique_vendors;
    let participation_fee = summary.participation_fee;
    let sales_fee_percent = summary.sales_fee_percent;
    let total_participation_fees = summary.total_participation_fees;
    let total_sales_fees = summary.total_sales_fees;
    let total_booth_revenue = summary.total_booth_revenue;
    let vendor_summaries = StoredValue::new_local(summary.vendor_summaries);
    let total_vendors = vendor_summaries.with_value(|rows| rows.len());
    let has_vendor_summaries = total_vendors > 0;
    let (current_page, set_current_page) = signal(0usize);
    let (page_size, set_page_size) = signal(10usize);
    let product_group_summaries = StoredValue::new_local(summary.product_group_summaries);

    // ponytail: plain closure, not Memo — Vec<AnyView> is !Clone so Memo<Vec<AnyView>> can't .get()
    let vendor_rows = move || {
        vendor_summaries.with_value(|rows| {
            let page = current_page.get();
            let size = page_size.get();
            let start = page.saturating_mul(size);

            rows.iter()
                .skip(start)
                .take(size)
                .map(|vs| vendor_row_view(vs, locale))
                .collect_view()
        })
    };

    view! {
        <div class="space-y-6">
            // Fee box — ThirdPartySale only
            {if !is_direct_sale {
                view! {
                    <div class="border rounded-lg p-6 bg-gradient-to-br from-blue-50 to-indigo-50">
                        <div class="space-y-3">
                            <div class="flex justify-between items-center gap-4">
                                <span class="text-gray-700">
                                    {move || configured_participation_fee_label(participation_fee, locale.get())}
                                </span>
                                <span class="font-semibold text-gray-900">
                                    {move || format_currency(total_participation_fees, locale.get())}
                                </span>
                            </div>
                            <div class="flex justify-between items-center gap-4">
                                <span class="text-gray-700">
                                    {move || configured_sales_fee_label(sales_fee_percent, locale.get())}
                                </span>
                                <span class="font-semibold text-gray-900">
                                    {move || format_currency(total_sales_fees, locale.get())}
                                </span>
                            </div>
                            <div class="flex justify-between items-center gap-4 pt-3 border-t-2 border-blue-200">
                                <span class="text-lg font-bold text-gray-900">{t!("report.total_booth_revenue")}</span>
                                <span class="text-2xl font-bold text-blue-700">
                                    {move || format_currency(total_booth_revenue, locale.get())}
                                </span>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}

            // Stats grid — 3 tiles for DirectSale, 4 for ThirdPartySale
            <div class={if is_direct_sale {
                "grid grid-cols-1 sm:grid-cols-3 gap-4"
            } else {
                "grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4"
            }}>
                <div class="p-4 bg-blue-50 rounded-lg">
                    <p class="text-sm text-gray-600">{t!("report.sales_total")}</p>
                    <p class="text-2xl font-bold text-blue-600">
                        {move || format_currency(total_revenue, locale.get())}
                    </p>
                </div>
                <div class="p-4 bg-green-50 rounded-lg">
                    <p class="text-sm text-gray-600">{t!("report.purchase_count")}</p>
                    <p class="text-2xl font-bold text-green-600">{total_purchases}</p>
                </div>
                <div class="p-4 bg-orange-50 rounded-lg">
                    <p class="text-sm text-gray-600">{t!("report.items")}</p>
                    <p class="text-2xl font-bold text-orange-600">{total_items}</p>
                </div>
                {if !is_direct_sale {
                    view! {
                        <div class="p-4 bg-purple-50 rounded-lg">
                            <p class="text-sm text-gray-600">{t!("report.vendors_count")}</p>
                            <p class="text-2xl font-bold text-purple-600">{unique_vendors}</p>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>

            // Table area — product groups for DirectSale, vendor table for ThirdPartySale
            {if is_direct_sale {
                view! {
                    <div class="border rounded-lg overflow-hidden">
                        {product_group_summaries.with_value(|groups| {
                            if groups.is_empty() {
                                view! {
                                    <div class="p-6 text-sm text-gray-600">{t!("report.no_data")}</div>
                                }.into_any()
                            } else {
                                groups.iter().map(|group| {
                                    let border_class = color_left_border(group.color);
                                    let dot_class = color_dot(group.color);
                                    let group_name = group.group_name.clone();
                                    let emoji = group.products.first().map(|_| group.group_name.clone()).unwrap_or_default();
                                    let _ = emoji;
                                    let subtotal = group.subtotal;
                                    let products = group.products.clone();
                                    view! {
                                        <div class=format!("border-b last:border-b-0 {}", border_class)>
                                            <div class="px-4 py-2 bg-gray-50 flex items-center gap-2">
                                                <span class=format!("w-3 h-3 rounded-full inline-block {}", dot_class)></span>
                                                <span class="font-medium text-sm text-gray-800">{group_name}</span>
                                            </div>
                                            <div class="overflow-x-auto">
                                                <table class="min-w-full divide-y divide-gray-200">
                                                    <thead class="bg-white">
                                                        <tr>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                                {t!("report.vendor_id")}
                                                            </th>
                                                            <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                                {t!("report.unit_price")}
                                                            </th>
                                                            <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                                {t!("report.quantity")}
                                                            </th>
                                                            <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                                {t!("report.gross_sales")}
                                                            </th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="bg-white divide-y divide-gray-100">
                                                        {products.into_iter().map(|pl| {
                                                            let name = pl.name.clone();
                                                            let unit_price = pl.unit_price;
                                                            let count = pl.count;
                                                            let total = pl.total;
                                                            view! {
                                                                <tr class="hover:bg-gray-50">
                                                                    <td class="px-4 py-2 text-sm text-gray-900">{name}</td>
                                                                    <td class="px-4 py-2 text-sm text-gray-700 text-right">
                                                                        {move || format_currency(unit_price, locale.get())}
                                                                    </td>
                                                                    <td class="px-4 py-2 text-sm text-gray-700 text-right">{count}</td>
                                                                    <td class="px-4 py-2 text-sm font-medium text-gray-900 text-right">
                                                                        {move || format_currency(total, locale.get())}
                                                                    </td>
                                                                </tr>
                                                            }
                                                        }).collect_view()}
                                                    </tbody>
                                                </table>
                                            </div>
                                            <div class="px-4 py-2 bg-gray-50 flex justify-between text-sm font-semibold text-gray-700">
                                                <span>{t!("report.subtotal")}</span>
                                                <span>{move || format_currency(subtotal, locale.get())}</span>
                                            </div>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }
                        })}
                        <div class="px-4 py-3 bg-blue-50 flex justify-between items-center font-bold text-gray-900">
                            <span>{t!("report.sales_total")}</span>
                            <span class="text-lg text-blue-700">
                                {move || format_currency(total_revenue, locale.get())}
                            </span>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="border rounded-lg overflow-hidden">
                        <Show
                            when=move || has_vendor_summaries
                            fallback=move || view! {
                                <div class="p-6 text-sm text-gray-600">{t!("report.no_data")}</div>
                            }
                        >
                            <div class="px-4 border-b border-gray-200">
                                <Pagination
                                    current_page=current_page
                                    total_items=Signal::derive(move || total_vendors)
                                    page_size=page_size
                                    on_page_change=move |page| set_current_page.set(page)
                                    on_page_size_change=move |size| {
                                        set_page_size.set(size);
                                        set_current_page.set(0);
                                    }
                                    translation_prefix="vendor.pagination"
                                    show_page_size_selector=true
                                />
                            </div>
                            <div class="overflow-x-auto">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {t!("report.vendor_id")}
                                            </th>
                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {t!("report.net_payout")}
                                            </th>
                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {t!("report.gross_sales")}
                                            </th>
                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {t!("report.fees_due")}
                                            </th>
                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {t!("report.item_count")}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white divide-y divide-gray-200">
                                        {vendor_rows}
                                    </tbody>
                                </table>
                            </div>
                            <div class="px-4 border-t border-gray-200 bg-white">
                                <Pagination
                                    current_page=current_page
                                    total_items=Signal::derive(move || total_vendors)
                                    page_size=page_size
                                    on_page_change=move |page| set_current_page.set(page)
                                    on_page_size_change=move |size| {
                                        set_page_size.set(size);
                                        set_current_page.set(0);
                                    }
                                    translation_prefix="vendor.pagination"
                                    show_page_size_selector=true
                                />
                            </div>
                        </Show>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
pub fn PrintBoothSummary(
    summary: BoothSummary,
    booth_name: String,
    booth_date: NaiveDate,
) -> impl IntoView {
    let locale = use_locale();
    let booth_name = StoredValue::new_local(booth_name);
    let is_direct_sale = summary.booth_type == BoothType::DirectSale;
    let total_revenue = summary.total_revenue;
    let total_purchases = summary.total_purchases;
    let total_items = summary.total_items;
    let unique_vendors = summary.unique_vendors;
    let participation_fee = summary.participation_fee;
    let sales_fee_percent = summary.sales_fee_percent;
    let total_participation_fees = summary.total_participation_fees;
    let total_sales_fees = summary.total_sales_fees;
    let total_booth_revenue = summary.total_booth_revenue;
    let vendor_summaries = summary.vendor_summaries;
    let has_vendor_summaries = !vendor_summaries.is_empty();
    let vendor_rows = vendor_summaries
        .into_iter()
        .map(|vs| {
            let vendor_id_str = vs.vendor_id.to_string();
            let net_payout = vs.net_payout;
            let gross_sales = vs.gross_sales;
            let fees_due = vs.fees_due;
            view! {
                <tr class="border-b border-gray-300">
                    <td class="px-4 py-3 font-medium">{vendor_id_str}</td>
                    <td class="px-4 py-3 text-right font-semibold">
                        {move || format_currency(net_payout, locale.get())}
                    </td>
                    <td class="px-4 py-3 text-right">
                        {move || format_currency(gross_sales, locale.get())}
                    </td>
                    <td class="px-4 py-3 text-right">
                        {move || format_currency(fees_due, locale.get())}
                    </td>
                    <td class="px-4 py-3 text-right">{vs.item_count}</td>
                </tr>
            }
        })
        .collect_view();
    let vendor_rows = StoredValue::new_local(vendor_rows);
    let product_group_summaries = summary.product_group_summaries;

    view! {
        <div class="p-8 max-w-4xl mx-auto">
            <div class="mb-8 pb-4 border-b-2 border-gray-800">
                <h1 class="text-3xl font-bold mb-2">{move || booth_name.get_value()}</h1>
                <p class="text-lg text-gray-700">{move || format_booth_date(booth_date, locale.get())}</p>
            </div>

            // Fee box — ThirdPartySale only
            {if !is_direct_sale {
                view! {
                    <div class="mb-8 border-2 border-gray-400 p-6 rounded bg-gray-50">
                        <div class="space-y-2">
                            <div class="flex justify-between text-base gap-4">
                                <span class="text-gray-700">
                                    {move || configured_participation_fee_label(participation_fee, locale.get())}
                                </span>
                                <span class="font-semibold">
                                    {move || format_currency(total_participation_fees, locale.get())}
                                </span>
                            </div>
                            <div class="flex justify-between text-base gap-4">
                                <span class="text-gray-700">
                                    {move || configured_sales_fee_label(sales_fee_percent, locale.get())}
                                </span>
                                <span class="font-semibold">
                                    {move || format_currency(total_sales_fees, locale.get())}
                                </span>
                            </div>
                            <div class="flex justify-between pt-3 border-t-2 border-gray-800 text-lg gap-4">
                                <span class="font-bold">{t!("report.total_booth_revenue")}</span>
                                <span class="font-bold text-2xl">
                                    {move || format_currency(total_booth_revenue, locale.get())}
                                </span>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}

            // Stats grid — 3 tiles for DirectSale, 4 for ThirdPartySale
            <div class="mb-8">
                <div class={if is_direct_sale { "grid grid-cols-3 gap-6 mb-6" } else { "grid grid-cols-4 gap-6 mb-6" }}>
                    <div class="border-2 border-gray-300 p-4 rounded">
                        <p class="text-sm text-gray-600 mb-1">{t!("report.sales_total")}</p>
                        <p class="text-3xl font-bold">
                            {move || format_currency(total_revenue, locale.get())}
                        </p>
                    </div>
                    <div class="border-2 border-gray-300 p-4 rounded">
                        <p class="text-sm text-gray-600 mb-1">{t!("report.purchase_count")}</p>
                        <p class="text-3xl font-bold">{total_purchases}</p>
                    </div>
                    <div class="border-2 border-gray-300 p-4 rounded">
                        <p class="text-sm text-gray-600 mb-1">{t!("report.items")}</p>
                        <p class="text-3xl font-bold">{total_items}</p>
                    </div>
                    {if !is_direct_sale {
                        view! {
                            <div class="border-2 border-gray-300 p-4 rounded">
                                <p class="text-sm text-gray-600 mb-1">{t!("report.vendors_count")}</p>
                                <p class="text-3xl font-bold">{unique_vendors}</p>
                            </div>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }}
                </div>
            </div>

            // Table area — product groups for DirectSale, vendor table for ThirdPartySale
            <div>
                {if is_direct_sale {
                    let groups_view = product_group_summaries.into_iter().map(|group| {
                        let border_class = color_left_border(group.color);
                        let dot_class = color_dot(group.color);
                        let group_name = group.group_name.clone();
                        let subtotal = group.subtotal;
                        let products = group.products;
                        view! {
                            <div class=format!("mb-6 {}", border_class)>
                                <div class="flex items-center gap-2 mb-2">
                                    <span class=format!("w-3 h-3 rounded-full inline-block {}", dot_class)></span>
                                    <span class="font-bold text-lg">{group_name}</span>
                                </div>
                                <table class="w-full border-collapse mb-1">
                                    <thead>
                                        <tr class="border-b border-gray-400">
                                            <th class="px-3 py-2 text-left font-semibold">{t!("report.vendor_id")}</th>
                                            <th class="px-3 py-2 text-right font-semibold">{t!("report.unit_price")}</th>
                                            <th class="px-3 py-2 text-right font-semibold">{t!("report.quantity")}</th>
                                            <th class="px-3 py-2 text-right font-semibold">{t!("report.gross_sales")}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {products.into_iter().map(|pl| {
                                            let name = pl.name;
                                            let unit_price = pl.unit_price;
                                            let count = pl.count;
                                            let total = pl.total;
                                            view! {
                                                <tr class="border-b border-gray-200">
                                                    <td class="px-3 py-2">{name}</td>
                                                    <td class="px-3 py-2 text-right">
                                                        {move || format_currency(unit_price, locale.get())}
                                                    </td>
                                                    <td class="px-3 py-2 text-right">{count}</td>
                                                    <td class="px-3 py-2 text-right font-medium">
                                                        {move || format_currency(total, locale.get())}
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                                <div class="flex justify-between px-3 py-1 text-sm font-semibold text-gray-700">
                                    <span>{t!("report.subtotal")}</span>
                                    <span>{move || format_currency(subtotal, locale.get())}</span>
                                </div>
                            </div>
                        }
                    }).collect_view();
                    view! {
                        <div>
                            {groups_view}
                            <div class="flex justify-between pt-4 border-t-2 border-gray-800 text-lg">
                                <span class="font-bold">{t!("report.sales_total")}</span>
                                <span class="font-bold text-2xl">
                                    {move || format_currency(total_revenue, locale.get())}
                                </span>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <Show
                            when=move || has_vendor_summaries
                            fallback=move || view! {
                                <p class="text-sm text-gray-600">{t!("report.no_data")}</p>
                            }
                        >
                            <table class="w-full border-collapse">
                                <thead>
                                    <tr class="border-b-2 border-gray-800">
                                        <th class="px-4 py-3 text-left font-bold">{t!("report.vendor_id")}</th>
                                        <th class="px-4 py-3 text-right font-bold">{t!("report.net_payout")}</th>
                                        <th class="px-4 py-3 text-right font-bold">{t!("report.gross_sales")}</th>
                                        <th class="px-4 py-3 text-right font-bold">{t!("report.fees_due")}</th>
                                        <th class="px-4 py-3 text-right font-bold">{t!("report.item_count")}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || vendor_rows.get_value()}
                                </tbody>
                            </table>
                        </Show>
                    }.into_any()
                }}
            </div>

            <div class="mt-8 pt-4 border-t border-gray-400 text-sm text-gray-600">
                <p>
                    {t!("report.generated_at")} " "
                    {Local::now().format("%d.%m.%Y %H:%M").to_string()}
                </p>
            </div>
        </div>
    }
}

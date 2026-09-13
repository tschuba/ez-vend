use leptos::*;
use log::warn;

use crate::components::pagination::Pagination;
use crate::formatting::{format_currency, format_date_with_contextual_year, format_datetime};
use crate::i18n::use_locale;
use crate::t;
use domain::models::booth::{ArchivedVendorSummary, Booth};

#[component]
pub fn ArchivedBoothSummaryDisplay(booth: Booth) -> impl IntoView {
    let locale = use_locale();
    let archived_at = booth.archived_at;
    let archived_label = create_memo(move |_| {
        archived_at.map(|timestamp| {
            t!("archive.archived_at")()
                .replace("{timestamp}", &format_datetime(timestamp, locale.get()))
        })
    });

    let Some(summary) = booth.archived_summary.clone() else {
        return view! {
            <div class="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
                {t!("archive.summary_missing")()}
            </div>
        }
        .into_view();
    };

    let summary = store_value(summary);
    let vendor_count = summary.with_value(|summary| summary.vendor_summaries.len());
    let (current_page, set_current_page) = create_signal(0usize);
    let (page_size, set_page_size) = create_signal(10usize);

    let paged_vendor_summaries = create_memo(move |_| {
        summary.with_value(|summary| {
            let start = current_page.get().saturating_mul(page_size.get());
            summary
                .vendor_summaries
                .iter()
                .skip(start)
                .take(page_size.get())
                .cloned()
                .collect::<Vec<_>>()
        })
    });

    view! {
        <div class="space-y-6">
            <div class="rounded-lg border border-slate-200 bg-slate-50 px-5 py-4">
                <div class="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                    <div>
                        <h3 class="text-lg font-semibold text-slate-900">{booth.description.clone()}</h3>
                        <p class="text-sm text-slate-600">{format_date_with_contextual_year(booth.date, locale.get())}</p>
                    </div>
                    <div class="text-sm text-slate-600">{move || archived_label.get().unwrap_or_default()}</div>
                </div>
            </div>

            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
                <SummaryMetricCard label=t!("report.sales_total")() value=summary.with_value(|summary| format_currency(summary.total_revenue, locale.get())) />
                <SummaryMetricCard label=t!("report.purchase_count")() value=summary.with_value(|summary| summary.purchase_count.to_string()) />
                <SummaryMetricCard label=t!("report.items")() value=summary.with_value(|summary| summary.item_count.to_string()) />
                <SummaryMetricCard label=t!("report.vendors_count")() value=summary.with_value(|summary| summary.vendor_count.to_string()) />
            </div>

            <div class="rounded-lg border border-blue-200 bg-blue-50 px-5 py-4">
                <div class="flex items-center justify-between gap-4">
                    <span class="text-sm font-medium text-slate-700">{t!("report.total_booth_revenue")()}</span>
                    <span class="text-2xl font-bold text-blue-700">{summary.with_value(|summary| format_currency(summary.total_booth_revenue, locale.get()))}</span>
                </div>
            </div>

            <div class="rounded-lg border border-gray-200 overflow-hidden">
                <div class="border-b border-gray-200 bg-white px-4">
                    <Pagination
                        current_page=current_page
                        total_items=Signal::derive(move || vendor_count)
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
                                <th class="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500">{t!("report.vendor_id")()}</th>
                                <th class="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-gray-500">{t!("report.gross_sales")()}</th>
                                <th class="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-gray-500">{t!("report.fees_due")()}</th>
                                <th class="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-gray-500">{t!("report.net_payout")()}</th>
                                <th class="px-4 py-3 text-right text-xs font-medium uppercase tracking-wider text-gray-500">{t!("report.item_count")()}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200 bg-white">
                            <For
                                each=move || paged_vendor_summaries.get()
                                key=|vendor| vendor.vendor_id.as_str().to_string()
                                children=move |vendor| vendor_row(vendor, locale)
                            />
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    }
    .into_view()
}

#[component]
pub fn PrintArchivedBoothSummary(booth: Booth) -> impl IntoView {
    let locale = use_locale();
    let archived_at = booth.archived_at;
    let archived_label = create_memo(move |_| {
        archived_at.map(|timestamp| {
            t!("archive.archived_at")()
                .replace("{timestamp}", &format_datetime(timestamp, locale.get()))
        })
    });

    let Some(summary) = booth.archived_summary.clone() else {
        warn!(
            "print view: archived booth {} missing archived_summary",
            booth.id.as_str()
        );
        return ().into_view();
    };

    let vendor_summaries = summary.vendor_summaries.clone();

    view! {
        <div class="mx-auto max-w-5xl p-8">
            <div class="mb-8 border-b-2 border-gray-800 pb-4">
                <h1 class="text-3xl font-bold">{booth.description.clone()}</h1>
                <p class="text-lg text-gray-700">{format_date_with_contextual_year(booth.date, locale.get())}</p>
                <Show when=move || archived_label.get().is_some()>
                    <p class="mt-2 text-base text-gray-600">{move || archived_label.get().unwrap_or_default()}</p>
                </Show>
            </div>

            <div class="mb-8 grid grid-cols-4 gap-6">
                <PrintMetricCard label=t!("report.sales_total")() value=format_currency(summary.total_revenue, locale.get()) />
                <PrintMetricCard label=t!("report.purchase_count")() value=summary.purchase_count.to_string() />
                <PrintMetricCard label=t!("report.items")() value=summary.item_count.to_string() />
                <PrintMetricCard label=t!("report.vendors_count")() value=summary.vendor_count.to_string() />
            </div>

            <div class="mb-8 rounded border-2 border-gray-400 bg-gray-50 p-6">
                <div class="flex items-center justify-between gap-4">
                    <span class="font-bold">{t!("report.total_booth_revenue")()}</span>
                    <span class="text-2xl font-bold">{format_currency(summary.total_booth_revenue, locale.get())}</span>
                </div>
            </div>

            <table class="w-full border-collapse">
                <thead>
                    <tr class="border-b-2 border-gray-800">
                        <th class="px-4 py-3 text-left font-bold">{t!("report.vendor_id")()}</th>
                        <th class="px-4 py-3 text-right font-bold">{t!("report.gross_sales")()}</th>
                        <th class="px-4 py-3 text-right font-bold">{t!("report.fees_due")()}</th>
                        <th class="px-4 py-3 text-right font-bold">{t!("report.net_payout")()}</th>
                        <th class="px-4 py-3 text-right font-bold">{t!("report.item_count")()}</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || vendor_summaries.clone()
                        key=|vendor| vendor.vendor_id.as_str().to_string()
                        children=move |vendor| view! {
                            <tr class="border-b border-gray-300">
                                <td class="px-4 py-3 font-medium">{vendor.vendor_id.to_string()}</td>
                                <td class="px-4 py-3 text-right">{format_currency(vendor.gross_sales, locale.get())}</td>
                                <td class="px-4 py-3 text-right">{format_currency(vendor.fees_due, locale.get())}</td>
                                <td class="px-4 py-3 text-right font-semibold">{format_currency(vendor.net_payout, locale.get())}</td>
                                <td class="px-4 py-3 text-right">{vendor.item_count}</td>
                            </tr>
                        }
                    />
                </tbody>
            </table>
        </div>
    }
    .into_view()
}

fn vendor_row(vendor: ArchivedVendorSummary, locale: RwSignal<crate::i18n::Locale>) -> View {
    let vendor_id = vendor.vendor_id.to_string();
    let gross_sales = vendor.gross_sales;
    let fees_due = vendor.fees_due;
    let net_payout = vendor.net_payout;
    let item_count = vendor.item_count;

    view! {
        <tr class="hover:bg-gray-50">
            <td class="px-4 py-3 text-sm font-medium text-gray-900">{vendor_id}</td>
            <td class="px-4 py-3 text-right text-sm text-gray-700">{move || format_currency(gross_sales, locale.get())}</td>
            <td class="px-4 py-3 text-right text-sm text-gray-700">{move || format_currency(fees_due, locale.get())}</td>
            <td class="px-4 py-3 text-right text-sm font-semibold text-gray-900">{move || format_currency(net_payout, locale.get())}</td>
            <td class="px-4 py-3 text-right text-sm text-gray-700">{item_count}</td>
        </tr>
    }
    .into_view()
}

#[component]
fn SummaryMetricCard(label: String, value: String) -> impl IntoView {
    view! {
        <div class="rounded-lg bg-white p-4 shadow-sm ring-1 ring-gray-200">
            <p class="text-sm text-gray-600">{label}</p>
            <p class="text-2xl font-bold text-slate-900">{value}</p>
        </div>
    }
}

#[component]
fn PrintMetricCard(label: String, value: String) -> impl IntoView {
    view! {
        <div class="rounded border-2 border-gray-300 p-4">
            <p class="mb-1 text-sm text-gray-600">{label}</p>
            <p class="text-3xl font-bold">{value}</p>
        </div>
    }
}

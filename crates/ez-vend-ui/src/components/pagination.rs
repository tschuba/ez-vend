#![allow(clippy::manual_div_ceil)]

use crate::components::pagination_prefs::PAGE_SIZE_OPTIONS;
use crate::components::{Icon, LuChevronLeft, LuChevronRight, LuChevronsLeft, LuChevronsRight};
use crate::t;
use leptos::*;

/// Compute total number of pages
pub fn compute_total_pages(total_items: usize, page_size: usize) -> usize {
    if page_size == 0 {
        return 0;
    }
    (total_items + page_size - 1) / page_size
}

/// Clamp page to valid range
#[allow(dead_code)]
pub fn clamp_page(page: usize, total_pages: usize) -> usize {
    if total_pages == 0 {
        0
    } else {
        page.min(total_pages.saturating_sub(1))
    }
}

#[component]
pub fn Pagination(
    /// Current page (0-indexed)
    current_page: ReadSignal<usize>,
    /// Total number of items (can be a signal via a closure)
    #[prop(into)]
    total_items: MaybeSignal<usize>,
    /// Current page size
    page_size: ReadSignal<usize>,
    /// Callback when page changes
    #[prop(into)]
    on_page_change: Callback<usize>,
    /// Callback when page size changes
    #[prop(into)]
    on_page_size_change: Callback<usize>,
    /// Translation key prefix (e.g., "checkout" or "vendor")
    translation_prefix: &'static str,
    /// Whether to show the page size selector
    #[prop(default = true)]
    show_page_size_selector: bool,
) -> impl IntoView {
    let total_pages = create_memo(move |_| compute_total_pages(total_items.get(), page_size.get()));

    view! {
        <div class="flex items-center justify-end gap-4 py-3">
            {/* Page size selector (if enabled) */}
            <Show when=move || show_page_size_selector>
                <div class="flex items-center gap-2 text-sm text-gray-700">
                    <label for="page-size-select" class="text-gray-600">
                        {move || t!(format!("{}.per_page_label", translation_prefix).as_str())()}
                    </label>
                    <select
                        id="page-size-select"
                        class="px-2 py-1 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            if let Ok(size) = value.parse::<usize>() {
                                on_page_size_change.call(size);
                            }
                        }
                    >
                        {move || {
                            let current_size = page_size.get();
                            PAGE_SIZE_OPTIONS.iter().map(|&size| {
                                view! {
                                    <option value={size.to_string()} selected={size == current_size}>
                                        {size.to_string()}
                                    </option>
                                }
                            }).collect_view()
                        }}
                    </select>
                </div>
            </Show>

            {/* Page info */}
            <div class="flex items-center gap-2 text-sm text-gray-700">
                <span>{move || {
                    let current = current_page.get() + 1;
                    let total = total_pages.get();
                    let count = total_items.get();
                    let key = format!("{}.page_info", translation_prefix);
                    t!(key.as_str())()
                        .replace("{current}", &current.to_string())
                        .replace("{total}", &total.to_string())
                        .replace("{count}", &count.to_string())
                }}</span>
            </div>

            {/* Navigation controls with icons */}
            <div class="flex items-center gap-1">
                {/* First page - |< icon */}
                <button
                    on:click=move |_| on_page_change.call(0)
                    disabled=move || current_page.get() == 0
                    class="p-2 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    aria-label={move || t!(format!("{}.first", translation_prefix).as_str())()}
                    title={move || t!(format!("{}.first", translation_prefix).as_str())()}
                >
                    <Icon icon=LuChevronsLeft class="w-4 h-4" />
                </button>

                {/* Previous page - < icon */}
                <button
                    on:click=move |_| {
                        let current = current_page.get();
                        if current > 0 {
                            on_page_change.call(current - 1);
                        }
                    }
                    disabled=move || current_page.get() == 0
                    class="p-2 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    aria-label={move || t!(format!("{}.previous", translation_prefix).as_str())()}
                    title={move || t!(format!("{}.previous", translation_prefix).as_str())()}
                >
                    <Icon icon=LuChevronLeft class="w-4 h-4" />
                </button>

                {/* Next page - > icon */}
                <button
                    on:click=move |_| {
                        let current = current_page.get();
                        let total = total_pages.get();
                        if current + 1 < total {
                            on_page_change.call(current + 1);
                        }
                    }
                    disabled={move || {
                        let total = total_pages.get();
                        total == 0 || current_page.get() >= total - 1
                    }}
                    class="p-2 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    aria-label={move || t!(format!("{}.next", translation_prefix).as_str())()}
                    title={move || t!(format!("{}.next", translation_prefix).as_str())()}
                >
                    <Icon icon=LuChevronRight class="w-4 h-4" />
                </button>

                {/* Last page - >| icon */}
                <button
                    on:click=move |_| {
                        let total = total_pages.get();
                        if total > 0 {
                            on_page_change.call(total - 1);
                        }
                    }
                    disabled={move || {
                        let total = total_pages.get();
                        total == 0 || current_page.get() >= total - 1
                    }}
                    class="p-2 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    aria-label={move || t!(format!("{}.last", translation_prefix).as_str())()}
                    title={move || t!(format!("{}.last", translation_prefix).as_str())()}
                >
                    <Icon icon=LuChevronsRight class="w-4 h-4" />
                </button>
            </div>
        </div>
    }
}

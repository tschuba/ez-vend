#![allow(clippy::clone_on_copy)]
#![allow(clippy::needless_borrows_for_generic_args)]

use crate::components::pagination::Pagination;
use crate::components::pagination_prefs::use_pagination_preference;
use crate::components::*;
use crate::formatting::{format_currency, format_decimal_for_input, parse_decimal_input};
use crate::i18n::{translate_with_params, use_locale};
use crate::selected_booth_context;
use crate::state::*;
use crate::t;
use domain::models::{PurchaseId, Vendor, VendorId};
use domain::services::VendorReportData;
use leptos::*;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[component]
pub fn VendorListPage() -> impl IntoView {
    let app_state = use_app_state();
    let (vendor_reports, set_vendor_reports) = create_signal(Vec::<VendorReportData>::new());
    let (vendors_without_purchases, set_vendors_without_purchases) =
        create_signal(Vec::<Vendor>::new());

    // Use global selected booth context
    let selected_booth = selected_booth_context::use_selected_booth();
    let booth_list_version = selected_booth_context::use_booth_list_version();
    let (is_loading, set_is_loading) = create_signal(true);

    // Accordion state - only one vendor can be expanded at a time
    let (expanded_vendor_id, set_expanded_vendor_id) = create_signal(None::<VendorId>);

    // Selection state
    let (selected_vendor_ids, set_selected_vendor_ids) = create_signal(Vec::<VendorId>::new());
    let (reports_for_print, set_reports_for_print) = create_signal(Vec::<VendorReportData>::new());

    // Accessibility announcements
    let (aria_announcement, set_aria_announcement) = create_signal(String::new());

    // Vendor deletion state
    let vendor_delete = use_two_step_delete::<VendorId>();
    let vendor_delete_signal = vendor_delete.signal();
    let (pending_vendor_deletion, set_pending_vendor_deletion) =
        create_signal::<Option<Vendor>>(None);
    let (show_delete_modal, set_show_delete_modal) = create_signal(false);
    let (pending_vendor_correction, set_pending_vendor_correction) =
        create_signal::<Option<VendorReportData>>(None);
    let (show_correction_modal, set_show_correction_modal) = create_signal(false);
    let (reload_vendors_toggle, set_reload_vendors_toggle) = create_signal(false);

    // Pagination state with readiness flag
    let (page_size, set_page_size, page_size_ready) =
        use_pagination_preference("vendor_page_size", 10);
    let (current_page, set_current_page) = create_signal(0);
    let (filter_non_positive, set_filter_non_positive) = create_signal(false);
    let (filter_corrected, set_filter_corrected) = create_signal(false);
    let (vendor_search_query, set_vendor_search_query) = create_signal(String::new());

    let filtered_vendor_reports: Memo<Vec<VendorReportData>> = create_memo(move |_| {
        let reports = vendor_reports.get();
        let non_positive = filter_non_positive.get();
        let corrected = filter_corrected.get();
        let search_query = vendor_search_query.get().trim().to_lowercase();

        reports
            .into_iter()
            .filter(|report| {
                let matches_non_positive = !non_positive || report.total_revenue <= Decimal::ZERO;
                let matches_corrected = !corrected
                    || report.payout_correction != Decimal::ZERO
                    || report
                        .payout_correction_note
                        .as_ref()
                        .is_some_and(|note| !note.trim().is_empty());
                let matches_search = search_query.is_empty()
                    || report
                        .vendor
                        .vendor_id
                        .as_str()
                        .to_lowercase()
                        .contains(&search_query);

                matches_non_positive && matches_corrected && matches_search
            })
            .collect()
    });

    let filtered_vendors_without_purchases = create_memo(move |_| {
        let search_query = vendor_search_query.get().trim().to_lowercase();

        vendors_without_purchases
            .get()
            .into_iter()
            .filter(|vendor| {
                search_query.is_empty()
                    || vendor
                        .vendor_id
                        .as_str()
                        .to_lowercase()
                        .contains(&search_query)
            })
            .collect::<Vec<_>>()
    });

    let non_positive_vendor_count = create_memo(move |_| {
        vendor_reports
            .get()
            .iter()
            .filter(|report| report.total_revenue <= Decimal::ZERO)
            .count()
    });

    let corrected_vendor_count = create_memo(move |_| {
        vendor_reports
            .get()
            .iter()
            .filter(|report| {
                report.payout_correction != Decimal::ZERO
                    || report
                        .payout_correction_note
                        .as_ref()
                        .is_some_and(|note| !note.trim().is_empty())
            })
            .count()
    });

    // Create paginated vendor reports - only slice when preference is ready
    let paginated_vendor_reports = create_memo(move |_| {
        // Wait for page size preference to be ready
        if !page_size_ready.get() {
            return Vec::new();
        }

        let reports: Vec<VendorReportData> = filtered_vendor_reports.get();
        let size = page_size.get();
        let page = current_page.get();
        let start = page * size;
        let end = (start + size).min(reports.len());

        if start >= reports.len() {
            Vec::new()
        } else {
            reports[start..end].to_vec()
        }
    });

    let toast = use_toast();

    create_effect(move |_| {
        if let Some(booth) = selected_booth.get() {
            if booth.is_archived() {
                toast.error(&t!("archive.cannot_select")());
                selected_booth.set(None);
                booth_list_version.update(|version| *version += 1);
            }
        }
    });

    let vendor_message = |key: &'static str, vendor_id: &VendorId| {
        translate_with_params(key, HashMap::from([("id", vendor_id.as_str().to_string())]))
    };

    // Reset to first page when vendor_reports or page_size changes
    create_effect(move |_| {
        let _ = vendor_reports.get();
        let _ = page_size.get();
        let _ = filter_non_positive.get();
        let _ = filter_corrected.get();
        let _ = vendor_search_query.get();
        set_current_page.set(0);
    });

    // Load vendors when booth is selected or reload toggle changes
    create_effect(move |_| {
        let state_result = app_state.get();
        let booth = selected_booth.get();
        let _ = reload_vendors_toggle.get(); // Track reload toggle

        if booth.is_none() {
            // No booth selected, clear vendors and selection state
            set_vendor_reports.set(Vec::new());
            set_vendors_without_purchases.set(Vec::new());
            set_selected_vendor_ids.set(Vec::new());
            set_expanded_vendor_id.set(None);
            set_is_loading.set(false);
        } else if let (Some(Ok(state)), Some(booth)) = (state_result, booth) {
            // Clear selection when changing booths
            set_selected_vendor_ids.set(Vec::new());
            set_expanded_vendor_id.set(None);

            set_is_loading.set(true);
            let booth_id = booth.id.clone();
            spawn_local(async move {
                // Get all registered vendors for the booth
                let all_vendors_result = state.vendor_repository.find_by_booth(&booth_id).await;

                // Get active vendors (those with purchases)
                let active_vendors_result = state
                    .report_service
                    .get_active_vendors(&booth_id, None)
                    .await;

                match (all_vendors_result, active_vendors_result) {
                    (Ok(mut all_vendors), Ok(active_vendor_ids)) => {
                        // Sort all vendors
                        all_vendors.sort_by(|a, b| a.vendor_id.cmp(&b.vendor_id));

                        // Generate reports for active vendors
                        if !active_vendor_ids.is_empty() {
                            match state
                                .report_service
                                .generate_vendor_reports(&booth_id, active_vendor_ids.clone(), None)
                                .await
                            {
                                Ok(reports) => {
                                    set_vendor_reports.set(reports);

                                    // Find vendors without purchases
                                    let vendors_without: Vec<Vendor> = all_vendors
                                        .into_iter()
                                        .filter(|v| !active_vendor_ids.contains(&v.vendor_id))
                                        .collect();
                                    set_vendors_without_purchases.set(vendors_without);
                                }
                                Err(_) => {
                                    toast.error(&t!("vendor.errors.load_reports_failed")());
                                }
                            }
                        } else {
                            // No active vendors, all vendors have no purchases
                            set_vendor_reports.set(Vec::new());
                            set_vendors_without_purchases.set(all_vendors);
                        }
                    }
                    (Err(_), _) => {
                        toast.error(&t!("vendor.errors.load_vendors_failed")());
                    }
                    (_, Err(_)) => {
                        toast.error(&t!("vendor.errors.load_active_vendors_failed")());
                    }
                }

                set_is_loading.set(false);
            });
        }
    });

    // Toggle vendor selection
    let toggle_vendor_selection = move |vendor_id: VendorId| {
        set_selected_vendor_ids.update(|ids| {
            if let Some(pos) = ids.iter().position(|v| v == &vendor_id) {
                ids.remove(pos);
            } else {
                ids.push(vendor_id);
            }
            ids.sort();
        });
    };

    // Generate reports for all vendors
    let generate_all_reports = move |_| {
        let state_result = app_state.get();
        let booth = selected_booth.get();

        if let (Some(Ok(state)), Some(booth)) = (state_result, booth) {
            set_is_loading.set(true);
            let booth_id = booth.id.clone();

            spawn_local(async move {
                match state
                    .report_service
                    .get_active_vendors(&booth_id, None)
                    .await
                {
                    Ok(vendor_ids) => {
                        if vendor_ids.is_empty() {
                            toast.error(&t!("vendor.no_vendors_with_purchases")());
                            set_is_loading.set(false);
                        } else {
                            match state
                                .report_service
                                .generate_vendor_reports(&booth_id, vendor_ids, None)
                                .await
                            {
                                Ok(reports) => {
                                    set_reports_for_print.set(reports);
                                    // Trigger print after a short delay to allow DOM update
                                    set_timeout(
                                        move || {
                                            if let Some(window) = web_sys::window() {
                                                let _ = window.print();
                                            }
                                        },
                                        std::time::Duration::from_millis(100),
                                    );
                                }
                                Err(_) => {
                                    toast.error(&t!("vendor.errors.generate_reports_failed")());
                                }
                            }
                            set_is_loading.set(false);
                        }
                    }
                    Err(_) => {
                        toast.error(&t!("vendor.errors.load_active_vendors_failed")());
                        set_is_loading.set(false);
                    }
                }
            });
        }
    };

    // Generate reports for selected vendors
    let generate_selected_reports = move |_| {
        let selected = selected_vendor_ids.get();
        if selected.is_empty() {
            toast.error(&t!("vendor.no_vendors_selected")());
            return;
        }

        let state_result = app_state.get();
        let booth = selected_booth.get();

        if let (Some(Ok(state)), Some(booth)) = (state_result, booth) {
            set_is_loading.set(true);
            let booth_id = booth.id.clone();

            spawn_local(async move {
                match state
                    .report_service
                    .generate_vendor_reports(&booth_id, selected, None)
                    .await
                {
                    Ok(reports) => {
                        set_reports_for_print.set(reports);
                        // Trigger print after a short delay to allow DOM update
                        set_timeout(
                            move || {
                                if let Some(window) = web_sys::window() {
                                    let _ = window.print();
                                }
                            },
                            std::time::Duration::from_millis(100),
                        );
                    }
                    Err(_) => {
                        toast.error(&t!("vendor.errors.generate_reports_failed")());
                    }
                }
                set_is_loading.set(false);
            });
        }
    };

    // Vendor deletion handlers
    // Handle clicking on a vendor card or its overlay
    // First click: arm the vendor for deletion (show red overlay)
    // Second click (on overlay): open confirmation modal
    let handle_vendor_delete_click = move |vendor_id: VendorId| {
        if vendor_delete_signal.get() == Some(vendor_id.clone()) {
            log::info!(
                "Opening vendor deletion modal for vendor_id: {:?}",
                vendor_id
            );
            let vendors = vendors_without_purchases.get();
            if let Some(vendor) = vendors.iter().find(|v| v.vendor_id == vendor_id) {
                set_pending_vendor_deletion.set(Some(vendor.clone()));
                set_show_delete_modal.set(true);
            } else {
                log::warn!(
                    "Could not find vendor with id {:?} in vendors_without_purchases",
                    vendor_id
                );
            }
        } else {
            log::info!("Arming vendor for deletion: {:?}", vendor_id);
            vendor_delete_signal.set(Some(vendor_id));
        }
    };

    // Perform actual vendor deletion
    let perform_vendor_delete = {
        let app_state = app_state.clone();
        let selected_booth = selected_booth.clone();
        let set_reload_vendors_toggle = set_reload_vendors_toggle.clone();
        let set_pending_vendor_deletion = set_pending_vendor_deletion.clone();
        let set_show_delete_modal = set_show_delete_modal.clone();
        let toast = toast.clone();
        move || {
            let pending = pending_vendor_deletion.get();
            if pending.is_none() {
                log::warn!("perform_vendor_delete called but no vendor in pending_vendor_deletion");
                return;
            }

            let Some(vendor) = pending else {
                log::warn!("perform_vendor_delete called without pending vendor");
                toast.error(&t!("vendor.errors.invalid_delete_state")());
                return;
            };
            let vendor_id = vendor.vendor_id.clone();
            let booth_id_opt = selected_booth.get().map(|b| b.id.clone());

            if booth_id_opt.is_none() {
                log::warn!("No booth selected, cannot delete vendor");
                toast.error(&t!("vendor.errors.no_booth_selected")());
                return;
            }

            let Some(booth_id) = booth_id_opt else {
                toast.error(&t!("vendor.errors.no_booth_selected")());
                return;
            };
            log::info!(
                "perform_vendor_delete: deleting vendor_id: {:?} from booth: {:?}",
                vendor_id,
                booth_id
            );
            let state_result = app_state.get();

            if let Some(Ok(state)) = state_result {
                spawn_local(async move {
                    log::info!("Calling vendor_repository.delete_from_booth for vendor_id: {:?}, booth_id: {:?}", vendor_id, booth_id);
                    match state
                        .vendor_repository
                        .delete_from_booth(&booth_id, &vendor_id)
                        .await
                    {
                        Ok(_) => {
                            log::info!("Successfully deleted vendor_id: {:?}", vendor_id);
                            set_pending_vendor_deletion.set(None);
                            vendor_delete_signal.set(None);
                            set_show_delete_modal.set(false);
                            set_reload_vendors_toggle.update(|v| *v = !*v);
                            log::info!("Toggled reload signal to refresh vendor list");
                            toast.success(&translate_with_params(
                                "vendor.delete.success",
                                HashMap::from([("vendor_id", vendor_id.as_str().to_string())]),
                            ));
                            toast.info(&t!("vendor.delete.aftercare")());
                        }
                        Err(e) => {
                            log::error!("Failed to delete vendor_id {:?}: {:?}", vendor_id, e);
                            set_pending_vendor_deletion.set(None);
                            vendor_delete_signal.set(None);
                            set_show_delete_modal.set(false);
                            toast.error(&translate_with_params(
                                "vendor.delete.error",
                                HashMap::from([("error", format_error_message(&e))]),
                            ));
                        }
                    }
                });
            } else {
                log::warn!("App state not available for deletion");
                // Reset state if app state not available
                set_pending_vendor_deletion.set(None);
                vendor_delete_signal.set(None);
                set_show_delete_modal.set(false);
            }
        }
    };

    // Cancel vendor deletion
    let cancel_vendor_delete = {
        let set_pending_vendor_deletion = set_pending_vendor_deletion.clone();
        let set_show_delete_modal = set_show_delete_modal.clone();
        move || {
            log::info!("Canceling vendor deletion");
            set_pending_vendor_deletion.set(None);
            vendor_delete_signal.set(None);
            set_show_delete_modal.set(false);
        }
    };

    let save_vendor_correction = {
        let app_state = app_state.clone();
        let set_reload_vendors_toggle = set_reload_vendors_toggle.clone();
        let toast = toast.clone();
        let set_show_correction_modal = set_show_correction_modal.clone();
        move |(mut vendor, correction, note): (Vendor, Decimal, Option<String>)| {
            let state_result = app_state.get();
            if let Some(Ok(state)) = state_result {
                vendor.payout_correction = if correction == Decimal::ZERO {
                    None
                } else {
                    Some(correction)
                };
                vendor.payout_correction_note = note;
                spawn_local(async move {
                    match state.vendor_repository.save(&vendor).await {
                        Ok(_) => {
                            set_reload_vendors_toggle.update(|v| *v = !*v);
                            set_show_correction_modal.set(false);
                            toast.success(&t!("vendor.correction_saved")());
                        }
                        Err(_) => {
                            toast.error(&t!("vendor.errors.save_correction_failed")());
                        }
                    }
                });
            } else {
                toast.error(&t!("vendor.errors.save_correction_failed")());
            }
        }
    };

    view! {
        <>
            // Aria-live region for accessibility announcements
            <div
                role="status"
                aria-live="polite"
                aria-atomic="true"
                class="sr-only"
            >
                {move || aria_announcement.get()}
            </div>

            // Screen-only UI (hidden during print)
            <div class="print:hidden" on:click=move |_| cancel_vendor_delete()>
                <Container class="mt-6">
                     <div>
                         <Card>
                            <Show
                                when=move || !is_loading.get()
                                fallback=move || view! { <p class="text-gray-600">{t!("common.loading")}</p> }
                            >
                                <Show
                                    when=move || selected_booth.get().is_some()
                                    fallback=move || view! { <p class="text-gray-500 text-center py-8">{t!("vendor.select_booth_prompt")}</p> }
                                >
                                    // Helper text section
                                    <div class="mb-2">
                                        <Show when=move || !vendor_reports.get().is_empty() || !vendors_without_purchases.get().is_empty()>
                                            <div class="space-y-3">
                                                <div class="max-w-md">
                                                    <input
                                                        type="search"
                                                        class="w-full rounded-lg border border-gray-300 px-4 py-2 text-sm text-gray-900 shadow-sm focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                                        placeholder=t!("common.search_placeholder")()
                                                        aria-label=t!("common.search_placeholder")()
                                                        prop:value=move || vendor_search_query.get()
                                                        on:input=move |ev| {
                                                            set_vendor_search_query.set(event_target_value(&ev));
                                                        }
                                                    />
                                                </div>

                                                <Show when=move || !vendor_reports.get().is_empty()>
                                                    <p class="text-sm text-gray-600">
                                                        {move || {
                                                            let total_count = vendor_reports.get().len();
                                                            let shown_count = filtered_vendor_reports.get().len();
                                                            let selected_count = selected_vendor_ids.get().len();
                                                            let non_positive = filter_non_positive.get();
                                                            let corrected = filter_corrected.get();
                                                            let filters_active = non_positive || corrected;

                                                            if filters_active {
                                                                let showing_text = translate_with_params(
                                                                    "vendor.filter_showing",
                                                                    HashMap::from([
                                                                        ("shown", shown_count.to_string()),
                                                                        ("total", total_count.to_string()),
                                                                    ]),
                                                                );

                                                                if selected_count > 0 {
                                                                    format!(
                                                                        "{} · {} {} {} {}",
                                                                        showing_text,
                                                                        selected_count,
                                                                        t!("vendor.vendors_selected_of")(),
                                                                        shown_count,
                                                                        t!("vendor.vendors")()
                                                                    )
                                                                } else {
                                                                    format!(
                                                                        "{} {}",
                                                                        showing_text,
                                                                        t!("vendor.click_vendors_hint")()
                                                                    )
                                                                }
                                                            } else if selected_count > 0 {
                                                                format!(
                                                                    "{} {} {} {}",
                                                                    selected_count,
                                                                    t!("vendor.vendors_selected_of")(),
                                                                    total_count,
                                                                    t!("vendor.vendors_with_purchases")()
                                                                )
                                                            } else {
                                                                format!(
                                                                    "{} {} {}",
                                                                    total_count,
                                                                    t!("vendor.vendors_with_purchases")(),
                                                                    t!("vendor.click_vendors_hint")()
                                                                )
                                                            }
                                                        }}
                                                    </p>

                                                    <div class="flex flex-wrap items-center gap-2">
                                                        <span class="text-sm font-medium text-gray-700">
                                                            {t!("vendor.filter_label")()}
                                                        </span>
                                                        <button
                                                            class=move || {
                                                                if filter_non_positive.get() {
                                                                    "rounded-full border border-amber-400 bg-amber-100 px-3 py-1 text-sm font-medium text-amber-900"
                                                                } else {
                                                                    "rounded-full border border-gray-300 bg-white px-3 py-1 text-sm font-medium text-gray-700 hover:bg-gray-50"
                                                                }
                                                            }
                                                            on:click=move |_| {
                                                                set_filter_non_positive.update(|v| *v = !*v);
                                                            }
                                                        >
                                                            {move || format!(
                                                                "{} ({})",
                                                                t!("vendor.filter_non_positive")(),
                                                                non_positive_vendor_count.get()
                                                            )}
                                                        </button>
                                                        <button
                                                            class=move || {
                                                                if filter_corrected.get() {
                                                                    "rounded-full border border-blue-400 bg-blue-100 px-3 py-1 text-sm font-medium text-blue-900"
                                                                } else {
                                                                    "rounded-full border border-gray-300 bg-white px-3 py-1 text-sm font-medium text-gray-700 hover:bg-gray-50"
                                                                }
                                                            }
                                                            on:click=move |_| {
                                                                set_filter_corrected.update(|v| *v = !*v);
                                                            }
                                                        >
                                                            {move || format!(
                                                                "{} ({})",
                                                                t!("vendor.filter_corrected")(),
                                                                corrected_vendor_count.get()
                                                            )}
                                                        </button>
                                                    </div>
                                                </Show>
                                            </div>
                                        </Show>
                                    </div>

                                    <div class="mx-auto max-w-5xl">
                                        // Top pagination controls
                                        <Show when=move || !filtered_vendor_reports.get().is_empty()>
                                            <div class="mb-4">
                                                <Pagination
                                                    current_page=current_page
                                                    total_items=Signal::derive(move || filtered_vendor_reports.get().len())
                                                    page_size=page_size
                                                    on_page_change=move |page| set_current_page.set(page)
                                                    on_page_size_change=move |size| set_page_size.set(size)
                                                    translation_prefix="vendor.pagination"
                                                    show_page_size_selector=true
                                                />
                                            </div>
                                        </Show>

                                        // Vendor list
                                        <Show
                                            when=move || !filtered_vendor_reports.get().is_empty() || !filtered_vendors_without_purchases.get().is_empty()
                                             fallback=move || view! {
                                                 <div class="flex flex-col items-center justify-center py-10 text-center">
                                                     <Icon icon=LuUsers class="mb-3 h-12 w-12 text-gray-300" />
                                                     <p class="text-sm font-medium text-gray-700">
                                                         {move || {
                                                             if vendor_search_query.get().trim().is_empty() {
                                                                 t!("vendor.no_vendors")()
                                                             } else {
                                                                 t!("common.no_results")()
                                                             }
                                                         }}
                                                     </p>
                                                     <Show when=move || vendor_search_query.get().trim().is_empty()>
                                                         <p class="mt-1 text-xs text-gray-500">{t!("vendor.empty_state_hint")}</p>
                                                     </Show>
                                                 </div>
                                             }
                                        >
                                            <div class="space-y-4">
                                            {/* Vendors with purchases */}
                                            {move || paginated_vendor_reports.get().into_iter().map(|report: VendorReportData| {
                                                // Clone all needed data upfront
                                                let report_data = report.clone();
                                                let report_data_for_correction = report_data.clone();
                                                let report_data_for_print = report_data.clone();
                                                let vendor_id = report.vendor.vendor_id.clone();
                                                let vendor_id_for_class1 = vendor_id.clone();
                                                let vendor_id_for_expanded = vendor_id.clone();
                                                let vendor_id_str = vendor_id.as_str().to_string();
                                                let locale = use_locale();

                                                // Create stores for detail data
                                                let (detail_report, _) = create_signal(report_data.clone());

                                                let is_expanded = create_memo(move |_| {
                                                    expanded_vendor_id.get() == Some(vendor_id_for_expanded.clone())
                                                });

                                                let is_selected = create_memo(move |_| {
                                                    selected_vendor_ids.get().contains(&vendor_id_for_class1)
                                                });

                                                // IDs for keyboard support closure captures
                                                let vendor_id_for_select = vendor_id.clone();
                                                let vendor_id_for_select_key = vendor_id.clone();
                                                let vendor_id_for_keydown = vendor_id.clone();
                                                let vendor_id_for_details_btn = vendor_id.clone();

                                                view! {
                                                    <div
                                                        class=move || {
                                                            let base = "group relative rounded-lg transition-all duration-200 overflow-hidden";
                                                            let border_color = if is_selected.get() {
                                                                "border-2 border-blue-500 shadow-md"
                                                            } else {
                                                                "border border-gray-200 hover:border-gray-300 hover:shadow-md"
                                                            };
                                                            let bg = if is_expanded.get() {
                                                                "bg-blue-50"
                                                            } else {
                                                                "bg-white"
                                                            };
                                                            format!("{} {} {}", base, border_color, bg)
                                                        }
                                                    >
                                                        {/* Left accent bar */}
                                                        <div
                                                            class=move || {
                                                                let color = if is_selected.get() {
                                                                    "bg-blue-500"
                                                                } else {
                                                                    "bg-gray-200"
                                                                };
                                                                format!("absolute left-0 top-0 bottom-0 w-1 transition-colors duration-200 {}", color)
                                                            }
                                                        />

                                                        {/* Summary section - clickable for selection */}
                                                        <div
                                                            class="pl-4 pr-4 py-4 cursor-pointer"
                                                            tabindex="0"
                                                            role="button"
                                                            aria-pressed=move || if is_selected.get() { "true" } else { "false" }
                                                            on:click=move |_| {
                                                                toggle_vendor_selection(vendor_id_for_select.clone());
                                                                let msg = if selected_vendor_ids.get().contains(&vendor_id_for_select) {
                                                                    vendor_message("vendor.vendor_selected", &vendor_id_for_select)
                                                                } else {
                                                                    vendor_message("vendor.vendor_deselected", &vendor_id_for_select)
                                                                };
                                                                set_aria_announcement.set(msg);
                                                            }
                                                            on:keydown=move |e| {
                                                                match e.key().as_str() {
                                                                    " " | "Enter" => {
                                                                        e.prevent_default();
                                                                        toggle_vendor_selection(vendor_id_for_select_key.clone());
                                                                        let msg = if selected_vendor_ids.get().contains(&vendor_id_for_select_key) {
                                                                            vendor_message("vendor.vendor_selected", &vendor_id_for_select_key)
                                                                        } else {
                                                                            vendor_message("vendor.vendor_deselected", &vendor_id_for_select_key)
                                                                        };
                                                                        set_aria_announcement.set(msg);
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                        >
                                                            {/* Content wrapper - Identity left, Print+Payout grouped right */}
                                                            <div class="flex items-center pr-16">
                                                                {/* Zone 1: Identity (Vendor ID + item count) - grows to fill space */}
                                                                <div class="flex flex-col gap-1 flex-1">
                                                                    <div class="flex items-baseline gap-2">
                                                                        <span class="text-xs text-gray-500 uppercase tracking-wide">{t!("vendor.id_label")()}</span>
                                                                        <span class="text-lg font-bold text-gray-900">{vendor_id_str.clone()}</span>
                                                                    </div>
                                                                    <div class="flex flex-wrap gap-2 mt-1">
                                                                        <Show when=move || report.total_revenue <= Decimal::ZERO>
                                                                            <span class="rounded-full bg-amber-100 text-amber-800 px-2 py-0.5 text-xs font-semibold">
                                                                                {t!("vendor.warning_payout_non_positive")()}
                                                                            </span>
                                                                        </Show>
                                                                        <Show when=move || {
                                                                            report.payout_correction != Decimal::ZERO
                                                                                || report
                                                                                    .payout_correction_note
                                                                                    .as_ref()
                                                                                    .is_some_and(|note| !note.trim().is_empty())
                                                                        }>
                                                                            <span class="rounded-full bg-blue-100 text-blue-800 px-2 py-0.5 text-xs font-semibold">
                                                                                {t!("vendor.corrected_badge")()}
                                                                            </span>
                                                                        </Show>
                                                                    </div>
                                                                    <div class="text-sm text-gray-600">
                                                                        <span class="font-medium">{report.items.len()}</span>
                                                                        <span class="ml-1">{t!("checkout.running_totals.items")()}</span>
                                                                    </div>
                                                                </div>

                                                                {/* Zone 2+3: Print button + Payout metrics grouped together */}
                                                                    <div class="flex items-center gap-3">
                                                                        <button
                                                                            on:click=move |e| {
                                                                                e.stop_propagation();
                                                                                set_pending_vendor_correction.set(Some(report_data_for_correction.clone()));
                                                                                set_show_correction_modal.set(true);
                                                                            }
                                                                            title={t!("vendor.open_correction_editor")()}
                                                                            aria-label={t!("vendor.open_correction_editor")()}
                                                                            class=move || format!(
                                                                                "px-3 py-2 rounded-md bg-indigo-600 text-white text-sm font-semibold transition-all hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 {}",
                                                                                if is_expanded.get() {
                                                                                    "opacity-100"
                                                                                } else {
                                                                                    "opacity-0 group-hover:opacity-100"
                                                                                }
                                                                            )
                                                                        >
                                                                            <span class="sr-only">{t!("vendor.correction_action")()}</span>
                                                                            <Icon icon=LuPenSquare class="w-5 h-5" />
                                                                        </button>
                                                                    {/* Print button - hover visible when collapsed, always visible when expanded */}
                                                                    <button
                                                                        on:click=move |e| {
                                                                            e.stop_propagation();
                                                                            set_reports_for_print.set(vec![report_data_for_print.clone()]);
                                                                            set_timeout(
                                                                                move || {
                                                                                    if let Some(window) = web_sys::window() {
                                                                                        let _ = window.print();
                                                                                    }
                                                                                },
                                                                                std::time::Duration::from_millis(100),
                                                                            );
                                                                        }
                                                                        title={t!("vendor.print_report")()}
                                                                        aria-label={t!("vendor.print_report")()}
                                                                        class=move || format!(
                                                                            "min-w-[4rem] min-h-[4rem] w-16 h-16 flex items-center justify-center rounded-full transition-all shadow-lg bg-blue-600 text-white hover:bg-blue-700 hover:scale-105 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 {}",
                                                                            if is_expanded.get() {
                                                                                "opacity-100"
                                                                            } else {
                                                                                "opacity-0 group-hover:opacity-100"
                                                                            }
                                                                        )
                                                                    >
                                                                        {/* Print icon - matches FAB */}
                                                                        <Icon icon=LuPrinter class="w-6 h-6" />
                                                                    </button>

                                                                    {/* Performance metrics - directly adjacent to print button */}
                                                                    <div class="flex flex-col items-end gap-1">
                                                                        <div class="text-right">
                                                                            <div class="text-xs text-gray-500 uppercase tracking-wide mb-0.5">
                                                                                {t!("vendor.net_payout")()}
                                                                            </div>
                                                                            <div class=move || {
                                                                                if report.total_revenue <= Decimal::ZERO {
                                                                                    "text-2xl font-bold text-red-700"
                                                                                } else {
                                                                                    "text-2xl font-bold text-green-700"
                                                                                }
                                                                            }>
                                                                                {format_currency(report.total_revenue, locale.get())}
                                                                            </div>
                                                                        </div>
                                                                        <div class="text-xs text-gray-600">
                                                                            <span>{t!("vendor.total_sales")()}</span>
                                                                            <span class="ml-1 font-semibold">{format_currency(report.sales_sum, locale.get())}</span>
                                                                        </div>
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        </div>

                                                        {/* Checkout-style expansion toggle on right edge */}
                                                        <div class="absolute right-0 top-0 h-full flex items-center">
                                                            {/* Vertical separator */}
                                                            <div class="h-full w-px bg-gray-300"></div>

                                                            {/* Expansion toggle button */}
                                                            <div
                                                                class="px-3 bg-white hover:bg-blue-50 transition-colors h-full flex items-center cursor-pointer"
                                                                on:click=move |e| {
                                                                    e.stop_propagation();
                                                                    set_expanded_vendor_id.update(|current| {
                                                                        let was_expanded = current.as_ref() == Some(&vendor_id_for_details_btn);
                                                                        *current = if was_expanded {
                                                                            None
                                                                        } else {
                                                                            Some(vendor_id_for_details_btn.clone())
                                                                        };
                                                                        let msg = if was_expanded {
                                                                            vendor_message(
                                                                                "vendor.vendor_details_collapsed",
                                                                                &vendor_id_for_details_btn,
                                                                            )
                                                                        } else {
                                                                            vendor_message(
                                                                                "vendor.vendor_details_expanded",
                                                                                &vendor_id_for_details_btn,
                                                                            )
                                                                        };
                                                                        set_aria_announcement.set(msg);
                                                                    });
                                                                }
                                                                on:keydown=move |e| {
                                                                    match e.key().as_str() {
                                                                        "ArrowRight" => {
                                                                            e.prevent_default();
                                                                            e.stop_propagation();
                                                                            set_expanded_vendor_id.set(Some(vendor_id_for_keydown.clone()));
                                                                            set_aria_announcement.set(vendor_message("vendor.vendor_details_expanded", &vendor_id_for_keydown));
                                                                        }
                                                                        "ArrowLeft" => {
                                                                            e.prevent_default();
                                                                            e.stop_propagation();
                                                                            set_expanded_vendor_id.set(None);
                                                                            set_aria_announcement.set(vendor_message("vendor.vendor_details_collapsed", &vendor_id_for_keydown));
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                }
                                                                tabindex="0"
                                                                role="button"
                                                                aria-expanded=move || if is_expanded.get() { "true" } else { "false" }
                                                                aria-controls=format!("vendor-details-{}", vendor_id_str.clone())
                                                                aria-label=move || if is_expanded.get() { t!("common.close")() } else { t!("vendor.view_details")() }
                                                            >
                                                                {/* Chevron icon */}
                                                                <Show
                                                                    when=move || is_expanded.get()
                                                                    fallback=move || {
                                                                        view! { <Icon icon=LuChevronDown class="w-5 h-5 text-gray-400 hover:text-blue-600 transition-all" /> }
                                                                    }
                                                                >
                                                                    <Icon icon=LuChevronDown class="w-5 h-5 rotate-180 text-gray-400 hover:text-blue-600 transition-all" />
                                                                </Show>
                                                            </div>
                                                        </div>

                                                        {/* Expanded detail drawer */}
                                                        <div
                                                            id=format!("vendor-details-{}", vendor_id_str.clone())
                                                            role="region"
                                                            aria-label=translate_with_params(
                                                                "vendor.vendor_details",
                                                                HashMap::from([("id", vendor_id_str.clone())]),
                                                            )
                                                            class=move || format!(
                                                                "overflow-hidden transition-all duration-200 {}",
                                                                if is_expanded.get() { "max-h-[1000px] opacity-100" } else { "max-h-0 opacity-0" }
                                                            )
                                                        >
                                                            <div class="border-t border-gray-200 p-4 pr-16 bg-gray-50">
                                                                {
                                                                    let report = detail_report.get();
                                                                    let locale = use_locale();

                                                                     view! {
                                                                        <div class="space-y-4">
                                                                            {/* Responsive grid layout with wider transaction list */}
                                                                            <div class="grid grid-cols-1 md:grid-cols-5 gap-4">
                                                                                {/* Left column: Fee breakdown with sum visualization */}
                                                                                <div class="space-y-3 md:col-span-2">
                                                                                    <h4 class="sr-only">{t!("vendor.financial_summary")()}</h4>
                                                                                    <div class="space-y-3">
                                                                                        {/* Individual fees side-by-side with plus indicator */}
                                                                                    <div class="flex items-center gap-2 flex-wrap sm:flex-nowrap">
                                                                                        <div class="bg-white p-3 rounded shadow-sm flex-1 min-w-[120px]">
                                                                                            <p class="text-xs text-gray-600">{t!("vendor.participation_fee")()}</p>
                                                                                            <p class="text-lg font-semibold">{format_currency(report.participation_fee, locale.get())}</p>
                                                                                        </div>

                                                                                        {/* Plus symbol */}
                                                                                        <div class="flex-shrink-0">
                                                                                            <div class="w-6 h-6 rounded-full bg-gray-200 flex items-center justify-center">
                                                                                                <span class="text-gray-600 text-sm font-bold">+</span>
                                                                                            </div>
                                                                                        </div>

                                                                                        <div class="bg-white p-3 rounded shadow-sm flex-1 min-w-[120px]">
                                                                                            <p class="text-xs text-gray-600">{t!("vendor.sales_fee")()}</p>
                                                                                            <p class="text-lg font-semibold">{format_currency(report.sales_fee, locale.get())}</p>
                                                                                        </div>
                                                                                    </div>

                                                                                    {/* Equals symbol */}
                                                                                    <div class="flex justify-center">
                                                                                        <div class="w-6 h-6 rounded-full bg-blue-100 flex items-center justify-center">
                                                                                            <span class="text-blue-600 text-sm font-bold">=</span>
                                                                                        </div>
                                                                                    </div>

                                                                                    {/* Cumulative total with emphasis */}
                                                                                    <div class="bg-gradient-to-br from-blue-50 to-blue-100 p-3 rounded shadow-md border-2 border-blue-500">
                                                                                        <p class="text-xs text-gray-700 font-semibold">{t!("vendor.fees_due")()}</p>
                                                                                        <p class="text-xl font-bold text-blue-700">{format_currency(report.participation_fee + report.sales_fee, locale.get())}</p>
                                                                                        <p class="text-xs text-gray-600 mt-1">{t!("vendor.fees_sum_explanation")()}</p>
                                                                                    </div>

                                                                                </div>
                                                                            </div>

                                                                                {/* Right column: Transactions */}
                                                                                <div class="space-y-3 md:col-span-3">
                                                                                    <h4 class="sr-only">{t!("vendor.transaction_id")()}</h4>
                                                                                    {/* Transactions list */}
                                                                                    <div class="bg-white p-3 rounded shadow-sm max-h-96 overflow-y-auto">
                                                                                        <div class="text-xs text-gray-600 space-y-2">
                                                                                            {
                                                                                            // Group items by transaction_id
                                                                                            let mut grouped: HashMap<PurchaseId, Vec<_>> = HashMap::new();
                                                                                            for item in report.items.iter() {
                                                                                                grouped.entry(item.transaction_id.clone()).or_default().push(item.clone());
                                                                                            }

                                                                                            let mut transactions: Vec<_> = grouped.into_iter().collect();
                                                                                            transactions.sort_by(|a, b| {
                                                                                                report.items.iter().position(|i| i.transaction_id == a.0)
                                                                                                    .cmp(&report.items.iter().position(|i| i.transaction_id == b.0))
                                                                                            });

                                                                                            transactions
                                                                                                .into_iter()
                                                                                                .map(|(transaction_id, transaction_items): (PurchaseId, Vec<domain::services::VendorReportItem>)| {
                                                                                                    let time_str = transaction_items[0].timestamp.format("%H:%M").to_string();
                                                                                                    let total: Decimal = transaction_items.iter().map(|i| i.item.amount).sum();
                                                                                                    let item_count = transaction_items.len();

                                                                                                    view! {
                                                                                                        <div class="border-l-2 border-blue-400 pl-2 py-1">
                                                                                                            <div class="flex justify-between items-center gap-2">
                                                                                                                <div class="flex items-center gap-2 flex-1">
                                                                                                                    <span class="text-gray-500 text-xs">
                                                                                                                        {t!("vendor.transaction_id")()}{": "}{transaction_id.to_string()}
                                                                                                                    </span>
                                                                                                                    <span class="text-gray-400">"·"</span>
                                                                                                                    <span class="text-xs text-gray-600">
                                                                                                                        {item_count}{" "}{t!("vendor.items")()}
                                                                                                                    </span>
                                                                                                                </div>
                                                                                                                <span class="text-gray-500 text-xs">{time_str}</span>
                                                                                                                <span class="font-medium">{format_currency(total, locale.get())}</span>
                                                                                                            </div>
                                                                                                        </div>
                                                                                                    }
                                                                                                })
                                                                                                .collect_view()
                                                                                        }
                                                                                        </div>
                                                                                    </div>
                                                                                </div>
                                                                            </div>
                                                                        </div>
                                                                    }
                                                                }
                                                            </div>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect_view()}

                                            {/* Bottom pagination */}
                                            <Show when=move || !filtered_vendor_reports.get().is_empty()>
                                                <div class="mt-4">
                                                    <Pagination
                                                        current_page=current_page
                                                        total_items=Signal::derive(move || filtered_vendor_reports.get().len())
                                                        page_size=page_size
                                                        on_page_change=move |page| set_current_page.set(page)
                                                        on_page_size_change=move |size| set_page_size.set(size)
                                                        translation_prefix="vendor.pagination"
                                                        show_page_size_selector=true
                                                    />
                                                </div>
                                            </Show>

                                            {/* Vendors without purchases heading */}
                                            <Show when=move || !filtered_vendors_without_purchases.get().is_empty()>
                                                <div class="mt-8 mb-4">
                                                    <h2 class="text-lg font-semibold text-gray-700 border-b border-gray-300 pb-2">
                                                        {t!("vendor.vendors_without_purchases_heading")}
                                                    </h2>
                                                    <p class="text-sm text-gray-500 mt-2">
                                                        {t!("vendor.vendors_without_purchases_help")}
                                                    </p>
                                                    <p class="mt-2 text-xs text-gray-500">
                                                        {t!("vendor.vendors_without_purchases_safe_delete_note")}
                                                    </p>
                                                </div>
                                            </Show>

                                             {/* Vendors without purchases */}
                                             {move || {
                                                 let handle_vendor_delete_click = handle_vendor_delete_click.clone();
                                                  filtered_vendors_without_purchases.get().into_iter().map(|vendor| {
                                                     let vendor_id = vendor.vendor_id.clone();
                                                     let vendor_id_stored = store_value(vendor_id.clone());
                                                     let vendor_id_str = vendor.vendor_id.as_str().to_string();
                                                     view! {
                                                    <div class="relative border border-gray-200 rounded-lg p-4 bg-gray-50 group transition-all duration-300">
                                                        <div
                                                            class="cursor-pointer"
                                                            on:click=move |e| {
                                                                e.stop_propagation();
                                                                handle_vendor_delete_click(vendor_id_stored.get_value());
                                                            }
                                                        >
                                                            <h3 class="text-lg font-semibold text-gray-600 flex items-center gap-2">
                                                                <span>{t!("vendor.id_label")()}</span>
                                                                <span>{vendor_id_str.clone()}</span>
                                                            </h3>
                                                            <p class="text-sm text-gray-500 mt-2">
                                                                {t!("vendor.no_purchases")}
                                                            </p>
                                                        </div>

                                                        {/* RED OVERLAY - shown when vendor is armed for deletion */}
                                                        <Show when=move || vendor_delete_signal.get() == Some(vendor_id_stored.get_value())>
                                                            <DeleteOverlay
                                                                prompt={t!("vendor.delete.arm_prompt")()}
                                                                aria_label={t!("vendor.delete.arm_prompt")()}
                                                                on_click={move |_| handle_vendor_delete_click(vendor_id_stored.get_value())}
                                                            />
                                                        </Show>
                                                    </div>
                                                }
                                            }).collect_view()
                                             }}
                                            </div>
                                        </Show>
                                    </div>
                                </Show>
                            </Show>
                        </Card>
                    </div>
                </Container>

                // Floating action buttons (bottom-right corner)
                <Show when=move || !vendor_reports.get().is_empty()>
                    <div class="fixed bottom-28 right-6 z-50 flex flex-col-reverse sm:flex-row items-end gap-3">
                        {/* Clear selection button - icon only with improved size and icon */}
                        <button
                            on:click=move |_| set_selected_vendor_ids.set(Vec::new())
                            disabled=move || selected_vendor_ids.get().is_empty()
                            title={t!("vendor.clear_selection")()}
                            aria-label={t!("vendor.clear_selection")()}
                            class="min-w-[4rem] min-h-[4rem] w-16 h-16 flex items-center justify-center rounded-full transition-all shadow-2xl bg-gray-800/80 backdrop-blur text-white hover:bg-gray-900 hover:scale-110 disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:scale-100 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                        >
                            <span class="sr-only">{t!("vendor.clear_selection")()}</span>
                            {/* Clear selection icon - list with X marks */}
                            <Icon icon=LuListX class="w-7 h-7" />
                        </button>

                        {/* Print button - with text */}
                        <button
                            on:click=move |_| {
                                let selected = selected_vendor_ids.get();
                                if selected.is_empty() {
                                    generate_all_reports(())
                                } else {
                                    generate_selected_reports(())
                                }
                            }
                            disabled=move || is_loading.get() || vendor_reports.get().is_empty()
                            title={move || {
                                let selected_count = selected_vendor_ids.get().len();
                                if selected_count > 0 {
                                    format!("{} ({})", t!("vendor.print_selection")(), selected_count)
                                } else {
                                    t!("vendor.print_all")()
                                }
                            }}
                            class="px-6 py-4 rounded-full font-semibold text-lg shadow-2xl transition-all bg-gradient-to-br from-blue-600 to-blue-700 text-white hover:from-blue-700 hover:to-blue-800 hover:shadow-2xl hover:scale-105 disabled:from-gray-400 disabled:to-gray-500 disabled:cursor-not-allowed disabled:hover:scale-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 flex items-center gap-3"
                        >
                            <Icon icon=LuPrinter class="w-6 h-6" />
                            {move || {
                                if is_loading.get() {
                                    t!("common.loading")()
                                } else {
                                    let selected_count = selected_vendor_ids.get().len();
                                    if selected_count > 0 {
                                        format!("{} ({})", t!("vendor.print_selection")(), selected_count)
                                    } else {
                                        t!("vendor.print_all")()
                                    }
                                }
                            }}
                        </button>
                    </div>
                </Show>
            </div>

            // Print-only layout (hidden on screen, visible during print)
            <div class="hidden print:block">
                <Show when=move || !reports_for_print.get().is_empty()>
                    {move || {
                        let reports = reports_for_print.get();
                        view! { <PrintVendorReports reports=reports /> }
                    }}
                </Show>
            </div>
        </>

        <Modal
            show=show_correction_modal
            on_close=Box::new(move || {
                set_pending_vendor_correction.set(None);
                set_show_correction_modal.set(false);
            })
            title=Signal::derive(move || t!("vendor.correction_modal_title")())
            size=ModalSize::Large
        >
            <Show when=move || pending_vendor_correction.get().is_some()>
                {move || {
                    if let Some(report) = pending_vendor_correction.get() {
                        view! {
                            <VendorCorrectionEditor
                                report=report
                                on_save=Callback::new(save_vendor_correction.clone())
                            />
                        }
                        .into_view()
                    } else {
                        View::default()
                    }
                }}
            </Show>
        </Modal>

        {/* Vendor deletion confirmation modal */}
        <Modal
            show=show_delete_modal
            on_close=cancel_vendor_delete.clone()
            title=Signal::derive(move || t!("vendor.delete.modal_title")())
            size=ModalSize::Medium
            action_bar=
                view! {
                    <div class="contents">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Box::new(cancel_vendor_delete.clone())
                        >
                            {t!("common.cancel")}
                        </Button>
                        <Button
                            variant=ButtonVariant::Danger
                            on_click=Box::new(perform_vendor_delete.clone())
                        >
                            {t!("vendor.delete.modal_confirm")}
                        </Button>
                    </div>
                }
                .into_view()
        >
            <Show when=move || pending_vendor_deletion.get().is_some()>
                <div class="space-y-4">
                    <p class="text-gray-700">
                        {move || {
                            if let Some(vendor) = pending_vendor_deletion.get() {
                                translate_with_params(
                                    "vendor.delete.modal_message",
                                    HashMap::from([("vendor_id", vendor.vendor_id.as_str().to_string())]),
                                )
                            } else {
                                String::new()
                            }
                        }}
                    </p>

                </div>
            </Show>
        </Modal>
    }
}

// Reuse PrintVendorReports component from reports page
#[component]
fn VendorCorrectionEditor(
    report: VendorReportData,
    on_save: Callback<(Vendor, Decimal, Option<String>)>,
) -> impl IntoView {
    let locale = use_locale();
    let (desired_payout_input, set_desired_payout_input) = create_signal(format_decimal_for_input(
        report.total_revenue,
        locale.get(),
        2,
    ));
    let (note_input, set_note_input) =
        create_signal(report.payout_correction_note.clone().unwrap_or_default());

    let (waive_participation, set_waive_participation) = create_signal(false);
    let (waive_revenue, set_waive_revenue) = create_signal(false);

    let set_desired = move |value: Decimal| {
        set_desired_payout_input.set(format_decimal_for_input(value, locale.get(), 2));
    };

    let compute_waived_desired = move |waive_participation: bool, waive_revenue: bool| {
        let participation_fee = if waive_participation {
            Decimal::ZERO
        } else {
            report.participation_fee
        };
        let sales_fee = if waive_revenue {
            Decimal::ZERO
        } else {
            report.sales_fee
        };
        report.sales_sum - participation_fee - sales_fee
    };

    let computed_correction = create_memo(move |_| {
        parse_decimal_input(&desired_payout_input.get())
            .map(|desired| desired - report.base_total_revenue)
            .unwrap_or(report.payout_correction)
    });

    let has_input_error =
        create_memo(move |_| parse_decimal_input(&desired_payout_input.get()).is_err());

    view! {
        <div class="space-y-4">
            <h4 class="text-sm font-semibold text-gray-800">{t!("vendor.correction_section_title")()}</h4>

            <div class="grid grid-cols-2 gap-2">
                <button
                    class=move || {
                        if waive_participation.get() {
                            "rounded-md border border-indigo-400 bg-indigo-100 px-3 py-2 text-sm font-semibold text-indigo-900"
                        } else {
                            "rounded-md border border-gray-300 px-3 py-2 text-sm font-medium hover:bg-gray-50"
                        }
                    }
                    on:click=move |_| {
                        let next_waive_participation = !waive_participation.get();
                        let current_waive_revenue = waive_revenue.get();
                        set_waive_participation.set(next_waive_participation);
                        set_desired(compute_waived_desired(
                            next_waive_participation,
                            current_waive_revenue,
                        ));
                    }
                >
                    <span>{t!("vendor.correction_waive_participation_fee")()}</span>
                    <span class="block text-xs opacity-80">
                        {format_currency(report.participation_fee, locale.get())}
                    </span>
                </button>
                <button
                    class=move || {
                        if waive_revenue.get() {
                            "rounded-md border border-indigo-400 bg-indigo-100 px-3 py-2 text-sm font-semibold text-indigo-900"
                        } else {
                            "rounded-md border border-gray-300 px-3 py-2 text-sm font-medium hover:bg-gray-50"
                        }
                    }
                    on:click=move |_| {
                        let current_waive_participation = waive_participation.get();
                        let next_waive_revenue = !waive_revenue.get();
                        set_waive_revenue.set(next_waive_revenue);
                        set_desired(compute_waived_desired(
                            current_waive_participation,
                            next_waive_revenue,
                        ));
                    }
                >
                    <span>{t!("vendor.correction_waive_revenue_fee")()}</span>
                    <span class="block text-xs opacity-80">
                        {format_currency(report.sales_fee, locale.get())}
                    </span>
                </button>
                <button
                    class="rounded-md border border-gray-300 px-3 py-2 text-sm font-medium hover:bg-gray-50"
                    on:click=move |_| {
                        set_waive_participation.set(false);
                        set_waive_revenue.set(false);
                        set_desired(Decimal::ZERO);
                    }
                >
                    {t!("vendor.correction_quick_set_zero")()}
                </button>
                <button
                    class="rounded-md border border-gray-300 px-3 py-2 text-sm font-medium hover:bg-gray-50"
                    on:click=move |_| {
                        set_waive_participation.set(false);
                        set_waive_revenue.set(false);
                        set_desired(report.base_total_revenue);
                    }
                >
                    {t!("vendor.correction_quick_reset")()}
                </button>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                <div class="rounded-md bg-gray-50 p-3">
                    <p class="text-xs text-gray-500">{t!("vendor.correction_auto_payout")()}</p>
                    <p class="text-lg font-semibold">{format_currency(report.base_total_revenue, locale.get())}</p>
                </div>
                <div class="rounded-md bg-gray-50 p-3">
                    <label class="text-xs text-gray-500">{t!("vendor.correction_desired_payout")()}</label>
                    <input
                        class="mt-1 w-full rounded-md border border-gray-300 px-2 py-1 text-sm"
                        type="text"
                        prop:value=desired_payout_input
                        on:input=move |ev| {
                            set_waive_participation.set(false);
                            set_waive_revenue.set(false);
                            set_desired_payout_input.set(event_target_value(&ev));
                        }
                    />
                </div>
                <div class="rounded-md bg-blue-50 p-3">
                    <p class="text-xs text-gray-500">{t!("vendor.correction_delta")()}</p>
                    <p class="text-lg font-semibold text-blue-700">
                        {move || format_currency(computed_correction.get(), locale.get())}
                    </p>
                </div>
            </div>

            <div>
                <label class="text-xs text-gray-500">{t!("vendor.correction_note")()}</label>
                <textarea
                    class="mt-1 w-full rounded-md border border-gray-300 px-2 py-1 text-sm"
                    rows="2"
                    prop:value=note_input
                    on:input=move |ev| set_note_input.set(event_target_value(&ev))
                />
            </div>

            <Show when=move || has_input_error.get()>
                <p class="text-sm text-red-600">{t!("vendor.errors.invalid_correction_amount")()}</p>
            </Show>

            <div class="flex justify-end">
                <button
                    class="rounded-md bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50"
                    disabled=move || has_input_error.get()
                    on:click=move |_| {
                        if let Ok(desired) = parse_decimal_input(&desired_payout_input.get()) {
                            let correction = desired - report.base_total_revenue;
                            let note = {
                                let trimmed = note_input.get().trim().to_string();
                                if trimmed.is_empty() { None } else { Some(trimmed) }
                            };
                            on_save.call((report.vendor.clone(), correction, note));
                        }
                    }
                >
                    {t!("vendor.correction_save")()}
                </button>
            </div>
        </div>
    }
}

#[component]
fn PrintVendorReports(reports: Vec<VendorReportData>) -> impl IntoView {
    let locale = use_locale();
    view! {
        <div class="print-reports-container">
            {reports
                .into_iter()
                .map(|report| {
                    let vendor_id = report.vendor.vendor_id.as_str().to_string();
                    let sales_sum = report.sales_sum;
                    let participation_fee = report.participation_fee;
                    let sales_fee = report.sales_fee;
                    let total_revenue = report.total_revenue;
                    let items = report.items;
                    let booth_description = report.booth.description.clone();
                    let booth_date = report.booth.date;

                    // Group items by transaction_id
                    let mut grouped: HashMap<PurchaseId, Vec<_>> = HashMap::new();
                    for item in items.iter() {
                        grouped.entry(item.transaction_id.clone()).or_default().push(item.clone());
                    }

                    let mut transactions: Vec<_> = grouped.into_iter().collect();
                    transactions.sort_by(|a, b| {
                        items.iter().position(|i| i.transaction_id == a.0)
                            .cmp(&items.iter().position(|i| i.transaction_id == b.0))
                    });
                    let has_multiple_transactions = transactions.len() > 1;

                    view! {
                        <div class="print-vendor-report">
                            // Vendor header - compact for efficiency
                            <div class="mb-3 pb-2 border-b border-gray-800">
                                <h1 class="text-2xl font-bold mb-1">{t!("vendor.report_title")}</h1>
                                <div class="text-base">
                                    <p class="font-semibold">
                                        {t!("vendor.id_label")()}{": "}{vendor_id.clone()}
                                    </p>
                                    <p class="text-sm text-gray-700">{booth_description.clone()}</p>
                                    <p class="text-sm text-gray-600">{booth_date.format("%d.%m.%Y").to_string()}</p>
                                </div>
                            </div>

                            // Financial summary - reduced spacing
                            <div class="mb-3">
                                <h2 class="sr-only">{t!("vendor.financial_summary")}</h2>
                                <div class="border border-gray-400 p-2">
                                    <div class="flex justify-between py-1">
                                        <span class="font-medium">{t!("vendor.gross_sales")}{"："}</span>
                                        <span class="text-base font-semibold">{move || format_currency(sales_sum, locale.get())}</span>
                                    </div>
                                    <div class="flex justify-between py-1 border-t border-gray-300">
                                        <span>{t!("vendor.participation_fee")}{"："}</span>
                                        <span>{move || format!("-{}", format_currency(participation_fee, locale.get()))}</span>
                                    </div>
                                    <div class="flex justify-between py-1 border-t border-gray-300">
                                        <span>{t!("vendor.sales_fee")}{"："}</span>
                                        <span>{move || format!("-{}", format_currency(sales_fee, locale.get()))}</span>
                                    </div>
                                    <div class="flex justify-between py-1 border-t-2 border-gray-800">
                                        <span class="text-base font-bold">{t!("vendor.net_payout")}{"："}</span>
                                        <span class="text-lg font-bold">{move || format_currency(total_revenue, locale.get())}</span>
                                    </div>
                                </div>
                            </div>

                            // Sales details - compact grid layout
                            <div class="print-sales-section">
                                <h2 class="text-lg font-bold mb-0">{t!("vendor.sales_details")}" ("{items.len()}" "{t!("vendor.items")}")"</h2>
                                <p class="text-xs text-gray-600 mt-1 mb-2">{t!("vendor.transaction_grouping_explanation")}</p>
                                <div class="print-transactions-container">
                                    {transactions
                                        .into_iter()
                                        .map(|(transaction_id, transaction_items)| {
                                            let is_multi_item = transaction_items.len() > 1;
                                            let transaction_total: Decimal = transaction_items.iter()
                                                .map(|item| item.item.amount)
                                                .sum();

                                            view! {
                                                <div class="print-transaction-group">
                                                    {if is_multi_item {
                                                        // Multi-item transaction
                                                        view! {
                                                            <div class="print-transaction-header">
                                                                <span class="text-xs text-gray-600">{t!("vendor.transaction_id")}{": "}{transaction_id.to_string()}</span>
                                                            </div>
                                                            <div class="print-items-grid">
                                                                {transaction_items
                                                                    .into_iter()
                                                                    .map(|report_item| {
                                                                        let time_str = report_item.timestamp
                                                                            .with_timezone(&chrono::Local)
                                                                            .format("%H:%M")
                                                                            .to_string();
                                                                        let amount = report_item.item.amount;
                                                                        view! {
                                                                            <div class="print-item">
                                                                                <span class="print-item-time">{time_str}</span>
                                                                                <span class="print-item-amount">{move || format_currency(amount, locale.get())}</span>
                                                                            </div>
                                                                        }
                                                                    })
                                                                    .collect_view()}
                                                            </div>
                                                            {if has_multiple_transactions {
                                                                view! {
                                                                    <div class="print-subtotal">
                                                                        <span>{t!("vendor.subtotal")}{"："}</span>
                                                                        <span class="font-semibold">{move || format_currency(transaction_total, locale.get())}</span>
                                                                    </div>
                                                                }.into_view()
                                                            } else {
                                                                View::default()
                                                            }}
                                                        }.into_view()
                                                    } else {
                                                        // Single-item transaction
                                                        let report_item = &transaction_items[0];
                                                        let time_str = report_item.timestamp
                                                            .with_timezone(&chrono::Local)
                                                            .format("%H:%M")
                                                            .to_string();
                                                        let amount = report_item.item.amount;
                                                        view! {
                                                            <div class="print-transaction-header">
                                                                <span class="text-xs text-gray-600">{t!("vendor.transaction_id")}{": "}{transaction_id.to_string()}</span>
                                                            </div>
                                                            <div class="print-items-grid">
                                                                <div class="print-item">
                                                                    <span class="print-item-time">{time_str}</span>
                                                                    <span class="print-item-amount">{move || format_currency(amount, locale.get())}</span>
                                                                </div>
                                                            </div>
                                                        }.into_view()
                                                    }}
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

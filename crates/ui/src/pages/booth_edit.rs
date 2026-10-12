#![allow(clippy::clone_on_copy)]

use crate::components::*;
use crate::error_translator::translate_domain_error;
use crate::i18n::use_locale;
use crate::state::*;
use crate::t;
use domain::models::booth::Booth;
use domain::models::{BoothId, BoothType};
use leptos::leptos_dom::helpers::{window_event_listener_untyped, WindowListenerHandle};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use std::str::FromStr;
use web_sys::window;

#[component]
pub fn BoothEditPage() -> impl IntoView {
    let app_state = use_app_state();
    let locale = use_locale();
    let booth_list_version = crate::selected_booth_context::use_booth_list_version();
    let navigate = StoredValue::new_local(use_navigate());
    let toast = use_toast();
    let params = use_params_map();
    let query = use_query_map();

    let booth_id = Signal::derive(move || {
        params
            .get()
            .get("id")
            .and_then(|id| uuid::Uuid::from_str(&id).ok())
            .map(BoothId::from_uuid)
    });

    let initial_tab_from_query = query
        .get_untracked()
        .get("tab")
        .and_then(|tab| tab.parse::<usize>().ok())
        .unwrap_or(0);

    let (booth, set_booth) = signal(None::<Booth>);
    let (is_loading, set_is_loading) = signal(true);
    let (purchase_count, set_purchase_count) = signal(0_usize);
    let active_tab = RwSignal::new(initial_tab_from_query);

    // Sticky title row pins under the app header. Height measured live (see
    // `booth_report.rs`/`booth_list.rs` for the same pattern).
    let (header_height, set_header_height) = signal(56.0_f64);

    let measure_sticky_heights = move || {
        if let Some(header) = window()
            .and_then(|w| w.document())
            .and_then(|document| document.get_element_by_id("app-header"))
        {
            set_header_height.set(header.get_bounding_client_rect().height());
        }
    };
    Effect::<LocalStorage>::new(move |prev: Option<WindowListenerHandle>| {
        drop(prev);
        let _ = booth.get();
        measure_sticky_heights();
        window_event_listener_untyped("resize", move |_| measure_sticky_heights())
    });

    let go_to =
        move |path: String| navigate.with_value(|navigate| navigate(&path, Default::default()));

    Effect::new(move |_| {
        let state_result = app_state.get();
        let Some(id) = booth_id.get() else {
            go_to("/booths".to_string());
            return;
        };

        if let Some(Ok(state)) = state_result {
            set_is_loading.set(true);
            spawn_local(async move {
                match state.booth_repository.find_by_id(&id).await {
                    Ok(Some(found)) => {
                        set_booth.set(Some(found));
                        set_is_loading.set(false);
                    }
                    Ok(None) | Err(_) => {
                        go_to("/booths".to_string());
                    }
                }
                if let Ok(purchases) = state.purchase_repository.find_by_booth(&id).await {
                    set_purchase_count.set(purchases.len());
                }
            });
        }
    });

    let go_to_report = move || {
        if let Some(b) = booth.get_untracked() {
            go_to(format!("/booths/{}/report", b.id));
        } else {
            go_to("/booths".to_string());
        }
    };

    let handle_edit_booth = move |data: BoothFormData| {
        let state_result = app_state.get();
        let booth_to_edit = booth.get();
        let locale = locale.get();

        spawn_local(async move {
            if let (Some(Ok(state)), Some(mut booth_value)) = (state_result, booth_to_edit) {
                let original_type = booth_value.booth_type;
                match data.update_booth(&mut booth_value, locale) {
                    Ok(_) => match state.booth_service.update_booth(booth_value.clone()).await {
                        Ok(_) => {
                            toast.success(
                                t!("booth.success.updated")()
                                    .replace("{description}", &booth_value.description),
                            );
                            booth_list_version.update(|v| *v += 1);
                            if original_type == BoothType::ThirdPartySale
                                && booth_value.booth_type == BoothType::DirectSale
                            {
                                // Stay on this page; only switch tab to products.
                                active_tab.set(1);
                                set_booth.set(Some(booth_value));
                            } else {
                                go_to_report();
                            }
                        }
                        Err(err) => {
                            toast.error(translate_domain_error(&err));
                        }
                    },
                    Err(err) => {
                        toast.error(translate_domain_error(&err));
                    }
                }
            }
        });
    };

    let translations = crate::i18n::use_translations();
    let edit_booth_title = move || translations.with(|t| t.get("booth.edit"));

    view! {
        <Container>
            <div class="pb-32">
                <Show
                    when=move || is_loading.get()
                    fallback=move || {
                        let Some(current_booth) = booth.get() else {
                            return ().into_any();
                        };
                        let current_locale = locale.get();
                        let initial_data = BoothFormData::from_booth(&current_booth, current_locale);
                        let tab = active_tab.get_untracked();
                        let pc = purchase_count.get_untracked();

                        view! {
                            <div
                                class="sticky z-30 flex items-center justify-between gap-3 border-b border-gray-200 bg-gray-50 pb-4 pt-6"
                                style=move || format!("top: {}px", header_height.get())
                            >
                                <h1 class="min-w-0 text-xl font-semibold text-gray-900">{edit_booth_title()}</h1>
                                <div class="flex shrink-0 items-center gap-2">
                                    <Button
                                        variant=ButtonVariant::Primary
                                        button_type="submit".to_string()
                                        form="edit-booth-form".to_string()
                                        class="min-h-11 min-w-11".to_string()
                                        title=t!("booth.save_button")()
                                        aria_label=t!("booth.save_button")()
                                    >
                                        <Icon icon=LuSave class="h-5 w-5 sm:mr-2" />
                                        <span class="hidden sm:inline">{t!("booth.save_button")()}</span>
                                    </Button>
                                    <Button
                                        variant=ButtonVariant::Secondary
                                        on_click=Box::new(go_to_report)
                                        class="min-h-11 min-w-11".to_string()
                                        title=t!("common.back")()
                                        aria_label=t!("common.back")()
                                    >
                                        <Icon icon=LuArrowLeft class="h-5 w-5 sm:mr-2" />
                                        <span class="hidden sm:inline">{t!("common.back")()}</span>
                                    </Button>
                                </div>
                            </div>

                            <div class="space-y-6 pt-6">
                                <BoothForm
                                    form_id="edit-booth-form".to_string()
                                    initial_data=initial_data
                                    on_submit=handle_edit_booth
                                    initial_tab=tab
                                    booth_type_locked={pc > 0}
                                    booth_id=current_booth.id
                                />
                            </div>
                        }
                        .into_any()
                    }
                >
                    <Card>
                        <div class="text-center py-12">
                            <p class="text-gray-600">{t!("booth.loading_message")}</p>
                        </div>
                    </Card>
                </Show>
            </div>
        </Container>
    }
}

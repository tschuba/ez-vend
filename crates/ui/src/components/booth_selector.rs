use crate::booth_ordering::sort_booths;
use crate::components::{
    toast::use_toast, Icon, LuCalendar, LuCheck, LuChevronDown, LuInbox, LuStore,
};
use crate::formatting::format_date;
use crate::i18n::use_locale;
use crate::selected_booth_context;
use crate::state::use_app_state;
use crate::t;
use domain::models::booth::Booth;
use leptos::html;
use leptos::leptos_dom::helpers::{window_event_listener_untyped, WindowListenerHandle};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

#[component]
pub fn BoothSelector() -> impl IntoView {
    let selected_booth = selected_booth_context::use_selected_booth();
    let booth_list_version = selected_booth_context::use_booth_list_version();
    let (booths, set_booths) = signal(Vec::<Booth>::new());
    let (archived_booth_count, set_archived_booth_count) = signal(0usize);
    let (is_open, set_is_open) = signal(false);
    let app_state = use_app_state();
    let toast = use_toast();
    let locale = use_locale();

    // Load available booths - reloads when booth_list_version changes
    Effect::new(move |_| {
        // Track booth_list_version to make this effect reactive to booth changes
        let _ = booth_list_version.get();

        let state_result = app_state.get();
        if let Some(Ok(state)) = state_result {
            spawn_local(async move {
                match state.booth_repository.find_all().await {
                    Ok(mut loaded_booths) => {
                        web_sys::console::log_1(
                            &format!("BoothSelector: Loaded {} booths", loaded_booths.len()).into(),
                        );
                        let archived_count = loaded_booths
                            .iter()
                            .filter(|booth| booth.is_archived())
                            .count();
                        sort_booths(&mut loaded_booths);
                        loaded_booths.retain(|booth| !booth.is_archived());
                        set_archived_booth_count.set(archived_count);
                        set_booths.set(loaded_booths);
                    }
                    Err(e) => {
                        let error_msg = t!("booth.errors.load_failed")();
                        toast.error(&error_msg);
                        web_sys::console::error_1(
                            &format!("Failed to load booths: {:?}", e).into(),
                        );
                    }
                }
            });
        }
    });

    // Close dropdown when clicking outside
    let dropdown_ref: NodeRef<html::Div> = NodeRef::new();

    // Handle Escape key to close dropdown
    Effect::<LocalStorage>::new(move |prev: Option<Option<WindowListenerHandle>>| {
        drop(prev);
        if is_open.get() {
            Some(window_event_listener_untyped("keydown", move |event| {
                let event: web_sys::KeyboardEvent = event.unchecked_into();
                if event.key() == "Escape" {
                    set_is_open.set(false);
                }
            }))
        } else {
            None
        }
    });

    view! {
        <div
            class={move || {
                if selected_booth.get().is_some() {
                    "relative ml-6".to_string()
                } else {
                    "relative w-full md:ml-6 md:w-auto".to_string()
                }
            }}
            node_ref=dropdown_ref
        >
            <div>
                // Badge Button
                <button
                    class={move || {
                        let base = "flex items-center gap-2 rounded-full px-4 py-2 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2";
                        let width = if selected_booth.get().is_some() {
                            ""
                        } else {
                            "w-full justify-between md:w-auto md:justify-start"
                        };
                        let variant = if selected_booth.get().is_some() {
                            "border border-blue-200 bg-blue-50 text-blue-900 hover:bg-blue-100 focus:ring-blue-500"
                        } else {
                            "border border-amber-300 bg-amber-50 text-amber-900 shadow-sm hover:bg-amber-100 focus:ring-amber-500"
                        };
                        format!("{} {} {}", base, width, variant)
                    }}
                    on:click=move |_| set_is_open.update(|open| *open = !*open)
                    aria-expanded=move || is_open.get()
                    aria-label=move || t!("booth.selector_aria_label")()
                >
                    {move || {
                        if let Some(booth) = selected_booth.get() {
                            let date_str = format_date(booth.date, locale.get());
                            view! {
                                <>
                                    <span class="text-sm font-medium">{date_str}</span>
                                    <span class="text-gray-400">"•"</span>
                                    <span class="max-w-[200px] text-sm font-semibold truncate">{booth.description}</span>
                                </>
                            }.into_any()
                        } else {
                            view! {
                                <>
                                    <span class="flex items-center gap-2">
                                        <Icon icon=LuStore class="h-5 w-5 text-amber-700" />
                                        <span class="text-sm font-semibold">{t!("booth.select_booth_cta")()}</span>
                                    </span>
                                </>
                            }.into_any()
                        }
                    }}
                    <Show
                        when=move || is_open.get()
                        fallback=move || {
                            view! { <Icon icon=LuChevronDown class="h-4 w-4 transition-transform duration-200" /> }
                        }
                    >
                        <Icon icon=LuChevronDown class="h-4 w-4 rotate-180 transition-transform duration-200" />
                    </Show>
                </button>
            </div>

            // Dropdown Menu with backdrop
            <Show when=move || is_open.get()>
                <>
                    // Invisible backdrop to capture outside clicks
                    <div
                        class="fixed inset-0 z-40"
                        on:click=move |_| set_is_open.set(false)
                    ></div>

                    // Dropdown menu
                    <div class="absolute right-0 z-50 mt-2 max-h-96 w-[calc(100vw-2rem)] max-w-80 overflow-y-auto rounded-lg border border-gray-200 bg-white py-2 shadow-xl">
                    {move || {
                        let booth_list = booths.get();
                        if booth_list.is_empty() {
                            view! {
                                <div class="px-4 py-8 text-center text-gray-500">
                                    <Icon icon=LuInbox class="w-12 h-12 mx-auto mb-3 text-gray-400" />
                                    <p class="text-sm font-medium">
                                        {move || if archived_booth_count.get() > 0 {
                                            t!("booth.selector_all_archived")()
                                        } else {
                                            t!("booth.no_booths_found")()
                                        }}
                                    </p>
                                    <p class="text-xs mt-1">
                                        {move || if archived_booth_count.get() > 0 {
                                            t!("booth.selector_count_with_archived")()
                                                .replace("{active}", "0")
                                                .replace("{archived}", &archived_booth_count.get().to_string())
                                        } else {
                                            t!("booth.create_first_booth")()
                                        }}
                                    </p>
                                </div>
                            }.into_any()
                        } else {
                            booth_list.into_iter().map(|booth| {
                                let booth_clone = booth.clone();
                                let is_selected = selected_booth.get().as_ref().map(|b| b.id == booth.id).unwrap_or(false);
                                let date_str = format_date(booth.date, locale.get());

                                view! {
                                    <button
                                        class={move || {
                                            let base = "w-full px-4 py-3 text-left hover:bg-gray-50 transition-colors duration-150 flex items-center gap-3";
                                            if is_selected {
                                                format!("{} bg-blue-50 border-l-4 border-blue-500", base)
                                            } else {
                                                format!("{} border-l-4 border-transparent", base)
                                            }
                                        }}
                                        on:click=move |_| {
                                            selected_booth.set(Some(booth_clone.clone()));
                                            set_is_open.set(false);
                                        }
                                        aria-pressed=is_selected
                                    >
                                        <div class="flex-1 min-w-0">
                                            <div class="flex items-center gap-2 mb-1">
                                                <span class="text-sm font-semibold text-gray-900">{booth.description}</span>
                                                {is_selected.then(|| view! {
                                                    <Icon icon=LuCheck class="w-4 h-4 text-blue-600" />
                                                })}
                                            </div>
                                            <div class="flex items-center gap-2 text-xs text-gray-500">
                                                <Icon icon=LuCalendar class="w-3 h-3" />
                                                <span>{date_str}</span>
                                            </div>
                                        </div>
                                    </button>
                                }
                            }).collect_view().into_any()
                        }
                    }}
                    </div>
                </>
            </Show>
        </div>
    }
}

#![allow(clippy::clone_on_copy)]

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

pub fn base_path() -> &'static str {
    use std::sync::OnceLock;

    static PATH: OnceLock<String> = OnceLock::new();

    PATH.get_or_init(|| {
        web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| {
                document
                    .query_selector("meta[name='router-base']")
                    .ok()
                    .flatten()
            })
            .and_then(|element| element.get_attribute("content"))
            .unwrap_or_default()
    })
    .as_str()
}

mod audio;
mod booth_ordering;
mod components;
pub mod config;
mod error;
mod error_logging;
mod error_translator;
mod formatting;
mod i18n;
mod pages;
mod selected_booth_context;
mod settings_support;
mod state;
mod utils;
pub use selected_booth_context::SelectedBoothProvider;

use components::*;
use i18n::*;
use pages::*;
use state::*;

#[component]
fn AppViewHeader() -> impl IntoView {
    let location = use_location();

    let title = Signal::derive(move || {
        let pathname = location.pathname.get();
        let path = pathname
            .strip_prefix(base_path())
            .unwrap_or(pathname.as_str());
        match path {
            "/booths" => Some(t!("booth.list_title")()),
            "/vendors" => Some(t!("vendor.list_title")()),
            "/checkout" => Some(t!("checkout.title")()),
            "/settings" => Some(t!("settings.title")()),
            _ => None,
        }
    });

    let show_import = Signal::derive(move || {
        let pathname = location.pathname.get();
        let path = pathname
            .strip_prefix(base_path())
            .unwrap_or(pathname.as_str());
        path == "/booths"
    });

    view! {
        <Show when=move || title.get().is_some()>
            <div class="bg-white shadow-sm">
                <Container>
                    <div class="flex min-h-16 items-center justify-between py-3">
                        <h1 class="text-2xl font-bold text-slate-900">{move || title.get().unwrap_or_default()}</h1>
                        <Show when=move || show_import.get()>
                            <ImportButton
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Small
                                class="border border-gray-300 hover:border-gray-400 hover:bg-gray-50 gap-1.5".to_string()
                            />
                        </Show>
                    </div>
                </Container>
            </div>
        </Show>
    }
}

/// Main application component
#[component]
pub fn App() -> impl IntoView {
    // Provide i18n context
    provide_i18n();

    // Provide metadata context
    provide_meta_context();

    // Provide refresh signal for the storage status footer
    let storage_refresh = create_rw_signal(0u32);
    provide_context(components::StorageStatusRefreshContext(storage_refresh));

    // Callback injected into Database: fires on every write commit so the footer
    // updates without manual signal increments at each call site.
    let on_write = std::rc::Rc::new(move || storage_refresh.update(|n| *n += 1));

    // Provide app state (repositories, services)
    let app_state = provide_app_state(Some(on_write));
    provide_context(app_state);

    let locale = use_locale();
    let selected_booth = selected_booth_context::use_selected_booth();

    // Update document title when locale changes
    {
        let locale = locale.clone();
        create_effect(move |_| {
            let _ = locale.get(); // Track locale changes
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    let title = t!("app.page_title")();
                    document.set_title(&title);
                }
            }
        });
    }

    view! {
        <ToastProvider>
            <Router base=base_path()>
                <div class="min-h-screen bg-gray-50 print:bg-white">
                    <div class="fixed left-0 right-0 top-0 z-40 bg-white print:hidden">
                        // Header (hidden during print)
                        <header>
                            <Container>
                                <div class="flex flex-wrap items-center justify-between gap-3 py-4 md:flex-nowrap">
                                    <a href=format!("{}/", base_path()) class="shrink-0 text-2xl font-bold text-blue-600">
                                        {t!("app.title")}
                                    </a>
                                    <div class="hidden md:block">
                                        <BoothSelector />
                                    </div>
                                    <div class="md:hidden">
                                        <Show when=move || selected_booth.get().is_some()>
                                            <BoothSelector />
                                        </Show>
                                    </div>
                                    <nav class="flex flex-wrap items-center gap-x-4 gap-y-2">
                                        <div class="flex items-center space-x-4">
                                            <a href=format!("{}/booths", base_path()) class="text-gray-700 transition-colors hover:text-blue-600">
                                                {t!("booth.list_title")}
                                            </a>
                                            <a href=format!("{}/vendors", base_path()) class="text-gray-700 transition-colors hover:text-blue-600">
                                                {t!("vendor.list_title")}
                                            </a>
                                            <a href=format!("{}/checkout", base_path()) class="text-gray-700 transition-colors hover:text-blue-600">
                                                {t!("checkout.title")}
                                            </a>
                                        </div>
                                        <div class="hidden h-6 w-px bg-gray-300 md:block"></div>
                                        <div class="flex items-center space-x-4">
                                            <button
                                                class="flex items-center gap-2 text-sm font-medium text-gray-700 transition-colors hover:text-blue-600"
                                                on:click=move |_| {
                                                    let new_locale = match locale.get() {
                                                        Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => Locale::En,
                                                        Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => Locale::De,
                                                    };
                                                    locale.set(new_locale);
                                                }
                                                aria-label=move || {
                                                    match locale.get() {
                                                        Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => {
                                                            t!("app.language_toggle_to_english")()
                                                        }
                                                        Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => {
                                                            t!("app.language_toggle_to_german")()
                                                        }
                                                    }
                                                }
                                            >
                                                <span class="text-2xl leading-none">
                                                    {move || match locale.get() {
                                                        Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => "🇬🇧",
                                                        Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => "🇩🇪",
                                                    }}
                                                </span>
                                                <span class="text-sm">
                                                    {move || match locale.get() {
                                                        Locale::De | Locale::DeDE | Locale::DeAT | Locale::DeCH => "EN",
                                                        Locale::En | Locale::EnUS | Locale::EnGB | Locale::EnEU => "DE",
                                                    }}
                                                </span>
                                            </button>
                                            <a
                                                href=format!("{}/settings", base_path())
                                                class="flex items-center gap-2 text-sm font-medium text-gray-700 transition-colors hover:text-blue-600"
                                            >
                                                <Icon icon=LuSettings class="h-4 w-4" />
                                                <span>{t!("settings.title")}</span>
                                            </a>
                                        </div>
                                    </nav>
                                 </div>
                              </Container>
                         </header>

                         <Show when=move || selected_booth.get().is_none()>
                             <div class="border-b border-amber-200 bg-amber-50 py-3 md:hidden">
                                 <Container>
                                     <BoothSelector />
                                 </Container>
                             </div>
                         </Show>

                         <AppViewHeader />
                     </div>


                    // Main content (remove padding during print)
                    <main id="main-content" tabindex="-1" class="pb-28 pt-36 print:py-0">
                        <Routes base=base_path().to_string()>
                            <Route path="/*any" view=HomePage/>
                            <Route path="/booths" view=BoothListPage/>
                            <Route path="/vendors" view=VendorListPage/>
                            <Route path="/checkout" view=CheckoutPage/>
                            <Route path="/settings" view=SettingsPage/>
                        </Routes>
                    </main>

                    // Footer (hidden during print)
                    <footer class="fixed bottom-0 left-0 right-0 z-20 border-t bg-white/95 backdrop-blur print:hidden">
                        <Container>
                            <div class="flex flex-col gap-2 py-3 text-center text-sm text-gray-600">
                                <StorageIndicator />
                                <div class="flex items-center justify-center gap-2">
                                    <span>{t!("app.copyright")}</span>
                                    <a
                                        href=format!(
                                            "{}/releases/tag/v{}",
                                            settings_support::REPOSITORY_URL,
                                            settings_support::APP_VERSION
                                        )
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        class="hover:underline hover:text-gray-900"
                                    >
                                        {format!("v{}", settings_support::APP_VERSION)}
                                    </a>
                                </div>
                            </div>
                        </Container>
                    </footer>
                </div>
            </Router>
        </ToastProvider>
    }
}

// Placeholder component for checkout (legacy - can be removed if not needed)
#[component]
fn CheckoutPlaceholder() -> impl IntoView {
    view! {
        <Container>
            <Card title_view={t!("checkout.title").into_view()}>
                <p class="text-gray-600">{t!("checkout.interface_coming_soon")}</p>
            </Card>
        </Container>
    }
}

#![allow(clippy::clone_on_copy)]

use crate::formatting::decimal_separator;
use crate::i18n::Locale;
use crate::t;
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmountInputMode {
    RightToLeft,
    #[default]
    Regular,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardKey {
    Digit(u8),
    Decimal,
    Backspace,
    Clear,
}

#[component]
pub fn OnScreenKeyboard(
    #[prop(into)] is_visible: Signal<bool>,
    on_key: Callback<KeyboardKey>,
    on_mode_change: Callback<()>,
    #[prop(into)] current_mode: Signal<AmountInputMode>,
    locale: Locale,
) -> impl IntoView {
    let decimal = decimal_separator(locale).to_string();
    let backspace_label = t!("checkout.keyboard_backspace");
    let decimal_label = t!("checkout.keyboard_decimal");
    let clear_label = t!("common.clear");
    let keyboard_mode_aria = t!("checkout.keyboard_mode_aria");
    let keyboard_mode_tooltip_rtl = t!("checkout.keyboard_mode_tooltip_rtl");
    let keyboard_mode_tooltip_regular = t!("checkout.keyboard_mode_tooltip_regular");
    view! {
        <Show when=move || is_visible.get()>
            <div class="rounded-2xl border border-slate-200 bg-gradient-to-b from-slate-50 to-white p-4 shadow-xl ring-1 ring-slate-200/60">
                <div class="space-y-4">
                    <div class="grid grid-cols-4 gap-2">
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="7"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(7))
                            }
                        >
                            "7"
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="8"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(8))
                            }
                        >
                            "8"
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="9"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(9))
                            }
                        >
                            "9"
                        </button>
                        <button
                            type="button"
                            class="row-span-2 min-h-14 rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-sm font-semibold text-amber-900 transition hover:bg-amber-100 focus:outline-none focus:ring-2 focus:ring-amber-500"
                            aria-label=backspace_label
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Backspace)
                            }
                        >
                            <span style="font-size: 2rem;">"⌫"</span>
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="4"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(4))
                            }
                        >
                            "4"
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="5"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(5))
                            }
                        >
                            "5"
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="6"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(6))
                            }
                        >
                            "6"
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="1"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(1))
                            }
                        >
                            "1"
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="2"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(2))
                            }
                        >
                            "2"
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="3"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(3))
                            }
                        >
                            "3"
                        </button>
                        <button
                            type="button"
                            class="row-span-2 min-h-14 rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm font-semibold text-rose-900 transition hover:bg-rose-100 focus:outline-none focus:ring-2 focus:ring-rose-500"
                            aria-label=clear_label
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Clear)
                            }
                        >
                            {clear_label}
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label="0"
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Digit(0))
                            }
                        >
                            "0"
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            aria-label=decimal_label
                            on:click={
                                let on_key = on_key.clone();
                                move |_| on_key.call(KeyboardKey::Decimal)
                            }
                        >
                            {decimal.clone()}
                        </button>
                        <button
                            type="button"
                            class="min-h-14 rounded-xl border border-indigo-200 bg-indigo-50 px-3 py-2 text-indigo-900 transition hover:bg-indigo-100 focus:outline-none focus:ring-2 focus:ring-indigo-500"
                            aria-label=keyboard_mode_aria
                            title={move || match current_mode.get() {
                                AmountInputMode::RightToLeft => keyboard_mode_tooltip_rtl(),
                                AmountInputMode::Regular => keyboard_mode_tooltip_regular(),
                            }}
                            on:click={
                                let on_mode_change = on_mode_change.clone();
                                move |_| on_mode_change.call(())
                            }
                        >
                            <span class="flex items-center justify-center text-lg font-mono font-semibold">
                                {move || match current_mode.get() {
                                    AmountInputMode::RightToLeft => "0.0_|",
                                    AmountInputMode::Regular => "_|",
                                }}
                            </span>
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

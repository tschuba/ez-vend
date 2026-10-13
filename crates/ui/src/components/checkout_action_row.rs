use crate::t;
use leptos::prelude::*;

/// Add (↵) and repeat (⟳) buttons shared by the on-screen keyboard and the
/// standalone touch action bar. Buttons never take focus so the native
/// keyboard stays open.
#[component]
pub fn CheckoutActionRow(
    on_add: Callback<()>,
    on_repeat: Callback<()>,
    #[prop(into)] can_repeat: Signal<bool>,
) -> impl IntoView {
    let add_label = t!("checkout.add_item");
    let repeat_label = t!("checkout.repeat_item");
    view! {
        <div class="flex gap-2">
            <button
                type="button"
                class="flex-[3] min-h-14 rounded-xl border border-emerald-600 bg-emerald-500 text-lg font-semibold text-white transition hover:bg-emerald-600 focus:outline-none focus:ring-2 focus:ring-emerald-500"
                title=add_label
                aria-label=add_label
                on:mousedown=move |ev| ev.prevent_default()
                on:click=move |_| on_add.run(())
            >
                <span style="font-size: 1.5rem;">"↵"</span>
            </button>
            <button
                type="button"
                disabled=move || !can_repeat.get()
                class="flex-1 min-h-14 rounded-xl border border-slate-200 bg-white text-lg font-semibold text-slate-900 transition hover:bg-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
                title=repeat_label
                aria-label=repeat_label
                on:mousedown=move |ev| ev.prevent_default()
                on:click=move |_| on_repeat.run(())
            >
                <span style="font-size: 1.5rem;">"⟳"</span>
            </button>
        </div>
    }
}

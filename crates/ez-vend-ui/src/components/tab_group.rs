use leptos::*;

#[derive(Clone)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub has_error: Signal<bool>,
}

#[component]
pub fn TabGroup(
    tabs: Vec<TabItem>,
    active_tab: RwSignal<usize>,
    children: Box<dyn Fn(usize) -> View + 'static>,
) -> impl IntoView {
    let tabs = StoredValue::new(tabs);
    let children = StoredValue::new(children);

    let tab_count = Signal::derive(move || tabs.with_value(|items| items.len()));

    let on_keydown = move |event: web_sys::KeyboardEvent| {
        let count = tab_count.get();
        if count == 0 {
            return;
        }

        let current = active_tab.get().min(count.saturating_sub(1));
        let next = match event.key().as_str() {
            "ArrowRight" => Some((current + 1) % count),
            "ArrowLeft" => Some((current + count - 1) % count),
            "Home" => Some(0),
            "End" => Some(count - 1),
            _ => None,
        };

        if let Some(index) = next {
            event.prevent_default();
            active_tab.set(index);
        }
    };

    view! {
        <div class="space-y-6">
            <div class="border-b border-gray-200">
                <div
                    class="flex gap-1 overflow-x-auto"
                    role="tablist"
                    aria-orientation="horizontal"
                >
                    <For
                        each=move || tabs.with_value(|items| items.clone())
                        key=|tab| tab.id.clone()
                        children=move |tab| {
                            let tab_id = format!("tab-{}", tab.id);
                            let panel_id = format!("panel-{}", tab.id);
                            let is_active = Signal::derive({
                                let tab_id = tab.id.clone();
                                move || {
                                    tabs.with_value(|items| {
                                        items
                                            .iter()
                                            .position(|item| item.id == tab_id)
                                            .map(|index| active_tab.get() == index)
                                            .unwrap_or(false)
                                    })
                                }
                            });

                            let on_click = {
                                let tab_id = tab.id.clone();
                                move |_| {
                                    tabs.with_value(|items| {
                                        if let Some(index) =
                                            items.iter().position(|item| item.id == tab_id)
                                        {
                                            active_tab.set(index);
                                        }
                                    });
                                }
                            };

                            view! {
                                <button
                                    type="button"
                                    id=tab_id.clone()
                                    class=move || {
                                        let base = "inline-flex min-w-fit items-center gap-2 whitespace-nowrap border-b-2 px-4 py-3 text-sm transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-inset rounded-t-lg";
                                        if is_active.get() {
                                            format!("{base} border-blue-600 font-semibold text-blue-600")
                                        } else {
                                            format!("{base} border-transparent font-medium text-gray-600 hover:border-gray-300 hover:text-gray-900")
                                        }
                                    }
                                    role="tab"
                                    aria-selected=move || if is_active.get() { "true" } else { "false" }
                                    aria-controls=panel_id.clone()
                                    tabindex=move || if is_active.get() { 0 } else { -1 }
                                    on:click=on_click
                                    on:keydown=on_keydown
                                >
                                    <span>{tab.label}</span>
                                    <Show when=move || tab.has_error.get()>
                                        <span
                                            class="h-2.5 w-2.5 rounded-full bg-red-500"
                                            aria-label="Validation errors"
                                            title="Validation errors"
                                        ></span>
                                    </Show>
                                </button>
                            }
                        }
                    />
                </div>
            </div>

            <div
                role="tabpanel"
                id=move || {
                    tabs.with_value(|items| {
                        items
                            .get(active_tab.get().min(items.len().saturating_sub(1)))
                            .map(|tab| format!("panel-{}", tab.id))
                            .unwrap_or_else(|| "panel-empty".to_string())
                    })
                }
                aria-labelledby=move || {
                    tabs.with_value(|items| {
                        items
                            .get(active_tab.get().min(items.len().saturating_sub(1)))
                            .map(|tab| format!("tab-{}", tab.id))
                            .unwrap_or_else(|| "tab-empty".to_string())
                    })
                }
            >
                {move || {
                    let index = tabs.with_value(|items| {
                        active_tab.get().min(items.len().saturating_sub(1))
                    });
                    children.with_value(|render| render(index))
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_item_can_be_constructed() {
        let has_error = create_rw_signal(false);
        let tab = TabItem {
            id: "basic".to_string(),
            label: "Basic".to_string(),
            has_error: has_error.into(),
        };

        assert_eq!(tab.id, "basic");
        assert_eq!(tab.label, "Basic");
        assert!(!tab.has_error.get());
    }
}

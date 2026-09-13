#![allow(clippy::clone_on_copy)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::redundant_closure)]

use leptos::*;

/// Input type
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputType {
    Text,
    Number,
    Email,
    Password,
    Date,
}

impl InputType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Number => "number",
            InputType::Email => "email",
            InputType::Password => "password",
            InputType::Date => "date",
        }
    }
}

/// Text input component
#[component]
pub fn Input(
    /// Input value signal
    value: RwSignal<String>,
    /// Input type
    #[prop(optional)]
    input_type: Option<InputType>,
    /// Placeholder text
    #[prop(optional)]
    placeholder: Option<String>,
    /// Label text
    #[prop(optional)]
    label: Option<String>,
    /// Whether the input is disabled
    #[prop(optional)]
    disabled: Option<bool>,
    /// Whether the input is required
    #[prop(optional)]
    required: Option<bool>,
    /// Error message to display
    #[prop(optional)]
    error: Option<ReadSignal<Option<String>>>,
    /// Additional CSS classes
    #[prop(optional)]
    class: Option<&'static str>,
    /// ARIA label for accessibility
    #[prop(optional)]
    aria_label: Option<String>,
    /// Node ref for imperative focus/select behavior
    #[prop(optional)]
    node_ref: Option<NodeRef<html::Input>>,
    /// Whether to focus the input when mounted
    #[prop(optional)]
    autofocus: Option<bool>,
    /// Whether to select the input contents after focusing
    #[prop(optional)]
    select_on_focus: Option<bool>,
) -> impl IntoView {
    let input_type = input_type.unwrap_or(InputType::Text);
    let disabled = disabled.unwrap_or(false);
    let required = required.unwrap_or(false);
    let autofocus = autofocus.unwrap_or(false);
    let select_on_focus = select_on_focus.unwrap_or(false);
    let input_ref = node_ref.unwrap_or_else(create_node_ref::<html::Input>);

    let has_error = move || error.map(|e| e.get().is_some()).unwrap_or(false);
    let input_classes = move || {
        if has_error() {
            "w-full px-3 py-2 border border-red-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-red-500"
        } else {
            "w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
        }
    };

    let additional_classes = class.unwrap_or("");
    let combined_classes = move || format!("{} {}", input_classes(), additional_classes);

    if autofocus {
        let node_ref_for_focus = input_ref.clone();
        set_timeout(
            move || {
                if let Some(input) = node_ref_for_focus.get() {
                    let _ = input.focus();
                    if select_on_focus {
                        let _ = input.select();
                    }
                }
            },
            std::time::Duration::from_millis(0),
        );
    }

    // Generate unique ID for error message association
    let error_id = move || {
        error.and_then(|e| {
            e.get()
                .as_ref()
                .map(|_| format!("input-error-{}", value.get_untracked().len()))
        })
    };

    view! {
        <div class="w-full">
            {label.map(|l| view! {
                <label class="block text-sm font-medium text-gray-700 mb-1">
                    {l}
                    {if required { " *" } else { "" }}
                </label>
            })}
            <input
                node_ref=input_ref
                type=input_type.as_str()
                class=combined_classes
                placeholder=placeholder.unwrap_or_default()
                disabled=disabled
                required=required
                aria-label=aria_label
                aria-invalid=move || if has_error() { Some("true") } else { None }
                aria-describedby=move || error_id()
                aria-required=if required { Some("true") } else { None }
                prop:value=move || value.get()
                on:input=move |ev| {
                    value.set(event_target_value(&ev));
                }
            />
            {move || error.and_then(|e| e.get().map(|err| view! {
                <p class="mt-1 text-sm text-red-600" id=error_id() role="alert">{err}</p>
            }))}
        </div>
    }
}

/// Number input component (specialized for decimals)
#[component]
pub fn NumberInput(
    /// Input value signal (as string for editing)
    value: RwSignal<String>,
    /// Label text
    #[prop(optional)]
    label: Option<String>,
    /// Placeholder text
    #[prop(optional)]
    placeholder: Option<String>,
    /// Minimum value (not enforced in HTML, for documentation)
    #[prop(optional)]
    _min: Option<f64>,
    /// Maximum value (not enforced in HTML, for documentation)
    #[prop(optional)]
    _max: Option<f64>,
    /// Step value (not enforced in HTML, for documentation)
    #[prop(optional)]
    _step: Option<f64>,
    /// Whether the input is disabled
    #[prop(optional)]
    disabled: Option<bool>,
    /// Whether the input is required
    #[prop(optional)]
    required: Option<bool>,
    /// Error message to display
    #[prop(optional)]
    error: Option<ReadSignal<Option<String>>>,
    /// ARIA label for accessibility
    #[prop(optional)]
    aria_label: Option<String>,
) -> impl IntoView {
    let disabled = disabled.unwrap_or(false);
    let required = required.unwrap_or(false);

    let has_error = move || error.map(|e| e.get().is_some()).unwrap_or(false);
    let input_classes = move || {
        if has_error() {
            "w-full px-3 py-2 border border-red-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-red-500"
        } else {
            "w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
        }
    };

    // Generate unique ID for error message association
    let error_id = move || {
        error.and_then(|e| {
            e.get()
                .as_ref()
                .map(|_| format!("number-input-error-{}", value.get_untracked().len()))
        })
    };

    view! {
        <div class="w-full">
            {label.map(|l| view! {
                <label class="block text-sm font-medium text-gray-700 mb-1">
                    {l}
                    {if required { " *" } else { "" }}
                </label>
            })}
            <input
                type="text"
                inputmode="decimal"
                class=input_classes
                placeholder=placeholder.unwrap_or_default()
                disabled=disabled
                required=required
                aria-label=aria_label
                aria-invalid=move || if has_error() { Some("true") } else { None }
                aria-describedby=move || error_id()
                aria-required=if required { Some("true") } else { None }
                prop:value=move || value.get()
                on:input=move |ev| {
                    value.set(event_target_value(&ev));
                }
            />
            {move || error.and_then(|e| e.get().map(|err| view! {
                <p class="mt-1 text-sm text-red-600" id=error_id() role="alert">{err}</p>
            }))}
        </div>
    }
}

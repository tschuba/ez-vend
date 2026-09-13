use crate::components::{Input, InputType};
use crate::formatting::format_date;
use crate::i18n::use_locale;
use crate::t;
use chrono::NaiveDate;
use domain::models::booth::Booth;
use leptos::*;

#[derive(Clone, Debug)]
pub struct CopyBoothFormData {
    pub description: String,
    pub date: String,
}

impl CopyBoothFormData {
    pub fn from_booth(booth: &Booth) -> Self {
        let today = chrono::Local::now().date_naive();

        Self {
            description: format!("{} ({})", booth.description, today.format("%Y-%m-%d")),
            date: today.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn parse_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }
}

#[component]
pub fn CopyBoothDialog(
    source_booth: Booth,
    form_id: String,
    #[prop(default = false)] autofocus_description: bool,
    #[prop(optional)] initial_data: Option<CopyBoothFormData>,
    on_submit: impl Fn(CopyBoothFormData) + 'static,
) -> impl IntoView {
    let locale = use_locale();
    let description_input_ref = create_node_ref::<html::Input>();
    let form_data = initial_data.unwrap_or_else(|| CopyBoothFormData::from_booth(&source_booth));
    let description = create_rw_signal(form_data.description);
    let date = create_rw_signal(form_data.date);

    let (description_error, set_description_error) = create_signal(None::<String>);
    let (date_error, set_date_error) = create_signal(None::<String>);

    let validate_and_submit = move || {
        set_description_error.set(None);
        set_date_error.set(None);

        let mut has_errors = false;
        let description_value = description.get();
        let date_value = date.get();

        if description_value.trim().is_empty() {
            set_description_error.set(Some(t!("booth.form_errors.description_required")()));
            has_errors = true;
        } else if description_value.chars().count() > 200 {
            set_description_error.set(Some(t!("booth.form_errors.description_length")()));
            has_errors = true;
        }

        if date_value.trim().is_empty() {
            set_date_error.set(Some(t!("booth.form_errors.date_required")()));
            has_errors = true;
        } else if NaiveDate::parse_from_str(&date_value, "%Y-%m-%d").is_err() {
            set_date_error.set(Some(t!("validation.date_invalid")()));
            has_errors = true;
        }

        if !has_errors {
            on_submit(CopyBoothFormData {
                description: description_value,
                date: date_value,
            });
        }
    };

    let source_date = format_date(source_booth.date, locale.get());

    view! {
        <div class="space-y-6">
            <div class="rounded-lg border border-blue-100 bg-blue-50 px-4 py-3 text-sm text-blue-900">
                <p class="font-medium">{t!("booth.copy_source_label")()}</p>
                <p>{source_booth.description.clone()}</p>
                <p>{t!("booth.date_prefix")} " " {source_date}</p>
            </div>

            <form
                id=form_id
                class="space-y-4"
                on:submit=move |e| {
                    e.prevent_default();
                    validate_and_submit();
                }
            >
                <Input
                    node_ref=description_input_ref
                    autofocus=autofocus_description
                    select_on_focus=autofocus_description
                    value=description
                    label=t!("booth.description_label")()
                    placeholder=t!("booth.description_placeholder")()
                    required=true
                    error=description_error
                />

                <Input
                    value=date
                    input_type=InputType::Date
                    label=t!("booth.date_label")()
                    required=true
                    error=date_error
                />
            </form>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::models::booth::FeeConfig;
    use rust_decimal::Decimal;

    fn test_booth(description: &str) -> Booth {
        Booth::new(
            description.to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            FeeConfig {
                participation_fee: Decimal::ZERO,
                sales_fee_percent: Decimal::ZERO,
                rounding_step: Decimal::ZERO,
            },
        )
        .unwrap()
    }

    #[test]
    fn copy_form_defaults_use_date_suffix_once() {
        let booth = test_booth("Spring Fair");
        let form = CopyBoothFormData::from_booth(&booth);

        assert_eq!(form.date.len(), 10);
        assert_eq!(form.description, format!("Spring Fair ({})", form.date));
    }

    #[test]
    fn copy_form_does_not_append_copy_copy_suffixes() {
        let booth = test_booth("Spring Fair - Copy");
        let form = CopyBoothFormData::from_booth(&booth);

        assert!(form.description.starts_with("Spring Fair - Copy ("));
        assert!(!form.description.contains(" - Copy - Copy"));
    }
}

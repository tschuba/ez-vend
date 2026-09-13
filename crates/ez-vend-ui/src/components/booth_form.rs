#![allow(clippy::unnecessary_lazy_evaluations)]

use crate::components::*;
use crate::error_translator::translate_domain_error;
use crate::formatting::{
    currency_symbol_for_label, format_decimal_for_input, parse_decimal_input,
    DecimalInputParseError,
};
use crate::i18n::{use_locale, Locale};
use crate::t;
use chrono::NaiveDate;
use domain::error::DomainError;
use domain::error_code::ValidationError;
use domain::models::booth::{
    Booth, FeeConfig, OmissionRule, VendorIdOmissionRules, VendorIdValidation,
};
use domain::validation::{validate_digits_only_constraints, validate_regex_pattern};
use leptos::*;
use rust_decimal::Decimal;
use std::collections::HashSet;

/// Form data for creating/editing a booth
#[derive(Clone, Debug)]
pub struct BoothFormData {
    pub description: String,
    pub date: String,
    pub participation_fee: String,
    pub sales_fee_percent: String,
    pub rounding_step: String,
    pub amount_stepping: String,
    pub vendor_validation_type: String, // "unrestricted", "digits_only", or "regex"
    pub vendor_validation_regex: String,
    pub vendor_validation_min: String,
    pub vendor_validation_max: String,
    pub vendor_omission_rules: VendorIdOmissionRules,
}

impl Default for BoothFormData {
    fn default() -> Self {
        // Use English locale for backward compatibility
        Self::default_with_locale(Locale::En)
    }
}

impl BoothFormData {
    fn parse_digits_only_config(&self) -> Result<(usize, Option<usize>), DomainError> {
        let min =
            parse_digits_only_field(&self.vendor_validation_min, false)?.ok_or_else(|| {
                DomainError::Validation(ValidationError::DigitsOnlyConstraintInvalidNumber)
            })?;
        let max = parse_digits_only_field(&self.vendor_validation_max, true)?;

        validate_digits_only_constraints(min, max)?;

        Ok((min, max))
    }

    /// Create default form data with locale-aware formatting
    pub fn default_with_locale(locale: Locale) -> Self {
        let today = chrono::Local::now().date_naive();
        let date_str = today.format("%Y-%m-%d").to_string();

        Self {
            description: String::new(),
            date: date_str,
            participation_fee: format_decimal_for_input(Decimal::ONE, locale, 2),
            sales_fee_percent: format_decimal_for_input(Decimal::from(15), locale, 2),
            rounding_step: format_decimal_for_input(Decimal::new(50, 2), locale, 2),
            amount_stepping: String::new(),
            vendor_validation_type: "digits_only".to_string(), // Default to digits only
            vendor_validation_regex: String::new(),
            vendor_validation_min: "1".to_string(),
            vendor_validation_max: String::new(),
            vendor_omission_rules: VendorIdOmissionRules::recommended(),
        }
    }

    /// Create form data from an existing Booth
    pub fn from_booth(booth: &Booth, locale: Locale) -> Self {
        let (validation_type, validation_regex, validation_min, validation_max) =
            match &booth.vendor_id_validation {
                VendorIdValidation::Unrestricted => (
                    "unrestricted".to_string(),
                    String::new(),
                    "1".to_string(),
                    String::new(),
                ),
                VendorIdValidation::DigitsOnly { min, max } => (
                    "digits_only".to_string(),
                    String::new(),
                    min.to_string(),
                    max.map(|value| value.to_string()).unwrap_or_default(),
                ),
                VendorIdValidation::Regex(pattern) => (
                    "regex".to_string(),
                    pattern.clone(),
                    "1".to_string(),
                    String::new(),
                ),
            };

        Self {
            description: booth.description.clone(),
            date: booth.date.format("%Y-%m-%d").to_string(),
            participation_fee: format_decimal_for_input(booth.fees.participation_fee, locale, 2),
            sales_fee_percent: format_decimal_for_input(booth.fees.sales_fee_percent, locale, 2),
            rounding_step: format_decimal_for_input(booth.fees.rounding_step, locale, 2),
            amount_stepping: booth
                .amount_stepping
                .map(|step| format_decimal_for_input(step, locale, 2))
                .unwrap_or_default(),
            vendor_validation_type: validation_type,
            vendor_validation_regex: validation_regex,
            vendor_validation_min: validation_min,
            vendor_validation_max: validation_max,
            vendor_omission_rules: booth.vendor_id_omission_rules.clone(),
        }
    }

    /// Convert form data to domain Booth model
    ///
    /// # Errors
    ///
    /// Returns DomainError if:
    /// - Date string cannot be parsed
    /// - Fee values cannot be parsed to Decimal
    /// - Fee configuration validation fails
    /// - Vendor ID validation configuration is invalid
    pub fn to_booth(&self, _locale: Locale) -> Result<Booth, DomainError> {
        // Parse date
        let date = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
            .map_err(|_| DomainError::Validation(ValidationError::DateInvalid))?;

        // Parse fee values using flexible parsing (accepts both comma and dot)
        let participation_fee = parse_decimal_input(&self.participation_fee)
            .map_err(|_| DomainError::Validation(ValidationError::ParticipationFeeInvalid))?;

        let sales_fee_percent = parse_decimal_input(&self.sales_fee_percent)
            .map_err(|_| DomainError::Validation(ValidationError::SalesFeePercentInvalid))?;

        let rounding_step = parse_decimal_input(&self.rounding_step)
            .map_err(|_| DomainError::Validation(ValidationError::RoundingStepInvalid))?;

        let amount_stepping = if self.amount_stepping.trim().is_empty() {
            None
        } else {
            Some(
                parse_decimal_input(&self.amount_stepping)
                    .map_err(|_| DomainError::Validation(ValidationError::AmountSteppingInvalid))?,
            )
        };

        // Create FeeConfig
        let fees = FeeConfig {
            participation_fee,
            sales_fee_percent,
            rounding_step,
        };

        // Parse vendor validation rule
        let vendor_id_validation = match self.vendor_validation_type.as_str() {
            "unrestricted" => VendorIdValidation::Unrestricted,
            "digits_only" => {
                let (min, max) = self.parse_digits_only_config()?;
                VendorIdValidation::DigitsOnly { min, max }
            }
            "regex" => {
                // Validate the regex pattern
                validate_regex_pattern(&self.vendor_validation_regex)?;
                VendorIdValidation::Regex(self.vendor_validation_regex.clone())
            }
            _ => VendorIdValidation::DigitsOnly { min: 1, max: None }, // Default fallback
        };

        // Create Booth (this validates the fee ranges)
        let mut booth = Booth::new(self.description.clone(), date, fees)?;
        self.vendor_omission_rules.validate()?;
        booth.vendor_id_validation = vendor_id_validation;
        booth.vendor_id_omission_rules = self.vendor_omission_rules.clone();
        booth.update_amount_stepping(amount_stepping)?;

        Ok(booth)
    }

    /// Update an existing booth with form data
    ///
    /// # Errors
    ///
    /// Returns DomainError if fee configuration validation fails
    pub fn update_booth(&self, booth: &mut Booth, _locale: Locale) -> Result<(), DomainError> {
        // Parse date
        let date = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
            .map_err(|_| DomainError::Validation(ValidationError::DateInvalid))?;

        // Parse fee values using flexible parsing (accepts both comma and dot)
        let participation_fee = parse_decimal_input(&self.participation_fee)
            .map_err(|_| DomainError::Validation(ValidationError::ParticipationFeeInvalid))?;

        let sales_fee_percent = parse_decimal_input(&self.sales_fee_percent)
            .map_err(|_| DomainError::Validation(ValidationError::SalesFeePercentInvalid))?;

        let rounding_step = parse_decimal_input(&self.rounding_step)
            .map_err(|_| DomainError::Validation(ValidationError::RoundingStepInvalid))?;

        let amount_stepping = if self.amount_stepping.trim().is_empty() {
            None
        } else {
            Some(
                parse_decimal_input(&self.amount_stepping)
                    .map_err(|_| DomainError::Validation(ValidationError::AmountSteppingInvalid))?,
            )
        };

        // Create and validate FeeConfig
        let fees = FeeConfig {
            participation_fee,
            sales_fee_percent,
            rounding_step,
        };
        fees.validate_ranges()?;

        // Parse vendor validation rule
        let vendor_id_validation = match self.vendor_validation_type.as_str() {
            "unrestricted" => VendorIdValidation::Unrestricted,
            "digits_only" => {
                let (min, max) = self.parse_digits_only_config()?;
                VendorIdValidation::DigitsOnly { min, max }
            }
            "regex" => {
                // Validate the regex pattern
                validate_regex_pattern(&self.vendor_validation_regex)?;
                VendorIdValidation::Regex(self.vendor_validation_regex.clone())
            }
            _ => VendorIdValidation::DigitsOnly { min: 1, max: None }, // Default fallback
        };

        // Update booth fields
        self.vendor_omission_rules.validate()?;
        booth.update_description(self.description.clone());
        booth.date = date;
        booth.update_fees(fees);
        booth.vendor_id_validation = vendor_id_validation;
        booth.vendor_id_omission_rules = self.vendor_omission_rules.clone();
        booth.update_amount_stepping(amount_stepping)?;

        Ok(())
    }
}

pub(crate) fn parse_digits_only_field(
    value: &str,
    allow_empty: bool,
) -> Result<Option<usize>, DomainError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return if allow_empty {
            Ok(None)
        } else {
            Err(DomainError::Validation(
                ValidationError::DigitsOnlyConstraintInvalidNumber,
            ))
        };
    }

    trimmed
        .parse::<usize>()
        .map(Some)
        .map_err(|_| DomainError::Validation(ValidationError::DigitsOnlyConstraintInvalidNumber))
}

pub(crate) fn sanitize_digits_only_field(value: &str) -> String {
    value.chars().filter(|c| c.is_ascii_digit()).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DigitsOnlyFieldValidation {
    MinInvalid,
    MaxInvalid,
    MinGreaterThanMax,
}

pub(crate) fn validate_digits_only_form_fields(
    min_input: &str,
    max_input: &str,
) -> Option<DigitsOnlyFieldValidation> {
    let parsed_min = match parse_digits_only_field(min_input, false) {
        Ok(Some(value)) => value,
        Ok(None) | Err(_) => return Some(DigitsOnlyFieldValidation::MinInvalid),
    };

    let parsed_max = match parse_digits_only_field(max_input, true) {
        Ok(value) => value,
        Err(_) => return Some(DigitsOnlyFieldValidation::MaxInvalid),
    };

    if let Err(err) = validate_digits_only_constraints(parsed_min, parsed_max) {
        debug_assert!(matches!(
            err,
            DomainError::Validation(ValidationError::DigitsOnlyMinMaxInvalid { .. })
        ));
        return Some(DigitsOnlyFieldValidation::MinGreaterThanMax);
    }

    None
}

fn digits_only_form_error_messages(
    validation: Option<DigitsOnlyFieldValidation>,
) -> (Option<String>, Option<String>) {
    match validation {
        Some(DigitsOnlyFieldValidation::MinInvalid) => (
            Some(t!("booth.form_errors.positive_number_required")()),
            None,
        ),
        Some(DigitsOnlyFieldValidation::MaxInvalid) => (
            None,
            Some(t!("booth.form_errors.positive_number_required")()),
        ),
        Some(DigitsOnlyFieldValidation::MinGreaterThanMax) => {
            let message = t!("validation.digits_only_min_max_invalid")();
            (Some(message.clone()), Some(message))
        }
        None => (None, None),
    }
}

fn parse_u32_input(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

pub(crate) fn parse_exact_omission_values(input: &str) -> Vec<String> {
    let mut seen = HashSet::new();

    input
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert((*value).to_string()))
        .map(str::to_string)
        .collect()
}

fn omission_rule_type_label(rule: &OmissionRule) -> String {
    match rule {
        OmissionRule::Exact(_) => t!("booth.vendor_omission_type_exact")(),
        OmissionRule::Wildcard(_) => t!("booth.vendor_omission_type_wildcard")(),
        OmissionRule::Regex(_) => t!("booth.vendor_omission_type_regex")(),
        OmissionRule::Range { .. } | OmissionRule::RangeWithStep { .. } => {
            t!("booth.vendor_omission_type_range")()
        }
    }
}

fn omission_rule_value(rule: &OmissionRule) -> String {
    match rule {
        OmissionRule::Exact(value) => value.clone(),
        OmissionRule::Wildcard(pattern) => pattern.as_str().to_string(),
        OmissionRule::Regex(pattern) => pattern.as_str().to_string(),
        OmissionRule::Range { start, end } => format!("{start}-{end}"),
        OmissionRule::RangeWithStep { start, end, step } => format!("{start}-{end} (step {step})"),
    }
}

fn omission_rule_key(index: usize, rule: &OmissionRule) -> String {
    format!(
        "{index}:{}:{}",
        omission_rule_type_label(rule),
        omission_rule_value(rule)
    )
}

/// Booth form component for creating and editing booths
#[component]
pub fn BoothForm(
    /// HTML form id used by modal footer submit button
    form_id: String,
    /// Whether to auto-focus the description field
    #[prop(default = false)]
    autofocus_description: bool,
    /// Initial form data (for editing)
    #[prop(optional)]
    initial_data: Option<BoothFormData>,
    /// Callback when form is submitted
    on_submit: impl Fn(BoothFormData) + 'static,
) -> impl IntoView {
    let form_data = create_rw_signal(initial_data.unwrap_or_default());
    let active_tab = create_rw_signal(0_usize);
    let description_input_ref = create_node_ref::<html::Input>();

    // Individual field signals for Input components
    let description = create_rw_signal(form_data.get_untracked().description);
    let date = create_rw_signal(form_data.get_untracked().date);
    let participation_fee = create_rw_signal(form_data.get_untracked().participation_fee);
    let sales_fee_percent = create_rw_signal(form_data.get_untracked().sales_fee_percent);
    let rounding_step = create_rw_signal(form_data.get_untracked().rounding_step);
    let amount_stepping = create_rw_signal(form_data.get_untracked().amount_stepping);
    let vendor_validation_type = create_rw_signal(form_data.get_untracked().vendor_validation_type);
    let vendor_validation_regex =
        create_rw_signal(form_data.get_untracked().vendor_validation_regex);
    let vendor_validation_min = create_rw_signal(form_data.get_untracked().vendor_validation_min);
    let vendor_validation_max = create_rw_signal(form_data.get_untracked().vendor_validation_max);
    let vendor_omission_rules = create_rw_signal(form_data.get_untracked().vendor_omission_rules);

    let new_omission_rule_type = create_rw_signal("exact".to_string());
    let new_omission_value = create_rw_signal(String::new());
    let new_omission_pattern = create_rw_signal(String::new());
    let new_omission_range_start = create_rw_signal(String::new());
    let new_omission_range_end = create_rw_signal(String::new());
    let new_omission_range_step = create_rw_signal(String::new());
    let omission_help_text = Signal::derive(move || match new_omission_rule_type.get().as_str() {
        "exact" => t!("booth.vendor_omission_help_exact")(),
        "wildcard" => t!("booth.vendor_omission_help_wildcard")(),
        "regex" => t!("booth.vendor_omission_help_regex")(),
        "range" => t!("booth.vendor_omission_help_range")(),
        _ => t!("booth.vendor_omission_help")(),
    });

    // Validation errors
    let (description_error, set_description_error) = create_signal(None::<String>);
    let (date_error, set_date_error) = create_signal(None::<String>);
    let (participation_fee_error, set_participation_fee_error) = create_signal(None::<String>);
    let (sales_fee_percent_error, set_sales_fee_percent_error) = create_signal(None::<String>);
    let (rounding_step_error, set_rounding_step_error) = create_signal(None::<String>);
    let (amount_stepping_error, set_amount_stepping_error) = create_signal(None::<String>);
    let (vendor_validation_regex_error, set_vendor_validation_regex_error) =
        create_signal(None::<String>);
    let (vendor_validation_min_error, set_vendor_validation_min_error) =
        create_signal(None::<String>);
    let (vendor_validation_max_error, set_vendor_validation_max_error) =
        create_signal(None::<String>);
    let (vendor_omission_error, set_vendor_omission_error) = create_signal(None::<String>);

    let basic_tab_has_errors = Signal::derive(move || {
        description_error.get().is_some()
            || date_error.get().is_some()
            || participation_fee_error.get().is_some()
            || sales_fee_percent_error.get().is_some()
            || rounding_step_error.get().is_some()
    });
    let validation_tab_has_errors = Signal::derive(move || {
        amount_stepping_error.get().is_some()
            || vendor_validation_regex_error.get().is_some()
            || vendor_validation_min_error.get().is_some()
            || vendor_validation_max_error.get().is_some()
    });
    let omission_tab_has_errors = Signal::derive(move || vendor_omission_error.get().is_some());

    let locale = use_locale();

    // Localized validation messages
    let description_required_msg = t!("booth.form_errors.description_required");
    let description_length_msg = t!("booth.form_errors.description_length");
    let date_required_msg = t!("booth.form_errors.date_required");
    let participation_fee_required_msg = move || {
        let locale_val = locale.get();
        let currency = currency_symbol_for_label(locale_val);
        t!("booth.form_errors.participation_fee_required")().replace("{currency}", currency)
    };
    let sales_fee_required_msg = t!("booth.form_errors.sales_fee_required");
    let rounding_step_required_msg = t!("booth.form_errors.rounding_step_required");
    let cannot_be_negative_msg = t!("booth.form_errors.cannot_be_negative");
    let cannot_exceed_100_msg = t!("booth.form_errors.cannot_exceed_100");
    let invalid_number_format_msg = t!("booth.form_errors.invalid_number_format");
    let max_two_decimals_msg = t!("booth.form_errors.max_two_decimals");

    create_effect(move |_| {
        if vendor_validation_type.get() != "digits_only" {
            set_vendor_validation_min_error.set(None);
            set_vendor_validation_max_error.set(None);
            return;
        }

        let min_value = vendor_validation_min.get();
        let min_sanitized = sanitize_digits_only_field(&min_value);
        if min_value != min_sanitized {
            vendor_validation_min.set(min_sanitized.clone());
            return;
        }

        let max_value = vendor_validation_max.get();
        let max_sanitized = sanitize_digits_only_field(&max_value);
        if max_value != max_sanitized {
            vendor_validation_max.set(max_sanitized.clone());
            return;
        }

        let (min_error, max_error) = digits_only_form_error_messages(
            validate_digits_only_form_fields(&min_sanitized, &max_sanitized),
        );
        set_vendor_validation_min_error.set(min_error);
        set_vendor_validation_max_error.set(max_error);
    });

    let validate_and_submit = move || {
        // Clear previous errors
        set_description_error.set(None);
        set_date_error.set(None);
        set_participation_fee_error.set(None);
        set_sales_fee_percent_error.set(None);
        set_rounding_step_error.set(None);
        set_amount_stepping_error.set(None);
        set_vendor_validation_regex_error.set(None);
        set_vendor_validation_min_error.set(None);
        set_vendor_validation_max_error.set(None);
        set_vendor_omission_error.set(None);

        let mut has_errors = false;
        let mut basic_errors = false;
        let mut validation_errors = false;
        let mut omission_errors = false;

        // Validate description
        let desc = description.get();
        if desc.trim().is_empty() {
            set_description_error.set(Some(description_required_msg()));
            has_errors = true;
            basic_errors = true;
        } else if desc.chars().count() > 200 {
            set_description_error.set(Some(description_length_msg()));
            has_errors = true;
            basic_errors = true;
        }

        let step = amount_stepping.get();
        if !step.trim().is_empty() {
            match parse_decimal_input(&step) {
                Ok(val) => {
                    if val <= Decimal::ZERO {
                        set_amount_stepping_error
                            .set(Some(t!("booth.form_errors.positive_number_required")()));
                        has_errors = true;
                        validation_errors = true;
                    }
                }
                Err(e) => {
                    let message = if e == DecimalInputParseError::TooManyDecimalPlaces {
                        max_two_decimals_msg()
                    } else {
                        invalid_number_format_msg()
                    };
                    set_amount_stepping_error.set(Some(message));
                    has_errors = true;
                    validation_errors = true;
                }
            }
        }

        // Validate date
        let date_str = date.get();
        if date_str.trim().is_empty() {
            set_date_error.set(Some(date_required_msg()));
            has_errors = true;
            basic_errors = true;
        }

        // Validate participation fee using flexible parsing (accepts both comma and dot)
        let part_fee = participation_fee.get();
        if part_fee.trim().is_empty() {
            set_participation_fee_error.set(Some(participation_fee_required_msg()));
            has_errors = true;
            basic_errors = true;
        } else {
            match parse_decimal_input(&part_fee) {
                Ok(val) => {
                    if val < Decimal::ZERO {
                        set_participation_fee_error.set(Some(cannot_be_negative_msg()));
                        has_errors = true;
                        basic_errors = true;
                    }
                }
                Err(e) => {
                    let message = if e == DecimalInputParseError::TooManyDecimalPlaces {
                        max_two_decimals_msg()
                    } else {
                        invalid_number_format_msg()
                    };
                    set_participation_fee_error.set(Some(message));
                    has_errors = true;
                    basic_errors = true;
                }
            }
        }

        // Validate revenue share percent using flexible parsing (accepts both comma and dot)
        let sales_pct = sales_fee_percent.get();
        if sales_pct.trim().is_empty() {
            set_sales_fee_percent_error.set(Some(sales_fee_required_msg()));
            has_errors = true;
            basic_errors = true;
        } else {
            match parse_decimal_input(&sales_pct) {
                Ok(val) => {
                    if val < Decimal::ZERO {
                        set_sales_fee_percent_error.set(Some(cannot_be_negative_msg()));
                        has_errors = true;
                        basic_errors = true;
                    } else if val > Decimal::from(100) {
                        set_sales_fee_percent_error.set(Some(cannot_exceed_100_msg()));
                        has_errors = true;
                        basic_errors = true;
                    }
                }
                Err(e) => {
                    let message = if e == DecimalInputParseError::TooManyDecimalPlaces {
                        max_two_decimals_msg()
                    } else {
                        invalid_number_format_msg()
                    };
                    set_sales_fee_percent_error.set(Some(message));
                    has_errors = true;
                    basic_errors = true;
                }
            }
        }

        // Validate rounding step using flexible parsing (accepts both comma and dot)
        let rounding = rounding_step.get();
        if rounding.trim().is_empty() {
            set_rounding_step_error.set(Some(rounding_step_required_msg()));
            has_errors = true;
            basic_errors = true;
        } else {
            match parse_decimal_input(&rounding) {
                Ok(val) => {
                    if val < Decimal::ZERO {
                        set_rounding_step_error.set(Some(cannot_be_negative_msg()));
                        has_errors = true;
                        basic_errors = true;
                    }
                }
                Err(e) => {
                    let message = if e == DecimalInputParseError::TooManyDecimalPlaces {
                        max_two_decimals_msg()
                    } else {
                        invalid_number_format_msg()
                    };
                    set_rounding_step_error.set(Some(message));
                    has_errors = true;
                    basic_errors = true;
                }
            }
        }

        // Validate vendor ID validation regex pattern if type is "regex"
        let validation_type = vendor_validation_type.get();
        if validation_type == "digits_only" {
            let (min_error, max_error) =
                digits_only_form_error_messages(validate_digits_only_form_fields(
                    &vendor_validation_min.get(),
                    &vendor_validation_max.get(),
                ));

            if let Some(message) = min_error {
                set_vendor_validation_min_error.set(Some(message));
                has_errors = true;
                validation_errors = true;
            }

            if let Some(message) = max_error {
                set_vendor_validation_max_error.set(Some(message));
                has_errors = true;
                validation_errors = true;
            }
        } else if validation_type == "regex" {
            let regex_pattern = vendor_validation_regex.get();
            if regex_pattern.trim().is_empty() {
                set_vendor_validation_regex_error
                    .set(Some(t!("booth.form_errors.regex_pattern_required")()));
                has_errors = true;
                validation_errors = true;
            } else {
                // Validate the regex pattern
                if let Err(e) = validate_regex_pattern(&regex_pattern) {
                    set_vendor_validation_regex_error.set(Some(translate_domain_error(&e)));
                    has_errors = true;
                    validation_errors = true;
                }
            }
        }

        if let Err(err) = vendor_omission_rules.get().validate() {
            set_vendor_omission_error.set(Some(translate_domain_error(&err)));
            has_errors = true;
            omission_errors = true;
        }

        if has_errors {
            let current_tab = active_tab.get();
            let current_tab_has_errors = match current_tab {
                0 => basic_errors,
                1 => validation_errors,
                2 => omission_errors,
                _ => false,
            };

            if !current_tab_has_errors {
                if basic_errors {
                    active_tab.set(0);
                } else if validation_errors {
                    active_tab.set(1);
                } else if omission_errors {
                    active_tab.set(2);
                }
            }

            return;
        }

        let data = BoothFormData {
            description: description.get(),
            date: date.get(),
            participation_fee: participation_fee.get(),
            sales_fee_percent: sales_fee_percent.get(),
            rounding_step: rounding_step.get(),
            amount_stepping: amount_stepping.get(),
            vendor_validation_type: vendor_validation_type.get(),
            vendor_validation_regex: vendor_validation_regex.get(),
            vendor_validation_min: vendor_validation_min.get(),
            vendor_validation_max: vendor_validation_max.get(),
            vendor_omission_rules: vendor_omission_rules.get(),
        };
        on_submit(data);
    };

    view! {
        <form
            id=form_id
            class="space-y-0"
            on:submit=move |e| {
                e.prevent_default();
                validate_and_submit();
            }
        >
            <TabGroup
                tabs=vec![
                    TabItem {
                        id: "basic-settings".to_string(),
                        label: t!("booth.tabs.basic_settings")(),
                        has_error: basic_tab_has_errors,
                    },
                    TabItem {
                        id: "validation-rules".to_string(),
                        label: t!("booth.tabs.validation_rules")(),
                        has_error: validation_tab_has_errors,
                    },
                    TabItem {
                        id: "vendor-omissions".to_string(),
                        label: t!("booth.tabs.vendor_omissions")(),
                        has_error: omission_tab_has_errors,
                    },
                ]
                active_tab=active_tab
                children=Box::new(move |tab_index| {
                    match tab_index {
                        0 => view! {
                            <div class="space-y-6">
                                <div class="grid gap-4 md:grid-cols-2">
                                    <Input
                                        node_ref=description_input_ref
                                        autofocus=autofocus_description
                                        value=description
                                        label=t!("booth.description_label")()
                                        placeholder=t!("booth.description_placeholder")()
                                        required=true
                                        error=description_error
                                    />

                                    <Input
                                        value=date
                                        input_type=crate::components::InputType::Date
                                        label=t!("booth.date_label")()
                                        required=true
                                        error=date_error
                                    />
                                </div>

                                <div class="rounded-lg border border-gray-200 bg-gray-50 p-6">
                                    <h3 class="mb-4 text-lg font-semibold text-gray-900">
                                        {t!("booth.fee_configuration_title")()}
                                    </h3>

                                    <div class="space-y-4">
                                        <div class="grid gap-4 md:grid-cols-2">
                                            <NumberInput
                                                value=participation_fee
                                                label={
                                                    let locale_val = locale.get();
                                                    let currency = currency_symbol_for_label(locale_val);
                                                    t!("booth.participation_fee")()
                                                        .replace("{currency}", currency)
                                                }
                                                placeholder=t!("common.placeholders.decimal_zero")()
                                                required=true
                                                error=participation_fee_error
                                            />

                                            <NumberInput
                                                value=sales_fee_percent
                                                label=t!("booth.sales_fee_percent")()
                                                placeholder=t!("common.placeholders.decimal_zero")()
                                                required=true
                                                error=sales_fee_percent_error
                                            />
                                        </div>

                                        <div>
                                            <NumberInput
                                                value=rounding_step
                                                label=t!("booth.rounding_step")()
                                                placeholder=t!("common.placeholders.decimal_half")()
                                                required=true
                                                error=rounding_step_error
                                            />
                                            <p class="mt-1 text-sm text-gray-600">
                                                {t!("booth.rounding_step_help")()}
                                            </p>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }
                        .into_view(),
                        1 => view! {
                            <div class="space-y-6">
                                <div class="rounded-lg border border-gray-200 bg-gray-50 p-6">
                                    <h3 class="mb-4 text-lg font-semibold text-gray-900">
                                        {t!("booth.amount_validation_title")()}
                                    </h3>

                                    <div>
                                        <NumberInput
                                            value=amount_stepping
                                            label=t!("booth.amount_stepping")()
                                            placeholder=t!("common.placeholders.decimal_half")()
                                            error=amount_stepping_error
                                        />
                                        <p class="mt-1 text-sm text-gray-600">
                                            {t!("booth.amount_stepping_help")()}
                                        </p>
                                    </div>
                                </div>

                                <div class="rounded-lg border border-gray-200 bg-gray-50 p-6">
                                    <h3 class="mb-4 text-lg font-semibold text-gray-900">
                                        {t!("booth.vendor_validation_title")()}
                                    </h3>
                                    <p class="mb-4 text-sm text-gray-600">
                                        {t!("booth.vendor_validation_description")()}
                                    </p>

                                    <div class="space-y-4">
                                        <div>
                                            <label class="mb-1 block text-sm font-medium text-gray-700">
                                                {t!("booth.vendor_validation_type_label")()}
                                            </label>
                                            <select
                                                class="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                                on:change=move |ev| {
                                                    vendor_validation_type.set(event_target_value(&ev));
                                                    set_vendor_validation_regex_error.set(None);
                                                    set_vendor_validation_min_error.set(None);
                                                    set_vendor_validation_max_error.set(None);
                                                }
                                                prop:value=vendor_validation_type
                                            >
                                                <option value="unrestricted" selected=move || vendor_validation_type.get() == "unrestricted">{t!("booth.vendor_validation_unrestricted")()}</option>
                                                <option value="digits_only" selected=move || vendor_validation_type.get() == "digits_only">{t!("booth.vendor_validation_digits_only")()}</option>
                                                <option value="regex" selected=move || vendor_validation_type.get() == "regex">{t!("booth.vendor_validation_regex")()}</option>
                                            </select>
                                        </div>

                                        {move || {
                                            if vendor_validation_type.get() == "digits_only" {
                                                view! {
                                                    <div class="grid gap-4 md:grid-cols-2">
                                                        <div>
                                                            <Input
                                                                value=vendor_validation_min
                                                                input_type=InputType::Number
                                                                label=t!("booth.vendor_validation_min_label")()
                                                                placeholder="1".to_string()
                                                                required=true
                                                                error=vendor_validation_min_error
                                                            />
                                                            <p class="mt-1 text-sm text-gray-600">
                                                                {t!("booth.vendor_validation_min_help")()}
                                                            </p>
                                                        </div>
                                                        <div>
                                                            <Input
                                                                value=vendor_validation_max
                                                                input_type=InputType::Number
                                                                label=t!("booth.vendor_validation_max_label")()
                                                                placeholder=t!("booth.vendor_validation_max_placeholder")()
                                                                error=vendor_validation_max_error
                                                            />
                                                            <p class="mt-1 text-sm text-gray-600">
                                                                {t!("booth.vendor_validation_max_help")()}
                                                            </p>
                                                        </div>
                                                    </div>
                                                }
                                                .into_view()
                                            } else if vendor_validation_type.get() == "regex" {
                                                view! {
                                                    <div class="space-y-2">
                                                        <Input
                                                            value=vendor_validation_regex
                                                            label=t!("booth.vendor_validation_regex_pattern")()
                                                            placeholder=r"^V\d{3}$".to_string()
                                                            required=true
                                                            error=vendor_validation_regex_error
                                                        />
                                                        <p class="text-sm text-gray-600">
                                                            {t!("booth.vendor_validation_regex_help")()}
                                                        </p>
                                                    </div>
                                                }
                                                .into_view()
                                            } else {
                                                view! { <div></div> }.into_view()
                                            }
                                        }}
                                    </div>
                                </div>
                            </div>
                        }
                        .into_view(),
                        2 => view! {
                            <div class="rounded-lg border border-gray-200 bg-gray-50 p-6">
                                <h3 class="mb-4 text-lg font-semibold text-gray-900">
                                    {t!("booth.vendor_omission_title")()}
                                </h3>
                                <p class="mb-4 text-sm text-gray-600">
                                    {t!("booth.vendor_omission_description")()}
                                </p>

                                <div class="space-y-4">
                                    <div>
                                        <label class="mb-1 block text-sm font-medium text-gray-700">
                                            {t!("booth.vendor_omission_type_label")()}
                                        </label>
                                        <select
                                            class="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                            on:change=move |ev| {
                                                new_omission_rule_type.set(event_target_value(&ev));
                                                set_vendor_omission_error.set(None);
                                            }
                                            prop:value=new_omission_rule_type
                                        >
                                            <option value="exact">{t!("booth.vendor_omission_type_exact")()}</option>
                                            <option value="wildcard">{t!("booth.vendor_omission_type_wildcard")()}</option>
                                            <option value="regex">{t!("booth.vendor_omission_type_regex")()}</option>
                                            <option value="range">{t!("booth.vendor_omission_type_range")()}</option>
                                        </select>
                                    </div>

                                    {move || {
                                        match new_omission_rule_type.get().as_str() {
                                            "exact" => view! {
                                                <Input
                                                    value=new_omission_value
                                                    label=t!("booth.vendor_omission_value_label")()
                                                    placeholder=t!("booth.vendor_omission_value_placeholder")()
                                                />
                                            }
                                            .into_view(),
                                            "wildcard" => view! {
                                                <Input
                                                    value=new_omission_pattern
                                                    label=t!("booth.vendor_omission_pattern_label")()
                                                    placeholder=t!("booth.vendor_omission_pattern_wildcard_placeholder")()
                                                />
                                            }
                                            .into_view(),
                                            "regex" => view! {
                                                <Input
                                                    value=new_omission_pattern
                                                    label=t!("booth.vendor_omission_pattern_label")()
                                                    placeholder=t!("booth.vendor_omission_pattern_regex_placeholder")()
                                                />
                                            }
                                            .into_view(),
                                            "range" => view! {
                                                <div class="grid gap-4 md:grid-cols-3">
                                                    <Input
                                                        value=new_omission_range_start
                                                        label=t!("booth.vendor_omission_start_label")()
                                                        placeholder="56".to_string()
                                                    />
                                                    <Input
                                                        value=new_omission_range_end
                                                        label=t!("booth.vendor_omission_end_label")()
                                                        placeholder="182".to_string()
                                                    />
                                                    <Input
                                                        value=new_omission_range_step
                                                        label=t!("booth.vendor_omission_step_label")()
                                                        placeholder="6".to_string()
                                                    />
                                                </div>
                                            }
                                            .into_view(),
                                            _ => view! {
                                                <Input
                                                    value=new_omission_value
                                                    label=t!("booth.vendor_omission_value_label")()
                                                    placeholder=t!("booth.vendor_omission_value_placeholder")()
                                                />
                                            }
                                            .into_view(),
                                        }
                                    }}

                                    <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                                        <p class="text-sm text-gray-600">{move || omission_help_text.get()}</p>
                                        <Button
                                            on_click=Box::new(move || {
                                                set_vendor_omission_error.set(None);

                                                let next_rules = match new_omission_rule_type.get().as_str() {
                                                    "exact" => {
                                                        let values = parse_exact_omission_values(&new_omission_value.get());
                                                        if values.is_empty() {
                                                            set_vendor_omission_error.set(Some(
                                                                t!("booth.form_errors.vendor_omission_value_required")(),
                                                            ));
                                                            return;
                                                        }
                                                        values
                                                            .into_iter()
                                                            .map(OmissionRule::Exact)
                                                            .collect::<Vec<_>>()
                                                    }
                                                    "wildcard" => {
                                                        let pattern = new_omission_pattern.get().trim().to_string();
                                                        if pattern.is_empty() {
                                                            set_vendor_omission_error.set(Some(
                                                                t!("booth.form_errors.vendor_omission_pattern_required")(),
                                                            ));
                                                            return;
                                                        }
                                                        if pattern.len() > 100 {
                                                            set_vendor_omission_error.set(Some(
                                                                t!("booth.form_errors.vendor_omission_pattern_too_long")(),
                                                            ));
                                                            return;
                                                        }
                                                        vec![OmissionRule::Wildcard(pattern.into())]
                                                    }
                                                    "regex" => {
                                                        let pattern = new_omission_pattern.get().trim().to_string();
                                                        if let Err(err) = validate_regex_pattern(&pattern) {
                                                            set_vendor_omission_error
                                                                .set(Some(translate_domain_error(&err)));
                                                            return;
                                                        }
                                                        vec![OmissionRule::Regex(pattern.into())]
                                                    }
                                                    "range" => {
                                                        let Some(start) = parse_u32_input(&new_omission_range_start.get()) else {
                                                            set_vendor_omission_error.set(Some(
                                                                t!("booth.form_errors.vendor_omission_number_required")(),
                                                            ));
                                                            return;
                                                        };
                                                        let Some(end) = parse_u32_input(&new_omission_range_end.get()) else {
                                                            set_vendor_omission_error.set(Some(
                                                                t!("booth.form_errors.vendor_omission_number_required")(),
                                                            ));
                                                            return;
                                                        };
                                                        if start > end {
                                                            set_vendor_omission_error.set(Some(
                                                                t!("booth.form_errors.vendor_omission_range_invalid")(),
                                                            ));
                                                            return;
                                                        }
                                                        let step_input = new_omission_range_step.get();
                                                        let step_input = step_input.trim();

                                                        if step_input.is_empty() {
                                                            vec![OmissionRule::Range { start, end }]
                                                        } else {
                                                            let Some(step) = parse_u32_input(step_input) else {
                                                                set_vendor_omission_error.set(Some(
                                                                    t!("booth.form_errors.vendor_omission_step_required")(),
                                                                ));
                                                                return;
                                                            };
                                                            if step == 0 {
                                                                set_vendor_omission_error.set(Some(
                                                                    t!("booth.form_errors.vendor_omission_step_invalid")(),
                                                                ));
                                                                return;
                                                            }
                                                            vec![OmissionRule::RangeWithStep { start, end, step }]
                                                        }
                                                    }
                                                    _ => {
                                                        let values = parse_exact_omission_values(&new_omission_value.get());
                                                        if values.is_empty() {
                                                            set_vendor_omission_error.set(Some(
                                                                t!("booth.form_errors.vendor_omission_value_required")(),
                                                            ));
                                                            return;
                                                        }
                                                        values
                                                            .into_iter()
                                                            .map(OmissionRule::Exact)
                                                            .collect::<Vec<_>>()
                                                    }
                                                };

                                                let mut updated_rules = vendor_omission_rules.get();
                                                updated_rules.rules.extend(next_rules);

                                                if let Err(err) = updated_rules.validate() {
                                                    set_vendor_omission_error
                                                        .set(Some(translate_domain_error(&err)));
                                                    return;
                                                }

                                                vendor_omission_rules.set(updated_rules);
                                                new_omission_value.set(String::new());
                                                new_omission_pattern.set(String::new());
                                                new_omission_range_start.set(String::new());
                                                new_omission_range_end.set(String::new());
                                                new_omission_range_step.set(String::new());
                                            })
                                            variant=crate::components::ButtonVariant::Secondary
                                        >
                                            {t!("booth.vendor_omission_add_rule")()}
                                        </Button>
                                    </div>

                                    <Show when=move || vendor_omission_error.get().is_some()>
                                        <p class="text-sm text-red-600">
                                            {move || vendor_omission_error.get().unwrap_or_default()}
                                        </p>
                                    </Show>

                                    <div class="space-y-2">
                                        <Show
                                            when=move || !vendor_omission_rules.get().rules.is_empty()
                                            fallback=move || view! {
                                                <p class="text-sm text-gray-500">{t!("booth.vendor_omission_empty")()}</p>
                                            }
                                        >
                                            <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                                                <For
                                                    each=move || {
                                                        vendor_omission_rules
                                                            .get()
                                                            .rules
                                                            .into_iter()
                                                            .enumerate()
                                                            .map(|(index, rule)| {
                                                                let key = omission_rule_key(index, &rule);
                                                                (index, key, rule)
                                                            })
                                                    }
                                                    key=|(_, key, _)| key.clone()
                                                    children=move |(_, rule_key, rule)| {
                                                        let type_label = omission_rule_type_label(&rule);
                                                        let value_label = omission_rule_value(&rule);

                                                        view! {
                                                            <div class="flex h-full flex-col justify-between gap-3 rounded-xl border border-gray-200 bg-white p-4 shadow-sm transition-shadow hover:shadow-md">
                                                                <div class="min-h-[4.5rem] space-y-1">
                                                                    <p class="text-xs font-semibold uppercase tracking-[0.12em] text-gray-500">
                                                                        {type_label}
                                                                    </p>
                                                                    <p
                                                                        class="text-sm font-medium text-gray-800"
                                                                        title=value_label.clone()
                                                                        style="display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; overflow: hidden;"
                                                                    >
                                                                        {value_label}
                                                                    </p>
                                                                </div>

                                                                <button
                                                                    type="button"
                                                                    class="w-full rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm font-medium text-red-700 transition-colors hover:border-red-300 hover:bg-red-100 hover:text-red-800"
                                                                    on:click=move |_| {
                                                                        vendor_omission_rules.update(|rules| {
                                                                            if let Some(index) = rules
                                                                                .rules
                                                                                .iter()
                                                                                .enumerate()
                                                                                .find_map(|(index, existing_rule)| {
                                                                                    (omission_rule_key(index, existing_rule) == rule_key)
                                                                                        .then_some(index)
                                                                                })
                                                                            {
                                                                                rules.rules.remove(index);
                                                                            }
                                                                        });
                                                                    }
                                                                >
                                                                    {t!("common.delete")()}
                                                                </button>
                                                            </div>
                                                        }
                                                    }
                                                />
                                            </div>
                                        </Show>
                                    </div>
                                </div>
                            </div>
                        }
                        .into_view(),
                        _ => view! { <div></div> }.into_view(),
                    }
                })
            />
        </form>
    }
}

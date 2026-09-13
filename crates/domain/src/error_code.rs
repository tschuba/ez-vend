use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationError {
    VendorIdEmpty,
    VendorIdTooShort { min: usize, actual: usize },
    VendorIdTooLong { max: usize, actual: usize },
    VendorIdValueTooSmall { min: usize, actual: String },
    VendorIdValueTooLarge { max: usize, actual: String },
    VendorIdDigitsOnly,
    VendorIdLeadingZeros,
    VendorIdPatternMismatch { value: String },
    VendorIdInvalidRegex,
    VendorIdOmitted { value: String },
    VendorOmissionPatternTooLong,
    VendorOmissionRangeInvalid,
    VendorOmissionStepInvalid,
    VendorOmissionRulesTooMany,
    BoothNameEmpty,
    BoothNameTooLong,
    BoothDuplicateNameAndDate { description: String, date: String },
    BoothArchived,
    BoothAlreadyArchived,
    BoothNotArchived,
    ParticipationFeeNegative,
    SalesFeePercentNegative,
    SalesFeePercentTooLarge,
    RoundingStepNegative,
    PurchaseAmountNegative,
    PurchaseAmountTooLarge,
    PurchaseEmpty,
    PurchaseTotalTooLarge,
    ItemAmountNotPositive,
    ItemAmountTooManyDecimals,
    ItemAmountTooLarge,
    RegexPatternEmpty,
    RegexPatternTooLong,
    RegexPatternInvalid,
    DateInvalid,
    ParticipationFeeInvalid,
    SalesFeePercentInvalid,
    RoundingStepInvalid,
    AmountSteppingInvalid,
    AmountSteppingMismatch { step: String },
    QuickAmountInvalid { value: String },
    QuickAmountsEmpty,
    QuickAmountsNonPositive,
    DigitsOnlyMinMaxInvalid { min: usize, max: usize },
    DigitsOnlyConstraintInvalidNumber,
}

impl ValidationError {
    pub fn key(&self) -> &'static str {
        match self {
            Self::VendorIdEmpty => "validation.vendor_id_empty",
            Self::VendorIdTooShort { .. } => "validation.vendor_id_too_short",
            Self::VendorIdTooLong { .. } => "validation.vendor_id_too_long",
            Self::VendorIdValueTooSmall { .. } => "validation.vendor_id_value_too_small",
            Self::VendorIdValueTooLarge { .. } => "validation.vendor_id_value_too_large",
            Self::VendorIdDigitsOnly => "validation.vendor_id_digits_only",
            Self::VendorIdLeadingZeros => "validation.vendor_id_leading_zeros",
            Self::VendorIdPatternMismatch { .. } => "validation.vendor_id_pattern_mismatch",
            Self::VendorIdInvalidRegex => "validation.vendor_id_invalid_regex",
            Self::VendorIdOmitted { .. } => "validation.vendor_id_omitted",
            Self::VendorOmissionPatternTooLong => "validation.vendor_omission_pattern_too_long",
            Self::VendorOmissionRangeInvalid => "validation.vendor_omission_range_invalid",
            Self::VendorOmissionStepInvalid => "validation.vendor_omission_step_invalid",
            Self::VendorOmissionRulesTooMany => "validation.vendor_omission_rules_too_many",
            Self::BoothNameEmpty => "validation.booth_name_empty",
            Self::BoothNameTooLong => "validation.booth_name_too_long",
            Self::BoothDuplicateNameAndDate { .. } => "validation.booth_duplicate_name_and_date",
            Self::BoothArchived => "validation.booth_archived",
            Self::BoothAlreadyArchived => "validation.booth_already_archived",
            Self::BoothNotArchived => "validation.booth_not_archived",
            Self::ParticipationFeeNegative => "validation.participation_fee_negative",
            Self::SalesFeePercentNegative => "validation.sales_fee_percent_negative",
            Self::SalesFeePercentTooLarge => "validation.sales_fee_percent_too_large",
            Self::RoundingStepNegative => "validation.rounding_step_negative",
            Self::PurchaseAmountNegative => "validation.purchase_amount_negative",
            Self::PurchaseAmountTooLarge => "validation.purchase_amount_too_large",
            Self::PurchaseEmpty => "validation.purchase_empty",
            Self::PurchaseTotalTooLarge => "validation.purchase_total_too_large",
            Self::ItemAmountNotPositive => "validation.item_amount_not_positive",
            Self::ItemAmountTooManyDecimals => "validation.item_amount_too_many_decimals",
            Self::ItemAmountTooLarge => "validation.item_amount_too_large",
            Self::RegexPatternEmpty => "validation.regex_pattern_empty",
            Self::RegexPatternTooLong => "validation.regex_pattern_too_long",
            Self::RegexPatternInvalid => "validation.regex_pattern_invalid",
            Self::DateInvalid => "validation.date_invalid",
            Self::ParticipationFeeInvalid => "validation.participation_fee_invalid",
            Self::SalesFeePercentInvalid => "validation.sales_fee_percent_invalid",
            Self::RoundingStepInvalid => "validation.rounding_step_invalid",
            Self::AmountSteppingInvalid => "validation.amount_stepping_invalid",
            Self::AmountSteppingMismatch { .. } => "validation.amount_stepping_mismatch",
            Self::QuickAmountInvalid { .. } => "validation.quick_amount_invalid",
            Self::QuickAmountsEmpty => "validation.quick_amounts_empty",
            Self::QuickAmountsNonPositive => "validation.quick_amounts_non_positive",
            Self::DigitsOnlyMinMaxInvalid { .. } => "validation.digits_only_min_max_invalid",
            Self::DigitsOnlyConstraintInvalidNumber => {
                "validation.digits_only_constraint_invalid_number"
            }
        }
    }

    pub fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::VendorIdTooShort { min, actual } => {
                vec![("min", min.to_string()), ("actual", actual.to_string())]
            }
            Self::VendorIdTooLong { max, actual } => {
                vec![("max", max.to_string()), ("actual", actual.to_string())]
            }
            Self::VendorIdValueTooSmall { min, actual } => {
                vec![("min", min.to_string()), ("actual", actual.clone())]
            }
            Self::VendorIdValueTooLarge { max, actual } => {
                vec![("max", max.to_string()), ("actual", actual.clone())]
            }
            Self::VendorIdPatternMismatch { value } => vec![("value", value.clone())],
            Self::VendorIdOmitted { value } => vec![("value", value.clone())],
            Self::BoothDuplicateNameAndDate { description, date } => {
                vec![("description", description.clone()), ("date", date.clone())]
            }
            Self::QuickAmountInvalid { value } => vec![("value", value.clone())],
            Self::AmountSteppingMismatch { step } => vec![("step", step.clone())],
            Self::DigitsOnlyMinMaxInvalid { min, max } => {
                vec![("min", min.to_string()), ("max", max.to_string())]
            }
            _ => Vec::new(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

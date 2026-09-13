use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

use super::shared::{BoothId, VendorId};
use crate::error::DomainError;
use crate::error_code::ValidationError;
use crate::validation::{
    validate_amount_stepping,
    vendor_id::{build_safe_regex, validate_digits_only_constraints},
};

const OMISSION_RULES_VERSION: u8 = 1;
const MAX_OMISSION_RULES: usize = 100;
const MAX_WILDCARD_PATTERN_LEN: usize = 100;

fn serialize_decimal_as_string<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.normalize().to_string())
}

fn deserialize_decimal_from_string<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    value.parse().map_err(D::Error::custom)
}

fn default_vendor_id_digits_min() -> usize {
    1
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CachedPattern {
    value: String,
    #[serde(skip, default)]
    compiled: Arc<OnceLock<Result<regex::Regex, ValidationError>>>,
}

impl CachedPattern {
    pub fn new(value: String) -> Self {
        Self {
            value,
            compiled: Arc::default(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    fn get_or_compile<F>(&self, builder: F) -> Result<&regex::Regex, DomainError>
    where
        F: FnOnce(&str) -> Result<regex::Regex, ValidationError>,
    {
        match self.compiled.get_or_init(|| builder(&self.value)) {
            Ok(regex) => Ok(regex),
            Err(err) => Err(DomainError::Validation(err.clone())),
        }
    }
}

impl Clone for CachedPattern {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            compiled: Arc::clone(&self.compiled),
        }
    }
}

impl PartialEq for CachedPattern {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for CachedPattern {}

impl From<String> for CachedPattern {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CachedPattern {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl std::fmt::Display for CachedPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}

/// Rules for omitting specific vendor IDs during checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VendorIdOmissionRules {
    #[serde(default = "default_omission_rules_version")]
    pub version: u8,
    pub rules: Vec<OmissionRule>,
}

impl VendorIdOmissionRules {
    pub fn empty() -> Self {
        Self {
            version: OMISSION_RULES_VERSION,
            rules: Vec::new(),
        }
    }

    pub fn recommended() -> Self {
        Self {
            version: OMISSION_RULES_VERSION,
            rules: vec![OmissionRule::RangeWithStep {
                start: 56,
                end: 182,
                step: 6,
            }],
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.rules.len() > MAX_OMISSION_RULES {
            return Err(DomainError::Validation(
                ValidationError::VendorOmissionRulesTooMany,
            ));
        }

        for rule in &self.rules {
            rule.validate()?;
        }

        Ok(())
    }

    pub fn is_omitted(&self, vendor_id: &str) -> Result<bool, DomainError> {
        for rule in &self.rules {
            if rule.matches(vendor_id)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl Default for VendorIdOmissionRules {
    fn default() -> Self {
        Self::empty()
    }
}

const fn default_omission_rules_version() -> u8 {
    OMISSION_RULES_VERSION
}

/// A single omission rule for blocking vendor IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OmissionRule {
    Exact(String),
    Wildcard(CachedPattern),
    Regex(CachedPattern),
    Range { start: u32, end: u32 },
    RangeWithStep { start: u32, end: u32, step: u32 },
}

impl OmissionRule {
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::Exact(value) => {
                if value.trim().is_empty() {
                    return Err(DomainError::Validation(ValidationError::VendorIdEmpty));
                }
                if value.len() > 50 {
                    return Err(DomainError::Validation(ValidationError::VendorIdTooLong {
                        max: 50,
                        actual: value.len(),
                    }));
                }
                Ok(())
            }
            Self::Wildcard(pattern) => validate_wildcard_pattern(pattern.as_str()),
            Self::Regex(pattern) => {
                build_safe_regex(pattern.as_str()).map_err(DomainError::Validation)?;
                Ok(())
            }
            Self::Range { start, end } => {
                if start > end {
                    return Err(DomainError::Validation(
                        ValidationError::VendorOmissionRangeInvalid,
                    ));
                }
                Ok(())
            }
            Self::RangeWithStep { start, end, step } => {
                if start > end {
                    return Err(DomainError::Validation(
                        ValidationError::VendorOmissionRangeInvalid,
                    ));
                }
                if *step == 0 {
                    return Err(DomainError::Validation(
                        ValidationError::VendorOmissionStepInvalid,
                    ));
                }
                Ok(())
            }
        }
    }

    pub fn matches(&self, vendor_id: &str) -> Result<bool, DomainError> {
        if vendor_id.is_empty() {
            return Ok(false);
        }

        match self {
            Self::Exact(value) => Ok(vendor_id == value),
            Self::Wildcard(pattern) => wildcard_matches(pattern, vendor_id),
            Self::Regex(pattern) => Ok(pattern
                .get_or_compile(build_safe_regex)?
                .is_match(vendor_id)),
            // Range matching parses vendor IDs as u32 integers.
            // Leading zeros are stripped ("007" becomes 7), and values outside u32 do not match.
            Self::Range { start, end } => vendor_id
                .parse::<u32>()
                .ok()
                .map(|value| value >= *start && value <= *end)
                .map(Ok)
                .unwrap_or(Ok(false)),
            Self::RangeWithStep { start, end, step } => {
                if *step == 0 {
                    return Err(DomainError::Validation(
                        ValidationError::VendorOmissionStepInvalid,
                    ));
                }

                // Range-with-step matching parses vendor IDs as u32 integers.
                // Leading zeros are stripped ("007" becomes 7), and values outside u32 do not match.
                vendor_id
                    .parse::<u32>()
                    .ok()
                    .map(|value| {
                        value >= *start
                            && value <= *end
                            && value
                                .checked_sub(*start)
                                .map(|difference| difference % *step == 0)
                                .unwrap_or(false)
                    })
                    .map(Ok)
                    .unwrap_or(Ok(false))
            }
        }
    }
}

fn validate_wildcard_pattern(pattern: &str) -> Result<(), DomainError> {
    if pattern.is_empty() {
        return Err(DomainError::Validation(ValidationError::RegexPatternEmpty));
    }

    if pattern.len() > MAX_WILDCARD_PATTERN_LEN {
        return Err(DomainError::Validation(
            ValidationError::VendorOmissionPatternTooLong,
        ));
    }

    build_wildcard_regex(pattern).map_err(DomainError::Validation)?;
    Ok(())
}

fn build_wildcard_regex(pattern: &str) -> Result<regex::Regex, ValidationError> {
    let mut regex_pattern = String::with_capacity(pattern.len() + 2);
    regex_pattern.push('^');

    for ch in pattern.chars() {
        match ch {
            '*' => regex_pattern.push_str(".*"),
            '?' => regex_pattern.push('.'),
            _ => regex_pattern.push_str(&regex::escape(&ch.to_string())),
        }
    }

    regex_pattern.push('$');

    build_safe_regex(&regex_pattern)
}

// Wildcard matching uses regex under the hood.
// '*' matches any sequence of characters, and '?' matches exactly one Unicode character.
fn wildcard_matches(pattern: &CachedPattern, value: &str) -> Result<bool, DomainError> {
    Ok(pattern
        .get_or_compile(build_wildcard_regex)?
        .is_match(value))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct VendorIdDigitsOnlyPattern {
    #[serde(default = "default_vendor_id_digits_min")]
    min: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max: Option<usize>,
}

impl Default for VendorIdDigitsOnlyPattern {
    fn default() -> Self {
        Self {
            min: default_vendor_id_digits_min(),
            max: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "pattern")]
enum VendorIdValidationSerde {
    Unrestricted,
    DigitsOnly(VendorIdDigitsOnlyPattern),
    Regex(String),
}

/// Vendor ID validation rules for a booth
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorIdValidation {
    /// No restrictions on vendor ID format
    Unrestricted,
    /// Only ASCII digits (0-9) are allowed
    DigitsOnly { min: usize, max: Option<usize> },
    /// Custom regular expression pattern
    Regex(String),
}

impl Default for VendorIdValidation {
    fn default() -> Self {
        Self::DigitsOnly {
            min: default_vendor_id_digits_min(),
            max: None,
        }
    }
}

impl Serialize for VendorIdValidation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = match self {
            Self::Unrestricted => VendorIdValidationSerde::Unrestricted,
            Self::DigitsOnly { min, max } => {
                VendorIdValidationSerde::DigitsOnly(VendorIdDigitsOnlyPattern {
                    min: *min,
                    max: *max,
                })
            }
            Self::Regex(pattern) => VendorIdValidationSerde::Regex(pattern.clone()),
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VendorIdValidation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "type", content = "pattern")]
        enum Helper {
            Unrestricted,
            #[serde(alias = "digits_only")]
            DigitsOnly(Option<VendorIdDigitsOnlyPattern>),
            Regex(String),
        }

        match Helper::deserialize(deserializer)? {
            Helper::Unrestricted => Ok(Self::Unrestricted),
            Helper::DigitsOnly(pattern) => {
                let pattern = pattern.unwrap_or_default();
                validate_digits_only_constraints(pattern.min, pattern.max)
                    .map_err(serde::de::Error::custom)?;
                Ok(Self::DigitsOnly {
                    min: pattern.min,
                    max: pattern.max,
                })
            }
            Helper::Regex(pattern) => Ok(Self::Regex(pattern)),
        }
    }
}

/// Represents a bazaar booth/event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Booth {
    pub id: BoothId,

    pub description: String,

    pub date: NaiveDate,

    pub fees: FeeConfig,

    #[serde(default)]
    pub vendor_id_validation: VendorIdValidation,

    #[serde(default)]
    pub vendor_id_omission_rules: VendorIdOmissionRules,

    #[serde(default)]
    pub keyboard_config: CheckoutKeyboardConfig,

    #[serde(default)]
    pub amount_stepping: Option<Decimal>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<DateTime<Utc>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_summary: Option<ArchivedBoothSummary>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchivedBoothSummary {
    #[serde(
        serialize_with = "serialize_decimal_as_string",
        deserialize_with = "deserialize_decimal_from_string"
    )]
    pub total_revenue: Decimal,
    #[serde(
        serialize_with = "serialize_decimal_as_string",
        deserialize_with = "deserialize_decimal_from_string"
    )]
    pub total_booth_revenue: Decimal,
    pub vendor_count: usize,
    pub purchase_count: usize,
    pub item_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_purchase_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_purchase_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub vendor_summaries: Vec<ArchivedVendorSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchivedVendorSummary {
    pub vendor_id: VendorId,
    #[serde(
        serialize_with = "serialize_decimal_as_string",
        deserialize_with = "deserialize_decimal_from_string"
    )]
    pub gross_sales: Decimal,
    #[serde(
        serialize_with = "serialize_decimal_as_string",
        deserialize_with = "deserialize_decimal_from_string"
    )]
    pub fees_due: Decimal,
    #[serde(
        serialize_with = "serialize_decimal_as_string",
        deserialize_with = "deserialize_decimal_from_string"
    )]
    pub net_payout: Decimal,
    pub item_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckoutKeyboardConfig {
    pub quick_amounts: Vec<Decimal>,
}

impl Default for CheckoutKeyboardConfig {
    fn default() -> Self {
        Self {
            quick_amounts: vec![
                Decimal::new(5, 1),
                Decimal::new(1, 0),
                Decimal::new(5, 0),
                Decimal::new(10, 0),
                Decimal::new(15, 0),
            ],
        }
    }
}

/// Fee configuration for booth charges
///
/// Note: The validator crate doesn't support range validation for rust_decimal::Decimal.
/// Range validation is performed via the `validate_ranges()` method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeConfig {
    /// Fixed participation fee per vendor
    pub participation_fee: Decimal,

    /// Sales commission percentage (0-100)
    pub sales_fee_percent: Decimal,

    /// Rounding step for net payout calculations (vendors receive rounded payouts; fees adjust accordingly)
    pub rounding_step: Decimal,
}

impl FeeConfig {
    /// Validate that all fee values are within acceptable ranges
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - Any fee value is negative
    /// - Sales fee percent is greater than 100
    pub fn validate_ranges(&self) -> Result<(), DomainError> {
        if self.participation_fee.is_sign_negative() {
            return Err(DomainError::Validation(
                ValidationError::ParticipationFeeNegative,
            ));
        }

        if self.sales_fee_percent.is_sign_negative() {
            return Err(DomainError::Validation(
                ValidationError::SalesFeePercentNegative,
            ));
        }

        if self.sales_fee_percent > Decimal::new(100, 0) {
            return Err(DomainError::Validation(
                ValidationError::SalesFeePercentTooLarge,
            ));
        }

        if self.rounding_step.is_sign_negative() {
            return Err(DomainError::Validation(
                ValidationError::RoundingStepNegative,
            ));
        }

        Ok(())
    }
}

impl Booth {
    /// Create a new booth with the given configuration
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - Description is empty or longer than 200 characters
    /// - Fee configuration is invalid
    pub fn new(description: String, date: NaiveDate, fees: FeeConfig) -> Result<Self, DomainError> {
        let description = description.trim().to_string();

        if description.is_empty() {
            return Err(DomainError::Validation(ValidationError::BoothNameEmpty));
        }
        if description.chars().count() > 200 {
            return Err(DomainError::Validation(ValidationError::BoothNameTooLong));
        }

        // Validate fee configuration
        fees.validate_ranges()?;

        // Validate optional amount stepping configuration
        let amount_stepping = None;

        let now = Utc::now();
        let booth = Self {
            id: BoothId::new(),
            description,
            date,
            fees,
            vendor_id_validation: VendorIdValidation::default(),
            vendor_id_omission_rules: VendorIdOmissionRules::empty(),
            keyboard_config: CheckoutKeyboardConfig::default(),
            amount_stepping,
            archived_at: None,
            archived_summary: None,
            created_at: now,
            updated_at: now,
        };

        Ok(booth)
    }

    pub fn update_description(&mut self, description: String) {
        self.description = description.trim().to_string();
        self.updated_at = Utc::now();
    }

    pub fn update_fees(&mut self, fees: FeeConfig) {
        self.fees = fees;
        self.updated_at = Utc::now();
    }

    pub fn update_keyboard_config(&mut self, keyboard_config: CheckoutKeyboardConfig) {
        self.keyboard_config = keyboard_config;
        self.updated_at = Utc::now();
    }

    pub fn update_amount_stepping(
        &mut self,
        amount_stepping: Option<Decimal>,
    ) -> Result<(), DomainError> {
        if let Some(step) = amount_stepping {
            validate_amount_stepping(step)?;
            self.amount_stepping = Some(step);
        } else {
            self.amount_stepping = None;
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn archive(&mut self, summary: ArchivedBoothSummary) -> Result<(), DomainError> {
        if self.is_archived() {
            return Err(DomainError::Validation(
                ValidationError::BoothAlreadyArchived,
            ));
        }

        let now = Utc::now();
        self.archived_at = Some(now);
        self.archived_summary = Some(summary);
        self.updated_at = now;
        Ok(())
    }

    pub fn restore(&mut self) -> Result<(), DomainError> {
        if !self.is_archived() {
            return Err(DomainError::Validation(ValidationError::BoothNotArchived));
        }

        self.archived_at = None;
        self.archived_summary = None;
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// Summary statistics for a booth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoothSummary {
    pub booth_id: BoothId,
    pub total_revenue: Decimal,
    pub total_purchases: usize,
    pub total_items: usize,
    pub unique_vendors: usize,
    pub vendor_summaries: Vec<VendorBoothSummary>,
    /// Configured participation fee per vendor
    pub participation_fee: Decimal,
    /// Configured revenue share percentage
    pub sales_fee_percent: Decimal,
    /// Total participation fees collected from all vendors
    pub total_participation_fees: Decimal,
    /// Total revenue share collected from all vendors
    pub total_sales_fees: Decimal,
    /// Total booth revenue (participation fees + revenue share)
    pub total_booth_revenue: Decimal,
}

/// Per-vendor statistics within a booth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorBoothSummary {
    pub vendor_id: VendorId,
    pub gross_sales: Decimal,
    pub fees_due: Decimal,
    pub net_payout: Decimal,
    pub item_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VendorId;
    use serde_json::json;

    fn test_fees() -> FeeConfig {
        FeeConfig {
            participation_fee: Decimal::ZERO,
            sales_fee_percent: Decimal::ZERO,
            rounding_step: Decimal::ZERO,
        }
    }

    #[test]
    fn omission_rule_exact_matches() {
        assert!(OmissionRule::Exact("123".to_string())
            .matches("123")
            .unwrap());
        assert!(!OmissionRule::Exact("123".to_string())
            .matches("124")
            .unwrap());
    }

    #[test]
    fn omission_rule_wildcard_matches() {
        assert!(OmissionRule::Wildcard("12*".into())
            .matches("1234")
            .unwrap());
        assert!(OmissionRule::Wildcard("*34".into())
            .matches("1234")
            .unwrap());
        assert!(OmissionRule::Wildcard("1?34".into())
            .matches("1234")
            .unwrap());
        assert!(!OmissionRule::Wildcard("12*".into())
            .matches("9912")
            .unwrap());
    }

    #[test]
    fn omission_rule_regex_matches() {
        assert!(OmissionRule::Regex("^A\\d+$".into())
            .matches("A42")
            .unwrap());
        assert!(!OmissionRule::Regex("^A\\d+$".into())
            .matches("B42")
            .unwrap());
    }

    #[test]
    fn omission_rule_range_with_step_matches() {
        let rule = OmissionRule::RangeWithStep {
            start: 56,
            end: 182,
            step: 6,
        };

        assert!(rule.matches("56").unwrap());
        assert!(rule.matches("62").unwrap());
        assert!(rule.matches("182").unwrap());
        assert!(!rule.matches("60").unwrap());
        assert!(!rule.matches("183").unwrap());
    }

    #[test]
    fn recommended_omission_rules_match_expected_values() {
        let rules = VendorIdOmissionRules::recommended();

        assert!(rules.is_omitted("56").unwrap());
        assert!(rules.is_omitted("62").unwrap());
        assert!(rules.is_omitted("182").unwrap());
        assert!(!rules.is_omitted("55").unwrap());
        assert!(!rules.is_omitted("60").unwrap());
        assert!(!rules.is_omitted("183").unwrap());
    }

    #[test]
    fn default_omission_rules_are_empty_for_backward_compatibility() {
        let rules = VendorIdOmissionRules::default();

        assert!(rules.rules.is_empty());
        assert_eq!(rules.version, OMISSION_RULES_VERSION);
    }

    #[test]
    fn omission_rule_range_with_step_does_not_underflow_before_start() {
        let rule = OmissionRule::RangeWithStep {
            start: 56,
            end: 182,
            step: 6,
        };

        assert!(!rule.matches("54").unwrap());
    }

    #[test]
    fn omission_rules_do_not_match_empty_vendor_ids() {
        let rules = VendorIdOmissionRules {
            version: OMISSION_RULES_VERSION,
            rules: vec![
                OmissionRule::Exact("123".to_string()),
                OmissionRule::Wildcard("12*".into()),
                OmissionRule::Regex("^1\\d+$".into()),
                OmissionRule::Range { start: 1, end: 10 },
                OmissionRule::RangeWithStep {
                    start: 2,
                    end: 10,
                    step: 2,
                },
            ],
        };

        assert!(!rules.is_omitted("").unwrap());
    }

    #[test]
    fn omission_rules_reject_invalid_regex_patterns() {
        let rules = VendorIdOmissionRules {
            version: OMISSION_RULES_VERSION,
            rules: vec![OmissionRule::Regex("[invalid".into())],
        };

        assert!(matches!(
            rules.validate(),
            Err(DomainError::Validation(
                ValidationError::RegexPatternInvalid
            ))
        ));
    }

    #[test]
    fn omission_rules_reject_too_many_rules() {
        let rules = VendorIdOmissionRules {
            version: OMISSION_RULES_VERSION,
            rules: (0..=MAX_OMISSION_RULES)
                .map(|index| OmissionRule::Exact(index.to_string()))
                .collect(),
        };

        assert!(matches!(
            rules.validate(),
            Err(DomainError::Validation(
                ValidationError::VendorOmissionRulesTooMany,
            ))
        ));
    }

    #[test]
    fn booth_new_trims_description() {
        let booth = Booth::new(
            "  Spring Fair  ".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            test_fees(),
        )
        .unwrap();

        assert_eq!(booth.description, "Spring Fair");
    }

    #[test]
    fn booth_update_description_trims_whitespace() {
        let mut booth = Booth::new(
            "Spring Fair".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            test_fees(),
        )
        .unwrap();

        booth.update_description("  Updated Fair  ".to_string());

        assert_eq!(booth.description, "Updated Fair");
    }

    #[test]
    fn booth_new_limits_description_by_character_count() {
        let valid = Booth::new(
            "ä".repeat(200),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            test_fees(),
        );
        let invalid = Booth::new(
            "ä".repeat(201),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            test_fees(),
        );

        assert!(valid.is_ok());
        assert!(matches!(
            invalid,
            Err(DomainError::Validation(ValidationError::BoothNameTooLong))
        ));
    }

    #[test]
    fn booth_amount_stepping_defaults_to_none() {
        let booth = Booth::new(
            "Spring Fair".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            test_fees(),
        )
        .unwrap();

        assert_eq!(booth.amount_stepping, None);
    }

    #[test]
    fn booth_update_amount_stepping_validates_positive_values() {
        let mut booth = Booth::new(
            "Spring Fair".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            test_fees(),
        )
        .unwrap();

        assert!(booth
            .update_amount_stepping(Some(Decimal::new(5, 1)))
            .is_ok());
        assert_eq!(booth.amount_stepping, Some(Decimal::new(5, 1)));
        assert!(matches!(
            booth.update_amount_stepping(Some(Decimal::ZERO)),
            Err(DomainError::Validation(
                ValidationError::AmountSteppingInvalid
            ))
        ));
    }

    #[test]
    fn archived_booth_round_trips_archive_state() {
        let mut booth = Booth::new(
            "Spring Fair".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            test_fees(),
        )
        .unwrap();

        let summary = ArchivedBoothSummary {
            total_revenue: Decimal::new(100, 0),
            total_booth_revenue: Decimal::new(20, 0),
            vendor_count: 2,
            purchase_count: 3,
            item_count: 4,
            first_purchase_at: None,
            last_purchase_at: None,
            vendor_summaries: vec![ArchivedVendorSummary {
                vendor_id: VendorId::new("101".to_string()),
                gross_sales: Decimal::new(100, 0),
                fees_due: Decimal::new(20, 0),
                net_payout: Decimal::new(80, 0),
                item_count: 4,
            }],
        };

        booth.archive(summary.clone()).unwrap();
        assert!(booth.is_archived());
        assert_eq!(booth.archived_summary, Some(summary));

        booth.restore().unwrap();
        assert!(!booth.is_archived());
        assert!(booth.archived_summary.is_none());
    }

    #[test]
    fn archived_summary_decimal_fields_round_trip_as_strings() {
        let summary = ArchivedBoothSummary {
            total_revenue: Decimal::new(760, 1),
            total_booth_revenue: Decimal::new(170, 1),
            vendor_count: 5,
            purchase_count: 8,
            item_count: 10,
            first_purchase_at: None,
            last_purchase_at: None,
            vendor_summaries: vec![ArchivedVendorSummary {
                vendor_id: VendorId::new("1".to_string()),
                gross_sales: Decimal::new(660, 1),
                fees_due: Decimal::new(110, 1),
                net_payout: Decimal::new(550, 1),
                item_count: 6,
            }],
        };

        let serialized = serde_json::to_value(&summary).unwrap();
        assert_eq!(serialized["total_revenue"], json!("76"));
        assert_eq!(serialized["total_booth_revenue"], json!("17"));
        assert_eq!(
            serialized["vendor_summaries"][0]["gross_sales"],
            json!("66")
        );

        let round_tripped: ArchivedBoothSummary = serde_json::from_value(serialized).unwrap();
        assert_eq!(round_tripped, summary);
    }

    #[test]
    fn archive_rejects_duplicate_archive_calls() {
        let mut booth = Booth::new(
            "Spring Fair".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            test_fees(),
        )
        .unwrap();

        let summary = ArchivedBoothSummary {
            total_revenue: Decimal::ZERO,
            total_booth_revenue: Decimal::ZERO,
            vendor_count: 0,
            purchase_count: 0,
            item_count: 0,
            first_purchase_at: None,
            last_purchase_at: None,
            vendor_summaries: Vec::new(),
        };

        booth.archive(summary.clone()).unwrap();

        assert!(matches!(
            booth.archive(summary),
            Err(DomainError::Validation(
                ValidationError::BoothAlreadyArchived
            ))
        ));
    }

    #[test]
    fn vendor_id_validation_deserializes_legacy_digits_only_shape() {
        let value = json!({ "type": "DigitsOnly" });

        let validation: VendorIdValidation = serde_json::from_value(value).unwrap();

        assert_eq!(
            validation,
            VendorIdValidation::DigitsOnly { min: 1, max: None }
        );
    }

    #[test]
    fn vendor_id_validation_rejects_invalid_digits_only_bounds_during_deserialization() {
        let value = json!({
            "type": "DigitsOnly",
            "pattern": {
                "min": 10,
                "max": 5
            }
        });

        let error = serde_json::from_value::<VendorIdValidation>(value).unwrap_err();

        assert!(error
            .to_string()
            .contains("validation.digits_only_min_max_invalid"));
    }
}

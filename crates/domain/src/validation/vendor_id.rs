use crate::error::DomainError;
use crate::error_code::ValidationError;
use crate::models::VendorIdValidation;
use std::cmp::Ordering;

const SAFE_REGEX_SIZE_LIMIT: usize = 10 * (1 << 20);

/// Validate a vendor ID against the booth's validation rules
///
/// # Arguments
///
/// * `value` - The vendor ID string to validate
/// * `rule` - The validation rule to apply
///
/// # Errors
///
/// Returns `DomainError::Validation` if the vendor ID doesn't match the rule
pub fn validate_vendor_id(value: &str, rule: &VendorIdValidation) -> Result<(), DomainError> {
    match rule {
        VendorIdValidation::Unrestricted => Ok(()),
        VendorIdValidation::DigitsOnly { min, max } => {
            if value.is_empty() {
                return Err(DomainError::Validation(ValidationError::VendorIdEmpty));
            }

            if !value.chars().all(|c| c.is_ascii_digit()) {
                return Err(DomainError::Validation(ValidationError::VendorIdDigitsOnly));
            }

            if has_forbidden_leading_zero(value) {
                return Err(DomainError::Validation(
                    ValidationError::VendorIdLeadingZeros,
                ));
            }

            if compare_numeric_string_to_usize(value, *min).is_lt() {
                return Err(DomainError::Validation(
                    ValidationError::VendorIdValueTooSmall {
                        min: *min,
                        actual: value.to_string(),
                    },
                ));
            }

            if let Some(max) = max {
                if compare_numeric_string_to_usize(value, *max).is_gt() {
                    return Err(DomainError::Validation(
                        ValidationError::VendorIdValueTooLarge {
                            max: *max,
                            actual: value.to_string(),
                        },
                    ));
                }
            }

            Ok(())
        }
        VendorIdValidation::Regex(pattern) => {
            if value.is_empty() {
                return Err(DomainError::Validation(ValidationError::VendorIdEmpty));
            }
            match build_safe_regex(pattern) {
                Ok(re) => {
                    if re.is_match(value) {
                        Ok(())
                    } else {
                        Err(DomainError::Validation(
                            ValidationError::VendorIdPatternMismatch {
                                value: value.to_string(),
                            },
                        ))
                    }
                }
                Err(_) => Err(DomainError::Validation(
                    ValidationError::VendorIdInvalidRegex,
                )),
            }
        }
    }
}

/// Validate a regex pattern for vendor ID validation
///
/// This should be called when saving a booth with a regex validation rule
/// to ensure the pattern is valid before persisting it.
///
/// # Errors
///
/// Returns `DomainError::Validation` if the pattern is invalid
pub fn validate_regex_pattern(pattern: &str) -> Result<(), DomainError> {
    if pattern.is_empty() {
        return Err(DomainError::Validation(ValidationError::RegexPatternEmpty));
    }

    if pattern.len() > 256 {
        return Err(DomainError::Validation(
            ValidationError::RegexPatternTooLong,
        ));
    }

    build_safe_regex(pattern)
        .map_err(|_| DomainError::Validation(ValidationError::RegexPatternInvalid))?;

    Ok(())
}

pub fn validate_digits_only_constraints(min: usize, max: Option<usize>) -> Result<(), DomainError> {
    if let Some(max) = max {
        if min > max {
            return Err(DomainError::Validation(
                ValidationError::DigitsOnlyMinMaxInvalid { min, max },
            ));
        }
    }

    Ok(())
}

fn has_forbidden_leading_zero(value: &str) -> bool {
    value.len() > 1 && value.starts_with('0')
}

fn compare_numeric_string_to_usize(value: &str, bound: usize) -> Ordering {
    let bound = bound.to_string();

    value
        .len()
        .cmp(&bound.len())
        .then_with(|| value.cmp(bound.as_str()))
}

pub fn build_safe_regex(pattern: &str) -> Result<regex::Regex, ValidationError> {
    regex::RegexBuilder::new(pattern)
        .size_limit(SAFE_REGEX_SIZE_LIMIT)
        .build()
        .map_err(|_| ValidationError::RegexPatternInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unrestricted_allows_any() {
        let rule = VendorIdValidation::Unrestricted;
        assert!(validate_vendor_id("123", &rule).is_ok());
        assert!(validate_vendor_id("abc", &rule).is_ok());
        assert!(validate_vendor_id("V-123", &rule).is_ok());
        assert!(validate_vendor_id("", &rule).is_ok());
    }

    #[test]
    fn test_digits_only_accepts_digits() {
        let rule = VendorIdValidation::DigitsOnly { min: 1, max: None };
        assert!(validate_vendor_id("123", &rule).is_ok());
        assert!(validate_vendor_id("999999", &rule).is_ok());
    }

    #[test]
    fn test_digits_only_rejects_non_digits() {
        let rule = VendorIdValidation::DigitsOnly { min: 1, max: None };
        assert!(validate_vendor_id("12a", &rule).is_err());
        assert!(validate_vendor_id("V123", &rule).is_err());
        assert!(validate_vendor_id("12-3", &rule).is_err());
        assert!(validate_vendor_id("", &rule).is_err());
    }

    #[test]
    fn test_digits_only_rejects_leading_zeros() {
        let rule = VendorIdValidation::DigitsOnly { min: 1, max: None };
        assert!(matches!(
            validate_vendor_id("007", &rule),
            Err(DomainError::Validation(
                ValidationError::VendorIdLeadingZeros
            ))
        ));
    }

    #[test]
    fn test_digits_only_allows_zero_when_min_is_zero() {
        let rule = VendorIdValidation::DigitsOnly {
            min: 0,
            max: Some(10),
        };

        assert!(validate_vendor_id("0", &rule).is_ok());
    }

    #[test]
    fn test_digits_only_rejects_zero_when_min_is_one() {
        let rule = VendorIdValidation::DigitsOnly { min: 1, max: None };

        assert!(matches!(
            validate_vendor_id("0", &rule),
            Err(DomainError::Validation(ValidationError::VendorIdValueTooSmall {
                min: 1,
                actual,
            })) if actual == "0"
        ));
    }

    #[test]
    fn test_digits_only_rejects_value_too_small() {
        let rule = VendorIdValidation::DigitsOnly { min: 50, max: None };
        assert!(matches!(
            validate_vendor_id("49", &rule),
            Err(DomainError::Validation(ValidationError::VendorIdValueTooSmall {
                min: 50,
                actual,
            })) if actual == "49"
        ));
        assert!(validate_vendor_id("50", &rule).is_ok());
    }

    #[test]
    fn test_digits_only_rejects_value_too_large() {
        let rule = VendorIdValidation::DigitsOnly {
            min: 1,
            max: Some(200),
        };
        assert!(validate_vendor_id("200", &rule).is_ok());
        assert!(matches!(
            validate_vendor_id("201", &rule),
            Err(DomainError::Validation(ValidationError::VendorIdValueTooLarge {
                max: 200,
                actual,
            })) if actual == "201"
        ));
    }

    #[test]
    fn test_digits_only_with_min_and_max() {
        let rule = VendorIdValidation::DigitsOnly {
            min: 50,
            max: Some(200),
        };
        assert!(validate_vendor_id("50", &rule).is_ok());
        assert!(validate_vendor_id("123", &rule).is_ok());
        assert!(validate_vendor_id("200", &rule).is_ok());
        assert!(validate_vendor_id("49", &rule).is_err());
        assert!(validate_vendor_id("201", &rule).is_err());
    }

    #[test]
    fn test_digits_only_with_equal_min_and_max_requires_exact_value() {
        let rule = VendorIdValidation::DigitsOnly {
            min: 42,
            max: Some(42),
        };

        assert!(validate_vendor_id("42", &rule).is_ok());
        assert!(validate_vendor_id("41", &rule).is_err());
        assert!(validate_vendor_id("43", &rule).is_err());
    }

    #[test]
    fn test_digits_only_compares_large_numeric_strings_correctly() {
        let rule = VendorIdValidation::DigitsOnly {
            min: 999_999,
            max: Some(1_000_000),
        };

        assert!(validate_vendor_id("999999", &rule).is_ok());
        assert!(validate_vendor_id("1000000", &rule).is_ok());
        assert!(validate_vendor_id("1000001", &rule).is_err());
    }

    #[test]
    fn test_validate_digits_only_constraints_valid() {
        assert!(validate_digits_only_constraints(1, None).is_ok());
        assert!(validate_digits_only_constraints(1, Some(10)).is_ok());
        assert!(validate_digits_only_constraints(5, Some(5)).is_ok());
    }

    #[test]
    fn test_validate_digits_only_constraints_invalid() {
        assert!(validate_digits_only_constraints(10, Some(5)).is_err());
    }

    #[test]
    fn test_regex_pattern_matching() {
        let rule = VendorIdValidation::Regex("^V\\d+$".to_string());
        assert!(validate_vendor_id("V123", &rule).is_ok());
        assert!(validate_vendor_id("V1", &rule).is_ok());
        assert!(validate_vendor_id("V999", &rule).is_ok());
    }

    #[test]
    fn test_regex_pattern_rejection() {
        let rule = VendorIdValidation::Regex("^V\\d+$".to_string());
        assert!(validate_vendor_id("123", &rule).is_err());
        assert!(validate_vendor_id("X123", &rule).is_err());
        assert!(validate_vendor_id("V", &rule).is_err());
        assert!(validate_vendor_id("", &rule).is_err());
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let rule = VendorIdValidation::Regex("[invalid".to_string());
        let result = validate_vendor_id("test", &rule);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_regex_pattern_valid() {
        assert!(validate_regex_pattern("^V\\d+$").is_ok());
        assert!(validate_regex_pattern("^[A-Z]\\d{3}$").is_ok());
        assert!(validate_regex_pattern("\\d+").is_ok());
    }

    #[test]
    fn test_validate_regex_pattern_invalid() {
        assert!(validate_regex_pattern("[invalid").is_err());
        assert!(validate_regex_pattern("").is_err());
    }

    #[test]
    fn test_validate_regex_pattern_too_long() {
        let long_pattern = "a".repeat(257);
        assert!(validate_regex_pattern(&long_pattern).is_err());
    }
}

use crate::error::{DomainError, DomainResult};
use crate::error_code::ValidationError;
use crate::models::*;
use rust_decimal::Decimal;

pub fn validate_amount_stepping(step: Decimal) -> DomainResult<()> {
    if step <= Decimal::ZERO {
        return Err(DomainError::Validation(
            ValidationError::AmountSteppingInvalid,
        ));
    }

    Ok(())
}

pub fn validate_amount_matches_step(amount: Decimal, step: Decimal) -> DomainResult<()> {
    validate_amount_stepping(step)?;

    if amount % step != Decimal::ZERO {
        return Err(DomainError::Validation(
            ValidationError::AmountSteppingMismatch {
                step: step.normalize().to_string(),
            },
        ));
    }

    Ok(())
}

/// Validation rules for booth entities
pub struct BoothValidator;

impl BoothValidator {
    pub fn validate_name(name: &str) -> DomainResult<()> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(ValidationError::BoothNameEmpty));
        }
        if name.chars().count() > 200 {
            return Err(DomainError::Validation(ValidationError::BoothNameTooLong));
        }
        Ok(())
    }
}

/// Validation rules for vendor entities
pub struct VendorValidator;

impl VendorValidator {
    pub fn validate_vendor_id(vendor_id: &VendorId) -> DomainResult<()> {
        if vendor_id.as_str().trim().is_empty() {
            return Err(DomainError::Validation(ValidationError::VendorIdEmpty));
        }
        if vendor_id.as_str().len() > 50 {
            return Err(DomainError::Validation(ValidationError::VendorIdTooLong {
                max: 50,
                actual: vendor_id.as_str().len(),
            }));
        }
        Ok(())
    }
}

/// Validation rules for purchase entities
pub struct PurchaseValidator;

impl PurchaseValidator {
    /// Validate that a purchase amount is within acceptable bounds
    ///
    /// # Errors
    ///
    /// Returns error if amount is negative or exceeds €1,000,000
    pub fn validate_amount(amount: &Decimal) -> DomainResult<()> {
        if amount.is_sign_negative() {
            return Err(DomainError::Validation(
                ValidationError::PurchaseAmountNegative,
            ));
        }

        // Maximum €1,000,000
        let max_amount = Decimal::new(1_000_000, 0);
        if *amount > max_amount {
            return Err(DomainError::Validation(
                ValidationError::PurchaseAmountTooLarge,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_booth_name_validation() {
        assert!(BoothValidator::validate_name("Valid Name").is_ok());
        assert!(BoothValidator::validate_name("").is_err());
        assert!(BoothValidator::validate_name("   ").is_err());
        assert!(BoothValidator::validate_name(&"x".repeat(201)).is_err());
        assert!(BoothValidator::validate_name(&"ä".repeat(200)).is_ok());
        assert!(BoothValidator::validate_name(&"ä".repeat(201)).is_err());
    }

    #[test]
    fn test_vendor_id_validation() {
        assert!(VendorValidator::validate_vendor_id(&VendorId::new("123".to_string())).is_ok());
        assert!(VendorValidator::validate_vendor_id(&VendorId::new("".to_string())).is_err());
    }

    #[test]
    fn test_purchase_amount_validation() {
        use rust_decimal_macros::dec;

        // Valid amounts
        assert!(PurchaseValidator::validate_amount(&dec!(10.0)).is_ok());
        assert!(PurchaseValidator::validate_amount(&Decimal::ZERO).is_ok());
        assert!(PurchaseValidator::validate_amount(&dec!(999999.99)).is_ok());

        // Invalid amounts
        assert!(PurchaseValidator::validate_amount(&dec!(-1.0)).is_err());
        assert!(PurchaseValidator::validate_amount(&dec!(-0.01)).is_err());
        assert!(PurchaseValidator::validate_amount(&Decimal::new(1_000_001, 0)).is_err());
    }

    #[test]
    fn amount_stepping_must_be_positive() {
        use rust_decimal_macros::dec;

        assert!(validate_amount_stepping(dec!(0)).is_err());
        assert!(validate_amount_stepping(dec!(-0.5)).is_err());
        assert!(validate_amount_stepping(dec!(0.5)).is_ok());
    }

    #[test]
    fn amount_must_match_step() {
        use rust_decimal_macros::dec;

        assert!(validate_amount_matches_step(dec!(3.5), dec!(0.5)).is_ok());
        assert!(validate_amount_matches_step(dec!(102), dec!(0.5)).is_ok());
        assert!(validate_amount_matches_step(dec!(1.1), dec!(0.5)).is_err());
        assert!(validate_amount_matches_step(dec!(3.5), dec!(1)).is_err());
    }
}

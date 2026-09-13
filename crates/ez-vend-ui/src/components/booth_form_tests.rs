#[cfg(test)]
mod tests {
    use super::super::booth_form::{
        parse_digits_only_field, parse_exact_omission_values, sanitize_digits_only_field,
        validate_digits_only_form_fields, BoothFormData, DigitsOnlyFieldValidation,
    };
    use crate::i18n::Locale;
    use chrono::NaiveDate;
    use domain::error::DomainError;
    use domain::error_code::ValidationError;
    use domain::models::booth::{Booth, FeeConfig, OmissionRule, VendorIdOmissionRules};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn booth_form() -> BoothFormData {
        BoothFormData {
            description: "Test Booth".to_string(),
            date: "2026-03-25".to_string(),
            participation_fee: "10.00".to_string(),
            sales_fee_percent: "15.00".to_string(),
            rounding_step: "0.50".to_string(),
            amount_stepping: String::new(),
            vendor_validation_type: "digits_only".to_string(),
            vendor_validation_regex: String::new(),
            vendor_validation_min: "1".to_string(),
            vendor_validation_max: String::new(),
            vendor_omission_rules: default_rules(),
        }
    }

    fn default_rules() -> VendorIdOmissionRules {
        VendorIdOmissionRules::recommended()
    }

    fn empty_rules() -> VendorIdOmissionRules {
        VendorIdOmissionRules::default()
    }

    #[test]
    fn test_default_form_data() {
        let form = BoothFormData::default();

        let today = chrono::Local::now().date_naive();
        let expected_date = today.format("%Y-%m-%d").to_string();

        assert_eq!(form.description, "");
        assert_eq!(form.date, expected_date);
        assert_eq!(form.participation_fee, "1.00");
        assert_eq!(form.sales_fee_percent, "15.00");
        assert_eq!(form.rounding_step, "0.50");
        assert_eq!(form.amount_stepping, "");
        assert_eq!(form.vendor_omission_rules, default_rules());
    }

    #[test]
    fn test_to_booth_valid_data() {
        let form = booth_form();

        let booth = form.to_booth(Locale::En);
        assert!(booth.is_ok());

        let booth = booth.unwrap();
        assert_eq!(booth.description, "Test Booth");
        assert_eq!(booth.date, NaiveDate::from_ymd_opt(2026, 3, 25).unwrap());
        assert_eq!(
            booth.fees.participation_fee,
            Decimal::from_str("10.00").unwrap()
        );
        assert_eq!(
            booth.fees.sales_fee_percent,
            Decimal::from_str("15.00").unwrap()
        );
        assert_eq!(booth.fees.rounding_step, Decimal::from_str("0.50").unwrap());
        assert_eq!(booth.amount_stepping, None);
        assert_eq!(booth.vendor_id_omission_rules, default_rules());
    }

    #[test]
    fn test_booth_new_defaults_to_empty_omission_rules() {
        let fees = FeeConfig {
            participation_fee: Decimal::from_str("10.00").unwrap(),
            sales_fee_percent: Decimal::from_str("15.00").unwrap(),
            rounding_step: Decimal::from_str("0.50").unwrap(),
        };

        let booth = Booth::new(
            "Test Booth".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            fees,
        )
        .unwrap();

        assert_eq!(booth.vendor_id_omission_rules, empty_rules());
    }

    #[test]
    fn test_to_booth_invalid_date() {
        let form = BoothFormData {
            description: "Test Booth".to_string(),
            date: "invalid-date".to_string(),
            ..booth_form()
        };

        let result = form.to_booth(Locale::En);
        assert!(result.is_err());
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn test_to_booth_invalid_participation_fee() {
        let form = BoothFormData {
            participation_fee: "invalid".to_string(),
            ..booth_form()
        };

        let result = form.to_booth(Locale::En);
        assert!(result.is_err());
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn test_to_booth_negative_participation_fee() {
        let form = BoothFormData {
            participation_fee: "-10.00".to_string(),
            ..booth_form()
        };

        let result = form.to_booth(Locale::En);
        assert!(result.is_err());
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn test_to_booth_invalid_sales_fee_percent() {
        let form = BoothFormData {
            sales_fee_percent: "invalid".to_string(),
            ..booth_form()
        };

        let result = form.to_booth(Locale::En);
        assert!(result.is_err());
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn test_to_booth_sales_fee_percent_out_of_range() {
        let form = BoothFormData {
            sales_fee_percent: "150.00".to_string(),
            ..booth_form()
        };

        let result = form.to_booth(Locale::En);
        assert!(result.is_err());
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn test_to_booth_invalid_rounding_step() {
        let form = BoothFormData {
            rounding_step: "invalid".to_string(),
            ..booth_form()
        };

        let result = form.to_booth(Locale::En);
        assert!(result.is_err());
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn test_to_booth_empty_description() {
        let form = BoothFormData {
            description: "".to_string(),
            ..booth_form()
        };

        let result = form.to_booth(Locale::En);
        assert!(result.is_err());
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn test_to_booth_trims_description() {
        let form = BoothFormData {
            description: "  Test Booth  ".to_string(),
            ..booth_form()
        };

        let booth = form.to_booth(Locale::En).unwrap();
        assert_eq!(booth.description, "Test Booth");
    }

    #[test]
    fn test_to_booth_allows_multibyte_description_up_to_200_characters() {
        let form = BoothFormData {
            description: "ä".repeat(200),
            ..booth_form()
        };

        assert!(form.to_booth(Locale::En).is_ok());
    }

    #[test]
    fn test_to_booth_rejects_multibyte_description_over_200_characters() {
        let form = BoothFormData {
            description: "ä".repeat(201),
            ..booth_form()
        };

        assert!(matches!(
            form.to_booth(Locale::En),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn test_from_booth() {
        let fees = FeeConfig {
            participation_fee: Decimal::from_str("25.50").unwrap(),
            sales_fee_percent: Decimal::from_str("12.50").unwrap(),
            rounding_step: Decimal::from_str("0.10").unwrap(),
        };

        let booth = Booth::new(
            "Original Booth".to_string(),
            NaiveDate::from_ymd_opt(2026, 4, 15).unwrap(),
            fees,
        )
        .unwrap();

        let form = BoothFormData::from_booth(&booth, Locale::En);

        assert_eq!(form.description, "Original Booth");
        assert_eq!(form.date, "2026-04-15");
        assert_eq!(form.participation_fee, "25.50");
        assert_eq!(form.sales_fee_percent, "12.50");
        assert_eq!(form.rounding_step, "0.10");
        assert_eq!(form.amount_stepping, "");
        assert_eq!(form.vendor_omission_rules, booth.vendor_id_omission_rules);
    }

    #[test]
    fn test_from_booth_locale_formatting() {
        let fees = FeeConfig {
            participation_fee: Decimal::from_str("10.50").unwrap(),
            sales_fee_percent: Decimal::from_str("15.00").unwrap(),
            rounding_step: Decimal::from_str("0.50").unwrap(),
        };

        let booth = Booth::new(
            "Test Booth".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            fees,
        )
        .unwrap();

        let form_en = BoothFormData::from_booth(&booth, Locale::En);
        assert_eq!(form_en.participation_fee, "10.50");
        assert_eq!(form_en.sales_fee_percent, "15.00");
        assert_eq!(form_en.rounding_step, "0.50");
        assert_eq!(form_en.amount_stepping, "");

        let form_de = BoothFormData::from_booth(&booth, Locale::De);
        assert_eq!(form_de.participation_fee, "10,50");
        assert_eq!(form_de.sales_fee_percent, "15,00");
        assert_eq!(form_de.rounding_step, "0,50");
        assert_eq!(form_de.amount_stepping, "");
    }

    #[test]
    fn test_to_booth_flexible_parsing() {
        let form_dot = BoothFormData {
            participation_fee: "10.50".to_string(),
            sales_fee_percent: "15.00".to_string(),
            rounding_step: "0.50".to_string(),
            ..booth_form()
        };

        let booth_dot = form_dot.to_booth(Locale::En).unwrap();
        assert_eq!(
            booth_dot.fees.participation_fee,
            Decimal::from_str("10.50").unwrap()
        );
        assert_eq!(
            booth_dot.fees.sales_fee_percent,
            Decimal::from_str("15.00").unwrap()
        );
        assert_eq!(
            booth_dot.fees.rounding_step,
            Decimal::from_str("0.50").unwrap()
        );

        let form_comma = BoothFormData {
            participation_fee: "10,50".to_string(),
            sales_fee_percent: "15,00".to_string(),
            rounding_step: "0,50".to_string(),
            ..booth_form()
        };

        let booth_comma = form_comma.to_booth(Locale::De).unwrap();
        assert_eq!(
            booth_comma.fees.participation_fee,
            Decimal::from_str("10.50").unwrap()
        );
        assert_eq!(
            booth_comma.fees.sales_fee_percent,
            Decimal::from_str("15.00").unwrap()
        );
        assert_eq!(
            booth_comma.fees.rounding_step,
            Decimal::from_str("0.50").unwrap()
        );

        assert_eq!(
            booth_dot.fees.participation_fee,
            booth_comma.fees.participation_fee
        );
        assert_eq!(
            booth_dot.fees.sales_fee_percent,
            booth_comma.fees.sales_fee_percent
        );
        assert_eq!(booth_dot.fees.rounding_step, booth_comma.fees.rounding_step);
    }

    #[test]
    fn test_update_booth_valid() {
        let fees = FeeConfig {
            participation_fee: Decimal::from_str("10.00").unwrap(),
            sales_fee_percent: Decimal::from_str("10.00").unwrap(),
            rounding_step: Decimal::from_str("0.50").unwrap(),
        };

        let mut booth = Booth::new(
            "Original".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
            fees,
        )
        .unwrap();

        let custom_rules = VendorIdOmissionRules {
            version: 1,
            rules: vec![OmissionRule::Exact("999".to_string())],
        };

        let form = BoothFormData {
            description: "Updated Booth".to_string(),
            date: "2026-03-25".to_string(),
            participation_fee: "20.00".to_string(),
            sales_fee_percent: "15.00".to_string(),
            rounding_step: "1.00".to_string(),
            vendor_omission_rules: custom_rules.clone(),
            ..booth_form()
        };

        let result = form.update_booth(&mut booth, Locale::En);
        assert!(result.is_ok());

        assert_eq!(booth.description, "Updated Booth");
        assert_eq!(booth.date, NaiveDate::from_ymd_opt(2026, 3, 25).unwrap());
        assert_eq!(
            booth.fees.participation_fee,
            Decimal::from_str("20.00").unwrap()
        );
        assert_eq!(
            booth.fees.sales_fee_percent,
            Decimal::from_str("15.00").unwrap()
        );
        assert_eq!(booth.fees.rounding_step, Decimal::from_str("1.00").unwrap());
        assert_eq!(booth.amount_stepping, None);
        assert_eq!(booth.vendor_id_omission_rules, custom_rules);
    }

    #[test]
    fn test_to_booth_parses_amount_stepping() {
        let form = BoothFormData {
            amount_stepping: "0.50".to_string(),
            ..booth_form()
        };

        let booth = form.to_booth(Locale::En).unwrap();

        assert_eq!(
            booth.amount_stepping,
            Some(Decimal::from_str("0.50").unwrap())
        );
    }

    #[test]
    fn test_to_booth_rejects_non_positive_amount_stepping() {
        let form = BoothFormData {
            amount_stepping: "0.00".to_string(),
            ..booth_form()
        };

        assert!(matches!(
            form.to_booth(Locale::En),
            Err(DomainError::Validation(
                ValidationError::AmountSteppingInvalid
            ))
        ));
    }

    #[test]
    fn test_from_booth_formats_amount_stepping() {
        let mut booth = Booth::new(
            "Original Booth".to_string(),
            NaiveDate::from_ymd_opt(2026, 4, 15).unwrap(),
            FeeConfig {
                participation_fee: Decimal::from_str("25.50").unwrap(),
                sales_fee_percent: Decimal::from_str("12.50").unwrap(),
                rounding_step: Decimal::from_str("0.10").unwrap(),
            },
        )
        .unwrap();
        booth
            .update_amount_stepping(Some(Decimal::from_str("0.50").unwrap()))
            .unwrap();

        let form_en = BoothFormData::from_booth(&booth, Locale::En);
        let form_de = BoothFormData::from_booth(&booth, Locale::De);

        assert_eq!(form_en.amount_stepping, "0.50");
        assert_eq!(form_de.amount_stepping, "0,50");
    }

    #[test]
    fn test_update_booth_invalid_date() {
        let fees = FeeConfig {
            participation_fee: Decimal::from_str("10.00").unwrap(),
            sales_fee_percent: Decimal::from_str("10.00").unwrap(),
            rounding_step: Decimal::from_str("0.50").unwrap(),
        };

        let mut booth = Booth::new(
            "Original".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
            fees,
        )
        .unwrap();

        let form = BoothFormData {
            description: "Updated".to_string(),
            date: "invalid-date".to_string(),
            participation_fee: "20.00".to_string(),
            sales_fee_percent: "15.00".to_string(),
            rounding_step: "1.00".to_string(),
            ..booth_form()
        };

        let result = form.update_booth(&mut booth, Locale::En);
        assert!(result.is_err());
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn test_parse_exact_omission_values_splits_and_trims() {
        let values = parse_exact_omission_values(" 56, 62 ,68 , TEST ");

        assert_eq!(values, vec!["56", "62", "68", "TEST"]);
    }

    #[test]
    fn test_parse_exact_omission_values_deduplicates_preserving_order() {
        let values = parse_exact_omission_values("56, 56, 62, TEST, 62, TEST");

        assert_eq!(values, vec!["56", "62", "TEST"]);
    }

    #[test]
    fn test_parse_exact_omission_values_ignores_empty_entries() {
        let values = parse_exact_omission_values(" , 56, , , TEST ,, ");

        assert_eq!(values, vec!["56", "TEST"]);
    }

    #[test]
    fn test_parse_digits_only_field_requires_value_for_min() {
        assert!(matches!(
            parse_digits_only_field("", false),
            Err(DomainError::Validation(
                ValidationError::DigitsOnlyConstraintInvalidNumber
            ))
        ));
    }

    #[test]
    fn test_parse_digits_only_field_allows_empty_max() {
        assert_eq!(parse_digits_only_field("", true).unwrap(), None);
    }

    #[test]
    fn test_sanitize_digits_only_field_removes_non_digits() {
        assert_eq!(sanitize_digits_only_field("12e-3abc"), "123");
    }

    #[test]
    fn test_validate_digits_only_form_fields_rejects_empty_min() {
        assert_eq!(
            validate_digits_only_form_fields("", ""),
            Some(DigitsOnlyFieldValidation::MinInvalid)
        );
    }

    #[test]
    fn test_validate_digits_only_form_fields_rejects_invalid_max() {
        assert_eq!(
            validate_digits_only_form_fields("1", "abc"),
            Some(DigitsOnlyFieldValidation::MaxInvalid)
        );
    }

    #[test]
    fn test_validate_digits_only_form_fields_rejects_min_greater_than_max() {
        assert_eq!(
            validate_digits_only_form_fields("10", "5"),
            Some(DigitsOnlyFieldValidation::MinGreaterThanMax)
        );
    }

    #[test]
    fn test_validate_digits_only_form_fields_accepts_equal_bounds() {
        assert_eq!(validate_digits_only_form_fields("42", "42"), None);
    }

    #[test]
    fn test_validate_digits_only_form_fields_accepts_zero_min() {
        assert_eq!(validate_digits_only_form_fields("0", "10"), None);
    }

    #[test]
    fn test_to_booth_supports_exact_numeric_value_constraint() {
        let form = BoothFormData {
            vendor_validation_min: "42".to_string(),
            vendor_validation_max: "42".to_string(),
            ..booth_form()
        };

        let booth = form.to_booth(Locale::En).unwrap();

        assert_eq!(
            booth.vendor_id_validation,
            domain::models::booth::VendorIdValidation::DigitsOnly {
                min: 42,
                max: Some(42)
            }
        );
    }
}

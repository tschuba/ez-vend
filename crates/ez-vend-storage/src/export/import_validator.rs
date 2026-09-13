use std::collections::HashSet;

use domain::validation::{validate_digits_only_constraints, validate_regex_pattern};
use domain::{BoothId, Purchase, Vendor, VendorIdValidation};
use log::{error, info, warn};
use rust_decimal::Decimal;

use super::backup_format::{BackupData, BoothBackupData, BACKUP_FORMAT_VERSION};
use super::checksum::{verify_backup_checksum, verify_booth_checksum, ChecksumVerificationResult};
use super::error::{ImportError, ValidationFailure};

#[derive(Debug, Default, Clone, Copy)]
pub struct ImportValidator;

impl ImportValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_backup(&self, raw: &str) -> Result<BackupData, ImportError> {
        let data: BackupData =
            serde_json::from_str(raw).map_err(|err| ImportError::InvalidJson(err.to_string()))?;

        self.verify_full_checksum(&data)?;
        self.validate_version(data.version)?;
        self.validate_all_records(&data.booths, &data.vendors, &data.purchases)?;

        Ok(data)
    }

    pub fn validate_booth_backup(&self, raw: &str) -> Result<BoothBackupData, ImportError> {
        let data: BoothBackupData =
            serde_json::from_str(raw).map_err(|err| ImportError::InvalidJson(err.to_string()))?;

        self.verify_booth_backup_checksum(&data)?;
        self.validate_version(data.version)?;
        self.validate_booth_structure(&data)?;
        self.validate_all_records(
            std::slice::from_ref(&data.booth),
            &data.vendors,
            &data.purchases,
        )?;

        Ok(data)
    }

    fn verify_full_checksum(&self, data: &BackupData) -> Result<(), ImportError> {
        match verify_backup_checksum(data) {
            Ok(ChecksumVerificationResult::Verified) => {
                info!("Checksum verification passed for full backup");
                Ok(())
            }
            Ok(ChecksumVerificationResult::Missing) => {
                warn!("Backup has no checksum; allowing import for backward compatibility");
                Ok(())
            }
            Err(ImportError::ChecksumMismatch { expected, actual }) => {
                error!(
                    "Checksum verification failed for full backup: expected {}, actual {}",
                    expected, actual
                );
                Err(ImportError::ChecksumMismatch { expected, actual })
            }
            Err(other) => Err(other),
        }
    }

    fn verify_booth_backup_checksum(&self, data: &BoothBackupData) -> Result<(), ImportError> {
        match verify_booth_checksum(data) {
            Ok(ChecksumVerificationResult::Verified) => {
                info!("Checksum verification passed for booth backup");
                Ok(())
            }
            Ok(ChecksumVerificationResult::Missing) => {
                warn!("Booth backup has no checksum; allowing import for backward compatibility");
                Ok(())
            }
            Err(ImportError::ChecksumMismatch { expected, actual }) => {
                error!(
                    "Checksum verification failed for booth backup: expected {}, actual {}",
                    expected, actual
                );
                Err(ImportError::ChecksumMismatch { expected, actual })
            }
            Err(other) => Err(other),
        }
    }

    fn validate_version(&self, version: u32) -> Result<(), ImportError> {
        if version != BACKUP_FORMAT_VERSION {
            return Err(ImportError::UnsupportedVersion {
                found: version,
                supported: BACKUP_FORMAT_VERSION,
            });
        }

        Ok(())
    }

    fn validate_booth_structure(&self, data: &BoothBackupData) -> Result<(), ImportError> {
        for vendor in &data.vendors {
            if vendor.booth_id != data.booth.id {
                return Err(ImportError::InvalidStructure(format!(
                    "vendor {} references booth {} but backup is for booth {}",
                    vendor.vendor_id, vendor.booth_id, data.booth.id
                )));
            }
        }

        for purchase in &data.purchases {
            if purchase.booth_id != data.booth.id {
                return Err(ImportError::InvalidStructure(format!(
                    "purchase {} references booth {} but backup is for booth {}",
                    purchase.id, purchase.booth_id, data.booth.id
                )));
            }
        }

        Ok(())
    }

    fn validate_all_records(
        &self,
        booths: &[domain::Booth],
        vendors: &[Vendor],
        purchases: &[Purchase],
    ) -> Result<(), ImportError> {
        self.validate_relationships(booths, vendors, purchases)?;

        let failures = self.validate_records(booths, vendors, purchases);
        if failures.is_empty() {
            Ok(())
        } else {
            Err(ImportError::ValidationFailed { failures })
        }
    }

    fn validate_relationships(
        &self,
        booths: &[domain::Booth],
        vendors: &[Vendor],
        purchases: &[Purchase],
    ) -> Result<(), ImportError> {
        let booth_ids: HashSet<BoothId> = booths.iter().map(|booth| booth.id).collect();
        let vendor_pairs: HashSet<(BoothId, String)> = vendors
            .iter()
            .map(|vendor| (vendor.booth_id, vendor.vendor_id.as_str().to_string()))
            .collect();

        let mut details = Vec::new();

        for vendor in vendors {
            if !booth_ids.contains(&vendor.booth_id) {
                details.push(format!(
                    "vendor {} references missing booth {}",
                    vendor.vendor_id, vendor.booth_id
                ));
            }
        }

        for purchase in purchases {
            if !booth_ids.contains(&purchase.booth_id) {
                details.push(format!(
                    "purchase {} references missing booth {}",
                    purchase.id, purchase.booth_id
                ));
            }

            for item in &purchase.items {
                let key = (purchase.booth_id, item.vendor_id.as_str().to_string());
                if !vendor_pairs.contains(&key) {
                    details.push(format!(
                        "purchase {} item {} references missing vendor {} for booth {}",
                        purchase.id, item.id, item.vendor_id, purchase.booth_id
                    ));
                }
            }
        }

        if details.is_empty() {
            Ok(())
        } else {
            Err(ImportError::OrphanedRecords { details })
        }
    }

    fn validate_records(
        &self,
        booths: &[domain::Booth],
        _vendors: &[Vendor],
        purchases: &[Purchase],
    ) -> Vec<ValidationFailure> {
        let mut failures = Vec::new();

        for booth in booths {
            if booth.description.trim().is_empty() {
                failures.push(ValidationFailure {
                    record_type: "booth".to_string(),
                    record_id: booth.id.to_string(),
                    reason: "description cannot be empty".to_string(),
                });
            }

            if booth.fees.participation_fee.is_sign_negative() {
                failures.push(ValidationFailure {
                    record_type: "booth".to_string(),
                    record_id: booth.id.to_string(),
                    reason: "participation fee cannot be negative".to_string(),
                });
            }

            if booth.fees.sales_fee_percent.is_sign_negative() {
                failures.push(ValidationFailure {
                    record_type: "booth".to_string(),
                    record_id: booth.id.to_string(),
                    reason: "sales fee percent cannot be negative".to_string(),
                });
            }

            if booth.fees.rounding_step.is_sign_negative() {
                failures.push(ValidationFailure {
                    record_type: "booth".to_string(),
                    record_id: booth.id.to_string(),
                    reason: "rounding step cannot be negative".to_string(),
                });
            }

            match &booth.vendor_id_validation {
                VendorIdValidation::Regex(pattern) => {
                    if let Err(err) = validate_regex_pattern(pattern) {
                        failures.push(ValidationFailure {
                            record_type: "booth".to_string(),
                            record_id: booth.id.to_string(),
                            reason: format!("invalid vendor ID validation: {err}"),
                        });
                    }
                }
                VendorIdValidation::DigitsOnly { min, max } => {
                    if let Err(err) = validate_digits_only_constraints(*min, *max) {
                        failures.push(ValidationFailure {
                            record_type: "booth".to_string(),
                            record_id: booth.id.to_string(),
                            reason: format!("invalid vendor ID validation: {err}"),
                        });
                    }
                }
                VendorIdValidation::Unrestricted => {}
            }

            if let Err(err) = booth.vendor_id_omission_rules.validate() {
                failures.push(ValidationFailure {
                    record_type: "booth".to_string(),
                    record_id: booth.id.to_string(),
                    reason: format!("invalid vendor omission rules: {err}"),
                });
            }
        }

        for purchase in purchases {
            if purchase.items.is_empty() {
                failures.push(ValidationFailure {
                    record_type: "purchase".to_string(),
                    record_id: purchase.id.to_string(),
                    reason: "purchase must contain at least one item".to_string(),
                });
            }

            for item in &purchase.items {
                if item.amount <= rust_decimal::Decimal::ZERO {
                    failures.push(ValidationFailure {
                        record_type: "purchase".to_string(),
                        record_id: purchase.id.to_string(),
                        reason: format!("item {} amount must be positive", item.id),
                    });
                }

                if item.amount.scale() > 2 {
                    failures.push(ValidationFailure {
                        record_type: "purchase".to_string(),
                        record_id: purchase.id.to_string(),
                        reason: format!("item {} amount cannot exceed 2 decimals", item.id),
                    });
                }

                if item.amount > Decimal::new(1_000_000, 0) {
                    failures.push(ValidationFailure {
                        record_type: "purchase".to_string(),
                        record_id: purchase.id.to_string(),
                        reason: format!("item {} amount is too large", item.id),
                    });
                }
            }

            if purchase.total_amount() > Decimal::new(10_000_000, 0) {
                failures.push(ValidationFailure {
                    record_type: "purchase".to_string(),
                    record_id: purchase.id.to_string(),
                    reason: "purchase total is too large".to_string(),
                });
            }
        }

        failures
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};
    use domain::{
        Booth, FeeConfig, OmissionRule, Purchase, PurchaseItem, ValidationError, Vendor, VendorId,
        VendorIdValidation,
    };
    use rust_decimal_macros::dec;

    use super::*;

    fn sample_booth() -> domain::Booth {
        Booth::new(
            "Spring Market 2026".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 29).unwrap(),
            FeeConfig {
                participation_fee: dec!(10.00),
                sales_fee_percent: dec!(15.00),
                rounding_step: dec!(0.50),
            },
        )
        .unwrap()
    }

    fn sample_vendor(booth_id: BoothId) -> Vendor {
        Vendor::new(VendorId::new("12".to_string()), booth_id)
    }

    fn sample_purchase(booth_id: BoothId, vendor_id: &domain::VendorId) -> Purchase {
        Purchase::new(
            booth_id,
            vec![PurchaseItem::new(dec!(42.00), vendor_id.clone()).unwrap()],
        )
        .unwrap()
    }

    fn sample_backup() -> BackupData {
        let booth = sample_booth();
        let vendor = sample_vendor(booth.id);
        let purchase = sample_purchase(booth.id, &vendor.vendor_id);

        BackupData {
            version: BACKUP_FORMAT_VERSION,
            created_at: Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap(),
            app_version: "test-version".to_string(),
            checksum: None,
            device_info: None,
            booths: vec![booth],
            vendors: vec![vendor],
            purchases: vec![purchase],
            metadata: Default::default(),
        }
    }

    #[test]
    fn accepts_valid_full_backup() {
        let validator = ImportValidator::new();
        let raw = serde_json::to_string_pretty(&sample_backup()).unwrap();

        let data = validator.validate_backup(&raw).unwrap();

        assert_eq!(data.version, BACKUP_FORMAT_VERSION);
        assert_eq!(data.booths.len(), 1);
    }

    #[test]
    fn rejects_invalid_json() {
        let validator = ImportValidator::new();
        let error = validator.validate_backup("{not-json}").unwrap_err();

        assert!(matches!(error, ImportError::InvalidJson(_)));
    }

    #[test]
    fn rejects_unsupported_version() {
        let validator = ImportValidator::new();
        let mut backup = sample_backup();
        backup.version = 99;

        let error = validator
            .validate_backup(&serde_json::to_string(&backup).unwrap())
            .unwrap_err();

        assert!(matches!(
            error,
            ImportError::UnsupportedVersion {
                found: 99,
                supported: BACKUP_FORMAT_VERSION
            }
        ));
    }

    #[test]
    fn rejects_orphaned_vendor_relationships() {
        let validator = ImportValidator::new();
        let mut backup = sample_backup();
        backup.vendors[0].booth_id = BoothId::new();

        let error = validator
            .validate_backup(&serde_json::to_string(&backup).unwrap())
            .unwrap_err();

        match error {
            ImportError::OrphanedRecords { details } => {
                assert_eq!(details.len(), 2);
                assert!(details.iter().any(|detail| detail.contains("vendor")));
            }
            other => panic!("expected OrphanedRecords, got {other:?}"),
        }
    }

    #[test]
    fn rejects_missing_purchase_vendor_relationships() {
        let validator = ImportValidator::new();
        let mut backup = sample_backup();
        backup.vendors.clear();

        let error = validator
            .validate_backup(&serde_json::to_string(&backup).unwrap())
            .unwrap_err();

        match error {
            ImportError::OrphanedRecords { details } => {
                assert_eq!(details.len(), 1);
                assert!(details[0].contains("missing vendor"));
            }
            other => panic!("expected OrphanedRecords, got {other:?}"),
        }
    }

    #[test]
    fn rejects_business_rule_failures_with_full_failure_list() {
        let validator = ImportValidator::new();
        let mut backup = sample_backup();
        backup.booths[0].description = "   ".to_string();
        backup.booths[0].fees.rounding_step = dec!(-0.50);
        backup.booths[0].vendor_id_validation = VendorIdValidation::Regex("[".to_string());
        backup.booths[0].vendor_id_omission_rules.rules = vec![OmissionRule::RangeWithStep {
            start: 10,
            end: 20,
            step: 0,
        }];
        backup.purchases[0].items[0].amount = dec!(0.001);

        let error = validator
            .validate_backup(&serde_json::to_string(&backup).unwrap())
            .unwrap_err();

        match error {
            ImportError::ValidationFailed { failures } => {
                assert!(failures.len() >= 5);
                assert!(failures
                    .iter()
                    .any(|failure| failure.reason.contains("description")));
                assert!(failures
                    .iter()
                    .any(|failure| failure.reason.contains("rounding step")));
                assert!(failures
                    .iter()
                    .any(|failure| failure.reason.contains("vendor ID validation")));
                assert!(failures
                    .iter()
                    .any(|failure| failure.reason.contains("vendor omission rules")));
                assert!(failures
                    .iter()
                    .any(|failure| failure.reason.contains("2 decimals")));
            }
            other => panic!("expected ValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn rejects_booth_backup_with_cross_booth_records() {
        let validator = ImportValidator::new();
        let booth = sample_booth();
        let mut booth_backup = BoothBackupData::new(booth.clone(), "test-version");
        booth_backup.created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();
        booth_backup.vendors = vec![sample_vendor(BoothId::new())];

        let error = validator
            .validate_booth_backup(&serde_json::to_string(&booth_backup).unwrap())
            .unwrap_err();

        assert!(matches!(error, ImportError::InvalidStructure(_)));
    }

    #[test]
    fn accepts_valid_booth_backup() {
        let validator = ImportValidator::new();
        let booth = sample_booth();
        let vendor = sample_vendor(booth.id);
        let purchase = sample_purchase(booth.id, &vendor.vendor_id);
        let mut booth_backup = BoothBackupData::new(booth, "test-version");
        booth_backup.created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();
        booth_backup.vendors = vec![vendor];
        booth_backup.purchases = vec![purchase];

        let data = validator
            .validate_booth_backup(&serde_json::to_string_pretty(&booth_backup).unwrap())
            .unwrap();

        assert_eq!(data.version, BACKUP_FORMAT_VERSION);
        assert_eq!(data.vendors.len(), 1);
    }

    #[test]
    fn rejects_backup_with_invalid_checksum() {
        let validator = ImportValidator::new();
        let mut backup = sample_backup();
        backup.checksum = Some("wrong".to_string());

        let error = validator
            .validate_backup(&serde_json::to_string(&backup).unwrap())
            .unwrap_err();

        match error {
            ImportError::ChecksumMismatch { expected, actual } => {
                assert_eq!(expected, "wrong");
                assert_eq!(actual.len(), 64);
            }
            other => panic!("expected ChecksumMismatch, got {other:?}"),
        }
    }

    #[test]
    fn rejects_booth_backup_with_invalid_checksum() {
        let validator = ImportValidator::new();
        let booth = sample_booth();
        let vendor = sample_vendor(booth.id);
        let purchase = sample_purchase(booth.id, &vendor.vendor_id);
        let mut booth_backup = BoothBackupData::new(booth, "test-version");
        booth_backup.created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();
        booth_backup.checksum = Some("wrong".to_string());
        booth_backup.vendors = vec![vendor];
        booth_backup.purchases = vec![purchase];

        let error = validator
            .validate_booth_backup(&serde_json::to_string(&booth_backup).unwrap())
            .unwrap_err();

        assert!(matches!(error, ImportError::ChecksumMismatch { .. }));
    }

    #[test]
    fn validation_failure_is_equatable() {
        let failure = ValidationFailure {
            record_type: "booth".to_string(),
            record_id: "123".to_string(),
            reason: ValidationError::VendorIdEmpty.to_string(),
        };

        assert_eq!(failure.record_type, "booth");
    }
}

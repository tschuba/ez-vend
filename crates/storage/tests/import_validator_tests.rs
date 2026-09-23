use chrono::{NaiveDate, TimeZone, Utc};
use domain::{
    Booth, BoothId, FeeConfig, OmissionRule, Purchase, PurchaseItem, Vendor, VendorId,
    VendorIdValidation,
};
use ez_vend_storage::export::{
    BackupData, BoothBackupData, ImportError, ImportValidator, ValidationFailure,
    BACKUP_FORMAT_VERSION,
};
use rust_decimal_macros::dec;

fn sample_booth() -> Booth {
    Booth::new(
        "Spring Market 2026".to_string(),
        NaiveDate::from_ymd_opt(2026, 3, 29).unwrap(),
        FeeConfig {
            participation_fee: dec!(10.00),
            sales_fee_percent: dec!(15.00),
            rounding_step: dec!(0.50),
        },
        domain::BoothType::ThirdPartySale,
        None,
    )
    .unwrap()
}

fn sample_vendor(booth_id: BoothId) -> Vendor {
    Vendor::new(VendorId::new("12".to_string()), booth_id)
}

fn sample_purchase(booth_id: BoothId, vendor_id: &VendorId) -> Purchase {
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
        products: vec![],
        product_groups: vec![],
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
fn accepts_backup_without_checksum_for_backward_compatibility() {
    let validator = ImportValidator::new();
    let data = validator
        .validate_backup(&serde_json::to_string(&sample_backup()).unwrap())
        .unwrap();

    assert_eq!(data.checksum, None);
}

#[test]
fn accepts_legacy_vendor_name_field_for_backward_compatibility() {
    let validator = ImportValidator::new();
    let mut raw: serde_json::Value = serde_json::to_value(sample_backup()).unwrap();
    raw["vendors"][0]["name"] = serde_json::Value::String("Legacy Vendor Name".to_string());

    let data = validator
        .validate_backup(&serde_json::to_string(&raw).unwrap())
        .unwrap();

    assert_eq!(data.vendors.len(), 1);
    assert_eq!(data.vendors[0].vendor_id.as_str(), "12");
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
fn rejects_full_backup_with_checksum_mismatch() {
    let validator = ImportValidator::new();
    let mut backup = sample_backup();
    backup.checksum = Some("wrong".to_string());

    let error = validator
        .validate_backup(&serde_json::to_string(&backup).unwrap())
        .unwrap_err();

    assert!(matches!(error, ImportError::ChecksumMismatch { .. }));
}

#[test]
fn rejects_booth_backup_with_checksum_mismatch() {
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
        reason: "description cannot be empty".to_string(),
    };

    assert_eq!(failure.record_type, "booth");
}

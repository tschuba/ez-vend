use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::backup_format::{BackupData, BoothBackupData};
use super::error::ImportError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumVerificationResult {
    Missing,
    Verified,
}

pub fn compute_backup_checksum(data: &BackupData) -> Result<String, serde_json::Error> {
    let mut payload = data.clone();
    payload.checksum = None;
    compute_checksum(&payload)
}

pub fn compute_booth_checksum(data: &BoothBackupData) -> Result<String, serde_json::Error> {
    let mut payload = data.clone();
    payload.checksum = None;
    compute_checksum(&payload)
}

pub fn verify_backup_checksum(
    data: &BackupData,
) -> Result<ChecksumVerificationResult, ImportError> {
    verify_checksum(data.checksum.as_deref(), compute_backup_checksum(data))
}

pub fn verify_booth_checksum(
    data: &BoothBackupData,
) -> Result<ChecksumVerificationResult, ImportError> {
    verify_checksum(data.checksum.as_deref(), compute_booth_checksum(data))
}

fn verify_checksum(
    expected: Option<&str>,
    actual: Result<String, serde_json::Error>,
) -> Result<ChecksumVerificationResult, ImportError> {
    let Some(expected) = expected else {
        return Ok(ChecksumVerificationResult::Missing);
    };

    let actual = actual.map_err(|err| ImportError::InvalidStructure(err.to_string()))?;

    if expected == actual {
        Ok(ChecksumVerificationResult::Verified)
    } else {
        Err(ImportError::ChecksumMismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}

fn compute_checksum<T: Serialize>(payload: &T) -> Result<String, serde_json::Error> {
    let normalized = normalize_json_value(serde_json::to_value(payload)?);
    let bytes = serde_json::to_vec(&normalized)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn normalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::Array(values.into_iter().map(normalize_json_value).collect())
        }
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));

            let mut normalized = Map::with_capacity(entries.len());
            for (key, value) in entries {
                normalized.insert(key, normalize_json_value(value));
            }

            Value::Object(normalized)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use domain::{Booth, FeeConfig, Purchase, PurchaseItem, Vendor, VendorId};
    use rust_decimal_macros::dec;
    use serde_json::json;

    use super::*;
    use crate::export::BACKUP_FORMAT_VERSION;

    fn sample_booth() -> Booth {
        Booth::new(
            "Spring Market 2026".to_string(),
            chrono::NaiveDate::from_ymd_opt(2026, 3, 29).unwrap(),
            FeeConfig {
                participation_fee: dec!(10.00),
                sales_fee_percent: dec!(15.00),
                rounding_step: dec!(0.50),
            },
        )
        .unwrap()
    }

    fn sample_backup() -> BackupData {
        let booth = sample_booth();
        let vendor = Vendor::new(VendorId::new("12".to_string()), booth.id);
        let purchase = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(dec!(42.00), vendor.vendor_id.clone()).unwrap()],
        )
        .unwrap();

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
    fn computes_deterministic_checksum_for_same_payload() {
        let mut left = sample_backup();
        left.metadata
            .insert("b".to_string(), json!({ "z": 1, "a": 2 }));
        left.metadata.insert("a".to_string(), json!([3, 2, 1]));

        let mut right = left.clone();
        right.metadata.clear();
        right.metadata.insert("a".to_string(), json!([3, 2, 1]));
        right
            .metadata
            .insert("b".to_string(), json!({ "a": 2, "z": 1 }));

        let left_checksum = compute_backup_checksum(&left).unwrap();
        let right_checksum = compute_backup_checksum(&right).unwrap();

        assert_eq!(left_checksum, right_checksum);
    }

    #[test]
    fn verifies_matching_checksum() {
        let mut backup = sample_backup();
        backup.checksum = Some(compute_backup_checksum(&backup).unwrap());

        let result = verify_backup_checksum(&backup).unwrap();

        assert_eq!(result, ChecksumVerificationResult::Verified);
    }

    #[test]
    fn allows_missing_checksum_for_backward_compatibility() {
        let backup = sample_backup();

        let result = verify_backup_checksum(&backup).unwrap();

        assert_eq!(result, ChecksumVerificationResult::Missing);
    }

    #[test]
    fn rejects_mismatched_checksum() {
        let mut backup = sample_backup();
        backup.checksum = Some("bad-checksum".to_string());

        let error = verify_backup_checksum(&backup).unwrap_err();

        match error {
            ImportError::ChecksumMismatch { expected, actual } => {
                assert_eq!(expected, "bad-checksum");
                assert_ne!(expected, actual);
                assert_eq!(actual.len(), 64);
            }
            other => panic!("expected checksum mismatch, got {other:?}"),
        }
    }
}

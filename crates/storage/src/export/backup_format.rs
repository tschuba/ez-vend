use std::collections::HashMap;

use chrono::{DateTime, Utc};
use domain::{Booth, BoothId, Purchase, Vendor};
use serde::{Deserialize, Serialize};

pub const BACKUP_FORMAT_VERSION: u32 = 1;
pub const BACKUP_FILE_EXTENSION: &str = "json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub identifier: String,
    pub platform: String,
    pub browser: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupData {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub app_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_info: Option<DeviceInfo>,
    #[serde(default)]
    pub booths: Vec<Booth>,
    #[serde(default)]
    pub vendors: Vec<Vendor>,
    #[serde(default)]
    pub purchases: Vec<Purchase>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl BackupData {
    pub fn new(app_version: impl Into<String>) -> Self {
        Self {
            version: BACKUP_FORMAT_VERSION,
            created_at: Utc::now(),
            app_version: app_version.into(),
            checksum: None,
            device_info: None,
            booths: Vec::new(),
            vendors: Vec::new(),
            purchases: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoothBackupData {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub app_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_info: Option<DeviceInfo>,
    pub booth: Booth,
    #[serde(default)]
    pub vendors: Vec<Vendor>,
    #[serde(default)]
    pub purchases: Vec<Purchase>,
}

impl BoothBackupData {
    pub fn new(booth: Booth, app_version: impl Into<String>) -> Self {
        Self {
            version: BACKUP_FORMAT_VERSION,
            created_at: Utc::now(),
            app_version: app_version.into(),
            checksum: None,
            device_info: None,
            booth,
            vendors: Vec::new(),
            purchases: Vec::new(),
        }
    }
}

pub fn generate_full_backup_filename(created_at: DateTime<Utc>) -> String {
    generate_full_backup_filename_with_device(created_at, None)
}

pub fn generate_full_backup_filename_with_device(
    created_at: DateTime<Utc>,
    device_identifier: Option<&str>,
) -> String {
    let device_suffix = sanitized_device_suffix(device_identifier);

    format!(
        "ez-vend-backup-{}{}.{}",
        created_at.format("%Y-%m-%d"),
        device_suffix,
        BACKUP_FILE_EXTENSION
    )
}

pub fn generate_booth_backup_filename(
    booth_id: &BoothId,
    description: &str,
    created_at: DateTime<Utc>,
) -> String {
    generate_booth_backup_filename_with_device(booth_id, description, created_at, None)
}

pub fn generate_booth_backup_filename_with_device(
    booth_id: &BoothId,
    description: &str,
    created_at: DateTime<Utc>,
    device_identifier: Option<&str>,
) -> String {
    let sanitized = sanitize_filename_component(description);
    let label = if sanitized.is_empty() {
        format!("booth-{}", booth_id)
    } else {
        sanitized
    };
    let device_suffix = sanitized_device_suffix(device_identifier);

    format!(
        "ez-vend-{}-{}{}.{}",
        label,
        created_at.format("%Y-%m-%d"),
        device_suffix,
        BACKUP_FILE_EXTENSION
    )
}

fn sanitized_device_suffix(device_identifier: Option<&str>) -> String {
    device_identifier
        .map(sanitize_filename_component)
        .filter(|value| !value.is_empty())
        .map(|value| format!("-{value}"))
        .unwrap_or_default()
}

pub fn sanitize_filename_component(input: &str) -> String {
    let mut value = String::with_capacity(input.len());
    let mut last_was_dash = false;

    for ch in input.trim().chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if matches!(ch, ' ' | '-' | '_') {
            Some('-')
        } else {
            None
        };

        match mapped {
            Some('-') if !last_was_dash => {
                value.push('-');
                last_was_dash = true;
            }
            Some(c) if c != '-' => {
                value.push(c);
                last_was_dash = false;
            }
            _ => {}
        }
    }

    value.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use domain::{Booth, FeeConfig};
    use rust_decimal_macros::dec;

    use super::*;

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

    #[test]
    fn sanitizes_filename_components() {
        assert_eq!(
            sanitize_filename_component(" Spring Market / 2026 "),
            "spring-market-2026"
        );
        assert_eq!(sanitize_filename_component("***"), "");
        assert_eq!(sanitize_filename_component("A__B  C"), "a-b-c");
    }

    #[test]
    fn builds_full_backup_filename() {
        let created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();
        assert_eq!(
            generate_full_backup_filename(created_at),
            "ez-vend-backup-2026-03-29.json"
        );
    }

    #[test]
    fn builds_full_backup_filename_with_device_identifier() {
        let created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();
        assert_eq!(
            generate_full_backup_filename_with_device(created_at, Some("Maria's Laptop")),
            "ez-vend-backup-2026-03-29-marias-laptop.json"
        );
    }

    #[test]
    fn builds_booth_backup_filename() {
        let booth = sample_booth();
        let created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();

        let filename = generate_booth_backup_filename(&booth.id, &booth.description, created_at);

        assert_eq!(filename, "ez-vend-spring-market-2026-2026-03-29.json");
    }

    #[test]
    fn builds_booth_backup_filename_with_device_identifier() {
        let booth = sample_booth();
        let created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();

        let filename = generate_booth_backup_filename_with_device(
            &booth.id,
            &booth.description,
            created_at,
            Some("Maria's Laptop"),
        );

        assert_eq!(
            filename,
            "ez-vend-spring-market-2026-2026-03-29-marias-laptop.json"
        );
    }

    #[test]
    fn falls_back_to_booth_id_when_description_is_empty_after_sanitizing() {
        let booth = sample_booth();
        let created_at = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();

        let filename = generate_booth_backup_filename(&booth.id, "***", created_at);

        assert!(filename.starts_with("ez-vend-booth-"));
        assert!(filename.ends_with("-2026-03-29.json"));
    }
}

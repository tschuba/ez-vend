use chrono::{DateTime, Duration, Utc};
use log::warn;
use serde::{Deserialize, Serialize};

pub const ERROR_LOG_RETENTION_DAYS: i64 = 7;
pub const ERROR_LOG_RETENTION_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorLogContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub booth_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchase_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_action: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorLogDeviceInfo {
    pub identifier: String,
    pub platform: String,
    pub browser: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorLogEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub error_type: String,
    pub error_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<ErrorLogContext>,
    pub device_info: ErrorLogDeviceInfo,
}

impl ErrorLogEntry {
    pub fn with_id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }
}

pub fn retention_cutoff(now: DateTime<Utc>) -> DateTime<Utc> {
    now - Duration::days(ERROR_LOG_RETENTION_DAYS)
}

pub fn retention_ids_to_delete(entries: &[ErrorLogEntry], now: DateTime<Utc>) -> Vec<u32> {
    let cutoff = retention_cutoff(now);
    let mut retained_recent = entries
        .iter()
        .filter(|entry| entry.timestamp >= cutoff)
        .cloned()
        .collect::<Vec<_>>();

    retained_recent.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));

    let keep_ids = retained_recent
        .into_iter()
        .take(ERROR_LOG_RETENTION_LIMIT)
        .filter_map(|entry| entry.id)
        .collect::<std::collections::HashSet<_>>();

    entries
        .iter()
        .filter_map(|entry| match entry.id {
            Some(id) if !keep_ids.contains(&id) => Some(id),
            None => {
                warn!("Error log entry without ID found during retention cleanup");
                None
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn sample_entry(id: u32, timestamp: DateTime<Utc>) -> ErrorLogEntry {
        ErrorLogEntry {
            id: Some(id),
            timestamp,
            session_id: "session-1".to_string(),
            error_type: "validation".to_string(),
            error_message: "Vendor required".to_string(),
            stack_trace: Some("Error: Vendor required".to_string()),
            context: Some(ErrorLogContext {
                booth_id: Some("booth-1".to_string()),
                vendor_id: Some("123".to_string()),
                purchase_id: None,
                route: Some("/checkout".to_string()),
                user_action: Some("submit purchase".to_string()),
                details: vec!["field=vendor_id".to_string()],
            }),
            device_info: ErrorLogDeviceInfo {
                identifier: "front-desk".to_string(),
                platform: "macOS".to_string(),
                browser: "Safari".to_string(),
            },
        }
    }

    #[test]
    fn error_log_entry_serializes_context_and_stack_trace() {
        let entry = sample_entry(1, Utc.with_ymd_and_hms(2026, 4, 4, 12, 30, 0).unwrap());

        let json = serde_json::to_string(&entry).unwrap();

        assert!(json.contains("Vendor required"));
        assert!(json.contains("/checkout"));
        assert!(json.contains("front-desk"));
    }

    #[test]
    fn retention_deletes_entries_older_than_cutoff() {
        let now = Utc.with_ymd_and_hms(2026, 4, 10, 12, 0, 0).unwrap();
        let entries = vec![
            sample_entry(1, now - Duration::days(8)),
            sample_entry(2, now - Duration::days(2)),
        ];

        let ids = retention_ids_to_delete(&entries, now);

        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn retention_deletes_oldest_entries_over_limit() {
        let now = Utc.with_ymd_and_hms(2026, 4, 10, 12, 0, 0).unwrap();
        let entries = (0..105)
            .map(|index| sample_entry(index + 1, now - Duration::minutes(index.into())))
            .collect::<Vec<_>>();

        let ids = retention_ids_to_delete(&entries, now);

        assert_eq!(ids.len(), 5);
        assert!(ids.contains(&101));
        assert!(ids.contains(&105));
    }
}

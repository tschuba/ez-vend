use chrono::{DateTime, Local, Utc};
use ez_vend_storage::diagnostics::{IntegrityStatus, StorageDiagnostics};
use ez_vend_storage::ArchiveAuditEvent;
use ez_vend_storage::ErrorLogEntry;
use serde::Serialize;

pub const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");
pub const DOCS_URL: &str = "https://tschuba.github.io/ez-vend/";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsExportData {
    pub app_version: String,
    pub database_version: u32,
    pub exported_at: DateTime<Utc>,
    pub device_identifier: String,
    pub device_platform: String,
    pub device_browser: String,
    pub session_id: String,
    pub last_backup_at: Option<DateTime<Utc>>,
    pub integrity_status: IntegrityStatus,
    pub recent_error_count: usize,
    pub errors: Vec<ErrorLogEntry>,
    pub archive_audit_events: Vec<ArchiveAuditEvent>,
    pub booth_summary: DiagnosticsBoothSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsBoothSummary {
    pub active_booth_id: Option<String>,
    pub active_booth_description: Option<String>,
    pub vendor_count: usize,
    pub purchase_count: usize,
}

pub fn format_optional_timestamp(timestamp: Option<DateTime<Utc>>) -> Option<String> {
    timestamp.map(format_timestamp)
}

pub fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

pub fn format_error_log_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn integrity_issue_count(status: &IntegrityStatus) -> usize {
    match status {
        IntegrityStatus::Healthy => 0,
        IntegrityStatus::IssuesFound { issues } => issues.len(),
    }
}

pub fn diagnostics_last_backup_label(diagnostics: &StorageDiagnostics) -> Option<String> {
    format_optional_timestamp(diagnostics.last_backup_at)
}

pub fn diagnostics_export_filename(exported_at: DateTime<Utc>) -> String {
    format!(
        "ez-vend-diagnostics-{}.json",
        exported_at.format("%Y-%m-%d-%H%M%S")
    )
}

pub fn error_log_print_filename(exported_at: DateTime<Utc>) -> String {
    format!(
        "ez-vend-error-log-{}.txt",
        exported_at.format("%Y-%m-%d-%H%M%S")
    )
}

pub fn format_error_log_text(entries: &[ErrorLogEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            let mut lines = vec![
                format!("Timestamp: {}", format_error_log_timestamp(entry.timestamp)),
                format!("Type: {}", entry.error_type),
                format!("Message: {}", entry.error_message),
                format!("Session: {}", entry.session_id),
                format!(
                    "Device: {} / {} / {}",
                    entry.device_info.identifier,
                    entry.device_info.platform,
                    entry.device_info.browser
                ),
            ];

            if let Some(context) = &entry.context {
                if let Some(route) = &context.route {
                    lines.push(format!("Route: {route}"));
                }
                if let Some(action) = &context.user_action {
                    lines.push(format!("Action: {action}"));
                }
                if let Some(booth_id) = &context.booth_id {
                    lines.push(format!("Booth: {booth_id}"));
                }
                if let Some(vendor_id) = &context.vendor_id {
                    lines.push(format!("Vendor: {vendor_id}"));
                }
                if let Some(purchase_id) = &context.purchase_id {
                    lines.push(format!("Purchase: {purchase_id}"));
                }
                if !context.details.is_empty() {
                    lines.push(format!("Details: {}", context.details.join(", ")));
                }
            }

            if let Some(stack_trace) = &entry.stack_trace {
                lines.push("Stack Trace:".to_string());
                lines.push(stack_trace.clone());
            }

            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

pub fn build_error_log_print_html(entries: &[ErrorLogEntry], title: &str) -> String {
    let rows = entries
        .iter()
        .map(|entry| {
            let stack_trace = entry.stack_trace.clone().unwrap_or_else(|| "-".to_string());
            let context = entry
                .context
                .as_ref()
                .map(|context| {
                    let mut values = Vec::new();
                    if let Some(route) = &context.route {
                        values.push(format!("route={route}"));
                    }
                    if let Some(action) = &context.user_action {
                        values.push(format!("action={action}"));
                    }
                    if let Some(booth_id) = &context.booth_id {
                        values.push(format!("booth={booth_id}"));
                    }
                    if let Some(vendor_id) = &context.vendor_id {
                        values.push(format!("vendor={vendor_id}"));
                    }
                    if let Some(purchase_id) = &context.purchase_id {
                        values.push(format!("purchase={purchase_id}"));
                    }
                    values.extend(context.details.iter().cloned());
                    values.join("<br>")
                })
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "-".to_string());

            format!(
                "<article class=\"entry\"><h2>{}</h2><p><strong>{}</strong></p><p>{}</p><p>{}</p><pre>{}</pre></article>",
                html_escape(&format_error_log_timestamp(entry.timestamp)),
                html_escape(&entry.error_type),
                html_escape(&entry.error_message),
                context,
                html_escape(&stack_trace),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title><style>body{{font-family:Arial,sans-serif;margin:24px;color:#0f172a}}h1{{margin-bottom:24px}}.entry{{break-inside:avoid;border:1px solid #cbd5e1;border-radius:8px;padding:16px;margin-bottom:16px}}pre{{white-space:pre-wrap;background:#f8fafc;padding:12px;border-radius:6px;overflow-wrap:anywhere}}</style></head><body><h1>{}</h1>{}</body></html>",
        html_escape(title),
        html_escape(title),
        rows
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn integrity_issue_count_reports_healthy_as_zero() {
        assert_eq!(integrity_issue_count(&IntegrityStatus::Healthy), 0);
    }

    #[test]
    fn integrity_issue_count_reports_issue_total() {
        let status = IntegrityStatus::IssuesFound {
            issues: vec!["one".to_string(), "two".to_string()],
        };

        assert_eq!(integrity_issue_count(&status), 2);
    }

    #[test]
    fn optional_timestamp_returns_none_when_absent() {
        assert_eq!(format_optional_timestamp(None), None);
    }

    #[test]
    fn optional_timestamp_formats_present_values() {
        let timestamp = Utc.with_ymd_and_hms(2026, 4, 4, 12, 30, 0).unwrap();

        assert!(format_optional_timestamp(Some(timestamp))
            .unwrap()
            .contains("2026-04-04"));
    }

    #[test]
    fn error_log_text_includes_key_fields() {
        let entry = ErrorLogEntry {
            id: Some(1),
            timestamp: Utc.with_ymd_and_hms(2026, 4, 4, 12, 30, 0).unwrap(),
            session_id: "session-1".to_string(),
            error_type: "validation".to_string(),
            error_message: "Vendor required".to_string(),
            stack_trace: Some("Error: Vendor required".to_string()),
            context: None,
            device_info: ez_vend_storage::ErrorLogDeviceInfo {
                identifier: "desk".to_string(),
                platform: "macOS".to_string(),
                browser: "Safari".to_string(),
            },
        };

        let text = format_error_log_text(&[entry]);

        assert!(text.contains("Vendor required"));
        assert!(text.contains("session-1"));
    }
}

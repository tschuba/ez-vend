use async_trait::async_trait;
use chrono::{DateTime, Utc};
use js_sys::{Object, Reflect};
use rexie::TransactionMode;
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::Arc;
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::error_log::{retention_ids_to_delete, ErrorLogEntry};
use crate::indexeddb::Database;

#[async_trait(?Send)]
pub trait ErrorLogRepository {
    async fn log_error(&self, entry: &ErrorLogEntry) -> Result<ErrorLogEntry, StorageError>;
    async fn get_recent_errors(&self, limit: usize) -> Result<Vec<ErrorLogEntry>, StorageError>;
    async fn get_errors_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<ErrorLogEntry>, StorageError>;
    async fn count_errors_since(&self, since: DateTime<Utc>) -> Result<usize, StorageError>;
    async fn clear_all(&self) -> Result<(), StorageError>;
}

#[derive(Clone)]
pub struct IndexedDbErrorLogRepository {
    db: Arc<Database>,
}

impl IndexedDbErrorLogRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    fn error_logs_store_missing(error: &StorageError) -> bool {
        match error {
            StorageError::TransactionError(message) | StorageError::DatabaseError(message) => {
                message.contains("NotFoundError")
                    || message.contains("object stores was not found")
                    || message.contains("object store was not found")
            }
            _ => false,
        }
    }

    async fn load_all_entries(&self) -> Result<Vec<ErrorLogEntry>, StorageError> {
        let transaction = match self
            .db
            .transaction(&["error_logs"], TransactionMode::ReadOnly)
        {
            Ok(transaction) => transaction,
            Err(error) if Self::error_logs_store_missing(&error) => {
                log::warn!("error_logs object store missing; returning empty error log results");
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };
        let store = transaction
            .store("error_logs")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let values = store
            .get_all(None, None)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let mut entries = values
            .into_iter()
            .map(deserialize_error_log_entry)
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        Ok(entries)
    }

    async fn prune_retention(&self, now: DateTime<Utc>) -> Result<(), StorageError> {
        let entries = self.load_all_entries().await?;
        let ids_to_delete = retention_ids_to_delete(&entries, now);
        if ids_to_delete.is_empty() {
            return Ok(());
        }

        let transaction = self
            .db
            .transaction(&["error_logs"], TransactionMode::ReadWrite)?;
        let store = transaction
            .store("error_logs")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        for id in ids_to_delete {
            store
                .delete(JsValue::from_f64(f64::from(id)))
                .await
                .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
        }

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl ErrorLogRepository for IndexedDbErrorLogRepository {
    async fn log_error(&self, entry: &ErrorLogEntry) -> Result<ErrorLogEntry, StorageError> {
        let transaction = match self
            .db
            .transaction(&["error_logs"], TransactionMode::ReadWrite)
        {
            Ok(transaction) => transaction,
            Err(error) if Self::error_logs_store_missing(&error) => {
                log::warn!("error_logs object store missing; skipping persisted error log entry");
                return Ok(entry.clone());
            }
            Err(error) => return Err(error),
        };
        let store = transaction
            .store("error_logs")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let value = serialize_error_log_entry(entry)?;
        let key = store
            .add(&value, None)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let id = js_value_to_u32(&key)?;
        let stored = entry.clone().with_id(id);
        self.prune_retention(entry.timestamp).await?;
        Ok(stored)
    }

    async fn get_recent_errors(&self, limit: usize) -> Result<Vec<ErrorLogEntry>, StorageError> {
        let mut entries = self.load_all_entries().await?;
        entries.truncate(limit);
        Ok(entries)
    }

    async fn get_errors_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<ErrorLogEntry>, StorageError> {
        let entries = self.load_all_entries().await?;
        Ok(entries
            .into_iter()
            .filter(|entry| entry.timestamp >= since)
            .collect())
    }

    async fn count_errors_since(&self, since: DateTime<Utc>) -> Result<usize, StorageError> {
        Ok(self.get_errors_since(since).await?.len())
    }

    async fn clear_all(&self) -> Result<(), StorageError> {
        let transaction = match self
            .db
            .transaction(&["error_logs"], TransactionMode::ReadWrite)
        {
            Ok(transaction) => transaction,
            Err(error) if Self::error_logs_store_missing(&error) => {
                log::warn!("error_logs object store missing; treating clear request as no-op");
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let store = transaction
            .store("error_logs")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        store
            .clear()
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(())
    }
}

fn serialize_error_log_entry(entry: &ErrorLogEntry) -> Result<JsValue, StorageError> {
    let object = Object::new();
    Reflect::set(
        &object,
        &JsValue::from_str("timestamp"),
        &to_value(&entry.timestamp).map_err(|e| StorageError::SerializationError(e.to_string()))?,
    )
    .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
    Reflect::set(
        &object,
        &JsValue::from_str("session_id"),
        &JsValue::from_str(&entry.session_id),
    )
    .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
    Reflect::set(
        &object,
        &JsValue::from_str("error_type"),
        &JsValue::from_str(&entry.error_type),
    )
    .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
    Reflect::set(
        &object,
        &JsValue::from_str("error_message"),
        &JsValue::from_str(&entry.error_message),
    )
    .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;

    if let Some(stack_trace) = &entry.stack_trace {
        Reflect::set(
            &object,
            &JsValue::from_str("stack_trace"),
            &JsValue::from_str(stack_trace),
        )
        .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
    }

    if let Some(context) = &entry.context {
        Reflect::set(
            &object,
            &JsValue::from_str("context"),
            &to_value(context).map_err(|e| StorageError::SerializationError(e.to_string()))?,
        )
        .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
    }

    Reflect::set(
        &object,
        &JsValue::from_str("device_info"),
        &to_value(&entry.device_info)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?,
    )
    .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;

    Ok(object.into())
}

fn deserialize_error_log_entry(value: JsValue) -> Result<ErrorLogEntry, StorageError> {
    let object = Object::from(value);

    let id = Reflect::get(&object, &JsValue::from_str("id"))
        .ok()
        .filter(|value| !value.is_undefined())
        .map(|value| js_value_to_u32(&value))
        .transpose()?;

    let timestamp = from_value(
        Reflect::get(&object, &JsValue::from_str("timestamp"))
            .map_err(|e| StorageError::JsError(format!("{:?}", e)))?,
    )
    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

    let session_id = js_string_field(&object, "session_id")?;
    let error_type = js_string_field(&object, "error_type")?;
    let error_message = js_string_field(&object, "error_message")?;
    let stack_trace = js_optional_string_field(&object, "stack_trace")?;

    let context = Reflect::get(&object, &JsValue::from_str("context"))
        .ok()
        .filter(|value| !value.is_undefined())
        .map(|value| from_value(value).map_err(|e| StorageError::SerializationError(e.to_string())))
        .transpose()?;

    let device_info = from_value(
        Reflect::get(&object, &JsValue::from_str("device_info"))
            .map_err(|e| StorageError::JsError(format!("{:?}", e)))?,
    )
    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

    Ok(ErrorLogEntry {
        id,
        timestamp,
        session_id,
        error_type,
        error_message,
        stack_trace,
        context,
        device_info,
    })
}

fn js_string_field(object: &Object, field: &str) -> Result<String, StorageError> {
    Reflect::get(object, &JsValue::from_str(field))
        .map_err(|e| StorageError::JsError(format!("{:?}", e)))?
        .as_string()
        .ok_or_else(|| StorageError::SerializationError(format!("Missing string field: {field}")))
}

fn js_optional_string_field(object: &Object, field: &str) -> Result<Option<String>, StorageError> {
    let value = Reflect::get(object, &JsValue::from_str(field))
        .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
    if value.is_undefined() || value.is_null() {
        Ok(None)
    } else {
        Ok(value.as_string())
    }
}

fn js_value_to_u32(value: &JsValue) -> Result<u32, StorageError> {
    value
        .as_f64()
        .and_then(|value| {
            if value >= 0.0 && value <= u32::MAX as f64 {
                Some(value as u32)
            } else {
                None
            }
        })
        .ok_or_else(|| StorageError::SerializationError("Invalid error log id".to_string()))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use crate::error_log::{ErrorLogContext, ErrorLogDeviceInfo};

    use super::*;

    #[test]
    fn error_log_entry_serializes_to_json() {
        let entry = ErrorLogEntry {
            id: Some(42),
            timestamp: Utc.with_ymd_and_hms(2026, 4, 4, 12, 30, 0).unwrap(),
            session_id: "session-1".to_string(),
            error_type: "panic".to_string(),
            error_message: "Something broke".to_string(),
            stack_trace: Some("stack line 1".to_string()),
            context: Some(ErrorLogContext {
                booth_id: Some("booth-1".to_string()),
                vendor_id: None,
                purchase_id: Some("purchase-1".to_string()),
                route: Some("/settings".to_string()),
                user_action: Some("copy diagnostics".to_string()),
                details: vec!["browser=safari".to_string()],
            }),
            device_info: ErrorLogDeviceInfo {
                identifier: "front-desk".to_string(),
                platform: "macOS".to_string(),
                browser: "Safari".to_string(),
            },
        };

        let json = serde_json::to_string(&entry).unwrap();

        assert!(json.contains("session-1"));
        assert!(json.contains("copy diagnostics"));
        assert!(json.contains("front-desk"));
    }

    #[test]
    fn recent_errors_are_sorted_newest_first() {
        let mut entries = [
            ErrorLogEntry {
                id: Some(1),
                timestamp: Utc.with_ymd_and_hms(2026, 4, 4, 12, 0, 0).unwrap(),
                session_id: "a".to_string(),
                error_type: "validation".to_string(),
                error_message: "older".to_string(),
                stack_trace: None,
                context: None,
                device_info: ErrorLogDeviceInfo {
                    identifier: "desk".to_string(),
                    platform: "macOS".to_string(),
                    browser: "Chrome".to_string(),
                },
            },
            ErrorLogEntry {
                id: Some(2),
                timestamp: Utc.with_ymd_and_hms(2026, 4, 4, 13, 0, 0).unwrap(),
                session_id: "a".to_string(),
                error_type: "validation".to_string(),
                error_message: "newer".to_string(),
                stack_trace: None,
                context: None,
                device_info: ErrorLogDeviceInfo {
                    identifier: "desk".to_string(),
                    platform: "macOS".to_string(),
                    browser: "Chrome".to_string(),
                },
            },
        ];

        entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));

        assert_eq!(entries[0].id, Some(2));
        assert_eq!(entries[1].id, Some(1));
    }
}

use chrono::{DateTime, Utc};
use domain::{BoothId, Purchase, Vendor, VendorId};
use js_sys::{Object, Reflect};
use rexie::TransactionMode;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use uuid::Uuid;
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::indexeddb::database::DB_VERSION;
use crate::indexeddb::Database;

const LAST_BACKUP_AT_METADATA_KEY: &str = "last_backup_at";
const LAST_MODIFIED_AT_METADATA_KEY: &str = "last_modified_at";
const SESSION_ID_METADATA_KEY: &str = "session_id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDiagnostics {
    pub database_version: u32,
    pub last_backup_at: Option<DateTime<Utc>>,
    pub last_modified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityStatus {
    Healthy,
    IssuesFound { issues: Vec<String> },
}

pub async fn load_storage_diagnostics(db: &Database) -> Result<StorageDiagnostics, StorageError> {
    Ok(StorageDiagnostics {
        database_version: DB_VERSION,
        last_backup_at: load_last_backup_at(db).await?,
        last_modified_at: load_last_modified_at(db).await?,
    })
}

pub async fn record_backup_completed(
    db: &Database,
    completed_at: DateTime<Utc>,
) -> Result<(), StorageError> {
    let transaction = db.transaction(&["metadata"], TransactionMode::ReadWrite)?;
    let store = transaction
        .store("metadata")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

    let metadata = metadata_value(
        LAST_BACKUP_AT_METADATA_KEY,
        to_value(&completed_at).map_err(|e| StorageError::SerializationError(e.to_string()))?,
    )?;

    store
        .put(&metadata, None)
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

    transaction.done().await?;

    Ok(())
}

pub async fn create_session_id(db: &Database) -> Result<String, StorageError> {
    let session_id = Uuid::new_v4().to_string();
    write_metadata_value(db, SESSION_ID_METADATA_KEY, JsValue::from_str(&session_id)).await?;
    Ok(session_id)
}

pub async fn run_integrity_check(db: &Database) -> Result<IntegrityStatus, StorageError> {
    let transaction = db.transaction(
        &["booths", "vendors", "purchases"],
        TransactionMode::ReadOnly,
    )?;
    let booth_store = transaction
        .store("booths")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    let vendor_store = transaction
        .store("vendors")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    let purchase_store = transaction
        .store("purchases")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

    let booth_values = booth_store
        .get_all(None, None)
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    let vendor_values = vendor_store
        .get_all(None, None)
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    let purchase_values = purchase_store
        .get_all(None, None)
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

    transaction.done().await?;

    let mut issues = Vec::new();

    let booths = deserialize_collection::<domain::Booth>("booth", booth_values, &mut issues);
    let vendors = deserialize_collection::<Vendor>("vendor", vendor_values, &mut issues);
    let purchases = deserialize_collection::<Purchase>("purchase", purchase_values, &mut issues);

    let booth_ids: std::collections::HashSet<BoothId> =
        booths.iter().map(|booth| booth.id).collect();
    let vendor_pairs: std::collections::HashSet<(BoothId, VendorId)> = vendors
        .iter()
        .map(|vendor| (vendor.booth_id, vendor.vendor_id.clone()))
        .collect();

    for vendor in &vendors {
        if !booth_ids.contains(&vendor.booth_id) {
            issues.push(format!(
                "Vendor {} references missing booth {}",
                vendor.vendor_id, vendor.booth_id
            ));
        }
    }

    for purchase in &purchases {
        if !booth_ids.contains(&purchase.booth_id) {
            issues.push(format!(
                "Purchase {} references missing booth {}",
                purchase.id, purchase.booth_id
            ));
        }

        if purchase.items.is_empty() {
            issues.push(format!("Purchase {} contains no items", purchase.id));
        }

        for item in &purchase.items {
            if !vendor_pairs.contains(&(purchase.booth_id, item.vendor_id.clone())) {
                issues.push(format!(
                    "Purchase {} references missing vendor {} in booth {}",
                    purchase.id, item.vendor_id, purchase.booth_id
                ));
            }
        }
    }

    if issues.is_empty() {
        Ok(IntegrityStatus::Healthy)
    } else {
        Ok(IntegrityStatus::IssuesFound { issues })
    }
}

async fn load_last_backup_at(db: &Database) -> Result<Option<DateTime<Utc>>, StorageError> {
    load_optional_timestamp(db, LAST_BACKUP_AT_METADATA_KEY).await
}

async fn load_last_modified_at(db: &Database) -> Result<Option<DateTime<Utc>>, StorageError> {
    load_optional_timestamp(db, LAST_MODIFIED_AT_METADATA_KEY).await
}

async fn load_optional_timestamp(
    db: &Database,
    key: &str,
) -> Result<Option<DateTime<Utc>>, StorageError> {
    let result = load_metadata_raw_value(db, key).await?;
    match result {
        Some(raw_value) => {
            let timestamp = from_value(raw_value)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            Ok(Some(timestamp))
        }
        None => Ok(None),
    }
}

async fn load_metadata_raw_value(
    db: &Database,
    key: &str,
) -> Result<Option<JsValue>, StorageError> {
    let transaction = db.transaction(&["metadata"], TransactionMode::ReadOnly)?;
    let store = transaction
        .store("metadata")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

    let result = store
        .get(JsValue::from_str(key))
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

    transaction.done().await?;

    match result {
        Some(value) => {
            let entry = Object::from(value);
            let raw_value = Reflect::get(&entry, &JsValue::from_str("value"))
                .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
            Ok(Some(raw_value))
        }
        None => Ok(None),
    }
}

async fn write_metadata_value(
    db: &Database,
    key: &str,
    value: JsValue,
) -> Result<(), StorageError> {
    let transaction = db.transaction(&["metadata"], TransactionMode::ReadWrite)?;
    let store = transaction
        .store("metadata")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

    let metadata = metadata_value(key, value)?;

    store
        .put(&metadata, None)
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

    transaction.done().await?;

    Ok(())
}

fn metadata_value(key: &str, value: JsValue) -> Result<JsValue, StorageError> {
    let object = Object::new();
    Reflect::set(&object, &JsValue::from_str("key"), &JsValue::from_str(key))
        .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
    Reflect::set(&object, &JsValue::from_str("value"), &value)
        .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
    Ok(object.into())
}

fn deserialize_collection<T>(label: &str, values: Vec<JsValue>, issues: &mut Vec<String>) -> Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    values
        .into_iter()
        .enumerate()
        .filter_map(|(index, value)| match from_value::<T>(value) {
            Ok(entity) => Some(entity),
            Err(err) => {
                issues.push(format!(
                    "Failed to deserialize {label} record at index {index}: {err}"
                ));
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn storage_diagnostics_serializes_last_backup() {
        let diagnostics = StorageDiagnostics {
            database_version: 4,
            last_backup_at: Some(Utc.with_ymd_and_hms(2026, 4, 4, 12, 30, 0).unwrap()),
            last_modified_at: Some(Utc.with_ymd_and_hms(2026, 4, 5, 8, 0, 0).unwrap()),
        };

        let json = serde_json::to_string(&diagnostics).unwrap();

        assert!(json.contains("database_version"));
        assert!(json.contains("2026-04-04T12:30:00Z"));
        assert!(json.contains("last_modified_at"));
    }

    #[test]
    fn integrity_status_serializes_detected_issues() {
        let status = IntegrityStatus::IssuesFound {
            issues: vec!["missing booth".to_string()],
        };

        let json = serde_json::to_string(&status).unwrap();

        assert!(json.contains("missing booth"));
    }
}

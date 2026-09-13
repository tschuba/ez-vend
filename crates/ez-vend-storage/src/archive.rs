use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::{
    ArchivedBoothSummary, ArchivedVendorSummary, Booth, BoothId, Purchase, ValidationError, Vendor,
};
use log::{debug, info};
use rexie::TransactionMode;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::export::DeviceInfo;
use crate::indexeddb::Database;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRecord {
    pub booth_id: BoothId,
    pub timestamp: DateTime<Utc>,
    pub checksum: String,
    pub file_name: String,
    pub file_size_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveAuditEventType {
    Archived,
    Restored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchiveAuditEvent {
    pub id: String,
    pub event_type: ArchiveAuditEventType,
    pub booth_id: BoothId,
    pub booth_description: String,
    pub timestamp: DateTime<Utc>,
    pub device_info: DeviceInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_snapshot: Option<ArchivedBoothSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_checksum: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveSizeWarning {
    pub vendor_count: usize,
    pub purchase_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArchivePreview {
    pub booth: Booth,
    pub summary: ArchivedBoothSummary,
    pub export_record: Option<ExportRecord>,
    pub size_warning: Option<ArchiveSizeWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveOutcome {
    pub deleted_vendors: usize,
    pub deleted_purchases: usize,
    pub archived_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ArchiveService {
    db: Arc<Database>,
}

impl ArchiveService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn generate_preview(
        &self,
        booth_id: &BoothId,
    ) -> Result<ArchivePreview, StorageError> {
        let transaction = self.db.transaction(
            &["booths", "vendors", "purchases", "export_records"],
            TransactionMode::ReadOnly,
        )?;

        let booth = load_booth_from_transaction(&transaction, booth_id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("Booth not found: {}", booth_id)))?;

        if booth.is_archived() {
            return Err(StorageError::from(domain::DomainError::Validation(
                ValidationError::BoothAlreadyArchived,
            )));
        }

        let vendors = load_vendors_from_transaction(&transaction, booth_id).await?;
        let purchases = load_purchases_from_transaction(&transaction, booth_id).await?;
        let export_record = load_export_record_from_transaction(&transaction, booth_id).await?;

        transaction.done().await?;

        let summary = build_archived_summary(&booth, &vendors, &purchases);
        let size_warning = archive_size_warning(vendors.len(), purchases.len());

        Ok(ArchivePreview {
            booth,
            summary,
            export_record,
            size_warning,
        })
    }

    pub async fn archive_booth(
        &self,
        booth_id: &BoothId,
        export_record: ExportRecord,
        device_info: DeviceInfo,
    ) -> Result<ArchiveOutcome, StorageError> {
        let transaction = self.db.transaction(
            &[
                "booths",
                "vendors",
                "purchases",
                "export_records",
                "archive_audit_log",
            ],
            TransactionMode::ReadWrite,
        )?;

        let mut booth = load_booth_from_transaction(&transaction, booth_id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("Booth not found: {}", booth_id)))?;

        if booth.is_archived() {
            return Err(StorageError::from(domain::DomainError::Validation(
                ValidationError::BoothAlreadyArchived,
            )));
        }

        let vendors = load_vendors_from_transaction(&transaction, booth_id).await?;
        let purchases = load_purchases_from_transaction(&transaction, booth_id).await?;
        let summary = build_archived_summary(&booth, &vendors, &purchases);

        info!(
            "Archiving booth {} with {} vendors, {} purchases, revenue {}, booth revenue {}",
            booth_id,
            vendors.len(),
            purchases.len(),
            summary.total_revenue,
            summary.total_booth_revenue
        );

        booth.archive(summary.clone()).map_err(StorageError::from)?;
        debug!(
            "Archived booth {} summary attached: vendor_count={}, purchase_count={}, vendor_summaries={}",
            booth_id,
            booth.archived_summary
                .as_ref()
                .map(|summary| summary.vendor_count)
                .unwrap_or_default(),
            booth.archived_summary
                .as_ref()
                .map(|summary| summary.purchase_count)
                .unwrap_or_default(),
            booth.archived_summary
                .as_ref()
                .map(|summary| summary.vendor_summaries.len())
                .unwrap_or_default()
        );
        save_booth_in_transaction(&transaction, &booth).await?;
        save_export_record_in_transaction(&transaction, &export_record).await?;

        let deleted_vendors = delete_vendors_from_transaction(&transaction, booth_id).await?;
        let deleted_purchases = delete_purchases_from_transaction(&transaction, booth_id).await?;

        let archived_at = booth.archived_at.expect("archive timestamp set");
        write_archive_audit_event(
            &transaction,
            ArchiveAuditEvent {
                id: format!("archive:{}:{}", booth_id, archived_at.timestamp_millis()),
                event_type: ArchiveAuditEventType::Archived,
                booth_id: *booth_id,
                booth_description: booth.description.clone(),
                timestamp: archived_at,
                device_info,
                summary_snapshot: Some(summary),
                export_checksum: Some(export_record.checksum),
            },
        )
        .await?;

        transaction.done().await?;

        Ok(ArchiveOutcome {
            deleted_vendors,
            deleted_purchases,
            archived_at,
        })
    }

    pub async fn restore_booth(
        &self,
        booth_id: &BoothId,
        device_info: DeviceInfo,
    ) -> Result<(), StorageError> {
        let transaction = self
            .db
            .transaction(&["booths", "archive_audit_log"], TransactionMode::ReadWrite)?;

        let mut booth = load_booth_from_transaction(&transaction, booth_id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("Booth not found: {}", booth_id)))?;

        let summary_snapshot = booth.archived_summary.clone();
        booth.restore().map_err(StorageError::from)?;
        save_booth_in_transaction(&transaction, &booth).await?;

        write_archive_audit_event(
            &transaction,
            ArchiveAuditEvent {
                id: format!("restore:{}:{}", booth_id, Utc::now().timestamp_millis()),
                event_type: ArchiveAuditEventType::Restored,
                booth_id: *booth_id,
                booth_description: booth.description.clone(),
                timestamp: Utc::now(),
                device_info,
                summary_snapshot,
                export_checksum: None,
            },
        )
        .await?;

        transaction.done().await?;
        Ok(())
    }

    pub async fn delete_booth_with_cascade(&self, booth_id: &BoothId) -> Result<(), StorageError> {
        let transaction = self.db.transaction(
            &["booths", "vendors", "purchases"],
            TransactionMode::ReadWrite,
        )?;

        delete_vendors_from_transaction(&transaction, booth_id).await?;
        delete_purchases_from_transaction(&transaction, booth_id).await?;

        let store = transaction.store("booths").map_err(database_error)?;
        store
            .delete(JsValue::from_str(&booth_id.as_str()))
            .await
            .map_err(database_error)?;

        transaction.done().await?;
        Ok(())
    }

    pub async fn list_audit_events(&self) -> Result<Vec<ArchiveAuditEvent>, StorageError> {
        let transaction = self
            .db
            .transaction(&["archive_audit_log"], TransactionMode::ReadOnly)?;
        let store = transaction
            .store("archive_audit_log")
            .map_err(database_error)?;

        let values = store.get_all(None, None).await.map_err(database_error)?;
        transaction.done().await?;

        let mut entries: Vec<ArchiveAuditEvent> = values
            .into_iter()
            .map(|value| {
                from_value(value).map_err(|err| StorageError::SerializationError(err.to_string()))
            })
            .collect::<Result<_, _>>()?;
        entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        Ok(entries)
    }
}

pub fn archive_size_warning(
    vendor_count: usize,
    purchase_count: usize,
) -> Option<ArchiveSizeWarning> {
    if vendor_count > 1000 || purchase_count > 10_000 {
        Some(ArchiveSizeWarning {
            vendor_count,
            purchase_count,
        })
    } else {
        None
    }
}

pub fn build_archived_summary(
    booth: &Booth,
    vendors: &[Vendor],
    purchases: &[Purchase],
) -> ArchivedBoothSummary {
    debug!(
        "Building archived summary for booth {} from {} vendors and {} purchases",
        booth.id,
        vendors.len(),
        purchases.len()
    );

    let charging_config = domain::ChargingConfig::from_booth(booth);
    let mut gross_sales_by_vendor: HashMap<_, VendorPayoutAccumulator> = HashMap::new();
    let mut first_purchase_at: Option<DateTime<Utc>> = None;
    let mut last_purchase_at: Option<DateTime<Utc>> = None;
    let mut item_count = 0usize;

    for purchase in purchases {
        first_purchase_at = Some(match first_purchase_at {
            Some(current) => current.min(purchase.timestamp),
            None => purchase.timestamp,
        });
        last_purchase_at = Some(match last_purchase_at {
            Some(current) => current.max(purchase.timestamp),
            None => purchase.timestamp,
        });

        for item in &purchase.items {
            item_count += 1;
            let entry = gross_sales_by_vendor
                .entry(item.vendor_id.clone())
                .or_default();
            entry.gross_sales += item.amount;
            entry.item_count += 1;
        }
    }

    let mut vendor_summaries: Vec<ArchivedVendorSummary> = gross_sales_by_vendor
        .into_iter()
        .map(|(vendor_id, accumulator)| {
            let payout = charging_config.calculate_payout(accumulator.gross_sales);
            ArchivedVendorSummary {
                vendor_id,
                gross_sales: payout.gross_sales,
                fees_due: payout.fees_due,
                net_payout: payout.net_payout,
                item_count: accumulator.item_count,
            }
        })
        .collect();
    vendor_summaries.sort_by(|left, right| left.vendor_id.cmp(&right.vendor_id));

    let summary = ArchivedBoothSummary {
        total_revenue: vendor_summaries
            .iter()
            .map(|summary| summary.gross_sales)
            .sum(),
        total_booth_revenue: vendor_summaries
            .iter()
            .map(|summary| summary.fees_due)
            .sum(),
        vendor_count: vendor_summaries.len(),
        purchase_count: purchases.len(),
        item_count,
        first_purchase_at,
        last_purchase_at,
        vendor_summaries,
    };

    debug!(
        "Built archived summary for booth {}: revenue={}, booth_revenue={}, vendors={}, purchases={}, items={}",
        booth.id,
        summary.total_revenue,
        summary.total_booth_revenue,
        summary.vendor_count,
        summary.purchase_count,
        summary.item_count
    );

    summary
}

#[derive(Default)]
struct VendorPayoutAccumulator {
    gross_sales: rust_decimal::Decimal,
    item_count: usize,
}

async fn load_booth_from_transaction(
    transaction: &rexie::Transaction,
    booth_id: &BoothId,
) -> Result<Option<Booth>, StorageError> {
    let store = transaction.store("booths").map_err(database_error)?;
    let result = store
        .get(JsValue::from_str(&booth_id.as_str()))
        .await
        .map_err(database_error)?;

    match result {
        Some(value) => from_value(value)
            .map(Some)
            .map_err(|err| StorageError::SerializationError(err.to_string())),
        None => Ok(None),
    }
}

async fn save_booth_in_transaction(
    transaction: &rexie::Transaction,
    booth: &Booth,
) -> Result<(), StorageError> {
    let store = transaction.store("booths").map_err(database_error)?;
    let value = to_value(booth).map_err(|err| StorageError::SerializationError(err.to_string()))?;
    store.put(&value, None).await.map_err(database_error)?;
    Ok(())
}

async fn load_vendors_from_transaction(
    transaction: &rexie::Transaction,
    booth_id: &BoothId,
) -> Result<Vec<Vendor>, StorageError> {
    let store = transaction.store("vendors").map_err(database_error)?;
    let index = store.index("booth_id").map_err(database_error)?;
    let key = JsValue::from_str(&booth_id.as_str());
    let values = index
        .get_all(
            Some(rexie::KeyRange::only(&key).map_err(StorageError::from)?),
            None,
        )
        .await
        .map_err(database_error)?;

    values
        .into_iter()
        .map(|value| {
            from_value(value).map_err(|err| StorageError::SerializationError(err.to_string()))
        })
        .collect()
}

async fn load_purchases_from_transaction(
    transaction: &rexie::Transaction,
    booth_id: &BoothId,
) -> Result<Vec<Purchase>, StorageError> {
    let store = transaction.store("purchases").map_err(database_error)?;
    let index = store.index("booth_id").map_err(database_error)?;
    let key = JsValue::from_str(&booth_id.as_str());
    let values = index
        .get_all(
            Some(rexie::KeyRange::only(&key).map_err(StorageError::from)?),
            None,
        )
        .await
        .map_err(database_error)?;

    values
        .into_iter()
        .map(|value| {
            from_value(value).map_err(|err| StorageError::SerializationError(err.to_string()))
        })
        .collect()
}

async fn delete_vendors_from_transaction(
    transaction: &rexie::Transaction,
    booth_id: &BoothId,
) -> Result<usize, StorageError> {
    let vendors = load_vendors_from_transaction(transaction, booth_id).await?;
    let store = transaction.store("vendors").map_err(database_error)?;

    for vendor in &vendors {
        let key = js_sys::Array::new();
        key.push(&JsValue::from_str(&booth_id.as_str()));
        key.push(&JsValue::from_str(vendor.vendor_id.as_str()));
        store.delete(key.into()).await.map_err(database_error)?;
    }

    Ok(vendors.len())
}

async fn delete_purchases_from_transaction(
    transaction: &rexie::Transaction,
    booth_id: &BoothId,
) -> Result<usize, StorageError> {
    let purchases = load_purchases_from_transaction(transaction, booth_id).await?;
    let store = transaction.store("purchases").map_err(database_error)?;

    for purchase in &purchases {
        let key = js_sys::Array::new();
        key.push(&JsValue::from_str(&booth_id.as_str()));
        key.push(&JsValue::from_str(&purchase.id.as_str()));
        store.delete(key.into()).await.map_err(database_error)?;
    }

    Ok(purchases.len())
}

async fn load_export_record_from_transaction(
    transaction: &rexie::Transaction,
    booth_id: &BoothId,
) -> Result<Option<ExportRecord>, StorageError> {
    let store = transaction
        .store("export_records")
        .map_err(database_error)?;
    let result = store
        .get(JsValue::from_str(&booth_id.as_str()))
        .await
        .map_err(database_error)?;

    match result {
        Some(value) => from_value(value)
            .map(Some)
            .map_err(|err| StorageError::SerializationError(err.to_string())),
        None => Ok(None),
    }
}

pub async fn save_export_record(db: &Database, record: &ExportRecord) -> Result<(), StorageError> {
    let transaction = db.transaction(&["export_records"], TransactionMode::ReadWrite)?;
    save_export_record_in_transaction(&transaction, record).await?;
    transaction.done().await?;
    Ok(())
}

async fn save_export_record_in_transaction(
    transaction: &rexie::Transaction,
    record: &ExportRecord,
) -> Result<(), StorageError> {
    let store = transaction
        .store("export_records")
        .map_err(database_error)?;
    let value =
        to_value(record).map_err(|err| StorageError::SerializationError(err.to_string()))?;
    store.put(&value, None).await.map_err(database_error)?;
    Ok(())
}

async fn write_archive_audit_event(
    transaction: &rexie::Transaction,
    event: ArchiveAuditEvent,
) -> Result<(), StorageError> {
    let store = transaction
        .store("archive_audit_log")
        .map_err(database_error)?;
    let value =
        to_value(&event).map_err(|err| StorageError::SerializationError(err.to_string()))?;
    store.put(&value, None).await.map_err(database_error)?;
    Ok(())
}

fn database_error(error: rexie::Error) -> StorageError {
    StorageError::DatabaseError(format!("{:?}", error))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use domain::{Booth, FeeConfig, PurchaseItem, VendorId};
    use rust_decimal_macros::dec;

    use super::*;

    fn booth() -> Booth {
        Booth::new(
            "Spring Fair".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
            FeeConfig {
                participation_fee: dec!(5.00),
                sales_fee_percent: dec!(10.00),
                rounding_step: dec!(0.50),
            },
        )
        .unwrap()
    }

    #[test]
    fn archive_size_warning_applies_thresholds() {
        assert!(archive_size_warning(1001, 10).is_some());
        assert!(archive_size_warning(10, 10001).is_some());
        assert!(archive_size_warning(1000, 10000).is_none());
    }

    #[test]
    fn archived_summary_keeps_vendor_totals() {
        let booth = booth();
        let vendors = vec![Vendor {
            vendor_id: VendorId::new("101".to_string()),
            booth_id: booth.id,
            created_at: Utc::now(),
            payout_correction: None,
            payout_correction_note: None,
        }];
        let purchases = vec![Purchase {
            id: domain::PurchaseId::new(),
            booth_id: booth.id,
            items: vec![PurchaseItem::new(dec!(20.00), VendorId::new("101".to_string())).unwrap()],
            timestamp: Utc::now(),
            note: None,
        }];

        let summary = build_archived_summary(&booth, &vendors, &purchases);

        assert_eq!(summary.vendor_count, 1);
        assert_eq!(summary.purchase_count, 1);
        assert_eq!(summary.item_count, 1);
        assert_eq!(summary.vendor_summaries[0].vendor_id.as_str(), "101");
        assert_eq!(summary.total_revenue, dec!(20.00));
    }
}

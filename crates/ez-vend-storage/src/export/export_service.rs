use std::collections::HashSet;
use std::sync::Arc;

use domain::{
    BoothId, BoothRepository, Purchase, PurchaseItem, PurchaseRepository, Vendor, VendorId,
    VendorRepository,
};
use log::info;

use super::backup_format::{
    generate_booth_backup_filename, generate_booth_backup_filename_with_device,
    generate_full_backup_filename, generate_full_backup_filename_with_device, BackupData,
    BoothBackupData,
};
use super::checksum::{compute_backup_checksum, compute_booth_checksum};
use super::error::ExportError;
use crate::archive::ExportRecord;
use crate::indexeddb::Database;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedBackup {
    pub file_name: String,
    pub json: String,
}

#[derive(Clone)]
pub struct ExportService {
    booth_repository: Arc<dyn BoothRepository>,
    vendor_repository: Arc<dyn VendorRepository>,
    purchase_repository: Arc<dyn PurchaseRepository>,
    database: Option<Arc<Database>>,
    app_version: String,
}

impl ExportService {
    pub fn new(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
    ) -> Self {
        Self::with_app_version(
            booth_repository,
            vendor_repository,
            purchase_repository,
            None,
            env!("CARGO_PKG_VERSION"),
        )
    }

    pub fn with_database(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
        database: Arc<Database>,
    ) -> Self {
        Self::with_app_version(
            booth_repository,
            vendor_repository,
            purchase_repository,
            Some(database),
            env!("CARGO_PKG_VERSION"),
        )
    }

    pub fn with_app_version(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
        database: Option<Arc<Database>>,
        app_version: impl Into<String>,
    ) -> Self {
        Self {
            booth_repository,
            vendor_repository,
            purchase_repository,
            database,
            app_version: app_version.into(),
        }
    }

    pub async fn export_all(&self) -> Result<BackupData, ExportError> {
        let mut data = BackupData::new(self.app_version.clone());

        let booths = self.booth_repository.find_all().await?;
        let vendors = self.vendor_repository.find_all().await?;
        let purchases = self.purchase_repository.find_all().await?;

        let (booths, vendors, purchases) =
            Self::filter_orphaned_records(booths, vendors, purchases);

        data.booths = booths;
        data.vendors = vendors;
        data.purchases = purchases;

        Ok(data)
    }

    pub async fn export_booth(&self, booth_id: &BoothId) -> Result<BoothBackupData, ExportError> {
        let booth = self
            .booth_repository
            .find_by_id(booth_id)
            .await?
            .ok_or(ExportError::BoothNotFound(*booth_id))?;

        let mut data = BoothBackupData::new(booth.clone(), self.app_version.clone());

        let vendors = self.vendor_repository.find_by_booth(booth_id).await?;
        let purchases = self.purchase_repository.find_by_booth(booth_id).await?;

        let (vendors, purchases) =
            Self::filter_booth_orphaned_purchases(*booth_id, vendors, purchases);

        data.vendors = vendors;
        data.purchases = purchases;

        Ok(data)
    }

    pub fn serialize_full_backup(
        &self,
        data: &BackupData,
    ) -> Result<SerializedBackup, ExportError> {
        self.serialize_full_backup_with_device_identifier(
            data,
            data.device_info
                .as_ref()
                .map(|info| info.identifier.as_str()),
        )
    }

    pub fn serialize_full_backup_with_device_identifier(
        &self,
        data: &BackupData,
        device_identifier: Option<&str>,
    ) -> Result<SerializedBackup, ExportError> {
        let mut payload = data.clone();
        let checksum = compute_backup_checksum(&payload)
            .map_err(|err| ExportError::Serialization(err.to_string()))?;
        info!("Computed checksum for full backup: {}", checksum);
        payload.checksum = Some(checksum);

        Ok(SerializedBackup {
            file_name: if device_identifier.is_some() {
                generate_full_backup_filename_with_device(data.created_at, device_identifier)
            } else {
                generate_full_backup_filename(data.created_at)
            },
            json: serde_json::to_string_pretty(&payload)
                .map_err(|err| ExportError::Serialization(err.to_string()))?,
        })
    }

    pub fn serialize_booth_backup(
        &self,
        data: &BoothBackupData,
    ) -> Result<SerializedBackup, ExportError> {
        self.serialize_booth_backup_with_device_identifier(
            data,
            data.device_info
                .as_ref()
                .map(|info| info.identifier.as_str()),
        )
    }

    pub fn serialize_booth_backup_with_device_identifier(
        &self,
        data: &BoothBackupData,
        device_identifier: Option<&str>,
    ) -> Result<SerializedBackup, ExportError> {
        let mut payload = data.clone();
        let checksum = compute_booth_checksum(&payload)
            .map_err(|err| ExportError::Serialization(err.to_string()))?;
        info!("Computed checksum for booth backup: {}", checksum);
        payload.checksum = Some(checksum);

        Ok(SerializedBackup {
            file_name: if device_identifier.is_some() {
                generate_booth_backup_filename_with_device(
                    &data.booth.id,
                    &data.booth.description,
                    data.created_at,
                    device_identifier,
                )
            } else {
                generate_booth_backup_filename(
                    &data.booth.id,
                    &data.booth.description,
                    data.created_at,
                )
            },
            json: serde_json::to_string_pretty(&payload)
                .map_err(|err| ExportError::Serialization(err.to_string()))?,
        })
    }

    pub async fn record_booth_export(
        &self,
        booth_id: &BoothId,
        serialized: &SerializedBackup,
    ) -> Result<Option<ExportRecord>, ExportError> {
        let Some(database) = &self.database else {
            return Ok(None);
        };

        let checksum = serde_json::from_str::<serde_json::Value>(&serialized.json)
            .ok()
            .and_then(|value| {
                value
                    .get("checksum")
                    .and_then(|checksum| checksum.as_str().map(|value| value.to_string()))
            })
            .ok_or_else(|| ExportError::Serialization("missing export checksum".to_string()))?;

        let record = ExportRecord {
            booth_id: *booth_id,
            timestamp: chrono::Utc::now(),
            checksum,
            file_name: serialized.file_name.clone(),
            file_size_bytes: serialized.json.len(),
        };

        crate::archive::save_export_record(database, &record)
            .await
            .map_err(|err| ExportError::Serialization(err.to_string()))?;

        Ok(Some(record))
    }

    fn filter_orphaned_records(
        booths: Vec<domain::Booth>,
        vendors: Vec<Vendor>,
        purchases: Vec<Purchase>,
    ) -> (Vec<domain::Booth>, Vec<Vendor>, Vec<Purchase>) {
        let booth_ids: HashSet<BoothId> = booths.iter().map(|booth| booth.id).collect();

        let mut skipped_vendor_count = 0;
        let valid_vendors: Vec<Vendor> = vendors
            .into_iter()
            .filter(|vendor| {
                if booth_ids.contains(&vendor.booth_id) {
                    true
                } else {
                    skipped_vendor_count += 1;
                    info!(
                        "Skipping vendor {} during export because booth {} is missing",
                        vendor.vendor_id, vendor.booth_id
                    );
                    false
                }
            })
            .collect();

        let vendor_pairs: HashSet<(BoothId, VendorId)> = valid_vendors
            .iter()
            .map(|vendor| (vendor.booth_id, vendor.vendor_id.clone()))
            .collect();

        let mut skipped_missing_booth_purchase_count = 0;
        let mut skipped_empty_purchase_count = 0;
        let mut skipped_missing_vendor_item_count = 0;
        let valid_purchases: Vec<Purchase> = purchases
            .into_iter()
            .filter_map(|purchase| {
                if !booth_ids.contains(&purchase.booth_id) {
                    skipped_missing_booth_purchase_count += 1;
                    info!(
                        "Skipping purchase {} during export because booth {} is missing",
                        purchase.id, purchase.booth_id
                    );
                    return None;
                }

                let purchase_id = purchase.id;
                let booth_id = purchase.booth_id;
                let (purchase, skipped_items) =
                    Self::filter_purchase_items(purchase, |item| {
                        vendor_pairs.contains(&(booth_id, item.vendor_id.clone()))
                    });

                if skipped_items > 0 {
                    skipped_missing_vendor_item_count += skipped_items;
                    info!(
                        "Removed {} orphaned items from purchase {} during export because their vendors are missing for booth {}",
                        skipped_items, purchase_id, booth_id
                    );
                }

                if let Some(purchase) = purchase {
                    Some(purchase)
                } else {
                    skipped_empty_purchase_count += 1;
                    info!(
                        "Skipping purchase {} during export because all items reference missing vendors for booth {}",
                        purchase_id, booth_id
                    );
                    None
                }
            })
            .collect();

        let skipped_total = skipped_vendor_count
            + skipped_missing_booth_purchase_count
            + skipped_empty_purchase_count;

        if skipped_total > 0 || skipped_missing_vendor_item_count > 0 {
            info!(
                "Export skipped {} orphaned records and removed {} orphaned items ({} vendors, {} purchases with missing booths, {} emptied purchases)",
                skipped_total,
                skipped_missing_vendor_item_count,
                skipped_vendor_count,
                skipped_missing_booth_purchase_count,
                skipped_empty_purchase_count
            );
        }

        (booths, valid_vendors, valid_purchases)
    }

    fn filter_booth_orphaned_purchases(
        booth_id: BoothId,
        vendors: Vec<Vendor>,
        purchases: Vec<Purchase>,
    ) -> (Vec<Vendor>, Vec<Purchase>) {
        let vendor_ids: HashSet<VendorId> = vendors
            .iter()
            .map(|vendor| vendor.vendor_id.clone())
            .collect();

        let mut skipped_purchase_count = 0;
        let mut skipped_item_count = 0;
        let valid_purchases: Vec<Purchase> = purchases
            .into_iter()
            .filter_map(|purchase| {
                let purchase_id = purchase.id;
                let (purchase, skipped_items) =
                    Self::filter_purchase_items(purchase, |item| vendor_ids.contains(&item.vendor_id));

                if skipped_items > 0 {
                    skipped_item_count += skipped_items;
                    info!(
                        "Removed {} orphaned items from purchase {} during booth export because their vendors are missing for booth {}",
                        skipped_items, purchase_id, booth_id
                    );
                }

                if let Some(purchase) = purchase {
                    Some(purchase)
                } else {
                    skipped_purchase_count += 1;
                    info!(
                        "Skipping purchase {} during booth export because all items reference missing vendors for booth {}",
                        purchase_id, booth_id
                    );
                    None
                }
            })
            .collect();

        if skipped_purchase_count > 0 || skipped_item_count > 0 {
            info!(
                "Booth export for {} skipped {} emptied purchases and removed {} orphaned items",
                booth_id, skipped_purchase_count, skipped_item_count
            );
        }

        (vendors, valid_purchases)
    }

    fn filter_purchase_items<F>(
        mut purchase: Purchase,
        mut keep_item: F,
    ) -> (Option<Purchase>, usize)
    where
        F: FnMut(&PurchaseItem) -> bool,
    {
        let original_len = purchase.items.len();
        purchase.items.retain(|item| keep_item(item));
        let skipped_items = original_len.saturating_sub(purchase.items.len());

        if purchase.items.is_empty() {
            (None, skipped_items)
        } else {
            (Some(purchase), skipped_items)
        }
    }
}

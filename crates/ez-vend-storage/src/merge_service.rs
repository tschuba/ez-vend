use std::sync::Arc;

use domain::{
    Booth, BoothId, BoothRepository, Purchase, PurchaseRepository, Vendor, VendorId,
    VendorRepository,
};
use js_sys;
use rexie::TransactionMode;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::indexeddb::Database;

#[derive(Clone)]
pub struct MergeService {
    booth_repository: Arc<dyn BoothRepository>,
    vendor_repository: Arc<dyn VendorRepository>,
    purchase_repository: Arc<dyn PurchaseRepository>,
    db: Option<Arc<Database>>,
}

impl MergeService {
    pub fn new(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
    ) -> Self {
        Self {
            booth_repository,
            vendor_repository,
            purchase_repository,
            db: None,
        }
    }

    pub fn with_database(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
        db: Arc<Database>,
    ) -> Self {
        Self {
            booth_repository,
            vendor_repository,
            purchase_repository,
            db: Some(db),
        }
    }

    /// Permanently merges `other` into `canonical` in a single atomic transaction.
    /// Vendors are consolidated using find-or-save (no validation).
    /// All purchases are re-assigned to `canonical`. The `other` booth is deleted.
    pub async fn merge_booths(
        &self,
        canonical_id: &BoothId,
        other_id: &BoothId,
    ) -> Result<(), StorageError> {
        let canonical = self
            .booth_repository
            .find_by_id(canonical_id)
            .await?
            .ok_or_else(|| {
                StorageError::NotFound(format!("Canonical booth not found: {}", canonical_id))
            })?;
        let other = self
            .booth_repository
            .find_by_id(other_id)
            .await?
            .ok_or_else(|| {
                StorageError::NotFound(format!("Other booth not found: {}", other_id))
            })?;

        let other_vendors = self.vendor_repository.find_by_booth(other_id).await?;
        let other_purchases = self.purchase_repository.find_by_booth(other_id).await?;

        if let Some(db) = &self.db {
            self.merge_transactional(db, &canonical, &other, other_vendors, other_purchases)
                .await
        } else {
            self.merge_sequential(&canonical, &other, other_vendors, other_purchases)
                .await
        }
    }

    // ─── Transactional path ──────────────────────────────────────────────────

    async fn merge_transactional(
        &self,
        db: &Database,
        canonical: &Booth,
        other: &Booth,
        other_vendors: Vec<Vendor>,
        other_purchases: Vec<Purchase>,
    ) -> Result<(), StorageError> {
        let transaction = db.transaction(
            &["booths", "vendors", "purchases"],
            TransactionMode::ReadWrite,
        )?;

        // Update canonical booth metadata: keep canonical ID, prefer newer updated_at
        let merged_booth = if other.updated_at > canonical.updated_at {
            Booth {
                id: canonical.id,
                ..other.clone()
            }
        } else {
            canonical.clone()
        };
        tx_save_booth(&transaction, &merged_booth).await?;

        // Consolidate vendors: find-or-save without validation
        for vendor in other_vendors {
            let remapped = Vendor {
                booth_id: canonical.id,
                ..vendor
            };
            match tx_find_vendor(&transaction, &canonical.id, &remapped.vendor_id).await? {
                None => {
                    tx_save_vendor(&transaction, &remapped).await?;
                }
                Some(existing) => {
                    // Apply payout_correction merge rules: keep canonical's if set
                    let merged_correction = if existing.payout_correction.is_some() {
                        existing.payout_correction
                    } else {
                        remapped.payout_correction
                    };
                    let merged_note = if existing.payout_correction_note.is_some() {
                        existing.payout_correction_note.clone()
                    } else {
                        remapped.payout_correction_note.clone()
                    };
                    if merged_correction != existing.payout_correction
                        || merged_note != existing.payout_correction_note
                    {
                        let updated = Vendor {
                            payout_correction: merged_correction,
                            payout_correction_note: merged_note,
                            ..existing
                        };
                        tx_save_vendor(&transaction, &updated).await?;
                    }
                }
            }
        }

        // Re-assign all purchases from other to canonical
        for purchase in other_purchases {
            let remapped = Purchase {
                booth_id: canonical.id,
                ..purchase
            };
            tx_save_purchase(&transaction, &remapped).await?;
        }

        // Delete the non-canonical booth
        tx_delete_booth(&transaction, &other.id).await?;

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(())
    }

    // ─── Sequential path (test/fallback) ────────────────────────────────────

    async fn merge_sequential(
        &self,
        canonical: &Booth,
        other: &Booth,
        other_vendors: Vec<Vendor>,
        other_purchases: Vec<Purchase>,
    ) -> Result<(), StorageError> {
        // Update canonical booth metadata
        let merged_booth = if other.updated_at > canonical.updated_at {
            Booth {
                id: canonical.id,
                ..other.clone()
            }
        } else {
            canonical.clone()
        };
        self.booth_repository.save(&merged_booth).await?;

        // Consolidate vendors
        for vendor in other_vendors {
            let remapped = Vendor {
                booth_id: canonical.id,
                ..vendor
            };
            match self
                .vendor_repository
                .find_by_id(&canonical.id, &remapped.vendor_id)
                .await?
            {
                None => {
                    self.vendor_repository.save(&remapped).await?;
                }
                Some(existing) => {
                    let merged_correction = if existing.payout_correction.is_some() {
                        existing.payout_correction
                    } else {
                        remapped.payout_correction
                    };
                    let merged_note = if existing.payout_correction_note.is_some() {
                        existing.payout_correction_note.clone()
                    } else {
                        remapped.payout_correction_note.clone()
                    };
                    if merged_correction != existing.payout_correction
                        || merged_note != existing.payout_correction_note
                    {
                        let updated = Vendor {
                            payout_correction: merged_correction,
                            payout_correction_note: merged_note,
                            ..existing
                        };
                        self.vendor_repository.save(&updated).await?;
                    }
                }
            }
        }

        // Re-assign all purchases
        for purchase in other_purchases {
            let remapped = Purchase {
                booth_id: canonical.id,
                ..purchase
            };
            self.purchase_repository.save(&remapped).await?;
        }

        // Delete the non-canonical booth
        self.booth_repository.delete(&other.id).await?;

        Ok(())
    }
}

// ─── Transaction-level helpers ───────────────────────────────────────────────

async fn tx_save_booth(tx: &rexie::Transaction, booth: &Booth) -> Result<(), StorageError> {
    let store = tx
        .store("booths")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    let value = to_value(booth).map_err(|e| StorageError::SerializationError(e.to_string()))?;
    store
        .put(&value, None)
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    Ok(())
}

async fn tx_delete_booth(tx: &rexie::Transaction, id: &BoothId) -> Result<(), StorageError> {
    let store = tx
        .store("booths")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    store
        .delete(JsValue::from_str(&id.as_str()))
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    Ok(())
}

fn vendor_compound_key(booth_id: &BoothId, vendor_id: &VendorId) -> JsValue {
    let arr = js_sys::Array::new();
    arr.push(&JsValue::from_str(&booth_id.as_str()));
    arr.push(&JsValue::from_str(vendor_id.as_str()));
    arr.into()
}

async fn tx_find_vendor(
    tx: &rexie::Transaction,
    booth_id: &BoothId,
    vendor_id: &VendorId,
) -> Result<Option<Vendor>, StorageError> {
    let store = tx
        .store("vendors")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    match store
        .get(vendor_compound_key(booth_id, vendor_id))
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?
    {
        Some(v) => {
            Ok(Some(from_value(v).map_err(|e| {
                StorageError::SerializationError(e.to_string())
            })?))
        }
        None => Ok(None),
    }
}

async fn tx_save_vendor(tx: &rexie::Transaction, vendor: &Vendor) -> Result<(), StorageError> {
    let store = tx
        .store("vendors")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    let value = to_value(vendor).map_err(|e| StorageError::SerializationError(e.to_string()))?;
    store
        .put(&value, None)
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    Ok(())
}

async fn tx_save_purchase(
    tx: &rexie::Transaction,
    purchase: &Purchase,
) -> Result<(), StorageError> {
    let store = tx
        .store("purchases")
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    let value = to_value(purchase).map_err(|e| StorageError::SerializationError(e.to_string()))?;
    store
        .put(&value, None)
        .await
        .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
    Ok(())
}

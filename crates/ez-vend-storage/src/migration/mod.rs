mod error;
mod schema_mapper;
mod sqlite_parser;
mod types;
mod validator;

use std::collections::HashSet;
use std::sync::Arc;

use domain::{Booth, BoothRepository, Purchase, PurchaseRepository, Vendor, VendorRepository};
use rexie::TransactionMode;
use serde_wasm_bindgen::to_value;

use crate::error::StorageError;
use crate::indexeddb::Database;
pub use error::MigrationError;
pub use schema_mapper::{map_booths, map_purchases, map_vendors};
pub use sqlite_parser::SqliteParser;
pub use types::{LegacyBooth, LegacyPurchase, LegacyPurchaseItem, LegacyVendor};
pub use validator::{validate_dataset, MigrationValidationSummary, ValidationIssue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationIssueStrategy {
    Cancel,
    SkipInvalid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationParseSummary {
    pub booths: Vec<Booth>,
    pub vendors: Vec<Vendor>,
    pub purchases: Vec<Purchase>,
    pub validation: MigrationValidationSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationResult {
    pub booths_migrated: usize,
    pub vendors_migrated: usize,
    pub purchases_migrated: usize,
}

#[derive(Clone)]
pub struct MigrationService {
    db: Option<Arc<Database>>,
    booth_repository: Arc<dyn BoothRepository>,
    vendor_repository: Arc<dyn VendorRepository>,
    purchase_repository: Arc<dyn PurchaseRepository>,
}

impl MigrationService {
    pub fn new(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
    ) -> Self {
        Self {
            db: None,
            booth_repository,
            vendor_repository,
            purchase_repository,
        }
    }

    pub fn new_with_database(
        db: Arc<Database>,
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
    ) -> Self {
        Self {
            db: Some(db),
            booth_repository,
            vendor_repository,
            purchase_repository,
        }
    }

    pub fn parse_and_validate(
        &self,
        bytes: Vec<u8>,
    ) -> Result<MigrationParseSummary, MigrationError> {
        let parser = SqliteParser::open_database(bytes)?;
        let legacy_booths = parser.parse_booths()?;
        let legacy_vendors = parser.parse_vendors()?;
        let legacy_purchases = parser.parse_purchases()?;

        let booths = schema_mapper::map_booths(&legacy_booths)?;
        let vendors = schema_mapper::map_vendors(&legacy_vendors)?;
        let purchases = schema_mapper::map_purchases(&legacy_purchases)?;
        let validation =
            validator::validate_dataset(&booths, &vendors, &purchases, &legacy_purchases);

        Ok(MigrationParseSummary {
            booths,
            vendors,
            purchases,
            validation,
        })
    }

    pub fn prepare_import(
        &self,
        summary: MigrationParseSummary,
        strategy: MigrationIssueStrategy,
    ) -> Result<MigrationParseSummary, MigrationError> {
        if summary.validation.issues.is_empty() {
            return Ok(summary);
        }

        match strategy {
            MigrationIssueStrategy::Cancel => Err(MigrationError::ReplaceFailed(
                "validation issues require operator confirmation before import".to_string(),
            )),
            MigrationIssueStrategy::SkipInvalid => Ok(filter_invalid_records(summary)),
        }
    }

    pub async fn replace_all(
        &self,
        booths: Vec<Booth>,
        vendors: Vec<Vendor>,
        purchases: Vec<Purchase>,
    ) -> Result<MigrationResult, MigrationError> {
        if let Some(db) = &self.db {
            replace_all_atomically(db, &booths, &vendors, &purchases).await?;
        } else {
            self.delete_existing_data().await?;

            for booth in &booths {
                self.booth_repository.save(booth).await?;
            }

            for vendor in &vendors {
                self.vendor_repository.save(vendor).await?;
            }

            for purchase in &purchases {
                self.purchase_repository.save(purchase).await?;
            }
        }

        Ok(MigrationResult {
            booths_migrated: booths.len(),
            vendors_migrated: vendors.len(),
            purchases_migrated: purchases.len(),
        })
    }

    async fn delete_existing_data(&self) -> Result<(), MigrationError> {
        let purchases = self.purchase_repository.find_all().await?;
        for purchase in purchases {
            self.purchase_repository
                .delete_from_booth(&purchase.booth_id, &purchase.id)
                .await?;
        }

        let vendors = self.vendor_repository.find_all().await?;
        for vendor in vendors {
            self.vendor_repository
                .delete_from_booth(&vendor.booth_id, &vendor.vendor_id)
                .await?;
        }

        let booths = self.booth_repository.find_all().await?;
        let mut deleted = HashSet::new();
        for booth in booths {
            if deleted.insert(booth.id) {
                self.booth_repository.delete(&booth.id).await?;
            }
        }

        Ok(())
    }
}

async fn replace_all_atomically(
    db: &Database,
    booths: &[Booth],
    vendors: &[Vendor],
    purchases: &[Purchase],
) -> Result<(), MigrationError> {
    replace_all_atomically_with_options(db, booths, vendors, purchases, false).await
}

async fn replace_all_atomically_with_options(
    db: &Database,
    booths: &[Booth],
    vendors: &[Vendor],
    purchases: &[Purchase],
    abort_after_clear: bool,
) -> Result<(), MigrationError> {
    let transaction = db
        .transaction(
            &["booths", "vendors", "purchases"],
            TransactionMode::ReadWrite,
        )
        .map_err(migration_storage_error)?;

    let booth_store = transaction
        .store("booths")
        .map_err(migration_storage_error)?;
    let vendor_store = transaction
        .store("vendors")
        .map_err(migration_storage_error)?;
    let purchase_store = transaction
        .store("purchases")
        .map_err(migration_storage_error)?;

    purchase_store
        .clear()
        .await
        .map_err(migration_storage_error)?;
    vendor_store
        .clear()
        .await
        .map_err(migration_storage_error)?;
    booth_store.clear().await.map_err(migration_storage_error)?;

    if abort_after_clear {
        transaction.abort().await.map_err(migration_storage_error)?;
        return Err(MigrationError::ReplaceFailed(
            "simulated transaction failure after clearing stores".to_string(),
        ));
    }

    for booth in booths {
        let value = to_value(booth).map_err(|error| {
            MigrationError::ReplaceFailed(
                StorageError::SerializationError(error.to_string()).to_string(),
            )
        })?;
        booth_store
            .put(&value, None)
            .await
            .map_err(migration_storage_error)?;
    }

    for vendor in vendors {
        let value = to_value(vendor).map_err(|error| {
            MigrationError::ReplaceFailed(
                StorageError::SerializationError(error.to_string()).to_string(),
            )
        })?;
        vendor_store
            .put(&value, None)
            .await
            .map_err(migration_storage_error)?;
    }

    for purchase in purchases {
        let value = to_value(purchase).map_err(|error| {
            MigrationError::ReplaceFailed(
                StorageError::SerializationError(error.to_string()).to_string(),
            )
        })?;
        purchase_store
            .put(&value, None)
            .await
            .map_err(migration_storage_error)?;
    }

    transaction.done().await.map_err(migration_storage_error)?;

    Ok(())
}

fn migration_storage_error(error: impl Into<StorageError>) -> MigrationError {
    MigrationError::ReplaceFailed(error.into().to_string())
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use domain::repositories::{BoothRepository, PurchaseRepository, VendorRepository};
    use domain::{Booth, FeeConfig, Purchase, PurchaseItem, Vendor, VendorId};
    use rust_decimal_macros::dec;
    use wasm_bindgen_test::*;

    use super::replace_all_atomically_with_options;
    use crate::indexeddb::Database;
    use crate::repositories::{
        IndexedDbBoothRepository, IndexedDbPurchaseRepository, IndexedDbVendorRepository,
    };

    wasm_bindgen_test_configure!(run_in_browser);

    async fn create_test_db() -> Database {
        let db_name = format!("test_migration_atomic_db_{}", js_sys::Math::random());
        Database::new_with_name(&db_name)
            .await
            .expect("Failed to create test database")
    }

    fn test_booth(description: &str) -> Booth {
        Booth::new(
            description.to_string(),
            NaiveDate::from_ymd_opt(2026, 4, 6).unwrap(),
            FeeConfig {
                participation_fee: dec!(10.00),
                sales_fee_percent: dec!(15.00),
                rounding_step: dec!(0.50),
            },
        )
        .unwrap()
    }

    fn test_vendor(booth_id: domain::BoothId, vendor_id: &str) -> Vendor {
        Vendor::new(VendorId::new(vendor_id.to_string()), booth_id)
    }

    #[wasm_bindgen_test]
    async fn atomic_replace_rolls_back_when_transaction_aborts() {
        let db = Arc::new(create_test_db().await);
        let booth_repo: Arc<dyn BoothRepository> =
            Arc::new(IndexedDbBoothRepository::new(db.clone()));
        let vendor_repo: Arc<dyn VendorRepository> =
            Arc::new(IndexedDbVendorRepository::new(db.clone()));
        let purchase_repo: Arc<dyn PurchaseRepository> =
            Arc::new(IndexedDbPurchaseRepository::new(db.clone()));

        let original_booth = test_booth("Original Booth");
        let original_vendor = test_vendor(original_booth.id, "1");
        let original_purchase = Purchase::new(
            original_booth.id,
            vec![PurchaseItem::new(dec!(25.00), original_vendor.vendor_id.clone()).unwrap()],
        )
        .unwrap();

        booth_repo.save(&original_booth).await.unwrap();
        vendor_repo.save(&original_vendor).await.unwrap();
        purchase_repo.save(&original_purchase).await.unwrap();

        let original_booths = booth_repo.find_all().await.unwrap();
        let original_vendors = vendor_repo.find_all().await.unwrap();
        let original_purchases = purchase_repo.find_all().await.unwrap();
        let replacement_booth = test_booth("Replacement Booth");
        let replacement_vendor = test_vendor(replacement_booth.id, "9");

        let result = replace_all_atomically_with_options(
            db.as_ref(),
            &[replacement_booth],
            &[replacement_vendor],
            &[],
            true,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(booth_repo.find_all().await.unwrap(), original_booths);
        assert_eq!(vendor_repo.find_all().await.unwrap(), original_vendors);
        assert_eq!(purchase_repo.find_all().await.unwrap(), original_purchases);
    }
}

fn filter_invalid_records(summary: MigrationParseSummary) -> MigrationParseSummary {
    let mut invalid_booth_ids = HashSet::new();
    let mut invalid_purchase_ids = HashSet::new();
    let mut invalid_vendor_keys = HashSet::new();
    let mut invalid_item_vendor_keys = HashSet::new();

    for issue in &summary.validation.issues {
        match issue {
            ValidationIssue::VendorMissingBooth {
                booth_id,
                vendor_id,
            } => {
                invalid_booth_ids.insert(booth_id.clone());
                invalid_vendor_keys.insert((booth_id.clone(), vendor_id.clone()));
            }
            ValidationIssue::PurchaseMissingBooth {
                booth_id,
                purchase_id,
            } => {
                invalid_booth_ids.insert(booth_id.clone());
                invalid_purchase_ids.insert(purchase_id.clone());
            }
            ValidationIssue::PurchaseWithoutItems { purchase_id }
            | ValidationIssue::PurchaseTotalMismatch { purchase_id, .. } => {
                invalid_purchase_ids.insert(purchase_id.clone());
            }
            ValidationIssue::PurchaseItemMissingVendor {
                booth_id,
                purchase_id,
                vendor_id,
                ..
            } => {
                invalid_item_vendor_keys.insert((
                    booth_id.clone(),
                    purchase_id.clone(),
                    vendor_id.clone(),
                ));
            }
        }
    }

    let booths = summary
        .booths
        .into_iter()
        .filter(|booth| !invalid_booth_ids.contains(&booth.id.to_string()))
        .collect::<Vec<_>>();

    let vendors = summary
        .vendors
        .into_iter()
        .filter(|vendor| {
            !invalid_booth_ids.contains(&vendor.booth_id.to_string())
                && !invalid_vendor_keys
                    .contains(&(vendor.booth_id.to_string(), vendor.vendor_id.to_string()))
        })
        .collect::<Vec<_>>();

    let purchases = summary
        .purchases
        .into_iter()
        .filter(|purchase| {
            !invalid_booth_ids.contains(&purchase.booth_id.to_string())
                && !invalid_purchase_ids.contains(&purchase.id.to_string())
        })
        .filter_map(|mut purchase| {
            purchase.items.retain(|item| {
                !invalid_item_vendor_keys.contains(&(
                    purchase.booth_id.to_string(),
                    purchase.id.to_string(),
                    item.vendor_id.to_string(),
                ))
            });

            if purchase.items.is_empty() {
                None
            } else {
                Some(purchase)
            }
        })
        .collect::<Vec<_>>();

    let validation = validate_dataset(&booths, &vendors, &purchases, &[]);

    MigrationParseSummary {
        booths,
        vendors,
        purchases,
        validation,
    }
}

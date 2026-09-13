use async_trait::async_trait;
use domain::{
    BoothId, BoothRunningTotals, DomainResult, PaginatedPurchases, Purchase, PurchaseId,
    PurchaseRepository, VendorId,
};
use js_sys;
use log::{error, info, warn};
use rexie::TransactionMode;
use rust_decimal::Decimal;
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::Arc;
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::indexeddb::Database;

pub struct IndexedDbPurchaseRepository {
    db: Arc<Database>,
}

impl IndexedDbPurchaseRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn find_by_booth_with_diagnostics(
        &self,
        booth_id: &BoothId,
    ) -> Result<(Vec<Purchase>, Vec<String>), StorageError> {
        let transaction = self
            .db
            .transaction(&["purchases"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("purchases")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let index = store
            .index("booth_id")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let key = JsValue::from_str(&booth_id.as_str());
        let values = index
            .get_all(
                Some(rexie::KeyRange::only(&key).map_err(StorageError::from)?),
                None,
            )
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let mut purchases = Vec::new();
        let mut errors = Vec::new();

        for (index, value) in values.into_iter().enumerate() {
            match from_value::<Purchase>(value) {
                Ok(purchase) => purchases.push(purchase),
                Err(e) => {
                    let detail = format!("Record index {}: {}", index, e);
                    error!("Failed to deserialize purchase: {}", detail);
                    errors.push(detail);
                }
            }
        }

        purchases.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok((purchases, errors))
    }
}

#[async_trait(?Send)]
impl PurchaseRepository for IndexedDbPurchaseRepository {
    async fn save(&self, purchase: &Purchase) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["purchases"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("purchases")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let value =
            to_value(purchase).map_err(|e| StorageError::SerializationError(e.to_string()))?;

        store
            .put(&value, None)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &PurchaseId) -> DomainResult<Option<Purchase>> {
        let purchases = self.find_all().await?;
        Ok(purchases.into_iter().find(|purchase| &purchase.id == id))
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Purchase>> {
        info!(
            "IndexedDbPurchaseRepository::find_by_booth called for booth_id: {:?}",
            booth_id
        );

        let transaction = self
            .db
            .transaction(&["purchases"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("purchases")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let index = store
            .index("booth_id")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let key = JsValue::from_str(&booth_id.as_str());

        let values = index
            .get_all(
                Some(rexie::KeyRange::only(&key).map_err(StorageError::from)?),
                None,
            )
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let mut purchases = Vec::new();
        let mut deserialization_errors = Vec::new();

        for (index, value) in values.into_iter().enumerate() {
            match from_value::<Purchase>(value) {
                Ok(purchase) => purchases.push(purchase),
                Err(e) => {
                    let detail = format!("Record index {}: {}", index, e);
                    error!("Failed to deserialize purchase: {}", detail);
                    deserialization_errors.push(detail);
                }
            }
        }

        if !deserialization_errors.is_empty() {
            warn!(
                "Found {} corrupted purchase records in booth {}",
                deserialization_errors.len(),
                booth_id
            );
        }

        info!(
            "IndexedDbPurchaseRepository::find_by_booth returning {} purchases",
            purchases.len()
        );
        Ok(purchases)
    }

    async fn find_by_booth_paginated(
        &self,
        booth_id: &BoothId,
        offset: usize,
        limit: usize,
    ) -> DomainResult<PaginatedPurchases> {
        info!("IndexedDbPurchaseRepository::find_by_booth_paginated called for booth_id: {:?}, offset: {}, limit: {}", booth_id, offset, limit);

        // Get all purchases for the booth
        let mut all_purchases = self.find_by_booth(booth_id).await?;

        // Sort by timestamp descending (newest first)
        all_purchases.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let total_count = all_purchases.len();

        // Apply pagination
        let items: Vec<Purchase> = all_purchases.into_iter().skip(offset).take(limit).collect();

        info!("IndexedDbPurchaseRepository::find_by_booth_paginated returning {} items (total_count: {})", items.len(), total_count);
        Ok(PaginatedPurchases { items, total_count })
    }

    async fn get_running_totals(&self, booth_id: &BoothId) -> DomainResult<BoothRunningTotals> {
        let purchases = self.find_by_booth(booth_id).await?;

        let total_sales: Decimal = purchases.iter().map(|p| p.total_amount()).sum();
        let total_items: usize = purchases.iter().map(|p| p.items.len()).sum();
        let total_checkouts = purchases.len();

        Ok(BoothRunningTotals {
            total_sales,
            total_items,
            total_checkouts,
        })
    }

    async fn find_by_vendor(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Vec<Purchase>> {
        // Get all purchases for the booth, then filter by vendor
        // Note: vendor_id is now on PurchaseItem, not Purchase
        let all_purchases = self.find_by_booth(booth_id).await?;

        let filtered: Vec<Purchase> = all_purchases
            .into_iter()
            .filter(|p| p.items.iter().any(|item| &item.vendor_id == vendor_id))
            .collect();

        Ok(filtered)
    }

    async fn find_all(&self) -> DomainResult<Vec<Purchase>> {
        let transaction = self
            .db
            .transaction(&["purchases"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("purchases")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let values = store
            .get_all(None, None)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let mut purchases = Vec::new();
        let mut deserialization_errors = Vec::new();

        for (index, value) in values.into_iter().enumerate() {
            match from_value::<Purchase>(value) {
                Ok(purchase) => purchases.push(purchase),
                Err(e) => {
                    let detail = format!("Record index {}: {}", index, e);
                    error!("Failed to deserialize purchase: {}", detail);
                    deserialization_errors.push(detail);
                }
            }
        }

        if !deserialization_errors.is_empty() {
            warn!(
                "Found {} corrupted purchase records while loading all purchases",
                deserialization_errors.len()
            );
        }

        Ok(purchases)
    }

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        let purchases = self.find_by_booth(booth_id).await?;

        let transaction = self
            .db
            .transaction(&["purchases"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("purchases")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        for purchase in &purchases {
            let key_array = js_sys::Array::new();
            key_array.push(&JsValue::from_str(&booth_id.as_str()));
            key_array.push(&JsValue::from_str(&purchase.id.as_str()));

            store
                .delete(key_array.into())
                .await
                .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
        }

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(purchases.len())
    }

    async fn delete(&self, id: &PurchaseId) -> DomainResult<()> {
        info!(
            "IndexedDbPurchaseRepository::delete called for id: {:?}",
            id
        );

        // First, fetch the purchase to get its booth_id (needed for compound key)
        let purchase = self.find_by_id(id).await?.ok_or_else(|| {
            StorageError::NotFound(format!("Purchase not found: {}", id.as_str()))
        })?;

        info!(
            "Found purchase with booth_id: {:?}, will delete with compound key",
            purchase.booth_id
        );

        self.delete_from_booth(&purchase.booth_id, id).await
    }

    async fn delete_from_booth(&self, booth_id: &BoothId, id: &PurchaseId) -> DomainResult<()> {
        info!(
            "IndexedDbPurchaseRepository::delete_from_booth called for booth_id: {:?}, id: {:?}",
            booth_id, id
        );

        let transaction = self
            .db
            .transaction(&["purchases"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("purchases")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        // Purchases use compound key: [booth_id, id]
        let key_array = js_sys::Array::new();
        key_array.push(&JsValue::from_str(&booth_id.as_str()));
        key_array.push(&JsValue::from_str(&id.as_str()));
        let key = JsValue::from(key_array);

        info!(
            "Deleting purchase with compound key: [booth_id: {}, id: {}]",
            booth_id.as_str(),
            id.as_str()
        );

        store
            .delete(key)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        info!("Delete operation completed, waiting for transaction.done()");
        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        info!(
            "Transaction committed successfully for purchase_id: {:?}",
            id
        );
        Ok(())
    }
}

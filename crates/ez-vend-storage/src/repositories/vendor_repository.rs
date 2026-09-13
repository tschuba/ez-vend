use async_trait::async_trait;
use domain::{BoothId, DomainResult, Vendor, VendorId, VendorRepository};
use rexie::TransactionMode;
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::Arc;
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::indexeddb::Database;

pub struct IndexedDbVendorRepository {
    db: Arc<Database>,
}

impl IndexedDbVendorRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait(?Send)]
impl VendorRepository for IndexedDbVendorRepository {
    async fn save(&self, vendor: &Vendor) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["vendors"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("vendors")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let value =
            to_value(vendor).map_err(|e| StorageError::SerializationError(e.to_string()))?;

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

    async fn find_by_id(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Option<Vendor>> {
        let transaction = self
            .db
            .transaction(&["vendors"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("vendors")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        // Create composite key array: [booth_id, vendor_id]
        let key_array = js_sys::Array::new();
        key_array.push(&JsValue::from_str(&booth_id.as_str()));
        key_array.push(&JsValue::from_str(vendor_id.as_str()));

        let result = store
            .get(key_array.into())
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        match result {
            Some(value) => {
                let vendor: Vendor = from_value(value)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(vendor))
            }
            None => Ok(None),
        }
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Vendor>> {
        let transaction = self
            .db
            .transaction(&["vendors"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("vendors")
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

        let vendors: Vec<Vendor> = values
            .into_iter()
            .filter_map(|value| from_value(value).ok())
            .collect();

        Ok(vendors)
    }

    async fn find_all(&self) -> DomainResult<Vec<Vendor>> {
        let transaction = self
            .db
            .transaction(&["vendors"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("vendors")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let values = store
            .get_all(None, None)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let vendors: Vec<Vendor> = values
            .into_iter()
            .filter_map(|value| from_value(value).ok())
            .collect();

        Ok(vendors)
    }

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        let vendors = self.find_by_booth(booth_id).await?;

        let transaction = self
            .db
            .transaction(&["vendors"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("vendors")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        for vendor in &vendors {
            let key_array = js_sys::Array::new();
            key_array.push(&JsValue::from_str(&booth_id.as_str()));
            key_array.push(&JsValue::from_str(vendor.vendor_id.as_str()));

            store
                .delete(key_array.into())
                .await
                .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
        }

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(vendors.len())
    }

    async fn delete(&self, booth_id: &BoothId, vendor_id: &VendorId) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["vendors"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("vendors")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        // Create composite key array: [booth_id, vendor_id]
        let key_array = js_sys::Array::new();
        key_array.push(&JsValue::from_str(&booth_id.as_str()));
        key_array.push(&JsValue::from_str(vendor_id.as_str()));

        store
            .delete(key_array.into())
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(())
    }

    async fn delete_from_booth(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<()> {
        log::info!("IndexedDbVendorRepository::delete_from_booth called for booth_id: {:?}, vendor_id: {:?}", booth_id, vendor_id);

        let transaction = self
            .db
            .transaction(&["vendors"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("vendors")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        // Vendors use compound key: [booth_id, vendor_id]
        let key_array = js_sys::Array::new();
        key_array.push(&JsValue::from_str(&booth_id.as_str()));
        key_array.push(&JsValue::from_str(vendor_id.as_str()));
        let key = JsValue::from(key_array);

        log::info!(
            "Deleting vendor with compound key: [booth_id: {}, vendor_id: {}]",
            booth_id.as_str(),
            vendor_id.as_str()
        );

        store
            .delete(key)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        log::info!("Delete operation completed, waiting for transaction.done()");
        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        log::info!(
            "Transaction committed successfully for vendor_id: {:?}",
            vendor_id
        );
        Ok(())
    }
}

use async_trait::async_trait;
use domain::{BoothId, DomainResult, ProductGroup, ProductGroupId, ProductGroupRepository};
use rexie::TransactionMode;
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::Arc;
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::indexeddb::Database;

pub struct IndexedDbProductGroupRepository {
    db: Arc<Database>,
}

impl IndexedDbProductGroupRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait(?Send)]
impl ProductGroupRepository for IndexedDbProductGroupRepository {
    async fn save(&self, group: &ProductGroup) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["product_groups"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("product_groups")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let value = to_value(group).map_err(|e| StorageError::SerializationError(e.to_string()))?;

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
        id: &ProductGroupId,
    ) -> DomainResult<Option<ProductGroup>> {
        let transaction = self
            .db
            .transaction(&["product_groups"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("product_groups")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let key_array = js_sys::Array::new();
        key_array.push(&JsValue::from_str(&booth_id.as_str()));
        key_array.push(&JsValue::from_str(&id.as_str()));

        let result = store
            .get(key_array.into())
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        match result {
            Some(value) => {
                let group: ProductGroup = from_value(value)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(group))
            }
            None => Ok(None),
        }
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<ProductGroup>> {
        let transaction = self
            .db
            .transaction(&["product_groups"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("product_groups")
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

        let mut groups: Vec<ProductGroup> = values
            .into_iter()
            .filter_map(|value| from_value(value).ok())
            .collect();

        groups.sort_by_key(|g| g.sort_order);

        Ok(groups)
    }

    async fn delete(&self, booth_id: &BoothId, id: &ProductGroupId) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["product_groups"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("product_groups")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let key_array = js_sys::Array::new();
        key_array.push(&JsValue::from_str(&booth_id.as_str()));
        key_array.push(&JsValue::from_str(&id.as_str()));

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

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        let groups = self.find_by_booth(booth_id).await?;

        let transaction = self
            .db
            .transaction(&["product_groups"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("product_groups")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        for group in &groups {
            let key_array = js_sys::Array::new();
            key_array.push(&JsValue::from_str(&booth_id.as_str()));
            key_array.push(&JsValue::from_str(&group.id.as_str()));

            store
                .delete(key_array.into())
                .await
                .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
        }

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(groups.len())
    }
}

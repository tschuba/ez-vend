use async_trait::async_trait;
use domain::{BoothId, DomainResult, Product, ProductGroupId, ProductId, ProductRepository};
use rexie::TransactionMode;
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::Arc;
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::indexeddb::Database;

pub struct IndexedDbProductRepository {
    db: Arc<Database>,
}

impl IndexedDbProductRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait(?Send)]
impl ProductRepository for IndexedDbProductRepository {
    async fn save(&self, product: &Product) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["products"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("products")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let value =
            to_value(product).map_err(|e| StorageError::SerializationError(e.to_string()))?;

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
        id: &ProductId,
    ) -> DomainResult<Option<Product>> {
        let transaction = self
            .db
            .transaction(&["products"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("products")
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
                let product: Product = from_value(value)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(product))
            }
            None => Ok(None),
        }
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Product>> {
        let transaction = self
            .db
            .transaction(&["products"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("products")
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

        let mut products: Vec<Product> = values
            .into_iter()
            .filter_map(|value| from_value(value).ok())
            .collect();

        products.sort_by_key(|p| p.sort_order);

        Ok(products)
    }

    async fn find_by_group(
        &self,
        booth_id: &BoothId,
        group_id: &ProductGroupId,
    ) -> DomainResult<Vec<Product>> {
        let all = self.find_by_booth(booth_id).await?;
        Ok(all
            .into_iter()
            .filter(|p| &p.product_group_id == group_id)
            .collect())
    }

    async fn delete(&self, booth_id: &BoothId, id: &ProductId) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["products"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("products")
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
        let products = self.find_by_booth(booth_id).await?;

        let transaction = self
            .db
            .transaction(&["products"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("products")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        for product in &products {
            let key_array = js_sys::Array::new();
            key_array.push(&JsValue::from_str(&booth_id.as_str()));
            key_array.push(&JsValue::from_str(&product.id.as_str()));

            store
                .delete(key_array.into())
                .await
                .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
        }

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(products.len())
    }
}

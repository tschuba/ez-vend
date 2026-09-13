use async_trait::async_trait;
use chrono::NaiveDate;
use domain::{Booth, BoothId, BoothRepository, DomainResult};
use js_sys;
use log::{debug, error, info, warn};
use rexie::TransactionMode;
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::Arc;
use wasm_bindgen::JsValue;

use crate::error::StorageError;
use crate::indexeddb::Database;

fn description_date_key(description: &str, date: &NaiveDate) -> JsValue {
    let key_array = js_sys::Array::new();
    key_array.push(&JsValue::from_str(description.trim()));
    key_array.push(&JsValue::from_str(&date.format("%Y-%m-%d").to_string()));
    key_array.into()
}

#[derive(Clone)]
pub struct IndexedDbBoothRepository {
    db: Arc<Database>,
}

impl IndexedDbBoothRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait(?Send)]
impl BoothRepository for IndexedDbBoothRepository {
    async fn save(&self, booth: &Booth) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("booths")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let value = to_value(booth).map_err(|e| StorageError::SerializationError(e.to_string()))?;

        if booth.is_archived() {
            if let Some(summary) = &booth.archived_summary {
                info!(
                    "Saving archived booth {} with revenue {}, booth revenue {}, vendor summaries {}",
                    booth.id,
                    summary.total_revenue,
                    summary.total_booth_revenue,
                    summary.vendor_summaries.len()
                );
            } else {
                warn!(
                    "Saving archived booth {} without archived summary",
                    booth.id
                );
            }
        }

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

    async fn find_by_id(&self, id: &BoothId) -> DomainResult<Option<Booth>> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("booths")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let key = JsValue::from_str(&id.as_str());

        let result = store
            .get(key)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        match result {
            Some(value) => {
                let booth: Booth = from_value(value)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

                if booth.is_archived() {
                    match booth.archived_summary.as_ref() {
                        Some(summary) => info!(
                            "Loaded archived booth {} with revenue {}, booth revenue {}, vendor summaries {}",
                            booth.id,
                            summary.total_revenue,
                            summary.total_booth_revenue,
                            summary.vendor_summaries.len()
                        ),
                        None => warn!(
                            "Loaded archived booth {} without archived summary",
                            booth.id
                        ),
                    }
                }

                Ok(Some(booth))
            }
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> DomainResult<Vec<Booth>> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("booths")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let values = store
            .get_all(None, None)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let booths: Vec<Booth> = values
            .into_iter()
            .filter_map(|value| match from_value::<Booth>(value) {
                Ok(booth) => {
                    if booth.is_archived() {
                        match booth.archived_summary.as_ref() {
                            Some(summary) => debug!(
                                "Loaded archived booth {} from list with revenue {}, booth revenue {}, vendor summaries {}",
                                booth.id,
                                summary.total_revenue,
                                summary.total_booth_revenue,
                                summary.vendor_summaries.len()
                            ),
                            None => warn!(
                                "Loaded archived booth {} from list without archived summary",
                                booth.id
                            ),
                        }
                    }

                    Some(booth)
                }
                Err(err) => {
                    error!("Failed to deserialize booth from IndexedDB: {}", err);
                    None
                }
            })
            .collect();

        Ok(booths)
    }

    async fn find_active(&self) -> DomainResult<Vec<Booth>> {
        Ok(self
            .find_all()
            .await?
            .into_iter()
            .filter(|booth| !booth.is_archived())
            .collect())
    }

    async fn find_archived(&self) -> DomainResult<Vec<Booth>> {
        Ok(self
            .find_all()
            .await?
            .into_iter()
            .filter(|booth| booth.is_archived())
            .collect())
    }

    async fn find_by_description_and_date(
        &self,
        description: &str,
        date: &NaiveDate,
    ) -> DomainResult<Option<Booth>> {
        Ok(self
            .find_all_by_description_and_date(description, date)
            .await?
            .into_iter()
            .next())
    }

    async fn find_all_by_description_and_date(
        &self,
        description: &str,
        date: &NaiveDate,
    ) -> DomainResult<Vec<Booth>> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadOnly)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("booths")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let index = store
            .index("description_date")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let key = description_date_key(description, date);
        let results = index
            .get_all(
                Some(rexie::KeyRange::only(&key).map_err(StorageError::from)?),
                None,
            )
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let booths: Vec<Booth> = results
            .into_iter()
            .filter_map(|value| match from_value::<Booth>(value) {
                Ok(booth) => Some(booth),
                Err(err) => {
                    error!(
                        "Failed to deserialize booth from description_date index: {}",
                        err
                    );
                    None
                }
            })
            .collect();

        Ok(booths)
    }

    async fn find_duplicate_groups(&self) -> DomainResult<Vec<Vec<Booth>>> {
        let all_active = self.find_active().await?;

        let mut groups: std::collections::HashMap<String, Vec<Booth>> =
            std::collections::HashMap::new();

        for booth in all_active {
            let key = format!(
                "{}|{}",
                booth.description.trim(),
                booth.date.format("%Y-%m-%d")
            );
            groups.entry(key).or_default().push(booth);
        }

        Ok(groups
            .into_values()
            .filter(|group| group.len() >= 2)
            .collect())
    }

    async fn delete(&self, id: &BoothId) -> DomainResult<()> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadWrite)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        let store = transaction
            .store("booths")
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        let key = JsValue::from_str(&id.as_str());

        store
            .delete(key)
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        transaction
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))?;

        Ok(())
    }
}

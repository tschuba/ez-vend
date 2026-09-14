use std::rc::Rc;

use chrono::Utc;
use js_sys::{Object, Reflect};
use rexie::{Index, ObjectStore, Rexie, TransactionMode};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

use crate::error::StorageError;

const DB_NAME: &str = "ez_vend_v1";
pub const DB_VERSION: u32 = 6;

const LAST_MODIFIED_AT_KEY: &str = "last_modified_at";

struct RawDatabase {
    db: Rexie,
}

impl RawDatabase {
    async fn new_with_name(db_name: &str) -> Result<Self, StorageError> {
        let rexie = Rexie::builder(db_name)
            .version(DB_VERSION)
            .add_object_store(
                ObjectStore::new("booths")
                    .key_path("id")
                    .add_index(Index::new("date", "date"))
                    .add_index(Index::new_array(
                        "description_date",
                        ["description", "date"],
                    )),
            )
            .add_object_store(
                ObjectStore::new("vendors")
                    .key_path_array(["booth_id", "id"])
                    .add_index(Index::new("booth_id", "booth_id")),
            )
            .add_object_store(
                ObjectStore::new("purchases")
                    .key_path_array(["booth_id", "id"])
                    .add_index(Index::new("booth_id", "booth_id"))
                    .add_index(Index::new("purchased_at", "purchased_at")),
            )
            .add_object_store(
                ObjectStore::new("error_logs")
                    .auto_increment(true)
                    .add_index(Index::new("timestamp", "timestamp")),
            )
            .add_object_store(ObjectStore::new("export_records").key_path("booth_id"))
            .add_object_store(
                ObjectStore::new("archive_audit_log")
                    .key_path("id")
                    .add_index(Index::new("timestamp", "timestamp"))
                    .add_index(Index::new("booth_id", "booth_id")),
            )
            .add_object_store(ObjectStore::new("metadata").key_path("key"))
            .add_object_store(
                ObjectStore::new("product_groups")
                    .key_path_array(["booth_id", "id"])
                    .add_index(Index::new("booth_id", "booth_id")),
            )
            .add_object_store(
                ObjectStore::new("products")
                    .key_path_array(["booth_id", "id"])
                    .add_index(Index::new("booth_id", "booth_id")),
            )
            .build()
            .await
            .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;

        Ok(Self { db: rexie })
    }

    fn db(&self) -> &Rexie {
        &self.db
    }

    fn transaction(
        &self,
        store_names: &[&str],
        mode: TransactionMode,
    ) -> Result<rexie::Transaction, StorageError> {
        self.db
            .transaction(store_names, mode)
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))
    }
}

/// Wraps a `rexie::Transaction` and automatically writes `last_modified_at`
/// to the metadata store on `.done()` for any write transaction that touches
/// non-metadata stores.
///
/// Implements `Deref<Target = rexie::Transaction>` so all existing callers
/// that pass `&TrackedTransaction` to functions expecting `&rexie::Transaction`
/// continue to work via deref coercion — no call-site changes required.
pub struct TrackedTransaction {
    inner: rexie::Transaction,
    is_write: bool,
    on_write: Option<Rc<dyn Fn()>>,
}

impl std::ops::Deref for TrackedTransaction {
    type Target = rexie::Transaction;

    fn deref(&self) -> &rexie::Transaction {
        &self.inner
    }
}

impl TrackedTransaction {
    /// Completes the transaction. For write transactions, `last_modified_at`
    /// is written to the metadata store atomically before committing.
    pub async fn done(self) -> Result<rexie::TransactionResult, StorageError> {
        if self.is_write {
            let now = Utc::now();
            let serialized =
                to_value(&now).map_err(|e| StorageError::SerializationError(e.to_string()))?;

            let entry = Object::new();
            Reflect::set(
                &entry,
                &JsValue::from_str("key"),
                &JsValue::from_str(LAST_MODIFIED_AT_KEY),
            )
            .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;
            Reflect::set(&entry, &JsValue::from_str("value"), &serialized)
                .map_err(|e| StorageError::JsError(format!("{:?}", e)))?;

            let store = self
                .inner
                .store("metadata")
                .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
            store
                .put(&JsValue::from(entry), None)
                .await
                .map_err(|e| StorageError::DatabaseError(format!("{:?}", e)))?;
        }

        let result = self
            .inner
            .done()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)));

        if result.is_ok() {
            if let Some(cb) = &self.on_write {
                cb();
            }
        }

        result
    }

    /// Aborts the transaction without writing `last_modified_at`.
    pub async fn abort(self) -> Result<(), StorageError> {
        self.inner
            .abort()
            .await
            .map_err(|e| StorageError::TransactionError(format!("{:?}", e)))
    }
}

/// Public database handle. All write transactions automatically record
/// `last_modified_at` in the metadata store via [`TrackedTransaction`].
///
/// Pure metadata-only write transactions (store list == `["metadata"]`) are
/// excluded from tracking so that recording a backup does not itself appear
/// as an unsaved change.
pub struct Database {
    inner: RawDatabase,
    on_write: Option<Rc<dyn Fn()>>,
    #[cfg(any(test, debug_assertions))]
    fail_writes: std::cell::Cell<bool>,
}

impl Database {
    pub async fn new() -> Result<Self, StorageError> {
        Self::new_with_callback(DB_NAME, None).await
    }

    pub async fn new_with_name(db_name: &str) -> Result<Self, StorageError> {
        Self::new_with_callback(db_name, None).await
    }

    pub async fn new_with_callback(
        db_name: &str,
        on_write: Option<Rc<dyn Fn()>>,
    ) -> Result<Self, StorageError> {
        Ok(Self {
            inner: RawDatabase::new_with_name(db_name).await?,
            on_write,
            #[cfg(any(test, debug_assertions))]
            fail_writes: std::cell::Cell::new(false),
        })
    }

    #[cfg(any(test, debug_assertions))]
    pub fn set_fail_writes(&self, fail: bool) {
        self.fail_writes.set(fail);
    }

    pub fn db(&self) -> &Rexie {
        self.inner.db()
    }

    /// Opens a transaction on the given stores. For `ReadWrite` transactions
    /// that touch at least one non-metadata store, `"metadata"` is added to
    /// the store list automatically so that `TrackedTransaction::done()` can
    /// write the `last_modified_at` timestamp atomically.
    pub fn transaction(
        &self,
        store_names: &[&str],
        mode: TransactionMode,
    ) -> Result<TrackedTransaction, StorageError> {
        #[cfg(any(test, debug_assertions))]
        if self.fail_writes.get() {
            return Err(StorageError::DatabaseError(
                "test: forced write failure".to_string(),
            ));
        }

        let is_write = mode == TransactionMode::ReadWrite;
        let metadata_only = store_names == ["metadata"];

        let inner = if is_write && !metadata_only && !store_names.contains(&"metadata") {
            let mut stores: Vec<&str> = store_names.to_vec();
            stores.push("metadata");
            self.inner.transaction(&stores, mode)?
        } else {
            self.inner.transaction(store_names, mode)?
        };

        Ok(TrackedTransaction {
            inner,
            is_write: is_write && !metadata_only,
            on_write: if is_write && !metadata_only {
                self.on_write.clone()
            } else {
                None
            },
        })
    }
}

use thiserror::Error;
use wasm_bindgen::JsValue;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("JS error: {0}")]
    JsError(String),

    #[error("Corrupted data detected: {count} records could not be loaded")]
    CorruptedData { count: usize, details: Vec<String> },
}

impl From<JsValue> for StorageError {
    fn from(err: JsValue) -> Self {
        StorageError::JsError(format!("{:?}", err))
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

impl From<rexie::Error> for StorageError {
    fn from(err: rexie::Error) -> Self {
        StorageError::DatabaseError(format!("{:?}", err))
    }
}

impl From<idb::Error> for StorageError {
    fn from(err: idb::Error) -> Self {
        StorageError::DatabaseError(format!("{:?}", err))
    }
}

/// Convert StorageError to DomainError
impl From<StorageError> for domain::DomainError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::NotFound(msg) => domain::DomainError::NotFound(msg),
            other => domain::DomainError::Storage(other.to_string()),
        }
    }
}

impl From<domain::DomainError> for StorageError {
    fn from(err: domain::DomainError) -> Self {
        match err {
            domain::DomainError::NotFound(msg) => StorageError::NotFound(msg),
            other => StorageError::TransactionError(other.to_string()),
        }
    }
}

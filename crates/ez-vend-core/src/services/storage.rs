use crate::models::Booth;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Booth not found: {0}")]
    NotFound(Uuid),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

/// Storage abstraction for booths
pub trait BoothStorage: Send + Sync {
    fn save_booth(&self, booth: &Booth) -> StorageResult<()>;
    fn load_booth(&self, id: &Uuid) -> StorageResult<Booth>;
    fn load_all_booths(&self) -> StorageResult<Vec<Booth>>;
    fn delete_booth(&self, id: &Uuid) -> StorageResult<()>;
    fn export_data(&self) -> StorageResult<String>;
    fn import_data(&self, data: &str) -> StorageResult<Vec<Booth>>;
}

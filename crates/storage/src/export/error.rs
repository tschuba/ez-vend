use domain::{BoothId, DomainError};
use thiserror::Error;

use crate::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFailure {
    pub record_type: String,
    pub record_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRecord {
    pub record_type: String,
    pub record_id: String,
    pub reason: String,
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Booth not found: {0}")]
    BoothNotFound(BoothId),

    #[error("Failed to serialize backup: {0}")]
    Serialization(String),
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("Invalid backup JSON: {0}")]
    InvalidJson(String),

    #[error("Checksum verification failed: expected {expected}, actual {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Unsupported backup version {found}; expected {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },

    #[error("Invalid backup structure: {0}")]
    InvalidStructure(String),

    #[error("Orphaned records detected")]
    OrphanedRecords { details: Vec<String> },

    #[error("Backup validation failed")]
    ValidationFailed { failures: Vec<ValidationFailure> },

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}

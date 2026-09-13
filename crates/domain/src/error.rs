use thiserror::Error;

use crate::error_code::ValidationError;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Validation error: {0}")]
    Validation(ValidationError),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Storage error: {0}")]
    Storage(String),
}

pub type DomainResult<T> = Result<T, DomainError>;

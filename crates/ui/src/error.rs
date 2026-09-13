use domain::DomainError;
use thiserror::Error;

/// UI-specific errors
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum UiError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Navigation error: {0}")]
    Navigation(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("UI state error: {0}")]
    StateError(String),
}

#[allow(dead_code)]
pub type UiResult<T> = Result<T, UiError>;

/// Convert UiError to user-friendly message key
#[allow(dead_code)]
pub fn error_to_message_key(error: &UiError) -> &'static str {
    match error {
        UiError::Domain(DomainError::NotFound(_)) => "error.not_found",
        UiError::Domain(DomainError::Validation(_)) => "error.validation",
        UiError::Domain(DomainError::Storage(_)) => "error.storage",
        UiError::Domain(DomainError::InvalidState(_)) => "error.generic",
        UiError::InvalidInput(_) => "error.validation",
        UiError::Navigation(_) => "error.generic",
        UiError::StateError(_) => "error.generic",
    }
}

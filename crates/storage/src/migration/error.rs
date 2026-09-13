use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum MigrationError {
    #[error("Invalid SQLite database file: {0}")]
    InvalidSqliteFile(String),

    #[error("SQLite query failed: {0}")]
    Database(String),

    #[error("Domain validation failed: {0}")]
    Domain(String),

    #[error("Invalid UUID for {field}: {value}")]
    InvalidUuid { field: &'static str, value: String },

    #[error("Invalid timestamp for {field}: {value}")]
    InvalidTimestamp { field: &'static str, value: i64 },

    #[error("Invalid booth date for {field}: {value}")]
    InvalidDate { field: &'static str, value: i64 },

    #[error("Migration failed while replacing data: {0}")]
    ReplaceFailed(String),
}

impl From<rusqlite::Error> for MigrationError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value.to_string())
    }
}

impl From<domain::DomainError> for MigrationError {
    fn from(value: domain::DomainError) -> Self {
        Self::Domain(value.to_string())
    }
}

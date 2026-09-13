pub mod archive;
pub mod diagnostics;
pub mod error;
pub mod error_log;
pub mod export;
pub mod indexeddb;
pub mod merge_service;
pub mod migration;
pub mod repositories;

pub use archive::{
    archive_size_warning, save_export_record, ArchiveAuditEvent, ArchiveAuditEventType,
    ArchiveOutcome, ArchivePreview, ArchiveService, ArchiveSizeWarning, ExportRecord,
};
pub use diagnostics::{
    create_session_id, load_storage_diagnostics, record_backup_completed, run_integrity_check,
    IntegrityStatus, StorageDiagnostics,
};
pub use error::StorageError;
pub use error_log::{
    retention_cutoff, ErrorLogContext, ErrorLogDeviceInfo, ErrorLogEntry, ERROR_LOG_RETENTION_DAYS,
    ERROR_LOG_RETENTION_LIMIT,
};
pub use indexeddb::Database;
pub use merge_service::MergeService;
pub use migration::{
    LegacyBooth, LegacyPurchase, LegacyPurchaseItem, LegacyVendor, MigrationError,
    MigrationIssueStrategy, MigrationParseSummary, MigrationResult, MigrationService,
    MigrationValidationSummary, SqliteParser, ValidationIssue,
};
pub use repositories::{ErrorLogRepository, IndexedDbErrorLogRepository};

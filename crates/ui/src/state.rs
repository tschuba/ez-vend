use domain::repositories::{BoothRepository, ProductGroupRepository, ProductRepository, PurchaseRepository, VendorRepository};
use domain::services::{BoothService, ReportService, VendorService};
use ez_vend_storage::export::{ExportService, ImportService};
use ez_vend_storage::indexeddb::Database;
use ez_vend_storage::repositories::{
    IndexedDbBoothRepository, IndexedDbErrorLogRepository, IndexedDbProductGroupRepository,
    IndexedDbProductRepository, IndexedDbPurchaseRepository, IndexedDbVendorRepository,
};
use ez_vend_storage::{
    create_session_id, load_storage_diagnostics, run_integrity_check, ArchiveService,
    ErrorLogEntry, ErrorLogRepository, IntegrityStatus, MergeService, MigrationService,
    StorageDiagnostics,
};
use leptos::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

/// Application state containing repositories and services
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<Database>,
    pub session_id: String,
    pub error_log_repository: Arc<dyn ErrorLogRepository>,
    pub booth_repository: Arc<dyn BoothRepository>,
    pub booth_service: Arc<BoothService<IndexedDbBoothRepository>>,
    pub vendor_repository: Arc<dyn VendorRepository>,
    pub purchase_repository: Arc<dyn PurchaseRepository>,
    pub product_group_repository: Arc<dyn ProductGroupRepository>,
    pub product_repository: Arc<dyn ProductRepository>,
    pub indexed_purchase_repository: Arc<IndexedDbPurchaseRepository>,
    pub export_service: Arc<ExportService>,
    pub archive_service: Arc<ArchiveService>,
    pub import_service: Arc<ImportService>,
    pub merge_service: Arc<MergeService>,
    pub migration_service: Arc<MigrationService>,
    pub vendor_service: Arc<VendorService<IndexedDbVendorRepository, IndexedDbBoothRepository>>,
    pub report_service: Arc<
        ReportService<
            IndexedDbPurchaseRepository,
            IndexedDbBoothRepository,
            IndexedDbVendorRepository,
        >,
    >,
}

impl AppState {
    #[allow(clippy::arc_with_non_send_sync)]
    pub async fn new_with_callback(on_write: Option<Rc<dyn Fn()>>) -> Result<Self, String> {
        // Initialize IndexedDB
        let db = Database::new_with_callback("ez_vend_v1", on_write)
            .await
            .map_err(|e| format!("Failed to initialize database: {:?}", e))?;

        let db = Arc::new(db);
        let session_id = create_session_id(&db)
            .await
            .map_err(|e| format!("Failed to initialize error logging session: {:?}", e))?;

        // Create repositories
        let error_log_repository: Arc<dyn ErrorLogRepository> =
            Arc::new(IndexedDbErrorLogRepository::new(db.clone()));
        let booth_repository: Arc<dyn BoothRepository> =
            Arc::new(IndexedDbBoothRepository::new(db.clone()));
        let vendor_repository: Arc<dyn VendorRepository> =
            Arc::new(IndexedDbVendorRepository::new(db.clone()));
        let indexed_purchase_repository = Arc::new(IndexedDbPurchaseRepository::new(db.clone()));
        let purchase_repository: Arc<dyn PurchaseRepository> = indexed_purchase_repository.clone();
        let product_group_repository: Arc<dyn ProductGroupRepository> =
            Arc::new(IndexedDbProductGroupRepository::new(db.clone()));
        let product_repository: Arc<dyn ProductRepository> =
            Arc::new(IndexedDbProductRepository::new(db.clone()));
        let export_service = Arc::new(ExportService::with_database(
            booth_repository.clone(),
            vendor_repository.clone(),
            purchase_repository.clone(),
            db.clone(),
        ));
        let archive_service = Arc::new(ArchiveService::new(db.clone()));
        let merge_service = Arc::new(MergeService::with_database(
            booth_repository.clone(),
            vendor_repository.clone(),
            purchase_repository.clone(),
            db.clone(),
        ));
        let import_service = Arc::new(
            ImportService::with_database(
                booth_repository.clone(),
                vendor_repository.clone(),
                purchase_repository.clone(),
                Some(archive_service.clone()),
                db.clone(),
            )
            .with_merge_service(merge_service.clone()),
        );
        let migration_service = Arc::new(MigrationService::new_with_database(
            db.clone(),
            booth_repository.clone(),
            vendor_repository.clone(),
            purchase_repository.clone(),
        ));

        // Create services (use separate instances for service layer)
        let vendor_service = Arc::new(VendorService::new(
            IndexedDbVendorRepository::new(db.clone()),
            IndexedDbBoothRepository::new(db.clone()),
        ));
        let booth_service = Arc::new(BoothService::new(IndexedDbBoothRepository::new(db.clone())));
        let report_service = Arc::new(ReportService::new(
            IndexedDbPurchaseRepository::new(db.clone()),
            IndexedDbBoothRepository::new(db.clone()),
            IndexedDbVendorRepository::new(db.clone()),
        ));

        Ok(Self {
            database: db,
            session_id,
            error_log_repository,
            booth_repository,
            booth_service,
            vendor_repository,
            purchase_repository,
            product_group_repository,
            product_repository,
            indexed_purchase_repository,
            export_service,
            archive_service,
            import_service,
            merge_service,
            migration_service,
            vendor_service,
            report_service,
        })
    }

    pub async fn load_storage_diagnostics(&self) -> Result<StorageDiagnostics, String> {
        load_storage_diagnostics(&self.database)
            .await
            .map_err(|err| format!("Failed to load storage diagnostics: {err}"))
    }

    pub async fn run_integrity_check(&self) -> Result<IntegrityStatus, String> {
        run_integrity_check(&self.database)
            .await
            .map_err(|err| format!("Failed to run integrity check: {err}"))
    }

    pub async fn log_error(&self, entry: &ErrorLogEntry) -> Result<ErrorLogEntry, String> {
        self.error_log_repository
            .log_error(entry)
            .await
            .map_err(|err| format!("Failed to log error: {err}"))
    }

    pub async fn get_recent_errors(&self, limit: usize) -> Result<Vec<ErrorLogEntry>, String> {
        self.error_log_repository
            .get_recent_errors(limit)
            .await
            .map_err(|err| format!("Failed to load error log: {err}"))
    }

    pub async fn count_errors_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize, String> {
        self.error_log_repository
            .count_errors_since(since)
            .await
            .map_err(|err| format!("Failed to count errors: {err}"))
    }

    pub async fn clear_error_log(&self) -> Result<(), String> {
        self.error_log_repository
            .clear_all()
            .await
            .map_err(|err| format!("Failed to clear error log: {err}"))
    }
}

/// Provide app state to the component tree
pub fn provide_app_state(
    on_write: Option<Rc<dyn Fn()>>,
) -> LocalResource<Result<AppState, String>> {
    LocalResource::new(move || {
        let cb = on_write.clone();
        async move { AppState::new_with_callback(cb).await }
    })
}

/// Use app state from context
pub fn use_app_state() -> LocalResource<Result<AppState, String>> {
    if let Some(app_state) = use_context::<LocalResource<Result<AppState, String>>>() {
        app_state
    } else {
        web_sys::console::warn_1(
            &"AppState context not found. Returning fallback error resource.".into(),
        );
        LocalResource::new(|| async {
            Err(
                "AppState context not found. Make sure provide_app_state() is called in a parent component.".to_string(),
            )
        })
    }
}

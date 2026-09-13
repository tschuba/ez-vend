use std::collections::HashMap;
use std::sync::Arc;

use domain::{
    Booth, BoothId, BoothRepository, Purchase, PurchaseRepository, Vendor, VendorId,
    VendorRepository,
};
use js_sys;
use rexie::TransactionMode;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsValue;

use super::analysis::{
    booth_to_candidate, BoothCandidate, BoothImportAnalysis, BoothMatchKind, BoothResolution,
    ImportAnalysis, ImportPayload,
};
use super::backup_format::{BackupData, BoothBackupData, DeviceInfo};
use super::error::{ImportError, SkippedRecord};
use crate::archive::ArchiveService;
use crate::error::StorageError;
use crate::indexeddb::Database;
use crate::merge_service::MergeService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    Skip,
    Replace,
    Merge,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportSummary {
    pub booths_imported: usize,
    pub vendors_imported: usize,
    pub purchases_imported: usize,
    pub conflicts_resolved: usize,
    pub skipped_records: Vec<SkippedRecord>,
}

/// Canonical booth outcome after resolution — drives vendor/purchase remapping
struct BoothImportOutcome {
    canonical_id: BoothId,
    /// None = new booth; Some = matched existing
    match_kind: Option<BoothMatchKind>,
}

#[derive(Clone)]
pub struct ImportService {
    booth_repository: Arc<dyn BoothRepository>,
    vendor_repository: Arc<dyn VendorRepository>,
    purchase_repository: Arc<dyn PurchaseRepository>,
    archive_service: Option<Arc<ArchiveService>>,
    merge_service: Option<Arc<MergeService>>,
    db: Option<Arc<Database>>,
}

impl ImportService {
    pub fn new(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
    ) -> Self {
        Self::with_archive_service(
            booth_repository,
            vendor_repository,
            purchase_repository,
            None,
        )
    }

    pub fn with_archive_service(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
        archive_service: Option<Arc<ArchiveService>>,
    ) -> Self {
        Self {
            booth_repository,
            vendor_repository,
            purchase_repository,
            archive_service,
            merge_service: None,
            db: None,
        }
    }

    pub fn with_database(
        booth_repository: Arc<dyn BoothRepository>,
        vendor_repository: Arc<dyn VendorRepository>,
        purchase_repository: Arc<dyn PurchaseRepository>,
        archive_service: Option<Arc<ArchiveService>>,
        db: Arc<Database>,
    ) -> Self {
        Self {
            booth_repository,
            vendor_repository,
            purchase_repository,
            archive_service,
            merge_service: None,
            db: Some(db),
        }
    }

    pub fn with_merge_service(mut self, merge_service: Arc<MergeService>) -> Self {
        self.merge_service = Some(merge_service);
        self
    }

    // ─── Public API ─────────────────────────────────────────────────────────────

    pub async fn import_all(
        &self,
        data: BackupData,
        strategy: ConflictStrategy,
    ) -> Result<ImportSummary, ImportError> {
        let mut summary = ImportSummary::default();

        // Analysis pass: resolve each booth and build remapping map
        let mut outcomes: HashMap<BoothId, BoothImportOutcome> = HashMap::new();
        for booth in &data.booths {
            match self.resolve_canonical_booth(booth).await? {
                BoothResolution::New => {
                    outcomes.insert(
                        booth.id,
                        BoothImportOutcome {
                            canonical_id: booth.id,
                            match_kind: None,
                        },
                    );
                }
                BoothResolution::Single(candidate) => {
                    outcomes.insert(
                        booth.id,
                        BoothImportOutcome {
                            canonical_id: candidate.id,
                            match_kind: Some(candidate.match_kind),
                        },
                    );
                }
                BoothResolution::Ambiguous(_) => {
                    summary.skipped_records.push(SkippedRecord {
                        record_type: "booth".to_string(),
                        record_id: booth.id.to_string(),
                        reason: format!(
                            "ambiguous: multiple local events match '{}' on {} — use the duplicate cleanup tool first, then re-import",
                            booth.description.trim(),
                            booth.date
                        ),
                    });
                }
                BoothResolution::UnresolvableAmbiguous => {
                    summary.skipped_records.push(SkippedRecord {
                        record_type: "booth".to_string(),
                        record_id: booth.id.to_string(),
                        reason: format!(
                            "unresolvable: multiple archived events match '{}' on {} — archive the duplicate locally, then re-import",
                            booth.description.trim(),
                            booth.date
                        ),
                    });
                }
            }
        }

        // Pre-write: restore any archived booths (must run before the main transaction)
        let restore_device_info = data.device_info.clone().unwrap_or_else(|| DeviceInfo {
            identifier: "import".to_string(),
            platform: "unknown".to_string(),
            browser: "unknown".to_string(),
        });
        for outcome in outcomes.values() {
            if outcome.match_kind.is_some()
                && self.is_archived_candidate(&outcome.canonical_id).await?
            {
                self.restore_archived_if_needed(&outcome.canonical_id, restore_device_info.clone())
                    .await?;
            }
        }

        // Write phase
        if let Some(db) = &self.db {
            self.write_all_transactional(db, &data, &outcomes, strategy, &mut summary)
                .await?;
        } else {
            self.write_all_sequential(&data, &outcomes, strategy, &mut summary)
                .await?;
        }

        Ok(summary)
    }

    pub async fn import_booth_backup(
        &self,
        data: BoothBackupData,
        strategy: ConflictStrategy,
    ) -> Result<ImportSummary, ImportError> {
        let mut summary = ImportSummary::default();

        // Restore archived canonical before writing (UI has already confirmed)
        let resolution = self.resolve_canonical_booth(&data.booth).await?;
        if let BoothResolution::Single(ref candidate) = resolution {
            if candidate.is_archived {
                let device_info = data.device_info.clone().unwrap_or_else(|| DeviceInfo {
                    identifier: "import".to_string(),
                    platform: "unknown".to_string(),
                    browser: "unknown".to_string(),
                });
                self.restore_archived_if_needed(&candidate.id, device_info)
                    .await?;
            }
        }

        let outcome = self
            .apply_resolution(&data.booth, resolution, strategy, &mut summary)
            .await?;

        if let Some(outcome) = outcome {
            let canonical_id = outcome.canonical_id;
            let needs_remap = matches!(outcome.match_kind, Some(BoothMatchKind::ByNameAndDate));

            for vendor in data.vendors {
                let vendor = if needs_remap {
                    Vendor {
                        booth_id: canonical_id,
                        ..vendor
                    }
                } else {
                    vendor
                };
                self.import_vendor_record(vendor, strategy, &mut summary)
                    .await?;
            }

            for purchase in data.purchases {
                let purchase = if needs_remap {
                    Purchase {
                        booth_id: canonical_id,
                        ..purchase
                    }
                } else {
                    purchase
                };
                self.import_purchase_record(purchase, strategy, &mut summary)
                    .await?;
            }
        }

        Ok(summary)
    }

    pub async fn import_booth_backup_restoring_archived(
        &self,
        data: BoothBackupData,
        strategy: ConflictStrategy,
        device_info: DeviceInfo,
    ) -> Result<ImportSummary, ImportError> {
        let resolution = self.resolve_canonical_booth(&data.booth).await?;

        match &resolution {
            BoothResolution::Single(candidate) if candidate.is_archived => {
                self.restore_archived_if_needed(&candidate.id, device_info)
                    .await?;
            }
            BoothResolution::New => {}
            _ => {}
        }

        self.import_booth_backup(data, strategy).await
    }

    /// Read-only pre-import analysis. Retries once on IDB failure.
    pub async fn analyze_import(
        &self,
        data: &ImportPayload,
    ) -> Result<ImportAnalysis, ImportError> {
        match self.analyze_import_inner(data).await {
            Ok(analysis) => Ok(analysis),
            Err(_) => self.analyze_import_inner(data).await,
        }
    }

    // ─── Resolution ─────────────────────────────────────────────────────────────

    /// 3-pass canonical booth resolution. Never writes.
    async fn resolve_canonical_booth(
        &self,
        incoming: &Booth,
    ) -> Result<BoothResolution, ImportError> {
        // Pass 1: UUID lookup across all booths
        let mut archived_by_id: Option<Booth> = None;

        match self.booth_repository.find_by_id(&incoming.id).await? {
            Some(existing) if !existing.is_archived() => {
                let candidate = self.make_candidate(&existing, BoothMatchKind::ById).await?;
                return Ok(BoothResolution::Single(candidate));
            }
            Some(existing) => {
                archived_by_id = Some(existing);
            }
            None => {}
        }

        // Pass 2+3: name+date lookup, split active / archived
        let all_matching = self
            .booth_repository
            .find_all_by_description_and_date(&incoming.description, &incoming.date)
            .await?;

        let mut active_matches: Vec<Booth> = Vec::new();
        let mut archived_matches: Vec<Booth> = Vec::new();
        for b in all_matching {
            if b.is_archived() {
                archived_matches.push(b);
            } else {
                active_matches.push(b);
            }
        }

        // Pass 2: active name+date matches
        match active_matches.len() {
            0 => {
                if let Some(archived) = archived_by_id {
                    let candidate = self.make_candidate(&archived, BoothMatchKind::ById).await?;
                    return Ok(BoothResolution::Single(candidate));
                }
                // Fall through to Pass 3
            }
            1 => {
                // Active match supersedes any archived UUID hit
                let candidate = self
                    .make_candidate(&active_matches[0], BoothMatchKind::ByNameAndDate)
                    .await?;
                return Ok(BoothResolution::Single(candidate));
            }
            _ => {
                let mut candidates = Vec::new();
                for b in &active_matches {
                    let candidate = self
                        .make_candidate(b, BoothMatchKind::ByNameAndDate)
                        .await?;
                    candidates.push(candidate);
                }
                return Ok(BoothResolution::Ambiguous(candidates));
            }
        }

        // Pass 3: archived name+date matches
        match archived_matches.len() {
            0 => Ok(BoothResolution::New),
            1 => {
                let candidate = self
                    .make_candidate(&archived_matches[0], BoothMatchKind::ByNameAndDate)
                    .await?;
                Ok(BoothResolution::Single(candidate))
            }
            _ => Ok(BoothResolution::UnresolvableAmbiguous),
        }
    }

    async fn make_candidate(
        &self,
        booth: &Booth,
        match_kind: BoothMatchKind,
    ) -> Result<BoothCandidate, ImportError> {
        let vendors = self.vendor_repository.find_by_booth(&booth.id).await?;
        let purchases = self.purchase_repository.find_by_booth(&booth.id).await?;
        Ok(booth_to_candidate(
            booth,
            match_kind,
            vendors.len(),
            purchases.len(),
        ))
    }

    // ─── Write helpers (sequential / repository-based) ───────────────────────

    /// Apply a pre-computed resolution to produce an import outcome.
    async fn apply_resolution(
        &self,
        incoming: &Booth,
        resolution: BoothResolution,
        strategy: ConflictStrategy,
        summary: &mut ImportSummary,
    ) -> Result<Option<BoothImportOutcome>, ImportError> {
        match resolution {
            BoothResolution::New => {
                self.booth_repository.save(incoming).await?;
                summary.booths_imported += 1;
                Ok(Some(BoothImportOutcome {
                    canonical_id: incoming.id,
                    match_kind: None,
                }))
            }
            BoothResolution::Single(candidate) => {
                let canonical_id = candidate.id;
                self.apply_booth_strategy(incoming, &candidate, strategy, summary)
                    .await?;
                Ok(Some(BoothImportOutcome {
                    canonical_id,
                    match_kind: Some(candidate.match_kind),
                }))
            }
            BoothResolution::Ambiguous(_) => {
                summary.skipped_records.push(SkippedRecord {
                    record_type: "booth".to_string(),
                    record_id: incoming.id.to_string(),
                    reason: format!(
                        "ambiguous: multiple local events match '{}' on {} — use the duplicate cleanup tool first, then re-import",
                        incoming.description.trim(),
                        incoming.date
                    ),
                });
                Ok(None)
            }
            BoothResolution::UnresolvableAmbiguous => {
                summary.skipped_records.push(SkippedRecord {
                    record_type: "booth".to_string(),
                    record_id: incoming.id.to_string(),
                    reason: format!(
                        "unresolvable: multiple archived events match '{}' on {} — archive the duplicate locally, then re-import",
                        incoming.description.trim(),
                        incoming.date
                    ),
                });
                Ok(None)
            }
        }
    }

    /// Resolve + apply one booth write. Returns outcome for subordinate remapping.
    #[allow(dead_code)]
    async fn resolve_and_apply_booth(
        &self,
        incoming: &Booth,
        strategy: ConflictStrategy,
        summary: &mut ImportSummary,
    ) -> Result<Option<BoothImportOutcome>, ImportError> {
        let resolution = self.resolve_canonical_booth(incoming).await?;
        self.apply_resolution(incoming, resolution, strategy, summary)
            .await
    }

    async fn apply_booth_strategy(
        &self,
        incoming: &Booth,
        candidate: &BoothCandidate,
        strategy: ConflictStrategy,
        summary: &mut ImportSummary,
    ) -> Result<(), ImportError> {
        let canonical_id = candidate.id;
        match strategy {
            ConflictStrategy::Skip => {
                // Skip = metadata only; subordinate records still import
                summary.skipped_records.push(SkippedRecord {
                    record_type: "booth".to_string(),
                    record_id: incoming.id.to_string(),
                    reason: if matches!(candidate.match_kind, BoothMatchKind::ByNameAndDate) {
                        "cross-device duplicate: booth metadata not updated".to_string()
                    } else {
                        "record already exists".to_string()
                    },
                });
            }
            ConflictStrategy::Replace => {
                let canonical = Booth {
                    id: canonical_id,
                    ..incoming.clone()
                };
                self.booth_repository.save(&canonical).await?;
                summary.booths_imported += 1;
                summary.conflicts_resolved += 1;
            }
            ConflictStrategy::Merge => {
                let existing = self.booth_repository.find_by_id(&canonical_id).await?;
                let saved = if let Some(existing) = existing {
                    if incoming.updated_at > existing.updated_at {
                        Booth {
                            id: canonical_id,
                            ..incoming.clone()
                        }
                    } else {
                        existing
                    }
                } else {
                    Booth {
                        id: canonical_id,
                        ..incoming.clone()
                    }
                };
                self.booth_repository.save(&saved).await?;
                summary.booths_imported += 1;
                summary.conflicts_resolved += 1;
            }
        }
        Ok(())
    }

    async fn import_vendor_record(
        &self,
        incoming: Vendor,
        strategy: ConflictStrategy,
        summary: &mut ImportSummary,
    ) -> Result<(), ImportError> {
        match self
            .vendor_repository
            .find_by_id(&incoming.booth_id, &incoming.vendor_id)
            .await?
        {
            None => {
                self.vendor_repository.save(&incoming).await?;
                summary.vendors_imported += 1;
            }
            Some(existing) => match strategy {
                ConflictStrategy::Skip => summary.skipped_records.push(SkippedRecord {
                    record_type: "vendor".to_string(),
                    record_id: incoming.vendor_id.to_string(),
                    reason: format!("record already exists in booth {}", incoming.booth_id),
                }),
                ConflictStrategy::Replace => {
                    self.vendor_repository.save(&incoming).await?;
                    summary.vendors_imported += 1;
                    summary.conflicts_resolved += 1;
                }
                ConflictStrategy::Merge => {
                    let merged = Vendor {
                        created_at: incoming.created_at.min(existing.created_at),
                        // Keep canonical's payout fields if set; take incoming's if canonical has none
                        payout_correction: if existing.payout_correction.is_some() {
                            existing.payout_correction
                        } else {
                            incoming.payout_correction
                        },
                        payout_correction_note: if existing.payout_correction_note.is_some() {
                            existing.payout_correction_note
                        } else {
                            incoming.payout_correction_note
                        },
                        ..incoming
                    };
                    self.vendor_repository.save(&merged).await?;
                    summary.vendors_imported += 1;
                    summary.conflicts_resolved += 1;
                }
            },
        }
        Ok(())
    }

    async fn import_purchase_record(
        &self,
        incoming: Purchase,
        strategy: ConflictStrategy,
        summary: &mut ImportSummary,
    ) -> Result<(), ImportError> {
        match self.purchase_repository.find_by_id(&incoming.id).await? {
            None => {
                self.purchase_repository.save(&incoming).await?;
                summary.purchases_imported += 1;
            }
            Some(existing) => match strategy {
                ConflictStrategy::Skip => summary.skipped_records.push(SkippedRecord {
                    record_type: "purchase".to_string(),
                    record_id: incoming.id.to_string(),
                    reason: "record already exists".to_string(),
                }),
                ConflictStrategy::Replace => {
                    if existing.booth_id != incoming.booth_id {
                        self.purchase_repository
                            .delete_from_booth(&existing.booth_id, &existing.id)
                            .await?;
                    }
                    self.purchase_repository.save(&incoming).await?;
                    summary.purchases_imported += 1;
                    summary.conflicts_resolved += 1;
                }
                ConflictStrategy::Merge => {
                    let existing_booth_id = existing.booth_id;
                    let existing_id = existing.id;
                    let merged = if incoming.timestamp > existing.timestamp {
                        incoming
                    } else {
                        existing
                    };
                    if merged.id == existing_id && existing_booth_id != merged.booth_id {
                        self.purchase_repository
                            .delete_from_booth(&existing_booth_id, &existing_id)
                            .await?;
                    }
                    self.purchase_repository.save(&merged).await?;
                    summary.purchases_imported += 1;
                    summary.conflicts_resolved += 1;
                }
            },
        }
        Ok(())
    }

    // ─── Sequential write path (test/fallback) ──────────────────────────────

    async fn write_all_sequential(
        &self,
        data: &BackupData,
        outcomes: &HashMap<BoothId, BoothImportOutcome>,
        strategy: ConflictStrategy,
        summary: &mut ImportSummary,
    ) -> Result<(), ImportError> {
        for booth in &data.booths {
            if let Some(outcome) = outcomes.get(&booth.id) {
                match &outcome.match_kind {
                    None => {
                        self.booth_repository.save(booth).await?;
                        summary.booths_imported += 1;
                    }
                    Some(match_kind) => {
                        let candidate = BoothCandidate {
                            id: outcome.canonical_id,
                            description: booth.description.clone(),
                            date: booth.date,
                            match_kind: match_kind.clone(),
                            is_archived: false,
                            vendor_count: 0,
                            purchase_count: 0,
                            updated_at: booth.updated_at,
                        };
                        self.apply_booth_strategy(booth, &candidate, strategy, summary)
                            .await?;
                    }
                }
            }
        }

        for vendor in &data.vendors {
            match outcomes.get(&vendor.booth_id) {
                None => summary.skipped_records.push(SkippedRecord {
                    record_type: "vendor".to_string(),
                    record_id: vendor.vendor_id.to_string(),
                    reason: "booth_id not found in import".to_string(),
                }),
                Some(outcome) => {
                    let vendor =
                        if matches!(outcome.match_kind, Some(BoothMatchKind::ByNameAndDate)) {
                            Vendor {
                                booth_id: outcome.canonical_id,
                                ..vendor.clone()
                            }
                        } else {
                            vendor.clone()
                        };
                    self.import_vendor_record(vendor, strategy, summary).await?;
                }
            }
        }

        for purchase in &data.purchases {
            match outcomes.get(&purchase.booth_id) {
                None => summary.skipped_records.push(SkippedRecord {
                    record_type: "purchase".to_string(),
                    record_id: purchase.id.to_string(),
                    reason: "booth_id not found in import".to_string(),
                }),
                Some(outcome) => {
                    let purchase =
                        if matches!(outcome.match_kind, Some(BoothMatchKind::ByNameAndDate)) {
                            Purchase {
                                booth_id: outcome.canonical_id,
                                ..purchase.clone()
                            }
                        } else {
                            purchase.clone()
                        };
                    self.import_purchase_record(purchase, strategy, summary)
                        .await?;
                }
            }
        }

        Ok(())
    }

    // ─── Transactional write path ────────────────────────────────────────────

    async fn write_all_transactional(
        &self,
        db: &Database,
        data: &BackupData,
        outcomes: &HashMap<BoothId, BoothImportOutcome>,
        strategy: ConflictStrategy,
        summary: &mut ImportSummary,
    ) -> Result<(), ImportError> {
        let transaction = db
            .transaction(
                &["booths", "vendors", "purchases"],
                TransactionMode::ReadWrite,
            )
            .map_err(|e| {
                ImportError::Storage(StorageError::TransactionError(format!("{:?}", e)))
            })?;

        // Write booths
        for booth in &data.booths {
            if let Some(outcome) = outcomes.get(&booth.id) {
                match &outcome.match_kind {
                    None => {
                        tx_save_booth(&transaction, booth).await?;
                        summary.booths_imported += 1;
                    }
                    Some(BoothMatchKind::ById) | Some(BoothMatchKind::ByNameAndDate) => {
                        let canonical_id = outcome.canonical_id;
                        match strategy {
                            ConflictStrategy::Skip => {
                                summary.skipped_records.push(SkippedRecord {
                                    record_type: "booth".to_string(),
                                    record_id: booth.id.to_string(),
                                    reason: if matches!(
                                        outcome.match_kind,
                                        Some(BoothMatchKind::ByNameAndDate)
                                    ) {
                                        "cross-device duplicate: booth metadata not updated"
                                            .to_string()
                                    } else {
                                        "record already exists".to_string()
                                    },
                                });
                            }
                            ConflictStrategy::Replace => {
                                let b = Booth {
                                    id: canonical_id,
                                    ..booth.clone()
                                };
                                tx_save_booth(&transaction, &b).await?;
                                summary.booths_imported += 1;
                                summary.conflicts_resolved += 1;
                            }
                            ConflictStrategy::Merge => {
                                let existing = tx_load_booth(&transaction, &canonical_id).await?;
                                let saved = match existing {
                                    Some(ex) if ex.updated_at >= booth.updated_at => ex,
                                    _ => Booth {
                                        id: canonical_id,
                                        ..booth.clone()
                                    },
                                };
                                tx_save_booth(&transaction, &saved).await?;
                                summary.booths_imported += 1;
                                summary.conflicts_resolved += 1;
                            }
                        }
                    }
                }
            }
        }

        // Write vendors (remapped + get-or-update)
        for vendor in &data.vendors {
            match outcomes.get(&vendor.booth_id) {
                None => summary.skipped_records.push(SkippedRecord {
                    record_type: "vendor".to_string(),
                    record_id: vendor.vendor_id.to_string(),
                    reason: "booth_id not found in import".to_string(),
                }),
                Some(outcome) => {
                    let vendor =
                        if matches!(outcome.match_kind, Some(BoothMatchKind::ByNameAndDate)) {
                            Vendor {
                                booth_id: outcome.canonical_id,
                                ..vendor.clone()
                            }
                        } else {
                            vendor.clone()
                        };
                    match tx_find_vendor(&transaction, &vendor.booth_id, &vendor.vendor_id).await? {
                        None => {
                            tx_save_vendor(&transaction, &vendor).await?;
                            summary.vendors_imported += 1;
                        }
                        Some(existing) => match strategy {
                            ConflictStrategy::Skip => summary.skipped_records.push(SkippedRecord {
                                record_type: "vendor".to_string(),
                                record_id: vendor.vendor_id.to_string(),
                                reason: format!(
                                    "record already exists in booth {}",
                                    vendor.booth_id
                                ),
                            }),
                            ConflictStrategy::Replace => {
                                tx_save_vendor(&transaction, &vendor).await?;
                                summary.vendors_imported += 1;
                                summary.conflicts_resolved += 1;
                            }
                            ConflictStrategy::Merge => {
                                let merged = Vendor {
                                    created_at: vendor.created_at.min(existing.created_at),
                                    payout_correction: if existing.payout_correction.is_some() {
                                        existing.payout_correction
                                    } else {
                                        vendor.payout_correction
                                    },
                                    payout_correction_note: if existing
                                        .payout_correction_note
                                        .is_some()
                                    {
                                        existing.payout_correction_note
                                    } else {
                                        vendor.payout_correction_note
                                    },
                                    ..vendor
                                };
                                tx_save_vendor(&transaction, &merged).await?;
                                summary.vendors_imported += 1;
                                summary.conflicts_resolved += 1;
                            }
                        },
                    }
                }
            }
        }

        // Write purchases (remapped)
        for purchase in &data.purchases {
            match outcomes.get(&purchase.booth_id) {
                None => summary.skipped_records.push(SkippedRecord {
                    record_type: "purchase".to_string(),
                    record_id: purchase.id.to_string(),
                    reason: "booth_id not found in import".to_string(),
                }),
                Some(outcome) => {
                    let purchase =
                        if matches!(outcome.match_kind, Some(BoothMatchKind::ByNameAndDate)) {
                            Purchase {
                                booth_id: outcome.canonical_id,
                                ..purchase.clone()
                            }
                        } else {
                            purchase.clone()
                        };
                    // Purchases are always additive — just save if not already present
                    tx_save_purchase(&transaction, &purchase).await?;
                    summary.purchases_imported += 1;
                }
            }
        }

        transaction.done().await.map_err(|e| {
            ImportError::Storage(StorageError::TransactionError(format!("{:?}", e)))
        })?;

        Ok(())
    }

    // ─── Analysis (read-only) ────────────────────────────────────────────────

    async fn analyze_import_inner(
        &self,
        data: &ImportPayload,
    ) -> Result<ImportAnalysis, ImportError> {
        let booths_iter: Box<dyn Iterator<Item = &Booth>> = match data {
            ImportPayload::Full(full) => Box::new(full.booths.iter()),
            ImportPayload::Booth(single) => Box::new(std::iter::once(&single.booth)),
        };

        // Pre-build vendor/purchase count maps for Full backup (single pass, avoids O(n×m))
        let counts: HashMap<BoothId, (usize, usize)> = match data {
            ImportPayload::Full(full) => {
                let mut map: HashMap<BoothId, (usize, usize)> = HashMap::new();
                for v in &full.vendors {
                    map.entry(v.booth_id).or_default().0 += 1;
                }
                for p in &full.purchases {
                    map.entry(p.booth_id).or_default().1 += 1;
                }
                map
            }
            ImportPayload::Booth(single) => {
                let mut map = HashMap::new();
                map.insert(
                    single.booth.id,
                    (single.vendors.len(), single.purchases.len()),
                );
                map
            }
        };

        let mut analyses = Vec::new();
        for booth in booths_iter {
            let resolution = self.resolve_canonical_booth(booth).await?;
            let (vendor_count, purchase_count) = counts.get(&booth.id).copied().unwrap_or((0, 0));
            analyses.push(BoothImportAnalysis {
                incoming_id: booth.id,
                incoming_description: booth.description.clone(),
                incoming_date: booth.date,
                resolution,
                vendor_count,
                purchase_count,
            });
        }

        Ok(ImportAnalysis { booths: analyses })
    }

    // ─── Archive helpers ─────────────────────────────────────────────────────

    async fn is_archived_candidate(&self, id: &BoothId) -> Result<bool, ImportError> {
        Ok(self
            .booth_repository
            .find_by_id(id)
            .await?
            .map(|b| b.is_archived())
            .unwrap_or(false))
    }

    async fn restore_archived_if_needed(
        &self,
        canonical_id: &BoothId,
        device_info: DeviceInfo,
    ) -> Result<(), ImportError> {
        if let Some(archive_service) = &self.archive_service {
            archive_service
                .restore_booth(canonical_id, device_info)
                .await?;
        }
        Ok(())
    }
}

// ─── Transaction-level helpers ───────────────────────────────────────────────

async fn tx_load_booth(
    tx: &rexie::Transaction,
    id: &BoothId,
) -> Result<Option<Booth>, ImportError> {
    let store = tx
        .store("booths")
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?;
    match store
        .get(JsValue::from_str(&id.as_str()))
        .await
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?
    {
        Some(v) => {
            let booth: Booth = from_value(v).map_err(|e| {
                ImportError::Storage(StorageError::SerializationError(e.to_string()))
            })?;
            Ok(Some(booth))
        }
        None => Ok(None),
    }
}

async fn tx_save_booth(tx: &rexie::Transaction, booth: &Booth) -> Result<(), ImportError> {
    let store = tx
        .store("booths")
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?;
    let value = to_value(booth)
        .map_err(|e| ImportError::Storage(StorageError::SerializationError(e.to_string())))?;
    store
        .put(&value, None)
        .await
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?;
    Ok(())
}

fn vendor_key(booth_id: &BoothId, vendor_id: &VendorId) -> JsValue {
    let arr = js_sys::Array::new();
    arr.push(&JsValue::from_str(&booth_id.as_str()));
    arr.push(&JsValue::from_str(vendor_id.as_str()));
    arr.into()
}

async fn tx_find_vendor(
    tx: &rexie::Transaction,
    booth_id: &BoothId,
    vendor_id: &VendorId,
) -> Result<Option<Vendor>, ImportError> {
    let store = tx
        .store("vendors")
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?;
    match store
        .get(vendor_key(booth_id, vendor_id))
        .await
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?
    {
        Some(v) => Ok(Some(from_value(v).map_err(|e| {
            ImportError::Storage(StorageError::SerializationError(e.to_string()))
        })?)),
        None => Ok(None),
    }
}

async fn tx_save_vendor(tx: &rexie::Transaction, vendor: &Vendor) -> Result<(), ImportError> {
    let store = tx
        .store("vendors")
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?;
    let value = to_value(vendor)
        .map_err(|e| ImportError::Storage(StorageError::SerializationError(e.to_string())))?;
    store
        .put(&value, None)
        .await
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?;
    Ok(())
}

async fn tx_save_purchase(tx: &rexie::Transaction, purchase: &Purchase) -> Result<(), ImportError> {
    let store = tx
        .store("purchases")
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?;
    let value = to_value(purchase)
        .map_err(|e| ImportError::Storage(StorageError::SerializationError(e.to_string())))?;
    store
        .put(&value, None)
        .await
        .map_err(|e| ImportError::Storage(StorageError::DatabaseError(format!("{:?}", e))))?;
    Ok(())
}

use chrono::{DateTime, NaiveDate, Utc};
use domain::{Booth, BoothId};

use super::backup_format::{BackupData, BoothBackupData};

/// How the canonical local booth was found during import resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoothMatchKind {
    /// Incoming UUID matched an existing booth (same-device or re-import)
    ById,
    /// Same description+date, different UUID (cross-device duplicate)
    ByNameAndDate,
}

/// A resolved local booth candidate with metadata for UI display
#[derive(Debug, Clone, PartialEq)]
pub struct BoothCandidate {
    pub id: BoothId,
    pub description: String,
    pub date: NaiveDate,
    pub match_kind: BoothMatchKind,
    pub is_archived: bool,
    pub vendor_count: usize,
    pub purchase_count: usize,
    pub updated_at: DateTime<Utc>,
}

/// Result of resolving a canonical local booth for an incoming booth record
#[derive(Debug, Clone, PartialEq)]
pub enum BoothResolution {
    /// No local booth found — incoming booth will be inserted as new
    New,
    /// Exactly one local booth resolved
    Single(BoothCandidate),
    /// Two or more active local booths share the same name+date — operator must choose
    Ambiguous(Vec<BoothCandidate>),
    /// Two or more archived booths match by name+date with no active match — cannot auto-resolve
    UnresolvableAmbiguous,
}

/// Input to import and analysis operations, replacing the UI-private ParsedImportData
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ImportPayload {
    Full(BackupData),
    Booth(BoothBackupData),
}

/// Per-booth analysis result for the pre-import analysis pass
#[derive(Debug, Clone, PartialEq)]
pub struct BoothImportAnalysis {
    pub incoming_id: BoothId,
    pub incoming_description: String,
    pub incoming_date: NaiveDate,
    pub resolution: BoothResolution,
    pub vendor_count: usize,
    pub purchase_count: usize,
}

/// Full analysis result returned by `ImportService::analyze_import`
#[derive(Debug, Clone, PartialEq)]
pub struct ImportAnalysis {
    pub booths: Vec<BoothImportAnalysis>,
}

impl ImportAnalysis {
    pub fn has_ambiguous(&self) -> bool {
        self.booths
            .iter()
            .any(|b| matches!(b.resolution, BoothResolution::Ambiguous(_)))
    }

    pub fn has_archived_single(&self) -> bool {
        self.booths
            .iter()
            .any(|b| matches!(&b.resolution, BoothResolution::Single(c) if c.is_archived))
    }
}

/// Build a BoothCandidate from a Booth with precomputed counts
pub(crate) fn booth_to_candidate(
    booth: &Booth,
    match_kind: BoothMatchKind,
    vendor_count: usize,
    purchase_count: usize,
) -> BoothCandidate {
    BoothCandidate {
        id: booth.id,
        description: booth.description.clone(),
        date: booth.date,
        match_kind,
        is_archived: booth.is_archived(),
        vendor_count,
        purchase_count,
        updated_at: booth.updated_at,
    }
}

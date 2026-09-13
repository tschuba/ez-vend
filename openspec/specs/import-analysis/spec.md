## ADDED Requirements

### Requirement: analyze_import is read-only and returns per-booth resolution
`ImportService` SHALL expose `analyze_import(data: &ImportPayload) -> Result<ImportAnalysis, ImportError>` which resolves each booth in the payload and returns a `BoothImportAnalysis` per booth. No writes occur during analysis.

#### Scenario: Analysis resolves each booth independently
- **WHEN** `analyze_import` is called with a full backup containing N booths
- **THEN** `ImportAnalysis.booths` contains exactly N entries, one per incoming booth, each with its `BoothResolution` and diff counts

#### Scenario: Analysis failure retries once automatically
- **WHEN** `analyze_import` fails on the first attempt due to an IDB error
- **THEN** it retries once automatically — if the retry also fails, a blocking error is surfaced with the raw IDB error string and safe recovery steps

### Requirement: Vendor and purchase diff counts are computed in a single pass
For `ImportPayload::Full`, `analyze_import` SHALL pre-build a `HashMap<BoothId, (vendor_count, purchase_count)>` in a single pass over the data before iterating booths. This avoids O(n×m) per-booth scans.

#### Scenario: Counts are available per booth in ImportAnalysis
- **WHEN** a full backup is analyzed
- **THEN** each `BoothImportAnalysis` includes the incoming vendor count and purchase count for that booth

### Requirement: ImportPayload replaces UI-private ParsedImportData
`ImportPayload { Full(BackupData), Booth(BoothBackupData) }` SHALL be defined in `crates/storage/src/export/analysis.rs` and re-exported from `export/mod.rs`. The UI-private `ParsedImportData` enum SHALL be removed and replaced with `ImportPayload` at all UI call sites.

#### Scenario: UI imports ImportPayload from storage crate
- **WHEN** the import button parses a file
- **THEN** it constructs an `ImportPayload` (not a UI-local type) and passes it to both `analyze_import` and the import write methods

### Requirement: Analysis is triggered async after file parse, before modal opens
Analysis SHALL run inside the existing `spawn_local` in `on_file_change`, after the parse loop and before `set_candidates` / `set_show_modal`. All analysis results are collected and then the signal is set once — not updated per-candidate.

#### Scenario: Modal opens with analysis already available
- **WHEN** the operator selects a file to import
- **THEN** the modal opens with `AnalysisState::Available` on each candidate — not in a loading state

#### Scenario: Analysis failure does not block import
- **WHEN** `analyze_import` fails for a candidate after one retry
- **THEN** the candidate's `AnalysisState` is `Failed(msg)` — the import button remains enabled and the operator can still import without the analysis indicators

### Requirement: Per-booth outcome indicators in the import modal
The import modal SHALL display a per-booth indicator based on `match_kind`:

| match_kind | Indicator |
|---|---|
| None (New) | "New event" — neutral/blue |
| ById | "Will merge (same device)" — green |
| ByNameAndDate | "Will merge into '*{existing_description}*' from another device" — amber, prominent |

#### Scenario: Cross-device match shows amber indicator
- **WHEN** a booth is resolved via `ByNameAndDate` in the analysis
- **THEN** the import modal shows an amber "Will merge into" indicator naming the existing local event

#### Scenario: New booth shows neutral indicator
- **WHEN** a booth has no local match (resolution is `New`)
- **THEN** the import modal shows a neutral "New event" indicator

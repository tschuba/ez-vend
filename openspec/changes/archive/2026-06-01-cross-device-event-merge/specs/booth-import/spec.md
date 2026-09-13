## ADDED Requirements

### Requirement: Canonical booth resolution uses 3-pass lookup
`ImportService` SHALL resolve the canonical local booth for each incoming booth record using a 3-pass algorithm before any write occurs. Pass 1 is UUID lookup across all booths (active and archived). Pass 2 is name+date lookup across active booths only. Pass 3 is name+date lookup across archived booths only. Active booths always take priority over archived ones.

#### Scenario: UUID matches active booth (case 1a)
- **WHEN** the incoming booth's UUID matches an active local booth
- **THEN** the resolution is `Single(ById/active)` and `ConflictStrategy` is applied to booth metadata

#### Scenario: UUID matches archived booth only, no active name+date match (case 1b)
- **WHEN** the incoming booth's UUID matches an archived local booth AND no active booth with the same name and date exists
- **THEN** the resolution is `Single(ById/archived)` and the archived booth is restored before import

#### Scenario: UUID matches archived booth but active exists with same key (case 1c)
- **WHEN** the incoming booth's UUID matches an archived local booth AND an active booth with the same name and date also exists
- **THEN** the resolution is `Single(ByNameAndDate/active)` — the active booth is used and the archived booth is left untouched

#### Scenario: No UUID match, one active booth with same name+date (case 2a)
- **WHEN** no local booth matches the incoming UUID AND exactly one active booth shares the incoming name and date
- **THEN** the resolution is `Single(ByNameAndDate/active)` and `ConflictStrategy` is applied using the existing local booth as canonical

#### Scenario: No UUID match, two or more active booths with same key (case 2b)
- **WHEN** no local booth matches the incoming UUID AND two or more active booths share the incoming name and date
- **THEN** the resolution is `Ambiguous(Vec<BoothCandidate>)` and the import write path produces a `SkippedRecord` with guidance to resolve duplicates first

#### Scenario: No UUID match, no active match, one archived name+date match (case 2c)
- **WHEN** no local booth matches the incoming UUID AND no active booth matches the name and date AND exactly one archived booth matches the name and date
- **THEN** the resolution is `Single(ByNameAndDate/archived)` and the archived booth is restored before import

#### Scenario: No UUID match, no active match, two or more archived name+date matches (case 2d)
- **WHEN** no local booth matches the incoming UUID AND no active booth matches the name and date AND two or more archived booths match the name and date
- **THEN** the resolution is `UnresolvableAmbiguous` and a `SkippedRecord` is produced — the rest of the import continues

#### Scenario: No UUID match, no active match, no archived match (case 2e)
- **WHEN** no local booth matches the incoming UUID AND no booth (active or archived) matches the name and date
- **THEN** the resolution is `New` and the incoming booth is inserted as a new active booth

### Requirement: Booth ID remapping for name+date matches
When a booth is resolved via `ByNameAndDate`, `ImportService` SHALL remap all `vendor.booth_id` and `purchase.booth_id` fields in the incoming records to the canonical local booth ID before saving.

#### Scenario: Vendor and purchase IDs are remapped on cross-device import
- **WHEN** an incoming booth is resolved via `ByNameAndDate` (different UUID, same name+date)
- **THEN** all vendors and purchases from the incoming file that belong to that booth are saved under the canonical local booth's UUID, not the incoming UUID

#### Scenario: No remapping when booth matched by UUID
- **WHEN** an incoming booth is resolved via `ById`
- **THEN** vendor and purchase `booth_id` fields are saved as-is (no remapping needed)

### Requirement: Orphaned records produce SkippedRecord, not storage errors
In `import_all`, if a vendor or purchase references a `booth_id` that is not present in the import resolution map, `ImportService` SHALL push a `SkippedRecord` with reason "booth_id not found in import" and continue importing other records.

#### Scenario: Orphaned vendor in full backup
- **WHEN** a full backup contains a vendor whose `booth_id` does not correspond to any booth in the same backup
- **THEN** a `SkippedRecord` is produced for that vendor and the import continues

### Requirement: ConflictStrategy applies only to booth metadata and payout_correction
`ConflictStrategy` SHALL govern booth metadata fields and vendor `payout_correction` only. Vendor existence and purchase accumulation are unconditionally additive regardless of strategy.

#### Scenario: Skip strategy still imports vendors and purchases
- **WHEN** an incoming booth is resolved to an existing local booth and `ConflictStrategy` is `Skip`
- **THEN** the local booth's metadata is not updated AND all incoming vendors and purchases are still imported under the canonical booth ID

#### Scenario: Merge strategy uses newer updated_at for booth metadata
- **WHEN** an incoming booth is resolved to an existing local booth and `ConflictStrategy` is `Merge`
- **THEN** the booth metadata saved is from whichever side has the newer `updated_at` timestamp

#### Scenario: Replace strategy overwrites booth metadata
- **WHEN** an incoming booth is resolved to an existing local booth and `ConflictStrategy` is `Replace`
- **THEN** the incoming booth's metadata replaces the local booth's metadata, with the canonical local ID preserved

### Requirement: Import write phase is transactional
The `import_all` write phase SHALL be wrapped in a single IDB read-write transaction spanning booths, vendors, and purchases. If any write fails, the entire import rolls back and no partial state is persisted.

#### Scenario: Write failure triggers full rollback
- **WHEN** any write operation fails during `import_all`
- **THEN** all writes in that import are rolled back and the storage state is identical to before the import was initiated

### Requirement: Archived booth restore requires ArchiveService
When a `Single(archived)` resolution requires restoring a booth, `ImportService` SHALL use `ArchiveService` to perform the restore. Tests that cover archived booth cases MUST provide `ImportService::with_archive_service(...)`.

#### Scenario: Restore uses canonical ID, not incoming ID
- **WHEN** a booth is resolved via `ByNameAndDate` to an archived booth
- **THEN** `ArchiveService.restore_booth()` is called with the canonical local booth's ID, not the incoming file's booth ID

### Requirement: `find_by_description_and_date` is a wrapper
The existing `BoothRepository::find_by_description_and_date` method SHALL remain on the trait and be implemented as `find_all_by_description_and_date(...).into_iter().next()`. No call sites are changed.

#### Scenario: Wrapper returns first match
- **WHEN** `find_by_description_and_date` is called and multiple booths match the key
- **THEN** it returns `Some` with one of the matching booths (first result)

---
title: Data Backup Implementation Plan
nav_order: 4
parent: Planning
---

# Data Backup And Recovery Implementation Plan

Prepared on 2026-03-29 to capture the agreed follow-up work for export/import, browser-storage warnings, and recovery guidance.

## Goal

Add practical operator-facing backup and recovery support without changing the validated money and reporting logic.

The work should make it safer to rely on EZ Booth in real event operation where browser data may be cleared intentionally or accidentally.

## Decisions Already Made

- export format: pretty-printed JSON only
- export scope: both full-database export and per-booth export
- UI placement: booth-list backup actions with app-wide warning surfaces
- storage warning strategy: both a first-visit warning and a persistent indicator
- documentation scope: both an operator guide and a technical guide

## Current Status

As of 2026-03-29, the planned backup and recovery track has been implemented and merged to `main` in focused PRs.

Merged PRs in this track:

- `#45` Export Foundation
- `#47` Export UI
- `#49` Booth Export Actions
- `#50` Import Validation
- `#51` Import Validation UI
- `#52` Import Apply Service
- `#53` Import Apply UI
- `#54` Storage Warnings
- `#55` Guides And Validation

Implemented outcomes:

- versioned JSON backup format for full and booth backups
- export service and import service in `crates/storage/src/export/`
- strict import validation with structure and relationship checks
- import conflict handling with `Skip`, `Replace`, and `Merge`
- booth-list backup entry points with app-wide warning surfaces
- booth-level export actions
- dismissible global storage warning plus persistent warning surfaces
- bilingual EN/DE copy for backup and warning flows
- operator and technical documentation
- updated Safari and bilingual UAT validation artifacts

## Remaining Work Before Sign-off

The main implementation is complete. The remaining work for this track is now:

1. capture and prioritize any final UX or wording improvements before manual validation
2. run the documented manual browser validation in Chrome and Safari
3. record validation evidence and any follow-up fixes in focused PRs

## Next Planned Slice

Before executing the final browser validation pass, use a small follow-up branch for any agreed polish work that improves:

- warning clarity
- backup/import usability
- recovery messaging
- validation ergonomics for operators

The exact improvement scope should be decided before starting the next branch so the follow-up remains small and reviewable.

## Problem Statement

Today the app stores booth data locally in the browser:

- IndexedDB stores booths, vendors, purchases, and metadata
- `localStorage` stores UI preferences, selected booth state, and checkout draft data

If a user clears browser storage, all locally stored booth data is lost. The current app has recovery safeguards for corruption and draft restore, but it does not yet provide a durable backup/restore workflow.

## Scope

This plan covers:

1. full export/import support for browser-stored data
2. per-booth export support for operator convenience
3. clear browser-storage warnings in the UI
4. operator and developer documentation for backup and recovery
5. validation coverage for the new workflows

This plan does not cover:

- cloud sync
- automatic remote backup
- storage-layer replacement
- report-format exports such as CSV
- financial logic changes

## Implementation Shape

### 1. Backup Format

Add a versioned JSON backup format in `crates/storage/src/export/`.

Recommended structures:

```rust
pub struct BackupData {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub app_version: String,
    pub booths: Vec<Booth>,
    pub vendors: Vec<Vendor>,
    pub purchases: Vec<Purchase>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct BoothBackupData {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub app_version: String,
    pub booth: Booth,
    pub vendors: Vec<Vendor>,
    pub purchases: Vec<Purchase>,
}
```

Serialization rules:

- pretty-printed JSON
- 2-space indentation
- UTF-8 encoding
- `.json` file extension
- filenames include the date and, for booth exports, a sanitized booth description

Examples:

- `ez-vend-backup-2026-03-29.json`
- `ez-vend-spring-market-2026-03-29.json`

### 2. Export And Import Services

Replace the placeholder export module with a real service layer in `crates/storage/src/export/`.

Suggested files:

- `backup_format.rs`
- `export_service.rs`
- `import_service.rs`
- `error.rs`
- `mod.rs`

Suggested responsibilities:

- `export_all()` returns a full backup payload
- `export_booth(booth_id)` returns a single-booth payload
- `validate_backup(raw)` parses JSON and validates format/version
- `import_all(data, strategy)` restores a full backup
- `import_booth(data, strategy)` restores a booth backup

Suggested conflict strategies:

- `Skip`: keep existing records when IDs conflict
- `Replace`: overwrite existing conflicting records
- `Merge`: import non-conflicting records and update matching ones when safe

Import validation should cover:

- invalid JSON
- unsupported backup version
- invalid or partial record structure
- orphaned booth/vendor/purchase relationships
- storage quota failures

### 3. UI Surfaces

#### Booth List Backup Access

Update `crates/ez-vend-ui/src/pages/booth_list.rs` to include:

- quick full export action
- quick import action
- per-booth export action on each booth card/row

#### Reusable Components

Add components in `crates/ez-vend-ui/src/components/` for:

- export button/download handling
- import file picker and validation flow
- import preview modal
- storage warning banner
- persistent storage indicator
- first-visit onboarding modal

## Warning Strategy

### First-Visit Warning

Show a dismissible first-visit message explaining:

- data is stored in this browser
- clearing browser data removes stored events, vendors, and purchases
- exporting backups is recommended before browser cleanup or device changes

Persist dismissal in `localStorage` so the warning does not reappear every session.

### Persistent Indicator

Keep a subtle persistent indicator available from the footer, banner, or booth list so operators can always find:

- where data is stored
- why backups matter
- how to export a backup

## Documentation Deliverables

### Operator Guide

Add `docs/user-guides/DATA_BACKUP_GUIDE.md` with bilingual or clearly structured operator-facing guidance covering:

- what browser-local storage means
- what happens when browser data is cleared
- how to export all data
- how to export a single booth
- how to import a backup
- recommended backup timing
- where to keep backup files
- what to do after data loss

### Technical Guide

Add `docs/technical/DATA_STORAGE_ARCHITECTURE.md` covering:

- IndexedDB and `localStorage` responsibilities
- backup JSON format
- versioning expectations
- recovery scenarios
- quota and browser behavior notes
- test and validation strategy

## Translation Work

Update both locale files:

- `crates/ez-vend-ui/locales/en.json`
- `crates/ez-vend-ui/locales/de.json`

Add strings for:

- export/import actions
- import validation and conflict messaging
- storage warnings and indicators
- operator guidance text

## Validation Plan

### Automated

Add storage tests for:

- full backup export/import
- per-booth export/import
- invalid JSON handling
- version mismatch handling
- conflict-strategy behavior
- large-payload behavior where practical

### Browser Validation

Update the manual artifacts to cover:

- export download in Chrome and Safari
- import restore in Chrome and Safari
- first-visit warning behavior
- persistent storage indicator behavior
- operator comprehension of the warning copy

Likely artifact updates:

- `docs/validation/SAFARI_VALIDATION_CHECKLIST.md`
- `docs/validation/UAT_Ausfuehrungsplan_DE_EN.html`
- milestone result file if this work is executed as a named milestone

## Recommended Execution Order

Completed:

1. implement the backup format and service layer
2. add unit and integration coverage for export/import behavior
3. add booth-list backup actions and app-wide access points
4. add first-visit warning and persistent indicator
5. add operator and technical documentation
6. update validation artifacts

Remaining:

7. apply final pre-validation improvements, if needed
8. run browser validation and capture results

## Branching And Pull Request Strategy

Execute this work through multiple small pull requests per sprint so backend, UI, validation, and docs can be reviewed separately when helpful.

Working rules for this track:

- branch from an up-to-date `main`
- use focused short-lived branches named `feature/backup-...`
- prefer one concern per branch, for example export backend, export UI, import validation, or warning copy
- open a pull request for review before merge
- use squash merge after review so `main` keeps one clean commit per approved change
- delete the feature branch after merge and start the next item from refreshed `main`

Suggested branch breakdown:

1. `feature/backup-export-format`
2. `feature/backup-export-ui`
3. `feature/backup-import-validation`
4. `feature/backup-import-ui`
5. `feature/backup-import-merge`
6. `feature/backup-booth-actions`
7. `feature/backup-storage-warning`
8. `feature/backup-docs-and-validation`

Completed branch sequence:

1. `feature/backup-export-format`
2. `feature/backup-export-ui`
3. `feature/backup-import-validation`
4. `feature/backup-import-ui`
5. `feature/backup-import-merge`
6. `feature/backup-booth-actions`
7. `feature/backup-storage-warning`
8. `feature/backup-docs-and-validation`

Recommended next follow-up branch, only if improvement work is approved:

9. `feature/backup-polish`

Commit guidance for each branch:

- follow the existing repository style such as `feat: ...`, `fix: ...`, `test: ...`, and `docs: ...`
- keep commits scoped and readable, even when the pull request will later be squash-merged
- include validation and documentation updates in the same branch when they are required to make the change reviewable
- avoid mixing unrelated cleanup into backup branches unless it is necessary for the backup work itself

Pull request guidance for this track:

- explain why the change is needed for backup and recovery, not just which files changed
- reference this document in the PR summary when relevant
- note automated validation run, for example `./run-tests.sh`
- note any manual browser validation completed or still pending in Chrome and Safari
- call out follow-up branches that depend on the PR being merged

## Acceptance Criteria

The work is complete when:

- operators can export a full backup from the UI
- operators can export a single booth from the UI
- operators can import a valid backup with clear conflict handling
- the app warns users that data is browser-local and can be lost when cleared
- English and German translations cover the new flows
- automated and manual validation prove the workflows in Chrome and Safari
- the repo contains clear documentation for both operators and developers

Implementation status: all acceptance criteria are implemented in `main`; manual browser validation evidence is the remaining sign-off activity.

## Open Follow-Up Decisions For Execution

These do not block implementation, but should be finalized while building the import flow:

- confirm whether `Merge` should update records only by ID or also use timestamps when both records are valid
- confirm how storage usage should be presented if browser APIs are unavailable

Decisions already confirmed for execution:

- source `app_version` from the app build/package version
- use strict validation before import so invalid backups are rejected with a clear error summary
- keep the storage warning as a dismissible banner rather than a blocking first-visit modal

## Notes For Future Extensions

Future phases may add:

- optional cloud backup/sync
- scheduled or reminder-based backups
- report-friendly export formats such as CSV
- backup migration helpers for future schema changes

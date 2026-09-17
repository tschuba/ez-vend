---
title: Data Storage Architecture
nav_order: 1
parent: Technical Docs
---

# Data Storage Architecture

This document describes how ez-vend stores event data locally, how backup files are structured, and how recovery flows are validated.

## Overview

ez-vend currently uses browser-local storage only. The app does not depend on a remote backend for booth, vendor, or purchase persistence.

Storage responsibilities are split between:

- IndexedDB for primary event data
- `localStorage` for lightweight UI state and operator conveniences
- downloaded JSON files for durable operator-managed backups

## Storage Layers

### IndexedDB

IndexedDB is the main persistence layer for business data:

- booths
- vendors
- purchases
- related metadata stored by the storage repositories

This is the data that full backup and booth backup exports protect.

### localStorage

`localStorage` is used for lighter browser-local state, including:

- selected booth state
- UI preferences
- checkout draft recovery state
- dismissal state for the storage warning banner

`localStorage` improves operator convenience but should not be treated as durable storage.

## Backup Format

Backup types live in `crates/storage/src/export/backup_format.rs`.

### Full backup

`BackupData` contains:

- `version`
- `created_at`
- `app_version`
- `booths`
- `vendors`
- `purchases`
- `metadata`

### Booth backup

`BoothBackupData` contains:

- `version`
- `created_at`
- `app_version`
- `booth`
- `vendors`
- `purchases`

### Serialization rules

- JSON only
- UTF-8 text
- pretty-printed with `serde_json::to_string_pretty`
- `.json` file extension
- format version currently fixed at `1`

Filename generation is deterministic:

- full backup: `ez-vend-backup-YYYY-MM-DD.json`
- booth backup: `ez-vend-<sanitized-description>-YYYY-MM-DD.json`

## Export Flow

The export implementation lives in `crates/storage/src/export/export_service.rs`.

`ExportService` is responsible for:

- reading all booth, vendor, and purchase records for full export
- reading a single booth plus related vendors and purchases for booth export
- serializing the selected payload to JSON
- returning the final filename and JSON payload

In the UI, `crates/ez-vend-ui/src/components/export_button.rs` triggers the export and uses browser download APIs to save the generated JSON file.

## Import Flow

Import validation and apply logic are intentionally separated.

### Validation

Validation lives in `crates/storage/src/export/import_validator.rs`.

The validator checks:

- JSON parsing
- backup format version
- booth backup structural consistency
- relationships between booths, vendors, and purchases
- record-level rules such as empty descriptions or invalid purchase amounts

Important detail: the UI validates booth backup format before full backup format. This avoids misclassifying empty booth backups as empty full backups because the schemas partially overlap.

### Apply phase

Apply logic lives in `crates/storage/src/export/import_service.rs`.

Supported conflict strategies:

- `Skip`: existing record wins, imported conflicting record is skipped
- `Replace`: imported record overwrites existing conflicting record
- `Merge`: service picks the safer merged outcome based on record type

Current merge behavior:

- booths: strictly newer `updated_at` wins; equal timestamps keep the existing local booth record
- vendors: preserves the earliest `created_at` when multiple devices provide the same vendor record
- purchases: strictly newer `timestamp` wins; equal timestamps keep the existing local purchase record; if booth ownership changes, the old booth mapping is removed first

### Verified merge guarantees

Focused browser-backed regression coverage now verifies these storage-layer guarantees for the current import implementation:

- repeated imports of the same booth history do not duplicate already-imported records when the record IDs match
- multi-device booth merges preserve unique purchases created independently on different devices
- round-trip imports merge new records without re-adding shared history
- full backup imports can merge one shared booth while preserving unrelated booths
- same-purchase conflicts across devices resolve to the strictly newer purchase record
- equal-timestamp booth and purchase conflicts keep the existing local record rather than flipping between payloads
- vendor merges preserve the earliest creation timestamp instead of blindly overwriting with the latest import

### Important limits

- purchases are identity-based, not content-deduplicated; ez-vend does not collapse different purchase IDs even if the amounts look similar
- multi-file import is sequential rather than atomic; a later file can fail after earlier files were already applied
- `Merge` is a practical recovery and synchronization strategy for single-team offline workflows, not a collaborative multi-user conflict-resolution system

The service returns `ImportSummary` so the UI can report:

- imported booth count
- imported vendor count
- imported purchase count
- resolved conflict count
- skipped records

## UI Surfaces

Backup and recovery entry points are available in multiple places:

- `crates/ez-vend-ui/src/pages/booth_list.rs`
- `crates/ez-vend-ui/src/lib.rs`

Relevant UI components:

- `crates/ez-vend-ui/src/components/export_button.rs`
- `crates/ez-vend-ui/src/components/import_button.rs`
- `crates/ez-vend-ui/src/components/storage_warning.rs`

The warning strategy has two layers:

- dismissible global banner for first-visit awareness
- persistent reminder surfaces in the global banner, footer, and booth list

## Recovery Scenarios

Supported operator scenarios include:

- restore the full application state from a full backup
- restore a deleted or missing booth from a booth backup
- re-import data with `Skip`, `Replace`, or `Merge`
- reject invalid or incomplete backup files before any apply step

One important bug fix in this track ensured that importing a booth backup after deleting the original event recreates the booth correctly and refreshes the UI afterwards.

## Browser And Quota Notes

- IndexedDB and `localStorage` behavior depends on the browser profile and device
- clearing browser storage can remove both event data and UI state
- downloaded backup files remain available only if the operator stores them outside the browser sandbox
- quota-related failures are modeled in the validation plan even when they are harder to reproduce consistently in local development

## Validation Strategy

Automated validation covers the storage layer and UI behavior.

Examples already covered in this track:

- full backup export and import behavior
- booth backup export and import behavior
- invalid JSON and unsupported version rejection
- orphaned relationship rejection
- delete-then-import booth recovery scenarios
- UI refresh after successful import
- repeated booth-history imports without duplicate records
- multi-device booth merges with parallel purchases
- round-trip merge behavior across repeated booth exports
- equal-timestamp conflict behavior for booth and purchase records
- larger multi-device booth merge count verification

Manual validation artifacts for backup and recovery live in:

- `docs/validation/SAFARI_VALIDATION_CHECKLIST.md`
- `docs/validation/UAT_Ausfuehrungsplan_DE_EN.html`
- `docs/validation/VALIDATION_WORKFLOW.md`

## Future Compatibility

The backup format is explicitly versioned so future schema changes can add migration or compatibility logic without changing the current import contract silently.

Future extensions may include:

- schema migration helpers
- reminder-based backup prompts
- cloud sync or remote backup
- richer export formats for reporting

## Why

When the same event is created independently on two devices, each gets a different random UUID. Importing device B's backup into device A creates a duplicate entry instead of merging — and there is no way to detect or clean up existing duplicates. This has been confirmed as a real operator pain point and is the foundational correctness issue for multi-device workflows.

## What Changes

- **Fix import duplicate bug**: `ImportService` currently matches booths only by UUID. After the fix, it falls back to `(description, date)` name matching when no UUID match is found, and merges into the existing local booth instead of creating a duplicate.
- **New `analysis.rs` module**: Introduces `BoothResolution`, `BoothCandidate`, `BoothMatchKind`, `ImportPayload` — the shared types that power analysis, import logic, and the wizard UI.
- **New `find_all_by_description_and_date` repository method**: Returns all booths matching a `(description, date)` key, enabling detection of local duplicates. The existing `find_by_description_and_date` becomes a wrapper over it.
- **New `MergeService`**: Local-to-local booth consolidation (`merge_booths`). Separate from `ImportService` since it has no relationship to backup files.
- **New `find_duplicate_groups` repository method**: Detects groups of active booths sharing the same `(description, date)` but different UUIDs.
- **Duplicate cleanup UI**: Amber banner on the event list when local duplicates exist. Modal with diff view (per-candidate vendor/purchase counts, shared vs. unique vendors, post-merge preview) and a consequence statement before confirming.
- **Import conflict wizard**: Dual-modal UX — simple modal for clean imports, step-by-step wizard when ambiguous booths are detected. Operators choose per-event before any writes begin.
- **Merge-then-import path**: The wizard's "Advanced" disclosure lets operators merge local duplicates and import in one guarded flow.
- **ConflictStrategy copy corrections**: Labels and descriptions updated to reflect true semantics — Skip = metadata only, vendors and purchases always imported.
- **Event list card detail**: Vendor count and last-updated timestamp visible on each booth card.

## Capabilities

### New Capabilities

- `booth-import`: Core import logic — UUID + name/date lookup, `BoothResolution` types, 3-pass canonical resolution algorithm, `ConflictStrategy` application, transactional write boundary.
- `booth-merge`: Local-to-local booth consolidation — `MergeService`, canonical selection, vendor re-assignment, purchase re-assignment, deletion of non-canonical booth.
- `import-analysis`: Pre-import read-only analysis — `analyze_import`, `BoothImportAnalysis`, per-booth match kind + incoming diff counts.
- `import-conflict-wizard`: Dual-modal import UX — simple modal for clean imports, conflict wizard for ambiguous cases, archived booth restore confirmation, `UnresolvableAmbiguous` callout.
- `duplicate-detection`: Detection and cleanup of pre-existing local duplicates — `find_duplicate_groups`, amber banner, merge modal with diff view.

### Modified Capabilities

## Impact

- `crates/storage/src/export/import_service.rs` — core logic rewrite
- `crates/storage/src/export/analysis.rs` — new file
- `crates/storage/src/export/mod.rs` — re-exports
- `crates/storage/src/merge_service.rs` — new file
- `crates/storage/src/repositories/booth_repository.rs` — new lookup methods
- `crates/domain/src/repositories.rs` — new trait methods
- Mock `BoothRepository` impls (~7 files) — new method signatures
- `crates/storage/tests/import_service_tests.rs` — new test cases
- `crates/ez-vend-ui/src/pages/booth_list.rs` — duplicate banner + merge modal + card detail
- `crates/ez-vend-ui/src/components/import_button.rs` — dual-modal UX, copy fixes
- `crates/ez-vend-ui/locales/en.json` + `de.json` — new wizard copy + corrected labels
- `docs/user-guides/MULTI_DEVICE_MERGE_GUIDE.md` — corrected semantics, new section
- `docs/technical/ADR_DEVICE_TO_DEVICE_TRANSFER.md` — scope note update

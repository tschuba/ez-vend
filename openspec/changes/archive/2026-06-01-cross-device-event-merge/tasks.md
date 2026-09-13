## 1. Storage Foundation — Repository Layer

- [x] 1.1 Add `find_all_by_description_and_date(description, date) -> DomainResult<Vec<Booth>>` to `BoothRepository` trait in `crates/domain/src/repositories.rs`
- [x] 1.2 Add `find_duplicate_groups() -> DomainResult<Vec<Vec<Booth>>>` to `BoothRepository` trait in `crates/domain/src/repositories.rs`
- [x] 1.3 Implement `find_all_by_description_and_date` using `index.get_all` on the `description_date` IDB index in `crates/storage/src/repositories/booth_repository.rs`
- [x] 1.4 Implement `find_duplicate_groups` using `find_all_by_description_and_date` on all booths grouped by key in `crates/storage/src/repositories/booth_repository.rs`
- [x] 1.5 Update `find_by_description_and_date` to be a wrapper: `find_all_by_description_and_date(...).into_iter().next()` in `booth_repository.rs`
- [x] 1.6 Update all mock `BoothRepository` impls (~7 files) to add the two new methods

## 2. Storage Foundation — Analysis Types

- [x] 2.1 Create `crates/storage/src/export/analysis.rs` with: `BoothResolution`, `BoothCandidate`, `BoothMatchKind`, `ImportPayload`, `BoothImportAnalysis`, `ImportAnalysis`
- [x] 2.2 Re-export the new types from `crates/storage/src/export/mod.rs`

## 3. Storage Foundation — Import Logic

- [x] 3.1 Add transactional boundary to `import_all` write phase (single IDB read-write transaction spanning booths, vendors, purchases) in `crates/storage/src/export/import_service.rs`
- [x] 3.2 Implement `resolve_canonical_booth` as a private helper returning `BoothResolution` (3-pass algorithm per design.md D3) in `import_service.rs`
- [x] 3.3 Rewrite `import_booth_record` to use `resolve_canonical_booth` and handle all scenario matrix cases — remap `booth_id` only when `match_kind == ByNameAndDate`
- [x] 3.4 Update `import_all` to collect `HashMap<incoming_BoothId, BoothResolution>` from the booth pass, then remap each vendor/purchase `booth_id` through the map — orphaned records → `SkippedRecord`
- [x] 3.5 Update `import_booth_backup_restoring_archived` to call `resolve_canonical_booth` and use `canonical_booth.id` (not `data.booth.id`) when calling `archive_service.restore_booth`
- [x] 3.6 Add `analyze_import` as a read-only public method on `ImportService` — pre-build vendor/purchase count HashMap in one pass for `ImportPayload::Full`

## 4. Storage Foundation — MergeService

- [x] 4.1 Create `crates/storage/src/merge_service.rs` with `MergeService` struct taking `Arc<dyn BoothRepository>`, `Arc<dyn VendorRepository>`, `Arc<dyn PurchaseRepository>`
- [x] 4.2 Implement `merge_booths(canonical_id, other_id)` — vendor find-or-save (no validation), purchase re-assignment, metadata merge (newer `updated_at`), delete non-canonical, all in one IDB transaction
- [x] 4.3 Add `merge_service: Option<Arc<MergeService>>` field to `ImportService` with a `with_merge_service` constructor

## 5. Storage Foundation — Tests

- [x] 5.1 Write unit test: case 1a — UUID match active → ConflictStrategy applied, ID preserved
- [x] 5.2 Write unit test: case 1b — UUID matches archived only → booth restored + import succeeds (requires `with_archive_service`)
- [x] 5.3 Write unit test: case 1c — UUID matches archived + active with same key → active used, archived untouched
- [x] 5.4 Write unit test: case 2a — no UUID; 1 active key match → merged under existing ID
- [x] 5.5 Write unit test: case 2b — no UUID; 2 active key matches → `Ambiguous` returned, `SkippedRecord` produced
- [x] 5.6 Write unit test: case 2c — no UUID; 0 active; 1 archived key match → archived restored, import succeeds
- [x] 5.7 Write unit test: case 2d — no UUID; 2 archived key matches → `SkippedRecord`, other booths still imported
- [x] 5.8 Write unit test: case 3c — restore path; no UUID; 1 active key match → active used, archived stays archived
- [x] 5.9 Write unit test: case 3f — restore path; no match → inserted as new active booth
- [x] 5.10 Write unit test: Skip on `ByNameAndDate` — booth metadata not updated, vendors/purchases still imported under canonical ID
- [x] 5.11 Write unit test: same-name/different-date — two booths with identical description but different dates are not merged
- [x] 5.12 Write WASM integration test: `import_all` partial failure → full IDB transaction rollback, no orphaned records

## 6. ConflictStrategy Copy Fixes (ships with Issue 1)

- [x] 6.1 Update `backup.import_strategy_label` in `en.json` and `de.json`
- [x] 6.2 Update `backup.strategy_skip` label in both locale files
- [x] 6.3 Add `backup.strategy_merge_desc`, `backup.strategy_skip_desc`, `backup.strategy_replace_desc` (all include the "Always adds new vendors and purchases" clarifier) in both locale files
- [x] 6.4 Add per-option description rendering to the strategy selector in `crates/ez-vend-ui/src/components/import_button.rs`
- [x] 6.5 Fix `backup.import_apply_ready` to remove misleading "existing records" phrasing in both locale files
- [x] 6.6 Update `docs/user-guides/MULTI_DEVICE_MERGE_GUIDE.md`: correct "When To Use Skip" section, remove "non-atomic" bullet (now fixed), remove broken refs to non-existent validation docs

## 7. Event List Card Detail (Issue 2 — independent)

- [x] 7.1 Add vendor count and `updated_at` timestamp to the booth card component in `crates/ez-vend-ui/src/pages/booth_list.rs`
- [x] 7.2 Verify sufficient contrast and layout for the additional metadata (manual check)

## 8. Duplicate Detection UI (Issue 3)

- [x] 8.1 Add `find_duplicate_groups` call on booth list page load in `booth_list.rs`
- [x] 8.2 Render amber banner when duplicate groups exist: "X events share the same name and date. Review duplicates?"
- [x] 8.3 Implement merge modal with per-group diff view: vendor counts, purchase counts, last-updated per candidate, unique/shared vendor diff, post-merge total preview, consequence statement
- [x] 8.4 Wire "Merge" button per group to call `MergeService::merge_booths` with canonical auto-selection (more purchases wins, tie-break earlier `created_at`)
- [x] 8.5 After merge, re-run `find_duplicate_groups` and update banner/modal state

## 9. Import Conflict Wizard (Issue 4)

- [x] 9.1 Replace `ParsedImportData` with `ImportPayload` (from storage crate) in `import_button.rs`
- [x] 9.2 Add `AnalysisState { Pending | Available(ImportAnalysis) | Failed(String) }` to `ImportCandidate`
- [x] 9.3 Wire `analyze_import` into `spawn_local` in `on_file_change` — collect all results, set signal once, then open modal
- [x] 9.4 Implement mode detection: any `Ambiguous` → wizard (Mode 2), otherwise simple modal (Mode 1)
- [x] 9.5 Implement simple modal (Mode 1): global strategy selector with corrected labels + archived restore confirmation checkbox
- [x] 9.6 Implement conflict wizard (Mode 2): Step 0 overview, Steps 1..N per `Ambiguous` booth (candidate cards, "Advanced" disclosure, "Don't import this event" option), `Single(archived)` step cards, final summary step
- [x] 9.7 Wire the write phase: collect all wizard decisions into `Vec<(BoothId, BoothResolution, ConflictStrategy)>` before any writes, execute single transactional pass
- [x] 9.8 Add wizard-specific locale keys (EN + DE): step 0 overview, source badges, diff format, skip label, final step strategy label, archived restore card copy, cancel toast
- [x] 9.9 Update `docs/user-guides/MULTI_DEVICE_MERGE_GUIDE.md`: remove "no manual conflict-resolution UI" from limitations, add section on full backup vs. single-booth backup for multi-device workflows
- [x] 9.10 Update `docs/technical/ADR_DEVICE_TO_DEVICE_TRANSFER.md`: update context scope note to reflect per-event reconciliation is now supported

## 10. Merge-then-Import Path (Issue 5)

- [x] 10.1 Wire the "Advanced" option in wizard `Ambiguous` step cards: amber styling, below divider, outside radio group, consequence statement showing which booth survives and which is deleted
- [x] 10.2 Implement the write phase for "Advanced" selections: call `MergeService::merge_booths`, then re-run `resolve_canonical_booth` on the incoming booth (pre-wizard candidate is stale after merge), then execute import write
- [x] 10.3 Write WASM integration test: merge-then-import with two local duplicates → duplicate removed, import succeeds, data correct
- [x] 10.4 Write WASM integration test: idempotency — run merge-then-import twice → second run finds `Single` match, import proceeds normally

## 11. Cleanup — Planning Docs

- [x] 11.1 Delete the bottom section (original Issues 1–3) from `docs/planning/CROSS_DEVICE_EVENT_MERGE.md` — the top Design Decisions section is the canonical spec; the bottom is superseded

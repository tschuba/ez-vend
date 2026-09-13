## Context

`BoothId` is a random UUID v4 with no deterministic component. `(description.trim(), date)` is the business-level unique key for active booths, enforced at the UI layer by `ensure_unique_name_and_date()` in `booth_service.rs`. That guard checks active booths only.

`ImportService` currently bypasses `BoothService` entirely and calls the repository directly, so the uniqueness guard is never reached during import. `import_booth_record` matches only by `booth.id` — the `find_by_description_and_date` index exists but is never consulted. The result: importing from a different device always creates a duplicate.

Additionally, `find_by_description_and_date` uses `index.get()` which silently returns at most one result. This makes `Ambiguous` (two active local booths with the same name/date) unreachable. Detecting and cleaning up pre-existing duplicates requires `index.get_all()`.

## Goals / Non-Goals

**Goals:**
- Fix the import duplicate bug for all cross-device scenarios
- Provide a guided UI for ambiguous cases (wizard)
- Provide a cleanup tool for pre-existing local duplicates
- Keep import and merge as separate, composable operations
- Maintain full offline capability — no server coordination
- Ensure the entire import write phase is transactional (all-or-nothing)

**Non-Goals:**
- Real-time sync or CRDT-style conflict resolution
- Server-assisted merge
- Merging events across different event names or dates
- Automatic resolution of ambiguous cases without operator confirmation

## Decisions

### D1: Identity model — no change

`BoothId` stays random UUID v4. The business key `(description.trim(), date)` is the merge signal, not the storage key. Rationale: changing the ID scheme would require a migration across all stored data and all export formats. The name+date fallback lookup achieves the same result at import time without touching the identity model.

### D2: `BoothResolution` as the resolution type

`resolve_canonical_booth` returns `BoothResolution { New, Single(BoothCandidate), Ambiguous(Vec<BoothCandidate>), UnresolvableAmbiguous }` rather than `Option<(Booth, BoothMatchKind)>`.

Rationale: `BoothCandidate` carries vendor_count, purchase_count, updated_at — all needed by both the wizard candidate cards and the dedup modal. The simpler `Option<(Booth, BoothMatchKind)>` would require a second lookup for counts at display time.

Alternative considered: separate analysis and import paths with different return types. Rejected: duplication of the lookup logic.

### D3: 3-pass lookup algorithm

`resolve_canonical_booth` uses three passes:

**Pass 1** — UUID lookup across all booths (active + archived):
- Active UUID match → `Single(ById/active)`. Done.
- Archived UUID match → record it; continue to Pass 2.
- No match → continue to Pass 2.

**Pass 2** — `find_all_active_by_description_and_date`:
- One active match → `Single(ByNameAndDate/active)`. Supersedes any archived UUID match from Pass 1.
- Two or more active matches → `Ambiguous(Vec<BoothCandidate>)`.
- No active match + had archived UUID match → `Single(ById/archived)`.
- No active match + no UUID match → Pass 3.

**Pass 3** — `find_all_archived_by_description_and_date`:
- One archived → `Single(ByNameAndDate/archived)`.
- Two or more archived → `UnresolvableAmbiguous`.
- None → `New`.

Active booths always take priority over archived ones. The UUID match is not sufficient on its own if an active booth with the same key exists.

### D4: `ConflictStrategy` semantics — booth metadata + `payout_correction` only

`ConflictStrategy` governs booth metadata fields and vendor `payout_correction` only. Vendor existence (via find-or-save) and purchase accumulation are unconditionally additive and are not affected by the strategy choice.

| Strategy | Booth metadata | Vendor `payout_correction` |
|---|---|---|
| Merge | Keep whichever side has the newer `updated_at` | Keep canonical's if set; take incoming's if canonical has none |
| Skip | Keep existing, do not update | Keep existing |
| Replace | Replace with incoming | Replace with incoming |

"Skip" does not skip the whole booth import — it skips metadata only. Vendors and purchases are always imported under the canonical ID.

Rationale: operators care about not losing purchase data. Silently discarding vendors/purchases under "Skip" would be destructive and unexpected.

### D5: `Ambiguous` interim behavior (pre-wizard)

Between Issue 1 (storage foundation) and Issue 4 (wizard) shipping, `import_all` encountering an `Ambiguous` result pushes a `SkippedRecord` with reason "ambiguous: multiple local events match this name and date — use the duplicate cleanup tool first, then re-import". Other booths in the file still import normally.

Rationale: Hard failure would block importing all other booths in the same file. Silent first-match would silently pick the wrong booth. `SkippedRecord` is visible in the import summary and gives the operator a clear action to take.

### D6: `MergeService` is separate from `ImportService`

`merge_booths` is placed in a dedicated `MergeService` at `crates/storage/src/merge_service.rs`. `ImportService` owns file-in → local. `MergeService` owns local → local consolidation. `ImportService` holds `merge_service: Option<Arc<MergeService>>` for the Issue 5 "Advanced" path. Issue 3's UI calls `MergeService` directly.

Rationale: `merge_booths` has no relationship to backup files or import format. Embedding it in `ImportService` would muddy the responsibility boundary.

### D7: `MergeService` uses direct repository calls, not `VendorService`

`VendorService.get_or_create()` validates vendor IDs against booth validation rules and omission rules. During a local-to-local merge, vendors already exist and were valid when created. Re-running validation during merge could reject vendors that are legitimate on one side if the two booths have different validation configs.

`MergeService` implements find-or-save directly: `vendor_repository.find_by_id(canonical_booth_id, vendor_id)`, save if absent. No validation.

### D8: `find_by_description_and_date` stays as a wrapper

The existing single-result method `find_by_description_and_date → Option<Booth>` is kept on the trait, implemented as `find_all_by_description_and_date(...).into_iter().next()`. Two call sites (`booth_service.rs:125` and tests) only need to know if any match exists — they don't need the full list. Zero call-site churn.

### D9: Transactional boundary

The `import_all` write phase is wrapped in a single IDB read-write transaction spanning `booths`, `vendors`, `purchases`. If any write fails the entire import rolls back. Transaction rollback correctness is verified by a WASM integration test (not a mock-based unit test, since IDB transaction semantics can't be meaningfully simulated in unit tests).

Merge-then-import (Issue 5) uses two independent transactions: merge, then import. If merge succeeds and import fails, a retry will find a `Single` match (the duplicate was already merged), making the overall flow idempotent.

### D10: `archive_service: None` + archived booth restore

`ImportService` takes `archive_service: Option<Arc<ArchiveService>>`. In production (`state.rs`), this is always `Some`. Tests that cover archived booth cases must use `ImportService::with_archive_service(...)`. There is no code-level fallback — requiring it in tests is the right constraint.

### D11: Canonical selection in `MergeService`

When merging two booths, canonical is the booth with more purchases; tie-break by earlier `created_at`. Rationale: the booth that has been used more (more purchases) is more likely to be the "primary" one in the operator's mental model.

## Risks / Trade-offs

**[Risk] Archived restore is irreversible without undo** → Mitigation: `Single(archived)` resolutions always show an explicit confirmation step (amber warning + checkbox in Mode 1, dedicated step card in wizard). The import button is disabled until acknowledged.

**[Risk] `UnresolvableAmbiguous` silently skips booths** → Mitigation: surfaced as a named `SkippedRecord` in the import summary and as a callout in the wizard's final step with the event name, cause, and manual resolution instructions.

**[Risk] Merge-then-import is partially irreversible** → Mitigation: the wizard's "Advanced" option uses amber styling, is placed below a divider, is outside the main radio group, and is never pre-selected. The consequence statement shows which booth will be deleted before confirmation.

**[Risk] `find_all_by_description_and_date` uses `index.get_all` which returns all archived + active** → Mitigation: callers filter by `is_archived` field after retrieval. Pass 2 uses `find_all_active_by_description_and_date` (filters active). Pass 3 uses `find_all_archived_by_description_and_date` (filters archived).

**[Risk] Mock impls (~7 files) need updating** → Mitigation: the new methods are additive. Existing mock behavior is unchanged. The wrapper `find_by_description_and_date` can be default-implemented on the trait as a call to `find_all_by_description_and_date` if desired, but this requires trait object safety review.

## Migration Plan

No data migration required. The change is purely additive at the storage layer:
- New index methods operate on the existing `description_date` IDB index.
- Existing booth UUIDs and data are untouched.
- The import logic change is behavioral: previously created duplicates are not automatically merged. The dedup tool (Issue 3) handles pre-existing duplicates when operators choose to run it.

Deployment: standard web app release. No rollback procedure needed — the import logic change only affects future imports, and the dedup tool is operator-triggered.

## Open Questions

None — all design decisions were resolved in the explore session on 2026-05-31.

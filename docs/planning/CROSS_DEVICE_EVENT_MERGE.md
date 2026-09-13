# Cross-Device Event Merge — Implementation Plan

> Originally extracted from [TESTING_SESSION_FIXES_2026-05-21.md](TESTING_SESSION_FIXES_2026-05-21.md). Revised and expanded following expert review (UX Architect + Software Architect) and a full design session on 2026-05-22.

**Context:** When the same event is created independently on two devices (e.g., device A and device B both create "Spring Market 2026-05-10"), they receive different booth UUIDs. Importing device B's data into device A currently creates a duplicate entry instead of merging. This plan replaces the original three-issue structure with five issues that fix the import logic, provide a guided conflict wizard for ambiguous cases, and add a cleanup tool for duplicates that already exist.

---

## Issue Structure

| # | Title | Layer | Depends on |
|---|---|---|---|
| 1 | Storage foundation: transactional boundary + multi-result lookup + `BoothResolution` types + corrected import logic + copy fix | Storage + Domain + UI | — |
| 2 | Event list card detail: vendor count + last-updated visible on card | UI | — |
| 3 | Local duplicate cleanup standalone tool: amber banner + merge modal | Domain + Storage + UI | 1 |
| 4 | Import conflict wizard: dual-modal design, richer candidate cards, archived restore confirmation | UI + Storage | 1 |
| 5 | Merge-then-import path: wizard "Advanced" disclosure live, re-resolve after merge | Storage + UI | 1 + 3 + 4 |

```
1 (storage foundation)
├──→ 3 (dedup tool)  ──→ 5 (merge-then-import)
└──→ 4 (wizard)      ──→ 5
2 (card detail) — independent, can ship anytime
```

---

## Design Decisions

### Identity model

`BoothId` is a random UUID v4 with no deterministic component. `(description.trim(), date)` is the business-level unique key for active booths, enforced at the UI layer by `ensure_unique_name_and_date()` in `booth_service.rs:117`. That guard checks active booths only — archived booths do not occupy the slot. `ImportService` currently bypasses `BoothService` entirely and calls the repository directly, which is the root gap this plan closes.

### `ConflictStrategy` semantics

`ConflictStrategy` governs **booth metadata fields** and **vendor `payout_correction`** only. Vendor existence (via `get_or_create`) and purchase accumulation are unconditionally additive and are **not** affected by the strategy choice.

| Strategy | Booth metadata | Vendor `payout_correction` |
|---|---|---|
| **Merge** | Keep whichever side has the newer `updated_at` | Keep canonical's if set; take incoming's if canonical has none |
| **Skip** | Keep existing, do not update | Keep existing |
| **Replace** | Replace with incoming | Replace with incoming |

Vendors and purchases are always imported under the canonical ID regardless of strategy. "Skip" does not skip the whole booth import — it skips metadata only.

### Lookup algorithm (2-pass)

`resolve_canonical_booth` is a private helper in `ImportService`. It must never stop at the first UUID match — active booths take priority over archived ones.

**Pass 1:** UUID lookup across all booths (active + archived).
- Active UUID match → `Single(ById/active)`. Done.
- Archived UUID match → record it; continue to Pass 2.
- No match → continue to Pass 2.

**Pass 2:** `find_all_active_by_description_and_date` (uses `index.get_all`, not `index.get`).
- One active match → `Single(ByNameAndDate/active)`. Supersedes any archived UUID match from Pass 1.
- Two or more active matches → `Ambiguous(Vec<BoothCandidate>)`. Wizard required.
- No active match + had archived UUID match → `Single(ById/archived)`. Done.
- No active match + no UUID match → Pass 3.

**Pass 3:** `find_all_archived_by_description_and_date`.
- One archived → `Single(ByNameAndDate/archived)`.
- Two or more archived → `UnresolvableAmbiguous`.
- None → `New`.

### Scenario matrix

| Case | Local state | Resolution |
|---|---|---|
| 1a | UUID matches active booth | `Single(ById/active)` → apply ConflictStrategy |
| 1b | UUID matches archived only; no active with same key | `Single(ById/archived)` → restore then import |
| 1c | UUID matches archived; active exists with same key | `Single(ByNameAndDate/active)` → use active, ignore archived |
| 2a | No UUID; 1 active with same key | `Single(ByNameAndDate/active)` → apply ConflictStrategy |
| 2b | No UUID; 2+ active with same key | `Ambiguous` → wizard required |
| 2c | No UUID; 0 active; 1 archived with same key | `Single(ByNameAndDate/archived)` → restore then import |
| 2d | No UUID; 0 active; 2+ archived with same key | `UnresolvableAmbiguous` → `SkippedRecord`, not a hard error |
| 2e | No UUID; 1 active + 1 archived, both match key | `Single(ByNameAndDate/active)` → use active, ignore archived |
| 2f | No UUID; 1 active + 2+ archived, all match key | `Single(ByNameAndDate/active)` → use active, ignore archived |
| 3b | Restore path; UUID matches active | Use active, do not restore |
| 3c | Restore path; no UUID; 1 active with key | Use active, do not restore |
| 3f | Restore path; no match at all | Insert as new active booth |

Remapping rule: remap `vendor.booth_id` / `purchase.booth_id` only when `match_kind == ByNameAndDate`.

### Core types

```rust
// crates/storage/src/export/analysis.rs

pub enum BoothResolution {
    New,
    Single(BoothCandidate),
    Ambiguous(Vec<BoothCandidate>),
    UnresolvableAmbiguous,
}

pub struct BoothCandidate {
    pub id: BoothId,
    pub description: String,
    pub match_kind: BoothMatchKind,
    pub is_archived: bool,
    pub vendor_count: usize,
    pub purchase_count: usize,
    pub updated_at: DateTime<Utc>,
}

pub enum BoothMatchKind {
    ById,
    ByNameAndDate,
}

pub enum ImportPayload {
    Full(BackupData),
    Booth(BoothBackupData),
}
```

`ImportPayload` replaces the UI-private `ParsedImportData`. It belongs in the storage crate since `BackupData` and `BoothBackupData` are already defined there.

### Archived restore confirmation

`Single(archived)` resolutions must never happen silently — the operator must explicitly acknowledge that a dormant event will become active again.

- **Mode 1 (simple modal):** If any `Single(archived)` appears in the analysis, show an amber warning section listing the event names, and a mandatory checkbox ("I understand these events will become active again"). The import button is disabled until checked. ConflictStrategy applies to booth metadata after the restore.
- **Mode 2 (wizard):** `Single(archived)` cases get a dedicated per-event step card: *"[Name] was archived on this device. Importing this file will make it active again and add any new vendors or purchases."* Options: [Restore and import] [Import as a separate new event] [Don't import this event].
- `BoothCandidate.is_archived: bool` is the flag the UI uses to detect these cases.

### Import UX: dual-modal design

**Mode 1 — simple modal** (no `Ambiguous` in analysis):
- File summary + global ConflictStrategy selector (corrected labels from Issue 1).
- If any `Single(archived)`: amber warning section + mandatory confirmation checkbox.
- Single Apply triggers the transactional write phase.

**Mode 2 — conflict wizard** (triggered automatically when any `Ambiguous` appears):
- Step 0: overview in plain language. Avoid "conflict." State how many events need attention and how many will be imported automatically.
- Steps 1..N: one step per `Ambiguous` booth. Operator must make a selection before Apply unlocks.
- Final step: summary of all decisions + `UnresolvableAmbiguous` callouts + global ConflictStrategy selector ("For all other events:") + "Import now" / "Review decisions."

**Wizard step — Ambiguous candidate card layout:**
```
○  Spring Market 2026-05-10   [on this device]
   12 vendors · 45 purchases · last updated 2 days ago
   Importing this file adds: +3 vendors, +12 purchases

○  Spring Market 2026-05-10   [on this device]
   3 vendors · 8 purchases · last updated 14 days ago
   Importing this file adds: +12 vendors, +49 purchases

○  Don't import this event

────────────────────────────────────────────
▸ Advanced: clean up duplicates and import
  Permanently merges the two events into one, then imports.
  One event will be deleted. This cannot be undone.
```

**Wizard UX rules:**
- "Advanced" option: amber/warning styling, below a divider, outside the main radio group. Not pre-selected. Operator must actively expand and choose it.
- Per-step skip label: "Don't import this event" — never "Skip."
- Cancel mid-wizard: toast "Import cancelled. No data was changed."
- `UnresolvableAmbiguous` is not a wizard step. Surface in the final summary as a callout with the event name, cause, and manual resolution instructions (archive the duplicate locally, then re-import).
- Apply button: "Import now." Back navigation: "Review decisions."

**Critical ordering rule:** All wizard decisions must be collected upfront before any writes begin. Apply triggers one transactional write pass across all decisions.

### Transactional safety

- The `import_all` write phase is wrapped in a single IDB read-write transaction spanning `booths`, `vendors`, `purchases`. If any write fails the entire import rolls back.
- `UnresolvableAmbiguous` produces a `SkippedRecord` with a reason and does not block the rest of the import.
- Merge-then-import (Issue 5) uses two independent transactions: merge, then import. If merge succeeds and import fails, a retry will find a `Single` match (the duplicate was already merged), making the overall flow idempotent.

### Vendor deduplication and `MergeService`

Vendor consolidation follows the `get_or_create` pattern: look up the vendor by `(booth_id, vendor_id_string)` and create only if absent. This means the same user-facing vendor ID is never duplicated under the same canonical booth regardless of how many devices contributed records.

`payout_correction` resolution during merge: keep canonical's value if set; take incoming's if canonical has none; for Replace strategy, use incoming's.

Purchases are always additive. No purchase is ever deleted during import or merge.

`merge_booths` is placed in a **dedicated `MergeService`** at `crates/storage/src/merge_service.rs` — not in `ImportService`. Rationale: `merge_booths` is a local-to-local operation with no relationship to backup files. `ImportService` owns file-in → local; `MergeService` owns local → local consolidation. `ImportService` calls `MergeService` internally for the Issue 5 "Advanced" path. Issue 3's UI calls `MergeService` directly.

---

## Issue 1 — Storage foundation

**What it delivers:** All domain and storage changes that Issues 3, 4, and 5 depend on, plus corrected operator-facing copy shipped at the same time as the semantic change.

### Code tasks

1. **H-4: Transactional boundary in `import_all`**
   Wrap the write phase in a single IDB read-write transaction across `booths`, `vendors`, `purchases`.
   File: `crates/storage/src/export/import_service.rs`

2. **C-1: `find_all_by_description_and_date` on `BoothRepository`**
   New async method using `index.get_all(key)` — not `index.get`. Returns `DomainResult<Vec<Booth>>`; callers filter by `is_archived`. The existing `find_by_description_and_date` (which uses `index.get` and can only return one result) makes `Ambiguous` and `UnresolvableAmbiguous` unreachable without this fix.
   Files: `crates/storage/src/repositories/booth_repository.rs`, `crates/domain/src/repositories.rs`, all mock impls (~7 files).

3. **New `crates/storage/src/export/analysis.rs`**
   Define `BoothResolution`, `BoothCandidate`, `BoothMatchKind`, `ImportPayload`. Re-export from `export/mod.rs`.

4. **`resolve_canonical_booth` in `import_service.rs`**
   Private helper implementing the 2-pass algorithm. Returns `BoothResolution`.

5. **Updated `import_booth_record`**
   Uses `resolve_canonical_booth`. Handles all scenario matrix cases. Remaps `booth_id` fields only when `match_kind == ByNameAndDate`. Skip still imports vendors and purchases.

6. **Updated `import_all`**
   Collects `HashMap<incoming_BoothId, BoothResolution>` from the booth pass. Remaps each vendor/purchase `booth_id` through the map. Orphaned records → `SkippedRecord`. `UnresolvableAmbiguous` → `SkippedRecord`, not a hard error.

7. **Updated `import_booth_backup_restoring_archived`**
   Must call `restore_booth(&canonical_booth.id, ...)` — not `&data.booth.id`. When the match is `ByNameAndDate`, `data.booth.id` does not exist in storage.

### Copy and doc tasks (ship together with the semantic change)

8. **ConflictStrategy label and description update**
   - `backup.import_strategy_label` → "How to handle matching events" / "Behandlung übereinstimmender Veranstaltungen"
   - `backup.strategy_skip` → "Keep existing event settings" / "Veranstaltungseinstellungen beibehalten"
   - Add `backup.strategy_merge_desc`, `backup.strategy_skip_desc`, `backup.strategy_replace_desc` — each must include the critical clarifier "Always adds any new vendors and purchases." / "Fügt immer neue Verkäufer und Käufe hinzu."
   - Add per-option description rendering in `import_button.rs` (currently a label-only dropdown with no descriptions).
   - Fix `backup.import_apply_ready` to remove the misleading "existing records" phrasing.
   Files: `crates/ez-vend-ui/locales/en.json`, `crates/ez-vend-ui/locales/de.json`, `crates/ez-vend-ui/src/components/import_button.rs`

9. **`docs/user-guides/MULTI_DEVICE_MERGE_GUIDE.md` corrections**
   - "When To Use Skip": update to reflect corrected semantics (skip = metadata only).
   - "Current Limits": remove the bullet stating multi-file imports are non-atomic (Issue 1 fixes this).
   - Remove broken references to `VALIDATION_WORKFLOW.md` and `SAFARI_VALIDATION_CHECKLIST.md` — neither file exists.

### Tests

- Case 1a: UUID match active → ConflictStrategy applied, ID preserved
- Case 1b: UUID matches archived only → booth restored + import succeeds
- Case 1c: UUID matches archived + active with same key → active used, archived untouched
- Case 2a: No UUID; 1 active key match → merged under existing ID
- Case 2b: No UUID; 2 active key matches → `Ambiguous` returned (wizard path deferred to Issue 4)
- Case 2c: No UUID; 0 active; 1 archived key match → archived restored, import succeeds
- Case 2d: No UUID; 2 archived key matches → `SkippedRecord` produced, other booths still imported
- Case 3c: Restore path; no UUID; 1 active key match → active used, archived stays archived
- Case 3f: Restore path; no match → inserted as new active booth
- Skip on `ByNameAndDate`: booth metadata not updated, vendors/purchases still imported under canonical ID
- `import_all` partial failure: transaction rolls back, no orphaned records

### Verification

`cargo test -p storage`, `wasm-pack test --headless --chrome crates/storage`, `wasm-pack test --headless --safari crates/storage`.

---

## Issue 2 — Event list card detail

**What it delivers:** Vendor count and last-updated timestamp visible on each booth card without opening the event. Independent of all other issues.

### Tasks

1. Add vendor count and last-updated fields to the booth list card component.
2. Ensure sufficient contrast and layout for the additional metadata.

### Notes

No domain logic changes. The card pattern established here feeds directly into the wizard candidate cards in Issue 4 and the dedup modal in Issue 3.

### Files

- `crates/ez-vend-ui/src/pages/booth_list.rs`

### Verification

`cargo test -p ez-vend-ui --lib`. Manual check: open event list, confirm counts and timestamp are visible without opening any event.

---

## Issue 3 — Local duplicate cleanup tool

**What it delivers:** Detection and guided merge of pre-existing local duplicates — booths that share the same name and date but have different UUIDs.

### Tasks

1. **`find_duplicate_groups` on `BoothRepository`**
   Returns `DomainResult<Vec<Vec<Booth>>>` — groups sharing `(description.trim(), date)`. Uses the `description_date` IDB index. Must use `find_all_by_description_and_date` from Issue 1 (the old single-result `find_by_description_and_date` cannot detect multiple matches).
   Files: `crates/domain/src/repositories.rs`, `crates/storage/src/repositories/booth_repository.rs`

2. **`MergeService` in `crates/storage/src/merge_service.rs`**
   - `merge_booths(canonical_id: &BoothId, other_id: &BoothId) -> DomainResult<()>`
   - Canonical selection: the booth with more purchases; tie-break by earlier `created_at`.
   - Vendor consolidation: `get_or_create(canonical_booth_id, vendor_id_string)` for each vendor from the other booth. Never creates a second vendor with the same user-facing `VendorId` under the same booth.
   - `payout_correction` resolution: keep canonical's if set; take other's if canonical has none.
   - Purchases: re-assign all purchases from the other booth to the canonical booth ID. Always additive; no purchases deleted.
   - Booth metadata: keep canonical ID; prefer newer `updated_at`.
   - Delete the non-canonical booth.
   - Wrapped in a single IDB transaction.

3. **Merge modal with diff view**
   Each group in the modal shows: event name, date, per-candidate vendor count + purchase count + last-updated. A VendorId-level diff: vendors unique to each side and vendors shared by both. A post-merge total preview. A consequence statement: "Spring Market 2026-04-10 (3 vendors, 8 purchases) will be permanently deleted."
   Files: `crates/ez-vend-ui/src/pages/booth_list.rs`

4. **Amber banner on booth list page**
   On load: run `find_duplicate_groups`; if non-empty, show amber banner "X events share the same name and date. Review duplicates?" Clicking opens the merge modal. After each merge, re-fetch and re-check.

### Files

- `crates/domain/src/repositories.rs`
- `crates/storage/src/repositories/booth_repository.rs`
- `crates/storage/src/merge_service.rs` (new)
- `crates/ez-vend-ui/src/pages/booth_list.rs`

### Verification

`cargo test -p storage --lib`, `wasm-pack test --headless --chrome crates/storage`, `wasm-pack test --headless --safari crates/storage`. Manual check: with pre-existing duplicates present, confirm the amber banner appears, the merge modal shows the correct diff, and after merge the list shows one event with the combined vendor and purchase count.

---

## Issue 4 — Import conflict wizard

**What it delivers:** Full dual-modal UX — simple modal for clean imports, conflict wizard for ambiguous cases. Full-backup per-event reconciliation becomes a first-class operator flow.

### Tasks

1. **Replace `ParsedImportData` with `ImportPayload`** from `ez_vend_storage::export`.

2. **`analyze_import` on `ImportService`** (read-only, no writes)
   Returns `ImportAnalysis { booths: Vec<BoothImportAnalysis> }`. Each `BoothImportAnalysis` carries: incoming id/description/date, `BoothResolution`, vendor_count, purchase_count, incoming diff (+vendors, +purchases relative to the local candidate).
   For full backup: pre-build `HashMap<BoothId, (vendor_count, purchase_count)>` in a single pass over the data before iterating booths (avoids O(n×m) scan).
   Analysis failure: retry once automatically. If still failing, surface a blocking error with: the raw IDB error string, DevTools console path for the current browser, common causes, and safe recovery steps in order. No "proceed anyway" option.

3. **Mode detection:** Any `Ambiguous` in the analysis → Mode 2 (wizard). Otherwise → Mode 1 (simple modal).

4. **Simple modal (Mode 1)**
   - Global ConflictStrategy selector with corrected labels and descriptions from Issue 1.
   - If any `Single(archived)`: amber warning section listing event names + mandatory checkbox ("I understand these events will become active again"). Import button disabled until checked.
   - Single Apply triggers the transactional write phase.

5. **Conflict wizard (Mode 2)**
   - `AnalysisState { Pending | Available(ImportAnalysis) | Failed(String) }` on `ImportCandidate`.
   - Step 0: plain-language overview. State how many events need attention and how many will be handled automatically. Avoid the word "conflict."
   - Steps 1..N: one step per `Ambiguous` booth. Candidate card layout per UX decisions above.
   - `Single(archived)` cases: dedicated step card with restore-consequence copy and three options.
   - Final step: all decisions listed + `UnresolvableAmbiguous` callouts + global ConflictStrategy ("For all other events:") + "Import now" / "Review decisions."
   - Apply is disabled until every ambiguous step has a selection.
   - Cancel mid-wizard: toast "Import cancelled. No data was changed."

6. **Write phase:** Collect all wizard decisions into `Vec<(BoothId, BoothResolution, ConflictStrategy)>` before any writes. Single transactional pass.

7. **Wizard-specific locale keys** (EN + DE)
   - Step 0 overview text
   - Candidate card source badges: "on this device" / "from the file"
   - Incoming diff preview format: "+N vendors, +M purchases"
   - Per-step skip option: "Don't import this event"
   - Global strategy label in final step: "For all other events:"
   - Archived restore card body copy
   - Mode 1 archived checkbox label: "I understand these events will become active again"
   - `UnresolvableAmbiguous` callout with event name + manual resolution instructions
   - Cancel toast: "Import cancelled. No data was changed."

8. **Doc updates**
   - `docs/user-guides/MULTI_DEVICE_MERGE_GUIDE.md`: update "What Merge Does Not Try To Do" (remove "no manual conflict-resolution UI"); add a new section explaining when to use full backup vs. single-booth backup for multi-device workflows.
   - `docs/technical/ADR_DEVICE_TO_DEVICE_TRANSFER.md`: update the Context "transfer scope" note — full-backup import with per-event reconciliation is now supported. The transport mechanism (file sharing) is unchanged.

### Files

- `crates/ez-vend-ui/src/components/import_button.rs`
- `crates/ez-vend-ui/locales/en.json`
- `crates/ez-vend-ui/locales/de.json`
- `crates/storage/src/export/import_service.rs` (add `analyze_import`)
- `crates/storage/src/export/analysis.rs` (extend `BoothImportAnalysis`)
- `docs/user-guides/MULTI_DEVICE_MERGE_GUIDE.md`
- `docs/technical/ADR_DEVICE_TO_DEVICE_TRANSFER.md`

### Verification

`cargo test -p ez-vend-ui --lib`, `wasm-pack test --headless --chrome crates/ez-vend-ui`, `wasm-pack test --headless --safari crates/ez-vend-ui`. Wizard flow has no automated browser-test coverage — use a manual smoke test with the existing test cases in `docs/validation/`.

---

## Issue 5 — Merge-then-import integration path

**What it delivers:** The wizard's "Advanced" disclosure becomes functional. The operator can merge local duplicates and import in a single guarded flow.

**Prerequisites:** Issues 1, 3, and 4 must all be complete.

### Tasks

1. **Wire the "Advanced" option in the wizard step** (case 2b, `Ambiguous`)
   - Amber styling, below a divider, outside the main radio group.
   - Consequence statement showing which event survives (the one with more purchases) and which will be deleted.
   - Not pre-selected. Operator must actively expand and choose it.

2. **Execution sequence in the write phase**
   a. Collect all wizard decisions including any "Advanced" selections.
   b. For each "Advanced" selection: call `MergeService::merge_booths(canonical_id, other_id)`.
   c. Re-run `resolve_canonical_booth` on the incoming booth (H-1: the pre-wizard `BoothCandidate` is stale after merge).
   d. Import write phase for all booths in a single transaction.

3. **Idempotency verification**
   If merge succeeds and import fails: on retry, `resolve_canonical_booth` will find a `Single` match (the duplicate is gone). Import proceeds normally. Confirm this holds in a test.

### Files

- `crates/ez-vend-ui/src/components/import_button.rs`
- `crates/storage/src/export/import_service.rs`
- `crates/storage/src/merge_service.rs`

### Verification

`wasm-pack test --headless --chrome crates/storage`. Manual smoke test: trigger the "Advanced" merge-then-import path with two local duplicates and an incoming file; confirm the duplicate is removed, the import succeeds, and the data is correct. Run the same scenario twice to verify idempotency.

---

## Future discussion

**Browser storage safety warnings** — independent issue candidate:
1. Safari: historically clears IDB data under storage pressure (improved in Safari 16.4 but not fully reliable). Show a warning when the app is opened in Safari.
2. Private/incognito mode: IDB data is cleared when the session closes. Show a warning when the app is opened in a private window.
Both are UI-only safety features with no domain logic change.

---

## Files affected

| File | Issues |
|---|---|
| `crates/storage/src/export/analysis.rs` (new) | 1, 4 |
| `crates/storage/src/export/import_service.rs` | 1, 4, 5 |
| `crates/storage/src/export/mod.rs` | 1 |
| `crates/storage/src/repositories/booth_repository.rs` | 1, 3 |
| `crates/storage/src/merge_service.rs` (new) | 3, 5 |
| `crates/domain/src/repositories.rs` | 1, 3 |
| Mock repository impls (~7 files) | 1 |
| `crates/storage/tests/import_service_tests.rs` | 1 |
| `crates/ez-vend-ui/src/pages/booth_list.rs` | 2, 3 |
| `crates/ez-vend-ui/src/components/import_button.rs` | 1 (copy), 4, 5 |
| `crates/ez-vend-ui/locales/en.json` | 1 (copy fix), 4 (wizard copy) |
| `crates/ez-vend-ui/locales/de.json` | 1 (copy fix), 4 (wizard copy) |
| `docs/user-guides/MULTI_DEVICE_MERGE_GUIDE.md` | 1 (Skip/limits/broken refs), 4 (full-backup section) |
| `docs/technical/ADR_DEVICE_TO_DEVICE_TRANSFER.md` | 4 (scope note) |


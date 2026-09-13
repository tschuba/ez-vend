## ADDED Requirements

### Requirement: MergeService consolidates two local booths into one
`MergeService` SHALL expose `merge_booths(canonical_id, other_id)` which permanently consolidates the `other` booth into the `canonical` booth in a single IDB transaction. The non-canonical booth is deleted after all its data is transferred.

#### Scenario: Vendor re-assignment uses find-or-save, no validation
- **WHEN** `merge_booths` is called
- **THEN** for each vendor from the non-canonical booth, `MergeService` checks if a vendor with the same `vendor_id` already exists under the canonical booth — if not, it saves it with the canonical booth ID — if yes, it is already present and no duplicate is created — no booth validation rules are applied

#### Scenario: payout_correction resolution during vendor merge
- **WHEN** both canonical and non-canonical vendors have a `payout_correction` value
- **THEN** the canonical vendor's `payout_correction` is kept

#### Scenario: payout_correction taken from incoming when canonical has none
- **WHEN** the canonical vendor has no `payout_correction` and the non-canonical vendor has one
- **THEN** the non-canonical vendor's `payout_correction` is used

#### Scenario: payout_correction_note follows the same rule as payout_correction
- **WHEN** `merge_booths` resolves vendor fields
- **THEN** `payout_correction_note` follows the same rule as `payout_correction`: keep canonical's if set; take the non-canonical's if canonical has none

#### Scenario: All purchases are re-assigned to canonical booth
- **WHEN** `merge_booths` is called
- **THEN** all purchases that belonged to the non-canonical booth are saved with the canonical booth's ID — no purchases are deleted

#### Scenario: Booth metadata keeps canonical ID, prefers newer updated_at
- **WHEN** `merge_booths` is called
- **THEN** the canonical booth's UUID is preserved, and the `updated_at` timestamp used is whichever side is newer

#### Scenario: Non-canonical booth is deleted after merge
- **WHEN** `merge_booths` completes successfully
- **THEN** the non-canonical booth no longer exists in storage

#### Scenario: Merge is atomic — failure rolls back
- **WHEN** any write inside `merge_booths` fails
- **THEN** the IDB transaction is rolled back and both booths remain in their pre-merge state

### Requirement: Canonical selection prefers booth with more purchases
When `MergeService` auto-selects the canonical booth (from the dedup UI), it SHALL choose the booth with the greater purchase count. Tie-break: earlier `created_at`.

#### Scenario: More purchases wins
- **WHEN** two booths share the same name and date and the UI triggers auto-canonical selection
- **THEN** the booth with more purchases is designated canonical

#### Scenario: Tie broken by created_at
- **WHEN** two booths have equal purchase counts
- **THEN** the booth with the earlier `created_at` is designated canonical

### Requirement: MergeService is separate from ImportService
`MergeService` SHALL reside in `crates/storage/src/merge_service.rs` and be independent of `ImportService`. `ImportService` holds `merge_service: Option<Arc<MergeService>>` for the merge-then-import path. `MergeService` has no dependency on import format or backup files.

#### Scenario: Dedup UI calls MergeService directly
- **WHEN** the operator confirms a merge from the duplicate cleanup modal
- **THEN** the UI calls `MergeService::merge_booths` directly — not through `ImportService`

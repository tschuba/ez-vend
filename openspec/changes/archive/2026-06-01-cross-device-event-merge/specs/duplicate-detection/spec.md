## ADDED Requirements

### Requirement: find_duplicate_groups detects active booths with same name+date
`BoothRepository` SHALL expose `find_duplicate_groups() -> DomainResult<Vec<Vec<Booth>>>` which returns groups of active booths sharing `(description.trim(), date)` but having different UUIDs. Uses the existing `description_date` IDB index via `find_all_by_description_and_date`.

#### Scenario: Two active booths with same key form a group
- **WHEN** two active booths have identical description (after trim) and date but different UUIDs
- **THEN** `find_duplicate_groups` returns a `Vec` containing one group with both booths

#### Scenario: Booths with different dates are not grouped
- **WHEN** two booths have the same description but different dates
- **THEN** they are not returned as a duplicate group

#### Scenario: Archived booths are excluded from duplicate detection
- **WHEN** one active and one archived booth share the same name+date key
- **THEN** `find_duplicate_groups` does not include them as a duplicate group

### Requirement: Amber banner appears on booth list when local duplicates exist
On booth list page load, `find_duplicate_groups` SHALL be called. If any groups are found, an amber warning banner SHALL be shown: "X events share the same name and date. Review duplicates?" Clicking the banner opens the merge modal.

#### Scenario: Banner appears when duplicates exist
- **WHEN** the booth list page loads and `find_duplicate_groups` returns non-empty groups
- **THEN** an amber banner is visible on the page

#### Scenario: Banner is absent when no duplicates exist
- **WHEN** `find_duplicate_groups` returns an empty list
- **THEN** no banner is shown

#### Scenario: Banner re-evaluates after merge
- **WHEN** the operator completes a merge in the modal
- **THEN** `find_duplicate_groups` is called again and the banner is dismissed if no duplicates remain

### Requirement: Merge modal shows diff view per duplicate group
The merge modal SHALL show for each group: event name, date, per-candidate vendor count + purchase count + last-updated, a vendor-level diff (vendors unique to each side + shared vendors), a post-merge total preview, and a consequence statement naming which booth will be deleted.

#### Scenario: Consequence statement names the booth to be deleted
- **WHEN** the merge modal renders a group
- **THEN** it shows e.g. "Spring Market 2026-04-10 (3 vendors, 8 purchases) will be permanently deleted." before the confirm button

#### Scenario: Vendor diff shows unique and shared vendors
- **WHEN** the merge modal renders a group
- **THEN** it shows which vendors exist only on the left candidate, only on the right, and on both

### Requirement: Event list booth card shows vendor count and last-updated
Each booth card on the event list page SHALL display the vendor count and the `updated_at` timestamp without requiring the operator to open the event.

#### Scenario: Card shows vendor count
- **WHEN** the event list page renders
- **THEN** each booth card displays the number of vendors for that booth

#### Scenario: Card shows last-updated timestamp
- **WHEN** the event list page renders
- **THEN** each booth card displays the `updated_at` timestamp with sufficient contrast

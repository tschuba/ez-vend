## ADDED Requirements

### Requirement: Mode detection — Ambiguous triggers wizard, otherwise simple modal
`ImportService` analysis results SHALL determine the import modal mode. Any `Ambiguous` resolution in the analysis triggers Mode 2 (conflict wizard). Otherwise Mode 1 (simple modal) is used.

#### Scenario: Clean import uses simple modal
- **WHEN** all booths resolve to `New`, `Single`, or `UnresolvableAmbiguous` (no `Ambiguous`)
- **THEN** the simple modal (Mode 1) is shown

#### Scenario: Any Ambiguous triggers wizard
- **WHEN** at least one booth resolves to `Ambiguous`
- **THEN** the conflict wizard (Mode 2) is shown

### Requirement: Simple modal (Mode 1) includes ConflictStrategy selector with corrected labels
The simple modal SHALL show a global `ConflictStrategy` selector with corrected labels and per-option descriptions. If any `Single(archived)` is present, an amber warning section with a mandatory confirmation checkbox is shown. The import button is disabled until the checkbox is checked.

#### Scenario: Corrected ConflictStrategy labels
- **WHEN** the simple modal renders the strategy selector
- **THEN** the label reads "How to handle matching events" and the descriptions include "Always adds any new vendors and purchases" for all three options

#### Scenario: Archived restore confirmation checkbox
- **WHEN** the analysis includes at least one `Single(archived)` resolution
- **THEN** an amber warning section lists the event names and shows a mandatory checkbox — the import button is disabled until the checkbox is checked

### Requirement: Conflict wizard (Mode 2) collects all decisions before any writes
The conflict wizard SHALL collect all operator decisions upfront before triggering the write phase. No writes occur until the operator reaches the final step and clicks "Import now."

#### Scenario: Apply is disabled until all ambiguous steps have a selection
- **WHEN** the operator is on the final wizard step
- **THEN** the "Import now" button is disabled if any ambiguous step has no selection

#### Scenario: Cancel mid-wizard produces no writes
- **WHEN** the operator cancels the wizard at any step
- **THEN** no data is written and a toast shows "Import cancelled. No data was changed."

### Requirement: Wizard step structure and navigation
The conflict wizard SHALL have: Step 0 (plain-language overview), Steps 1..N (one step per `Ambiguous` booth, and one per `Single(archived)` booth), Final step (summary + global strategy + "Import now"). Back navigation is "Review decisions."

#### Scenario: Step 0 avoids the word "conflict"
- **WHEN** Step 0 renders
- **THEN** it states how many events need attention and how many will be handled automatically, without using the word "conflict"

#### Scenario: Single(archived) gets a dedicated step card
- **WHEN** a booth resolves to `Single(archived)`
- **THEN** it appears as a dedicated wizard step with copy: "[Name] was archived on this device. Importing this file will make it active again and add any new vendors or purchases." — with options [Restore and import] [Import as a separate new event] [Don't import this event]

#### Scenario: UnresolvableAmbiguous is not a wizard step
- **WHEN** the analysis contains `UnresolvableAmbiguous` resolutions
- **THEN** they are not given wizard steps — they appear as callouts on the final summary step with the event name, cause, and manual resolution instructions

### Requirement: Ambiguous wizard step candidate card layout
Each `Ambiguous` wizard step SHALL show one radio option per candidate with: event name, source badge ("on this device"), vendor count, purchase count, last updated. An incoming diff row showing "+N vendors, +M purchases". An "Advanced" disclosure below a divider outside the main radio group (amber styling, never pre-selected, requires active expansion).

#### Scenario: Advanced option requires active expansion
- **WHEN** an Ambiguous step is rendered
- **THEN** the "Advanced" option is collapsed by default and is not pre-selected — the operator must actively expand and choose it

#### Scenario: Per-step skip label is "Don't import this event"
- **WHEN** an Ambiguous step is rendered
- **THEN** the skip option label is "Don't import this event" — not "Skip"

### Requirement: Write phase executes one transactional pass over all wizard decisions
After the operator clicks "Import now", `ImportService` SHALL execute a single transactional write pass across all decisions collected during the wizard. No partial writes occur.

#### Scenario: All decisions applied in one transaction
- **WHEN** the operator confirms the wizard
- **THEN** all booth, vendor, and purchase writes happen in a single IDB transaction — failure rolls back all writes

### Requirement: ConflictStrategy labels and descriptions are corrected
Locale keys SHALL be updated:
- `backup.import_strategy_label` → "How to handle matching events" / "Behandlung übereinstimmender Veranstaltungen"
- `backup.strategy_skip` → "Keep existing event settings" / "Veranstaltungseinstellungen beibehalten"
- Per-option descriptions SHALL include "Always adds any new vendors and purchases." / "Fügt immer neue Verkäufer und Käufe hinzu."
- `backup.import_apply_ready` SHALL be corrected to remove misleading "existing records" phrasing.

#### Scenario: Strategy descriptions are visible in the import modal
- **WHEN** the import modal renders the strategy selector
- **THEN** each option shows a description below its label, including the clarifier about always importing vendors and purchases

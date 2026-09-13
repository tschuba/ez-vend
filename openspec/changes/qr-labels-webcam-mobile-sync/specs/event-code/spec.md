## ADDED Requirements

### Requirement: event_code field on Booth
The `Booth` model SHALL have a mandatory `event_code: String` field. This is the cross-device event identifier shared across all registers and mobile devices for the same event. `booth.id` (UUID) remains device-local for per-register accounting. `event_code` is stored in every sync payload and used to route mobile purchases to the correct booth.

#### Scenario: New booth created with event_code
- **WHEN** a cashier creates a new event
- **THEN** the resulting `Booth` record contains a non-empty `event_code`

#### Scenario: Existing booth after migration
- **WHEN** the Kassen-App is opened after migration `0003_booth_event_code.sql` has run
- **THEN** every existing booth has a non-empty `event_code` (backfilled by the migration)

---

### Requirement: event_code derivation algorithm
The Kassen-App SHALL derive an initial `event_code` suggestion from the event name and start date using this algorithm:
1. Transliterate umlauts (ä→A, ö→O, ü→U, ß→S); strip non-alphanumeric characters
2. Skip words whose normalised form is ≤ 3 chars (language-agnostic; catches articles and prepositions)
3. Take the first letter (uppercase) of each remaining significant word, up to 4 initials
4. If only one significant word remains, take its first 2 chars
5. Fallback: if no significant words remain, use first 2 chars of the first word
6. Append `-{MMYY}` where MMYY is derived from the event's start date (not the current system date); month is zero-padded

The derived code is displayed and editable before the organiser confirms.

#### Scenario: Typical event name
- **WHEN** the cashier enters event name "Flohmarkt Mai 2026" with date 2026-05-15
- **THEN** the suggested event_code is "FM-0526"

#### Scenario: Name with umlauts
- **WHEN** the cashier enters "Großer Herbstmarkt" with date 2026-10-10
- **THEN** the suggested event_code is "GH-1026"

#### Scenario: Name where most words are short
- **WHEN** the cashier enters "Markt am Rathaus" with date 2026-05-20
- **THEN** "am" is skipped (≤ 3 chars after normalisation) and the code is "MR-0526"

#### Scenario: Fallback — all words too short
- **WHEN** the cashier enters "Auf dem" with date 2026-06-01
- **THEN** the fallback applies and the code is "AU-0626" (first 2 chars of "Auf")

---

### Requirement: Local event_code collision avoidance
When the Kassen-App generates an `event_code` suggestion, it MUST check all existing `booth.event_code` values. If a match exists, it SHALL append `-2` (or increment the suffix until unique) and display a notice to the organiser.

#### Scenario: Derived code already exists
- **WHEN** the derived code "FM-0526" already exists in the booth list
- **THEN** the suggestion shows "FM-0526-2" and a notice: "Dieser Code existiert bereits — Suffix wurde angehängt."

---

### Requirement: event_code lock after onboarding QR is shown
The `event_code` field SHALL be locked — editing disabled — once the mobile onboarding QR has been first rendered on screen for this booth. The lock timestamp is stored as `onboarding_qr_shown_at`. The lock MUST apply symmetrically on the leading register (which never receives a sync).

Before the lock triggers, any attempt to edit the event_code MUST show a warning: *"Das Ändern des Event-Codes unterbricht alle mobilen Geräte, die dieses Event bereits eingerichtet haben."*

After the lock, the edit field is disabled and the warning is replaced by a notice: *"Event-Code gesperrt — QR-Code wurde bereits angezeigt."*

#### Scenario: Editing event_code before onboarding QR is shown
- **WHEN** the cashier tries to edit the event_code before the onboarding QR has been rendered
- **THEN** the field is editable and a warning message is shown

#### Scenario: event_code locked after onboarding QR rendered
- **WHEN** the cashier renders the onboarding QR for the first time
- **THEN** `onboarding_qr_shown_at` is set and the event_code edit field becomes disabled

#### Scenario: Locked event_code on page reload
- **WHEN** the page is reloaded after `onboarding_qr_shown_at` is set
- **THEN** the event_code edit field is still disabled

---

### Requirement: event_code sharing via onboarding QR
The Kassen-App event creation confirmation screen SHALL display the `event_code` prominently with the instruction: *"Teile diesen Code mit allen Kassen, bevor die Veranstaltung beginnt."* The mobile onboarding QR (`ez-booth://onboard?e={event_code}&n={name}`) SHALL be displayed on the same screen. Scanning this QR on another Kassen-App MUST pre-fill that register's `event_code` field in its event creation form.

#### Scenario: Second register scans onboarding QR

- **WHEN** the cashier on a second register scans the onboarding QR from the first register
- **THEN** the event_code field in the second register's event creation form is pre-filled with the scanned code

#### Scenario: QR scan would overwrite existing non-empty event_code

- **WHEN** the cashier scans an onboarding QR and the event_code field already contains a different non-empty value
- **THEN** a confirmation dialog is shown: "Der gescannte Code (FM-0526) weicht vom eingetragenen Code (GH-1026) ab. Code übernehmen?" before overwriting

---

### Requirement: event_code mismatch recovery
The design MUST document a recovery procedure for the case where a mismatch is discovered after the event has begun:
1. Identify the mismatched register by comparing event_code on each register's event screen
2. On the mismatched register: re-enter event creation and type the correct code manually
3. Purchases already taken on the mismatched register are unaffected (booth-UUID-local)
4. Mobile sync to the corrected register resumes once codes match
5. Any purchases synced to the mismatched register before correction must be re-exported and re-imported to the correct register

#### Scenario: Mismatched register corrected mid-event

- **WHEN** a cashier updates the event_code on a mismatched register to match the leading register
- **THEN** subsequent mobile sync imports from helpers are routed correctly to that register

#### Scenario: Override lock for mismatch recovery on a locked register

- **WHEN** a cashier on a mismatched register attempts mismatch recovery but `onboarding_qr_shown_at` is already set (field is locked)
- **THEN** an "Override lock" action is available with a strong confirmation: "Achtung: Dieser Register hat bereits mobile Geräte eingebunden. Das Ändern des Codes bricht die Synchronisation für alle bisher eingebundenen Geräte. Trotzdem ändern?"
- **THEN** on confirmation the field becomes editable for one change, the new code is saved, and the lock re-engages immediately

---

### Requirement: event_code migration for existing booths
Migration `0003_booth_event_code.sql` SHALL add the `event_code` column (nullable), backfill all existing rows using the derivation algorithm applied to `(description, date)`, and then set the column `NOT NULL`. After migration, if a booth's `event_code` was generated by backfill and never confirmed, the Kassen-App SHALL show a one-time prompt on first open: *"Bitte überprüfe und bestätige deinen Event-Code, bevor du mobile Geräte einrichtest."*

#### Scenario: Existing booth opened after migration
- **WHEN** a cashier opens an existing event after the migration has run
- **THEN** the booth has a non-empty event_code and the one-time confirmation prompt is shown

#### Scenario: Prompt dismissed persists
- **WHEN** the cashier confirms the event_code in the prompt
- **THEN** the prompt is not shown again for that booth

# Deferred — Phase 5

This capability is deferred. It has no technical risk blockers (no spike required) but depends on Phases 1–3 being stable first.

---

## ADDED Requirements

### Requirement: Central event_code generation
The Organiser-App SHALL derive `event_code` client-side using the same name+date algorithm as the Kassen-App. The derived code SHALL be editable before the organiser saves the event.

#### Scenario: Organiser creates event
- **WHEN** the organiser enters "Flohmarkt Mai 2026" with date 2026-05-15
- **THEN** the suggested event_code is "FM-0526" and it is editable before saving

---

### Requirement: Vendor list management
The Organiser-App SHALL allow the organiser to create, edit, and archive vendors for an event before it starts.

#### Scenario: Adding a vendor
- **WHEN** the organiser adds vendor with ID "42" and name "Hans Müller"
- **THEN** the vendor appears in the vendor list associated with the event

---

### Requirement: Export for Kassen-App import
The Organiser-App SHALL export event data (vendors, event_code, event metadata) as a `.json` file importable by the Kassen-App. The exported `event_code` MUST be preserved on import so label links and mobile onboarding remain consistent.

#### Scenario: Exporting and importing organiser data
- **WHEN** the organiser exports event data and the cashier imports the file into the Kassen-App
- **THEN** the Kassen-App's booth is configured with the organiser's event_code and all vendors are present

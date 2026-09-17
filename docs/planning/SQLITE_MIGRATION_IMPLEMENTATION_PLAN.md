---
title: SQLite Migration Implementation Plan
nav_order: 5
parent: Planning
---

# SQLite Migration From Original ez-booth - Implementation Plan

Prepared on 2026-04-06 to document the agreed implementation plan for migrating data from the original Java-based `ez-booth` SQLite database to `ez-vend` IndexedDB storage.

## Goal

Add a one-time migration wizard in `ez-vend` that allows operators to upload their original `booth.db` file and migrate booths, vendors, and purchases with strict validation.

The migration must preserve money semantics exactly, fail safely on invalid data, and clearly warn operators that the migration replaces existing browser data.

## Decisions Already Made

- migration input: a single uploaded `booth.db` SQLite file
- migration scope: full database migration only
- conflict strategy: replace all existing `ez-vend` data after explicit confirmation
- safety behavior: automatically download a JSON backup before applying the migration
- validation mode: strict validation with fail-fast behavior
- closed booth fields from the legacy database: ignore
- vendor names: do not support additional CSV or mapping import in this version
- defaults for new `ez-vend` settings fields: conservative defaults
- UI placement: new `Migration` tab in the Settings page

## Current Status

As of 2026-04-06, migration from the original `ez-booth` database is still planned work rather than a user-facing capability.

The current product already includes:

- IndexedDB persistence for booths, vendors, and purchases
- JSON export and import services in `crates/storage/src/export/`
- conflict-aware import behavior with `Skip`, `Replace`, and `Merge`
- storage diagnostics and settings UI in `crates/ez-vend-ui/src/pages/settings.rs`

This plan reuses the existing import/export foundation where practical and adds a dedicated SQLite parsing and transformation path for the legacy source database.

## Legacy Database Analysis

The legacy SQLite file used for analysis was:

- `/Users/thomas/Documents/tschuba/ez-booth/booth.db`

Observed record counts in that database:

- 4 booths
- 106 vendors
- 45 purchases

Observed SQLite schema:

```sql
CREATE TABLE booths (
    booth_id varchar(255) not null,
    closed boolean,
    closed_on timestamp,
    date date not null,
    description varchar(255) not null,
    fees_rounding_step numeric(38,2) not null,
    participation_fee numeric(38,2) not null,
    sales_fee numeric(38,2) not null,
    primary key (booth_id)
);

CREATE TABLE vendors (
    booth_id varchar(255) not null,
    vendor_id varchar(255) not null,
    primary key (booth_id, vendor_id)
);

CREATE TABLE purchases (
    booth_id varchar(255) not null,
    purchase_id varchar(255) not null,
    purchased_on timestamp not null,
    value numeric(38,2) not null,
    primary key (booth_id, purchase_id)
);

CREATE TABLE purchase_items (
    item_id varchar(255) not null,
    booth_id varchar(255) not null,
    purchase_id varchar(255) not null,
    price numeric(38,2) not null,
    purchased_on timestamp not null,
    vendor_id varchar(255) not null,
    primary key (item_id, booth_id, purchase_id)
);

CREATE TABLE event_publication (
    id blob not null,
    completion_date timestamp,
    event_type varchar(255),
    listener_id varchar(255),
    publication_date timestamp,
    serialized_event varchar(255),
    primary key (id)
);
```

Important observations:

- booth dates are stored as epoch milliseconds
- purchase timestamps are stored as epoch milliseconds
- booth, purchase, and item identifiers use UUID-looking strings
- vendor identifiers are stored as strings and may be numeric values
- legacy `closed` and `closed_on` are not represented in the current domain model
- `event_publication` is infrastructure data and should not be migrated

## Schema Mapping

### Booth Mapping

| Legacy SQLite | ez-vend | Notes |
| --- | --- | --- |
| `booth_id` | `Booth.id` | Parse as UUID-backed `BoothId` |
| `description` | `Booth.description` | Copy directly |
| `date` | `Booth.date` | Convert epoch milliseconds to `NaiveDate` |
| `participation_fee` | `Booth.fees.participation_fee` | Preserve as `Decimal` |
| `sales_fee` | `Booth.fees.sales_fee_percent` | Preserve as `Decimal` |
| `fees_rounding_step` | `Booth.fees.rounding_step` | Preserve as `Decimal` |
| `closed` | not mapped | Intentionally ignored |
| `closed_on` | not mapped | Intentionally ignored |

New `Booth` fields that do not exist in the legacy schema will use conservative defaults:

- `vendor_id_validation`: default value
- `vendor_id_omission_rules`: empty rules
- `keyboard_config`: default value
- `amount_stepping`: `None`
- `created_at`: migration timestamp
- `updated_at`: migration timestamp

### Vendor Mapping

| Legacy SQLite | ez-vend | Notes |
| --- | --- | --- |
| `booth_id` | `Vendor.booth_id` | Parse as `BoothId` |
| `vendor_id` | `Vendor.vendor_id` | Wrap as `VendorId` |

New `Vendor` fields that do not exist in the legacy schema will use:

- `name`: `None`
- `created_at`: migration timestamp

### Purchase Mapping

The legacy schema stores purchases across `purchases` and `purchase_items`, while `ez-vend` embeds items inside each `Purchase`.

| Legacy SQLite | ez-vend | Notes |
| --- | --- | --- |
| `purchases.purchase_id` | `Purchase.id` | Parse as `PurchaseId` |
| `purchases.booth_id` | `Purchase.booth_id` | Parse as `BoothId` |
| `purchases.purchased_on` | `Purchase.timestamp` | Convert epoch milliseconds to `DateTime<Utc>` |
| `purchase_items.item_id` | `PurchaseItem.id` | Parse as `ItemId` |
| `purchase_items.vendor_id` | `PurchaseItem.vendor_id` | Wrap as `VendorId` |
| `purchase_items.price` | `PurchaseItem.amount` | Preserve as `Decimal` |
| `purchases.value` | validation-only | Must equal the sum of item amounts |

New `Purchase` fields that do not exist in the legacy schema will use:

- `note`: `None`

## Validation Requirements

Money and fee calculations are business-critical, so migration validation must be strict.

### Required validation checks

- every migrated booth ID parses successfully as a UUID
- every migrated purchase ID parses successfully as a UUID
- every migrated item ID parses successfully as a UUID
- every migrated booth date converts cleanly from epoch milliseconds to `NaiveDate`
- every migrated purchase timestamp converts cleanly from epoch milliseconds to `DateTime<Utc>`
- every fee value remains valid under existing domain validation rules
- every purchase contains at least one item
- every purchase item amount is positive and valid for the current domain model
- every vendor belongs to a migrated booth
- every purchase references a migrated booth
- every purchase item references a migrated vendor
- the sum of purchase item amounts equals `purchases.value` exactly

### Failure behavior

- migration must not partially import invalid data
- validation errors must stop the migration before data replacement
- the operator must receive a clear summary of what failed
- if validation succeeds, the operator must still explicitly confirm replacement of current browser data

## Technical Approach

### Recommended parser approach

Implement SQLite parsing in Rust using a WASM-compatible SQLite path so that the uploaded legacy file can be read directly in the browser.

The recommended path is:

- add a WASM-compatible SQLite dependency in `crates/storage`
- read uploaded file bytes in the browser
- extract legacy rows through a dedicated migration parser
- transform those rows into the existing domain models
- apply the migration using the existing storage repositories and import patterns

### Why this approach

- it keeps migration logic close to existing Rust domain and storage code
- it allows strict validation before any write is applied
- it avoids introducing a separate external conversion tool for operators
- it keeps the migration workflow inside the application where backup and confirmation can be enforced

### Explicit non-goals for this slice

- incremental merge from a legacy SQLite file
- support for importing multiple SQLite files
- migration of `event_publication`
- adding legacy closed-booth behavior to the current product model
- extra migration inputs such as vendor-name CSV files

## Proposed Module Layout

New storage-side modules:

```text
crates/storage/src/migration/
  mod.rs
  error.rs
  sqlite_parser.rs
  schema_mapper.rs
  validator.rs
```

Planned UI integration points:

```text
crates/ez-vend-ui/src/components/migration_wizard.rs
crates/ez-vend-ui/src/pages/settings.rs
```

## Migration Flow

### Operator flow

1. Open Settings and switch to the new `Migration` tab.
2. Read the warning that migration replaces current browser data.
3. Review the always-visible file location help with the typical `booth.db` path for the current platform.
4. Copy the default path if needed, then upload a single `booth.db` file manually.
5. The app parses and validates the SQLite data.
6. If validation succeeds, the app automatically downloads a JSON backup of current local data.
7. The app shows a summary of booths, vendors, and purchases to be migrated.
8. The operator confirms replacement.
9. The migration writes transformed data into IndexedDB.
10. The app shows a migration success summary.

### Internal flow

1. Read SQLite bytes from the uploaded file.
2. Parse legacy rows from `booths`, `vendors`, `purchases`, and `purchase_items`.
3. Ignore `event_publication`.
4. Transform legacy records into `Booth`, `Vendor`, and `Purchase` models.
5. Run strict validation across the transformed data set.
6. Export current IndexedDB data to JSON for backup download.
7. Apply migration with replacement semantics.
8. Return a detailed import summary to the UI.

## UI Plan

The migration UI should live in the existing Settings page as a third tab next to `General` and `Diagnostics`.

The first implementation should use a simple step-based wizard:

1. introduction and file upload
2. validation progress and result
3. replacement warning and confirmation
4. success or failure summary

The UI should emphasize:

- replacement warning before any destructive action
- automatic backup behavior
- counts of booths, vendors, and purchases found in the legacy file
- validation failures with actionable messages
- file-location guidance because browsers cannot preselect the legacy database folder
- migration discoverability from the empty booth list for first-run users with legacy data

## Implementation Phases

### Phase 1: Foundation

- add the SQLite dependency needed for browser-compatible parsing
- create the migration module structure in `crates/storage/src/migration/`
- implement a parser that can read the required legacy tables
- add focused tests for parsing and row extraction

### Phase 2: Transformation

- implement booth mapping from legacy rows to current domain models
- implement vendor mapping from legacy rows to current domain models
- implement purchase and purchase-item grouping into current `Purchase` objects
- add targeted transformation tests

### Phase 3: Validation

- implement money-total validation for every purchase
- implement referential integrity checks across booths, vendors, and purchases
- reuse existing domain validation for fees and item amounts
- add regression tests for strict failure scenarios

### Phase 4: UI Integration

- add a `Migration` tab to `SettingsPage`
- implement the migration wizard component
- wire uploaded file handling to the storage migration service
- trigger automatic JSON backup download before replacement
- add bilingual EN/DE strings for migration-related copy
- show platform-specific `booth.db` location help with clipboard copy support
- add empty-state navigation from the booth list to `Settings?tab=migration`

### Phase 5: Validation And Manual Verification

- validate migration against the real `booth.db` sample used during planning
- run the smallest relevant automated test suite
- verify the workflow in Chrome
- verify the workflow in Safari
- compare migrated totals and counts against the source database

### Phase 6: Documentation

- add an operator-facing migration guide
- update `README.md` to reflect migration availability when implementation ships
- update `docs/COMPARISON_TO_ORIGINAL.md`
- update roadmap and validation references after the feature is complete

## Testing Strategy

### Automated tests

- unit tests for timestamp conversion from epoch milliseconds
- unit tests for UUID parsing failures
- unit tests for booth, vendor, and purchase transformation
- validation tests for money-total mismatches
- validation tests for missing booth and vendor references
- browser-compatible tests for parsing and migration application where feasible

### Manual validation

Manual validation is required because this is an operator-facing data migration flow.

Minimum manual validation should include:

- upload and validate the real `booth.db`
- confirm booth count matches the legacy source
- confirm vendor count matches the legacy source
- confirm purchase count matches the legacy source
- compare at least one booth's totals and reporting outputs against the legacy data
- verify the backup file downloads before replacement
- verify the file-location help and clipboard copy action
- verify the workflow in both Chrome and Safari

## Risks And Mitigations

### Money mismatch risk

Risk:

- legacy purchase totals may not match the sum of stored items

Mitigation:

- fail migration when totals differ
- surface the purchase identifier and mismatch details clearly

### Browser compatibility risk

Risk:

- SQLite parsing in WASM may behave differently across browsers

Mitigation:

- keep parser coverage focused on the required legacy schema
- explicitly validate in Chrome and Safari before shipping

### Data replacement risk

Risk:

- operators may unintentionally replace existing browser data

Mitigation:

- show clear warnings in the wizard
- require explicit confirmation
- automatically download a JSON backup before replacement

## Success Criteria

The implementation should be considered complete when:

- operators can upload a legacy `booth.db` file in the app
- the app validates and migrates booths, vendors, and purchases successfully
- money totals are preserved exactly
- invalid legacy data stops the migration safely
- a backup is downloaded automatically before replacement
- the workflow is validated in Chrome and Safari
- documentation is updated to reflect the new capability

## Estimated Delivery Shape

This work is expected to span multiple focused implementation slices:

- storage parser and row extraction
- transformation and validation
- settings UI and workflow wiring
- documentation and validation updates

The exact PR breakdown can be decided once implementation starts, but the work should remain scoped so each slice can be reviewed safely.

## Follow-Up Documentation To Update When Feature Ships

- `README.md`
- `docs/COMPARISON_TO_ORIGINAL.md`
- `docs/planning/ROADMAP.md`
- `docs/user-guides/` migration guide
- relevant validation results in `docs/validation/`

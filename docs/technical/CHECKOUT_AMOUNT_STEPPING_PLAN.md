---
title: Checkout Amount Stepping Plan
nav_order: 4
parent: Technical Docs
---

# Checkout Amount Stepping Plan

Prepared on 2026-04-01 to capture the agreed implementation plan for configurable checkout amount stepping.

## Goal

Add an optional per-booth checkout rule that restricts entered amounts to a configured step.

Examples:

- step `0.5` allows `3.5`, `4.0`, and `102`
- step `0.5` rejects `1.1`, `8.7`, and `154.3`

When no step is configured, checkout keeps the current behavior.

## Decisions Already Made

- configuration scope: per booth
- default: `None` / disabled
- input modes: enforce in both regular and right-to-left entry modes
- input behavior: allow typing values that do not match the step, but show a validation error
- error style: specific message that includes the configured step
- configuration UI: extend the existing booth edit modal
- persistence: reuse existing localStorage and IndexedDB flows
- quick amount buttons: no additional work planned because they are not currently used in checkout UI

## Scope

This plan covers:

1. adding optional `amount_stepping` to booth configuration
2. validating checkout amounts against the configured step
3. exposing the setting in the booth create/edit form
4. adding EN/DE copy for the new validation and form field
5. testing the behavior in both amount input modes

This plan does not cover:

- auto-correcting invalid amounts
- changing amount keyboard layout based on step
- reintroducing or redesigning quick amount buttons
- changing fee rounding behavior

## Implementation Shape

### 1. Domain model

Update `crates/domain/src/models/booth.rs` to add:

```rust
pub amount_stepping: Option<Decimal>
```

Rules:

- `None` means stepping validation is disabled
- `Some(step)` must be strictly positive
- existing booths without this field should continue to work with `None`

Also update booth construction and update helpers so the new field is carried through consistently.

### 2. Validation

Add shared validation in `crates/domain/src/validation.rs`:

- validate that the configured step is positive
- validate that an entered amount is an exact multiple of the configured step

Add new validation errors in `crates/domain/src/error_code.rs` for:

- invalid booth step configuration
- entered amount not matching the configured step

The validation should operate on `Decimal` values only.

### 3. Checkout UI

Update `crates/ez-vend-ui/src/pages/checkout.rs` so inline amount validation also considers the selected booth's `amount_stepping`.

Behavior:

- regular input mode: keep allowing typing, show inline error when the amount does not align to the step
- right-to-left input mode: same rule and same error behavior
- add-item and submit flows must still validate so invalid amounts cannot be committed through checkout

The message should explain the required increment, for example:

- `Amount must be in increments of EUR 0.50`

### 4. Booth edit modal

Update `crates/ez-vend-ui/src/components/booth_form.rs` to add an optional amount stepping field.

Recommended UX:

- label: amount stepping
- placeholder example: `0.50` or `1.00`
- help text that explains leaving it empty disables the rule
- validation that the configured step is a positive decimal number

The field should round-trip through:

- `BoothFormData::default_with_locale`
- `BoothFormData::from_booth`
- form submit parsing back into `Booth`

### 5. Localization

Update:

- `crates/ez-vend-ui/locales/en.json`
- `crates/ez-vend-ui/locales/de.json`

Add copy for:

- booth form label, placeholder, and help text
- invalid configuration error
- checkout validation error that includes the configured step

### 6. Persistence

No schema migration is expected.

The new booth field should flow through existing local persistence:

- IndexedDB booth storage
- localStorage state that references booth data where applicable

Implementation work should verify serialization and deserialization paths already used for `Booth`.

## Validation Plan

Automated:

- run the project test suite after implementation

Manual:

1. booth form accepts empty step and valid positive step values
2. booth form rejects zero, negative, and malformed step values
3. checkout accepts valid values for step `0.5`
4. checkout rejects invalid values for step `0.5`
5. checkout accepts only whole-number values for step `1.0`
6. both amount input modes show the same stepping behavior
7. EN and DE copy render correctly

## Suggested Delivery Order

1. add domain field and validation helpers
2. add error codes and translations
3. wire the setting into the booth form
4. wire stepping validation into checkout
5. run automated checks and a short manual checkout pass

## Branch

Working branch for later pickup:

- `feature/checkout-amount-stepping`

---
description: "Use when writing or editing domain business logic, fee calculations, payout math, validation, or money-related types in crates/domain."
applyTo: "crates/domain/**/*.rs"
---
# Domain Crate Rules

- Money, fees, and payouts: always `rust_decimal::Decimal` — never `f64` or `f32`.
- Return `DomainResult<T>` (not `Result<T, String>`) for fallible operations.
- Validate at the domain boundary; do not rely on the UI layer to catch invalid state.
- Trim and normalize input before persisting or validating.
- Fee calculation and rounding are regression-sensitive — any change requires a targeted unit test proving the before/after behavior.
- Amount stepping, vendor ID rules, and omission rules are equally regression-sensitive.
- Use typed enums (`ConflictStrategy`, `DraftLoadOutcome`) over free-form strings for mode/state.
- Use `thiserror` enums for structured errors; convert infrastructure errors at crate boundaries.
- Avoid `unwrap()` and `expect()` in non-test paths.

See [`crates/domain`](../../crates/domain/) for the canonical fee logic in `services/report_service.rs`.

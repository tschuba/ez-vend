---
name: money-math
description: "Use when changing fee calculations, payout math, amount stepping, rounding logic, or any Decimal arithmetic in ez-vend-rs. Enforces regression-test-first workflow for money-critical changes."
---

# Money Math Changes

Use for any change to fee calculation, rounding, payout math, or amount stepping in `crates/domain`.

## When to Use

- Modifying `services/report_service.rs` (fee/payout logic)
- Changing amount stepping or rounding configuration
- Updating vendor ID omission rules that affect totals
- Fixing decimal precision or serialization edge cases

## Procedure

### 1. Read the existing logic
Read the relevant section of `crates/domain/src/services/report_service.rs` and its tests before touching anything.

### 2. Write a failing regression test first
Add a `#[test]` in the same module that captures the current (broken) behavior and the expected (correct) behavior:
```rust
#[test]
fn test_fee_calculation_with_rounding() {
    // Arrange: booth with 15% fee, round to 0.50
    // Act: vendor grosses 100.00
    // Assert: net payout is 85.00 (not 85.50 or 84.50)
}
```
Run it: `cargo test -p domain` — it must fail before your fix.

### 3. Implement the fix
- Always use `rust_decimal::Decimal` — never `f64`.
- Preserve existing rounding semantics unless the task explicitly changes them.
- Keep the change minimal; do not refactor surrounding code.

### 4. Verify the test passes
```bash
cargo test -p domain
```

### 5. Check for ripple effects
```bash
./run-tests.sh --chrome
```
Amount stepping, reporting totals, and export checksums can all be affected by fee changes.

### 6. Note validation in PR summary
State: what changed, what test proves it, whether Safari validation is still needed.

## Key Files

- `crates/domain/src/services/report_service.rs` — fee and payout logic
- `crates/domain/src/models/` — `BoothFeeConfig`, `Decimal` usage
- `docs/user-guides/FEE_CALCULATION.md` — business rules reference

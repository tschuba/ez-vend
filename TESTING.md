# EZ Booth Testing Guide

This document describes the testing strategy and how to run tests for the EZ Booth project.

## Test Structure

The project uses multiple levels of testing:

### 1. Unit Tests
**Location**: `crates/ez-vend-ui/src/components/booth_form_tests.rs`

Tests individual functions and logic in isolation, particularly:
- `BoothFormData` validation
- Form data conversion (`to_booth()`, `from_booth()`, `update_booth()`)
- Error handling for invalid inputs
- Edge cases (empty descriptions, negative fees, out-of-range values)

**Run with**:
```bash
cargo test -p ez-vend-ui --lib
```

**Coverage**:
- ✅ Valid booth creation from form data
- ✅ Invalid date formats
- ✅ Invalid numeric values  
- ✅ Negative fees
- ✅ Out-of-range percentages (> 100%)
- ✅ Empty descriptions
- ✅ Conversion from Booth to form data
- ✅ Updating existing booths
- ✅ Update validation failures

### 2. Integration Tests  
**Location**: `crates/storage/tests/booth_repository_tests.rs`

Tests the IndexedDB storage layer with real browser database operations:
- Save and retrieve booths
- Find by ID
- Find all booths
- Update existing booths
- Delete booths
- Concurrent operations
- Error handling

**Run with** (requires browser):
```bash
wasm-pack test --headless --chrome crates/storage
# or
wasm-pack test --headless --firefox crates/storage
```

**Coverage**:
- ✅ Save and find booth by ID
- ✅ Find all booths (empty and populated)
- ✅ Update booth data
- ✅ Delete booth
- ✅ Delete non-existent booth (idempotent)
- ✅ Concurrent save operations

### 3. Purchase Storage Diagnostics Tests
**Location**: `crates/storage/tests/purchase_repository_tests.rs`

Tests IndexedDB purchase handling with corruption diagnostics:
- Detect corrupted purchase records
- Verify valid purchases still load when corrupted data exists

**Run with**:
```bash
wasm-pack test --headless --chrome crates/storage
```

**Coverage**:
- ✅ Corrupted purchase record detection
- ✅ Valid purchase preservation during corrupted loads

### 4. Browser Checkout/Report Tests
**Location**: `crates/ez-vend-ui/tests/checkout_integration_tests.rs`

Tests checkout-critical persistence and report integrity in a real browser environment:
- Single-item purchase persistence
- Multi-vendor purchase persistence
- Running totals correctness
- Domain validation rejection for invalid amounts
- Fee/report consistency across storage-backed services

**Run with**:
```bash
wasm-pack test --headless --chrome crates/ez-vend-ui
```

**Coverage**:
- ✅ Single-item checkout persistence
- ✅ Multi-vendor checkout persistence
- ✅ Running totals correctness
- ✅ Invalid amount rejection
- ✅ Empty purchase rejection
- ✅ Fee/report consistency

### 5. Safari-Specific Browser Resilience Tests
**Location**: `crates/storage/tests/safari_specific_tests.rs`

Tests Safari-sensitive persistence and recovery behavior proactively:
- Large checkout draft payload handling in localStorage
- Rapid IndexedDB round trips under repeated save/read sequences
- Decimal serialization edge cases for currency amounts
- Corrupted draft detection and safe replacement

**Run with**:
```bash
./run-tests.sh --safari
```

**Coverage**:
- ✅ Large draft payload write/read behavior
- ✅ Rapid IndexedDB round-trip consistency
- ✅ Decimal precision preservation for currency edge cases
- ✅ Corrupted draft overwrite readiness

## Running Tests

### Prerequisites

1. **Rust toolchain**: Install from https://rustup.rs/
2. **wasm-pack**: Install for browser tests
   ```bash
   cargo install wasm-pack
   ```
3. **Safari browser tests**: enable Safari WebDriver once per Mac
   ```bash
   sudo safaridriver --enable
   ```

   This is checked automatically by `scripts/ensure-safaridriver.sh` when Safari tests are requested.

### Run All Tests

```bash
# Unit tests only (default)
./run-tests.sh

# Unit tests + Chrome browser tests
./run-tests.sh --chrome

# Unit tests + Safari browser tests
./run-tests.sh --safari

# Full automated suite
./run-tests.sh --chrome --safari
```

### Run Specific Test Suites

```bash
# Only UI unit tests
cargo test -p ez-vend-ui --lib

# Only storage integration tests in Chrome
wasm-pack test --headless --chrome crates/storage

# Only storage integration tests in Safari
wasm-pack test --headless --safari crates/storage

# Only checkout/report browser tests in Chrome
wasm-pack test --headless --chrome crates/ez-vend-ui

# Only checkout/report browser tests in Safari
wasm-pack test --headless --safari crates/ez-vend-ui

# Run a specific test
cargo test -p ez-vend-ui test_to_booth_valid_data
```

### Watch Mode (Development)

```bash
# Rerun tests on file changes
cargo watch -x "test -p ez-vend-ui --lib"
```

## Test Configuration

### Cargo.toml Configuration

Unit tests are configured in each crate's `Cargo.toml`:

```toml
[dev-dependencies]
# For regular unit tests
# (uses existing workspace dependencies)

# For browser tests
wasm-bindgen-test = "0.3"
```

### Browser Test Configuration

Browser tests use `wasm-bindgen-test` which automatically:
- Compiles Rust to WebAssembly
- Starts a headless browser
- Runs tests in the browser environment
- Reports results back to the terminal

Helper scripts wrap the browser setup details:
- `scripts/ensure-chromedriver.sh` aligns ChromeDriver with the installed Chrome version
- `scripts/ensure-safaridriver.sh` verifies Safari WebDriver is available and reminds testers how to enable it on a new machine

## Writing Tests

### Unit Tests Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_input() {
        let form = BoothFormData {
            description: "Test".to_string(),
            date: "2026-03-25".to_string(),
            // ...
        };
        
        let result = form.to_booth();
        assert!(result.is_ok());
    }
}
```

### Integration Tests Example

```rust
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_save_booth() {
    let db = create_test_db().await;
    let repo = IndexedDbBoothRepository::new(Arc::new(db));
    
    let booth = create_test_booth("Test");
    assert!(repo.save(&booth).await.is_ok());
}
```

## Continuous Integration

### GitHub Actions (Future)

Add to `.github/workflows/test.yml`:

```yaml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace --lib

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install wasm-pack
      - run: wasm-pack test --headless --chrome crates/storage
```

## Test Coverage

### Automated Test Inventory
- ✅ Unit tests: 102
- ✅ Browser integration tests per browser run: 20
  - Booth repository tests: 6
  - Purchase diagnostics and compound-key tests: 3
  - Safari-specific resilience tests: 4
  - Checkout/report browser tests: 7

### Reusable Manual Validation Assets
- `docs/validation/Rauchtest_DE.html` - Safari-specific and general pre-session smoke test (10-point checklist, includes checkout/report verification)
- `docs/validation/Testsitzungs_Leitfaden_DE.html` - structured multi-device test session guide (replaces UAT_Ausfuehrungsplan)
- `node scripts/check-validation-consistency.mjs` - verifies that manual validation scripts, Team C fixture data, and AB/AC/BC validation tables stay numerically consistent

To generate coverage reports:

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --workspace --out Html
# Open tarpaulin-report.html
```

## Manual Testing Checklist

While automated tests cover logic and storage, manual browser testing is still needed for:

### Archive And Restore Workflow
- [ ] Archive action is only available from the booth list for active booths
- [ ] Archive wizard requires a successful booth export before confirmation is enabled
- [ ] Large booth warning appears when archive thresholds are exceeded
- [ ] Confirming archive moves the booth into the archived section and removes it from active selection
- [ ] Archived booth remains viewable but cannot be selected for checkout or vendor operations
- [ ] Archived booth summary report shows vendor totals without transaction detail rows
- [ ] Importing the exported booth backup restores the archived booth to active use
- [ ] Diagnostics export includes archive audit events for the archive and restore actions
- [ ] Archived booth print view displays correct non-zero monetary values
- [ ] Archived booth modal title shows translated archived summary text
- [ ] Archiving a booth in one tab clears the selection in other tabs
- [ ] Archived booths cannot be deleted from the booth list UI
- [ ] Archive history is visible in Settings diagnostics
- [ ] Archived booth cards show a localized archived timestamp
- [ ] Booth selector explains when all booths are archived
- [ ] Archived section visibility persists across reloads

### Create Operation
- [ ] Valid form submission creates booth
- [ ] Empty fields show validation errors
- [ ] Invalid values show errors
- [ ] Booth appears immediately in list
- [ ] Success toast shows

### Read Operation  
- [ ] Booths display in grid
- [ ] All booth data shows correctly
- [ ] Page refresh preserves data
- [ ] Multiple booths display correctly

### Update Operation
- [ ] Edit button opens modal
- [ ] Modal pre-fills with existing data
- [ ] Valid changes save successfully
- [ ] Invalid changes show errors
- [ ] Changes appear immediately
- [ ] Cancel button discards changes

### Delete Operation
- [ ] Delete button opens confirmation
- [ ] Confirmation shows booth description
- [ ] Confirm deletes and removes from list immediately
- [ ] Cancel keeps booth
- [ ] Success toast shows

### UI/UX
- [ ] Language toggle works (EN/DE)
- [ ] Responsive layout (mobile/desktop)
- [ ] No console errors
- [ ] Loading states show appropriately

### Safari Checkout And Report Validation
- [ ] Start app with `trunk serve` from `crates/ez-vend-app`
- [ ] Open app in Safari
- [ ] Create booth with participation fee `10.00`, sales fee `15`, rounding `0.50`
- [ ] Add checkouts for vendors `1`, `2`, `3` with amounts `100.00`, `518.11`, `75.25`
- [ ] Verify booth summary `total_booth_revenue` equals sum of all vendor fees
- [ ] Verify printed booth summary matches on-screen values
- [ ] Verify each vendor report shows correct gross sales, fees, and payout
- [ ] Enter invalid amount `10.123` and verify checkout rejects it
- [ ] Enter negative amount and verify checkout rejects it
- [ ] Trigger draft persistence by entering checkout items, then refresh
- [ ] Verify draft recovery behaves as expected
- [ ] Export and archive the booth, then verify the archived summary print view still renders correctly in Safari
- [ ] Re-import the booth backup and verify the booth becomes selectable again
- [ ] Corrupt one purchase record and verify a visible warning explains that only recoverable data is shown
- [ ] Confirm there are no uncaught console errors during checkout/report flows

Use `docs/validation/Rauchtest_DE.html` for the detailed Safari-specific version of this flow, including the draft-persistence reload check and import/export verification.

### Reusable User Acceptance Testing
- [ ] Open `docs/validation/Testsitzungs_Leitfaden_DE.html` for the full multi-device UAT session guide
- [ ] Use `docs/validation/TC_Multidevice_Merge_Overview_DE.html` for a structured overview of test phases and document links
- [ ] Use device-specific tracking sheets (`TC_Multidevice_Merge_Device_A_DE.html`, `_B_DE.html`) for per-device execution
- [ ] Record findings in `docs/validation/Beobachtungsvorlage_DE.html`
- [ ] Validate post-merge results against `docs/validation/Validierungstabelle_DE.html` (session leader only)

Recommended module focus by current maintenance goal:
- checkout and report safety checks: Modules `C` and `D`
- persistence and recovery checks: Modules `E` and `F`
- onboarding walkthrough: run all modules end-to-end
- post-cleanup smoke validation: Modules `A`, `C`, `D`, and `E`

## Debugging Tests

### Enable Debug Logging

```rust
// In test
web_sys::console::log_1(&format!("Debug: {:?}", value).into());
```

### Run Tests with Backtrace

```bash
RUST_BACKTRACE=1 cargo test
```

### Run Tests in Visible Browser

```bash
# Remove --headless to see browser
wasm-pack test --chrome crates/storage
wasm-pack test --chrome crates/ez-vend-ui
```

## Known Issues

1. **Safari WebDriver requires enablement**: each Mac must run `sudo safaridriver --enable` once before Safari tests can run
2. **IndexedDB cleanup**: Each test uses a unique database name to avoid conflicts
3. **Async timing**: Some tests may be flaky due to async operations

## Best Practices

1. **Isolation**: Each test should be independent
2. **Cleanup**: Integration tests use unique DB names per test
3. **Descriptive names**: Test names should describe what they test
4. **AAA pattern**: Arrange, Act, Assert structure
5. **Fast feedback**: Unit tests should be fast (<1s each)
6. **Deterministic**: Tests should always produce the same result

## Resources

- [wasm-bindgen-test docs](https://rustwasm.github.io/wasm-bindgen/wasm-bindgen-test/index.html)
- [Leptos testing](https://leptos-rs.github.io/leptos/testing.html)
- [Rust book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)

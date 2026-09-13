## Why

Users store valuable event data (booths, vendors, purchases) exclusively in the browser's local storage, which can be wiped silently by the OS, browser, or a simple cache clear — with no recovery path. Today, only the footer briefly signals this risk, which is easy to overlook. A prominent, mandatory onboarding dialog ensures every user understands the data-loss risk before they accumulate irreplaceable data, and a periodic re-reminder prevents long-absent users from forgetting it.

## What Changes

- Introduce a full-screen blocking modal dialog that appears on first app launch and then recurs after 30 days of inactivity since last dismissal.
- The dialog presents live storage quota benchmarks (used / available via the Storage API), so users see the concrete capacity constraints.
- A dedicated Safari/iOS variant of the dialog warns about that browser's 7-day eviction policy with stronger language, mirroring the logic already in the footer component.
- Dismissal records a timestamp to localStorage; the dialog does not reappear until that threshold is crossed again.
- No existing functionality is removed; the footer status indicator remains unchanged.

## Capabilities

### New Capabilities

- `storage-risk-warning-dialog`: A mandatory, dismissible modal dialog that educates users about browser-storage data-loss risk on first use and after a 30-day silence period. Displays live storage quota benchmarks and an elevated Safari/iOS variant warning.

### Modified Capabilities

<!-- No existing requirement-level specs are being changed. The storage footer behavior is unchanged. -->

## Impact

- **New component**: `StorageRiskWarningDialog` in `crates/ez-vend-ui/src/components/`
- **New localStorage key**: `ez-vend-storage-warning-dismissed-at` (ISO timestamp, written on dismissal)
- **Storage API**: First use of `navigator.storage.estimate()` for quota benchmarks — needs WASM/web-sys wiring
- **App shell / root component**: Must render and gate the dialog on startup; blocks the rest of the UI until dismissed
- **Existing code to reuse**:
  - `is_safari()` / `detect_browser()` in `storage_warning.rs`
  - `StorageDiagnostics` in `crates/storage/src/diagnostics.rs`
  - Existing modal/dialog patterns (if any) in the UI crate

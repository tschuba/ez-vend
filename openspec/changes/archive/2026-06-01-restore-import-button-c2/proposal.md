## Why

PR #157 replaced the `StorageWarningInfo` banner with a smart footer status bar but did not carry the `ImportButton` over to any new location — leaving users with no way to import backups from the UI. This fix restores the button using the simplest viable approach (C2: path-check in the shared page header) while a proper context-signal infrastructure (C1) is planned separately.

## What Changes

- `ImportButton` is added to the shared page header (`lib.rs` `AppViewHeader`) when the current route is `/booths`
- The header flex row gains `justify-between` so the h1 and the Import button sit at opposite ends
- `LuUpload` icon is added to the icon exports
- `pub use import_button::*;` re-export is added to `components/mod.rs` (missing, causing the component to be private)
- `ImportButton` renders with `LuUpload` icon + responsive label (`hidden sm:inline`) and a ghost+bordered visual style

## Capabilities

### New Capabilities

- `import-button-header-placement`: The ImportButton is accessible from the Events page header, always visible regardless of backup status, with correct visual weight (ghost/outlined, subordinate to the teal Create Event FAB).

### Modified Capabilities

_(none — no existing spec-level requirements change)_

## Impact

- `crates/ez-vend-ui/src/lib.rs` — `AppViewHeader` component
- `crates/ez-vend-ui/src/components/mod.rs` — re-export fix
- `crates/ez-vend-ui/src/components/import_button.rs` — icon + responsive label
- `crates/ez-vend-ui/src/components/icons.rs` — add `LuUpload`
- No API, storage, or locale changes required
- No breaking changes

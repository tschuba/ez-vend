## Why

`restore-import-button-c2` placed the ImportButton in the shared `AppViewHeader` using a path-check — a pragmatic fix that works for one page but couples booth-specific logic into the global layout. As more pages grow page-level actions (e.g., a "Create Vendor" shortcut on `/vendors`, an export action on `/checkout`), the path-check accumulates and becomes a maintenance problem. This change replaces the path-check with a proper context-signal infrastructure: pages inject their own header actions via a Leptos context; the shared header renders whatever is provided.

## What Changes

- New `HeaderActionsContext` type wraps a `WriteSignal<Option<View>>`, provided at the App root
- `AppViewHeader` reads from `HeaderActionsContext` and renders the signal value (if `Some`) right-aligned in the header row
- The path-check for ImportButton is removed from `AppViewHeader`
- The booth list page (`booth_list.rs`) provides an ImportButton view into `HeaderActionsContext` on mount and clears it on unmount
- All other pages that need header actions can follow the same pattern without touching `lib.rs`

## Capabilities

### New Capabilities

- `page-header-actions`: Any page can inject a `View` into the shared page header's right slot via `HeaderActionsContext`. The header renders it right-aligned opposite the h1. Pages are responsible for providing and clearing the signal.

### Modified Capabilities

- `import-button-header-placement`: The ImportButton remains in the Events page header, but is now injected via `HeaderActionsContext` from `booth_list.rs` instead of conditionally rendered via path-check in `lib.rs`.

## Impact

- `crates/ez-vend-ui/src/lib.rs` — remove path-check, add context provision + signal render
- `crates/ez-vend-ui/src/pages/booth_list.rs` — inject ImportButton into `HeaderActionsContext` on mount, clear on unmount
- No API, storage, locale, or breaking changes
- Supersedes the C2 path-check introduced in `restore-import-button-c2`

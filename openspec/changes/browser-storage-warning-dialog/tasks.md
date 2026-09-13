# Tasks: browser-storage-warning-dialog

## 1. Platform & Browser Detection

- [ ] 1.1 Implement iOS platform detection helper: `is_ios() -> bool` using `/iPhone|iPad|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1)` — covers all iOS/iPadOS browsers including iPads that report `MacIntel`; `> 1` avoids false positives on touchscreen Windows laptops and touchscreen MacBook Pro
- [ ] 1.2 If `detect_browser()` / `is_safari()` are private to `storage_warning.rs`, extract them to a shared module (e.g. `crates/ez-vend-ui/src/browser.rs`); update existing callers and import in the new dialog

## 2. localStorage Utilities

- [ ] 2.1 Add `get_storage_warning_dismissed_at() -> Option<DateTime<Utc>>` helper that reads `ez-vend-storage-warning-dismissed-at` from localStorage, returning `None` if absent or unreadable (try/catch wrapper)
- [ ] 2.2 Add `set_storage_warning_dismissed_at(now: DateTime<Utc>)` helper that writes the ISO 8601 timestamp, silently ignoring write failures
- [ ] 2.3 Add `should_show_storage_warning(is_ios: bool) -> bool` that returns `true` when the key is absent OR elapsed time >= 90 days (non-iOS) / >= 7 days (iOS)

## 3. Storage API Calls

- [ ] 3.1 Wire `navigator.storage.persist()` via `web-sys` — call on dialog open; return a `PersistResult` enum (`Granted`, `Denied`, `Unsupported`); on iOS always return `Denied` regardless of API result (WebKit no-op)
- [ ] 3.2 Wire `navigator.storage.estimate()` via `web-sys` — return `Option<(used: u64, quota: u64)>`; treat 0/null/undefined `usage` or `quota` as `None`
- [ ] 3.3 Add human-readable byte formatting utility (e.g. `format_bytes(n: u64) -> String` → "4.2 MB") for quota display

## 4. Dialog Component

- [ ] 4.1 Create `StorageRiskWarningDialog` Dioxus component in `crates/ez-vend-ui/src/components/storage_risk_warning_dialog.rs`
- [ ] 4.2 Implement full-screen backdrop overlay; apply `inert` attribute to the app root element when the dialog is open (use `web-sys` to set/remove the attribute on mount/unmount)
- [ ] 4.3 Implement ARIA semantics on the dialog container: `role="dialog"`, `aria-modal="true"`, `aria-labelledby` (headline id), `aria-describedby` (accessible description id including "This notice must be acknowledged before continuing"); add `tabindex="-1"` and move focus to the container on open
- [ ] 4.4 Implement the headline as a semantic `<h2>` element; it must carry the full risk message in one sentence (e.g. "Your data exists only in this browser — if it is lost, it cannot be recovered")
- [ ] 4.5 Implement the summary tier: headline + at most 2 one-line bullets; platform-specific first bullet:
  - **iOS**: "Safari and all iOS browsers may delete your data if you don't open the app for 7 days" (red styling)
  - **macOS Safari**: "Safari may delete your data if you don't open the app for 7 days" (amber styling)
  - **Other browsers**: no platform-specific bullet; standard risk bullets only
- [ ] 4.6 Implement the disclosure toggle as a `<button>` with `aria-expanded` (false/true) and `aria-controls` pointing to the details panel id; use a contextual label — "How browser storage works" on non-iOS, "Why this matters on iPhone & iPad" on iOS
- [ ] 4.7 Implement the collapsible details panel: quota benchmarks section (with `aria-live="polite"` and `aria-atomic="true"` live region, pre-existing in DOM before data loads), fuller explanation of browser storage mechanics, and platform-specific mitigation copy:
  - **iOS**: plain-language eviction explanation + mitigation note ("actively launch the app at least once a week, export regularly — this is a system-level restriction that applies to all browsers on iPhone and iPad")
  - **macOS Safari**: browser recommendation ("any non-Safari browser — Chrome, Firefox, Brave, Edge, etc. — removes this restriction"); add code comment `// Verify against WebKit ITP release notes annually`
  - **Other browsers**: standard storage risk explanation only
- [ ] 4.8 Populate the `aria-live` quota region only when the details panel is expanded AND `storage.estimate()` has resolved with non-null, non-zero values; skip the section entirely otherwise
- [ ] 4.9 On mount, fire `storage.persist()` async; update a local signal with `PersistResult`; if `Granted` on non-iOS, add a moderating note to the details tier ("Your browser has granted this app protected storage, reducing eviction risk")
- [ ] 4.10 On mount, fire `storage.estimate()` async; store result in local signal; render in quota live region when details expand
- [ ] 4.11 Implement confirmation button with a label that restates the risk (e.g. "Got it — my data stays on this device"); on click: call `set_storage_warning_dismissed_at(now)` and signal parent to close; ensure no other dismiss path exists (no ESC handler, no backdrop click handler, no X button)
- [ ] 4.12 Implement focus trap: Tab cycles between disclosure toggle and confirmation button only; Shift+Tab reverses; use `keydown` interception on the dialog container

## 5. Root Integration

- [ ] 5.1 In the app shell / root component, call `should_show_storage_warning(is_ios())` on startup and store result in a reactive signal
- [ ] 5.2 Conditionally render `<StorageRiskWarningDialog>` at the root level when the signal is `true`
- [ ] 5.3 Wire the dialog's on-dismiss callback to set the signal to `false`

## 6. Verification

- [ ] 6.1 Clear localStorage, launch app on desktop Chrome — dialog appears, blocks interaction, shows 1 headline + 2 bullets, details collapsed, no iOS bullet
- [ ] 6.2 Dismiss dialog — timestamp written; relaunch within 90 days — dialog does not appear
- [ ] 6.3 Set `ez-vend-storage-warning-dismissed-at` to 91 days ago — dialog reappears on launch
- [ ] 6.4 Test on iOS device or iOS simulator — iOS-specific first bullet visible, 7-day threshold applies, details show plain-language eviction text + "no iOS browser can avoid this" mitigation note; NO browser-switch recommendation shown
- [ ] 6.5 Test on macOS Safari — first summary bullet warns "Safari may delete your data if you don't open the app for 7 days" (amber); details show "any non-Safari browser (Chrome, Firefox, Brave, Edge, etc.) removes this restriction"; no iOS-specific text shown
- [ ] 6.6 Test on desktop Chrome with UA-spoof to iOS — iOS branch fires (platform detection, not UA string)
- [ ] 6.7 Expand details — quota figures appear with "browser-allocated quota" label; interpretive text present; loading spinner shown before resolve
- [ ] 6.8 Test with `storage.estimate()` returning null/0 — quota section omitted, no "0 bytes" shown
- [ ] 6.9 Test `storage.persist()` granted (Chrome/Edge) — moderating note appears in details tier
- [ ] 6.10 Screen reader test (VoiceOver on Safari): dialog name announced on open, ESC does nothing but description explains why, details toggle announces expanded/collapsed state, quota figures announced when details expand
- [ ] 6.11 Keyboard-only test: Tab cycles between toggle and button only; Shift+Tab reverses; background content unreachable
- [ ] 6.12 Simulate localStorage unavailability (private mode) — dialog appears, app does not crash, no error surfaced
- [ ] 6.13 Run `cargo check` and WASM build — no new warnings or errors

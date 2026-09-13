# Spec: storage-risk-warning-dialog

## ADDED Requirements

### Requirement: First-use warning dialog

The system SHALL display a full-screen, blocking modal dialog on every app launch when no dismissal timestamp has been recorded in localStorage (key: `ez-vend-storage-warning-dismissed-at`).

#### Scenario: First launch with no dismissal record

- **WHEN** the app starts and `ez-vend-storage-warning-dismissed-at` is absent from localStorage
- **THEN** the storage risk warning dialog MUST appear as a full-screen overlay before any app content is usable

#### Scenario: Subsequent launch within threshold — non-iOS

- **WHEN** the app starts on a non-iOS browser and `ez-vend-storage-warning-dismissed-at` is present and fewer than 90 days have elapsed since that timestamp
- **THEN** the dialog MUST NOT appear and the app proceeds normally

#### Scenario: Subsequent launch within threshold — iOS

- **WHEN** the app starts on an iOS browser and `ez-vend-storage-warning-dismissed-at` is present and fewer than 7 days have elapsed since that timestamp
- **THEN** the dialog MUST NOT appear and the app proceeds normally

### Requirement: Periodic re-display

The system SHALL re-display the warning dialog after a recurrence threshold has elapsed since the last recorded dismissal. The threshold differs by platform: 90 days for non-iOS, 7 days for iOS (all browsers), reflecting the iOS 7-day storage eviction window.

#### Scenario: Launch after 90-day silence on non-iOS

- **WHEN** the app starts on a non-iOS browser and 90 or more days have elapsed since the last dismissal timestamp
- **THEN** the storage risk warning dialog MUST appear as a full-screen overlay

#### Scenario: Launch after 7-day silence on iOS

- **WHEN** the app starts on an iOS browser (any browser — Chrome, Firefox, Safari — since all use WebKit on iOS) and 7 or more days have elapsed since the last dismissal timestamp
- **THEN** the storage risk warning dialog MUST appear as a full-screen overlay

#### Scenario: Threshold boundary is inclusive

- **WHEN** exactly the recurrence threshold has elapsed (to the millisecond)
- **THEN** the dialog MUST appear (boundary is inclusive)

### Requirement: Active dismissal required

The dialog SHALL require an explicit user action to dismiss. No passive dismiss path (ESC key, backdrop click, or close button) SHALL be available. The confirmation button label MUST restate the key risk so users cannot click it reflexively without reading it.

#### Scenario: User confirms understanding

- **WHEN** the user clicks the confirmation button (labelled to restate the risk, e.g. "Got it — my data stays on this device")
- **THEN** the dialog MUST close and the current UTC timestamp MUST be written to `ez-vend-storage-warning-dismissed-at` in localStorage

#### Scenario: ESC key pressed while dialog is open

- **WHEN** the dialog is visible and the user presses the ESC key
- **THEN** the dialog MUST remain open and no dismissal timestamp SHALL be recorded

#### Scenario: Backdrop click while dialog is open

- **WHEN** the dialog is visible and the user clicks outside the dialog panel
- **THEN** the dialog MUST remain open and no dismissal timestamp SHALL be recorded

### Requirement: Concise summary layout

The dialog SHALL present information in two tiers. The summary tier MUST consist of exactly one short headline sentence stating the core risk, plus at most 2 supporting one-line bullets. It MUST be readable in under 10 seconds. On iOS and macOS Safari, the platform-specific 7-day eviction bullet MUST be the first bullet point.

#### Scenario: Dialog opens — summary tier visible, details collapsed

- **WHEN** the dialog is displayed
- **THEN** only the summary tier SHALL be visible: one headline sentence and at most 2 one-line bullets
- **THEN** a disclosure control labelled contextually (e.g. "How browser storage works" or "Why this matters on Safari" for iOS) MUST be present but collapsed

#### Scenario: User expands details

- **WHEN** the user activates the disclosure control
- **THEN** the details tier MUST expand in-place beneath the summary, showing storage quota benchmarks, a fuller explanation of browser storage mechanics, and (on iOS) the eviction risk explanation

#### Scenario: Details section defaults to collapsed on recurrence

- **WHEN** the dialog is displayed on any subsequent appearance
- **THEN** the details tier MUST default to collapsed, not expanded

### Requirement: Persistent storage request

On dialog open, the system SHALL call `navigator.storage.persist()` to request durable storage for the origin. The result SHALL be reflected in the dialog's messaging.

#### Scenario: persist() granted

- **WHEN** `navigator.storage.persist()` resolves to `true`
- **THEN** the dialog MUST indicate that the origin has been granted durable storage and is not subject to routine eviction, while still advising that "Clear site data" can delete all data

#### Scenario: persist() denied or unsupported

- **WHEN** `navigator.storage.persist()` resolves to `false` or the API is unavailable (e.g. Safari, where it resolves `true` but has no effect)
- **THEN** the full elevated warning MUST be shown without moderation; on iOS this includes the 7-day eviction warning regardless of the `persist()` result

#### Scenario: persist() call fails

- **WHEN** `navigator.storage.persist()` rejects
- **THEN** the system MUST treat the result as denied and show the full warning; the error MUST be silently swallowed

### Requirement: Storage quota benchmark display

The storage quota figures (used bytes, total quota from `navigator.storage.estimate()`) SHALL be shown inside the collapsible details tier only — not in the summary. Figures MUST be labelled as "browser-allocated quota", not "available disk space". Values of 0, null, or undefined MUST be treated as unavailable and omitted.

#### Scenario: Storage estimate resolves and details are expanded

- **WHEN** the user expands the details tier and `navigator.storage.estimate()` has resolved with non-zero values
- **THEN** the details MUST show used storage and browser-allocated quota in a human-readable format (e.g. "4.2 MB used of 500 MB browser-allocated quota") alongside interpretive text explaining this is not total disk space

#### Scenario: Storage estimate not yet resolved when details are expanded

- **WHEN** the user expands the details tier before `navigator.storage.estimate()` resolves
- **THEN** a loading indicator MUST appear in place of the quota figures until the promise settles

#### Scenario: Storage estimate returns zero or null values

- **WHEN** `navigator.storage.estimate()` resolves but `usage` or `quota` is 0, null, or undefined
- **THEN** the quota section MUST be omitted from the details tier as if the API were unavailable

#### Scenario: Storage estimate fails or is unavailable

- **WHEN** `navigator.storage.estimate()` rejects or is unavailable
- **THEN** the details tier MUST still render without the quota section; no error SHALL be surfaced to the user

### Requirement: iOS elevated warning variant

The dialog SHALL display an elevated warning for iOS browsers. Detection MUST use platform detection (not user-agent string matching), since all iOS browsers — including Chrome and Firefox — use WebKit and are subject to the same storage eviction policy. The warning text MUST describe the user behaviour that triggers eviction, not the technical mechanism.

iOS detection MUST use: `/iPhone|iPad|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1)`. The `maxTouchPoints > 1` threshold (not `> 0`) avoids false positives on touchscreen Windows laptops and the emerging touchscreen MacBook; the `MacIntel` + `maxTouchPoints` clause catches iPads that report macOS platform strings (iPadOS 13+).

#### Scenario: iOS platform detected — summary tier

- **WHEN** the dialog is shown on an iOS device (detected via platform, not user-agent)
- **THEN** the first summary bullet MUST state: "Safari and all iOS browsers may delete your data if you don't open the app for 7 days"

#### Scenario: iOS platform detected — details tier

- **WHEN** the user expands the details tier on iOS
- **THEN** the details MUST include a plain-language explanation: "Apple's browser engine on iOS deletes stored data for apps that haven't been opened in 7 days. There is no way to prevent this."
- **THEN** the details MUST include a mitigation note: "To protect your data on iPhone or iPad: actively launch the app at least once a week, and export a backup regularly. This is a system-level restriction that applies to all browsers on iPhone and iPad."

#### Scenario: Non-iOS browser detected

- **WHEN** the dialog is shown on a non-iOS device
- **THEN** no iOS-specific content SHALL appear in either tier

### Requirement: macOS Safari browser recommendation

When the user is on macOS Safari (not iOS), the dialog SHALL surface the 7-day eviction risk in the summary tier and recommend switching browsers in the details tier.

#### Scenario: macOS Safari detected — summary tier

- **WHEN** the dialog is shown and `detect_browser()` returns `"Safari"` on a non-iOS platform
- **THEN** the first summary bullet MUST state: "Safari may delete your data if you don't open the app for 7 days"

#### Scenario: macOS Safari detected — details tier

- **WHEN** the user expands the details tier and the browser is macOS Safari (detected via `detect_browser()` returning `"Safari"` on a non-iOS platform)
- **THEN** the details MUST include: "On Mac, any non-Safari browser (Chrome, Firefox, Brave, Edge, etc.) removes this restriction — those browsers do not evict stored data based on inactivity."

#### Scenario: Non-Safari macOS browser

- **WHEN** the dialog is shown on macOS Chrome, Firefox, Brave, Edge, or any Chromium-based browser
- **THEN** no browser recommendation SHALL appear; the standard storage risk explanation is sufficient

#### Scenario: iOS browser (any)

- **WHEN** the dialog is shown on iOS
- **THEN** the iOS mitigation note (weekly launch + export) MUST appear instead of a browser-switch recommendation; this is a system-level restriction that applies to all iOS browsers

### Requirement: Accessible dialog implementation

The dialog MUST be implemented with correct ARIA semantics, focus management, and keyboard behaviour to meet WCAG 2.1 Level AA.

#### Scenario: Dialog opens — ARIA role and focus

- **WHEN** the dialog opens
- **THEN** the dialog container MUST have `role="dialog"`, `aria-modal="true"`, and `aria-labelledby` pointing to the headline element's `id`
- **THEN** focus MUST move programmatically to the dialog container (which MUST have `tabindex="-1"`)
- **THEN** the `inert` attribute MUST be applied to all content outside the dialog to prevent keyboard and VoiceOver cursor escape

#### Scenario: Focus trap within dialog

- **WHEN** the dialog is open and the user presses Tab
- **THEN** focus MUST cycle only between the disclosure toggle and the confirmation button
- **WHEN** the user presses Shift+Tab from the disclosure toggle
- **THEN** focus MUST wrap to the confirmation button

#### Scenario: ESC produces no response — accessible description

- **WHEN** the dialog is open and a screen reader user presses ESC
- **THEN** nothing happens; the dialog's accessible description MUST include text such as "This notice must be acknowledged before continuing" so screen reader users understand ESC is intentionally disabled

#### Scenario: Quota figures announced to screen readers

- **WHEN** the details tier is expanded and `navigator.storage.estimate()` resolves
- **THEN** the quota figures region (with `aria-live="polite"` and `aria-atomic="true"`) MUST announce the resolved values
- **THEN** the live region MUST NOT announce values while the details tier is collapsed

#### Scenario: Disclosure toggle semantics

- **WHEN** the disclosure toggle is rendered
- **THEN** it MUST be a `<button>` element with `aria-expanded` (false when collapsed, true when expanded) and `aria-controls` pointing to the details panel's `id`

#### Scenario: Headline is a semantic heading

- **WHEN** the dialog is rendered
- **THEN** the headline MUST be a semantic heading element (e.g. `<h2>`) not a styled `<div>`

### Requirement: Resilience when localStorage is unavailable

The system SHALL handle environments where `localStorage` is inaccessible (e.g. private/incognito mode) without crashing.

#### Scenario: localStorage read fails at startup

- **WHEN** the app starts and reading `ez-vend-storage-warning-dismissed-at` from localStorage throws an exception
- **THEN** the system MUST treat the state as "never dismissed" and show the dialog

#### Scenario: localStorage write fails on dismissal

- **WHEN** the user confirms the dialog and writing `ez-vend-storage-warning-dismissed-at` throws an exception
- **THEN** the dialog MUST still close and the app MUST continue normally; the failed write MUST be silently swallowed

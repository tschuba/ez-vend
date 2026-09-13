## Context

ez-booth stores all event data (booths, vendors, purchases) in IndexedDB via the browser's local storage. There is zero server-side backup — data loss is permanent and unrecoverable. The existing `StorageIndicator` footer hints at risk, but only surfaces after data already exists and requires deliberate attention to read. New users are never formally warned before they begin entering data.

The system runs as a WASM app in the browser. State persistence is done through IndexedDB and a small number of `localStorage` keys. The UI crate uses Dioxus (Rust/WASM reactive framework). All browser API calls go through `web-sys`.

## Goals / Non-Goals

**Goals:**
- Show a mandatory, full-screen blocking modal on first app launch (no existing `ez-vend-storage-warning-dismissed-at` key in localStorage)
- Re-show after 90 days on non-iOS; re-show after 7 days on iOS (matching the iOS eviction window)
- Call `navigator.storage.persist()` and reflect the result in the dialog messaging
- Display live storage quota figures in the collapsible details tier, labelled correctly
- Show an elevated iOS variant covering all iOS browsers (not just Safari UA) with plain-language eviction description
- Record dismissal timestamp to localStorage on explicit user confirmation
- Meet WCAG 2.1 Level AA

**Non-Goals:**
- Replacing or modifying the footer `StorageIndicator`
- Server-side session tracking or analytics around dismissal
- Forcing users to take a backup before using the app (only informing them)
- Supporting a "never show again" opt-out

## Decisions

### 1. Persistence key: `localStorage` timestamp

**Decision:** Store `ez-vend-storage-warning-dismissed-at` as an ISO 8601 string in `localStorage`.

**Alternatives considered:**
- IndexedDB: Overkill for a single flag; adds async complexity at startup
- In-memory only (session): Dialog would reappear on every page load, defeating the purpose

**Rationale:** Consistent with existing app patterns (`ez-vend-selected-booth-id`, pagination preferences). Synchronous read at startup avoids async gating.

**Note:** If the user clears browser storage (the exact scenario being warned about), this key is also wiped and the warning reappears as first-use. This is correct and deliberate behavior.

### 2. Show threshold: 90 days (non-iOS) / 7 days (iOS)

**Decision:** Re-show if `now - dismissed_at >= 90 days` on non-iOS; `>= 7 days` on iOS.

**Alternatives considered:**

- 30 days for all platforms: Too short — catches routine users who need reminders least; footer already provides ongoing prompts for active users
- Time-based only: The more accurate trigger would be data-volume-based (re-show when storage crosses a meaningful threshold), but that adds complexity. 90 days is a defensible time-based fallback

**Rationale (iOS):** The iOS storage eviction window is 7 days of inactivity. A 30-day recurrence would mean the dialog could fire *after* data has already been deleted on iOS. 7-day recurrence keeps the warning within the risk window.

### 3. Call `navigator.storage.persist()` and surface the result

**Decision:** Call `navigator.storage.persist()` on dialog open. If it resolves `true` on a non-iOS browser, moderate the warning ("your data is protected from routine eviction"). On iOS, treat all `persist()` results as denied — WebKit resolves `true` but the call has no effect.

**Alternatives considered:**

- Skip `persist()`: Misses the opportunity to actually reduce the risk, not just warn about it
- Always show full warning: Inaccurate if the origin has been granted durable storage on Chrome/Edge

**Rationale:** `persist()` is the correct mitigation. Showing the same warning to a user whose origin is already persistent is inaccurate and erodes trust. On Safari it is a no-op (returns `true` without effect) — document this and always show the elevated warning on iOS regardless.

### 4. iOS platform detection (not user-agent Safari matching)

**Decision:** Detect iOS using: `/iPhone|iPad|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1)`.

**Alternatives considered:**

- `'WebKit' in window && navigator.maxTouchPoints > 0`: Too broad — `maxTouchPoints > 0` fires on touchscreen Windows laptops and the new touchscreen MacBook Pro (2025). Using `> 1` filters these out since standard laptop trackpads report 1 touch point.
- UA string matching for "Safari": Chrome and Firefox on iOS both use WebKit and are subject to the same eviction policy, but neither reports "Safari" in their UA string. Matching "Safari" leaves Chrome/iOS users without the warning.
- `navigator.userAgentData.platform`: Not yet reliable across all targets.

**Rationale:** The regex covers iPhone, iPad, and iPod touch directly. The `MacIntel` + `maxTouchPoints > 1` clause catches iPads on iPadOS 13+ which report `MacIntel` as their platform string — a known Apple quirk. Together these cover all current iOS/iPadOS devices without false-positiving on touch-enabled Macs or Windows tablets.

### 5. Storage quota display: details-only, validated, correctly labelled

**Decision:** Quota figures appear in the collapsible details tier only, labelled as "browser-allocated quota" with interpretive text. Validate before display: skip the quota section if `usage` or `quota` is 0, null, or undefined.

**Rationale:** `navigator.storage.estimate()` returns a browser-determined fraction of disk space, not raw free space. Displaying it without context creates false reassurance or confusion. Safari/iOS has historically returned 0 or very small values. Graceful degradation prevents displaying "0 bytes available."

### 6. Safari detection: reuse `detect_browser()` for UA display only

**Decision:** `detect_browser()` is retained for non-security display decisions (e.g. tooltip copy). iOS eviction-risk branching uses the platform detection from Decision 4.

**Rationale:** Single source of truth for UA parsing; platform detection handles the security-relevant iOS branching.

### 7. Dialog placement: root-level overlay

**Decision:** Render the dialog in the app shell / root component as an overlay above all content. The rest of the UI renders underneath but is made inert via the `inert` attribute.

**Alternatives considered:**
- Render only in a specific route/page: Dialog would not appear if user navigates directly to a sub-route
- Separate route/page for onboarding: Requires navigation changes, more complex

**Rationale:** Root-level placement guarantees the dialog appears regardless of entry URL. The `inert` attribute (not `aria-hidden`) is the correct mechanism for removing background content from keyboard and screen reader access — this is especially important for VoiceOver on Safari/iOS where `aria-modal` alone is historically unreliable.

### 8. Dismiss action: specific CTA label

**Decision:** One confirmation button whose label restates the key risk (e.g. "Got it — my data stays on this device"). No secondary button, no close-X, no ESC dismiss.

**Alternatives considered:**

- Generic "I Understand": Reads as a reflex-click label — users associate it with fine-print consent flows and dismiss without reading. This defeats the purpose.
- Two-step micro-interaction: Would maximize comprehension but adds friction beyond what's appropriate here

**Rationale:** A label that restates the consequence forces the user to process the message before clicking. "I Understand" does not.

### 9. Progressive disclosure: 1 headline + max 2 bullets + collapsible details

**Decision:** Summary tier: one short headline sentence (the entire risk in plain language) + at most 2 supporting one-line bullets. Details tier (collapsed by default): quota benchmarks, deeper explanation, iOS eviction details. Disclosure control uses a contextual label ("How browser storage works" / "Why this matters on Safari").

**Alternatives considered:**

- Show everything at once: Users faced with a wall of text are more likely to dismiss without reading
- 3 bullets in the summary: Research shows users read the headline and first bullet, then stop. The critical message must be in the headline alone.
- "Show details" as the toggle label: Too generic — doesn't convey why the user should click

**Rationale:** Progressive disclosure lets casual users get the full message from the headline, while the detail tier serves users who want to understand the mechanism. The disclosure label signals specific value ("how browser storage works") to increase click-through among users who would benefit most.

### 10. Accessibility: `inert`, focus management, `aria-live`, button pattern

**Decision:**

- Apply `inert` to the app root element (not `aria-hidden`) when dialog is open
- Move focus to the dialog container (`tabindex="-1"`) on open, not to the button
- Use `role="dialog"` + `aria-modal="true"` + `aria-labelledby`
- Make headline a semantic `<h2>`
- Use `<button>` + `aria-expanded` + `aria-controls` for the disclosure toggle (not `<details>`/`<summary>`)
- Wrap quota figures in `aria-live="polite"` + `aria-atomic="true"`; only populate the live region when the details section is expanded
- Add "This notice must be acknowledged before continuing" to the dialog's accessible description

**Rationale:** `<details>`/`<summary>` has inconsistent VoiceOver support on iOS and doesn't compose well with `aria-live`. `aria-modal` alone does not reliably contain VoiceOver's virtual cursor on older Safari/iOS — `inert` is required. Focus moving to the dialog container (not the button) ensures screen readers announce the dialog name and context before the user can dismiss.

## Risks / Trade-offs

- **`navigator.storage.estimate()` on Safari/iOS returns 0 or null** → Validate before display; skip the quota section if values are missing. Already specced.
- **`persist()` is a no-op on Safari** → Treat all iOS results as denied and show full warning regardless. Already specced.
- **localStorage cleared with browser data** → Warning re-appears as first-use. This is correct behavior — document it so future developers don't "fix" it.
- **iOS platform detection heuristic** → Updated to `/iPhone|iPad|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1)`. Touchscreen MacBook Pro (2025) is an emerging edge case; `maxTouchPoints > 1` mitigates it. Review if Apple changes platform strings.
- **macOS Safari copy may age** → The statement "any non-Safari browser removes this restriction" is accurate today. If Apple changes Safari's storage eviction policy in a future ITP release, this copy becomes inaccurate. Add a code comment in the component: `// Verify against WebKit ITP release notes annually: https://webkit.org/blog/category/privacy/`.
- **User annoyance for active iOS users** → 7-day recurrence for iOS is frequent. Acceptable: the risk window is 7 days; the footer provides daily-use reminders; the dialog is short and requires one tap.

## Migration Plan

No data migration required. The new localStorage key is absent for all existing users, so the dialog appears once for everyone on the first build — intentional.

Rollback: Remove the dialog component and its root-level render call. The localStorage key is harmless if left behind.

## Open Questions

- Should the confirmation button label be "Got it — my data stays on this device" or something that also calls out the backup action (e.g. "I'll export backups regularly")?
- Should a secondary "Export backup now" CTA be offered alongside the confirmation, linking to the footer's export functionality?

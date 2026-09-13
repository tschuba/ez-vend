# Testing Session Findings — Implementation Plan

> Session date: 2026-05-21

Implementation status as of 2026-05-22: Issues 1, 2, and 3 are merged to `main`. The remaining issues in this document are still planned work.

## Issue 1 — WASM app becomes inaccessible after a regular PR merge to main

Implementation status: Merged to `main`. The workflow required two follow-up fixes after the initial merge: reclaiming ownership of `_site` after `actions/jekyll-build-pages`, and removing the unsupported `gh release download --latest` flag. The manual verification step below still depends on shipping a release that includes `wasm-bundle.zip`.

**Root cause:** `deploy-pages.yml` has two triggers:

1. `workflow_run` on "Tag Release" → builds docs + WASM → deploys both
2. `push` to `main` (paths: `docs/**`, README, the workflow file itself) → builds docs **only**, WASM steps are gated by `if: github.event_name == 'workflow_run'`

When a PR touching `docs/` or `README.md` is merged, trigger 2 fires and deploys a Pages site with no `/pos/` directory, making the WASM app return 404.

**Context — one build must serve two environments:**

The WASM app runs in two contexts with different base paths: the launcher (`http://127.0.0.1:<port>/`, base `""`) and GitHub Pages (`/ez-vend-rs/pos/`). `Trunk.toml` already sets `public_url = "./"` so asset references are relative and portable. The only thing preventing a single shared build is `ROUTER_BASE`, currently a compile-time constant in `crates/ez-vend-ui/src/lib.rs:5–8`. The `build-wasm` job in `release.yml` already builds without `--public-url` — no change needed there.

**Fix — runtime router base + single WASM build downloaded from release:**

### Part A — App change: runtime router base

In `crates/ez-vend-ui/src/lib.rs`, replace the compile-time `BASE_PATH` constant with a function that reads `<meta name="router-base">` from the DOM at startup via `web_sys`, defaulting to `""`:

```rust
pub fn base_path() -> &'static str {
    use std::sync::OnceLock;
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.query_selector("meta[name='router-base']").ok().flatten())
            .and_then(|el| el.get_attribute("content"))
            .unwrap_or_default()
    })
}
```

Update all `BASE_PATH` references to `base_path()` in three files:

- `crates/ez-vend-ui/src/lib.rs` — Router init (line 94), Routes init (line 185), navigation hrefs (lines 101, 114, 117, 120, 160), pathname stripping (line 38)
- `crates/ez-vend-ui/src/pages/home.rs` — lines 61, 64
- `crates/ez-vend-ui/src/components/storage_warning.rs` — line 146

No new `web_sys` features needed — `Document`, `Element`, and `Window` are already in `Cargo.toml`. `OnceLock<String>` returning `&'static str` is sound; the Leptos Router `base` prop accepts `&'static str` exactly.

### Part B — CI/CD changes

**`.github/workflows/release.yml`** — `package` job only; `build-wasm` needs no change.

In the `package` job, after the "Prepare release archives" step, add a step to zip the WASM dist. Must `cd` into the source directory to avoid embedding the path prefix:

```bash
(cd release-assets/base && zip -rq "$GITHUB_WORKSPACE/packaged/wasm-bundle.zip" .)
```

Add `packaged/wasm-bundle.zip` to the existing `upload-artifact@v7` path list so the `publish` job picks it up via the `release-packages/*` glob in `gh release create`.

**`.github/workflows/deploy-pages.yml`** — two changes:

Change `workflow_run` trigger from `"Tag Release"` to `"Release"` (fires after the full release workflow — and `wasm-bundle.zip` — is published):

```yaml
workflow_run:
  workflows: ["Release"]   # was "Tag Release"
  types: [completed]
```

Replace the three WASM build steps (currently gated `if: github.event_name == 'workflow_run'`) with one combined step running on **both** triggers. In practice, this also needs one ownership-normalization step after `actions/jekyll-build-pages`, because `_site` is not writable by the runner user otherwise. The `sed` injection is guarded so a missing download does not fail the job:

```yaml
- name: Fix generated site ownership
  run: sudo chown -R "$USER":"$USER" _site

- name: Restore WASM from latest release
  env:
    GH_TOKEN: ${{ github.token }}
  run: |
    mkdir -p _site/pos
    if gh release download --pattern "wasm-bundle.zip" --dir /tmp/wasm-dl; then
      unzip -q /tmp/wasm-dl/wasm-bundle.zip -d _site/pos
      sed -i 's|</head>|<meta name="router-base" content="/ez-vend-rs/pos">\n</head>|' \
        _site/pos/index.html
    else
      echo "No wasm-bundle.zip found — /pos/ not included in this deploy."
    fi
```

### Result

| Trigger | Behavior |
| ------- | -------- |
| Release deploy (`workflow_run` on "Release") | Fires after release is fully published → downloads `wasm-bundle.zip` → injects router base → deploys docs + new WASM |
| Docs-only deploy (`push` to main) | Fires immediately on merge → downloads latest released WASM → injects router base → deploys updated docs + stable WASM |

Single WASM build per release serves both launcher and Pages. No race condition. WASM toolchain no longer needed in `deploy-pages.yml`.

> **Transition note:** The fix only takes effect after the first release containing `wasm-bundle.zip` is published. Doc pushes before that first release will hit the soft-fail fallback (no `/pos/`). Ship this change in a release before merging any docs-only PRs.

---

## Issue 2 — Vendor and position display order is swapped in two places

Implementation status: Merged to `main` as part of the `0.1.14` version bump.

**Problem:** In both the pending checkout items list and the expanded last-checkout detail view, the item position number is shown as the primary (bold) label and vendor as secondary (small/muted). Both should show vendor first (primary) and position number second (small/muted).

### Location A — Pending checkout items list

**File:** `crates/ez-vend-ui/src/pages/checkout.rs:2332–2333`

Swap the two `<p>` elements:

```rust
// Before
<p class="font-medium">{format!("{} {}", t!("checkout.item_label")(), display_number)}</p>
<p class="text-xs text-gray-500">{format!("{} {}", t!("checkout.vendor_label")(), vendor_label)}</p>

// After
<p class="font-medium">{format!("{} {}", t!("checkout.vendor_label")(), vendor_label)}</p>
<p class="text-xs text-gray-500">{format!("{} {}", t!("checkout.item_label")(), display_number)}</p>
```

### Location B — Last-checkout expanded detail view

**File:** `crates/ez-vend-ui/src/pages/checkout.rs:2573–2589`

Swap the two sub-rows inside the per-item `<div>`:

```rust
// Before: item number first, vendor second
<div class="flex justify-between text-sm">
    <span class="font-medium text-gray-900">{/* item_number */}</span>
    <span class="font-medium text-gray-900">{format_currency(item.amount, locale)}</span>
</div>
<div class="text-xs text-gray-500 mt-0.5">{vendor_text}</div>

// After: vendor first (with amount), item number second
<div class="flex justify-between text-sm">
    <span class="font-medium text-gray-900">{vendor_text}</span>
    <span class="font-medium text-gray-900">{format_currency(item.amount, locale)}</span>
</div>
<div class="text-xs text-gray-500 mt-0.5">{/* item_number */}</div>
```

---

## Issue 3 — Amount stepping validation rule not shown in active-rules info panel

Implementation status: Merged to `main`. During validation, the implementation also fixed a same-tab selected-booth refresh gap so edited booth rules become visible in checkout without a full page reload.

**Root cause:** `RulesInfoModal` (previously `VendorRulesInfoModal`, now in `crates/ez-vend-ui/src/components/rules_info.rs`) had no `amount_stepping` prop. The Amount section rendered a hardcoded static summary and ignored the `amount_stepping` booth setting.

### Changes

**`crates/ez-vend-ui/src/components/rules_info.rs`:**

- Add prop: `#[prop(into)] amount_stepping: Signal<Option<Decimal>>`
- Add imports (none of these are currently present in this file):
  - `use rust_decimal::Decimal`
  - `use crate::formatting::format_decimal`
  - `use crate::i18n::use_locale`
- In the Amount section, keep the static `rules_amount_summary` paragraph (base "> 0" rule — always shown), and add a conditional block using `{move || amount_stepping.get().map(|step| view! { <p>...</p> })}` (no fallback branch needed). When `amount_stepping` is `None`, nothing extra is shown.
- Format `step` with `format_decimal(step, locale, 2)` — do not use `step.to_string()`, which always produces dot notation and would show `"0.50"` to German users instead of `"0,50"`.

**`crates/ez-vend-ui/src/pages/checkout.rs:2733–2738`:**

- Pass `amount_stepping` to the modal (`amount_stepping` memo already exists at line 859):

  ```rust
  amount_stepping=Signal::derive(move || amount_stepping.get())
  ```

**`crates/ez-vend-ui/locales/en.json` and `locales/de.json`:**

- Add key under `checkout`:
  - EN: `"rules_amount_stepping": "Amounts must be in increments of {step}."`
  - DE: `"rules_amount_stepping": "Beträge müssen in Schritten von {step} erfasst werden."`

**`crates/ez-vend-ui/src/selected_booth_context.rs`:**

- Refresh the selected booth in the same tab when `booth_list_version` changes so editing booth settings updates checkout state immediately.
- Reuse the same repository-backed refresh path for both same-tab version changes and cross-tab `storage` events.

**Style cleanup:**

- Rename the modal/component surface from `vendor_rules_info` / `VendorRulesInfoModal` to `rules_info` / `RulesInfoModal` so the name matches its broader scope.

---

## Issues 4–6 — Cross-device event merge

The import duplicate bug, pre-import merge analysis, and local duplicate cleanup are specified in a dedicated document:

**→ [CROSS_DEVICE_EVENT_MERGE.md](CROSS_DEVICE_EVENT_MERGE.md)**

---

---

## Issue 7 — Browser storage safety warnings

**Context:** Two browser contexts can silently destroy all IndexedDB data without warning the operator. Neither is currently surfaced in the UI.

### Scenario A — Safari under storage pressure

Safari has historically cleared origin-scoped storage (including IDB) when the device is low on space. This improved in Safari 16.4 but Safari still does not guarantee IDB durability the way Chrome/Firefox do on desktop. An operator running a live event in Safari could lose all vendor and purchase records without any browser-level warning. This applies to all iOS browsers (Chrome for iOS, Firefox for iOS) since they all run on WebKit and share the same storage behavior.

**Detection:** Reuse the existing `detect_browser()` function in `crates/ez-vend-ui/src/utils.rs` — it already uses the correct exclusion-based UA pattern. Treat all Apple-platform browsers as in-scope since the WebKit storage risk applies to all of them.

### Scenario B — Private / incognito window

In all major browsers, IDB created in a private window is destroyed when the window closes. An operator who opens the app in a private window and records an entire event’s purchases will lose all data at the end of the session.

**Detection:** Two-stage approach:

1. **Primary — IDB open test:** Attempt to open an IDB database at startup. Safari private mode throws a `SecurityError` synchronously — the most reliable signal for the most dangerous case.
2. **Supplementary — `navigator.storage.persisted()`:** Returns `false` without prompting in all private/incognito modes across Chrome, Firefox, and Safari. Use as a corroborating signal for Chrome/Firefox incognito where the IDB test does not throw.

Do **not** use a `navigator.storage.estimate()` quota threshold. Since Chrome 116 (2023), incognito uses disk-backed IDB and returns a large quota value — the threshold approach silently fails to detect Chrome incognito.

For the async `navigator.storage.persisted()` call: add `"StorageManager"` to the workspace `web-sys` feature list; call via `spawn_local` inside `create_effect` in Leptos.

The warning is framed as *“you are in a private window”* when the IDB test confirms it (Safari private), and *“this may be a private window”* when relying on the supplementary signal alone (Chrome/Firefox incognito).

### Visual theming

The two warnings use **different color themes** to signal conditional versus certain risk:

- **Safari banner — amber** (`bg-amber-100 border-amber-300 text-gray-900`): matches the existing `StorageWarningInfo` advisory pattern. Operators already associate amber with backup advisories. Amber signals *“this might happen.”*
- **Private window banner — red** (`bg-red-50 border-red-300 text-gray-900`): escalates to signal guaranteed loss, not a possibility. Red signals *“this will happen.”*

Color alone must not carry the meaning — both banners include a warning icon and severity language in the text so they work in forced-colors / Windows high-contrast mode.

### Behavior

Both banners appear at the top of the app on page load, non-blocking, dismissible with a close button (no checkbox required).

**Safari banner** dismiss is persisted to `localStorage` (key: `ez-vend-safari-storage-warning-dismissed`), matching the existing `StorageWarningInfo` collapse behavior. The banner reappears only after browser data is cleared — not on every page load.

**Private window banner** dismiss is in-memory only (session-scoped Leptos signal). `localStorage` is unreliable in Safari private mode. The banner reappears on every page load within the same private session, which is correct since the risk is still present.

**Precedence:** When both conditions are detected, show the red private window banner only. After it is dismissed, do not cascade to the Safari amber banner.

**Placement:** Both banners render at layout level (every page). An operator may navigate directly to the checkout page via bookmark.

**Overlap with existing `StorageWarningInfo`:** Suppress `StorageWarningInfo` when the Safari banner is active to avoid two overlapping amber banners on the same page.

**Export CTA:** Include an inline “Export Backup” / “Backup exportieren” button in the Safari banner using the existing `export_button.rs` component. For the private window banner, include keyboard shortcut instructions in the copy instead (no programmatic way to open a non-private window).

### Copy

**Safari (EN):**
> Safari can silently delete event data when your device runs low on storage — without any warning. Export a backup before and after each event to keep your records safe. [Export Backup ↓]

**Safari (DE):**
> Safari kann Veranstaltungsdaten lautlos löschen, wenn der Gerätespeicher knapp wird – ohne Vorwarnung. Exportieren Sie ein Backup vor und nach jeder Veranstaltung. [Backup exportieren ↓]

**Private window, IDB-confirmed (EN):**
> You are in a private window. Any data you enter here will be permanently lost when this window closes. Your existing records are safe — open EZ Booth in a regular browser window. (⌘N / Ctrl+N opens one)

**Private window, heuristic only (EN):**
> This may be a private window. If it is, all data entered here will be permanently lost when this window closes. Open EZ Booth in a regular browser window to keep your records. (⌘N / Ctrl+N)

Provide DE equivalents for all four variants.

### Accessibility

- `role="alert"` on both banner containers (assertive — data-loss risk warrants it).
- Pre-register an empty alert container in `index.html` before WASM loads so the screen reader registers the live region before content arrives:
  ```html
  <div id="storage-warning-region" role="alert" aria-atomic="true"></div>
  ```
- Dismiss button: `type="button"` + language-specific `aria-label` naming the specific warning (e.g. `"Dismiss Safari storage warning"` / `"Hinweis zu Safari schließen"`). Not a plain `"Dismiss"`.
- Icon inside dismiss button: `aria-hidden="true" focusable="false"`.
- On dismiss, move programmatic focus to `#main-content` (`tabindex="-1"`). Do not move focus to the banner on appearance.
- Banner mount point placed early in DOM order (before app content).

### i18n keys (under `backup.` namespace)

```
backup.safari_warning_label
backup.safari_warning_message
backup.safari_warning_cta
backup.private_window_warning_label
backup.private_window_warning_message_confirmed
backup.private_window_warning_message_heuristic
backup.private_window_warning_cta_instructions
```

### Changes

**`index.html`:**
- Add `<div id="storage-warning-region" role="alert" aria-atomic="true"></div>` before the Leptos mount point.

**`crates/ez-vend-ui/src/components/storage_warning.rs`:**
- Safari detection via `detect_browser()` from `utils.rs` (already correct for this use case).
- Private-mode detection: IDB open test (primary, synchronous) + `navigator.storage.persisted()` (supplementary, async via `spawn_local` in `create_effect`).
- Two new warning variants with amber vs. red theming.
- Safari dismiss persisted to `localStorage` (key `ez-vend-safari-storage-warning-dismissed`).
- Private window dismiss in session-scoped Leptos signal only.
- Precedence and no-cascade logic.
- Suppress `StorageWarningInfo` when Safari banner is active.

**`Cargo.toml` (workspace `web-sys` features):**
- Add `"StorageManager"`.

**`crates/ez-vend-ui/locales/en.json` and `de.json`:**
- Add all keys under `backup.` namespace as listed above.

### Files

- `index.html`
- `crates/ez-vend-ui/src/components/storage_warning.rs`
- `crates/ez-vend-ui/locales/en.json`
- `crates/ez-vend-ui/locales/de.json`
- `Cargo.toml` (workspace `web-sys` features)

### Verification

Safari (regular): confirm **amber** banner appears with Export CTA. Dismiss — confirm no reappearance on next load. Clear Safari site data — confirm it reappears.

Safari private: confirm **red** private-window banner appears (not amber). Dismiss and reload — confirm it reappears.

Chrome/Firefox incognito: confirm **red** private-window banner appears.

Regular Chrome/Firefox: confirm neither banner appears.

Screen reader (VoiceOver + Safari): confirm banner is announced on page load without requiring focus navigation to it. Confirm dismiss button label names the specific warning. Confirm focus moves to main content on dismiss.

---

## Files to modify (summary)

| File | Issues |
| ---- | ------ |
| `crates/ez-vend-ui/src/pages/checkout.rs` | 2, 3 |
| `crates/ez-vend-ui/src/components/vendor_rules_info.rs` | 3 |
| `crates/ez-vend-ui/locales/en.json` | 3, 7 |
| `crates/ez-vend-ui/locales/de.json` | 3, 7 |
| `crates/ez-vend-ui/src/lib.rs` | 1 |
| `crates/ez-vend-ui/src/pages/home.rs` | 1 |
| `crates/ez-vend-ui/src/components/storage_warning.rs` | 1, 7 |
| `index.html` | 7 |
| `Cargo.toml` | 7 |
| `.github/workflows/release.yml` | 1 |
| `.github/workflows/deploy-pages.yml` | 1 |

See [CROSS_DEVICE_EVENT_MERGE.md](CROSS_DEVICE_EVENT_MERGE.md) for files affected by Issues 4–6.

---

## Verification

- **Issue 1:** After the fix ships in a release, merge a docs-only PR and confirm the WASM app is still reachable at `/ez-vend-rs/pos/` after the Pages deploy completes.
- **Issue 2:** Start dev server, open register view, add items — confirm vendor name is the primary (bold) label and position number is secondary in both the pending list and the last-checkout detail view.
- **Issue 3:** Open register view with a booth that has `amount_stepping` set; open the rules info panel — confirm the stepping rule appears. Test with no stepping set — confirm only the static base summary shows.
- **Issue 7:** See per-scenario verification steps in the Issue 7 section above.

See [CROSS_DEVICE_EVENT_MERGE.md](CROSS_DEVICE_EVENT_MERGE.md) for verification of Issues 4–6.

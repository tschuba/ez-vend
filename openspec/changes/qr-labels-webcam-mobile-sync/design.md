## Context

ez-vend-rs is a Leptos/WASM offline-first flea-market checkout app. The core flow today requires manual entry of vendor ID and price per item. This design covers the full technical architecture for Phases 1–3 (Label printing, Webcam scanning, Mobile sync) and deferred Phases 4–5 (OCR, Organiser-App).

**Constraints that must not be violated:**
- Offline-first: the core checkout flow works without internet at all times
- No npm dependencies in WASM crates (subject to `rxing` spike result)
- Separate WASM bundles per audience to minimise download size and prevent cross-boundary dependencies
- Single-organiser deployments for the Coolify server path; multi-tenant is unsupported

**Deployment Prerequisite:** Each organiser must use a dedicated Coolify instance. `event_code` uniqueness is not enforced server-side; collisions silently interleave financial data. Shared server deployments are unsupported.

---

## Goals / Non-Goals

**Goals:**
- Replace manual vendor-ID + price entry with QR scan at the register
- Allow helpers to scan items on a phone and sync purchases to the register
- Give vendors a browser-based tool to print QR stickers before the event
- Maintain offline-first guarantee across all phases and all deployment scenarios
- Provide a validated feasibility spike before committing to Phase 2 / Phase 4 implementation

**Non-Goals:**
- Bluetooth sync (insufficient browser support)
- Server-side PDF generation
- Vendor login / authentication
- Event code embedded in item sticker QR codes (stickers encode only `vendor_id` + `price_cents`)
- Real-time relay between phone and register (replaced by batch sync)
- Remote shopping cart ("Send to register") — physically insecure, excluded entirely
- Digital payment (cash-only)
- Multi-tenant Coolify deployments

---

## Decisions

### §1 — Separate WASM bundles per audience

Each target audience gets its own independent Leptos WASM crate with its own deployment URL:
```
crates/ez-vend-app/       → Kassen-App    (cashier, stationary register)
crates/ez-vend-labels/    → Label-App     (vendor, prints stickers)
crates/ez-vend-mobile/    → Mobile-App    (helper, scans with phone)
crates/ez-vend-organizer/ → Organiser-App (organiser, Phase 5)
crates/domain/             → shared models, services, QrLabelPayload
crates/ez-vend-ui/        → UI components (Kassen-App only)
```

`ez-vend-mobile` depends **only** on `crates/domain` — not on `ez-vend-ui`. It reimplements minimal UI components to avoid dragging in checkout-specific code. This is accepted code duplication in exchange for clean separation.

*Alternative considered:* shared UI crate for all apps — rejected because it couples checkout-specific components to mobile and labels, increasing bundle size and creating unwanted dependency paths.

---

### §2 — QR payload format

**Format:** `v={vendor_id}&p={price_cents}` — URL query parameter format, percent-encoded.

Example: `v=42&p=300` encodes vendor 42 at €3.00. Maximum-length payload: `v=999&p=999999` (15 chars).

Parsed via `url::form_urlencoded` in Rust. The plain-text format on the sticker encodes only vendor + price; no event binding (the event is not relevant at scan time — any register with matching `event_code` can accept any sticker).

---

### §3 — Label link URL format (plain, no obfuscation)

**Format:** `{labels-url}/?v={vendor_id}&e={event_code}`

The previous design used XOR obfuscation with a key distributed publicly in onboarding QRs — the obfuscation was cosmetic. Removing it eliminates complexity with no real security loss.

**Note:** Label link (`?v=&e=`) ≠ sticker QR (`v=&p=`). The label link navigates a vendor to the Label-App to configure and print stickers. The sticker QR is what the cashier scans at checkout.

---

### §4 — `event_code` as cross-device event identifier

`Booth` gains a mandatory `event_code: String` field. This is the shared identifier across all registers and mobile devices for the same event. `booth.id` (UUID) remains device-local for per-register accounting.

**Derivation:** `{initials}-{MMYY}` from event name + date.
Rules:
- Transliterate umlauts (ä→A, ö→O, ü→U, ß→S); strip non-alphanumeric
- Skip words whose normalised form is ≤ 3 chars (catches articles/prepositions without a hardcoded list)
- Take the first letter (uppercase) of each remaining significant word, up to 4
- If only one significant word remains, take its first 2 chars
- Date: zero-padded month + 2-digit year (MMYY) from the event's start date (not system date)
- Separator: `-` between initials and date
- Fallback: if no significant words remain, use first 2 chars of the first word

Examples: `"Flohmarkt Mai 2026"` → `FM-0526`, `"Großer Herbstmarkt"` → `GH-1026`

**Local collision avoidance:** When the Kassen-App generates an event_code suggestion, it checks all existing `booth.event_code` values. If a match exists, it appends `-2` (incrementing until unique) and notifies the organiser.

**Lock trigger — amended from ADR:** The `event_code` field is locked **when the mobile onboarding QR is first rendered on screen** (not on first sync received, which was asymmetric — the leading register never receives a sync). Implementation: `onboarding_qr_shown_at: Option<DateTime<Utc>>` added as a field on the `Booth` struct, serialised to localStorage as part of the `Booth` JSON object. Once set, the edit field is disabled with a warning. This covers the leading register symmetrically.

Before the lock, a strong warning is shown on any edit attempt: *"Changing the event code breaks any mobile device that has already configured this event."*

**Recovery procedure for event_code mismatch (post-event start):**
1. Identify the mismatched register by comparing `event_code` on each register's event screen
2. On the mismatched register: re-enter event creation, type the correct code manually
3. Purchases already taken on the mismatched register are unaffected (they are booth-UUID-local)
4. Mobile sync to the corrected register resumes immediately once codes match
5. Any purchases from helpers that synced to the mismatched register before correction must be re-exported and re-imported to the correct register

**Pre-event coordination workflow:** The event creation confirmation screen shows `event_code` prominently with the instruction: *"Share this code with all registers before the event starts."* The mobile onboarding QR is displayed on the same screen — scanning it on another register pre-fills its `event_code` field.

**Onboarding QR encoding:** `ez-booth://onboard?e={event_code}&n={url_encoded_event_name}`

**Server behaviour:** `events.booth_id` is NULL for mobile-synced purchases. `event_code` is stored in the `payload` JSONB field.

---

### §5 — QR decoding in WASM — `rxing` crate

**Decision:** `rxing` (pure Rust, no JS/npm dependency) for QR decoding.

**Gate:** This is unverified. Phase 2 must not begin until the feasibility spike (`crates/ez-vend-prototype`) confirms:
- `trunk build` succeeds with `rxing` in Cargo.toml
- Live QR decode latency < 200ms/frame in-browser
- `rxing` WASM contribution to the bundle is < 500 KB (measured via artifact size diff before/after adding `rxing`)

**Fallback:** If `rxing` fails the spike, use `ZXing-wasm` via `wasm-bindgen`. This introduces a JS dependency but is the only viable alternative.

**Spike — crate structure:**

```text
crates/ez-vend-prototype/
├── Cargo.toml         # cdylib; leptos, rxing (wasm feature), qrcode, web-sys, js-sys, gloo-timers
├── index.html         # trunk entry point; Tesseract.js <script> tag
└── src/
    ├── lib.rs         # app root; ScanMode signal; log_entries RwSignal; mounts QrGenerator, Scanner, Log
    ├── qr_gen.rs      # QR Generator component
    ├── qr_scan.rs     # QR Scanner component (rxing + webcam frame loop)
    └── ocr_scan.rs    # OCR Scanner component (Tesseract.js interop + webcam)
```

**Spike — page layout** (single scrollable page, no router):

```text
┌──────────────────────────────────────┐
│  ez-booth prototype — feasibility    │
├──────────────────────────────────────┤
│  § QR GENERATOR                      │
│  Vendor ID: [__]  Price (¢): [____]  │
│  [Generate QR]                       │
│  [canvas — QR image]                 │
├──────────────────────────────────────┤
│  § SCANNER                           │
│  [QR Scan] [OCR Scan]  ← toggle      │
│  [video — webcam feed]               │
│  [canvas — hidden, frame extraction] │
│  Result: …  Confidence: …%           │
│  → AUTO-ACCEPT / NEEDS CONFIRMATION  │
├──────────────────────────────────────┤
│  § LOG                               │
│  [scrolling text of decode attempts] │
└──────────────────────────────────────┘
```

**Spike — shared webcam lifecycle:** One `<video>` element is shared between QR Scan and OCR Scan modes. `getUserMedia` is called once when the Scanner section mounts. The frame loop (100 ms via `gloo_timers::callback::Interval`) feeds the hidden `<canvas>` and dispatches to whichever mode is active. Camera is stopped (`MediaStream.getTracks()[0].stop()`) on unmount.

**Spike — `qr_gen.rs`:** Inputs `vendor_id: String`, `price_cents: u32`. Encodes `format!("v={vendor_id}&p={price_cents}")`. Uses `qrcode` crate to produce a pixel matrix; renders to `<canvas>` via `web_sys::CanvasRenderingContext2d`. Validates inputs (non-empty vendor_id, price > 0) before generating.

**Spike — `qr_scan.rs`:** Shares `<video>` via Leptos `NodeRef`. Frame loop: `gloo_timers::callback::Interval::new(100, ...)`. Each tick: copy frame to canvas → `ImageData` → luma buffer → `rxing`. Dedup: `HashMap<String, f64>` with `Performance::now()`, suppress within 2000 ms. Displays decoded payload, parsed `v=`/`p=` fields, and decode latency (ms). On failure: logs to LOG section.

**Spike — key dependencies:**

| Crate | Purpose | Notes |
| --- | --- | --- |
| `rxing` | QR decode (pure Rust) | Enable `wasm` feature |
| `qrcode` | QR encode | Pure Rust, no WASM feature needed |
| `gloo-timers` | 100 ms frame loop | Check if already in workspace |
| `wasm-bindgen-futures` | Async Tesseract.js calls | Likely already present |
| `js-sys` | Tesseract.js JS interop | Likely already present |

**Spike — verification:**
```bash
cd crates/ez-vend-prototype && trunk serve
# 1. QR Generator: vendor_id=42, price_cents=300 → scan with phone → "v=42&p=300"
# 2. QR Scan: hold QR to webcam → decoded payload appears in DOM
# 3. Check LOG for decode latency
```

---

### §6 — Dedup timing in WASM

`web_sys::Performance::now()` (returns `f64` milliseconds) — not `std::time::Instant` which is unavailable in WASM.

```rust
last_scanned: HashMap<String, f64>
// Same QR payload within 2000ms → suppress; pulse highlight on existing cart row, no sound
```

---

### §7 — Item source tracking

Every `CheckoutItem` and `StoredCheckoutItem` carries `source: ItemSource`:

```rust
enum ItemSource {
    Manual,        // cashier typed entry
    Scanned,       // QR decoded, price unchanged
    ScannedEdited, // QR decoded, cashier manually corrected the price
}
```

- `Scanned` → 📷 icon in cart
- `ScannedEdited` → ✎ icon (price was corrected; camera icon would mislead)
- `StoredCheckoutItem` serialises `source` so the icon persists across reloads

---

### §8 — Input modes

```rust
enum InputMode { Manual, Scan }  // Phase 2
// Phase 4 adds: OcrScan
```

**UI mechanism:** A toolbar toggle button labelled "Eingabe | QR-Scan" with keyboard shortcut `S`. In Phase 4 the toggle becomes three-way: "Eingabe | QR-Scan | Handschrift". Mode persists in localStorage key `input_mode`.

---

### §9 — Storage strategy per app

| App | Storage | Keys / Notes |
|-----|---------|--------------|
| Kassen-App | `localStorage` | Small dataset, synchronous access sufficient |
| Mobile-App | `IndexedDB` | Larger dataset, async API, survives longer sessions |

**Complete Kassen-App localStorage key schema:**

"Cleared on close" means cleared when the cashier explicitly closes/ends the event via the UI, or when a new event is created on the same device.

| Key (actual string) | Type | Lifecycle | Max size |
| --- | --- | --- | --- |
| `ez-vend-checkout-draft` | JSON `StoredCheckoutForm` (booth_id, vendor_id, amount, items with `source`) | Per-event; cleared on event close | ~50 KB |
| `completed_purchases` | JSON array of UUID strings | Per-event; cleared on event close | ~500 KB |
| `last_sync_sequence` | integer | Per-booth; persists across events | 8 bytes |
| `input_mode` | string enum (`"Manual"` / `"Scan"` / `"OcrScan"`) | Persists across reloads | < 20 bytes |
| `ez-vend-checkout-keyboard-visible` | boolean | Persists | 1 byte |
| `ez-vend-checkout-amount-input-mode` | string enum (existing `AmountInputMode`) | Persists | < 20 bytes |
| `ez-vend-checkout-error-sound-enabled` | boolean | Persists | 1 byte |

**Note:** `last_upload_ok` is runtime in-memory state only — it must NOT be stored in localStorage. If it were persisted, it would incorrectly skip the POST after a page reload. The Sync button initialises it as `false` on each page load.

**Note:** `onboarding_qr_shown_at` and `event_code_confirmed` are fields on the `Booth` struct, serialised within the `Booth` JSON object in localStorage (not standalone keys). See §21 for `event_code_confirmed`.

---

### §10 — Purchase deduplication

Every purchase carries a UUID v4 generated locally at creation.

- Collision probability across devices: negligible (122 random bits)
- **Vendor table scope:** `get_or_create(vendor_id, booth_id)` uses `(vendor_id, booth_id)` as its uniqueness key — the vendor table is booth-scoped, not global. Vendor ID 42 on Event A and Event B are independent records.
- Server dedup: partial unique index `idx_events_purchase_dedup ON events(entity_id) WHERE event_type = 'purchase_upserted'`; insert: `ON CONFLICT DO NOTHING`
- File import: UUID check against `completed_purchases` in localStorage; duplicates silently skipped

---

### §11 — Mobile sync: API endpoints and file export

**POST /api/sync** (upload — Coolify only, unauthenticated):
```http
POST /api/sync
Body: { purchases: [...], client_id: "device-uuid" }
Each purchase: { id, vendor_id, price_cents, occurred_at, event_code, ... }
```

`client_id` is a UUID v4 generated on first Mobile-App start, stored in IndexedDB as a stable device ID.

**GET /api/sync** (download — Coolify only):
```
GET /api/sync?since={last_sequence}
Response: { purchases: [...], next_sequence: 42 }
```

Returns `purchase_upserted` events with `sequence > since`, up to 500 per request.

**Pagination contract:** The Kassen-App loops `GET /api/sync?since={next_sequence}` while `purchases.length == 500`. Loop terminates only when the response has fewer than 500 items. Maximum 20 iterations (safety guard — if reached, surface a sync error, do not silently stop). The server guarantees `next_sequence > since` whenever `purchases.length > 0`. `last_sync_sequence` in localStorage stores the cursor.

**POST + GET error handling:** The Sync button displays split status: "Upload: OK / Download: fehlgeschlagen" as independent indicators. `last_upload_ok` is **in-memory runtime state only** (not stored in localStorage — see §9). If POST succeeded but GET failed, the next Sync attempt within the same page session skips POST and retries GET only. After a page reload, `last_upload_ok` resets to `false` and POST is re-sent; this is safe because the server deduplicates via `ON CONFLICT DO NOTHING`.

**Threat model:** `POST /api/sync` provides no protection against a participant who scanned the onboarding QR and submits fabricated purchase data after the fact. The `event_code` is predictable from the public event name. **Organisers must cross-check server-synced totals against register totals before vendor payout.**

**Server-side collision warning:** The server logs a warning (not error) when two different `client_id` values POST with the same `event_code` in the same calendar month. This gives an ops signal without breaking the sync flow.

**File export/import** (always available, all deployment scenarios):
- Mobile-App "Export" button downloads pending purchases as a `.json` file
- **Naming convention:** `ez-booth_{event_code}_{client_id_short8}_{YYYY-MM-DD}.json` — set via `<a download="...">` attribute
- Transfer via USB, AirDrop, email, or shared folder
- Kassen-App imports via file picker; same UUID dedup as server sync

---

### §12 — Mobile: multi-event support + batch state machine

The Mobile-App supports multiple configured events simultaneously. Events are added via:
1. QR scan from the Kassen-App onboarding screen (primary)
2. "Enter code manually" fallback — `event_code` + event name text fields, below the QR scanner

**Batch state machine:**
```
pending
  → server_uploaded   (POST /api/sync returned 200)
  → file_exported     (user triggered download, file transfer considered user's responsibility)
```

`file_exported` suppresses the event-removal warning — the organiser accepted transfer responsibility. Server-uploaded batches are excluded from the next POST payload.

**Batch status UI:** Event detail screen shows "N batches ausstehend · zuletzt synchronisiert HH:MM" (or "nie"). End-of-event state: "Alles synchronisiert ✓". "Verlauf anzeigen" disclosure opens the batch list.

---

### §13 — Mobile PWA + offline support

- `manifest.json` for Add-to-Home-Screen on Android and iOS (`name`, `icons`, `display: standalone`)
- Manually written `sw.js` with Cache-First strategy; versioned cache name; `skipWaiting()` + `clients.claim()` for immediate activation
- The OCR model (~15 MB, Phase 4) is **not** precached — Tesseract.js caches it independently after first explicit download
- Scan loop pauses via Page Visibility API when backgrounded

**iOS limitation:** Safari purges SW cache and IndexedDB after ~7 days of inactivity.

**Cache staleness detection (in-app):** On startup, the Mobile-App compares an embedded build-version constant against a lightweight `version.json` at `/version.json` relative to the app root. If the fetch fails (offline) and the last-opened timestamp is > 6 days old, it displays a banner: *"Dein Offline-Speicher könnte veraltet sein. Vor dem Event kurz neu laden."*

**`version.json` generation:** Trunk emits this file via a post-build hook or `build.rs`. Minimum content: `{ "version": "<git_sha_short>" }`. The file is deployed alongside the WASM bundle at the app root. The SW cache manifest does **not** precache it — it must always be fetched live to reflect the latest build.

---

### §14 — Phase 4: Handwritten label OCR (DEFERRED)

**Decision:** Tesseract.js (JavaScript library, ~15 MB model) for client-side OCR.

**Gate:** Requires the feasibility spike to confirm that the `wasm-bindgen`/`js-sys` extern block can drive Tesseract.js's **Worker-mode API** (`createWorker()`) from Rust. Worker mode is the only Tesseract.js configuration that avoids blocking the main JS thread — `spawn_local` alone is insufficient because Tesseract.js v4+ runs inference synchronously inside the Promise executor unless explicitly initialised as a Web Worker. The spike question is not *whether* a worker is needed (it is), but *whether* `wasm-bindgen` can call the Worker API.

If Worker-mode interop is infeasible, Phase 4 is deferred until a pure-Rust OCR crate is viable, or falls back to server-side `/api/ocr` (Coolify only).

**Spike — `ocr_scan.rs`:** Same webcam feed and frame loop as `qr_scan.rs`. Tesseract.js interop via `#[wasm_bindgen]` extern block targeting `createWorker()`:

```rust
#[wasm_bindgen]
extern "C" {
    type TesseractWorker;
    // createWorker(lang) -> Promise<TesseractWorker>
    // worker.recognize(imageData) -> Promise<{data: {text, confidence}}>
}
```

Tesseract.js loaded lazily on first OCR attempt (avoids 15 MB download at startup). Injected via `<script>` tag in `index.html`; language hint: `"deu"`. Recognition dispatched via `wasm_bindgen_futures::spawn_local` awaiting the Worker Promise. If the Worker API is not callable via `wasm-bindgen` extern syntax, the LOG section records the compile/runtime error.

**Spike — verification:**

```bash
# 3. OCR Scan: hold handwritten "42 / 3,50" to webcam → text + confidence % appear
# 4. Check LOG for interop errors or UI-blocking indicators
```

**Confidence flow:**
- Confidence ≥ 85% + validation OK → auto-accept (no dialog)
- Confidence < 85% or validation error → confirmation dialog

**Confirmation dialog:** Shows (a) captured image frame cropped to label area, (b) parsed result in an editable price field (pre-filled), (c) confidence % in muted text, (d) Accept + Cancel buttons. Validation on Accept: result must parse as a German decimal price.

**Failure / empty result:** If OCR returns empty or non-parseable output after timeout, toast: *"Beschriftung nicht lesbar — Preis manuell eingeben."* Focus moves to manual price field. `InputMode` stays in `OcrScan` (cashier can retry).

---

### §15 — Phase 5: Organiser-App `event_code` generation (DEFERRED)

When the Organiser-App creates an event, it derives `event_code` client-side using the same name+date algorithm. The `event_code` is part of the exported JSON; Kassen-App import preserves it so label links and mobile onboarding remain consistent.

---

### §16 — Label-App: minimum label size enforcement

Two-tier validation:

| Tier | Threshold | Behaviour |
|------|-----------|-----------|
| Hard block | QR code region < 25×25 mm | Printing blocked. Message: *"Der QR-Code-Bereich muss mindestens 25×25 mm groß sein, damit er zuverlässig gescannt werden kann. Druck gesperrt."* |
| Soft warning | Total label < 40×25 mm | Warning shown, dismissible per session. Message: *"Etiketten dieser Größe werden möglicherweise nicht zuverlässig gescannt. Du kannst trotzdem drucken und das Risiko akzeptieren."* |

The three preset sizes (Klein 48×30 mm, Mittel 64×34 mm, Groß 70×50 mm) all exceed the soft-warning threshold.

**Payload-sensitive validation:** The minimum is computed against the *actual* QR payload being encoded, not a fixed example. Worst-case payload `v=999&p=999999` (15 chars) at ECC level M maps to QR version 2 (25×25 modules); at 25 mm, each module ≈ 1 mm — within reliable scanning range. The Label-App must dynamically compute the required QR version for the actual payload and derive the minimum label size from it.

---

### §17 — Label link distribution

**Vendor list page** gains a "Label-Link" column with:
- A QR code icon → expands a modal showing the link as a scannable QR (for vendor check-in tablets)
- A "Link kopieren" button
- **Bulk export:** "Alle Vendor-Links exportieren" downloads a single-page HTML with all vendor QR codes — printable as a reference sheet for check-in or pre-distribution

---

### §18 — InputMode switch UI

- **Toggle button** in checkout toolbar: "Eingabe" / "QR-Scan" (Phase 2); three-way "Eingabe | QR-Scan | Handschrift" (Phase 4)
- **Keyboard shortcut `S`** — switches between Manual and Scan modes
- Mode persists in localStorage key `input_mode`

---

### §19 — Scan feedback details

- **Successful scan:** scan success sound plays; new cart row added
- **Duplicate suppressed (within 2000ms window):** existing matching cart row pulses with a brief highlight (~300ms CSS animation); no new row; scan success sound does *not* play
- **ItemSource transitions:** `Manual` → typed; `Scanned` → QR decoded, price unchanged; `ScannedEdited` → QR decoded, cashier corrected price (icon: ✎)

---

### §20 — Vendor table scope

`vendor_service::get_or_create(vendor_id, booth_id)` uses `(vendor_id, booth_id)` as the uniqueness key. The vendor table is **booth-scoped** — not global. Vendor ID 42 on two different events are independent records. This is critical for correct payout accounting when the same Kassen-App device is used across multiple events.

---

### §21 — Migration strategy

**`0003_booth_event_code.sql`:**
1. Add `event_code TEXT` column (nullable)
2. Backfill: apply the derivation algorithm to `(description, date)` for each existing row — the algorithm is deterministic and safe to run server-side in SQL or a one-off migration script
3. Set `NOT NULL` constraint after backfill

**`event_code_confirmed: bool` field:** Added to the `Booth` struct alongside `onboarding_qr_shown_at`, serialised within the `Booth` JSON object. Defaults to `false`. Set to `true` when the organiser explicitly confirms the code.

**Post-migration UX:** If a booth's `event_code_confirmed` is `false` (all backfilled booths start with `false`), the Kassen-App shows a one-time prompt on first open: *"Bitte überprüfe und bestätige deinen Event-Code, bevor du mobile Geräte einrichtest."* On confirmation, `event_code_confirmed` is set to `true` and the prompt is not shown again.

**Rollback:** Remove `NOT NULL` constraint and set column nullable; existing data is unaffected.

---

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| `rxing` WASM binary too large or decode > 200ms/frame | Phase 2 spike gates this. Fallback: `ZXing-wasm`. Update design.md §5 if switched. |
| Tesseract.js JS interop blocks UI thread | `spawn_local` + Web Worker investigation in spike. Phase 4 deferred if not feasible. |
| Tesseract.js not callable via `wasm-bindgen` extern at all | Phase 4 deferred until a pure-Rust OCR crate is viable; server-side `/api/ocr` promoted to primary path. |
| OCR confidence < 85% on clean handwriting | Threshold needs adjustment; consider 70% or manual-only fallback. Update design.md §14. |
| `ez-vend-mobile` reimplements UI components already in `ez-vend-ui` | Accepted duplication. Mobile crate is intentionally decoupled. |
| event_code mismatch between registers discovered mid-event | Recovery procedure documented in §4. Organisers trained on pre-event coordination workflow. |
| Server-side event_code collision interleaves financial data | Deployment constraint: one organiser per Coolify instance. Server logs collision warning. Organisers cross-check totals before payout. |
| iOS Safari purges SW cache after 7 days of inactivity | In-app staleness banner + user reminder to open app before event. |
| File export is fire-and-forget — Kassen-App import not confirmed | `file_exported` batch status marks transfer as user-accepted-responsibility. Removal warning is suppressed. |
| Two registers derive different event_codes from slightly different event names | Pre-event coordination workflow on creation screen. Local collision check appends suffix. |
| Mobile-App IndexedDB quota exceeded on low-storage device | Wrap all IndexedDB writes in `QuotaExceededError` catch; show persistent banner "Gerätespeicher fast voll — exportiere jetzt"; auto-trigger file export dialog if `navigator.storage.estimate()` shows < 5 MB remaining for origin. |

---

## Migration Plan

1. Deploy `0002_purchase_dedup_index.sql` — additive, zero downtime
2. Deploy `0003_booth_event_code.sql` — add column, run backfill script, set NOT NULL; existing booths work immediately
3. Deploy Label-App (`ez-vend-labels`) — independent static deployment, no Kassen-App dependency
4. Deploy Kassen-App with Phase 1 changes — vendor list label-link UI
5. Run feasibility spike — gate Phase 2 on results
6. Deploy Kassen-App with Phase 2 changes — webcam scan
7. Deploy Mobile-App (`ez-vend-mobile`) — independent PWA deployment
8. Deploy server sync routes (Phase 3, Coolify only)
9. Phases 4 and 5 are independent follow-on deployments

**Rollback (per phase):** Each phase is additive. Roll back by deploying the previous Kassen-App / Label-App / Mobile-App artifact. Migrations are forward-only (no data loss on column add; `NOT NULL` can be relaxed).

---

## Open Questions

- **rxing spike outcome** — will determine whether Phase 2 can proceed on schedule or needs `ZXing-wasm`
- **Tesseract.js Web Worker requirement** — spike determines whether Phase 4 needs a worker thread wrapper
- **`ez-vend-server` crate** — must be created as a new workspace crate for Phase 3; initial structure TBD during Phase 3 planning
- **`version.json` endpoint** — needs to be generated as part of the Mobile-App build and deployed alongside the WASM bundle

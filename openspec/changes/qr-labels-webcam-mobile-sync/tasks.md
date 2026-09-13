## 0. Feasibility Spike (Phase Gate — complete before Phase 2)

- [ ] 0.1 Add `crates/ez-vend-prototype` to workspace `Cargo.toml`
- [ ] 0.2 Implement `qr_gen.rs` — encode `v={vendor_id}&p={price_cents}` via `qrcode` crate, render to `<canvas>` via `CanvasRenderingContext2d`
- [ ] 0.3 Implement `qr_scan.rs` — webcam frame loop (100 ms via `gloo_timers`), luma buffer to `rxing` decoder, dedup via `Performance::now()`
- [ ] 0.4 Implement `ocr_scan.rs` — `#[wasm_bindgen]` extern block targeting Tesseract.js Worker-mode API (`createWorker()`); lazy load; recognition dispatched via `spawn_local`
- [ ] 0.5 Run spike: verify rxing gate criteria (trunk build, decode < 200ms/frame, bundle delta < 500 KB) and Tesseract.js Worker-mode interop gate criteria (no compile error, Worker callable from Rust, UI not blocked)
- [ ] 0.6 Record spike outcomes: update design.md §5 (rxing result) and §14 (Tesseract.js result); if rxing fails → task 0.7; if Tesseract.js Worker-mode fails → Phase 4 deferred
- [ ] 0.7 Gate decision: if `rxing` decode > 200ms/frame or bundle delta > 500 KB → switch to `zxing-wasm`, update design.md §5, update tasks 4.6 and 5.7 accordingly
- [ ] 0.8 Add CI `trunk build` step for `crates/ez-vend-prototype` (build-only, no deploy)

## 1. Domain Model

- [ ] 1.1 Add `crates/domain/src/models/qr_label.rs` — `QrLabelPayload { vendor_id: String, price_cents: u32 }` with `encode() -> String` (`v={}&p={}`) and `decode(s: &str) -> Result<Self>` via `url::form_urlencoded`
- [ ] 1.2 Export `qr_label` module from `crates/domain/src/models/mod.rs`
- [ ] 1.3 Add `event_code: String` field to `Booth` struct in `crates/domain/src/models/booth.rs`
- [ ] 1.4 Add `onboarding_qr_shown_at: Option<DateTime<Utc>>` field to `Booth` struct — `#[serde(default, skip_serializing_if = "Option::is_none")]`; persists via existing `Booth` JSON serialisation to localStorage
- [ ] 1.5 Add `event_code_confirmed: bool` field to `Booth` struct — `#[serde(default)]`; defaults to `false`; set to `true` when organiser confirms code in one-time prompt
- [ ] 1.6 Implement `event_code` derivation function in `crates/domain/src/services/booth_service.rs` — algorithm from design.md §4 (umlaut transliteration, short-word skip, initials, MMYY suffix)
- [ ] 1.7 Add local collision check in `booth_service.rs` — check derived code against existing booth `event_code` values; append `-2` suffix if collision
- [ ] 1.8 Add `get_or_create(vendor_id: &str, booth_id: BoothId) -> Vendor` to `crates/domain/src/services/vendor_service.rs` — uniqueness key `(vendor_id, booth_id)`
- [ ] 1.9 Add `ItemSource { Manual, Scanned, ScannedEdited }` enum to `crates/domain/src/models/` — derive `Serialize`, `Deserialize`, `Clone`, `PartialEq` (must be in `domain`, not `ez-vend-ui`, so `ez-vend-mobile` can import it without taking an `ez-vend-ui` dependency)

## 2. Migrations

- [ ] 2.1 Create `crates/ez-vend-server/migrations/0002_purchase_dedup_index.sql` — `CREATE UNIQUE INDEX idx_events_purchase_dedup ON events (entity_id) WHERE event_type = 'purchase_upserted'`
- [ ] 2.2 Create `crates/ez-vend-server/migrations/0003_booth_event_code.sql` — `ALTER TABLE booths ADD COLUMN event_code TEXT`; backfill using derivation logic; `ALTER TABLE booths ALTER COLUMN event_code SET NOT NULL`

## 3. Phase 1 — Label-App (`ez-vend-labels`)

- [ ] 3.1 Add `crates/ez-vend-labels` to workspace `Cargo.toml` as a `cdylib` Leptos WASM crate
- [ ] 3.2 Add CI `trunk build` step for `crates/ez-vend-labels` (build + deploy to labels subdomain or path)
- [ ] 3.3 Create `crates/ez-vend-labels/index.html` — Trunk entry point; add `<script>` for browser capability check (WebAssembly, CanvasRenderingContext2D, window.print)
- [ ] 3.4 Implement capability check component — show plain-language error if any API missing; no WASM init attempted
- [ ] 3.5 Implement URL parameter reader — parse `?v=` (vendor_id) and `?e=` (event_code); show error if missing or malformed
- [ ] 3.6 Implement preset size selector — Klein (48×30), Mittel (64×34), Groß (70×50); all presets pass validation without warning
- [ ] 3.7 Implement custom dimension input with real-time validation — compute actual QR version from payload; hard block < 25×25 mm; soft warning < 40×25 mm; error messages per design.md §16
- [ ] 3.8 Implement QR sticker preview — render `v={vendor_id}&p={price_cents}` to `<canvas>` using `qrcode` crate; live update on dimension change
- [ ] 3.9 Implement print button — `window.print()` via `web_sys`; disabled when hard block is active; CSS `@media print` styles for label layout
- [ ] 3.10 Add "Public app URL" setting to `crates/ez-vend-ui/src/pages/settings.rs` (used to build label link URLs); disable "Label-Link" column and bulk-export button when setting is unset
- [ ] 3.11 Add vendor list "Label-Link" column to `crates/ez-vend-ui/src/pages/vendor_list.rs` — per-vendor QR modal (link as scannable QR) + "Link kopieren" clipboard button
- [ ] 3.12 Add "Alle Vendor-Links exportieren" button — download single-page HTML with all vendor QR codes; button disabled if vendor list is empty or labels-url is unset

## 4. Phase 2 — Webcam QR Scan in Kassen-App (requires Spike gate)

- [ ] 4.1 Create `crates/ez-vend-ui/src/audio.rs` — `play_scan_success_sound()` via Web Audio API
- [ ] 4.2 Add `InputMode { Manual, Scan }` enum to `crates/ez-vend-ui/src/pages/checkout.rs`
- [ ] 4.3 Add `source: ItemSource` to `CheckoutItem` and `StoredCheckoutItem` in `checkout.rs` — include in JSON serialisation using actual localStorage key `ez-vend-checkout-draft`
- [ ] 4.4 Add `ItemSource::ScannedEdited` transition — when price of a `Scanned` item is manually edited, update `source` to `ScannedEdited`
- [ ] 4.5 Add toolbar toggle "Eingabe | QR-Scan" to checkout page — keyboard shortcut `S`; mode persists in localStorage key `input_mode`
- [ ] 4.6 Implement QR scan component in checkout — webcam via `getUserMedia`, `<video>` + hidden `<canvas>`, 100 ms frame loop via `gloo_timers`, `rxing` decode (or `zxing-wasm` per spike outcome from 0.7); stop `MediaStream` tracks when InputMode switches away from Scan
- [ ] 4.7 Implement dedup logic — `HashMap<String, f64>` keyed on full payload string with `Performance::now()`; 2000 ms window; on suppression: highlight most recent matching row (~300 ms CSS pulse), no sound
- [ ] 4.8 Add 📷 icon to `Scanned` cart rows; ✎ icon to `ScannedEdited` rows
- [ ] 4.9 Add "QR-Code für Mobile" button to checkout — renders onboarding QR (`ez-booth://onboard?e={event_code}&n={name}`); sets `onboarding_qr_shown_at` on `Booth` and persists
- [ ] 4.10 Add event_code lock logic to event creation/edit UI — disabled when `onboarding_qr_shown_at` is set; warning on edit attempt before lock; "override lock" action in mismatch-recovery flow with strong confirmation dialog
- [ ] 4.11 Show `event_code` prominently on event creation confirmation screen with sharing instruction and onboarding QR
- [ ] 4.12 Add one-time `event_code_confirmed` prompt — on Kassen-App startup, if active booth has `event_code_confirmed == false`, show prompt; on confirm set `true` and persist
- [ ] 4.13 Pause scan frame loop on Page Visibility API `hidden` event; resume on `visible`

## 5. Phase 3 — Mobile-App (`ez-vend-mobile`)

- [ ] 5.1 Add `crates/ez-vend-mobile` to workspace `Cargo.toml` as a `cdylib` Leptos WASM crate (depends only on `crates/domain`)
- [ ] 5.2 Add CI `trunk build` step for `crates/ez-vend-mobile` (build + PWA deploy)
- [ ] 5.3 Create `manifest.json` — `name`, `icons`, `display: standalone`
- [ ] 5.4 Create manually written `sw.js` — Cache-First strategy, versioned cache name, `skipWaiting()` + `clients.claim()`; do NOT precache `version.json`
- [ ] 5.5 Emit `dist/version.json` at build time — `{ "version": "<git_sha_short>" }` via `build.rs` or Trunk post-build hook; include in deployment artifact at `/version.json` relative to app root
- [ ] 5.6 Embed build-version constant; implement startup version check fetching `/version.json`; show staleness banner if fetch fails and > 6 days since last open
- [ ] 5.7 Implement startup IndexedDB availability check — if unavailable (private mode, quota error), show blocking error: "Diese App kann in diesem Modus keine Daten speichern. Bitte öffne sie in einem normalen Browserfenster."; wrap all subsequent IndexedDB writes in `QuotaExceededError` handler with persistent export prompt
- [ ] 5.8 Implement event onboarding screen — QR scan (`ez-booth://onboard?e=…&n=…`) with "Enter code manually" fallback (text fields for `event_code` + name); show confirmation dialog if QR scan would overwrite a non-empty existing `event_code` value
- [ ] 5.9 Implement event list — IndexedDB; active-event selection; event detail with batch status bar ("N Batches ausstehend · zuletzt synchronisiert HH:MM")
- [ ] 5.10 Implement scan screen per event — webcam + `rxing` QR decode (or `zxing-wasm` per spike outcome from 0.7); decoded items stored in IndexedDB under active event
- [ ] 5.11 Implement batch state machine — `pending → server_uploaded | file_exported`; `file_exported` suppresses removal warning
- [ ] 5.12 Implement file export — filename `ez-booth_{event_code}_{client_id8}_{YYYY-MM-DD}.json`; transitions batch to `file_exported`
- [ ] 5.13 Implement event removal safeguard — warn if any batch is `pending`; show deletion-time confirmation if any batch is `file_exported` but not `server_uploaded`: "Du hast Daten exportiert, aber nicht über den Server synchronisiert. Wenn die Datei nicht importiert wurde, gehen die Daten verloren. Trotzdem löschen?"
- [ ] 5.14 Implement "Verlauf anzeigen" disclosure — full batch list with individual statuses
- [ ] 5.15 Implement `client_id` — UUID v4, generated on first start, stored in IndexedDB

## 6. Phase 3 — Server Sync (`ez-vend-server`)

- [ ] 6.1 Create `crates/ez-vend-server` workspace crate (if not yet exists) — add to workspace `Cargo.toml`
- [ ] 6.2 Implement `POST /api/sync` in `crates/ez-vend-server/src/routes/sync.rs` — accept `{ purchases, client_id }`; insert via `ON CONFLICT DO NOTHING` using dedup index; log warning when two `client_id` values use same `event_code` in same month; guarantee `next_sequence > since` on every non-empty response
- [ ] 6.3 Implement `GET /api/sync?since={seq}` — return `purchase_upserted` events with `sequence > since`, up to 500; include `next_sequence`
- [ ] 6.4 Register sync routes in `crates/ez-vend-server/src/routes/mod.rs`
- [ ] 6.5a Implement Sync button UI shell in Kassen-App — split-status display "Upload: OK / Download: fehlgeschlagen" with independent indicators; `last_upload_ok` as in-memory signal (not localStorage)
- [ ] 6.5b Implement `POST /api/sync` client call — initialise `last_upload_ok = false`; on 200 set `true`; on error show "Upload: fehlgeschlagen"
- [ ] 6.5c Implement paginated `GET /api/sync` client loop — loop while `purchases.length == 500`; update `last_sync_sequence` in localStorage per page; stop after 20 iterations and surface sync error if limit reached
- [ ] 6.6 Implement Kassen-App file import — file picker, JSON parse, UUID dedup against `completed_purchases`, vendor auto-creation via `get_or_create`
- [ ] 6.7 Implement Kassen-App retry logic — skip POST on next Sync tap if `last_upload_ok` is `true` in current page session

## 7. Phase 4 — Handwriting OCR (DEFERRED — requires spike outcome)

- [ ] 7.1 Extend `InputMode` with `OcrScan` variant
- [ ] 7.2 Extend toolbar to three-way toggle "Eingabe | QR-Scan | Handschrift"
- [ ] 7.3 Implement `ocr_scan` component — Tesseract.js lazy load, Worker-mode interop via `wasm-bindgen` extern
- [ ] 7.4 Implement confidence threshold logic — ≥ 85% auto-accept; < 85% show confirmation dialog
- [ ] 7.5 Implement confirmation dialog — cropped image frame + editable price field + confidence % + Übernehmen/Abbrechen
- [ ] 7.6 Implement OCR failure state — toast "Beschriftung nicht lesbar" + focus to manual price field
- [ ] 7.7 Implement optional server OCR endpoint `POST /api/ocr` (Coolify only) — accept image frame, return `{ text, confidence }`; wire to settings toggle

## 8. Phase 5 — Organiser-App (DEFERRED)

- [ ] 8.1 Add `crates/ez-vend-organizer` to workspace `Cargo.toml`
- [ ] 8.2 Implement event creation with central `event_code` derivation (same algorithm as Kassen-App)
- [ ] 8.3 Implement vendor list management — create, edit, archive vendors
- [ ] 8.4 Implement export to Kassen-App import format — preserving `event_code`

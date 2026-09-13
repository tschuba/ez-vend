## Why

Cashiers at flea-market events must manually type a vendor ID and price for every item, slowing queues and introducing transcription errors. Replacing manual entry with QR-sticker scanning (cashier) and mobile-device scanning (helpers) eliminates the error class and significantly increases checkout throughput, while maintaining the offline-first guarantee that the core flow works without internet access.

## What Changes

- **New crate `ez-vend-labels`** — standalone Leptos WASM app; vendors open a link, configure sticker dimensions, and print QR labels from any browser before the event
- **New crate `ez-vend-mobile`** — standalone Leptos WASM PWA; helpers scan items on a phone during the event and batch-sync purchases to registers (file export or server upload)
- **New crate `ez-vend-prototype`** — developer spike to validate `rxing` WASM compilation and Tesseract.js interop before Phase 2 / Phase 4 implementation begins
- **`Booth` model gains `event_code: String`** — a cross-device event identifier derived from event name + date; required for label links, mobile onboarding, and sync routing; migration backfills existing booths
- **Checkout page gains `InputMode` + `ItemSource`** — toggle between Manual / Scan modes; scanned items display an icon; `ScannedEdited` tracks price corrections; duplicate suppression with visual feedback
- **Vendor list page gains label-link distribution UI** — per-vendor QR modal, copy button, and bulk HTML export
- **New server routes `POST /api/sync` + `GET /api/sync`** — unauthenticated batch upload/download for mobile sync (Coolify deployment only); paginated, idempotent via UUID dedup
- **New migration `0002_purchase_dedup_index.sql`** — partial unique index on `events(entity_id)` for purchase dedup
- **New migration `0003_booth_event_code.sql`** — add and backfill `event_code` column on `booths`
- **Deferred — `handwriting-ocr`** (Phase 4): Tesseract.js OCR for handwritten price labels; requires spike result
- **Deferred — `organiser-app`** (Phase 5): centralised pre-event vendor management

## Capabilities

### New Capabilities

- `qr-label-printing`: Vendors open a browser link to configure label dimensions and print QR stickers. Covers the Label-App crate, label link format, size validation (hard block + soft warning), capability check, and link distribution from the vendor list page.
- `webcam-qr-scan`: Cashier activates Scan mode on the checkout page to decode QR stickers via the register's webcam. Covers InputMode, ItemSource, frame loop, 2-second dedup, audio feedback, and item icons.
- `mobile-purchase-sync`: Helper uses a phone PWA to scan items during the event and sync purchases to a register via file export or server upload. Covers the Mobile-App crate, event onboarding, IndexedDB storage, batch state machine, sync UI, and PWA offline support.
- `event-code`: Cross-device event identifier on the `Booth` model used by label links, mobile onboarding, and sync routing. Covers derivation algorithm, lock trigger, collision avoidance, coordination workflow, and migration.
- `handwriting-ocr`: **(Deferred — Phase 4, requires spike)** OCR recognition of handwritten price labels via Tesseract.js, with confidence-based auto-accept / confirmation dialog, and optional server-side fallback.
- `organiser-app`: **(Deferred — Phase 5)** Centralised pre-event vendor and event management via a dedicated Leptos WASM crate; generates `event_code` centrally and exports data for Kassen-App import.

### Modified Capabilities

<!-- No existing specs — all capabilities above are new -->

## Impact

**Code**
- `crates/domain/src/models/booth.rs` — `event_code: String` field
- `crates/domain/src/models/qr_label.rs` — new; `QrLabelPayload` encode/decode
- `crates/domain/src/services/vendor_service.rs` — `get_or_create(vendor_id, booth_id)`
- `crates/domain/src/services/booth_service.rs` — `event_code` derivation + collision check
- `crates/ez-vend-ui/src/pages/checkout.rs` — `InputMode`, `ItemSource`, scan component
- `crates/ez-vend-ui/src/pages/vendor_list.rs` — label-link distribution UI
- `crates/ez-vend-ui/src/pages/settings.rs` — public app URL setting
- `crates/ez-vend-ui/src/audio.rs` — new; scan success sound
- `crates/ez-vend-server/src/routes/sync.rs` — new; POST + GET /api/sync
- New workspace crates: `ez-vend-labels`, `ez-vend-mobile`, `ez-vend-prototype`

**Migrations** (server / Coolify deployments only)
- `0002_purchase_dedup_index.sql` — partial unique index
- `0003_booth_event_code.sql` — event_code column + backfill

**Dependencies** (subject to spike validation)
- `rxing` (pure Rust QR decode, `wasm` feature) — primary; fallback: `zxing-wasm`
- `qrcode` (pure Rust QR encode)
- `gloo-timers` (frame loop)
- Tesseract.js (Phase 4 only, via `<script>` tag — not a Cargo dependency)

**Deployment constraint**
Each organiser must use a dedicated Coolify instance. Shared multi-tenant server deployments are unsupported — `event_code` uniqueness is not enforced server-side and collisions silently interleave financial data.

# Deferred — Phase 4 (requires feasibility spike outcome)

This capability is deferred until the feasibility spike (`crates/ez-vend-prototype`) confirms that Tesseract.js is callable from Leptos WASM without blocking the UI thread. If the spike fails, this phase is deferred further until a pure-Rust OCR crate is viable, or falls back to server-side `/api/ocr` (Coolify only).

---

## ADDED Requirements

### Requirement: OcrScan mode in InputMode
Phase 4 SHALL extend `InputMode` with a third variant `OcrScan`. The checkout toolbar SHALL become a three-way toggle: "Eingabe | QR-Scan | Handschrift". Keyboard shortcut `S` cycles through modes.

#### Scenario: Cashier switches to OCR mode
- **WHEN** the cashier clicks "Handschrift" in the toolbar or presses `S` twice from Manual
- **THEN** `InputMode` is `OcrScan` and the OCR webcam feed is activated

---

### Requirement: Tesseract.js lazy loading
Tesseract.js (~15 MB model) SHALL be loaded lazily on the first OCR attempt to avoid a large download at startup. It SHALL be injected via a `<script>` tag in `index.html` and called from Rust via a `#[wasm_bindgen]` extern block. The recognition call MUST be dispatched via `wasm_bindgen_futures::spawn_local` to avoid blocking the UI thread. If `spawn_local` is insufficient (blocks despite async), a Web Worker MUST be used.

#### Scenario: First OCR attempt triggers model load
- **WHEN** the cashier switches to OCR mode for the first time
- **THEN** Tesseract.js begins downloading and a loading indicator is shown

#### Scenario: OCR call does not block UI
- **WHEN** the OCR recognition call is in progress
- **THEN** the rest of the checkout UI remains responsive

---

### Requirement: Confidence-based acceptance
The OCR result SHALL be processed as follows:
- Confidence ≥ 85% AND the result parses as a valid German decimal price → auto-accept: add item to cart without dialog
- Confidence < 85% OR result fails price validation → show confirmation dialog

#### Scenario: High-confidence OCR result
- **WHEN** OCR returns "3,50" with 91% confidence and it parses as a valid price
- **THEN** the item is added to the cart automatically without showing a dialog

#### Scenario: Low-confidence OCR result
- **WHEN** OCR returns "3,5O" (letter O) with 62% confidence
- **THEN** the confirmation dialog is shown

---

### Requirement: OCR confirmation dialog
The confirmation dialog SHALL display:
- The captured image frame cropped to the label area
- An editable price field pre-filled with the parsed OCR result
- The confidence percentage in muted secondary text
- "Übernehmen" and "Abbrechen" buttons

On "Übernehmen", the value in the price field MUST be re-validated as a parseable German decimal price before being accepted.

#### Scenario: Cashier corrects OCR result in dialog
- **WHEN** the dialog shows "3,50" but the actual price is "4,50"
- **THEN** the cashier edits the field to "4,50" and taps "Übernehmen" to add the correct item

#### Scenario: Invalid price in dialog rejected
- **WHEN** the cashier clears the price field and taps "Übernehmen"
- **THEN** an inline error is shown and the item is not added

---

### Requirement: OCR failure fallback
If OCR returns an empty result or a result that fails all price validation after a fixed timeout, the system SHALL display a toast: *"Beschriftung nicht lesbar — Preis manuell eingeben."* Input focus MUST move to the manual price field. `InputMode` SHALL remain `OcrScan` so the cashier can retry without switching modes.

#### Scenario: Illegible label
- **WHEN** the cashier holds a torn or faded label to the camera and OCR returns an empty result
- **THEN** the toast is shown, focus moves to the manual price field, and the cashier can type the price

---

### Requirement: Optional server-side OCR endpoint
When enabled in settings, the Kassen-App SHALL send the captured image frame to `POST /api/ocr` instead of running Tesseract.js locally. This endpoint is available in Coolify deployments only. The response format MUST include `{ text: String, confidence: f64 }`. The same confidence threshold and dialog logic applies.

#### Scenario: Server OCR enabled in settings

- **WHEN** server OCR is enabled and the cashier holds a label to the camera
- **THEN** the image is sent to /api/ocr and the result is processed with the same confidence logic

---

### Requirement: Feasibility spike validates Tesseract.js interop

Before Phase 4 implementation begins, the `ez-vend-prototype` spike SHALL demonstrate that Tesseract.js is callable from Leptos WASM and that recognition does not block the UI thread.

#### Scenario: Tesseract.js callable without compile error

- **WHEN** the `#[wasm_bindgen]` extern block for Tesseract.js is compiled in `crates/ez-vend-prototype`
- **THEN** `trunk build` completes without errors and `recognize()` is callable from Rust

#### Scenario: OCR quality meets threshold on clean handwriting

- **WHEN** clearly handwritten "42" and "3,50" on white paper are held to the webcam
- **THEN** Tesseract.js returns confidence ≥ 85% and the result is labelled AUTO-ACCEPT

#### Scenario: OCR shows NEEDS CONFIRMATION on messy handwriting

- **WHEN** small or messy handwriting is held to the webcam
- **THEN** confidence is < 85% and the result is labelled NEEDS CONFIRMATION in red

#### Scenario: UI remains responsive during OCR

- **WHEN** the OCR recognition call is in progress via `spawn_local`
- **THEN** the rest of the spike UI (buttons, inputs) remains interactive and is not frozen

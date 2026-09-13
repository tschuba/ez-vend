## ADDED Requirements

### Requirement: InputMode toggle in checkout toolbar
The checkout page SHALL display a toggle in the toolbar that switches between "Eingabe" (Manual) and "QR-Scan" (Scan) modes. The keyboard shortcut `S` MUST switch between the two modes. The selected mode MUST persist across page reloads via localStorage key `input_mode`.

#### Scenario: Switching to Scan mode via button
- **WHEN** the cashier clicks the "QR-Scan" segment of the toolbar toggle
- **THEN** `InputMode` changes to `Scan` and the webcam feed is activated

#### Scenario: Switching to Scan mode via keyboard shortcut
- **WHEN** the cashier presses `S` while in Manual mode
- **THEN** `InputMode` changes to `Scan`

#### Scenario: Mode persists across reload
- **WHEN** the cashier is in Scan mode and the page is reloaded
- **THEN** the page opens in Scan mode (loaded from `input_mode` in localStorage)

---

### Requirement: QR code decoding from webcam feed
In Scan mode, the checkout page SHALL activate the device webcam and run a frame loop at approximately 10 fps (100 ms interval via `gloo_timers::callback::Interval`). Each frame MUST be extracted to a hidden `<canvas>` (cropped to the viewfinder area) and passed as a luma buffer to the `rxing` decoder. The scan SHALL stop when the component unmounts.

#### Scenario: Valid QR sticker held to webcam
- **WHEN** the cashier holds a sticker with payload `v=42&p=300` to the webcam
- **THEN** vendor ID 42 and price €3.00 are decoded within 500 ms and added to the cart

#### Scenario: Non-QR image held to webcam
- **WHEN** the cashier holds a plain piece of paper to the webcam
- **THEN** no item is added and no error is shown (silent no-op)

#### Scenario: Camera permission denied

- **WHEN** the browser denies camera access
- **THEN** Scan mode is unavailable and a message is shown: "Kamerazugriff verweigert — bitte Browserberechtigungen prüfen."

#### Scenario: Webcam stopped when switching away from Scan mode

- **WHEN** the cashier switches from Scan mode to Manual mode via the toolbar toggle
- **THEN** `MediaStream.getTracks().forEach(t => t.stop())` is called and the browser camera indicator turns off

#### Scenario: Webcam restarted when switching back to Scan mode

- **WHEN** the cashier switches from Manual mode back to Scan mode
- **THEN** `getUserMedia` is called again and the webcam feed resumes

#### Scenario: Frame loop paused when tab is hidden

- **WHEN** the cashier switches to another browser tab or app while Scan mode is active
- **THEN** the frame loop is paused via the Page Visibility API (`document.hidden === true`) and the camera stream is not actively read

#### Scenario: Frame loop resumed when tab becomes visible

- **WHEN** the cashier returns to the checkout tab while Scan mode is active
- **THEN** the frame loop resumes

---

### Requirement: Item source tracking
Every `CheckoutItem` and `StoredCheckoutItem` SHALL carry a `source: ItemSource` field. `ItemSource` is an enum with three variants: `Manual`, `Scanned`, `ScannedEdited`. Scanned items MUST display a 📷 icon in the cart. `ScannedEdited` items MUST display a ✎ icon. The `source` field MUST be serialised in `StoredCheckoutItem` so icons persist across reloads.

#### Scenario: Item added via QR scan
- **WHEN** a QR sticker is decoded and added to the cart
- **THEN** the cart row shows a 📷 icon and `source` is `Scanned`

#### Scenario: Scanned item price edited by cashier
- **WHEN** the cashier manually edits the price of a previously scanned item
- **THEN** the cart row icon changes to ✎ and `source` transitions to `ScannedEdited`

#### Scenario: Manually entered item has no icon
- **WHEN** an item is added via manual vendor ID + price entry
- **THEN** no source icon is displayed in the cart row

#### Scenario: Source icon persists across reload
- **WHEN** the cashier reloads the page after scanning an item
- **THEN** the 📷 icon is still displayed on the scanned item

---

### Requirement: Duplicate scan suppression
A QR payload decoded within 2000 ms of the same payload's last decode MUST be suppressed (not added to the cart again). The dedup key is the **full payload string** (e.g. `v=42&p=300`) — not just the vendor ID. The dedup clock MUST use `web_sys::Performance::now()` (not `std::time::Instant`, which is unavailable in WASM). The suppression MUST trigger a visual pulse highlight (~300 ms CSS animation) on the **most recent cart row whose full decoded payload matches**. The scan success sound MUST NOT play on a suppressed duplicate.

#### Scenario: Same sticker scanned twice quickly

- **WHEN** the cashier scans the same QR sticker (payload `v=42&p=300`) twice within 500 ms
- **THEN** only one cart row is added; the most recent cart row with payload `v=42&p=300` is highlighted; no sound plays

#### Scenario: Same sticker scanned after dedup window
- **WHEN** the cashier scans the same QR sticker 2500 ms after the first scan
- **THEN** a second cart row is added (treated as a new item)

---

### Requirement: Scan success audio feedback
A scan success sound SHALL play when a new (non-duplicate) QR payload is decoded and added to the cart. The sound is implemented in `crates/ez-vend-ui/src/audio.rs` as `play_scan_success_sound()`.

#### Scenario: Successful new scan
- **WHEN** a QR sticker is decoded and the item is not a duplicate
- **THEN** the scan success sound plays once

#### Scenario: Duplicate suppressed scan
- **WHEN** a QR sticker is decoded within the 2000 ms dedup window
- **THEN** no sound plays

---

### Requirement: Mobile onboarding QR in checkout
The checkout page SHALL display a "QR-Code für Mobile" button that renders the mobile onboarding QR code (`ez-booth://onboard?e={event_code}&n={url_encoded_event_name}`). When this QR is first rendered, the `event_code` field MUST be locked (see `event-code` spec).

#### Scenario: Generating the onboarding QR
- **WHEN** the cashier clicks "QR-Code für Mobile"
- **THEN** the onboarding QR is displayed and `event_code` editing is locked for this booth

---

### Requirement: Feasibility spike validates rxing in WASM
Before Phase 2 implementation begins, the `ez-vend-prototype` spike SHALL demonstrate that `rxing` compiles to WASM and meets performance criteria. The spike MUST be run in Chrome with a live webcam.

#### Scenario: rxing WASM compilation

- **WHEN** `rxing` is added to `crates/ez-vend-prototype/Cargo.toml` with the `wasm` feature
- **THEN** `trunk build` completes without errors

#### Scenario: Live QR decode within 500 ms

- **WHEN** a printed QR sticker is held to the webcam
- **THEN** the decoded payload appears in the DOM within 500 ms

#### Scenario: Decode latency per frame

- **WHEN** the spike logs decode timing for each frame
- **THEN** reported decode time is < 100 ms per frame

#### Scenario: Generated QR is scannable

- **WHEN** vendor_id=42 and price_cents=300 are entered in the QR Generator and the Generate button is clicked
- **THEN** the resulting canvas QR code is scannable by a smartphone camera app and yields "v=42&p=300"

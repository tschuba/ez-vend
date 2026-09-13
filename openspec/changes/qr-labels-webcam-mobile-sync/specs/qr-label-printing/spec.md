## ADDED Requirements

### Requirement: labels-url setting must be configured before label links are available

The Kassen-App SHALL require the "Public app URL" (`labels-url`) to be configured in Settings before label links can be generated or copied. When `labels-url` is unset, the "Label-Link" column, "Link kopieren" buttons, and "Alle Vendor-Links exportieren" button MUST be disabled, and a guidance message MUST direct the organiser to configure the setting.

#### Scenario: labels-url not configured

- **WHEN** the organiser opens the vendor list without having configured the Public app URL in Settings
- **THEN** the "Label-Link" column shows a "URL nicht konfiguriert — Einstellungen öffnen" prompt instead of QR icons and copy buttons, and the bulk-export button is disabled

#### Scenario: labels-url configured

- **WHEN** the organiser has configured `https://labels.example.com` as the Public app URL
- **THEN** the label link for vendor 42 resolves to `https://labels.example.com/?v=42&e={event_code}` and all label-link controls are enabled

#### Scenario: Bulk export with empty vendor list

- **WHEN** the organiser clicks "Alle Vendor-Links exportieren" with zero vendors configured
- **THEN** the button is disabled and a message is shown: "Keine Verkäufer vorhanden."

---

### Requirement: Label-App browser capability check
The Label-App SHALL verify that the browser supports `WebAssembly`, `CanvasRenderingContext2D`, and `window.print` before initialising the WASM module. If any capability is missing, a plain-language error message MUST be displayed and no WASM initialisation SHALL be attempted.

#### Scenario: Unsupported browser opens the Label-App
- **WHEN** a vendor opens the Label-App URL in a browser without WebAssembly support
- **THEN** the app displays "Dein Browser unterstützt diese Funktion nicht. Bitte verwende Chrome 90+, Safari 15+ oder Firefox 90+." before any WASM is loaded

#### Scenario: Supported browser opens the Label-App
- **WHEN** a vendor opens the Label-App URL in a supported browser
- **THEN** the WASM module initialises and the label configuration UI is displayed

---

### Requirement: Label-App reads vendor and event from URL
The Label-App SHALL read `vendor_id` and `event_code` from the URL query parameters `?v=` and `?e=`. On success, the vendor ID MUST be displayed. If either parameter is missing or malformed, a user-friendly error message MUST be shown.

#### Scenario: Valid URL parameters
- **WHEN** a vendor opens `{labels-url}/?v=42&e=FM-0526`
- **THEN** the app displays the vendor ID 42 and the configured event name

#### Scenario: Missing vendor_id parameter
- **WHEN** a vendor opens the Label-App URL with `?e=FM-0526` but no `?v=`
- **THEN** the app displays an error: "Ungültiger Link. Bitte wende dich an den Veranstalter."

---

### Requirement: Preset label sizes
The Label-App SHALL offer three preset label sizes that can be selected with one tap: Klein (48×30 mm), Mittel (64×34 mm), Groß (70×50 mm). All presets MUST pass label size validation without warning.

#### Scenario: Selecting a preset
- **WHEN** the vendor selects the "Mittel" preset
- **THEN** the dimension fields are set to 64 mm × 34 mm and no validation warning is shown

---

### Requirement: Custom label dimension validation
The Label-App SHALL validate custom label dimensions in real time on every change. Validation computes the minimum required label size against the **actual QR payload** being encoded (not a fixed example).

Hard block (printing disabled): the QR code region must be ≥ 25×25 mm.
Soft warning (printing allowed with acknowledgement): total label must be ≥ 40×25 mm.

#### Scenario: Label too small to scan (hard block)
- **WHEN** a vendor enters custom dimensions where the QR code region would be < 25×25 mm
- **THEN** the print button is disabled and the message is shown: "Der QR-Code-Bereich muss mindestens 25×25 mm groß sein, damit er zuverlässig gescannt werden kann. Druck gesperrt."

#### Scenario: Label below suggested minimum (soft warning)
- **WHEN** a vendor enters dimensions where the total label is < 40×25 mm but the QR region is ≥ 25×25 mm
- **THEN** a dismissible warning is shown: "Etiketten dieser Größe werden möglicherweise nicht zuverlässig gescannt. Du kannst trotzdem drucken und das Risiko akzeptieren."
- **THEN** the print button remains enabled

#### Scenario: Warning dismissed persists for session

- **WHEN** a vendor dismisses the soft warning
- **THEN** the warning is not shown again for the remainder of the browser session, regardless of the exact dimensions entered, as long as dimensions remain in the soft-warning tier (< 40×25 mm total but ≥ 25×25 mm QR region)

#### Scenario: Worst-case payload validation
- **WHEN** vendor ID is 999 and a price of €9999.99 (999999 cents) is being encoded
- **THEN** the minimum QR region is computed for payload `v=999&p=999999` (15 chars, QR version 2) and the 25×25 mm hard block threshold is applied correctly

---

### Requirement: QR sticker preview and print
The Label-App SHALL render a live preview of the QR sticker on a `<canvas>` element. The QR payload MUST use the format `v={vendor_id}&p={price_cents}`. The print button SHALL trigger `window.print()` using CSS `@media print` styles.

#### Scenario: Generating and printing a sticker
- **WHEN** a vendor enters price 300 (€3.00) and taps "Drucken"
- **THEN** the browser print dialog opens with the sticker layout scaled to the selected label dimensions

#### Scenario: Zero or invalid price
- **WHEN** a vendor enters 0 or a non-numeric price
- **THEN** the print button is disabled and an inline validation error is shown

---

### Requirement: Label link distribution on vendor list page
The vendor list page in the Kassen-App SHALL display a "Label-Link" column for each vendor with:
- A QR code icon that opens a modal showing the vendor's label link as a scannable QR code
- A "Link kopieren" button that copies the label link to the clipboard

An "Alle Vendor-Links exportieren" button SHALL download a single-page HTML file containing all vendor QR codes, suitable for printing as a check-in reference sheet.

#### Scenario: Copying a single vendor link
- **WHEN** the cashier clicks "Link kopieren" for vendor 42
- **THEN** the URL `{labels-url}/?v=42&e={event_code}` is copied to the clipboard

#### Scenario: Viewing a vendor QR code
- **WHEN** the cashier taps the QR icon for vendor 42
- **THEN** a modal opens showing a scannable QR code for that vendor's label link

#### Scenario: Bulk export of all vendor links
- **WHEN** the cashier clicks "Alle Vendor-Links exportieren"
- **THEN** a single HTML file is downloaded containing one QR code per vendor, suitable for printing

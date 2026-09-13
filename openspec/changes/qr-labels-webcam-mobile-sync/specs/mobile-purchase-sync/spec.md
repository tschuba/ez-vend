## ADDED Requirements

### Requirement: Mobile-App PWA install
The Mobile-App (`crates/ez-vend-mobile`) SHALL include a `manifest.json` with `name`, `icons`, and `display: standalone` to enable Add-to-Home-Screen on Android and iOS. A manually written `sw.js` with Cache-First strategy, a versioned cache name, `skipWaiting()`, and `clients.claim()` MUST be included so the app functions fully offline after first load.

#### Scenario: Helper installs Mobile-App on Android
- **WHEN** a helper opens the Mobile-App URL in Chrome on Android and is prompted to install
- **THEN** the app is added to the home screen and opens in standalone mode

#### Scenario: App loads offline after initial install
- **WHEN** a helper opens the Mobile-App with no internet connection after a previous successful load
- **THEN** the app loads from the Service Worker cache and is fully functional

---

### Requirement: IndexedDB availability check on startup

On startup the Mobile-App SHALL verify that IndexedDB is available and not in a restricted mode. If IndexedDB is unavailable (e.g. private browsing on iOS), a blocking error MUST be shown. All subsequent IndexedDB write paths MUST catch `QuotaExceededError` and surface a persistent export prompt.

#### Scenario: IndexedDB unavailable (private mode)

- **WHEN** a helper opens the Mobile-App in a private/incognito browser window on iOS
- **THEN** a blocking message is shown: "Diese App kann in diesem Modus keine Daten speichern. Bitte öffne sie in einem normalen Browserfenster." and no purchases can be recorded

#### Scenario: Storage quota exceeded during scan

- **WHEN** a helper scans a purchase and the IndexedDB write fails with a `QuotaExceededError`
- **THEN** a persistent banner is shown: "Gerätespeicher fast voll — exportiere jetzt" and the file export dialog is triggered automatically

---

### Requirement: Cache staleness detection
On each startup the Mobile-App SHALL compare an embedded build-version constant against `/version.json` at the app root. If the fetch fails (device is offline) and the last-opened timestamp is more than 6 days ago, a banner MUST be displayed. The staleness threshold is strictly greater than 6 days (not ≥ 6).

#### Scenario: App used after 7 days of inactivity (offline)

- **WHEN** the Mobile-App starts on a device that has been offline for 8 days since the last open
- **THEN** a banner is shown: "Dein Offline-Speicher könnte veraltet sein. Vor dem Event kurz neu laden."

#### Scenario: App used within 6 days, online

- **WHEN** the Mobile-App starts and the device is online with a fresh cache
- **THEN** no staleness banner is shown (check succeeds via HTTP, day count irrelevant)

#### Scenario: Exactly 6 days offline — no banner

- **WHEN** the Mobile-App starts offline and the last-opened timestamp is exactly 6 days ago
- **THEN** no staleness banner is shown (threshold is strictly > 6 days)

#### Scenario: 7 days offline — banner shown

- **WHEN** the Mobile-App starts offline and the last-opened timestamp is 7 days ago
- **THEN** the staleness banner is shown

---

### Requirement: Event onboarding — QR scan
The Mobile-App SHALL allow helpers to onboard to an event by scanning the Kassen-App's onboarding QR code (`ez-booth://onboard?e={event_code}&n={name}`). The scanned `event_code` and `event_name` MUST be stored in IndexedDB and the event MUST appear in the event list.

#### Scenario: Helper scans onboarding QR
- **WHEN** a helper scans the onboarding QR from the Kassen-App
- **THEN** the event is added to the Mobile-App event list with the correct event_code and name

---

### Requirement: Event onboarding — manual entry fallback
The Mobile-App onboarding screen SHALL provide a "Enter code manually" fallback below the QR scanner. Tapping it reveals a text field for `event_code` and an event name field. This path MUST produce the same result as the QR path.

#### Scenario: Helper enters event code manually
- **WHEN** a helper taps "Enter code manually" and types `FM-0526` and event name "Flohmarkt Mai"
- **THEN** the event is added to the Mobile-App event list with those values

---

### Requirement: Item scanning in Mobile-App
The Mobile-App SHALL present a scan screen per active event that decodes QR stickers via webcam (same frame loop and dedup logic as the Kassen-App). Decoded items MUST be stored in IndexedDB under the active event.

#### Scenario: Helper scans an item
- **WHEN** the helper scans a sticker `v=42&p=300` while event FM-0526 is active
- **THEN** a purchase record `{vendor_id: 42, price_cents: 300, event_code: "FM-0526"}` is added to IndexedDB

---

### Requirement: Batch sync state machine
Purchases in the Mobile-App are grouped into batches. Each batch SHALL have a UUID and follow this state machine:

- `pending` → initial state after creation
- `server_uploaded` → after `POST /api/sync` returns 200
- `file_exported` → after the user triggers a file download

`file_exported` batches MUST NOT be included in subsequent `POST /api/sync` payloads. `server_uploaded` batches MUST NOT be re-uploaded. `file_exported` suppresses the event-removal warning — the helper has accepted transfer responsibility.

#### Scenario: Batch exported to file
- **WHEN** the helper taps "Exportieren" and the file downloads successfully
- **THEN** the batch status transitions to `file_exported` and is excluded from future uploads

#### Scenario: Batch uploaded to server
- **WHEN** the helper taps "Synchronisieren" and the server returns 200
- **THEN** the batch status transitions to `server_uploaded` and is excluded from future uploads

#### Scenario: Already-uploaded batch not re-sent
- **WHEN** the helper taps "Synchronisieren" a second time after a successful upload
- **THEN** the server_uploaded batch is not included in the POST body

---

### Requirement: Batch status visibility
The event detail screen in the Mobile-App SHALL display: "N Batches ausstehend · zuletzt synchronisiert HH:MM" (or "nie" if never synced). When all batches are in `server_uploaded` or `file_exported` state, the status SHALL show "Alles synchronisiert ✓". A "Verlauf anzeigen" disclosure SHALL reveal the full batch list with individual statuses.

#### Scenario: All batches synced
- **WHEN** all batches for an event are in server_uploaded or file_exported state
- **THEN** the status bar shows "Alles synchronisiert ✓"

#### Scenario: Pending batches displayed
- **WHEN** 3 batches are pending and the last sync was at 14:30
- **THEN** the status bar shows "3 Batches ausstehend · zuletzt synchronisiert 14:30"

---

### Requirement: File export with naming convention
The Mobile-App "Exportieren" function SHALL download pending purchases as a `.json` file. The filename MUST follow the convention `ez-booth_{event_code}_{client_id_short8}_{YYYY-MM-DD}.json` set via the `<a download="...">` attribute.

#### Scenario: Exporting purchases
- **WHEN** the helper taps "Exportieren" for event FM-0526 on 2026-05-15
- **THEN** a file named `ez-booth_FM-0526_a1b2c3d4_2026-05-15.json` is downloaded

---

### Requirement: Event removal safeguard
The Mobile-App SHALL warn the helper before removing an event if any batches remain in `pending` state. For events with `file_exported` (but not `server_uploaded`) batches, a deletion-time confirmation MUST be shown, distinct from the export-time acknowledgement. Events with all batches in `server_uploaded` state MUST be removable without warning.

#### Scenario: Removing event with pending batches

- **WHEN** the helper attempts to remove an event that has 2 pending batches
- **THEN** a warning is shown: "Dieser Event hat noch 2 nicht übertragene Batches. Trotzdem entfernen?"

#### Scenario: Removing event with only file_exported batches

- **WHEN** the helper removes an event where all batches are `file_exported` but none are `server_uploaded`
- **THEN** a confirmation is shown: "Du hast Daten exportiert, aber nicht über den Server synchronisiert. Wenn die Datei nicht importiert wurde, gehen die Daten verloren. Trotzdem löschen?"

#### Scenario: Removing fully synced event

- **WHEN** the helper removes an event where all batches are server_uploaded
- **THEN** the event is removed without warning

---

### Requirement: Kassen-App file import
The Kassen-App SHALL accept a `.json` sync file via a file picker. Imported purchases MUST be checked against the `completed_purchases` UUID list in localStorage; duplicates MUST be silently skipped. For any `vendor_id` not in the local database, the vendor MUST be auto-created via `get_or_create(vendor_id, booth_id)` resolved from `event_code`.

#### Scenario: Importing a file with new and duplicate purchases
- **WHEN** the cashier imports a file with 10 purchases, 3 of which were already imported
- **THEN** 7 new purchases are added and the 3 duplicates are silently skipped

#### Scenario: Importing a purchase from an unknown vendor
- **WHEN** a purchase references vendor ID 99 not in the local database
- **THEN** vendor 99 is auto-created under the correct booth and the purchase is recorded

---

### Requirement: Kassen-App server sync
The Kassen-App Sync button SHALL trigger `POST /api/sync` (upload) followed by `GET /api/sync?since={last_sync_sequence}` (download). The two operations MUST display independent status indicators. The GET response MUST be paginated: the client loops while `purchases.length == 500`, with `last_sync_sequence` updated per page, up to 20 iterations.

#### Scenario: Successful full sync
- **WHEN** the cashier taps "Synchronisieren" and both POST and GET succeed
- **THEN** "Upload: OK" and "Download: OK" are shown; `last_sync_sequence` is updated

#### Scenario: POST succeeds, GET fails
- **WHEN** POST returns 200 but GET fails with a network error
- **THEN** "Upload: OK / Download: fehlgeschlagen" is shown; `last_upload_ok` is stored as true; `last_sync_sequence` is unchanged

#### Scenario: Retry after partial failure
- **WHEN** the cashier taps "Synchronisieren" again after a POST-succeeded/GET-failed state
- **THEN** the POST step is skipped (last_upload_ok is true); only GET is retried

#### Scenario: Paginated download

- **WHEN** the server returns exactly 500 purchases on the first GET
- **THEN** the Kassen-App issues a second GET with the updated since cursor, up to 20 iterations total

#### Scenario: Cross-session retry — POST re-sent after page reload

- **WHEN** POST succeeds and GET fails, then the cashier reloads the page and taps "Synchronisieren" again
- **THEN** POST is re-sent (because `last_upload_ok` reset to false on reload); the server silently deduplicates via `ON CONFLICT DO NOTHING`; no duplicate purchases appear in the register

#### Scenario: GET returns purchases with mismatched event_code

- **WHEN** the Kassen-App downloads purchases via GET /api/sync and receives items whose `event_code` does not match the active booth's `event_code`
- **THEN** those purchases are displayed with a warning indicator and are not automatically merged; the cashier sees a count: "N Käufe mit abweichendem Event-Code empfangen — bitte prüfen"

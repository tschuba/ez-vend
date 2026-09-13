## Why

Die bisherigen Speicher-Warnbanner (fixed-top für Safari und Privat-Fenster) sind visuell aufdringlich, und die Privat-Fenster-Erkennung ist zu unzuverlässig für den produktiven Einsatz. Nutzer brauchen klare, ehrliche Rückmeldung über ihren Datensicherungsstatus — ohne Fehlalarme und ohne den Arbeitsfluss zu unterbrechen.

## What Changes

- **Entfernt**: Beide fixed-top-Banner (`StorageSafetyBanners`), die klappbare Warnkarte auf der Veranstaltungsseite (`StorageWarningInfo`) und die gesamte Privat-Fenster-Erkennungslogik
- **Neu**: `TrackedDb`-Proxy im Storage-Crate schreibt `last_modified_at` atomar bei jedem Schreibzugriff mit — ohne Änderungen an bestehenden Schreibstellen
- **Neu**: `StorageDiagnostics` bekommt das Feld `last_modified_at: Option<DateTime<Utc>>`
- **Geändert**: `StorageIndicator` im Footer wird zur intelligenten 4-Zustands-Statuszeile mit kontextabhängiger Farbe, erklärendem Text und direktem Backup-CTA
- **Entfernt**: Veraltete i18n-Keys (`backup.private_window_warning_*`, `backup.safari_warning_*`, `backup.storage_warning_*`)

## Capabilities

### New Capabilities
- `tracked-db`: Transparenter Proxy um die Datenbank, der `last_modified_at` automatisch bei jedem Schreibzugriff mitschreibt
- `storage-status-footer`: Intelligente Footer-Statuszeile mit 4 kontextabhängigen Zuständen (OK, Änderungen, überfällig, Safari)

### Modified Capabilities

## Impact

- `crates/storage/src/indexeddb/database.rs` — `Database` → `RawDatabase` intern, neuer `TrackedDb`-Wrapper
- `crates/storage/src/diagnostics.rs` — neues Feld `last_modified_at`
- `crates/ez-vend-ui/src/components/storage_warning.rs` — Banner entfernt, `StorageIndicator` neu aufgebaut
- `crates/ez-vend-ui/src/lib.rs` — `<StorageSafetyBanners />` entfernt
- `crates/ez-vend-ui/src/pages/booth_list.rs` — `<StorageWarningInfo>` entfernt
- `crates/ez-vend-ui/locales/de.json` + `en.json` — Keys bereinigt und ergänzt

## 1. Storage-Crate: TrackedDb-Proxy

- [x] 1.1 `crates/storage/src/indexeddb/database.rs`: bestehenden `Database`-Typ zu `RawDatabase` umbenennen (crate-intern, nicht pub)
- [x] 1.2 Neuen `TrackedDb`-Wrapper implementieren der `RawDatabase` umschließt und als `pub struct Database` exportiert wird
- [x] 1.3 `TrackedDb::transaction()` implementieren: bei `ReadWrite` automatisch `"metadata"` zu den Store-Namen hinzufügen — **außer** wenn die Store-Liste ausschließlich `["metadata"]` enthält (Metadata-interne Schreibvorgänge nicht tracken)
- [x] 1.4 `TrackedTransaction`-Typ implementieren: `store()` delegiert an innere Transaktion; `.done()` schreibt `last_modified_at = Utc::now()` vor dem eigentlichen Done-Call (nicht `commit()` — die Codebase verwendet ausschließlich `.done()`)
- [x] 1.5 Sicherstellen dass `ReadOnly`-Transaktionen `last_modified_at` nicht verändern

## 2. StorageDiagnostics: last_modified_at

- [x] 2.1 `crates/storage/src/diagnostics.rs`: Konstante `LAST_MODIFIED_AT_METADATA_KEY` hinzufügen
- [x] 2.2 Feld `pub last_modified_at: Option<DateTime<Utc>>` zu `StorageDiagnostics` hinzufügen
- [x] 2.3 Ladefunktion `load_last_modified_at()` implementieren (analog zu `load_last_backup_at()`)
- [x] 2.4 `load_storage_diagnostics()` um das neue Feld erweitern
- [x] 2.5 Bestehende Tests im storage-Crate anpassen (Serialisierungs-Tests etc.)

## 3. UI: Banner und Warnkarte entfernen

- [x] 3.1 `crates/ez-vend-ui/src/components/storage_warning.rs`: `StorageSafetyBanners`, `StorageWarningInfo`, `SafetyBannerContext` entfernen
- [x] 3.2 Privat-Fenster-Logik entfernen: `check_idb_blocked()`, `check_storage_not_persisted()` und alle zugehörigen Hilfsfunktionen
- [x] 3.3 `crates/ez-vend-ui/src/lib.rs`: `<StorageSafetyBanners />` und den zugehörigen Import entfernen
- [x] 3.4 `crates/ez-vend-ui/src/pages/booth_list.rs`: `<StorageWarningInfo>` und die Suppression-Logik via `SafetyBannerContext` entfernen

## 4. UI: Smarter Footer aufbauen

- [x] 4.1 Zustandslogik in `StorageIndicator` implementieren: `is_safari()` via `detect_browser()`, Vergleich von `last_modified_at` und `last_backup_at`, Schwellen berechnen (30 Tage nicht-Safari / 3 Tage Safari)
- [x] 4.2 Grünen Zustand umsetzen: grüne Pille mit ⓘ-Icon, Backup-Altersanzeige, "Backups öffnen"-Link
- [x] 4.3 Amber-Footer-Layout umsetzen: gelber Hintergrund (`#fffbeb`), 2px oberer Border (`#fbbf24`), responsive (mobile gestapelt, Desktop inline)
- [x] 4.4 Zustand "BACKUP EMPFOHLEN" (Änderungen) umsetzen: Pille + Text "Einträge ohne Backup"
- [x] 4.5 Zustand "BACKUP ÜBERFÄLLIG" (> 30 Tage, non-Safari) umsetzen: Pille + Backup-Alterstext
- [x] 4.6 Zustand "BACKUP EMPFOHLEN" (Safari, > 3 Tage) umsetzen: Pille + Safari-Erklärungstext + Backup-Alter
- [x] 4.7 Sonderfall `last_backup_at = None` (erster Start): immer amber, Text "Noch kein Backup erstellt" — kein grüner Zustand ohne vorhandenes Backup
- [x] 4.8 "Backup erstellen"-Button implementieren: `ExportButton` mit `ExportScope::All` einbinden
- [x] 4.9 Hover-Tooltip auf grüner Pille: dunkler Popup, Titel + Body-Text (inkl. Inkognito-Hinweis), kein Link; Pille als `<button>` oder `tabindex="0"` damit Tooltip per Tastatur erreichbar ist (`:focus-visible`)

## 5. Barrierefreiheit

- [x] 5.1 `aria-live="polite"` auf den Footer-Statusbereich setzen, damit Screenreader Statusänderungen ankündigen
- [x] 5.2 Kontrastverhältnis aller Amber-Text/Hintergrund-Kombinationen gegen WCAG AA (4,5:1) prüfen und ggf. korrigieren
- [x] 5.3 Sicherstellen dass "Backups öffnen" als `<a>` (Navigation) und "Backup erstellen" als `<button>` (Aktion) korrekte HTML-Semantik haben

## 6. Lokalisierung

- [x] 6.1 Neue Keys in `crates/ez-vend-ui/locales/de.json` ergänzen: `backup.status_ok_label`, `backup.status_warning_label`, `backup.status_overdue_label`, `backup.status_changes`, `backup.status_overdue_days`, `backup.status_safari`, `backup.status_never`, `backup.status_last_backup`, `backup.tooltip_title`, `backup.tooltip_body`, `backup.create_backup`
- [x] 6.2 Gleiche Keys in `locales/en.json` ergänzen
- [x] 6.3 Veraltete Keys entfernen: `backup.private_window_warning_*`, `backup.safari_warning_*`, `backup.storage_warning_*`, `backup.storage_indicator_*`

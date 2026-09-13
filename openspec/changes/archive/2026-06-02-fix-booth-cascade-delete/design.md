## Context

Beim Löschen einer Veranstaltung (`handle_delete_booth` in `booth_list.rs`) wird aktuell nur `booth_repository.delete(&booth.id)` aufgerufen. Die zugehörigen Stores `vendors` und `purchases` in IndexedDB werden dabei nicht bereinigt — sie bleiben mit der alten `booth_id` als Fremdschlüssel erhalten.

Da IndexedDB keine Foreign-Key-Constraints kennt, entstehen „verwaiste" Datensätze. Wenn dieselbe Veranstaltung erneut importiert wird (gleiche `booth_id`), behandelt die App diese Altdaten als vorhandene Einträge und addiert neu importierte Daten hinzu.

Die `ArchiveService`-Klasse in `archive.rs` enthält bereits zwei private Hilfsfunktionen — `delete_vendors_from_transaction` und `delete_purchases_from_transaction` — die innerhalb einer Multi-Store-Transaktion arbeiten und genau für diesen Zweck geeignet sind.

## Goals / Non-Goals

**Goals:**
- Verkäufer und Kassiervorgänge werden atomar zusammen mit dem Booth-Datensatz gelöscht
- Kein inkonsistenter Zwischenstand (teils gelöscht) möglich
- Keine neuen Abstraktionen oder Services notwendig

**Non-Goals:**
- Export-Records (`export_records`-Store) werden nicht bereinigt — konsistent mit bisherigem Verhalten
- Das `booth_repository.delete()` wird nicht geändert oder entfernt (wird intern weiter genutzt, z. B. bei Merge)
- Kein UI-Feedback über die Anzahl gelöschter Unter-Datensätze

## Decisions

### Neue Methode auf `ArchiveService` statt separatem Service

`ArchiveService` besitzt bereits `self.db: Arc<Database>` und die benötigten privaten Helpers in derselben Datei. Eine neue Methode `delete_booth_with_cascade` auf dem bestehenden Service ist minimal invasiv, benötigt keine neuen Traits, keine neuen Abhängigkeiten und keinen neuen Eintrag in `AppState`.

**Alternative: Fail-fast ohne Transaktion** — drei sequentielle Deletes mit Fehlerabbruch. Abgelehnt: Wenn `delete_vendors_from_transaction` erfolgreich ist, aber `delete_purchases_from_transaction` fehlschlägt, entstehen verwaiste Vendors. Der Booth bleibt zwar erhalten (Retry möglich), aber der Zustand ist inkonsistent.

**Alternative: Neuer `DeleteService`** — sauberere Semantik, aber unnötige Abstraktion für zwei Dateien. Abgelehnt.

### Multi-Store-Transaktion über `["booths", "vendors", "purchases"]`

IndexedDB unterstützt Transaktionen über mehrere Object Stores. Dieser Ansatz wird bereits in `archive_booth`, `merge_transactional` und `diagnostics.rs` genutzt — es ist das etablierte Muster im Projekt.

### Booth zuletzt löschen

Vendors und Purchases werden innerhalb der Transaktion zuerst gelöscht, dann der Booth-Datensatz. Reihenfolge ist bei atomaren Transaktionen unkritisch, aber semantisch sinnvoller (Kind vor Eltern).

## Risks / Trade-offs

**Bereits archivierte Booths** → `delete_booth_with_cascade` kann auch auf archivierten Booths aufgerufen werden (Vendors/Purchases wurden dort bereits gelöscht). Die Methode löscht 0 Einträge in diesen Stores und schlägt nicht fehl. ✓

**`archive_service` für Nicht-Archivierungs-Operationen** → Die neue Methode liegt semantisch leicht neben dem Kernzweck von `ArchiveService`. Akzeptabler Trade-off angesichts der gemeinsam genutzten Infrastruktur; kann bei einem späteren Refactoring in einen generischen `BoothService` ausgelagert werden.

## Migration Plan

Keine Datenmigration notwendig. Bereits verwaiste Datensätze in bestehenden Instanzen (aus vorherigen Löschvorgängen) bleiben erhalten — sie werden nicht rückwirkend bereinigt. Das ist akzeptabel, da verwaiste Datensätze nur dann sichtbar werden, wenn eine Veranstaltung mit derselben `booth_id` reimportiert wird, was im normalen Betrieb nicht vorkommt.

## Open Questions

*(keine)*

## Why

Wenn eine Veranstaltung gelöscht wird, werden nur der Booth-Datensatz entfernt — Verkäufer und Kassiervorgänge bleiben als verwaiste Datensätze in IndexedDB. Beim nächsten Import derselben Veranstaltung (gleiche `booth_id`) werden neue Daten auf die alten aufaddiert, was zu doppelten Umsätzen und falschen Abrechnungen führt. Das Problem ist reproduzierbar und blockiert den Validierungstest.

## What Changes

- `ArchiveService` erhält eine neue Methode `delete_booth_with_cascade`, die Verkäufer, Kassiervorgänge und den Booth-Datensatz in **einer einzigen atomaren IndexedDB-Transaktion** löscht
- `handle_delete_booth` in der UI ruft künftig `archive_service.delete_booth_with_cascade()` statt `booth_repository.delete()` auf
- Das bisherige `booth_repository.delete()` bleibt erhalten (wird weiterhin intern genutzt, z. B. bei Merge-Operationen)

## Capabilities

### New Capabilities

- `booth-cascade-delete`: Atomares Löschen einer Veranstaltung inklusive aller zugehörigen Verkäufer und Kassiervorgänge in einer einzigen Transaktion

### Modified Capabilities

*(keine geänderten Spec-Level-Anforderungen bestehender Capabilities)*

## Impact

**Geänderte Dateien:**
- `crates/storage/src/archive.rs` — neue Methode `delete_booth_with_cascade` auf `ArchiveService`
- `crates/ez-vend-ui/src/pages/booth_list.rs` — `handle_delete_booth` nutzt neue Methode

**Genutzte bestehende Infrastruktur:**
- Private Helpers `delete_vendors_from_transaction` und `delete_purchases_from_transaction` aus `archive.rs` (bereits vorhanden, getestet durch Archivierungspfad)
- `ArchiveService` ist bereits in `AppState` eingebunden — keine neuen Abhängigkeiten

**Kein Breaking Change** — nur das Verhalten beim Löschen ändert sich (Cascade statt Orphan).

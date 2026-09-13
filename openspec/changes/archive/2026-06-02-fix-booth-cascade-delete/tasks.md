## 1. Storage-Schicht — neue Methode auf ArchiveService

- [x] 1.1 In `crates/storage/src/archive.rs`: Methode `pub async fn delete_booth_with_cascade(&self, booth_id: &BoothId) -> Result<(), StorageError>` zu `ArchiveService` hinzufügen
- [x] 1.2 Methode öffnet eine ReadWrite-Transaktion über `["booths", "vendors", "purchases"]`
- [x] 1.3 Ruft `delete_vendors_from_transaction(&transaction, booth_id)` auf (privater Helper, bereits vorhanden)
- [x] 1.4 Ruft `delete_purchases_from_transaction(&transaction, booth_id)` auf (privater Helper, bereits vorhanden)
- [x] 1.5 Löscht den Booth-Datensatz direkt im `booths`-Store innerhalb der Transaktion
- [x] 1.6 Ruft `transaction.done().await?` auf, um atomar zu committen

## 2. UI-Schicht — Delete-Handler aktualisieren

- [x] 2.1 In `crates/ez-vend-ui/src/pages/booth_list.rs`: In `handle_delete_booth` den Aufruf `state.booth_repository.delete(&booth.id).await` durch `state.archive_service.delete_booth_with_cascade(&booth.id).await` ersetzen
- [x] 2.2 Sicherstellen, dass der Success- und Error-Pfad im `match`-Block unverändert bleibt

## 3. Verifikation

- [x] 3.1 `cargo build --target wasm32-unknown-unknown` — Kompilierung ohne Fehler
- [x] 3.2 App starten, `geraet_a_vor_merge.json` importieren → 7 Käufe, 569,50 € Gesamtumsatz, Verkäufer V1–V7 prüfen
- [x] 3.3 Veranstaltung löschen, dieselbe Datei erneut importieren → immer noch 7 Käufe, 569,50 € — keine Doppelzählung
- [x] 3.4 Bestehende UUID-Fix-Änderungen auf derselben Branch committen (testdata + cascade-delete in einem Commit)

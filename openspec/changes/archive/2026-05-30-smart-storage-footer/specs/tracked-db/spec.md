## ADDED Requirements

### Requirement: Automatisches Tracking von Schreibzugriffen
Das System SHALL bei jedem erfolgreichen Schreibzugriff auf die Datenbank automatisch einen `last_modified_at`-Timestamp in den Metadata-Store schreiben, ohne dass Schreibstellen explizit daran denken müssen.

#### Scenario: Schreibtransaktion aktualisiert last_modified_at
- **WHEN** eine ReadWrite-Transaktion via `.done()` abgeschlossen wird
- **THEN** wird `last_modified_at` mit dem aktuellen UTC-Timestamp atomar in derselben Transaktion in den Metadata-Store geschrieben

#### Scenario: Lesetransaktion ändert last_modified_at nicht
- **WHEN** eine ReadOnly-Transaktion via `.done()` abgeschlossen wird
- **THEN** bleibt `last_modified_at` unverändert

#### Scenario: Fehlgeschlagene Transaktion ändert last_modified_at nicht
- **WHEN** eine ReadWrite-Transaktion mit einem Fehler abbricht (kein `.done()`)
- **THEN** bleibt `last_modified_at` unverändert

#### Scenario: Reine Metadata-Transaktion wird nicht getrackt
- **WHEN** eine ReadWrite-Transaktion ausschließlich auf dem `"metadata"`-Store operiert (z.B. `record_backup_completed`)
- **THEN** wird `last_modified_at` nicht aktualisiert — Metadata-interne Schreibvorgänge gelten nicht als Datenänderung

### Requirement: last_modified_at in StorageDiagnostics
Das System SHALL `last_modified_at` als optionales Feld in `StorageDiagnostics` bereitstellen, das beim Laden der Diagnostics aus dem Metadata-Store gelesen wird.

#### Scenario: Wert vorhanden nach erstem Schreibzugriff
- **WHEN** `load_storage_diagnostics()` nach mindestens einem Schreibzugriff (non-metadata) aufgerufen wird
- **THEN** enthält `StorageDiagnostics.last_modified_at` einen `Some(DateTime<Utc>)`-Wert

#### Scenario: None bei frischer Installation
- **WHEN** `load_storage_diagnostics()` aufgerufen wird ohne dass je ein Schreibzugriff stattgefunden hat
- **THEN** ist `StorageDiagnostics.last_modified_at` gleich `None`

### Requirement: Externe API unverändert
Das System SHALL die öffentliche `Database`-API unverändert lassen — alle Aufrufer erhalten weiterhin `&Database` und merken nichts vom internen Proxy. `TrackedTransaction` stellt `.done()` als eigene Methode bereit (kein neues `.commit()`), da die gesamte Codebase `.done()` verwendet.

#### Scenario: Bestehende Schreibfunktionen ohne Codeänderung
- **WHEN** eine bestehende Storage-Funktion `db: &Database` erhält und eine Transaktion öffnet und mit `.done()` abschließt
- **THEN** wird `last_modified_at` automatisch mitgeschrieben ohne Änderung an der Funktion selbst

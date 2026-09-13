## Context

EZ Booth speichert alle Daten lokal im Browser (IndexedDB). Bisher wurden Nutzer über Speicherrisiken durch zwei fixed-top-Banner informiert: einen Safari-spezifischen und einen für vermutete Privat-Fenster. Beide Ansätze haben Probleme — der Privat-Fenster-Check ist zu unzuverlässig, und die Banner sind visuell aufdringlich. Der Footer enthält bereits einen `StorageIndicator`, der aber nur statisch "Lokal gespeichert" anzeigt.

Zusätzlich fehlte bisher eine zuverlässige Möglichkeit zu erkennen, ob seit dem letzten Backup Änderungen vorgenommen wurden.

## Goals / Non-Goals

**Goals:**
- Alle Speicherhinweise in eine permanente, kontextabhängige Footer-Statuszeile konsolidieren
- Zuverlässige Erkennung von ungesicherten Änderungen ohne manuelle Aufrufe an Schreibstellen
- Safari-spezifisches Risiko kommunizieren ohne Fehlalarme für andere Browser
- Privat-Fenster-Erkennung vollständig entfernen (zu unzuverlässig)

**Non-Goals:**
- Cloud-Backup oder Server-seitige Synchronisierung
- Konfigurierbare Schwellenwerte (30-Tage-Grenze ist fix)
- Unterscheidung nach Typ der Änderung (neue Kaufvorgänge vs. Vendoränderungen)

## Decisions

### D1: TrackedDb-Proxy statt expliziter Aufrufe

**Entscheidung:** `Database` wird intern zu `RawDatabase`. Ein neuer `TrackedDb`-Wrapper wird als `Database` exportiert. `TrackedDb::transaction()` fügt bei `ReadWrite`-Transaktionen automatisch den `"metadata"`-Store hinzu; `TrackedTransaction::done()` schreibt `last_modified_at` atomar in derselben Transaktion vor dem eigentlichen Done-Call.

**Wichtig — `.done()` nicht `.commit()`:** Die gesamte Codebase beendet Transaktionen mit `transaction.done()` (Rexie-API), nicht mit einem eigenen `commit()`. `TrackedTransaction` muss deshalb `.done()` als eigene Methode bereitstellen (kein Deref-Shadowing), die den Timestamp schreibt und dann das innere `.done()` aufruft.

**Wichtig — Metadata-Transaktionen ausschließen:** `record_backup_completed()` und ähnliche interne Metadata-Schreibvorgänge dürfen `last_modified_at` nicht selbst auslösen (sonst zeigt der Footer amber direkt nach einem Backup). Transaktionen deren Store-Liste ausschließlich `["metadata"]` enthält, werden vom Tracking ausgenommen.

**Alternativen:**
- *Explizite Aufrufe*: `record_db_modified()` manuell in jeder Schreibfunktion. Einfach, aber vergessbar bei neuen Methoden.
- *Trait (`DbAccess`)*: Maximale Flexibilität für Tests, aber `async_trait`-Komplexität im WASM-Kontext unnötig.
- *Rename only*: `TrackedDb` als neue primäre `Database` — gewählt, weil externe API unverändert bleibt und keine zusätzlichen Abstractions nötig sind.

**Warum Proxy:** Neue Schreibmethoden können `last_modified_at` nicht vergessen — es ist in `done()` eingebaut. Einmal pro Transaktion, nicht pro Store. Atomar.

### D2: 4-Zustands-Footer statt schließbarer Banner

**Entscheidung:** `StorageIndicator` zeigt permanent einen von 4 Zuständen (grün/gelb). Kein "X"-schließen, keine Position-fixed-Überlagerungen.

**Warum:** Der Nutzer soll den Status immer sehen können, ohne aktiv handeln zu müssen. Schließbare Banner werden weggeklickt und vergessen.

### D3: Zustandspriorität — Safari mit 3-Tage-Schwelle

**Entscheidung:** Safari wird nicht dauerhaft amber gezeigt, sondern mit einer kürzeren Schwelle (3 Tage statt 30). Safari-Nutzer sehen grün wenn Backup < 3 Tage alt und keine Änderungen vorhanden, sonst amber mit Safari-spezifischem Erklärungstext. Priorität: Änderungen → Alter (3d Safari / 30d andere) → OK.

**Warum nicht always-amber:** Dauerhaftes Gelb führt zu Warning Fatigue — Nutzer ignorieren den Hinweis nach wenigen Sessions systematisch. Die kürzere Schwelle kommuniziert das erhöhte Safari-Risiko ohne Dauerermüdung.

### D4: "Backup erstellen" löst direkten Export aus

**Entscheidung:** Der CTA-Button im amber Footer ruft `ExportButton` mit `ExportScope::All` auf — direkter Download, keine Navigation.

**Warum:** Minimale Friktion. Der Nutzer soll nicht erst zur Einstellungsseite navigieren müssen.

## Risks / Trade-offs

- **Metadata-Store in jeder ReadWrite-Transaktion** → Geringes Performance-Risiko bei sehr häufigen Schreibvorgängen. Mitigation: `last_modified_at` ist ein einfacher Timestamp-Schreib-Vorgang, IndexedDB-Overhead ist vernachlässigbar.
- **`last_modified_at = None` nach Update** → Nutzer die das Update einspielen haben initial keinen Tracking-Wert. Mitigation: `None` wird als "keine bekannten Änderungen" behandelt — grüner Zustand wenn `last_backup_at` vorhanden und < 30 Tage. Kein Fehlalarm.
- **Safari always-amber kann nerven** → Nutzer auf Safari sehen immer gelb, auch direkt nach einem Backup. Trade-off bewusst akzeptiert: das Risiko ist real und strukturell, nicht situationsabhängig.

## Migration Plan

Keine Datenmigration nötig. `last_modified_at` startet als `None` und wird beim nächsten Schreibzugriff gesetzt. Bestehende `last_backup_at`-Daten bleiben unverändert.

Rollback: Einfaches Revert des Commits — keine Schema-Änderungen in IndexedDB (nur neue Metadata-Keys).

## Open Questions

— keine —

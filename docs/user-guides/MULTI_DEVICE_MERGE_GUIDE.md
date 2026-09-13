---
title: Multi-Device Booth Merge Guide
nav_order: 3
parent: User Guides
---

# Multi-Device Booth Merge Guide / Leitfaden für Geräteübergreifende Stand-Zusammenführung

This guide explains the safest current workflow for working on one booth across multiple devices and then merging the booth data back together.

Dieser Leitfaden erklärt den sichersten aktuellen Arbeitsablauf für die Bearbeitung eines Standes auf mehreren Geräten und die anschließende Zusammenführung der Stand-Daten.

---

## Recommended Use Case / Empfohlener Anwendungsfall

### EN

Use this workflow when:

- one event booth is used on multiple laptops or tablets
- each device may record purchases while offline
- the team needs to merge those purchases back into one booth safely

This guide is about booth backups, not cloud sync.

### DE

Verwenden Sie diesen Arbeitsablauf, wenn:

- ein Veranstaltungsstand auf mehreren Laptops oder Tablets genutzt wird
- jedes Gerät möglicherweise Käufe offline erfasst
- das Team diese Käufe sicher zu einem Stand zusammenführen muss

Dieser Leitfaden behandelt Stand-Backups, nicht Cloud-Synchronisation.

---

## Safe Workflow / Sicherer Arbeitsablauf

### EN

1. Create or confirm one known-good booth backup.
2. Import that booth backup onto every device that will be used for the same booth.
3. Record new purchases on each device.
4. Export a booth backup from each device.
5. Choose one target device as the merge device.
6. Import each device backup on the target device with `Merge`.
7. Verify vendor list, purchase count, and booth totals.
8. Create one new booth backup from the merged result.

### DE

1. Erstellen oder bestätigen Sie ein bekanntes funktionierendes Stand-Backup.
2. Importieren Sie dieses Stand-Backup auf jedes Gerät, das für denselben Stand verwendet wird.
3. Erfassen Sie neue Käufe auf jedem Gerät.
4. Exportieren Sie ein Stand-Backup von jedem Gerät.
5. Wählen Sie ein Ziel-Gerät als Merge-Gerät aus.
6. Importieren Sie jedes Geräte-Backup auf dem Ziel-Gerät mit `Merge`.
7. Prüfen Sie Verkäuferliste, Kaufanzahl und Stand-Summen.
8. Erstellen Sie ein neues Stand-Backup aus dem zusammengeführten Ergebnis.

---

## What `Merge` Safely Does / Was `Merge` sicher durchführt

### EN

- imports new booths, vendors, and purchases that do not exist locally yet
- keeps both purchases when the devices created different purchase IDs
- chooses the strictly newer booth record when the same booth was updated on multiple devices
- chooses the strictly newer purchase record when the same purchase ID was updated on multiple devices
- keeps the local record when booth or purchase timestamps are exactly equal
- preserves the earliest vendor creation timestamp when the same vendor exists on multiple devices

### DE

- importiert neue Stände, Verkäufer und Käufe, die lokal noch nicht vorhanden sind
- behält beide Käufe, wenn die Geräte unterschiedliche Kauf-IDs erstellt haben
- wählt den eindeutig neueren Stand-Datensatz, wenn derselbe Stand auf mehreren Geräten aktualisiert wurde
- wählt den eindeutig neueren Kauf-Datensatz, wenn dieselbe Kauf-ID auf mehreren Geräten aktualisiert wurde
- behält den lokalen Datensatz, wenn Stand- oder Kauf-Zeitstempel exakt gleich sind
- behält den frühesten Erstellungszeitpunkt eines Verkäufers, wenn derselbe Verkäufer auf mehreren Geräten vorhanden ist

---

## What `Merge` Does Not Try To Do / Was `Merge` nicht versucht

### EN

- it does not guess that two different purchase IDs are the same real-world sale
- it does not combine two conflicting booth edits field by field
- it does not guess that two different purchase IDs are the same real-world sale (this is by design)
- when the same event was independently created on two devices and both local copies have duplicated further, a guided conflict wizard helps the operator choose which copy to merge into

### DE

- es errät nicht, dass zwei unterschiedliche Kauf-IDs derselbe reale Verkauf sind
- es kombiniert nicht zwei widersprüchliche Stand-Änderungen Feld für Feld
- es errät nicht, dass zwei unterschiedliche Kauf-IDs derselbe reale Verkauf sind (dies ist beabsichtigt)
- wenn dieselbe Veranstaltung unabhängig auf zwei Geräten erstellt wurde, hilft ein geführter Konflikt-Assistent dem Anwendenden, welche Kopie zusammengeführt werden soll

---

## Practical Team Rules / Praktische Team-Regeln

### EN

- prefer booth backups over full backups for single-booth device transfer
- import the latest known booth backup before entering more data on another device
- keep all exported files until the merged result is verified
- after merging, create a new backup and treat that file as the current recovery point
- if totals look wrong, stop and compare the imported purchase list before recording more sales

### DE

- bevorzugen Sie Stand-Backups gegenüber Voll-Backups für Einzelstand-Geräteübertragungen
- importieren Sie das neueste bekannte Stand-Backup, bevor Sie auf einem anderen Gerät weitere Daten erfassen
- bewahren Sie alle exportierten Dateien auf, bis das Merge-Ergebnis geprüft ist
- erstellen Sie nach dem Merge ein neues Backup und behandeln Sie diese Datei als aktuellen Wiederherstellungspunkt
- wenn Summen falsch aussehen, stoppen Sie und vergleichen Sie die importierte Kaufliste, bevor Sie weitere Verkäufe erfassen

---

## Verification Checklist After Merge / Prüfliste nach dem Merge

### EN

- the expected booth is present
- vendor count looks correct
- the expected vendor IDs are still present
- purchase count matches the combined expected count from all devices
- latest purchase notes or corrections are present where expected
- booth totals match the combined purchases
- no unrelated booths were changed

### DE

- der erwartete Stand ist vorhanden
- die Verkäuferanzahl sieht korrekt aus
- die erwarteten Verkäufer-IDs sind weiterhin vorhanden
- die Kaufanzahl stimmt mit der kombinierten erwarteten Anzahl von allen Geräten überein
- neueste Kauf-Notizen oder Korrekturen sind vorhanden, wo erwartet
- die Stand-Summen stimmen mit den kombinierten Käufen überein
- keine nicht betroffenen Stände wurden geändert

---

## When To Use `Skip` Or `Replace` / Wann `Skip` oder `Replace` verwenden

### EN

All three strategies always import any new vendors and purchases — the strategy choice only controls what happens to **booth metadata** (name, date, fee settings) when a matching event already exists locally.

- use `Skip` when you want the local device's event settings to stay unchanged; new vendors and purchases from the file are still added
- use `Replace` when one backup should fully overwrite the local event settings
- use `Merge` for the normal multi-device booth workflow (keeps whichever event settings are more recent)

### DE

Alle drei Strategien importieren immer neue Verkäufer und Käufe — die Strategieauswahl steuert nur, was mit den **Veranstaltungseinstellungen** (Name, Datum, Gebühren) passiert, wenn eine passende Veranstaltung bereits lokal vorhanden ist.

- verwenden Sie `Skip`, wenn die Veranstaltungseinstellungen des lokalen Geräts unverändert bleiben sollen; neue Verkäufer und Käufe aus der Datei werden trotzdem hinzugefügt
- verwenden Sie `Replace`, wenn ein Backup die lokalen Veranstaltungseinstellungen vollständig überschreiben soll
- verwenden Sie `Merge` für den normalen geräteübergreifenden Stand-Arbeitsablauf (behält die aktuelleren Veranstaltungseinstellungen)

---

## Current Limits To Communicate To Operators / Aktuelle Einschränkungen für Anwendende

### EN

- if two devices independently record the same real-world sale as two different purchases, EZ Booth will keep both because the purchase IDs are different
- if two devices edit the same booth or purchase at the exact same timestamp, the target device keeps its existing local record
- if a single import contains events that cannot be automatically matched (ambiguous local duplicates), those events are skipped with a visible reason — the rest of the import still applies

### DE

- wenn zwei Geräte unabhängig voneinander denselben realen Verkauf als zwei unterschiedliche Käufe erfassen, behält EZ Booth beide, da die Kauf-IDs unterschiedlich sind
- wenn zwei Geräte denselben Stand oder Kauf zum exakt gleichen Zeitstempel bearbeiten, behält das Ziel-Gerät seinen bestehenden lokalen Datensatz
- wenn ein einzelner Import Veranstaltungen enthält, die nicht automatisch zugeordnet werden können (mehrdeutige lokale Duplikate), werden diese Veranstaltungen mit einem sichtbaren Grund übersprungen — der Rest des Imports wird trotzdem angewendet

---

## Validation Status / Validierungsstatus

### EN

The storage-layer merge behavior for this workflow has automated browser-backed coverage for:

- repeated import of shared booth history without duplicate records
- parallel multi-device booth purchase merges
- round-trip imports
- same-purchase conflict resolution by newer timestamp
- vendor records preserve the earliest creation timestamp during merge
- mixed booth-backup and full-backup merge sequences

For cross-browser operator validation, also use:

- `docs/user-guides/DATA_BACKUP_GUIDE.md`

### DE

Das Merge-Verhalten auf Storage-Ebene für diesen Arbeitsablauf hat automatisierte browsergestützte Abdeckung für:

- wiederholten Import gemeinsamer Stand-Historie ohne doppelte Datensätze
- parallele geräteübergreifende Stand-Kauf-Merges
- Roundtrip-Importe
- Konfliktlösung bei identischen Käufen anhand des neueren Zeitstempels
- Erhalt des frühesten Erstellungszeitpunkts von Verkäuferdatensätzen beim Merge
- gemischte Stand-Backup- und Voll-Backup-Merge-Sequenzen

Für browserübergreifende Operator-Validierung verwenden Sie auch:

- `docs/user-guides/DATA_BACKUP_GUIDE.md`

---
title: Data Backup Guide
nav_order: 2
parent: User Guides
---

# Data Backup Guide / Leitfaden Datensicherung

This guide explains how ez-vend backup and recovery works for event operators.

Dieser Leitfaden erklärt, wie Datensicherung und Wiederherstellung in ez-vend für Veranstaltungs-Teams funktionieren.

## 1. Where Your Data Lives / Wo Ihre Daten gespeichert werden

### EN

- ez-vend stores event data in the current browser on the current device.
- The app does not automatically sync data to a server or cloud storage.
- If browser storage is cleared, local event data can be removed.
- Backups are JSON files that you download and store outside the browser.

### DE

- ez-vend speichert Veranstaltungsdaten im aktuellen Browser auf dem aktuellen Gerät.
- Die App synchronisiert Daten nicht automatisch mit einem Server oder Cloud-Speicher.
- Wenn Browserdaten gelöscht werden, können lokale Veranstaltungsdaten verloren gehen.
- Backups sind JSON-Dateien, die Sie herunterladen und außerhalb des Browsers speichern.

## 2. What Can Cause Data Loss / Was zu Datenverlust führen kann

### EN

Data can be lost if you:

- clear browser history, website data, or storage
- switch to a different browser profile
- move to another laptop or tablet without exporting first
- use private browsing and close the session
- reset or replace the device

### DE

Daten können verloren gehen, wenn Sie:

- Browserverlauf, Webseitendaten oder Speicher löschen
- zu einem anderen Browser-Profil wechseln
- auf ein anderes Notebook oder Tablet wechseln, ohne vorher zu exportieren
- einen privaten Browsermodus verwenden und die Sitzung schließen
- das Gerät zurücksetzen oder austauschen

## 3. Full Backup: Export Everything / Vollständiges Backup: Alles exportieren

### EN

Use a full backup when you want to protect all booths, vendors, and purchases.

Steps:

1. Open the booth list page.
2. Click `Export All`.
3. Save the downloaded JSON file to a safe location.
4. Confirm the file exists outside the browser, for example in `Downloads`, a team folder, or external storage.

Typical filename:

- `ez-vend-backup-2026-03-29.json`

### DE

Verwenden Sie ein vollständiges Backup, wenn Sie alle Veranstaltungen, Verkäufer und Käufe sichern möchten.

Schritte:

1. Öffnen Sie die Standliste.
2. Klicken Sie auf `Alle exportieren`.
3. Speichern Sie die heruntergeladene JSON-Datei an einem sicheren Ort.
4. Prüfen Sie, dass die Datei außerhalb des Browsers vorhanden ist, zum Beispiel in `Downloads`, einem Team-Ordner oder auf einem externen Speichermedium.

Typischer Dateiname:

- `ez-vend-backup-2026-03-29.json`

## 4. Booth Backup: Export One Event / Stand-Backup: Eine Veranstaltung exportieren

### EN

Use a booth backup when you only need one event.

Steps:

1. Open the booth list.
2. Find the booth card you want to protect.
3. Click the booth export action on that card.
4. Save the downloaded JSON file.

Typical filename:

- `ez-vend-spring-market-2026-2026-03-29.json`

### DE

Verwenden Sie ein Stand-Backup, wenn Sie nur eine einzelne Veranstaltung sichern möchten.

Schritte:

1. Öffnen Sie die Standliste.
2. Suchen Sie die Standkarte der gewünschten Veranstaltung.
3. Klicken Sie auf die Export-Aktion dieses Standes.
4. Speichern Sie die heruntergeladene JSON-Datei.

Typischer Dateiname:

- `ez-vend-spring-market-2026-2026-03-29.json`

## 5. Import A Backup / Ein Backup importieren

### EN

You can import either a full backup or a booth backup.

Steps:

1. Open the booth list page.
2. Click `Import`.
3. Select a `.json` backup file.
4. Review the preview shown by ez-vend.
5. Choose how conflicts should be handled.
6. Apply the import.
7. Verify the booth list, vendors, and purchases afterwards.

Conflict strategies:

- `Merge`: import new records, prefer strictly newer booth and purchase records, and preserve the earliest vendor creation timestamp when multiple devices exported the same booth
- `Skip`: keep existing records and ignore conflicting imported ones
- `Replace`: overwrite existing conflicting records with imported data

Important merge details:

- If the same booth or purchase already exists and the timestamps are exactly equal, ez-vend keeps the existing local record.
- If the same vendor exists in both imports, ez-vend keeps the earliest `created_at` timestamp for that vendor.
- If two devices created different purchases, both purchases are kept as long as they have different purchase IDs.

### DE

Sie können sowohl ein vollständiges Backup als auch ein Stand-Backup importieren.

Schritte:

1. Öffnen Sie die Standliste.
2. Klicken Sie auf `Importieren`.
3. Wählen Sie eine `.json`-Backup-Datei aus.
4. Prüfen Sie die von ez-vend angezeigte Vorschau.
5. Wählen Sie, wie Konflikte behandelt werden sollen.
6. Starten Sie den Import.
7. Prüfen Sie danach Standliste, Verkäufer und Käufe.

Konfliktstrategien:

- `Merge`: importiert neue Datensätze, bevorzugt bei Ständen und Käufen nur eindeutig neuere Datensätze und behält bei Verkäufern den frühesten `created_at`-Zeitstempel
- `Skip`: behält vorhandene Datensätze und ignoriert konfligierende importierte Datensätze
- `Replace`: überschreibt vorhandene konfligierende Datensätze mit den importierten Daten

Wichtige Merge-Details:

- Wenn derselbe Stand oder Kauf bereits vorhanden ist und die Zeitstempel exakt gleich sind, behält ez-vend den bereits lokalen Datensatz.
- Wenn derselbe Verkäufer in beiden Importen vorkommt, behält ez-vend einen nicht-leeren Verkäufernamen und bevorzugt den aussagekräftigeren Namen.
- Wenn zwei Geräte unterschiedliche Käufe erzeugt haben, bleiben beide Käufe erhalten, solange sie unterschiedliche Kauf-IDs haben.

## 5a. Multi-Device Booth Workflow / Mehrgeräte-Workflow für einzelne Stände

### EN

This is the main recommended workflow when one booth is worked on across multiple devices.

Recommended sequence:

1. Start from one known-good booth backup.
2. Import that booth backup on each additional device before entering more data.
3. Let each device record its own new purchases.
4. Export the booth again from each device.
5. Import those booth backups on the target device with `Merge`.
6. Verify the final booth totals, vendor list, and recent purchases.

Best practices:

- prefer booth backups for device-to-device booth work instead of full backups
- import before creating new data on another device whenever possible
- keep the exported files from each device until the merged result is verified
- after a successful merge, create one fresh booth backup as the new recovery point

Limits to understand:

- ez-vend does not try to guess whether two different purchase IDs are actually the same real-world sale
- if two devices change the same booth or purchase at the same timestamp, the existing local record is kept during `Merge`
- multi-file imports are applied one after another, so verify the result after importing several files

### DE

Dies ist der empfohlene Hauptablauf, wenn ein einzelner Stand auf mehreren Geräten bearbeitet wird.

Empfohlene Reihenfolge:

1. Starten Sie mit einem bekannten gültigen Stand-Backup.
2. Importieren Sie dieses Stand-Backup auf jedem weiteren Gerät, bevor neue Daten erfasst werden.
3. Lassen Sie jedes Gerät seine neuen Käufe erfassen.
4. Exportieren Sie den Stand anschließend erneut von jedem Gerät.
5. Importieren Sie diese Stand-Backups auf dem Zielgerät mit `Merge`.
6. Prüfen Sie die finalen Stand-Summen, die Verkäuferliste und die letzten Käufe.

Empfohlene Praxis:

- verwenden Sie für Geräte-zu-Geräte-Standarbeit bevorzugt Stand-Backups statt Voll-Backups
- importieren Sie nach Möglichkeit immer zuerst, bevor auf einem weiteren Gerät neue Daten erfasst werden
- bewahren Sie die Exportdateien aller Geräte auf, bis das Merge-Ergebnis geprüft ist
- erstellen Sie nach einem erfolgreichen Merge ein neues Stand-Backup als neuen Wiederherstellungspunkt

Wichtige Grenzen:

- ez-vend versucht nicht zu erraten, ob zwei unterschiedliche Kauf-IDs denselben realen Verkauf meinen
- wenn zwei Geräte denselben Stand oder Kauf mit exakt gleichem Zeitstempel ändern, bleibt beim `Merge` der bereits lokale Datensatz erhalten
- mehrere Dateien werden nacheinander importiert; prüfen Sie deshalb das Ergebnis nach dem Import mehrerer Dateien

## 6. When To Create Backups / Wann Backups erstellt werden sollten

### EN

Create a backup:

- before clearing browser data
- before browser updates or profile cleanup
- before moving to another device
- before and after an event day
- before testing imports or larger cleanup actions
- after important data entry sessions

### DE

Erstellen Sie ein Backup:

- vor dem Löschen von Browserdaten
- vor Browser-Updates oder Profilbereinigungen
- vor dem Wechsel auf ein anderes Gerät
- vor und nach einem Veranstaltungstag
- vor Test-Importen oder größeren Bereinigungen
- nach wichtigen Eingabephasen

## 7. Where To Store Backup Files / Wo Backup-Dateien gespeichert werden sollten

### EN

Recommended storage locations:

- a shared team folder
- an external USB drive
- an organization-managed cloud folder
- a second device controlled by the event team

Good practice:

- keep more than one copy
- include the event date in your folder structure
- avoid storing the only copy inside the browser environment

### DE

Empfohlene Speicherorte:

- ein gemeinsamer Team-Ordner
- ein externer USB-Datenträger
- ein von der Organisation verwalteter Cloud-Ordner
- ein zweites Gerät des Veranstaltungsteams

Gute Praxis:

- bewahren Sie mehr als eine Kopie auf
- verwenden Sie die Veranstaltungsdaten in Ihrer Ordnerstruktur
- speichern Sie nicht die einzige Kopie nur innerhalb der Browser-Umgebung

## 8. After Data Loss / Nach einem Datenverlust

### EN

If data is missing:

1. Stop entering new event data until you know what was lost.
2. Check whether the issue is only a different browser profile or device.
3. Locate the newest relevant backup file.
4. Import the backup from the booth list.
5. Verify booth counts, vendor lists, and recent purchases.
6. Create a fresh backup after successful recovery.

### DE

Wenn Daten fehlen:

1. Erfassen Sie keine neuen Veranstaltungsdaten, bis klar ist, was verloren ging.
2. Prüfen Sie, ob nur ein anderes Browser-Profil oder Gerät verwendet wird.
3. Suchen Sie die neueste passende Backup-Datei.
4. Importieren Sie das Backup über die Standliste.
5. Prüfen Sie Standanzahl, Verkäuferlisten und aktuelle Käufe.
6. Erstellen Sie nach erfolgreicher Wiederherstellung ein neues Backup.

## 9. Quick Recommendation / Kurze Empfehlung

### EN

Before each event session, export a full backup and confirm the file was saved outside the browser.

### DE

Exportieren Sie vor jeder Veranstaltungssitzung ein vollständiges Backup und prüfen Sie, dass die Datei außerhalb des Browsers gespeichert wurde.

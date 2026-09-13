## ADDED Requirements

### Requirement: Footer zeigt kontextabhängigen Speicherstatus
Der Footer SHALL einen von vier Zuständen anzeigen, abhängig von Browser, Backup-Alter und ungesicherten Änderungen. Zustandspriorität: Änderungen → Alter (3 Tage Safari / 30 Tage andere) → OK.

#### Scenario: Grüner Zustand bei aktuellem Backup (nicht Safari)
- **WHEN** der Browser kein Safari ist UND `last_backup_at` vorhanden und < 30 Tage alt ist UND keine ungesicherten Änderungen vorliegen (`last_modified_at` ist None oder `<= last_backup_at`)
- **THEN** zeigt der Footer eine grüne Pille "LOKAL GESPEICHERT" mit ⓘ-Icon und das Alter des letzten Backups

#### Scenario: Grüner Zustand bei frischem Backup auf Safari
- **WHEN** der Browser Safari ist UND `last_backup_at` vorhanden und < 3 Tage alt ist UND keine ungesicherten Änderungen vorliegen
- **THEN** zeigt der Footer eine grüne Pille "LOKAL GESPEICHERT" mit ⓘ-Icon und das Alter des letzten Backups

#### Scenario: Gelber Zustand bei ungesicherten Änderungen
- **WHEN** `last_modified_at > last_backup_at`
- **THEN** zeigt der Footer eine amber Statuszeile mit Pille "BACKUP EMPFOHLEN", Text "Einträge ohne Backup" und den Buttons "Backups öffnen" und "Backup erstellen"

#### Scenario: Gelber Zustand bei überfälligem Backup (nicht Safari)
- **WHEN** der Browser kein Safari ist UND `last_backup_at` ist None oder > 30 Tage alt UND keine ungesicherten Änderungen vorliegen
- **THEN** zeigt der Footer eine amber Statuszeile mit Pille "BACKUP ÜBERFÄLLIG" und Angabe des letzten Backup-Datums

#### Scenario: Gelber Zustand bei überfälligem Backup auf Safari
- **WHEN** der Browser Safari ist UND `last_backup_at` ist None oder > 3 Tage alt UND keine ungesicherten Änderungen vorliegen
- **THEN** zeigt der Footer eine amber Statuszeile mit Pille "BACKUP EMPFOHLEN", Text "Safari kann Daten ohne Vorwarnung löschen" und Backup-Alter

#### Scenario: Noch nie gesichert (erster Start)
- **WHEN** `last_backup_at` ist None (unabhängig von `last_modified_at`)
- **THEN** zeigt der Footer amber — "Noch kein Backup erstellt" anstelle des Backup-Alters. Grüner Zustand ist ohne vorhandenes `last_backup_at` nicht möglich.

### Requirement: Tooltip auf grüner Pille
Die grüne "LOKAL GESPEICHERT"-Pille SHALL einen Tooltip anzeigen (Hover auf Desktop, Tap auf Mobile), der die lokale Speicherung ohne technischen Jargon erklärt. Die Pille SHALL als fokussierbares Element (`<button>` oder `tabindex="0"`) umgesetzt sein, damit der Tooltip auch per Tastatur erreichbar ist.

#### Scenario: Tooltip erscheint bei Hover
- **WHEN** der Nutzer mit der Maus über die grüne Pille fährt
- **THEN** erscheint ein Tooltip mit Titel "Nur auf diesem Gerät" und dem Text "Alle Einträge sind ausschließlich in diesem Browser gespeichert. Nutzen Sie ein neues Gerät, einen anderen Browser, ein privates Fenster oder löschen Sie den Verlauf — sind die Daten weg. Regelmäßige Backups schützen davor."

#### Scenario: Tooltip per Tastatur erreichbar
- **WHEN** der Nutzer die Pille per Tab fokussiert
- **THEN** erscheint der Tooltip (via `:focus-visible` oder `onFocus`-Handler)

#### Scenario: Tooltip enthält keinen redundanten Link
- **WHEN** der Tooltip angezeigt wird
- **THEN** enthält er keinen Link zu den Backups (dieser ist bereits direkt neben der Pille sichtbar)

### Requirement: "Backup erstellen" löst direkten Export aus
Der Button "Backup erstellen" im amber Footer SHALL einen direkten Datei-Download aller Daten auslösen — ohne Navigation zu einer anderen Seite.

#### Scenario: Direkter Download bei Klick
- **WHEN** der Nutzer auf "Backup erstellen" klickt
- **THEN** wird sofort ein Datei-Download mit allen Veranstaltungsdaten gestartet (äquivalent zu ExportButton mit ExportScope::All)

### Requirement: Keine schließbaren Speicher-Banner
Das System SHALL keine fixed-position-Banner für Speicherwarnungen mehr anzeigen. Alle Speicherstatusinformationen sind ausschließlich im Footer sichtbar.

#### Scenario: Kein Safari-Banner beim Laden
- **WHEN** die App in Safari geöffnet wird
- **THEN** erscheint kein fixed-top-Banner — der Hinweis ist ausschließlich im Footer sichtbar

#### Scenario: Keine Privat-Fenster-Erkennung
- **WHEN** die App in einem Privat-Fenster geöffnet wird
- **THEN** erscheint kein Banner — die Erkennung wurde entfernt

### Requirement: Barrierefreiheit des Footer-Status
Der Footer-Statusbereich SHALL für Screenreader und Tastaturnutzer zugänglich sein.

#### Scenario: Statusänderungen werden von Screenreadern angekündigt
- **WHEN** der Footer-Status von grün auf amber wechselt (oder umgekehrt)
- **THEN** kündigt ein `aria-live="polite"`-Bereich die Änderung an, ohne den Lesefluss zu unterbrechen

#### Scenario: Farbkodierung ist nicht der einzige Unterschied
- **WHEN** der Footer einen der vier Zustände zeigt
- **THEN** unterscheiden sich die Zustände sowohl durch Farbe als auch durch Text (Pille-Label + Beschreibungstext), sodass Nutzer mit Farbsehschwäche die Zustände unterscheiden können

#### Scenario: Kontrast erfüllt WCAG AA
- **WHEN** der Footer in einem der amber Zustände angezeigt wird
- **THEN** erfüllen alle Text-Hintergrund-Kombinationen ein Kontrastverhältnis von mindestens 4,5:1

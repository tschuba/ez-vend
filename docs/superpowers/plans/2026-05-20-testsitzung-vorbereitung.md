# Testsitzungs-Vorbereitung 2026-05-21 — Implementierungsplan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Alle Materialien für die Testsitzung am 2026-05-21 bereitstellen: Ansage-Skripte, Testplan-Updates, Beobachtungsvorlage, Validierungstabelle (mit Soll-Werten), Rauchtest, Leitfaden, Referenz-Testdaten.

**Architecture:** HTML-Dokumente für den Druckbetrieb, JSON-Testdaten über App-UI erzeugen, parallel 4 Code-Review-Agenten dispatchen. Alle Dokumente auf Deutsch.

**Tech Stack:** HTML/CSS (kein Framework), JSON (App-eigenes Export-Format), Claude Agents für Code Review.

**Veranstaltungskonfiguration für alle Dokumente:**
- Name: **Bazar März 2026**
- Datum: **29.03.2026**
- Teilnahmegebühr: **1,00 €**
- Umsatzbeteiligung: **15 %**
- Rundungsschritt: **0,50 €**

**Gebührenformel:**
```
theoretical_net = gross - 1,00 - gross × 0,15
net = commercial_round(theoretical_net, 0,50)   ← 0,5 rundet HOCH
fees = gross - net
floor bei 0,00 € (kein negativer Nettobetrag)
```

---

## Referenzdaten: Kassiersequenzen

### Team A — K-01 bis K-07

| Vorgang | Pos | Verk. | Betrag | Hinweis |
|---------|-----|-------|--------|---------|
| K-01 | 1 | V1 | 5,50 € | Wechsel-Fall |
| K-01 | 2 | V3 | 12,00 € | |
| K-01 | 3 | V1 | 8,00 € | zurück zu V1 |
| K-01 | 4 | V2 | 14,50 € | |
| K-01 | 5 | V4 | 7,00 € | |
| K-01 | 6 | V1 | 9,50 € | zweite Rückkehr |
| K-01 | 7 | V3 | 6,00 € | **Kassensumme: 62,50 €** |
| K-02 | 1 | V2 | 25,00 € | |
| K-02 | 2 | V4 | 8,00 € | |
| K-02 | 3 | V5 | 15,50 € | |
| K-02 | 4 | V2 | 11,00 € | |
| K-02 | 5 | V6 | 20,00 € | |
| K-02 | 6 | V5 | 8,50 € | **Kassensumme: 88,00 €** |
| K-03 | 1 | V1 | 15,00 € | |
| K-03 | 2 | V3 | 6,50 € | |
| K-03 | 3 | V5 | 10,00 € | Sequenz-Fall |
| K-03 | 4 | V5 | 10,00 € | V5 direkt erneut |
| K-03 | 5 | V6 | 18,50 € | |
| K-03 | 6 | V2 | 12,00 € | |
| K-03 | 7 | V4 | 5,50 € | |
| K-03 | 8 | V1 | 7,00 € | **Kassensumme: 84,50 €** |
| K-04 | 1 | V6 | 45,00 € | |
| K-04 | 2 | V1 | 12,00 € | |
| K-04 | 3 | V3 | 8,50 € | |
| K-04 | 4 | V2 | 9,50 € | |
| K-04 | 5 | V7 | 1,00 € | Netto < 0 vor Begrenzung → 0,00 € |
| | | | | **Kassensumme: 76,00 €** |
| K-05 | 1 | V4 | 11,00 € | |
| K-05 | 2 | V6 | 22,50 € | |
| K-05 | 3 | V1 | 8,00 € | |
| K-05 | — | — | — | **[PAUSE ~30 s — Draft-Persistenz prüfen]** |
| K-05 | 4 | V5 | 14,00 € | |
| K-05 | 5 | V3 | 6,50 € | |
| K-05 | 6 | V2 | 7,50 € | **Kassensumme: 69,50 €** |
| K-06 | 1 | V6 | 49,50 € | nahe Maximum |
| K-06 | 2 | V2 | 23,00 € | |
| K-06 | 3 | V4 | 18,50 € | |
| K-06 | 4 | V1 | 11,00 € | |
| K-06 | 5 | V5 | 7,50 € | |
| K-06 | 6 | V3 | 13,50 € | |
| K-06 | 7 | V6 | 15,00 € | **Kassensumme: 138,00 €** |
| K-07 | 1 | V3 | 9,00 € | |
| K-07 | 2 | V5 | 12,50 € | |
| K-07 | 3 | V1 | 16,00 € | |
| K-07 | 4 | V2 | 8,50 € | |
| K-07 | 5 | V4 | 5,00 € | **Kassensumme: 51,00 €** |

### Team B — K-08 bis K-14

| Vorgang | Pos | Verk. | Betrag | Hinweis |
|---------|-----|-------|--------|---------|
| K-08 | 1 | V6 | 28,50 € | |
| K-08 | 2 | V8 | 0,50 € | Netto = 0-Fall |
| K-08 | 3 | V1 | 9,00 € | |
| K-08 | 4 | V3 | 14,00 € | |
| K-08 | 5 | V2 | 11,50 € | **Kassensumme: 63,50 €** |
| K-09 | 1 | V2 | 18,50 € | Sequenz-Fall |
| K-09 | 2 | V2 | 10,00 € | V2 direkt erneut |
| K-09 | 3 | V1 | 9,00 € | |
| K-09 | 4 | V4 | 12,50 € | |
| K-09 | 5 | V5 | 8,00 € | |
| K-09 | 6 | V3 | 7,50 € | |
| K-09 | 7 | V6 | 15,50 € | **Kassensumme: 81,00 €** |
| K-10 | 1 | V1 | 7,50 € | Wechsel-Fall |
| K-10 | 2 | V5 | 12,00 € | |
| K-10 | 3 | V1 | 5,00 € | zurück zu V1 |
| K-10 | 4 | V6 | 26,50 € | |
| K-10 | 5 | V4 | 4,00 € | |
| K-10 | 6 | V3 | 9,50 € | |
| K-10 | 7 | V2 | 7,00 € | |
| K-10 | 8 | V5 | 6,00 € | **Kassensumme: 77,50 €** |
| K-11 | 1 | V3 | 22,00 € | |
| K-11 | 2 | V2 | 14,00 € | |
| K-11 | 3 | V4 | 8,50 € | |
| K-11 | 4 | V6 | 19,00 € | |
| K-11 | 5 | V1 | 11,00 € | |
| K-11 | 6 | V5 | 13,50 € | **Kassensumme: 88,00 €** |
| K-12 | 1 | V6 | 35,00 € | |
| K-12 | 2 | V3 | 8,50 € | |
| K-12 | 3 | V5 | 11,00 € | |
| K-12 | — | — | — | **[PAUSE ~30 s — Draft-Persistenz prüfen]** |
| K-12 | 4 | V4 | 15,00 € | |
| K-12 | 5 | V2 | 9,50 € | |
| K-12 | 6 | V1 | 7,50 € | |
| K-12 | 7 | V6 | 12,00 € | **Kassensumme: 98,50 €** |
| K-13 | 1 | V6 | 47,50 € | nahe Maximum |
| K-13 | 2 | V4 | 21,00 € | |
| K-13 | 3 | V2 | 13,50 € | |
| K-13 | 4 | V5 | 9,00 € | |
| K-13 | 5 | V3 | 16,00 € | |
| K-13 | 6 | V1 | 8,50 € | **Kassensumme: 115,50 €** |
| K-14 | 1 | V5 | 14,00 € | |
| K-14 | 2 | V3 | 11,50 € | |
| K-14 | 3 | V2 | 7,00 € | |
| K-14 | 4 | V4 | 6,00 € | |
| K-14 | 5 | V6 | 18,50 € | **Kassensumme: 57,00 €** |

---

## Soll-Werte (Gebührenberechnung)

### Gerät A — Vor Merge

| Verkäufer | Brutto | Netto | Gebühren |
|-----------|--------|-------|----------|
| V1 | 92,00 € | 77,00 € | 15,00 € |
| V2 | 111,00 € | 93,50 € | 17,50 € |
| V3 | 62,00 € | 51,50 € | 10,50 € |
| V4 | 55,00 € | 46,00 € | 9,00 € |
| V5 | 78,00 € | 65,50 € | 12,50 € |
| V6 | 170,50 € | 144,00 € | 26,50 € |
| V7 | 1,00 € | 0,00 € | 1,00 € |
| **Gesamt** | **569,50 €** | **477,50 €** | **92,00 €** |

Herleitung V1: theoretical_net = 92,00 − 1,00 − 13,80 = 77,20 → 77,20/0,50 = 154,4 → 154 × 0,50 = 77,00  
Herleitung V4: theoretical_net = 55,00 − 1,00 − 8,25 = 45,75 → 45,75/0,50 = 91,5 → **92** × 0,50 = 46,00 (kaufm. Rundung)  
Herleitung V7: theoretical_net = 1,00 − 1,00 − 0,15 = −0,15 → floor → 0,00

### Gerät B — Vor Merge

| Verkäufer | Brutto | Netto | Gebühren |
|-----------|--------|-------|----------|
| V1 | 57,50 € | 48,00 € | 9,50 € |
| V2 | 91,00 € | 76,50 € | 14,50 € |
| V3 | 89,00 € | 74,50 € | 14,50 € |
| V4 | 67,00 € | 56,00 € | 11,00 € |
| V5 | 73,50 € | 61,50 € | 12,00 € |
| V6 | 202,50 € | 171,00 € | 31,50 € |
| V8 | 0,50 € | 0,00 € | 0,50 € |
| **Gesamt** | **581,00 €** | **487,50 €** | **93,50 €** |

Herleitung V8: theoretical_net = 0,50 − 1,00 − 0,075 = −0,575 → floor → 0,00

### Nach Merge — Soll (NUR Sitzungsleiter)

| Verkäufer | Brutto | Netto | Gebühren |
|-----------|--------|-------|----------|
| V1 | 149,50 € | 126,00 € | 23,50 € |
| V2 | 202,00 € | 170,50 € | 31,50 € |
| V3 | 151,00 € | 127,50 € | 23,50 € |
| V4 | 122,00 € | 102,50 € | 19,50 € |
| V5 | 151,50 € | 128,00 € | 23,50 € |
| V6 | 373,00 € | 316,00 € | 57,00 € |
| V7 | 1,00 € | 0,00 € | 1,00 € |
| V8 | 0,50 € | 0,00 € | 0,50 € |
| **Gesamt** | **1150,50 €** | **970,50 €** | **180,00 €** |

Probe: 569,50 + 581,00 = 1150,50 ✓ · 477,50 + 487,50 ≠ 970,50 (Netto wird auf kombinierter Basis gerechnet) ✓  
Herleitung V5: combined gross = 151,50; theoretical = 151,50−1,00−22,725 = 127,775; 255,55 → 256 × 0,50 = 128,00

---

## Aufgabenliste

### Task 1: Parallele Code-Review-Agenten dispatchen

- [ ] 4 Agenten gleichzeitig starten (je einen Message-Block pro Agent):
  - **Code Reviewer** → `crates/ez-vend-ui/src/pages/checkout.rs` (2920 Zeilen) + `import_service.rs`: Grenzfälle, Datenverlust, unerwartete Zustände
  - **Security Engineer** → Import-Validierung, Prüfsummen-Integrität, browser-spezifische Storage-Risiken
  - **Backend Architect** → Merge-Konsistenz, Konfliktauflösung, Datenverlust-Szenarien Mehrgerätebetrieb
  - **Evidence Collector (QA)** → Testabdeckungslücken vs. Testsitzungsszenarien, Safari-spezifische Testanalyse
- [ ] Alle Agenten in den Hintergrund schicken (`run_in_background: true`)
- [ ] Ergebnisse in `docs/validation/code_review_findings_2026-05-20.md` sammeln
- Ausgabe fließt in Task 10 (Leitfaden-Risikoregister) ein

### Task 2: Testdaten-Referenzdateien anlegen

Verzeichnis: `docs/validation/testdata/`

- [ ] Verzeichnis anlegen: `mkdir -p docs/validation/testdata`
- [ ] **basis_veranstaltung.json**: App öffnen → Neue Veranstaltung anlegen (Konfiguration oben) → sofort exportieren (0 Käufe) → Datei speichern
- [ ] **geraet_a_vor_merge.json**: Frische App-Instanz → `basis_veranstaltung.json` importieren → K-01 bis K-07 eingeben → exportieren
- [ ] **geraet_b_vor_merge.json**: Frische App-Instanz → `basis_veranstaltung.json` importieren → K-08 bis K-14 eingeben → exportieren
- [ ] **nach_merge_referenz.json**: `geraet_a_vor_merge.json` importieren → `geraet_b_vor_merge.json` mit Merge importieren → exportieren
- [ ] **grenzfall_netto.json**: Neue Veranstaltung (gleiche Konfiguration) → nur V7 1,00 € und V8 0,50 € eingeben → exportieren
- [ ] **korrupter_datensatz.json**: `basis_veranstaltung.json` kopieren → einen Kauf-Datensatz gezielt manipulieren (z. B. `amount`-Feld auf negativen Wert setzen) → speichern

### Task 3: TC_Multidevice_Merge_Device_A_DE.html aktualisieren

Datei: `docs/validation/TC_Multidevice_Merge_Device_A_DE.html`

Änderungen:
- [ ] **Kopfzeile**: Rollen-Header ergänzen (Kassierer/in + Ansager/in — keine Unterschriften). Rolle-Erklärung in Sektion 1 einfügen.
- [ ] **Sektion 1 (Einrichtung)**: Team A startet Greenfield. Kein Import — leere App.
- [ ] **Sektion 2 (Kassiervorgänge)**: Tabelle komplett ersetzen durch K-01 bis K-07 aus den Referenzdaten oben. Spalten: Vorgang | Pos | Verkäufer | Betrag | Hinweis | Kassensumme | OK
- [ ] **Sektion 3 (Prüfung vor Merge)**: Soll-Spalten entfernen. Nur Brutto-Referenz behalten. Tester trägt Netto/Gebühren aus App ein. Tabelle: Verkäufer | Brutto (Soll) | Netto (aus App) | Gebühren (aus App) | OK. Brutto-Werte: V1 92,00 | V2 111,00 | V3 62,00 | V4 55,00 | V5 78,00 | V6 170,50 | V7 1,00 | Gesamt 569,50
- [ ] **Sektion 5 (Prüfung nach Merge)**: Tabelle auf Leer-Felder umstellen. Tester füllt aus App aus. Gesamt-Brutto 1150,50 € sichtbar (ist nur Addition). Edge-Case-Zeilen: V7 Netto 0,00 € (Kontrollfall Netto < 0), V8 Netto 0,00 € (Kontrollfall Netto = 0). Pro-Verkäufer-Zeilen: Brutto sichtbar, Netto/Gebühren leer.
- [ ] **Sektion 6 (Abschluss)**: Unterschrift-Block ENTFERNEN. Nur Ergebnis-Checkbox + Notizfeld behalten.
- [ ] Commit: `docs/validation/TC_Multidevice_Merge_Device_A_DE.html`

### Task 4: TC_Multidevice_Merge_Device_B_DE.html aktualisieren

Datei: `docs/validation/TC_Multidevice_Merge_Device_B_DE.html`

Änderungen identisch zu Task 3, aber:
- [ ] **Sektion 1**: Team B importiert `basis_veranstaltung.json` zu Beginn (kein Greenfield). Schritt: "Veranstaltung importieren (Strategie: Merge oder Replace)" statt manuell anlegen.
- [ ] **Sektion 2**: K-08 bis K-14 aus Referenzdaten
- [ ] **Sektion 3 Brutto-Werte**: V1 57,50 | V2 91,00 | V3 89,00 | V4 67,00 | V5 73,50 | V6 202,50 | V8 0,50 | Gesamt 581,00
- [ ] Sektion 5, 6: gleich wie Task 3
- [ ] Commit: `docs/validation/TC_Multidevice_Merge_Device_B_DE.html`

### Task 5: Ansage-Skript_A_DE.html erstellen

Datei: `docs/validation/Ansage-Skript_A_DE.html`

- [ ] HTML-Datei mit grüner Farbgebung (wie Device A TC) erstellen
- [ ] Inhalt: Titelzeile „Ansage-Skript — Team A", Datum, Hinweis: „Nur Verkäufer-ID und Betrag ansagen — keine Soll-Werte für Abrechnung"
- [ ] Tabelle mit K-01 bis K-07 (alle Positionen aus Referenzdaten oben)
  - Spalten: Vorgang | Pos | Verkäufer-ID | Betrag | Ansager-Hinweis
  - Kassensumme nach jedem Vorgang fett hervorgehoben (eigene Zeile oder letzte Spalte)
  - PAUSE-Zeilen klar markiert (andere Hintergrundfarbe, z. B. gelb)
- [ ] Rollen-Hinweis oben: Ansager/in sagt an, prüft Bildschirm nach jeder Eingabe, notiert Abweichungen
- [ ] Commit: `docs/validation/Ansage-Skript_A_DE.html`

### Task 6: Ansage-Skript_B_DE.html erstellen

Datei: `docs/validation/Ansage-Skript_B_DE.html`

- [ ] HTML-Datei mit grüner Farbgebung (wie Device B TC) erstellen
- [ ] Identisches Format wie Task 5, K-08 bis K-14
- [ ] Rollen-Hinweis: Team B startet mit `basis_veranstaltung.json`-Import
- [ ] Commit: `docs/validation/Ansage-Skript_B_DE.html`

### Task 7: Beobachtungsvorlage_DE.html erstellen

Datei: `docs/validation/Beobachtungsvorlage_DE.html`

- [ ] Zwei Abschnitte:
  1. **Allgemeine UI-Probleme** — Freitextfelder für: Beschreibung, Schritt der Reproduktion, Schweregrad (Hoch/Mittel/Niedrig), Browser/OS, Screenshot-Ref
  2. **Abrechnungs-/Berechnungsfehler** — Tabelle: Vorgang | Verkäufer | Erwarteter Wert | Tatsächlicher Wert | Differenz
- [ ] Kopfzeile: Team (A/B), Gerät, Browser, OS, Tester/in
- [ ] Mindestens 5 Zeilen pro Abschnitt vorbereitet
- [ ] Commit: `docs/validation/Beobachtungsvorlage_DE.html`

### Task 8: Validierungstabelle_DE.html erstellen

Datei: `docs/validation/Validierungstabelle_DE.html`

**NUR FÜR SITZUNGSLEITER — wird erst am Ende aufgedeckt.**

- [ ] Warnhinweis oben: „Nicht vor Sitzungsende öffnen / weitergeben"
- [ ] Abschnitt A: Gerät A Vor-Merge Soll-Werte (aus Soll-Tabelle oben)
- [ ] Abschnitt B: Gerät B Vor-Merge Soll-Werte
- [ ] Abschnitt C: **Drei-Wege-Vergleich Nach Merge** — Tabelle mit Spalten: Verkäufer | Soll (vorausgefüllt) | Gerät A (leer) | Gerät B (leer). Alle Post-Merge-Werte aus Soll-Tabelle oben eintragen.
- [ ] Diagnose-Matrix am Ende: A=B≠Soll → Berechnungsfehler; A≠B → Sync-Fehler; A=B=Soll → bestanden
- [ ] Commit: `docs/validation/Validierungstabelle_DE.html`

### Task 9: Rauchtest_DE.html erstellen

Datei: `docs/validation/Rauchtest_DE.html`

5-Minuten-Schnellprüfung vor dem Hauptlauf.

- [ ] Checkliste mit Zeitschätzung pro Punkt (Gesamt ≤ 5 min):
  1. App lädt ohne Fehler (WASM-Ladezeit < 5 s) ☐
  2. Neue Veranstaltung anlegen mit Konfiguration oben ☐
  3. Einen Kassiervorgang mit 3 Positionen eingeben ☐
  4. Kassiersumme stimmt ☐
  5. Vendor-Abrechnung für Test-Vendor prüfen (Netto-Berechnung korrekt) ☐
  6. Export funktioniert (Datei wird heruntergeladen) ☐
  7. Import mit Merge funktioniert (geraet_a_vor_merge.json) ☐
  8. Netto=0-Fall: V8 mit 0,50 € eingeben → Netto 0,00 € bestätigt ☐
  9. Browser-Tab-Reload → Draft noch vorhanden ☐
  10. Auf zweitem Gerät/Browser testen ☐
- [ ] Ergebnis-Block: ☐ Alle OK — Hauptlauf starten / ☐ Probleme → [Notizfeld]
- [ ] Hinweis: Alle ❌/⚠️ aus Code-Review-Ergebnissen hier als eigene Punkte ergänzen (nach Task 1)
- [ ] Commit: `docs/validation/Rauchtest_DE.html`

### Task 10: Testsitzungs_Leitfaden_DE.html erstellen

Datei: `docs/validation/Testsitzungs_Leitfaden_DE.html`

**Abhängigkeit:** Task 1 (Code-Review-Ergebnisse) sollte abgeschlossen sein.

- [ ] Abschnitte:
  1. **Überblick** — Datum, Teams, Geräte, Ziel
  2. **Rollen** — Kassierer/in und Ansager/in: je 3-4 Bullet Points zu Verantwortlichkeiten
  3. **Zeitplan** — Tabelle aus Spec (7 Phasen, 45 min total)
  4. **Browser-Empfehlung** — bevorzugte Browser/OS-Kombinationen, bekannte Workarounds
  5. **Koordination Team A ↔ Team B** — Merge-Ablauf, wer exportiert zuerst, Dateiübertragung
  6. **Risikoregister** — Tabelle aus Code-Review-Ausgabe: Befund | Schweregrad (🔴/🟡/⚫) | Workaround
  7. **Notfallplan** — Was tun bei: App startet nicht / Export schlägt fehl / Merge-Konflikt
- [ ] Risikoregister aus `code_review_findings_2026-05-20.md` übernehmen
- [ ] Commit: `docs/validation/Testsitzungs_Leitfaden_DE.html`

### Task 11: TC_Multidevice_Merge_Overview_DE.html aktualisieren

Datei: `docs/validation/TC_Multidevice_Merge_Overview_DE.html`

- [ ] Verweise auf alle neuen Dokumente eintragen:
  - `Testsitzungs_Leitfaden_DE.html`
  - `Rauchtest_DE.html`
  - `Ansage-Skript_A_DE.html`
  - `Ansage-Skript_B_DE.html`
  - `Beobachtungsvorlage_DE.html`
  - `Validierungstabelle_DE.html` (mit Hinweis: nur Sitzungsleiter)
  - `testdata/` (6 Dateien)
- [ ] Commit: `docs/validation/TC_Multidevice_Merge_Overview_DE.html`

### Task 12: TESTING.md reparieren

Datei: `TESTING.md`

- [ ] Referenz auf `docs/validation/SAFARI_VALIDATION_CHECKLIST.md` → Hinweis: „Safari-spezifische Prüfpunkte sind in `Rauchtest_DE.html` integriert" ersetzen
- [ ] Referenz auf `docs/validation/UAT_Ausfuehrungsplan_DE_EN.html` → Verweis auf `Testsitzungs_Leitfaden_DE.html` ersetzen
- [ ] Commit: `TESTING.md`

---

## Ausführungsreihenfolge

```
Task 1 (Agenten starten, Hintergrund) → parallel zu allem anderen
Task 2 (Testdaten, manuell via App-UI)
Task 3 + Task 4 (TC-Dokumente aktualisieren)
Task 5 + Task 6 (Ansage-Skripte)
Task 7 (Beobachtungsvorlage)
Task 8 (Validierungstabelle)
Task 9 (Rauchtest)
Task 10 (Leitfaden — nach Task 1-Ergebnissen)
Task 11 (Overview)
Task 12 (TESTING.md)
```

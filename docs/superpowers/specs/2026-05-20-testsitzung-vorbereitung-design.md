# Design Spec: Testsitzungs-Vorbereitung ez-vend

**Datum:** 2026-05-20  
**Testsitzung:** 2026-05-21 (morgen Abend)  
**Erstes echtes Event:** Oktober 2026  
**Status:** Zur Implementierung freigegeben

---

## Kontext

Vorbereitung einer strukturierten Testsitzung mit zwei Teams à zwei Personen auf gemischten Geräten (Mac, Windows, Tablets). Ziel: maximale Sicherheit vor dem ersten echten Event im Oktober 2026. Priorität liegt auf dem Kassiervorgang, gefolgt von Datenkonsistenz und Geräte-übergreifender Synchronisation via Export/Import-Merge.

Die Sitzung dauert **45 Minuten**. Beide Teams arbeiten parallel an derselben Veranstaltung, starten jedoch von unterschiedlichen Ausgangszuständen.

---

## Teamstruktur

Jedes Team besteht aus **2 Personen** mit klar definierten Rollen:

### Kassierer/in
- Bedient das Gerät und gibt Verkäufer-IDs und Beträge ein
- Gibt exakt das ein, was angesagt wird — kein Interpretieren, kein Runden
- Bestätigt jeden Eintrag, bevor der nächste angesagt wird

### Ansager/in (= Kontrolleur/in)
- Liest vom Ansage-Skript in der vorgegebenen Reihenfolge vor
- Sagt Verkäufer-ID und Betrag klar und deutlich an
- Beobachtet nach jeder Eingabe den Bildschirm und bestätigt, dass die Anzeige dem Angesagten entspricht
- Notiert Abweichungen sofort in der Beobachtungsvorlage
- Übernimmt beide Rollen: Ansagen und Kontrollieren

Die Rollen werden vor Beginn der Sitzung einmal kurz erklärt — keine Unterschriften, keine Formalitäten.

---

## Test-Tracks

### Team A — Greenfield (Leerstart)
- Startet mit einer vollständig leeren Anwendung: keine Veranstaltungen, keine Verkäufer, keine Käufe
- Legt die Veranstaltung manuell an und konfiguriert alle Gebühren
- Gibt alle Kassiervorgänge wie angesagt ein
- Testet den vollständigen Erstbenutzungspfad

### Team B — Vorbereitete Daten
- Importiert `basis_veranstaltung.json` zu Beginn: Veranstaltung und Gebühren bereits konfiguriert, null Käufe
- Überspringt die Veranstaltungsanlage, beginnt direkt mit Kassiervorgängen
- Testet das reale Szenario: Organisator konfiguriert vorab und verteilt die Datei an die Kassierteams

Beide Teams arbeiten gegen **dieselbe Veranstaltung** (gleicher Name, gleiche Gebühren), aber durch unterschiedliche Einstiegswege. Der Merge muss Daten zusammenführen, die aus nativ angelegten und importierten Ausgangszuständen stammen.

---

## Zeitplan (45 Minuten)

| Phase | Dauer | Inhalt |
|---|---|---|
| Setup | 5 min | Team A: Greenfield-Start · Team B: Import `basis_veranstaltung.json` (parallel) |
| Rauchtest | 5 min | Schnellprüfung der kritischen Funktionen vor dem Hauptlauf |
| Kassiervorgänge | 15 min | Beide Teams parallel: 7 Vorgänge, je 5–15 Positionen |
| Prüfung vor Merge | 3 min | Jedes Team prüft den eigenen Gerätestand unabhängig |
| Export · Transfer · Merge | 7 min | Export beider Geräte, Dateiübertragung, Merge-Import |
| Prüfung nach Merge | 4 min | Beide Teams füllen ihre Spalten in der Validierungstabelle aus |
| Validierung + Auswertung | 6 min | Soll-Werte werden aufgedeckt, drei-Wege-Vergleich, Befunde erfassen |

---

## Kassiervolumen

**7 Vorgänge pro Team** (K-01 bis K-07 für Team A, K-08 bis K-14 für Team B)  
**5–15 Positionen pro Vorgang**, Durchschnitt ~10  
**~70 Positionen pro Team**, ~140 kombiniert

Dieses Volumen entspricht einem realistischen Veranstaltungsabschnitt und erlaubt die natürliche Einbettung aller Grenzfälle ohne künstliche Hervorhebung.

### Einzubettende Grenzfälle (gleichmäßig verteilt, nicht markiert)
- Gleicher Verkäufer zweimal direkt hintereinander (Sequenz-Fall)
- Verkäufer-Wechsel innerhalb eines Vorgangs (Wechsel-Fall)
- Rückkehr zu einem früheren Verkäufer innerhalb desselben Vorgangs
- Betrag ergibt Netto < 0,00 € vor Begrenzung → Auszahlung 0,00 €
- Betrag ergibt Netto = 0,00 € genau
- Betrag nahe dem Maximum
- Kurze Pause/Unterbrechung zwischen zwei Positionen (Draft-Persistenz)

---

## Dokumente — Übersicht

### Neu zu erstellen

| Datei | Zweck |
|---|---|
| `Testsitzungs_Leitfaden_DE.html` | Master-Dokument: Rollen, Zeitplan, Koordination, Risikobemerkungen, Browser-Empfehlung |
| `Rauchtest_DE.html` | 5-min-Schnellprüfung vor dem Hauptlauf; enthält alle ❌/⚠️-Grenzfälle aus der Vorab-Prüfung |
| `Ansage-Skript_A_DE.html` | Diktierskript für Team A — Verkäufer-ID und Betrag pro Position, Kassensumme nach jedem Vorgang; keine Soll-Werte für Abrechnung |
| `Ansage-Skript_B_DE.html` | Gleiches Format, andere Daten für Team B |
| `Beobachtungsvorlage_DE.html` | Befunderfassung: allgemeine UI-Probleme + separater Abschnitt für Abrechnungs-/Berichtsfehler |
| `Validierungstabelle_DE.html` | **Nur beim Sitzungsleiter.** Enthält Soll-Werte: Brutto/Netto/Gebühren je Verkäufer und Booth-Gesamt nach Merge. Wird erst am Ende aufgedeckt. |
| `docs/validation/testdata/basis_veranstaltung.json` | Saubere Veranstaltung mit konfigurierten Gebühren, null Käufe |
| `docs/validation/testdata/geraet_a_vor_merge.json` | Zustand Gerät A nach K-01 bis K-07 |
| `docs/validation/testdata/geraet_b_vor_merge.json` | Zustand Gerät B nach K-08 bis K-14 |
| `docs/validation/testdata/nach_merge_referenz.json` | Erwarteter Post-Merge-Zustand |
| `docs/validation/testdata/grenzfall_netto.json` | Verkäufer mit Netto < 0 und Netto = 0 bereits vorhanden |
| `docs/validation/testdata/korrupter_datensatz.json` | Ein absichtlich beschädigter Kaufdatensatz — testet Korruptionserkennung |

### Bestehend — gezielt erweitern

| Datei | Änderung |
|---|---|
| `TC_Multidevice_Merge_Device_A_DE.html` | Rollen-Header hinzufügen (keine Unterschriften); Kassiervorgang-Tabelle auf 7 Vorgänge erweitern; Soll-Spalten entfernen (werden in Validierungstabelle ausgelagert) |
| `TC_Multidevice_Merge_Device_B_DE.html` | Gleiches wie A |
| `TC_Multidevice_Merge_Overview_DE.html` | Referenzen auf alle neuen Dokumente eintragen |
| `TESTING.md` | Verweise auf fehlende Dateien (`SAFARI_VALIDATION_CHECKLIST.md`, `UAT_Ausfuehrungsplan_DE_EN.html`) korrigieren |

---

## Grenzfall-Strategie: Vorab-Prüfung

### Herleitung (Black-Box-First)
Grenzfälle werden aus der Domänenperspektive abgeleitet — was kann einem Veranstaltungsoperator passieren? — nicht aus der Code-Lektüre.

Bereiche:

| Bereich | Beispiel-Grenzfälle |
|---|---|
| **Kassiervorgang** | Gleicher Verkäufer mehrfach · Betrag am Maximum · Falsche Dezimalstellen · Leerer Vorgang bestätigt · Draft-Recovery nach Browser-Reload · Fehlersound |
| **Veranstaltung anlegen** | Doppelter Name · Fehlende Gebührenfelder · Gebühr = 0 % · Rundungsschritt größer als Beträge |
| **Sync / Merge** | Gleiche Kauf-ID von beiden Geräten · Gleicher Verkäufer auf beiden bearbeitet · Datei aus anderer App-Version · Beschädigte/unvollständige Datei |
| **Datenkonsistenz** | Post-Merge-Summen = Summe beider Geräte · Netto-Boden bei 0 · Rundung konsistent |
| **Browser/Gerät** | IndexedDB-Verhalten · localStorage-Limits · WASM-Laden · Datei-Download/-Import |

### Vorab-Prüfung (zweistufig)

**Stufe 1 — Automatisiert (heute Nacht):**  
Jeder Grenzfall wird gegen die bestehende Testsuite geprüft.  
Status: ✅ Abgedeckt · ⚠️ Teilweise · ❌ Offen

**Stufe 2 — Manuell (vor Sitzungsbeginn, Teil des Rauchtests):**  
Alle ❌ und ⚠️ Fälle werden einmal manuell durchgespielt. Ergebnis:
- Besteht → weiter
- Schlägt fehl → vor Sitzung beheben, oder als bekannte Einschränkung dokumentieren mit Workaround

---

## Browser/OS-Kompatibilitätsprüfung

Geprüfte Kombinationen: Chrome, Edge, Firefox, Safari × Windows, macOS  
iOS Safari: notiert als außerhalb des Umfangs für diese Sitzung, für Oktober relevant

Geprüfte Oberflächen: IndexedDB-Verhalten · localStorage · WASM-Laden · CSS/Layout · Datei-Download (Export) · Datei-Import

Ausgabe: **Kompatibilitätsmatrix** (Grün/Gelb/Rot pro Zelle) + **Empfehlung für die Testsitzung** — welcher Browser auf welchem System das geringste Risiko hat. Befunde mit bekannten Workarounds werden inline dokumentiert. Befunde ohne Workaround: Schweregrad *vor Sitzung beheben* / *dokumentieren und fortfahren* / *Kombination meiden*.

---

## Validierung nach Merge (dreistufig)

### Stufe 1 — Abrechnungskorrektheit (jedes Gerät unabhängig)
- Brutto aller erfassten Positionen korrekt
- Gebühren korrekt: Teilnahmegebühr + Umsatzbeteiligung, Rundungsschritt eingehalten
- Netto korrekt, Boden bei 0,00 € eingehalten
- Verkäuferbericht-Ansicht stimmt mit Zusammenfassungszahlen überein

### Stufe 2 — Geräteübergreifende Konsistenz
Beide Teams lesen spezifische Werte vor — das andere Team bestätigt:  
„Gerät A: Verkäufer 3, Brutto 68,50 €" → Gerät B bestätigt: „68,50 € — stimmt"

### Stufe 3 — Drei-Wege-Vergleich gegen Soll-Werte

Die `Validierungstabelle_DE.html` wird erst jetzt aufgedeckt:

| Verkäufer | **Soll** | Gerät A | Gerät B |
|---|---|---|---|
| V1 Brutto | *vorausgefüllt* | ___ | ___ |
| V1 Netto | *vorausgefüllt* | ___ | ___ |
| ... | | | |
| Veranstaltung Gesamt | *vorausgefüllt* | ___ | ___ |

- Gerät A = Gerät B ≠ Soll → Abrechnungs-/Berechnungsfehler
- Gerät A ≠ Gerät B → Sync-/Merge-Fehler
- Gerät A = Gerät B = Soll → bestanden

---

## Code-Review-Abdeckung (parallele Agenten)

Vier Agenten laufen gleichzeitig:

| Agent | Fokus |
|---|---|
| Code Reviewer | `checkout.rs` (2920 Zeilen) + `import_service.rs` — Grenzfälle, Datenverlust-Risiken, unerwartete Zustände |
| Security Engineer | Import-Validierung, Prüfsummen-Integrität, browser-spezifische Storage-Risiken |
| Backend Architect | Merge-Konsistenz, Konfliktauflösung, Datenverlust-Szenarien bei Mehrgerätebetrieb |
| QA / Evidence Collector | Testabdeckungs-Lücken gegen die Testsitzungsszenarien; Safari-spezifische Testanalyse |

Jeder Agent: zuerst Black-Box-Domänenanalyse, dann Implementierungsüberprüfung. Browser/OS-Kompatibilität ist expliziter Teil jedes Review.

Ausgabe fließt in `Testsitzungs_Leitfaden_DE.html` als Risikoregister ein.

---

## Referenzdatensätze

Gespeichert in `docs/validation/testdata/`. Alle Dateien im app-eigenen Export-Format.

| Datei | Inhalt | Primäre Verwendung |
|---|---|---|
| `basis_veranstaltung.json` | Veranstaltung + Gebühren konfiguriert, null Käufe | Team-B-Start · Rauchtest-Reset |
| `geraet_a_vor_merge.json` | Vollständiger Zustand nach K-01 bis K-07 | Merge-Phase isoliert testen |
| `geraet_b_vor_merge.json` | Vollständiger Zustand nach K-08 bis K-14 | Merge-Phase isoliert testen |
| `nach_merge_referenz.json` | Erwarteter Post-Merge-Zustand | Vergleichs-Baseline |
| `grenzfall_netto.json` | V7 (Netto < 0) und V8 (Netto = 0) vorhanden | Isolierter Grenzfall-Test |
| `korrupter_datensatz.json` | Gültiger Export, ein Datensatz manipuliert | Korruptionserkennung prüfen |

Alle Nicht-Korruptionsdateien werden über den App-eigenen Export-Mechanismus erzeugt. `korrupter_datensatz.json` ist ein gültiger Export mit einem gezielt veränderten Datensatz.

---

## Vorab-Aktionsliste (Ausgabe aus Code-Review)

Befunde werden priorisiert nach:
- 🔴 **Vor Sitzung beheben** — blockiert valide Testergebnisse
- 🟡 **Dokumentieren und fortfahren** — bekannte Einschränkung, Workaround vorhanden
- ⚫ **Kombination meiden** — spezifische Browser/OS-Kombination ausschließen

Die vollständige Liste wird nach Abschluss der parallelen Agenten-Reviews befüllt.

---

## Fehlende Dateien beheben

`TESTING.md` referenziert zwei Dateien, die nicht existieren:
- `docs/validation/SAFARI_VALIDATION_CHECKLIST.md` — Safari-spezifische Prüfpunkte werden in den Rauchtest integriert; Referenz in `TESTING.md` aktualisieren
- `docs/validation/UAT_Ausfuehrungsplan_DE_EN.html` — Referenz entfernen oder durch `Testsitzungs_Leitfaden_DE.html` ersetzen

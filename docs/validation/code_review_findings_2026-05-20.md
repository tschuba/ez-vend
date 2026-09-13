# Code Review Findings — 2026-05-20

Vier Agenten-Reviews (Code Reviewer, Security Engineer, Backend Architect, QA) für die Testsitzung 2026-05-21.
Zieldateien: `checkout.rs`, `import_service.rs`, `import_validator.rs`, `backup_format.rs`, `dto.rs`, `settings_support.rs`.

---

## HOCH — Sitzungsrelevant

### H-1 · Kein Floor-Guard in `calculate_payout` — negativer Netto möglich

**Datei:** `crates/domain/src/services/dto.rs:108`  
**Quelle:** QA-Agent (Szenario 3 + 4), Security-Agent (Finding 2)

`theoretical_net = gross − participation_fee − gross × sales_fee_percent` wird nicht auf 0,00 € begrenzt.
Konfiguration Bazar März 2026: TG 1,00 €, UA 15 %. V7 mit 1,00 € Brutto ergibt `1,00 − 1,00 − 0,15 = −0,15`.
`round_to_step(−0.15, 0.50)` liefert 0,00 (zufällig korrekt bei Rundungsschritt 0,50), aber mit
anderen Rundungsschritten (0,25, 1,00) wäre das Ergebnis negativ. Kein `max(ZERO, net)` vorhanden.

**Testergebnis morgen:** Für die aktuelle Konfiguration (RS 0,50 €) tritt das Problem nicht auf —
V7 1,00 € und V8 0,50 € zeigen voraussichtlich korrekt 0,00 € Netto. Risiko bleibt latent für andere Konfigurationen.

**Maßnahme nach Sitzung:** `max(Decimal::ZERO, round_to_step(theoretical_net, step))` in `calculate_payout`.

---

### H-2 · Draft wird ohne Booth-ID-Prüfung wiederhergestellt ✅ Behoben

**Datei:** `crates/ez-vend-ui/src/pages/checkout.rs:683–695`  
**Quelle:** Code-Reviewer-Agent (Finding 1)  
**Behoben in:** commit `546497f`

`DraftLoadOutcome::Restored` trägt eine `booth_id`, die beim Wiederherstellen mit `..` destructured
und nie gegen die aktuelle Booth-ID geprüft wird. Geräte, die zwischen zwei Veranstaltungen wechseln
oder nach einer Pause eine andere Veranstaltung öffnen, stellen den falschen Draft still wieder her.

**Fix:** `load_saved_form_data()` ist jetzt in einen Block gewrappt, der `draft_bid` gegen
`selected_booth.get_untracked()` prüft. Stimmen beide `Some`-Werte nicht überein, wird
`DraftLoadOutcome::Empty` zurückgegeben — stiller Verwerfen, kein neuer Toast, kein neuer i18n-Key.
Alte Drafts ohne `booth_id` (`None`) werden rückwärtskompatibel weiterhin wiederhergestellt.

---

### H-3 · Doppel-Submit erzeugt doppelte Kaufvorgänge ✅ Behoben

**Datei:** `crates/ez-vend-ui/src/pages/checkout.rs:1514–1631, 2213–2229`  
**Quelle:** Code-Reviewer-Agent (Finding 2)  
**Behoben in:** commit `546497f`

`submit_purchase` spawnte `spawn_local` ohne Submit-Guard. Der Submit-Button hatte kein `disabled`-Prop.
Zwei schnelle Taps oder Enter-Doppeldruck liefen beide durch und erzeugten zwei `Purchase`-Datensätze
mit verschiedenen UUIDs — beide in IndexedDB gespeichert und in der Abrechnung.

**Fix:** `is_submitting: RwSignal<bool>` hinzugefügt. Wird auf `true` gesetzt unmittelbar vor
`spawn_local`, auf `false` zurückgesetzt an beiden Async-Exitpoints (Vendor-Fehler-`return` und
nach dem Purchase-Save-Match). Der Submit-Button erhält `disabled=Signal::derive(move || is_submitting.get())`
nach dem bereits etablierten Muster der Datei (vgl. Zeile 2664).

---

### H-4 · Kein transaktionaler Import — Teilimport bei Absturz möglich

**Datei:** `crates/storage/src/export/import_service.rs`  
**Quelle:** Architektur-Agent (Finding 1)

`import_all` schreibt Booths, Vendors und Purchases in getrennten `await`-Aufrufen ohne übergreifende
IndexedDB-Transaktion. Ein Browser-Absturz oder Tab-Reload nach dem 5. von 14 Purchases hinterlässt
die DB in einem inkonsistenten Teilzustand ohne Fehleranzeige.

**Testergebnis morgen:** Niedrige Wahrscheinlichkeit bei stabiler Verbindung, aber katastrophal wenn
es passiert. Nach jedem Import die Kaufanzahl (14) und Gesamt-Brutto (1.150,50 €) prüfen.

---

## MITTEL — Bekannte Einschränkungen

### M-1 · FeeConfig wird bei Merge still überschrieben

**Datei:** `crates/storage/src/export/import_service.rs:151–160`  
**Quelle:** Architektur-Agent (Finding 2), Code-Reviewer-Agent (Finding 4)

Merge-Strategie: Booth-Record mit neuerem `updated_at` gewinnt vollständig, einschließlich FeeConfig.
Wenn ein Gerät die Booth-Konfiguration ändert, überschreibt der Import die Gebührenparameter ohne Warnung.

**Testergebnis morgen:** Kein Risiko solange beide Geräte dieselbe Basisdatei nutzen und die
Booth-Konfiguration nicht ändern. **Vor dem Merge: FeeConfig auf beiden Geräten überprüfen.**

---

### M-2 · Vendor-Merge überschreibt alle Felder außer `created_at`

**Datei:** `crates/storage/src/export/import_service.rs:193–200`  
**Quelle:** Architektur-Agent (Finding 4), Code-Reviewer-Agent (Finding 5)

`..incoming` übernimmt alle Vendor-Felder außer `created_at`. Manuell gesetzte `payout_correction`-Werte
werden bei einem nachfolgenden Merge-Import überschrieben.

**Testergebnis morgen:** Kein Risiko, da keine Payout-Korrekturen vor dem Merge gesetzt werden.

---

### M-3 · Fehlende Prüfsummen-Warnung im Import-UI

**Datei:** `crates/storage/src/export/import_validator.rs:43–83`  
**Quelle:** Security-Agent (Finding 1)

Importdateien ohne `checksum`-Feld werden ohne UI-Warnung akzeptiert (`warn!()` im Log ist nicht sichtbar).
Eine manuell manipulierte Datei ist dadurch nicht vom Original zu unterscheiden.

**Testergebnis morgen:** Testdateien haben keine Prüfsumme (manuell erstellt) — der Import läuft durch,
was korrekt ist. Für den Oktober-Event: Prüfsummen-Warnung im UI ergänzen.

---

### M-4 · `sales_fee_percent > 100` wird im Import nicht abgelehnt

**Datei:** `crates/storage/src/export/import_validator.rs:191–253`  
**Quelle:** Security-Agent (Finding 2)

`FeeConfig::validate_ranges()` wird in `validate_records` nicht aufgerufen. Ein import mit
`sales_fee_percent: 150` wird gespeichert; `calculate_payout` liefert dann negative Netto-Werte.

**Testergebnis morgen:** Kein Risiko mit den vorbereiteten Testdaten (korrekte Konfiguration).
**Maßnahme nach Sitzung:** `booth.fees.validate_ranges()` in `validate_records` aufrufen.

---

### M-5 · Draft-Persistenz ohne Browser-Test (Reload-Szenario)

**Datei:** `crates/ez-vend-ui/src/pages/checkout.rs` + localStorage  
**Quelle:** QA-Agent (Szenario 1)

Nur ein Unit-Test für `parse_stored_form_data` vorhanden. Kein `wasm_bindgen_test`, der den
vollständigen Reload-Zyklus (Eintippen → localStorage → Reload → Restore) im Browser testet.
Das PAUSE-Szenario in K-05 und K-12 ist daher nicht automatisch abgesichert.

**Testergebnis morgen:** PAUSE-Schritte in K-05 (Team A, 3 Items) und K-12 (Team B, 3 Items)
explizit prüfen. Beobachtungsvorlage Abschnitt A bereit halten.

---

## NIEDRIG — Nicht sitzungskritisch

| ID | Befund | Datei |
|----|--------|-------|
| L-1 | XSS im Print-HTML: `context.details` ohne `html_escape()` | `settings_support.rs:153–164` |
| L-2 | localStorage-Quota-Fehler nur per transientem Toast signalisiert | `checkout.rs:593–648` |
| L-3 | Kein Limit für Items pro Purchase — DoS durch manipulierte Datei | `import_validator.rs:255–299` |
| L-4 | `AmountInputMode::RightToLeft` wird nach Reload auf Regular zurückgesetzt | `checkout.rs:356–368` |
| L-5 | Import-Validator prüft Vendor-Referenzen nur gegen Import-Datei, nicht gegen DB | `import_validator.rs:134–180` |
| L-6 | Booth-Merge-Tiebreak = `updated_at` (Konfig-Änderung), nicht Checkout-Zeit | `import_service.rs:152–156` |

---

## Empfehlung für Sitzungsbeginn (2026-05-21)

1. **Beide Geräte: FeeConfig vor dem ersten Kassiervorgang prüfen** (TG 1,00 €, UA 15 %, RS 0,50 €).
2. **Kassierer anweisen, langsam zu tippen** — kein Doppel-Tap auf den Submit-Button (H-3).
3. **Nach dem Merge: Kaufanzahl (14) und Gesamt-Brutto (1.150,50 €) verifizieren** (H-4).
4. **PAUSE-Reload in K-05 und K-12 bewusst testen** — Draft-Restore beobachten (M-5).
5. **Nur Merge-Strategie verwenden**, nie Replace — schützt gegen Datenverlust.

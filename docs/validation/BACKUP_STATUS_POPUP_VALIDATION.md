---
title: Backup-Status-Popup
parent: Validation
---

# Backup-Status-Popup – manuelle Validierung

Gilt für das Status-Pill im Footer (`StorageIndicator`) und sein Popup mit
Sofort-Export. Automatisiert abgedeckt: Statusauswahl (`backup_status`,
Unit-Tests) und Öffnen per Klick / Schließen per Escape (Browser-Test in
`storage_warning.rs`). Touch-Verhalten und Safari sind **nur manuell** prüfbar,
insbesondere auf einem **echten iOS-Gerät** (Simulator reicht ggf. nicht).

Start: `trunk serve` in `crates/app`. Zustände erzeugen:

- **Grün:** Backup exportieren, danach nichts ändern (nicht-Safari: Backup < 30 Tage).
- **Ungesicherte Änderungen:** nach dem Backup einen Verkäufer/Kauf anlegen.
- **Überfällig:** frische Datenbank ohne Backup (nicht-Safari).
- **Safari:** Safari öffnen, Backup älter als 3 Tage oder keins vorhanden.
- **Laden:** direkt nach Seitenaufruf, bevor die Diagnose geladen ist.

## Touch (iPhone/Safari **und** Android/Chrome)

- [ ] Tap aufs Pill öffnet das Popup linksbündig über dem Pill, ohne am Rand abgeschnitten zu werden.
- [ ] Erneuter Tap aufs Pill schließt es.
- [ ] Tap außerhalb schließt es (der Tap wird durchgereicht und löst die getippte Aktion aus).
- [ ] ✕ im Popup schließt es.
- [ ] Kein „klebendes Hover“: nach dem Schließen bleibt das Popup zu.
- [ ] Button hat mindestens 44 px Höhe und ist gut tippbar.

## Desktop

- [ ] Hover über das Pill zeigt das Popup; Maus ins Popup bewegen hält es offen, der Button ist erreichbar.
- [ ] Klick aufs Pill pinnt das Popup (bleibt nach Verlassen offen); erneuter Klick schließt.
- [ ] Escape und ✕ schließen das Popup.

## Tastatur

- [ ] Tab aufs Pill zeigt das Popup, Tab erreicht ✕ und den Export-Button, Enter löst aus.
- [ ] Escape schließt das Popup.

## Inhalte und Export

- [ ] Jeder der vier Zustände zeigt den passenden Titel und Erklärtext (DE und EN).
- [ ] Grün: Button sekundär, Amber: primär; Beschriftung in beiden Fällen „Jetzt sichern ⬇“ (DE) bzw. „Back up now ⬇“ (EN).
- [ ] Im Zustand „Laden“ gibt es kein Popup und keine Interaktion.
- [ ] Export: Button zeigt Ladezustand, Datei wird heruntergeladen, Toast bestätigt, Popup schließt, Pill wird grün.
- [ ] Export-Fehler: Popup bleibt offen, Toast zeigt den Fehler, Button wieder aktiv.

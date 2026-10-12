---
title: Produkt-Kacheln & Sortierung
parent: Validation
---

# Produkt-Kacheln & Sortierung – manuelle Validierung

Gilt für den Produkte-Tab der Veranstaltungs-Konfiguration (Kacheln, Drag, Modals).
Automatisiert abgedeckt: `apply_reorder`/`move_product` (Unit-Tests). Touch,
Safari und Auto-Scroll sind **nur manuell** prüfbar.

Start: `trunk serve` in `crates/app`, Veranstaltung mit ≥ 2 Gruppen und mehr
Produkten, als dass sie auf einen Bildschirm passen.

## Touch (iPad/Safari **und** Android/Chrome)

- [ ] Wischen über Kacheln scrollt die Seite; es startet kein Drag.
- [ ] Antippen einer Kachel / Gruppenzeile öffnet das Bearbeiten-Modal.
- [ ] Finger ~0,3 s halten: Kachel pulsiert, dann „hebt ab"; danach folgt sie dem Finger (Ghost), die Seite scrollt dabei **nicht**.
- [ ] Ablegen auf einer Kachel setzt das Produkt **davor**; blauer Balken links zeigt das Ziel.
- [ ] Ablegen auf der „+"-Kachel hängt das Produkt ans Ende der Gruppe an.
- [ ] Ablegen in einer anderen (auch leeren) Gruppe verschiebt das Produkt dorthin.
- [ ] Nach dem Ablegen öffnet sich **kein** Bearbeiten-Modal.
- [ ] Ziehen nahe am oberen/unteren Bildschirmrand scrollt die Seite automatisch.
- [ ] Gruppe ziehen: alle Produktgitter klappen zu, Ablegen sortiert die Gruppen um.
- [ ] Gruppe ziehen: blauer Balken in der Lücke vor der Ziel-Gruppe; über „+ Gruppe hinzufügen“ erscheint er unter der letzten Gruppe, Ablegen hängt ans Ende.
- [ ] Kein Kontextmenü/Textauswahl-Lupe beim langen Drücken (iOS).

## Gruppen-Buttons und Modals

- [ ] ▲/▼ verschieben die Gruppe um eine Position; ▲ der ersten und ▼ der letzten Gruppe sind deaktiviert.
- [ ] Neue Gruppe / Gruppe bearbeiten / Neues Produkt / Produkt bearbeiten öffnen als Modal; Tippen neben das Modal schließt es **nicht**, Abbrechen und Escape schon.
- [ ] Löschen nur im Bearbeiten-Modal, zweistufig („Gruppe löschen?" / „Produkt löschen?").
- [ ] Gruppe mit Produkten und bereits verkauftes Produkt: Löschen deaktiviert, Grund als Text sichtbar.
- [ ] Reihenfolge und Gruppenzuordnung bleiben nach Neuladen erhalten.

## Checkout

- [ ] Produktraster im Checkout unverändert (3 Spalten, Tap = Kauf, Long-Press = Mengen-Modal, Bestands-Badge).

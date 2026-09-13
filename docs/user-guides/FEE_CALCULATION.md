---
title: Fee Calculation
nav_order: 1
parent: User Guides
---

# Gebührenberechnung / Fee Calculation

Diese Dokumentation erklärt, wie die Gebühren und Auszahlungen für Verkäufer berechnet werden.

*This document explains how fees and payouts are calculated for vendors.*

---

## Überblick / Overview

**Deutsch:**
Das System berechnet für jeden Verkäufer die Netto-Auszahlung, indem es von den Bruttoverkäufen die Gebühren abzieht. Der Rundungsschritt wird auf die Netto-Auszahlung angewendet, um dem Verkäufer einen sauberen, gerundeten Betrag auszuzahlen.

**English:**
The system calculates the net payout for each vendor by subtracting fees from gross sales. The rounding step is applied to the net payout to give the vendor a clean, rounded amount.

---

## Gebührenstruktur / Fee Structure

Es gibt zwei Arten von Gebühren:

*There are two types of fees:*

1. **Standgebühr / Participation Fee**: Ein fester Betrag, der einmalig erhoben wird / *A fixed amount charged once*
2. **Umsatzbeteiligung / Revenue Share**: Ein Prozentsatz der Bruttoverkäufe / *A percentage of gross sales*

---

## Berechnungslogik / Calculation Logic

### Schritt 1: Bruttoverkäufe / Step 1: Gross Sales

Die Summe aller Verkäufe des Verkäufers.

*The sum of all vendor's sales.*

```
Bruttoverkäufe = Summe aller Artikelpreise
Gross Sales = Sum of all item prices
```

### Schritt 2: Theoretische Netto-Auszahlung / Step 2: Theoretical Net Payout

Berechne die Netto-Auszahlung vor der Rundung:

*Calculate the net payout before rounding:*

```
Theoretische Verkaufsgebühr = Bruttoverkäufe × (Umsatzbeteiligung% / 100)
Theoretische Netto = Bruttoverkäufe - Standgebühr - Theoretische Verkaufsgebühr

Theoretical Sales Fee = Gross Sales × (Revenue Share% / 100)
Theoretical Net = Gross Sales - Participation Fee - Theoretical Sales Fee
```

### Schritt 3: Rundung / Step 3: Rounding

Runde die Netto-Auszahlung auf den nächsten Rundungsschritt:

*Round the net payout to the nearest rounding step:*

```
Netto-Auszahlung = Runde(Theoretische Netto) auf Rundungsschritt
Net Payout = Round(Theoretical Net) to Rounding Step
```

**Rundungsverhalten / Rounding Behavior:**
- Bei **0,50 €**: Rundung auf 0,00 €, 0,50 €, 1,00 €, 1,50 €, usw. / *Round to 0.00 €, 0.50 €, 1.00 €, 1.50 €, etc.*
- Bei **1,00 €**: Rundung auf volle Euro / *Round to full euros*
- Bei **0,25 €**: Rundung auf Vierteleuro / *Round to quarter euros*
- Bei **0,00 €**: Rundung auf Cent (2 Dezimalstellen) / *Round to cents (2 decimal places)*

**Rundungsregel / Rounding Rule:** Kaufmännische Rundung (0,5 wird aufgerundet) / *Commercial rounding (0.5 rounds up)*

### Schritt 4: Tatsächliche Gebühren / Step 4: Actual Fees

Berechne die tatsächlichen Gebühren als Differenz:

*Calculate the actual fees as the difference:*

```
Gebühren gesamt = Bruttoverkäufe - Netto-Auszahlung
Total Fees = Gross Sales - Net Payout
```

---

## Beispielrechnung / Example Calculation

### Konfiguration / Configuration

- **Standgebühr / Participation Fee**: 10,00 €
- **Umsatzbeteiligung / Revenue Share**: 15%
- **Rundungsschritt / Rounding Step**: 0,50 €

### Berechnung / Calculation

**Schritt 1: Bruttoverkäufe / Step 1: Gross Sales**
```
Bruttoverkäufe = 518,11 €
Gross Sales = 518.11 €
```

**Schritt 2: Theoretische Netto-Auszahlung / Step 2: Theoretical Net Payout**
```
Theoretische Verkaufsgebühr = 518,11 € × 15% = 77,72 €
Theoretische Netto = 518,11 € - 10,00 € - 77,72 € = 430,39 €

Theoretical Sales Fee = 518.11 € × 15% = 77.72 €
Theoretical Net = 518.11 € - 10.00 € - 77.72 € = 430.39 €
```

**Schritt 3: Rundung / Step 3: Rounding**
```
Netto-Auszahlung = Runde(430,39 €) auf 0,50 € = 430,50 €
Net Payout = Round(430.39 €) to 0.50 € = 430.50 €
```

**Schritt 4: Tatsächliche Gebühren / Step 4: Actual Fees**
```
Gebühren gesamt = 518,11 € - 430,50 € = 87,61 €
Total Fees = 518.11 € - 430.50 € = 87.61 €
```

### Endergebnis / Final Result

| Beschreibung / Description | Betrag / Amount |
|---------------------------|-----------------|
| **Bruttoverkäufe** / *Gross Sales* | 518,11 € |
| **Gebühren gesamt** / *Total Fees* | 87,61 € |
| **Netto-Auszahlung** / *Net Payout* | **430,50 €** |

---

## Weitere Beispiele / Additional Examples

### Beispiel 2: Rundungsschritt 1,00 € / Example 2: Rounding Step 1.00 €

**Konfiguration / Configuration:**
- Standgebühr / Participation Fee: 5,00 €
- Umsatzbeteiligung / Revenue Share: 10%
- Rundungsschritt / Rounding Step: 1,00 €

**Berechnung mit 100,50 € Bruttoverkäufe / Calculation with 100.50 € Gross Sales:**

1. Theoretische Verkaufsgebühr = 100,50 € × 10% = 10,05 €
2. Theoretische Netto = 100,50 € - 5,00 € - 10,05 € = 85,45 €
3. **Netto-Auszahlung = 85,00 €** (gerundet auf volle Euro / *rounded to full euro*)
4. Gebühren gesamt = 100,50 € - 85,00 € = 15,50 €

### Beispiel 3: Rundungsschritt 0,25 € / Example 3: Rounding Step 0.25 €

**Konfiguration / Configuration:**
- Standgebühr / Participation Fee: 2,00 €
- Umsatzbeteiligung / Revenue Share: 12%
- Rundungsschritt / Rounding Step: 0,25 €

**Berechnung mit 50,00 € Bruttoverkäufe / Calculation with 50.00 € Gross Sales:**

1. Theoretische Verkaufsgebühr = 50,00 € × 12% = 6,00 €
2. Theoretische Netto = 50,00 € - 2,00 € - 6,00 € = 42,00 €
3. **Netto-Auszahlung = 42,00 €** (bereits gerundet / *already rounded*)
4. Gebühren gesamt = 50,00 € - 42,00 € = 8,00 €

### Beispiel 4: Kein Rundungsschritt (0,00 €) / Example 4: No Rounding Step (0.00 €)

**Konfiguration / Configuration:**
- Standgebühr / Participation Fee: 3,00 €
- Umsatzbeteiligung / Revenue Share: 8%
- Rundungsschritt / Rounding Step: 0,00 €

**Berechnung mit 47,33 € Bruttoverkäufe / Calculation with 47.33 € Gross Sales:**

1. Theoretische Verkaufsgebühr = 47,33 € × 8% = 3,79 €
2. Theoretische Netto = 47,33 € - 3,00 € - 3,79 € = 40,54 €
3. **Netto-Auszahlung = 40,54 €** (gerundet auf Cent / *rounded to cents*)
4. Gebühren gesamt = 47,33 € - 40,54 € = 6,79 €

---

## Vorteile dieser Methode / Benefits of This Method

**Deutsch:**

1. **Klare Auszahlungsbeträge**: Verkäufer erhalten saubere, gerundete Beträge (z.B. 430,50 € statt 430,39 €)
2. **Einfaches Handling**: Bargeldauszahlungen sind einfacher, da weniger kleine Münzen benötigt werden
3. **Faire Berechnung**: Die Rundung erfolgt transparent und nachvollziehbar
4. **Flexibilität**: Der Rundungsschritt kann an die Bedürfnisse angepasst werden

**English:**

1. **Clean Payout Amounts**: Vendors receive clean, rounded amounts (e.g., 430.50 € instead of 430.39 €)
2. **Easy Handling**: Cash payouts are easier since fewer small coins are needed
3. **Fair Calculation**: The rounding is transparent and traceable
4. **Flexibility**: The rounding step can be adjusted to meet needs

---

## Hinweise / Notes

**Deutsch:**

- Die Rundung erfolgt **immer zur Netto-Auszahlung**, nicht zu den Gebühren
- Die Standgebühr wird **nie gerundet** und bleibt immer der konfigurierte Betrag
- Die Verkaufsgebühr wird **automatisch angepasst**, damit die Rechnung aufgeht
- Bei einem Rundungsschritt von 0,00 € wird auf **2 Dezimalstellen** (Cent) gerundet

**English:**

- Rounding is **always applied to the net payout**, not to the fees
- The participation fee is **never rounded** and always stays the configured amount
- The revenue share is **automatically adjusted** to make the calculation work
- With a rounding step of 0.00 €, rounding is done to **2 decimal places** (cents)

---

## Technische Implementierung / Technical Implementation

Die Berechnung wird in der Datei `crates/domain/src/services/dto.rs` durch die Methode `ChargingConfig::calculate_payout()` durchgeführt.

*The calculation is performed in the file `crates/domain/src/services/dto.rs` by the method `ChargingConfig::calculate_payout()`.*

Siehe auch die umfassenden Tests in derselben Datei für weitere Beispiele.

*See also the comprehensive tests in the same file for more examples.*

---

**Letzte Aktualisierung / Last Updated:** März 2026

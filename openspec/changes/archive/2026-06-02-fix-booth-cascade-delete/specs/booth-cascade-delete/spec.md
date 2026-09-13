## ADDED Requirements

### Requirement: Veranstaltung löschen entfernt alle zugehörigen Daten atomar
Wenn eine Veranstaltung gelöscht wird, SHALL das System alle zugehörigen Verkäufer und Kassiervorgänge in derselben atomaren Transaktion löschen. Es darf kein Zwischenzustand entstehen, bei dem der Booth-Datensatz gelöscht ist, aber Vendor- oder Purchase-Datensätze noch existieren.

#### Scenario: Löschen entfernt Verkäufer und Kassiervorgänge
- **WHEN** der Nutzer eine Veranstaltung löscht
- **THEN** werden alle Verkäufer dieser Veranstaltung aus dem `vendors`-Store gelöscht
- **THEN** werden alle Kassiervorgänge dieser Veranstaltung aus dem `purchases`-Store gelöscht
- **THEN** wird der Booth-Datensatz aus dem `booths`-Store gelöscht
- **THEN** sind alle drei Löschoperationen Teil einer einzigen Transaktion (atomar)

#### Scenario: Reimport nach Löschen zeigt nur neue Daten
- **WHEN** eine Veranstaltung gelöscht und danach dieselbe Datei erneut importiert wird
- **THEN** enthält die reimportierte Veranstaltung ausschließlich die Daten aus der Importdatei
- **THEN** sind keine Altdaten aus dem vorherigen Import vorhanden

#### Scenario: Löschen einer Veranstaltung ohne Daten
- **WHEN** eine Veranstaltung gelöscht wird, die keine Verkäufer oder Kassiervorgänge hat
- **THEN** wird der Booth-Datensatz erfolgreich gelöscht
- **THEN** schlägt die Operation nicht fehl

#### Scenario: Löschen einer archivierten Veranstaltung
- **WHEN** eine archivierte Veranstaltung gelöscht wird (deren Vendors/Purchases bereits durch Archivierung entfernt wurden)
- **THEN** wird der Booth-Datensatz erfolgreich gelöscht
- **THEN** schlägt die Operation nicht fehl (0 Vendors/Purchases zu löschen ist kein Fehler)

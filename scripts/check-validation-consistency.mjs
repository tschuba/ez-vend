#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

const repoRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

function parseEuro(value) {
  return Number(value.replace(/\s*€/g, '').replace(/\./g, '').replace(',', '.').trim());
}

function formatEuro(value) {
  return value.toFixed(2).replace('.', ',');
}

function roundToStep(value, step = 0.5) {
  return Math.round(value / step) * step;
}

function calculateNet(gross) {
  return Math.max(0, roundToStep(gross - 1 - gross * 0.15));
}

function addToMap(map, key, amount) {
  map.set(key, Number(((map.get(key) || 0) + amount).toFixed(2)));
}

function sumMap(map) {
  return Number([...map.values()].reduce((total, amount) => total + amount, 0).toFixed(2));
}

function parseScriptTransactions(html) {
  const matches = html.matchAll(/Verkäufer\s*<strong>(\d+)<\/strong>\s*[—-]\s*<strong>([0-9]+,[0-9]{2})\s*€/g);
  const totals = new Map();

  for (const [, vendorId, amount] of matches) {
    addToMap(totals, vendorId, parseEuro(amount));
  }

  return totals;
}

function parseScriptSummary(html) {
  const match = html.match(/Gesamt Brutto Team [A-Z]:\s*([0-9.,]+)\s*€/);
  return match ? parseEuro(match[1]) : null;
}

function parseTableSection(html, headingText) {
  const escapedHeading = headingText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = html.match(new RegExp(`<h2>${escapedHeading}<\\/h2>[\\s\\S]*?<tbody>([\\s\\S]*?)<\\/tbody>`, 'm'));
  if (!match) {
    throw new Error(`Section not found: ${headingText}`);
  }

  const tbody = match[1];
  const rowMatches = tbody.matchAll(/<tr[^>]*>([\s\S]*?)<\/tr>/g);
  const vendors = new Map();
  let totalRow = null;

  for (const [, rowHtml] of rowMatches) {
    const cells = [...rowHtml.matchAll(/<td[^>]*>([\s\S]*?)<\/td>/g)].map((cell) =>
      cell[1].replace(/<[^>]+>/g, '').replace(/&lt;/g, '<').replace(/&gt;/g, '>').trim(),
    );
    if (cells.length < 2) {
      continue;
    }

    const vendorMatch = cells[0].match(/V(\d+)/);
    if (vendorMatch) {
      vendors.set(vendorMatch[1], {
        gross: parseEuro(cells[1]),
        net: cells[2] ? parseEuro(cells[2]) : null,
        fees: cells[3] ? parseEuro(cells[3]) : null,
      });
      continue;
    }

    if (/Gesamt/.test(cells[0])) {
      totalRow = {
        gross: parseEuro(cells[1]),
        net: cells[2] ? parseEuro(cells[2]) : null,
        fees: cells[3] ? parseEuro(cells[3]) : null,
      };
    }
  }

  return { vendors, totalRow };
}

function mergeMaps(...maps) {
  const result = new Map();
  for (const map of maps) {
    for (const [key, value] of map.entries()) {
      addToMap(result, key, value);
    }
  }
  return result;
}

function buildPayoutTable(grossMap) {
  const rows = new Map();
  for (const [vendorId, gross] of grossMap.entries()) {
    const net = calculateNet(gross);
    rows.set(vendorId, {
      gross,
      net,
      fees: Number((gross - net).toFixed(2)),
    });
  }
  return rows;
}

function assertEqual(label, actual, expected, errors) {
  if (Number(actual.toFixed(2)) !== Number(expected.toFixed(2))) {
    errors.push(`${label}: expected ${formatEuro(expected)}, got ${formatEuro(actual)}`);
  }
}

function compareRows(label, actualRows, expectedRows, errors) {
  const keys = [...new Set([...actualRows.keys(), ...expectedRows.keys()])].sort((left, right) => Number(left) - Number(right));
  for (const key of keys) {
    const actual = actualRows.get(key);
    const expected = expectedRows.get(key);
    if (!actual || !expected) {
      errors.push(`${label} V${key}: row missing in ${actual ? 'expected' : 'actual'} data`);
      continue;
    }
    assertEqual(`${label} V${key} gross`, actual.gross, expected.gross, errors);
    if (expected.net !== null) {
      assertEqual(`${label} V${key} net`, actual.net, expected.net, errors);
    }
    if (expected.fees !== null) {
      assertEqual(`${label} V${key} fees`, actual.fees, expected.fees, errors);
    }
  }
}

function compareTotal(label, grossMap, expectedTotalRow, errors) {
  if (!expectedTotalRow) {
    errors.push(`${label}: total row missing`);
    return;
  }
  const gross = sumMap(grossMap);
  const net = Number([...grossMap.values()].reduce((total, amount) => total + calculateNet(amount), 0).toFixed(2));
  const fees = Number((gross - net).toFixed(2));
  assertEqual(`${label} total gross`, gross, expectedTotalRow.gross, errors);
  if (expectedTotalRow.net !== null) {
    assertEqual(`${label} total net`, net, expectedTotalRow.net, errors);
  }
  if (expectedTotalRow.fees !== null) {
    assertEqual(`${label} total fees`, fees, expectedTotalRow.fees, errors);
  }
}

const errors = [];

const basis = JSON.parse(read('docs/validation/testdata/basis_veranstaltung.json'));
const teamCFixture = JSON.parse(read('docs/validation/testdata/team_c_hochlast_start.json'));

for (const field of ['description', 'date']) {
  if (basis.booth[field] !== teamCFixture.booth[field]) {
    errors.push(`team_c_hochlast_start booth.${field} differs from basis_veranstaltung`);
  }
}

for (const field of ['participation_fee', 'sales_fee_percent', 'rounding_step']) {
  if (basis.booth.fees[field] !== teamCFixture.booth.fees[field]) {
    errors.push(`team_c_hochlast_start booth.fees.${field} differs from basis_veranstaltung`);
  }
}

for (const field of ['vendor_id_validation', 'vendor_id_omission_rules', 'keyboard_config']) {
  if (JSON.stringify(basis.booth[field]) !== JSON.stringify(teamCFixture.booth[field])) {
    errors.push(`team_c_hochlast_start booth.${field} differs from basis_veranstaltung`);
  }
}

if (teamCFixture.purchases.length !== 300) {
  errors.push(`team_c_hochlast_start purchase count: expected 300, got ${teamCFixture.purchases.length}`);
}

const fixtureGross = new Map();
for (const purchase of teamCFixture.purchases) {
  for (const item of purchase.items) {
    addToMap(fixtureGross, item.vendor_id, Number(item.amount));
  }
}

const scriptA = parseScriptTransactions(read('docs/validation/Ansage-Skript_A_DE.html'));
const scriptB = parseScriptTransactions(read('docs/validation/Ansage-Skript_B_DE.html'));
const scriptC = parseScriptTransactions(read('docs/validation/Ansage-Skript_C_DE.html'));

const summaryA = parseScriptSummary(read('docs/validation/Ansage-Skript_A_DE.html'));
const summaryB = parseScriptSummary(read('docs/validation/Ansage-Skript_B_DE.html'));
const summaryC = parseScriptSummary(read('docs/validation/Ansage-Skript_C_DE.html'));

if (summaryA !== null) {
  assertEqual('Ansage-Skript A summary gross', sumMap(scriptA), summaryA, errors);
}
if (summaryB !== null) {
  assertEqual('Ansage-Skript B summary gross', sumMap(scriptB), summaryB, errors);
}
if (summaryC !== null) {
  assertEqual('Ansage-Skript C summary gross', sumMap(scriptC), summaryC - sumMap(fixtureGross), errors);
}

const validationAB = read('docs/validation/Validierungstabelle_DE.html');
const validationACBC = read('docs/validation/Validierungstabelle_AC_BC_DE.html');

const sectionA = parseTableSection(validationAB, 'Teil A — Gerät A vor Merge (Soll)');
const sectionB = parseTableSection(validationAB, 'Teil B — Gerät B vor Merge (Soll)');
const sectionAB = parseTableSection(validationAB, 'Teil C — Drei-Wege-Vergleich nach Merge');
const sectionC = parseTableSection(validationACBC, 'Teil C — Gerät C vor Merge (Soll)');
const sectionAC = parseTableSection(validationACBC, 'Teil AC — Drei-Wege-Vergleich nach Merge');
const sectionBC = parseTableSection(validationACBC, 'Teil BC — Drei-Wege-Vergleich nach Merge');

compareRows('Pre-merge A', buildPayoutTable(scriptA), sectionA.vendors, errors);
compareTotal('Pre-merge A', scriptA, sectionA.totalRow, errors);

compareRows('Pre-merge B', buildPayoutTable(scriptB), sectionB.vendors, errors);
compareTotal('Pre-merge B', scriptB, sectionB.totalRow, errors);

const grossC = mergeMaps(fixtureGross, scriptC);
compareRows('Pre-merge C', buildPayoutTable(grossC), sectionC.vendors, errors);
compareTotal('Pre-merge C', grossC, sectionC.totalRow, errors);

const grossAB = mergeMaps(scriptA, scriptB);
compareRows('Scenario AB', buildPayoutTable(grossAB), sectionAB.vendors, errors);
compareTotal('Scenario AB', grossAB, sectionAB.totalRow, errors);

const grossAC = mergeMaps(scriptA, grossC);
compareRows('Scenario AC', buildPayoutTable(grossAC), sectionAC.vendors, errors);
compareTotal('Scenario AC', grossAC, sectionAC.totalRow, errors);

const grossBC = mergeMaps(scriptB, grossC);
compareRows('Scenario BC', buildPayoutTable(grossBC), sectionBC.vendors, errors);
compareTotal('Scenario BC', grossBC, sectionBC.totalRow, errors);

if (errors.length > 0) {
  console.error('Validation consistency check failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log('Validation consistency check passed.');
console.log(`- Team C fixture purchases: ${teamCFixture.purchases.length}`);
console.log(`- Scenario AB total gross: ${formatEuro(sumMap(grossAB))} €`);
console.log(`- Scenario AC total gross: ${formatEuro(sumMap(grossAC))} €`);
console.log(`- Scenario BC total gross: ${formatEuro(sumMap(grossBC))} €`);
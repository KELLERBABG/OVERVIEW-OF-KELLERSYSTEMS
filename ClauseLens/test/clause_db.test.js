"use strict";

/**
 * Klauseldatenbank: Integritaet, Abdeckungsmatrix und Exportdeterminismus.
 *
 * Der Test prueft die Datenbank als Ganzes (Manifest, Vertrag, Ids, Duplikate,
 * Containment), zaehlt die Abdeckungsluecken, verifiziert die Detektorzuordnung
 * gegen die echte Regelmaschine und stellt sicher, dass die Exportdateien
 * reproduzierbar und aktuell sind.
 *
 * Die Schwellen unten sind GEMESSENE Basiswerte und dienen als Regressionszaun.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  DETECTOR_STATUSES,
  DETECTOR_TYPES,
  DOC_FAMILIES,
  RECORD_FIELDS,
  buildCoverage,
  buildExports,
  findNearDuplicates,
  loadDatabase,
  query,
  summarize,
  validateRecord,
  verifyMappings,
} from "../src/clausedb.js";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const THRESHOLDS = Object.freeze({
  minRecords: 1300,
  minRecordsPerFamily: 12,
  minCoveredDetectors: 30,
  // `mapped` ist eine Messung je Datensatz, keine Quote: benennt ein Datensatz eine Regel,
  // muss diese Regel auf seine Formulierung auch feuern. Der frueher hier abgesenkte Wert
  // (0.83) hat diesen Anspruch einer Sammelquote geopfert; die Zuordnung steht jetzt bei 1.
  minMappedVerifiedRate: 1,
  // Der gemessene Bestand des mittleren Zustands: eine Regel deckt den Klauseltyp, reagiert
  // aber nicht auf die Formulierung. Das ist eine benannte Schwaeche, keine Abdeckung; jeder
  // Fall steht namentlich in data/export/coverage.md. Der Wert darf nur sinken, nie steigen.
  // 97 -> 92: fuenf blind gefuehrte Datensaetze (wkg-dynamischer-url-verweis, agbb2b-069,
  // agbb2c-059, dach-008, dach-034) feuern seit der Holdout-Haertung nachweislich,
  // wurden deshalb auf `mapped` umgehaengt und sind hier nicht mehr mitgezaehlt.
  // 92 -> 91: ges-036 feuert seit Salvatorische-Klausel-Haertung nachweislich.
  // 91 -> 79: blind gefuehrte Datensaetze feuern nachweislich und wurden auf `mapped`
  // umgehaengt; zuletzt agbb2b-034 (Telemetriedaten) und spt-022 (Kündigungsfrist).
  maxMappedBlindRecords: 79,
  // Die Zahl der Luecken-Klauseln, die zufaellig eine fremde Regel ausloesen, ist nach der
  // Zuordnung der 13 falsch als Luecke gefuehrten Datensaetze wieder unter dem alten Zaun.
  // 25 -> 30: fuenf weitere Luecken-Faelle (u.a. miet-022 Verzichtsbindung, enb-021
  // hinausgezoegerte Abschlagszahlung, isb-034 Kündigungsausschluss) feuern einzeln geprueft
  // plausibel mit einer Fremdregel.
  // 30 -> 33: drei weitere Luecken-Faelle feuern nach der Red-Team-Haertung einzeln geprueft
  // plausibel mit einer Fremdregel: enlaw-023 (online abrufbare Zusatzbedingungen ->
  // dynamic_url_referral), enlaw-053 (Verguetungsanpassung ohne Kuendigungsrecht ->
  // price_escalation), vsr-022 (hinterlegte Bedingungen in jeweils gueltiger Fassung ->
  // dynamic_url_referral). Sie bleiben als Luecken gefuehrt, der Treffer ist echt.
  // Red-Team-Korpus (Vertraege/): die nachtraeglich ergaenzten Synonym- und
  // Sprachmuster treffen Luecken-Klauseln. Jede wurde einzeln geprueft und ist
  // eine echte missbraeuchliche Klausel ohne registrierten Detektor; die
  // Falsch-Positiv-Quote im Korpus ist 0 bei 3058 Fallen und 2811 Kontrollklauseln.
  // 46 -> 61: die geminten Muster aus service/immobilien/marketing/evasion treffen
  // 11 weitere Luecken-Klauseln (u.a. arb-045 Widerrufsvorbehalt fuer Zulagen,
  // enlaw-035 unbefristete Vertraulichkeit, miet2-029 Werbeanlagen mit widerruflicher
  // Zustimmung, isb-003 einseitiges Beendigungsrecht). Jede wurde einzeln geprueft,
  // jede ist eine echte missbraeuchliche Klausel, nur unter fremdem Etikett.
  // 61 -> 67: die neuen Widerrufs- und Anfechtungsarme sowie die Mindestlaufzeit-Erkennung
  // treffen sechs Luecken-Klauseln ohne registrierten Detektor. Jede wurde einzeln geprueft
  // und ist eine echte missbraeuchliche Klausel unter fremdem Etikett:
  // aufh-048 (Verzicht auf Widerruf und Anfechtung), lsd-018 (Verbraucherleasing ohne
  // Widerrufsrecht), lsd-030 und sweep309-012 (Widerrufsverzicht gegen Rabatt),
  // sweep-312-widerrufsverzicht-01 (Verzicht bei Bestellabschluss) sowie b2c2-013
  // (neue Mindestlaufzeit von 36 Monaten bei Upgrade ohne Ruecktrittsrecht).
  maxGapRecordsWithForeignHit: 67,
  slices: 25,
});

let cached = null;
function db() {
  if (!cached) cached = loadDatabase(ROOT);
  return cached;
}

test("klauseldb: das Manifest besitzt jeden Datensatz und der Vertrag ist eingehalten", () => {
  const database = db();
  assert.ok(database.manifest, "data/manifest.json muss lesbar sein");
  assert.deepEqual(database.errors, [], `Datenbankfehler:\n${database.errors.join("\n")}`);
  assert.equal(database.manifest.slices.length, THRESHOLDS.slices, `${THRESHOLDS.slices} registrierte Slices erwartet`);
  assert.ok(
    database.records.length >= THRESHOLDS.minRecords,
    `zu wenige Datensaetze: ${database.records.length} < ${THRESHOLDS.minRecords}`
  );

  const ids = new Set();
  for (const record of database.records) {
    assert.ok(!ids.has(record.id), `doppelte id ${record.id}`);
    ids.add(record.id);
    assert.ok(
      DETECTOR_STATUSES.includes(record.detectorStatus),
      `${record.id}: detectorStatus muss einen der drei Zuordnungszustaende tragen`
    );
    if (record.detectorStatus === "no_detector") {
      assert.equal(record.detector, null, `${record.id}: no_detector braucht detector null`);
    } else {
      assert.ok(record.detector, `${record.id}: ${record.detectorStatus} braucht einen Detektornamen`);
    }
    assert.ok(record.legalBasis.length > 0, `${record.id}: rechtliche Grundlage fehlt`);
  }
});

test("klauseldb: jede Dokumentfamilie ist belegt", () => {
  const database = db();
  const counts = summarize(database.records).perFamily;
  const missing = DOC_FAMILIES.filter((family) => (counts[family] || 0) < THRESHOLDS.minRecordsPerFamily);
  assert.deepEqual(missing, [], `Familien unter ${THRESHOLDS.minRecordsPerFamily} Datensaetzen: ${missing.join(", ")}`);
});

test("klauseldb: umformulierte Klauseltexte sind ein Buildfehler", () => {
  const base = {
    id: "unit-001",
    docFamily: "b2b_agb",
    clauseType: "Einseitiger Anpassungsvorbehalt",
    clauseText:
      "Der Anbieter behaelt sich vor, die monatliche Verguetung der Leistung nach billigem Ermessen anzupassen; der Kunde kann der Anpassung innerhalb von vier Wochen widersprechen und den Vertrag kuendigen.",
    why: "Der einseitige Anpassungsvorbehalt verschiebt das Preisrisiko vollstaendig auf den Kunden.",
    legalBasis: ["§ 307 Abs. 1 S. 1 BGB"],
    caseLaw: [],
    otherStatutes: [],
    jurisdiction: "DE",
    language: "de",
    severity: "hoch",
    detector: null,
    detectorStatus: "no_detector",
    evasion: "direct",
  };
  const paraphrase = {
    ...base,
    id: "unit-002",
    clauseText:
      "Der Anbieter behaelt sich vor, die monatliche Verguetung nach billigem Ermessen anzupassen; der Kunde kann der Anpassung innerhalb von vier Wochen widersprechen und den Vertrag kuendigen.",
  };
  const distinct = {
    ...base,
    id: "unit-003",
    clauseText:
      "Fuer jeden Werktag der Verzoegerung schuldet der Auftragnehmer eine Pauschale von 250 EUR; der Nachweis eines geringeren Schadens ist ausgeschlossen und die Gesamtsumme ist auf zehn Prozent begrenzt.",
  };

  const twins = findNearDuplicates([base, paraphrase]);
  assert.equal(twins.pairs.length, 1, "eine umformulierte Klausel muss als Zwilling erkannt werden");
  assert.equal(twins.pairs[0].kind, "jaccard");

  const different = findNearDuplicates([base, distinct]);
  assert.deepEqual(different.pairs, [], "verschiedene Klauseltypen duerfen nicht kollidieren");
  assert.deepEqual(different.phrases, []);

  const database = db();
  const nearDuplicateErrors = database.errors.filter(
    (error) => error.includes("nahezu identischer Klauseltext") || error.includes("identische Passage")
  );
  assert.deepEqual(
    nearDuplicateErrors,
    [],
    `die Datenbank enthaelt umformulierte Dubletten:\n${nearDuplicateErrors.slice(0, 10).join("\n")}`
  );
});

test("klauseldb: die Exporte halten den veroeffentlichten Datensatzvertrag ein", () => {
  const database = db();
  const jsonl = readFileSync(path.join(ROOT, "data", "export", "clauses.jsonl"), "utf8")
    .split("\n")
    .filter((line) => line.trim().length > 0);
  assert.equal(jsonl.length, database.records.length, "der JSONL-Export muss jeden Datensatz enthalten");

  for (const line of jsonl) {
    const record = JSON.parse(line);
    assert.deepEqual(Object.keys(record).sort(), [...RECORD_FIELDS].sort(), `${record.id}: Exportfelder weichen vom Schema ab`);
    assert.deepEqual(validateRecord(record), [], `${record.id}: der Export verletzt schema/clause.schema.json`);
  }

  const bundle = JSON.parse(readFileSync(path.join(ROOT, "data", "export", "clauses.json"), "utf8"));
  for (const record of bundle.records) {
    assert.deepEqual(Object.keys(record).sort(), [...RECORD_FIELDS].sort(), `${record.id}: JSON-Export traegt Zusatzfelder`);
  }
});

test("klauseldb: Schema, Validator und Engine stimmen ueberein", () => {
  const schema = JSON.parse(readFileSync(path.join(ROOT, "schema", "clause.schema.json"), "utf8"));
  assert.deepEqual(
    [...schema.required].sort(),
    [...RECORD_FIELDS].sort(),
    "schema/clause.schema.json required und RECORD_FIELDS in src/clausedb.js muessen identisch sein"
  );
  assert.equal(schema.additionalProperties, false, "Zusatzfelder muessen verboten bleiben");
  assert.ok(Array.isArray(schema.allOf) && schema.allOf.length === 3, "die drei Querfeldregeln fehlen");
  assert.deepEqual(
    [...schema.properties.detectorStatus.enum].sort(),
    [...DETECTOR_STATUSES].sort(),
    "das detectorStatus-Enum im Schema muss die drei Zuordnungszustaende abbilden"
  );
  assert.ok(
    /mapped_blind/.test(schema.properties.detectorStatus.description),
    "der mittlere Zustand muss im Schema erklaert sein"
  );

  assert.deepEqual(
    [...schema.properties.docFamily.enum].sort(),
    [...DOC_FAMILIES].sort(),
    "das docFamily-Enum im Schema muss den Dokumentfamilien in src/clausedb.js entsprechen"
  );

  const detectorEnum = schema.properties.detector.enum.filter((value) => value !== null);
  assert.deepEqual(
    [...detectorEnum].sort(),
    [...DETECTOR_TYPES].sort(),
    "das Detektor-Enum im Schema muss der Regelmaschine in src/analyze.js entsprechen"
  );
  assert.equal(detectorEnum.length, 47, "47 Detektoren erwartet");
});

test("klauseldb: Abdeckungsmatrix macht die Luecken zaehlbar", () => {
  const database = db();
  const summary = summarize(database.records);
  const coverage = buildCoverage(database.records);

  assert.equal(coverage.rows.length, summary.clauseTypes, "jede Matrixzeile ist genau ein Klauseltyp");
  const rowCount = coverage.rows.reduce((sum, row) => sum + row.count, 0);
  assert.equal(rowCount, database.records.length, "die Matrix darf keinen Datensatz verlieren");
  assert.ok(summary.gapClauseTypes > 0, "ohne Luecken misst die Matrix nichts");
  assert.ok(
    coverage.coveredDetectors >= THRESHOLDS.minCoveredDetectors,
    `nur ${coverage.coveredDetectors} Detektoren mit Klauseltypen belegt`
  );

  const countsByDetector = Object.entries(coverage.totalsByDetector).length;
  assert.equal(countsByDetector, coverage.coveredDetectors + 1, "Summe der Detektoren plus Lueckenzeile erwartet");

  console.log("");
  console.log(`Klauseldatenbank: ${summary.total} Datensaetze, ${summary.clauseTypes} Klauseltypen`);
  console.log("  Datensaetze je Dokumentfamilie");
  for (const family of DOC_FAMILIES) {
    console.log(`    ${family.padEnd(26)}${String(summary.perFamily[family] || 0).padStart(4)}`);
  }
  console.log("  Klauseltypen je Detektor (Abdeckungsmatrix)");
  for (const [detector, count] of Object.entries(coverage.totalsByDetector).sort((a, b) => b[1] - a[1])) {
    console.log(`    ${detector.padEnd(34)}${String(count).padStart(3)}`);
  }
  console.log(`  Abgedeckte Detektoren: ${coverage.coveredDetectors}/${DETECTOR_TYPES.length}`);
  console.log("");
});

test("klauseldb: die drei Zuordnungszustaende werden von der Engine gemessen", () => {
  const database = db();
  const verification = verifyMappings(database.records);

  assert.equal(verification.mapped + verification.blind + verification.gaps, database.records.length);
  assert.deepEqual(
    verification.notFiring,
    [],
    "mapped behauptet ein Feuern der Regel und muss es belegen: " +
      verification.notFiring.map((item) => `${item.id}: ${item.detector}`).join(", ")
  );
  assert.deepEqual(
    verification.blindFiring,
    [],
    "mapped_blind behauptet, dass die Regel nicht feuert: " +
      verification.blindFiring.map((item) => `${item.id}: ${item.detector}`).join(", ")
  );
  assert.equal(verification.mappedVerified, verification.mapped);
  assert.ok(
    verification.mappedVerifiedRate >= THRESHOLDS.minMappedVerifiedRate,
    `nur ${Math.round(verification.mappedVerifiedRate * 1000) / 10}% der gemessenen Zuordnungen feuern ` +
      `(< ${THRESHOLDS.minMappedVerifiedRate * 100}) — eine gemessene Zuordnung darf nicht wandern`
  );
  assert.equal(verification.blind, database.records.filter((r) => r.detectorStatus === "mapped_blind").length);
  assert.ok(
    verification.blind <= THRESHOLDS.maxMappedBlindRecords,
    `blinde Zuordnungen: ${verification.blind} > ${THRESHOLDS.maxMappedBlindRecords} — neue blinde Faelle ` +
      "einzeln pruefen, statt den Zaun zu heben"
  );
  assert.ok(
    verification.gapHits.length <= THRESHOLDS.maxGapRecordsWithForeignHit,
    `zu viele Luecken-Klauseln mit Fremdtreffer: ${verification.gapHits.length}`
  );

  console.log(
    `  Engine-Verifikation: ${verification.mapped} verifiziert · ${verification.blind} blind ` +
      `(${Math.round(verification.blindShare * 1000) / 10}% der Klauseln mit beanspruchter Regel) · ` +
      `${verification.gaps} ohne Regel (${verification.gapHits.length} mit Fremdtreffer)`
  );
  console.log("");
});

test("klauseldb: die drei Zuordnungszustaende sind in den Exporten sichtbar", () => {
  const database = db();
  const printed = readFileSync(path.join(ROOT, "data", "export", "clauses.jsonl"), "utf8")
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => JSON.parse(line));
  for (const status of DETECTOR_STATUSES) {
    assert.ok(
      printed.some((record) => record.detectorStatus === status),
      `${status} fehlt im JSONL-Export — der mittlere Zustand darf nicht verschwinden`
    );
  }

  const coverage = JSON.parse(readFileSync(path.join(ROOT, "data", "export", "coverage.json"), "utf8"));
  const verification = coverage.verification;
  assert.equal(verification.blindRecords.length, verification.blind, "jeder blinde Datensatz muss namentlich im Export stehen");
  assert.equal(verification.blindFiring.length, 0, "kein blinder Datensatz darf die Regel doch ausloesen");
  assert.equal(
    coverage.stats.perStatus.mapped + coverage.stats.perStatus.mapped_blind + coverage.stats.perStatus.no_detector,
    coverage.stats.total,
    "die drei Zustaende muessen jeden Datensatz abdecken"
  );
  assert.ok(coverage.blindClauseTypes > 0 && coverage.gapClauseTypes > 0, "beide Schwaechegrade muessen gezaehlt sein");

  const markdown = readFileSync(path.join(ROOT, "data", "export", "coverage.md"), "utf8");
  assert.match(markdown, /blind/, "der Report muss den mittleren Zustand benennen");
  for (const item of verification.blindRecords) {
    assert.ok(markdown.includes(item.id), `${item.id} fehlt im Abdeckungsreport`);
  }
});

test("klauseldb: die Exporte sind deterministisch und auf der Platte aktuell", () => {
  const database = db();
  const first = buildExports(database);
  const second = buildExports(db());
  assert.deepEqual(Object.keys(first.files).sort(), Object.keys(second.files).sort());
  for (const [relative, content] of Object.entries(first.files)) {
    assert.equal(content, second.files[relative], `${relative}: zwei Laeufe muessen identisch sein`);
    const onDisk = readFileSync(path.join(ROOT, relative), "utf8");
    assert.equal(
      onDisk.replace(/\r\n/g, "\n"),
      content.replace(/\r\n/g, "\n"),
      `${relative} ist veraltet — mit "node bin/clause-db.js export" neu erzeugen`
    );
  }
  assert.deepEqual(Object.keys(first.files).sort(), [
    "data/export/clauses.csv",
    "data/export/clauses.json",
    "data/export/clauses.jsonl",
    "data/export/coverage.json",
    "data/export/coverage.md",
  ]);
});

test("klauseldb: die Abfrageoberflaeche filtert korrekt", () => {
  const database = db();
  const gaps = query(database.records, { gaps: true });
  assert.ok(gaps.length > 0);
  assert.ok(gaps.every((record) => record.detectorStatus === "no_detector" && record.detector === null));

  const mapped = query(database.records, { mapped: true });
  assert.ok(mapped.every((record) => record.detectorStatus === "mapped"));

  const blind = query(database.records, { blind: true });
  assert.ok(blind.length > 0);
  assert.ok(blind.every((record) => record.detectorStatus === "mapped_blind" && typeof record.detector === "string"));
  assert.equal(mapped.length + blind.length + gaps.length, database.records.length);

  const consumer = query(database.records, { family: "b2c_agb" });
  assert.ok(consumer.length > 0);
  assert.ok(consumer.every((record) => record.docFamily === "b2c_agb"));

  const jurisdictionRecords = query(database.records, { detector: "jurisdiction" });
  assert.ok(jurisdictionRecords.length > 0);
  assert.ok(jurisdictionRecords.every((record) => record.detector === "jurisdiction"));

  const single = query(database.records, { id: database.records[0].id });
  assert.equal(single.length, 1);
});

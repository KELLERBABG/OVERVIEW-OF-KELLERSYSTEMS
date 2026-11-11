"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  analyzeContract,
  parseDurationsMonths,
} from "../src/analyze.js";
import { decodeBuffer, splitLines } from "../src/textio.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..");

const dirtyText = readFileSync(path.join(root, "test/fixtures/sample_agb.txt"), "utf-8");
const cleanText = readFileSync(path.join(root, "test/fixtures/clean_contract.txt"), "utf-8");

const dirty = analyzeContract(dirtyText);
const clean = analyzeContract(cleanText);

const ALL_TYPES = [
  "auto_renewal",
  "notice_period",
  "liability",
  "penalty",
  "jurisdiction",
  "non_compete",
  "data_grant",
  "default_interest",
  "unilateral_change",
];

function countLines(text) {
  const lines = splitLines(text);
  while (lines.length && lines[lines.length - 1].trim() === "") lines.pop();
  return lines.length;
}

function byType(result, type) {
  return result.findings.filter((f) => f.type === type);
}

test("dirty fixture triggers every detector type", () => {
  const found = new Set(dirty.findings.map((f) => f.type));
  for (const type of ALL_TYPES) {
    assert.ok(found.has(type), `fehlender Typ: ${type}`);
  }
});

test("findings have complete shape and valid severity", () => {
  for (const result of [dirty, clean]) {
    for (const f of result.findings) {
      assert.ok(typeof f.type === "string" && f.type.length > 0);
      assert.ok(typeof f.titlePlainGerman === "string" && f.titlePlainGerman.length > 3);
      assert.ok(["hoch", "mittel", "niedrig"].includes(f.severity));
      assert.ok(typeof f.excerpt === "string" && f.excerpt.trim().length > 10);
      assert.ok(Array.isArray(f.lineNumbers) && f.lineNumbers.length === 2);
      assert.ok(typeof f.why === "string" && f.why.length > 10);
      assert.ok(typeof f.suggestion === "string" && f.suggestion.length > 10);
    }
  }
});

test("severity mapping: key traps are rated 'hoch'", () => {
  assert.equal(byType(dirty, "auto_renewal")[0].severity, "hoch");
  assert.equal(byType(dirty, "default_interest")[0].severity, "hoch"); // 12 Prozentpunkte > 9
  assert.equal(byType(dirty, "jurisdiction")[0].severity, "hoch"); // ausschliesslich + Anbieter-Sitz
  assert.equal(byType(dirty, "penalty")[0].severity, "hoch");
  assert.equal(byType(dirty, "data_grant")[0].severity, "hoch");
  // gesetzliche Verzugszinsen ohne Prozentsatz sind unkritisch -> kein Befund im clean fixture
  assert.equal(byType(clean, "default_interest").length, 0);
});

test("clean fixture produces no 'hoch' findings", () => {
  assert.equal(clean.counts.hoch, 0);
  const nc = byType(clean, "non_compete");
  assert.equal(nc.length, 1);
  assert.equal(nc[0].severity, "mittel"); // reine Erwaehnung ohne nachvertragliche Wirkung
});

test("line numbers are sane and point at the right clauses", () => {
  for (const [result, text] of [
    [dirty, dirtyText],
    [clean, cleanText],
  ]) {
    const total = countLines(text);
    for (const f of result.findings) {
      const [from, to] = f.lineNumbers;
      assert.ok(Number.isInteger(from) && Number.isInteger(to));
      assert.ok(from >= 1, `Startzeile >= 1 (${f.type})`);
      assert.ok(from <= to, `Start <= Ende (${f.type})`);
      assert.ok(to <= total, `Endezeile <= ${total} (${f.type}: ${to})`);
    }
  }

  const lines = splitLines(dirtyText);
  const lineOf = (needle) =>
    lines.findIndex((l) => l.toLowerCase().includes(needle)) + 1;

  const penaltyLine = lineOf("vertragsstrafe");
  const penaltyFinding = byType(dirty, "penalty")[0];
  assert.ok(
    penaltyFinding.lineNumbers[0] <= penaltyLine &&
      penaltyLine <= penaltyFinding.lineNumbers[1],
    `Vertragsstrafe steht in Zeile ${penaltyLine}, Befund: ${penaltyFinding.lineNumbers}`
  );

  const renewalLine = lineOf("verlängert sich");
  const renewalFinding = byType(dirty, "auto_renewal")[0];
  assert.ok(
    renewalFinding.lineNumbers[0] <= renewalLine &&
      renewalLine <= renewalFinding.lineNumbers[1]
  );

  const liabilityFinding = byType(dirty, "liability")[0];
  assert.ok(liabilityFinding.lineNumbers[1] >= lineOf("uneingeschränkt frei"));
});

test("deduplication: same-type hits in one paragraph merge into one finding", () => {
  const text =
    "Zusatzvereinbarung.\n" +
    "Die Haftung des Anbieters ist ausgeschlossen; übernimmt der Anbieter keine Haftung für Folgeschäden, haftet er zudem nicht für einfache Fahrlässigkeit.";
  const result = analyzeContract(text);
  const liability = byType(result, "liability");
  assert.equal(liability.length, 1, "drei Muster, ein Absatz -> genau ein Befund");
  assert.equal(liability[0].severity, "hoch");
});

test("excerpt quotes the matched clause", () => {
  const renewal = byType(dirty, "auto_renewal")[0];
  assert.match(renewal.excerpt, /verlängert sich/i);
  const interest = byType(dirty, "default_interest")[0];
  assert.match(interest.excerpt, /Verzugszinsen/i);
});

test("riskScore ordering and bounds", () => {
  assert.ok(dirty.riskScore > clean.riskScore, "dirty > clean");
  assert.ok(dirty.riskScore > 70);
  assert.ok(clean.riskScore < 35);
  for (const score of [dirty.riskScore, clean.riskScore]) {
    assert.ok(score >= 0 && score <= 100);
  }
});

test("summary is plain German and reflects counts", () => {
  assert.match(dirty.summary, /Befunde:/);
  assert.match(dirty.summary, /Gesamtrisiko:/);
  assert.match(clean.summary, /Keine Rechtsberatung|Empfehlung|Gesamtrisiko:/);
});

test("empty input yields zero findings and score 0", () => {
  const result = analyzeContract("");
  assert.deepEqual(result.findings, []);
  assert.equal(result.riskScore, 0);
  assert.match(result.summary, /Keine/);
});

test("duration parser understands digits and German number words", () => {
  assert.ok(Math.abs(parseDurationsMonths("mit einer Frist von sechs Monaten")[0] - 6) < 0.01);
  assert.ok(Math.abs(parseDurationsMonths("Laufzeit von zwölf Monaten")[0] - 12) < 0.01);
  const weeks = parseDurationsMonths("Kündigung mit einer Frist von vier Wochen");
  assert.equal(weeks.length, 1);
  assert.ok(weeks[0] < 1.5, "vier Wochen sind weniger als 1,5 Monate");
  assert.deepEqual(parseDurationsMonths("keine Fristen hier"), []);
});

test("textio tolerates Latin-1 umlauts and strips UTF-8 BOM", () => {
  assert.equal(decodeBuffer(Buffer.from([0x4b, 0xf6, 0x6c, 0x6e])), "K\u00f6ln");
  const bom = Buffer.concat([
    Buffer.from([0xef, 0xbb, 0xbf]),
    Buffer.from("\u00c4GB \u2013 ok", "utf-8"),
  ]);
  assert.equal(decodeBuffer(bom), "\u00c4GB \u2013 ok");
});

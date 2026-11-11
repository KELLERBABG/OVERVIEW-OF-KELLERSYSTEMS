import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { runChecks } from "../src/engine.js";
import { generateMarkdownReport, DISCLAIMER } from "../src/report.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

test("Bericht enthaelt Urteil, Paragraphen-Zitate und Disclaimer (Bad-Spec)", () => {
  const run = runChecks(fixture("bad_spec.json"));
  const md = generateMarkdownReport(run, { projekt: fixture("bad_spec.json").projekt });

  assert.ok(md.includes("# BauLens – Vorprüfungsbericht"));
  assert.ok(md.includes("**nicht bestanden**"));
  assert.ok(md.includes("§2 MBO"));
  assert.ok(md.includes("§6 Abs. 2 MBO"));
  assert.ok(md.includes("§19 BauNVO"));
  assert.ok(md.includes("§49 MBO"));
  assert.ok(md.includes("§46 MBO"));
  assert.ok(md.includes("§11 MBO"));
  assert.ok(md.includes("§16 BauNVO"));
  assert.ok(md.includes(DISCLAIMER));
  assert.ok(md.includes("| Regel | Paragraph | Status | Anzahl Befunde |"));
  assert.ok(md.includes("FEHLER"));
});

test("Bericht des Good-Specs zeigt bestanden und keine Fehler-Sektion", () => {
  const spec = fixture("good_spec.json");
  const md = generateMarkdownReport(runChecks(spec), { projekt: spec.projekt });
  assert.ok(md.includes("**bestanden**"));
  assert.ok(!md.includes("**nicht bestanden**"));
  assert.ok(md.includes("Musterweg 12"));
});

test("Disclaimer steht prominent am Anfang und am Ende", () => {
  const md = generateMarkdownReport(runChecks(fixture("good_spec.json")));
  const first = md.split("\n")[2];
  assert.ok(first.startsWith("> **Haftungsausschluss:**"));
  const lastBlock = md.slice(-600);
  assert.ok(lastBlock.includes(DISCLAIMER));
});

test("Fehlerliste im Bericht nennt Regel-IDs und konkrete Werte", () => {
  const md = generateMarkdownReport(runChecks(fixture("bad_spec.json")));
  assert.ok(md.includes("[abstandsflaeche]"));
  assert.ok(md.includes("[rettungswege]"));
  assert.ok(md.includes("erforderliche Abstandsfläche"));
});

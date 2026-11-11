"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { generateSarif, writeSarif } from "../src/sarif.js";
import { analyzeContract } from "../src/analyze.js";
import { mkdtempSync, readFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

test("sarif: generates valid OASIS SARIF 2.1.0 JSON format", () => {
  const text = `
    Der Vertrag verlängert sich automatisch um weitere 24 Monate.
    Die Haftung des Anbieters ist für sämtliche Schäden ausgeschlossen.
  `;
  const result = analyzeContract(text);
  const sarifStr = generateSarif(result, { sourceName: "agb_sample.txt" });

  assert.ok(typeof sarifStr === "string");
  const parsed = JSON.parse(sarifStr);

  assert.equal(parsed.version, "2.1.0");
  assert.equal(parsed.$schema, "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json");
  assert.ok(Array.isArray(parsed.runs));
  assert.equal(parsed.runs.length, 1);

  const run = parsed.runs[0];
  assert.equal(run.tool.driver.name, "ClauseLens");
  assert.equal(run.tool.driver.semanticVersion, "1.0.0");
  assert.ok(Array.isArray(run.results));
  assert.equal(run.results.length, result.findings.length);

  // Check result structure
  const first = run.results[0];
  assert.ok(first.ruleId);
  assert.ok(["error", "warning", "note"].includes(first.level));
  assert.ok(first.message && first.message.text);
  assert.ok(Array.isArray(first.locations));
  assert.equal(first.locations[0].physicalLocation.artifactLocation.uri, "agb_sample.txt");
});

test("sarif: writeSarif writes file to disk", () => {
  const tmp = mkdtempSync(path.join(tmpdir(), "cl-sarif-"));
  try {
    const outPath = path.join(tmp, "audit.sarif");
    const result = analyzeContract("Die Haftung ist ausgeschlossen.");
    const content = generateSarif(result);
    writeSarif(content, outPath);

    assert.ok(existsSync(outPath));
    const diskContent = readFileSync(outPath, "utf-8");
    assert.equal(diskContent, content);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

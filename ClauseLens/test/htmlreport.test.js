"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { generateHtmlReport, writeHtmlReport } from "../src/htmlreport.js";
import { analyzeContract } from "../src/analyze.js";
import { mkdtempSync, readFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

test("htmlreport: generates valid self-contained HTML with SVG gauge and filters", () => {
  const text = `
    Der Vertrag verlängert sich automatisch um weitere 24 Monate.
    Die Haftung des Anbieters ist ausgeschlossen für sämtliche Schäden.
  `;
  const result = analyzeContract(text);
  const html = generateHtmlReport(result, { sourceName: "vertrag_test.pdf" });

  assert.ok(html.includes("<!doctype html>"));
  assert.ok(html.includes("vertrag_test.pdf"));
  assert.ok(html.includes("<svg viewBox=\"0 0 140 140\""));
  assert.ok(html.includes(`>${result.riskScore}<`));
  assert.ok(html.includes("Gegenvorschlag zur Nachverhandlung"));
  assert.ok(html.includes("@media print"));
  assert.ok(html.includes("function copyRedline"));
  assert.ok(html.includes("data-severity=\"hoch\""));
});

test("htmlreport: writeHtmlReport writes file to disk", () => {
  const tmp = mkdtempSync(path.join(tmpdir(), "cl-html-"));
  try {
    const outPath = path.join(tmp, "sub", "report.html");
    const result = analyzeContract("Die Haftung ist ausgeschlossen.");
    writeHtmlReport(generateHtmlReport(result), outPath);

    assert.ok(existsSync(outPath));
    const content = readFileSync(outPath, "utf-8");
    assert.match(content, /ClauseLens Risikobericht/);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

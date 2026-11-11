"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { analyzeBatch, renderBatchMarkdown, generateBatchHtml, collectFiles } from "../src/batch.js";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..");

test("batch: collects fixtures and produces portfolio summary", () => {
  const fixturesDir = path.join(root, "test", "fixtures");
  const files = collectFiles(fixturesDir);
  assert.ok(files.length >= 2, "Must find sample txt fixtures");

  const batch = analyzeBatch(files);
  assert.equal(batch.schemaVersion, "1.0.0");
  assert.ok(batch.portfolioStats.totalDocuments >= 2);
  assert.ok(typeof batch.portfolioStats.averageScore === "number");
  assert.ok(batch.topRisksAcrossPortfolio.length > 0);

  const md = renderBatchMarkdown(batch);
  assert.ok(md.includes("# ClauseLens Portfolio-Risikobericht"));
  assert.ok(md.includes("Häufigste Risikoklauseln"));

  const html = generateBatchHtml(batch);
  assert.ok(html.includes("<!doctype html>"));
  assert.ok(html.includes("Portfolio Risikobericht"));
  assert.ok(html.includes("Vertragsübersicht"));
});

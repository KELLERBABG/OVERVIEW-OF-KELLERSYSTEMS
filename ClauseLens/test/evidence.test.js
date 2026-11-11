"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract } from "../src/analyze.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..");

const GOLDEN_CORPUS = [
  {
    name: "sample_agb.txt (Known Adversarial Trap Contract)",
    file: "test/fixtures/sample_agb.txt",
    expectedMinScore: 70,
    expectedRiskLevel: "hoch",
    expectedTrapTypes: [
      "auto_renewal",
      "notice_period",
      "liability",
      "penalty",
      "jurisdiction",
      "non_compete",
      "data_grant",
      "default_interest",
      "unilateral_change",
    ],
  },
  {
    name: "clean_contract.txt (Standard Compliant Baseline)",
    file: "test/fixtures/clean_contract.txt",
    expectedMaxScore: 30,
    expectedRiskLevel: "niedrig",
    expectedNoHoch: true,
  },
];

test("evidence QA harness: golden contract corpus satisfies scoring baselines", () => {
  for (const item of GOLDEN_CORPUS) {
    const filePath = path.join(root, item.file);
    assert.ok(existsSync(filePath), `Golden fixture ${item.file} must exist`);
    const content = readFileSync(filePath, "utf-8");
    const result = analyzeContract(content);

    if (item.expectedMinScore !== undefined) {
      assert.ok(
        result.riskScore >= item.expectedMinScore,
        `${item.name} expected score >= ${item.expectedMinScore}, got ${result.riskScore}`
      );
    }
    if (item.expectedMaxScore !== undefined) {
      assert.ok(
        result.riskScore <= item.expectedMaxScore,
        `${item.name} expected score <= ${item.expectedMaxScore}, got ${result.riskScore}`
      );
    }
    if (item.expectedRiskLevel) {
      assert.equal(result.riskLevel, item.expectedRiskLevel);
    }
    if (item.expectedTrapTypes) {
      const detected = new Set(result.findings.map((f) => f.type));
      for (const trap of item.expectedTrapTypes) {
        assert.ok(detected.has(trap), `Expected trap ${trap} missing in ${item.name}`);
      }
    }
    if (item.expectedNoHoch) {
      assert.equal(result.counts.hoch, 0, `Compliant contract must have 0 hoch findings`);
    }
  }
});

test("evidence QA harness: false-positive resistance against compliant statutory clauses", () => {
  // Phrases that explicitly reference statutory provisions should NOT be flagged as high risk
  const compliantClauses = [
    "Für Verzugszinsen gelten die gesetzlichen Bestimmungen des § 288 BGB.",
    "Die Parteien vereinbaren eine Kündigungsfrist von einem Monat zum Monatsende.",
    "Für die Haftung gelten ausschließlich die gesetzlichen Vorschriften.",
    "Es gilt der gesetzliche Gerichtsstand.",
  ];

  for (const clause of compliantClauses) {
    const res = analyzeContract(clause);
    assert.equal(
      res.counts.hoch,
      0,
      `Statutory compliant clause should not trigger hoch finding: "${clause}"`
    );
  }
});

test("evidence QA harness: finding data model strictly adheres to schema contracts", () => {
  const dirtyPath = path.join(root, "test/fixtures/sample_agb.txt");
  const content = readFileSync(dirtyPath, "utf-8");
  const result = analyzeContract(content);

  assert.equal(result.schemaVersion, "1.0.0");
  for (const f of result.findings) {
    assert.ok(f.id.startsWith("cl-"), "Finding ID must follow cl-<type>-<start>-<end> pattern");
    assert.ok(typeof f.confidence === "number" && f.confidence > 0 && f.confidence <= 1.0);
    assert.ok(typeof f.citation === "string" && f.citation.length > 0, `Finding ${f.id} must have a statutory citation`);
    assert.ok(Array.isArray(f.tags) && f.tags.length > 0, "Finding must have tags");
    assert.ok(Array.isArray(f.lineNumbers) && f.lineNumbers.length === 2);
    assert.ok(f.lineNumbers[0] <= f.lineNumbers[1]);
  }
});

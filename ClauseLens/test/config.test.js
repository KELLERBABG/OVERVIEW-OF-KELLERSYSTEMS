"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { loadConfig, mergeConfig, POLICY_PROFILES } from "../src/config.js";
import { analyzeContract } from "../src/analyze.js";

test("config: loads default policy correctly", () => {
  const cfg = mergeConfig({}, "default");
  assert.equal(cfg.profile, "default");
  assert.equal(cfg.weights.hoch, 15);
  assert.equal(cfg.thresholds.hoch, 70);
});

test("config: merges b2b_saas profile with custom thresholds", () => {
  const cfg = mergeConfig({ thresholds: { hoch: 60 } }, "b2b_saas");
  assert.equal(cfg.profile, "b2b_saas");
  assert.equal(cfg.weights.hoch, 18);
  assert.equal(cfg.thresholds.hoch, 60);
});

test("config: custom rule execution in analyzeContract", () => {
  const text = "Der Kunde verpflichtet sich, Dienstleistungen ausschließlich bei uns beziehen zu dürfen.";
  const userConfig = {
    customRules: [
      {
        type: "exclusive_source",
        title: "Ausschließlichkeitsbindung",
        patterns: [/ausschließlich bei uns beziehen/i],
        severity: "hoch",
        why: "Verhindert Anbieterwechsel.",
        suggestion: "Streichen.",
        tags: ["custom", "exklusiv"],
      },
    ],
  };

  const result = analyzeContract(text, { userConfig });
  const found = result.findings.find((f) => f.type === "exclusive_source");
  assert.ok(found, "Custom rule must be triggered");
  assert.equal(found.severity, "hoch");
  assert.equal(found.confidence, 0.9);
  assert.ok(found.tags.includes("custom"));
});

test("config: activeDetectors filter limits executed rules", () => {
  const text = `
    Der Vertrag verlängert sich automatisch um weitere 24 Monate.
    Die Haftung ist für sämtliche Schäden ausgeschlossen.
  `;
  const userConfig = {
    activeDetectors: ["liability"], // only liability active
  };

  const result = analyzeContract(text, { userConfig });
  assert.equal(result.findings.length, 1);
  assert.equal(result.findings[0].type, "liability");
});

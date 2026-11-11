"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { compareContracts, renderCompareMarkdown, generateCompareHtml } from "../src/compare.js";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

test("compare: detects resolved liability and newly introduced penalty", () => {
  const tmp = mkdtempSync(path.join(tmpdir(), "cl-compare-"));
  try {
    const v1Path = path.join(tmp, "v1.txt");
    const v2Path = path.join(tmp, "v2.txt");

    // V1 has liability exclusion
    writeFileSync(
      v1Path,
      "§ 1 Haftung\nDie Haftung des Anbieters ist für sämtliche Schäden ausgeschlossen."
    );

    // V2 removed liability exclusion, but added a contractual penalty
    writeFileSync(
      v2Path,
      "§ 1 Haftung\nDie Haftung richtet sich nach den gesetzlichen Vorschriften.\n\n§ 2 Strafe\nEine Vertragsstrafe in Höhe von 5000 Euro wird fällig bei Verzug."
    );

    const diff = compareContracts(v1Path, v2Path);
    assert.equal(diff.schemaVersion, "1.0.0");
    assert.equal(diff.changes.resolved.length, 1);
    assert.equal(diff.changes.resolved[0].type, "liability");
    assert.equal(diff.changes.introduced.length, 1);
    assert.equal(diff.changes.introduced[0].type, "penalty");

    const md = renderCompareMarkdown(diff);
    assert.ok(md.includes("Versionsvergleich & Risikodelta"));
    assert.ok(md.includes("Behobene / entschärfte Risiken"));
    assert.ok(md.includes("Neu hinzugekommene Risiken"));

    const html = generateCompareHtml(diff);
    assert.ok(html.includes("<!doctype html>"));
    assert.ok(html.includes("Erfolgreich behobene Risiken"));
    assert.ok(html.includes("Neu eingeführte Risiken"));
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

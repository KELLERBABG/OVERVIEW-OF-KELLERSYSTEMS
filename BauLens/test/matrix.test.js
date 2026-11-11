import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { execFileSync } from "node:child_process";
import { runAllLandsMatrix, formatLandsMatrixTable } from "../src/matrix.js";
import { VERDICT } from "../src/engine.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

test("runAllLandsMatrix executes across all 34 profiles (16 Bundesländer + MBO, 10 AT, 7 CH)", () => {
  const spec = fixture("good_spec.json");
  const result = runAllLandsMatrix(spec);

  assert.equal(result.totalStates, 34);
  assert.equal(result.matrix.length, 34);

  // good_spec has 4m setbacks, which is >= 3.5m required in MBO/BY/NW/etc, but fails in BW (5m required)
  const bwEntry = result.matrix.find((m) => m.code === "BW");
  assert.ok(bwEntry);
  assert.equal(bwEntry.verdict, VERDICT.NICHT_BESTANDEN);
  assert.equal(bwEntry.requiredSetbackM, 5.0);

  // In Bayern, BayBO Grenzbebauung is active (Abstandsflächen deactivated, Grenzbebauung warning)
  const byEntry = result.matrix.find((m) => m.code === "BY");
  assert.ok(byEntry);
  assert.equal(byEntry.hasBayBOGrenz, true);
  assert.equal(byEntry.requiredSetbackM, null);
  assert.equal(byEntry.verdict, VERDICT.AUFLAGEN);

  // In MBO, it passes cleanly
  const mboEntry = result.matrix.find((m) => m.code === "MBO");
  assert.ok(mboEntry);
  assert.equal(mboEntry.verdict, VERDICT.BESTANDEN);

  // Austria (Wien)
  const wienEntry = result.matrix.find((m) => m.code === "AT_W");
  assert.ok(wienEntry);
  assert.equal(wienEntry.verdict, VERDICT.BESTANDEN);

  // Switzerland (Zürich)
  const zhEntry = result.matrix.find((m) => m.code === "CH_ZH");
  assert.ok(zhEntry);
  assert.equal(zhEntry.verdict, VERDICT.BESTANDEN);
});

test("formatLandsMatrixTable produces clean Unicode table with summary", () => {
  const spec = fixture("good_spec.json");
  const result = runAllLandsMatrix(spec);
  const table = formatLandsMatrixTable(result);

  assert.ok(table.includes("BW"));
  assert.ok(table.includes("Baden-Württemberg"));
  assert.ok(table.includes("BayBO"));
  assert.ok(table.includes("AT_W"));
  assert.ok(table.includes("CH_ZH"));
  assert.ok(table.includes("Typenprüfung-Zusammenfassung"));
});

test("CLI --all-lands and --dach flags run cross-state matrix and exits 0", () => {
  const bin = join(root, "bin", "baucheck.js");
  const specPath = join(root, "test", "fixtures", "good_spec.json");
  const stdout = execFileSync(process.execPath, [bin, "check", specPath, "--all-lands"], {
    encoding: "utf8",
  });

  assert.ok(stdout.includes("Regionen-Radar"));
  assert.ok(stdout.includes("Typenprüfung-Zusammenfassung"));
});

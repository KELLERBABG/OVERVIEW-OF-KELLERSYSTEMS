import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract } from "../src/analyze.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

test("adversarial enterprise development contract: detects all 8 disguised traps", () => {
  const contractPath = path.join(__dirname, "fixtures", "user_adversarial_dev.txt");
  const text = fs.readFileSync(contractPath, "utf-8");

  const report = analyzeContract(text);

  assert.equal(report.riskLevel, "hoch");
  assert.equal(report.riskScore, 100);
  assert.ok(report.findings.length >= 8, `Expected at least 8 findings, got ${report.findings.length}`);

  const types = report.findings.map((f) => f.type);

  // 1. § 4 Abs. 2: Fingierte Abnahme & Rügefrist (§ 640 Abs. 2 S. 2 BGB)
  assert.ok(
    types.includes("deemed_acceptance"),
    "Should detect disguised deemed acceptance in § 4 Abs. 2"
  );
  const acceptanceFinding = report.findings.find((f) => f.type === "deemed_acceptance");
  assert.equal(acceptanceFinding.severity, "hoch");

  // 2. § 7 Abs. 2: Verdeckte Vertragsstrafe & Nachweis-Knebel (§ 309 Nr. 5 lit. b BGB)
  assert.ok(
    types.includes("penalty"),
    "Should detect disguised liquidated damages & proof restriction in § 7 Abs. 2"
  );
  const penaltyFinding = report.findings.find((f) => f.type === "penalty");
  assert.equal(penaltyFinding.severity, "hoch");

  // 3. § 9 Abs. 2 & Abs. 3: Aushöhlung Kardinalpflichten & Datenverlust-Falle (§ 307 BGB)
  const liabilityFindings = report.findings.filter((f) => f.type === "liability");
  assert.ok(
    liabilityFindings.length >= 2,
    `Expected at least 2 liability findings (§ 9 Abs. 2 and Abs. 3), got ${liabilityFindings.length}`
  );
  assert.ok(liabilityFindings.every((f) => f.severity === "hoch"));

  // 4. § 12 Abs. 1: Konzern-/Kontokorrentvorbehalt an Urheber- und Nutzungsrechten (§ 307 BGB)
  assert.ok(
    types.includes("data_grant"),
    "Should detect Kontokorrentvorbehalt on rights in § 12 Abs. 1"
  );
  const dataFinding = report.findings.find((f) => f.type === "data_grant");
  assert.equal(dataFinding.severity, "hoch");

  // 5. § 12 Abs. 2: Vertraglicher Kill-Switch & Zurückbehaltungsrecht-Aushöhlung (§ 320, § 307 BGB)
  assert.ok(
    types.includes("setoff_restriction"),
    "Should detect software license kill switch and retention right bypass in § 12 Abs. 2"
  );
  const setoffFinding = report.findings.find((f) => f.type === "setoff_restriction");
  assert.equal(setoffFinding.severity, "hoch");

  // 6. § 15 Abs. 1: Verjährung (§ 309 Nr. 7 BGB)
  assert.ok(
    types.includes("statute_of_limitations"),
    "Should detect short statute of limitations in § 15 Abs. 1"
  );

  // 7. § 15 Abs. 2: Negative Beweislast bei Mängeln (§ 309 Nr. 12 BGB, § 307 BGB)
  assert.ok(
    types.includes("burden_of_proof"),
    "Should detect negative causation proof burden in § 15 Abs. 2"
  );
  const proofFinding = report.findings.find((f) => f.type === "burden_of_proof");
  assert.equal(proofFinding.severity, "hoch");

  // 8. § 18 Abs. 1 & Abs. 2: Billiges Ermessen & Kostenfreigabefiktion (§ 308 Nr. 4, 5 BGB)
  const changeFindings = report.findings.filter((f) => f.type === "unilateral_change");
  assert.ok(
    changeFindings.length >= 1,
    "Should detect unilateral technical discretion and cost approval fiction in § 18"
  );
  assert.ok(changeFindings.some((f) => f.severity === "hoch"));
});

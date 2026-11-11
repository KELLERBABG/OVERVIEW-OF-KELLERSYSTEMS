import test from "node:test";
import assert from "node:assert/strict";
import { analyzeContract } from "../src/analyze.js";
import { normalizeText } from "../src/textio.js";

test("hardening: detects vicarious agents liability exclusion (§ 278, § 307 BGB)", () => {
  const clause = "Der Anbieter haftet für das Verschulden von Erfüllungsgehilfen und Subunternehmern nur bei vorsätzlicher oder grob fahrlässiger Verletzung der Sorgfalt bei deren Auswahl (beschränkt auf Auswahlverschulden).";
  const res = analyzeContract(clause);
  const hit = res.findings.find((f) => f.type === "vicarious_agents");
  assert.ok(hit, "Must detect vicarious agents liability exclusion");
  assert.equal(hit.severity, "hoch");
  assert.match(hit.why, /278 BGB/);
});

test("hardening: detects unreasonable audit & inspection rights (§ 307 BGB)", () => {
  const clause = "Der Anbieter ist berechtigt, die Geschäftsräume und IT-Systeme des Kunden jederzeit ohne Vorankündigung zu betreten und zu auditieren; sämtliche Auditkosten trägt in jedem Fall der Kunde.";
  const res = analyzeContract(clause);
  const hit = res.findings.find((f) => f.type === "unreasonable_audit");
  assert.ok(hit, "Must detect unreasonable audit right");
  assert.equal(hit.severity, "hoch");
  assert.match(hit.why, /Vorankündigung/);
});

test("hardening: detects data lock-in and paid data export (§ 307 BGB, Art. 20 DSGVO)", () => {
  const clause = "Nach Vertragsbeendigung ist die Herausgabe der Daten des Kunden kostenpflichtig und wird gegen eine gesonderte Aufwandspauschale von 5.000 EUR abgerechnet; ein Anspruch auf Bereitstellung in einem gängigen, maschinenlesbaren Standardformat besteht nicht.";
  const res = analyzeContract(clause);
  const hit = res.findings.find((f) => f.type === "data_lockin");
  assert.ok(hit, "Must detect data lock-in and paid return barriers");
  assert.equal(hit.severity, "hoch");
  assert.match(hit.why, /DSGVO/);
});

test("hardening: detects third-party indemnity without fault (§ 307 BGB)", () => {
  const clause = "Der Kunde stellt den Anbieter im Innenverhältnis von sämtlichen Ansprüchen Dritter frei, die im Zusammenhang mit der Nutzung geltend gemacht werden, unabhängig von einem Verschulden des Kunden.";
  const res = analyzeContract(clause);
  const hit = res.findings.find((f) => f.type === "third_party_indemnity");
  assert.ok(hit, "Must detect no-fault third-party indemnity trap");
  assert.equal(hit.severity, "hoch");
  assert.match(hit.why, /verschuldensunabhängig/);
});

test("hardening: detects exclusion of extraordinary termination (§ 314 BGB)", () => {
  const clause = "Das gesetzliche Recht zur außerordentlichen Kündigung aus wichtigem Grund wird hiermit vollumfänglich ausgeschlossen und abbedungen.";
  const res = analyzeContract(clause);
  const hit = res.findings.find((f) => f.type === "asymmetric_termination");
  assert.ok(hit, "Must detect waiver or exclusion of extraordinary termination");
  assert.equal(hit.severity, "hoch");
  assert.match(hit.why, /314/);
});

test("hardening: neutralizes soft-hyphens (\\u00ad) inserted to evade regex", () => {
  // Adversary inserts soft-hyphens into key words:
  const textWithSoftHyphens = "Die Haf\u00adtung für jeg\u00adliche Schäden ist voll\u00adumfänglich aus\u00adgeschlossen.";
  const res = analyzeContract(textWithSoftHyphens);
  const hit = res.findings.find((f) => f.type === "liability");
  assert.ok(hit, "Must neutralize soft-hyphens and detect liability trap");
  assert.equal(hit.severity, "hoch");
});

test("hardening: neutralizes zero-width and non-breaking spaces", () => {
  // Adversary inserts zero-width space \u200B and non-breaking space \u00A0:
  const textWithZeroWidth = "Ver\u200Btrags\u200Bstrafe in Höhe von\u00A010.000\u00A0EUR.";
  const res = analyzeContract(textWithZeroWidth);
  const hit = res.findings.find((f) => f.type === "penalty");
  assert.ok(hit, "Must neutralize zero-width characters and detect penalty");
  assert.equal(hit.severity, "hoch");
});

test("hardening: DACH regional variations (Austrian ABGB / Swiss OR legal terms)", () => {
  const dachContract = `
  1. Gerichtsstand und Recht
  Ausschließlicher Gerichtsstand ist Zürich (Schweiz).
  
  2. Pönale bei Terminverzug
  Bei Verzug verfällt eine Pönale in Höhe von 5.000 EUR.
  
  3. Konventionalstrafe
  Die Konventionalstrafe beträgt 10 % der Nettovergütung.
  
  4. Gewährsfrist
  Sämtliche Gewährleistungsansprüche verjähren innerhalb von 3 Monaten ab Ablieferung.
  
  5. Haftung
  Die Haftung für sämtliche Schäden und Mangelfolgeschäden wird vollumfänglich abbedungen.
  `;
  const res = analyzeContract(dachContract);
  const types = res.findings.map((f) => f.type);
  assert.ok(types.includes("jurisdiction"), "Must detect Swiss exclusive jurisdiction");
  assert.ok(types.includes("penalty"), "Must detect Pönale / Konventionalstrafe");
  assert.ok(types.includes("statute_of_limitations"), "Must detect 3 months limitation");
  assert.ok(types.includes("liability"), "Must detect abbedungene Haftung");
  assert.equal(res.riskLevel, "hoch");
});

test("hardening: clean, balanced commercial contract yields ZERO high findings", () => {
  const cleanContract = `
  IT-Rahmendienstleistungsvertrag
  § 1 Vertragsgegenstand
  Der Anbieter erbringt Entwicklungs- und Supportleistungen für den Kunden gemäß Einzelauftrag.
  
  § 2 Vergütung
  Die Vergütung richtet sich nach den im Einzelauftrag vereinbarten Tagessätzen und ist 14 Tage nach Rechnungseingang fällig.
  
  § 3 Haftung
  Der Anbieter haftet unbeschränkt für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit sowie für vorsätzlich oder grob fahrlässig verursachte Schäden. Bei einfacher Fahrlässigkeit haftet der Anbieter nur bei Verletzung wesentlicher Vertragspflichten (Kardinalpflichten), begrenzt auf den vertragstypischen, vorhersehbaren Schaden.
  
  § 4 Kündigung
  Jede Partei kann diesen Vertrag mit einer Frist von einem Monat zum Monatsende in Textform (z. B. per E-Mail) kündigen. Das Recht beider Parteien zur außerordentlichen Kündigung aus wichtigem Grund bleibt unberührt.
  
  § 5 Mängelansprüche
  Der Kunde zeigt Mängel innerhalb einer angemessenen Frist nach Entdeckung an. Die Verjährung von Mängelansprüchen beträgt 24 Monate ab Übergabe.
  
  § 6 Daten und Vertragsende
  Nach Beendigung des Vertrages stellt der Anbieter dem Kunden alle Daten kostenfrei in einem offenen Standardformat (JSON/CSV) zur Verfügung.
  `;
  const res = analyzeContract(cleanContract);
  const highFindings = res.findings.filter((f) => f.severity === "hoch");
  assert.equal(highFindings.length, 0, `Compliant contract must have 0 hoch findings, got: ${JSON.stringify(highFindings)}`);
  assert.equal(res.riskLevel, "niedrig");
  assert.ok(res.riskScore <= 15, `Score should be low, got ${res.riskScore}`);
});

test("hardening: ReDoS performance check against massive 5000-line repeated text", () => {
  const baseBlock = "Die Parteien arbeiten vertrauensvoll zusammen. Der Anbieter liefert wöchentliche Statusberichte.\n";
  const massiveText = baseBlock.repeat(2500) + "\nDie Haftung für sämtliche Schäden ist ausgeschlossen.\n" + baseBlock.repeat(2500);
  
  const start = performance.now();
  const res = analyzeContract(massiveText);
  const duration = performance.now() - start;

  assert.ok(duration < 3500, `Massive text analysis must complete in <3500ms, took ${duration.toFixed(1)}ms`);
  const hit = res.findings.find((f) => f.type === "liability");
  assert.ok(hit, "Must detect liability clause in massive document");
});

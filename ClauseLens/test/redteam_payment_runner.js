import { analyzeContract } from "../src/analyze.js";

const testCases = [
  {
    id: "D1",
    category: "Set-off / Zurückbehaltung",
    type: "Adversarial Deceptive",
    description: "Strict payment without deduction & complete exclusion of withholding right (abbedungen)",
    clause: "Sämtliche Zahlungen haben ohne jeden Abzug oder Einbehalt zu erfolgen; ein Zurückbehaltungsrecht sowie Gegenrechte jeglicher Art gegenüber Forderungen des Verwenders werden hiermit ausdrücklich abbedungen.",
    expectedRisk: "hoch/mittel",
    expectedLegalViolation: "§ 309 Nr. 2 lit. b, § 309 Nr. 3 BGB, § 307 BGB"
  },
  {
    id: "D2",
    category: "Set-off / Verrechnungsverbot",
    type: "Adversarial Deceptive",
    description: "Verrechnungsverbot with subjective approval barrier (Zustimmung der Geschäftsleitung)",
    clause: "Eine Verrechnung mit etwaigen Gegenforderungen des Vertragspartners ist unzulässig, es sei denn, die Geschäftsleitung hat dieser Verrechnung im Voraus schriftlich zugestimmt.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 309 Nr. 3 BGB, § 307 BGB"
  },
  {
    id: "D3",
    category: "Contractual Penalty",
    type: "Adversarial Deceptive",
    description: "Hidden contractual penalty disguised as 'Pönale' excluding judicial mitigation",
    clause: "Für jeden schuldhaften Fall der Zuwiderhandlung gegen die Geheimhaltungsbestimmungen verpflichtet sich der Auftragnehmer zur Zahlung einer pauschalen Pönale in Höhe von 10.000,00 EUR unter Ausschluss des Fortsetzungszusammenhangs.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 309 Nr. 6 BGB, § 307 BGB, § 348 HGB"
  },
  {
    id: "D4",
    category: "Liquidated Damages / Penalty",
    type: "Adversarial Deceptive",
    description: "Lump-sum compensation cutting off debtor's proof of lesser damage (80 % Schadenspauschale)",
    clause: "Kündigt der Auftraggeber den Vertrag vorzeitig ohne wichtigen Grund, so ist der Auftragnehmer berechtigt, eine Schadenspauschale in Höhe von 80 % der vereinbarten Gesamtvergütung zu verlangen; der Nachweis eines geringeren oder nicht entstandenen Schadens ist dem Auftraggeber verwehrt.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 309 Nr. 5 lit. b BGB, § 309 Nr. 6 BGB"
  },
  {
    id: "D5",
    category: "Default Interest",
    type: "Adversarial Deceptive",
    description: "Deceptive short-period rate disguised with 'je angefangenem Monat' (1,5 %/Monat = 18 % p.a.)",
    clause: "Gerät der Besteller mit einer Zahlung in Verzug, so werden Verzugszinsen in Höhe von 1,5 % je angefangenem Monat berechnet.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 288 BGB, § 138 BGB (Wucher)"
  },
  {
    id: "D6",
    category: "Default Interest",
    type: "Adversarial Deceptive",
    description: "Financial margin disguised with '1200 Basispunkten' and avoiding 'Verzugszinsen' keyword",
    clause: "Im Verzugsfall ist die offene Forderung mit dem jeweiligen Leitzins der Europäischen Zentralbank zzgl. einer Zinsmarge von 1200 Basispunkten per annum zu verzinsen.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 288 BGB, § 307 BGB"
  },
  {
    id: "C1",
    category: "Set-off (Compliant)",
    type: "Compliant Baseline",
    description: "Statutory compliant set-off clause allowing undisputed or legally binding claims",
    clause: "Ein Recht zur Aufrechnung steht dem Kunden nur zu, soweit seine Gegenansprüche rechtskräftig festgestellt oder von uns unbestritten sind.",
    expectedRisk: "kein Befund / unbedenklich",
    expectedLegalViolation: "Konform mit § 309 Nr. 3 BGB"
  },
  {
    id: "C2",
    category: "Default Interest (Compliant)",
    type: "Compliant Baseline",
    description: "Statutory default interest explicitly citing § 288 BGB (5 percentage points above base rate)",
    clause: "Bei Zahlungsverzug ist der Rechnungsbetrag mit den gesetzlichen Verzugszinsen gemäß § 288 BGB in Höhe von 5 Prozentpunkten über dem Basiszinssatz zu verzinsen.",
    expectedRisk: "kein Befund / unbedenklich (oder 'niedrig')",
    expectedLegalViolation: "Konform mit § 288 Abs. 1 BGB"
  },
  {
    id: "C3",
    category: "Liquidated Damages (Compliant)",
    type: "Compliant Baseline",
    description: "Permissible reminder fee with explicit statutory right to prove lower damage",
    clause: "Im Falle eines Zahlungsverzugs berechnen wir eine Mahnpauschale von 15,00 EUR; dem Kunden bleibt der Nachweis vorbehalten, dass ein Schaden überhaupt nicht oder in wesentlich niedrigerer Höhe entstanden ist.",
    expectedRisk: "kein Befund / unbedenklich",
    expectedLegalViolation: "Konform mit § 309 Nr. 5 lit. b BGB"
  }
];

console.log("================================================================================");
console.log(" ClauseLens Red Team Stress-Test: Payment, Set-off & Penalty Clauses");
console.log("================================================================================\n");

const results = [];

for (const tc of testCases) {
  const analysis = analyzeContract(tc.clause);
  const hits = analysis.findings.map(f => ({
    type: f.type,
    severity: f.severity,
    why: f.why
  }));

  const detected = hits.length > 0;
  const highestSev = hits.reduce((max, h) => {
    const order = { hoch: 3, mittel: 2, niedrig: 1 };
    return (order[h.severity] || 0) > (order[max] || 0) ? h.severity : max;
  }, "none");

  results.push({
    ...tc,
    detected,
    highestSev,
    hits,
    riskScore: analysis.riskScore,
    riskLevel: analysis.riskLevel
  });
}

for (const r of results) {
  console.log(`[${r.id}] ${r.type.toUpperCase()}: ${r.category}`);
  console.log(`Clause: "${r.clause}"`);
  console.log(`Expected: ${r.expectedRisk} (${r.expectedLegalViolation})`);
  console.log(`Actual: Detected=${r.detected}, HighestSeverity=${r.highestSev}, Score=${r.riskScore}`);
  if (r.hits.length > 0) {
    r.hits.forEach(h => console.log(`   -> Hit: [${h.type}] severity=${h.severity}`));
  } else {
    console.log(`   -> No findings detected.`);
  }
  console.log("--------------------------------------------------------------------------------");
}

console.log("\nSummary Evaluation:");
const deceptiveTotal = results.filter(r => r.type === "Adversarial Deceptive").length;
const deceptiveDetected = results.filter(r => r.type === "Adversarial Deceptive" && r.detected).length;
const deceptiveAdequate = results.filter(r => r.type === "Adversarial Deceptive" && r.highestSev === "hoch").length;

const compliantTotal = results.filter(r => r.type === "Compliant Baseline").length;
const compliantClean = results.filter(r => r.type === "Compliant Baseline" && (!r.detected || r.highestSev === "niedrig")).length;
const compliantFalseAlarms = results.filter(r => r.type === "Compliant Baseline" && (r.highestSev === "mittel" || r.highestSev === "hoch")).length;

console.log(`Deceptive Clauses Detected: ${deceptiveDetected}/${deceptiveTotal} (${Math.round(deceptiveDetected/deceptiveTotal*100)}%)`);
console.log(`Deceptive Clauses Flagged as 'hoch': ${deceptiveAdequate}/${deceptiveTotal} (${Math.round(deceptiveAdequate/deceptiveTotal*100)}%)`);
console.log(`Compliant Baseline Clean: ${compliantClean}/${compliantTotal} (${Math.round(compliantClean/compliantTotal*100)}%)`);
console.log(`Compliant Baseline False Alarms (mittel/hoch): ${compliantFalseAlarms}/${compliantTotal}`);

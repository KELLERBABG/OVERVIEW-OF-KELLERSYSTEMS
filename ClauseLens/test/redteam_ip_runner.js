import { analyzeContract } from "../src/analyze.js";

const testCases = [
  {
    id: "D1",
    category: "IP & Telemetry (§ 307 / § 305c BGB)",
    type: "Adversarial Deceptive",
    description: "Camouflaged IP transfer of custom scripts & configurations under platform operation guise",
    clause: "Sämtliche vom Kunden eingebrachten Automatisierungsskripte, Prompts, Vorlagen und Konfigurationsdaten gehen zur Sicherung und Fortentwicklung des Plattformbetriebs entschädigungslos in das unbeschränkte geistige Eigentum des Anbieters über.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 305c BGB, § 307 BGB (Überraschende Enteignung kundenindividueller IP)"
  },
  {
    id: "D2",
    category: "IP & Telemetry (§ 307 / § 305c BGB)",
    type: "Adversarial Deceptive",
    description: "Commercialization & sublicensing of customer workflows derived from telemetry & feedback",
    clause: "Der Anbieter ist berechtigt, aus Telemetriedaten, Diagnoseprotokollen und Kundenfeedback abgeleitete Workflows weltweit und zeitlich unbegrenzt kommerziell zu lizenzieren oder zu veräußern, ohne dass dem Kunden hieraus Vergütungs- oder Beteiligungsansprüche erwachsen.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 307 BGB, § 31 UrhG, § 32 UrhG (Unentgeltlicher Entzug von Verwertungsrechten)"
  },
  {
    id: "D3",
    category: "Post-Contractual Non-Compete (§ 74 HGB)",
    type: "Adversarial Deceptive",
    description: "Post-contractual activity ban disguised as customer protection without mandatory compensation",
    clause: "Dem Auftragnehmer ist es für die Dauer von 24 Monaten nach Beendigung des Vertragsverhältnisses untersagt, für Kunden oder unmittelbare Geschäftspartner des Auftraggebers in dessen Geschäftsbereich beratend, vermittelnd oder operativ tätig zu werden; eine Karenzentschädigung wird nicht gewährt.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 74 Abs. 2 HGB analog, § 138 BGB, § 307 BGB (Verstecktes Wettbewerbsverbot ohne Karenz)"
  },
  {
    id: "D4",
    category: "Post-Contractual Non-Compete (§ 74 HGB)",
    type: "Adversarial Deceptive",
    description: "Extended loyalty duty acting as activity lock-in beyond termination with waived compensation",
    clause: "Das Verbot jedweder Tätigkeit im Tätigkeitsbereich des Dienstleisters gilt über die Beendigung dieses Vertrages hinaus für weitere zwei Jahre fort; jeglicher Anspruch auf Ausgleichszahlung ist abbedungen.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 74 Abs. 2 HGB, § 138 BGB, § 307 BGB (Existenzgefährdendes Tätigkeitsverbot)"
  },
  {
    id: "D5",
    category: "Jurisdiction (§ 38 ZPO / § 307 BGB)",
    type: "Adversarial Deceptive",
    description: "Grossly asymmetric foreign forum selection (Delaware seat for customer, worldwide for provider)",
    clause: "Ausschließlicher Gerichtsstand für sämtliche Klagen und Ansprüche des Kunden gegen den Anbieter ist Wilmington, Delaware, USA; der Anbieter bleibt demgegenüber berechtigt, Klagen an jedem anderen gesetzlichen Gerichtsstand zu erheben.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 307 BGB, § 305c BGB (Asymmetrische Zwangszuweisung in Auslandsgerichtsstand)"
  },
  {
    id: "D6",
    category: "Jurisdiction (§ 38 ZPO / § 307 BGB)",
    type: "Adversarial Deceptive",
    description: "Exclusive provider seat evading the explicit statutory keyword 'Gerichtsstand'",
    clause: "Sämtliche Streitigkeiten aus oder im Zusammenhang mit diesem Vertrag sind ausschließlich vor den sachlich und örtlich zuständigen Gerichten am Sitz des Anbieters auszutragen, wobei der Anbieter berechtigt ist, den Kunden an dessen allgemeinem Forum zu verklagen.",
    expectedRisk: "hoch",
    expectedLegalViolation: "§ 307 BGB (Asymmetrische Gerichtsstandsregelung getarnt als Gerichts-Zuständigkeit)"
  },
  {
    id: "C1",
    category: "IP & License (Compliant)",
    type: "Compliant Baseline",
    description: "Statutory compliant simple, purpose-bound, non-exclusive license for contract fulfillment",
    clause: "Der Kunde räumt dem Anbieter für die Dauer der Vertragslaufzeit ein einfaches, nicht ausschließliches, widerrufliches und auf den Zweck der vertragsgemäßen Leistungserbringung beschränktes Recht ein, die übermittelten Daten zu verarbeiten.",
    expectedRisk: "kein Befund / unbedenklich",
    expectedLegalViolation: "Konform mit § 31 Abs. 2 UrhG"
  },
  {
    id: "C2",
    category: "Non-Compete (Compliant)",
    type: "Compliant Baseline",
    description: "Compliant post-contractual non-compete with mandatory statutory Karenzentschädigung (>= 50 %)",
    clause: "Dem Mitarbeiter ist für die Dauer von einem Jahr nach Beendigung des Arbeitsverhältnisses eine Konkurrenztätigkeit untersagt, wobei der Arbeitgeber für die Dauer des Verbots eine monatliche Karenzentschädigung in Höhe von mindestens der Hälfte der zuletzt bezogenen vertragsmäßigen Leistungen nach § 74 Abs. 2 HGB zahlt.",
    expectedRisk: "kein Befund / unbedenklich",
    expectedLegalViolation: "Konform mit § 74 Abs. 2 HGB"
  },
  {
    id: "C3",
    category: "Jurisdiction (Compliant)",
    type: "Compliant Baseline",
    description: "Statutory jurisdiction clause referencing general civil procedure venue (§ 12 ff. ZPO)",
    clause: "Für alle Streitigkeiten aus oder im Zusammenhang mit diesem Vertrag gilt der gesetzliche Gerichtsstand nach den Vorschriften der Zivilprozessordnung.",
    expectedRisk: "kein Befund / unbedenklich",
    expectedLegalViolation: "Konform mit §§ 12 ff. ZPO"
  }
];

console.log("================================================================================");
console.log(" ClauseLens Red Team Stress-Test: IP, Data Rights & Lock-in Clauses");
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

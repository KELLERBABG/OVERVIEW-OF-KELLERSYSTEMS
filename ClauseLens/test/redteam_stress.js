import { analyzeContract } from "../src/analyze.js";

// ============================================================================
// 1. SIX DECEPTIVE VARIATIONS
// ============================================================================
const deceptiveVariations = [
  {
    id: "VAR-1-LIABILITY-CAP",
    title: "V1: Broad Liability Cap at Contract Fee masking Gross Negligence & Bodily Harm",
    legalCategory: "Broad limitation of liability masking gross negligence or personal injury (§ 309 Nr. 7 BGB)",
    text: "Die Verpflichtung des Auftragnehmers zum Ersatz von Schäden jeglicher Art – sei es aus Verzug, Unmöglichkeit, positiver Forderungsverletzung oder deliktischer Handlung – ist dem Grunde nach auf die Höhe des für die jeweilige Einzelleistung vereinbarten Netto-Auftragswertes beschränkt.",
    expectedSeverity: "hoch",
  },
  {
    id: "VAR-2-LIABILITY-JARGON",
    title: "V2: Masked Gross Negligence & Consequential Damages Waiver using Archaic Jargon ('abbedungen')",
    legalCategory: "Broad limitation of liability masking gross negligence or personal injury (§ 309 Nr. 7 BGB)",
    text: "Schadensersatzansprüche gegen das Unternehmen, gleich aus welchem Rechtsgrund, insbesondere für mittelbare Schäden, Folgeschäden, entgangenen Gewinn oder Betriebsunterbrechungen, werden hiermit vollumfänglich abbedungen, es sei denn, die Schadensursache beruht auf vorsätzlicher Pflichtverletzung der gesetzlichen Vertreter.",
    expectedSeverity: "hoch",
  },
  {
    id: "VAR-3-LIMITATION-DAYS",
    title: "V3: Limitation Reduction phrased in Days ('180 Kalendertage')",
    legalCategory: "Statute of limitations reductions (§ 309 Nr. 7 BGB)",
    text: "Sämtliche Ansprüche des Bestellers wegen Sach- und Rechtsmängeln verjähren mit Ablauf von 180 Kalendertagen nach Gefahrübergang bzw. Bereitstellung der Leistung.",
    expectedSeverity: "hoch",
  },
  {
    id: "VAR-4-LIMITATION-FORFEITURE",
    title: "V4: Hidden Statute of Limitations Reduction disguised as 3-Month Forfeiture Clause ('erlöschen endgültig / treten in den Verfall')",
    legalCategory: "Statute of limitations reductions (§ 309 Nr. 7 BGB)",
    text: "Jegliche Mängel- und Ersatzansprüche des Kunden erlöschen endgültig und treten mit einer Frist von drei Monaten nach Kenntnisnahme oder grob fahrlässiger Unkenntnis der anspruchsbegründenden Umstände in den Verfall.",
    expectedSeverity: "hoch",
  },
  {
    id: "VAR-5-NOTICE-HOURS",
    title: "V5: Ultra-short Notice Period in Hours ('48 Stunden') with Deemed Acceptance Fiction",
    legalCategory: "Unfairly short notice periods / deemed acceptance for defects (§ 308 Nr. 5 BGB)",
    text: "Erkennbare Mängel sind binnen einer Ausschlussfrist von 48 Stunden nach Ablieferung der Ware schriftlich zu rügen; unterbleibt die rechtzeitige Rüge, gilt das Produkt als vorbehaltlos genehmigt und vertragsgemäß abgenommen.",
    expectedSeverity: "hoch",
  },
  {
    id: "VAR-6-NOTICE-WORDNUM",
    title: "V6: 5-Day Notice Period with Word Number ('fünf Werktagen') and Absolute Acceptance Fiction",
    legalCategory: "Unfairly short notice periods / deemed acceptance for defects (§ 308 Nr. 5 BGB)",
    text: "Der Besteller hat die Leistungen nach Übergabe unverzüglich zu prüfen; Beanstandungen müssen innerhalb von fünf Werktagen schriftlich gerügt werden, widrigenfalls gilt die gesamte Lieferung als vollumfänglich mangelfrei genehmigt und jedwede Mängelhaftung als erloschen.",
    expectedSeverity: "hoch",
  },
];

// ============================================================================
// 2. THREE COMPLIANT STATUTORY BASELINE CLAUSES
// ============================================================================
const compliantBaselines = [
  {
    id: "BASE-1-LIABILITY-COMPLIANT",
    title: "B1: Fully Compliant BGB § 309 Nr. 7 Standard Liability Clause",
    legalCategory: "Compliant Statutory Liability Clause (§§ 276, 309 Nr. 7 BGB)",
    text: "Der Auftragnehmer haftet unbeschränkt für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit sowie für vorsätzlich oder grob fahrlässig verursachte Schäden. Bei leichter Fahrlässigkeit haftet der Auftragnehmer nur bei Verletzung wesentlicher Vertragspflichten (Kardinalpflichten), begrenzt auf den vertragstypischen, vorhersehbaren Schaden.",
    expectedSeverity: "keine / unkritisch",
  },
  {
    id: "BASE-2-LIMITATION-COMPLIANT",
    title: "B2: Fully Compliant Statutory Limitation Clause (§§ 438, 634a BGB)",
    legalCategory: "Compliant Statutory Limitation Clause",
    text: "Ansprüche des Kunden wegen Sachmängeln verjähren nach den gesetzlichen Vorschriften in zwei Jahren ab Ablieferung der Sache. Für Schadensersatzansprüche bei Vorsatz, grober Fahrlässigkeit oder Körperschäden gelten die regelmäßigen gesetzlichen Verjährungsfristen.",
    expectedSeverity: "keine / unkritisch",
  },
  {
    id: "BASE-3-NOTICE-COMPLIANT",
    title: "B3: Fully Compliant Commercial Defect Notice Clause (§ 377 HGB / § 308 Nr. 5 BGB)",
    legalCategory: "Compliant Notice Period / No Fiction Clause",
    text: "Der kaufmännische Kunde hat gelieferte Waren unverzüglich nach Ablieferung auf Mängel zu untersuchen und offenkundige Mängel innerhalb einer angemessenen Frist von mindestens zwei Wochen schriftlich anzuzeigen. Die gesetzlichen Vorschriften des § 377 HGB bleiben unberührt.",
    expectedSeverity: "keine / unkritisch",
  },
];

console.log("================================================================================");
console.log("RED TEAM STRESS TEST EXECUTION: ClauseLens src/analyze.js");
console.log("================================================================================\n");

console.log("--- PART 1: DECEPTIVE VARIATIONS (ADVERSARIAL STRESS TEST) ---\n");

const deceptiveResults = [];

for (const v of deceptiveVariations) {
  const res = analyzeContract(v.text);
  const detected = res.findings.length > 0;
  deceptiveResults.push({ ...v, res, detected });

  console.log(`[${v.id}] ${v.title}`);
  console.log(`Text: "${v.text}"`);
  console.log(`Status: ${detected ? "DETECTED" : "MISSED (BLINDSPOT!)"}`);
  console.log(`Risk Score: ${res.riskScore} (${res.riskLevel})`);
  console.log(`Findings count: ${res.findings.length}`);
  for (const f of res.findings) {
    console.log(`  - Type: ${f.type} | Severity: ${f.severity} | Title: "${f.titlePlainGerman}"`);
    console.log(`    Excerpt: "${f.excerpt}"`);
  }
  console.log("--------------------------------------------------------------------------------\n");
}

console.log("--- PART 2: COMPLIANT BASELINE CLAUSES (FALSE-POSITIVE RESISTANCE) ---\n");

const baselineResults = [];

for (const b of compliantBaselines) {
  const res = analyzeContract(b.text);
  const hasFalsePositive = res.counts.hoch > 0 || res.counts.mittel > 0;
  baselineResults.push({ ...b, res, hasFalsePositive });

  console.log(`[${b.id}] ${b.title}`);
  console.log(`Text: "${b.text}"`);
  console.log(`False Positive: ${hasFalsePositive ? "YES (FALSE ALARM!)" : "NO (CLEAN)"}`);
  console.log(`Risk Score: ${res.riskScore} (${res.riskLevel})`);
  console.log(`Findings count: ${res.findings.length}`);
  for (const f of res.findings) {
    console.log(`  - Type: ${f.type} | Severity: ${f.severity} | Title: "${f.titlePlainGerman}"`);
    console.log(`    Excerpt: "${f.excerpt}"`);
  }
  console.log("--------------------------------------------------------------------------------\n");
}

"use strict";

/**
 * ClauseLens Stress-Corpus Harness
 * ===============================
 *
 * Gemeinsamer Vertrag fuer alle Korpus-Slices unter `test/stress/corpus/*.js`.
 *
 * Ein Korpus-Slice exportiert:
 *
 *   export const CASES = [ ... ];
 *   export default CASES;
 *
 * Jeder Fall (Case) hat exakt diese Form:
 *
 *   {
 *     id:            "liability-euphemism-01",   // global eindeutig, kebab-case
 *     detector:      "liability",                // erwarteter Detektor (variant=deceptive)
 *                                                //   oder null bei variant="compliant"
 *     nearestDetector: "liability",              // nur compliant: naechster Detektor (Reporting)
 *     variant:       "deceptive" | "compliant",
 *     technique:     "euphemism",                // Evasions-/Formulierungstechnik
 *     severity:      "hoch" | "mittel" | "niedrig" | null,  // erwartete Mindestschaerfe
 *     alsoExpect:    ["data_grant"],             // optional: weitere Pflicht-Treffer (Vertraege)
 *     text:          "…Klauseltext…",
 *     note:          "Warum das eine versteckte Falle ist bzw. warum es unbedenklich ist."
 *   }
 *
 * Bewertung:
 *  - variant="deceptive": PASS wenn ein Finding mit `detector` existiert, dessen Schwere
 *    >= `severity` ist, und alle `alsoExpect`-Detektoren ebenfalls getroffen wurden.
 *  - variant="compliant": PASS wenn KEIN Finding mit Schwere mittel/hoch existiert
 *    (False-Positive-Resistenz).
 */

import { analyzeContract } from "../../src/analyze.js";

export const EXPECTED_DETECTORS = Object.freeze([
  "auto_renewal",
  "notice_period",
  "liability",
  "penalty",
  "jurisdiction",
  "non_compete",
  "data_grant",
  "default_interest",
  "unilateral_change",
  "setoff_restriction",
  "deemed_acceptance",
  "statute_of_limitations",
  "written_form_trap",
  "burden_of_proof",
  "dynamic_url_referral",
  "vicarious_agents",
  "unreasonable_audit",
  "data_lockin",
  "third_party_indemnity",
  "asymmetric_termination",
  "assignment_restriction",
  "force_majeure_asymmetry",
  "non_solicitation",
  "unilateral_convenience_termination",
  "excessive_survival",
  "price_escalation",
  "salvatorische_klausel_falle",
  "extended_retention_of_title",
  "subcontractor_uncontrolled",
  "audit_log_manipulation",
  "service_suspension",
  "insolvency_termination",
  "tax_indemnity_shift",
  "asymmetric_confidentiality",
  "as_is_software_disclaimer",
  "entire_agreement",
  "marketing_reference",
  "feedback_ip_grab",
  "fictitious_receipt",
  "unverbindliche_sla_falle",
  "agency_commission_exclusion",
  "agency_goodwill_waiver",
  "international_data_processing",
  "mandatory_expert_determination",
  "service_credit_forfeiture",
  "early_termination_charge",
  "maintenance_conditional_shutdown",
]);

export const TECHNIQUES = Object.freeze([
  "direct", // Klartext, unverschleierte Falle
  "euphemism", // beschwichtigende, weiche Formulierung
  "synonym", // ungewoehnliche Synonyme / Archaismen / Juristenjargon
  "english", // englischsprachige Klausel
  "word-number", // Zahlen ausgeschrieben / roemisch
  "split-sentence", // Trigger und Rechtsfolge auf zwei Saetze/Striche verteilt
  "list-buried", // Falle mitten in einer langen Aufzaehlung begraben
  "nominal-style", // Nominalstil, Passiv, Verbvermeidung
  "cross-reference", // Falle nur ueber Verweis auf anderen Abschnitt sichtbar
  "dach", // AT/CH-Terminologie (Pönale, Gewährleistung, OR, ABGB)
  "typo-evasion", // Soft-Hyphen, Zero-Width, Homoglyph, Fullwidth, Whitespace-Trick
  "compound-multi", // mehrere Fallen in einer Klausel
  "full-contract", // kompletter mehrklauseliger Vertrag
]);

export const SEVERITY_RANK = Object.freeze({ niedrig: 1, mittel: 2, hoch: 3 });

const TECHNIQUE_SET = new Set(TECHNIQUES);
const DETECTOR_SET = new Set(EXPECTED_DETECTORS);
const ID_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

/** Validiert einen einzelnen Korpus-Fall. Gibt eine Liste von Fehlertexten zurueck. */
export function validateCase(entry, seenIds = new Set()) {
  const errors = [];
  const where = entry && typeof entry.id === "string" ? entry.id : "(ohne id)";
  if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
    return ["Fall ist kein Objekt"];
  }
  if (typeof entry.id !== "string" || !ID_RE.test(entry.id)) {
    errors.push(`${where}: id fehlt oder ist nicht kebab-case`);
  } else if (seenIds.has(entry.id)) {
    errors.push(`${where}: doppelte id`);
  }
  if (!["deceptive", "compliant"].includes(entry.variant)) {
    errors.push(`${where}: variant muss "deceptive" oder "compliant" sein`);
  }
  if (!TECHNIQUE_SET.has(entry.technique)) {
    errors.push(`${where}: unbekannte technique "${entry.technique}"`);
  }
  if (typeof entry.text !== "string" || entry.text.trim().length < 25) {
    errors.push(`${where}: text fehlt oder ist zu kurz`);
  }
  if (typeof entry.note !== "string" || entry.note.trim().length < 10) {
    errors.push(`${where}: note fehlt (kurze Begruendung noetig)`);
  }
  if (entry.variant === "deceptive") {
    if (!DETECTOR_SET.has(entry.detector)) {
      errors.push(`${where}: unbekannter detector "${entry.detector}"`);
    }
    if (!Object.prototype.hasOwnProperty.call(SEVERITY_RANK, entry.severity)) {
      errors.push(`${where}: severity muss "hoch"/"mittel"/"niedrig" sein`);
    }
    if (entry.alsoExpect !== undefined) {
      if (!Array.isArray(entry.alsoExpect)) {
        errors.push(`${where}: alsoExpect muss ein Array sein`);
      } else {
        for (const d of entry.alsoExpect) {
          if (!DETECTOR_SET.has(d)) errors.push(`${where}: alsoExpect enthaelt "${d}"`);
        }
      }
    }
  }
  if (entry.variant === "compliant") {
    if (entry.detector !== null) errors.push(`${where}: compliant-Faelle brauchen detector=null`);
    if (entry.severity !== null) errors.push(`${where}: compliant-Faelle brauchen severity=null`);
    if (entry.nearestDetector !== undefined && entry.nearestDetector !== "mixed" && !DETECTOR_SET.has(entry.nearestDetector)) {
      errors.push(`${where}: unbekannter nearestDetector "${entry.nearestDetector}"`);
    }
  }
  return errors;
}

/**
 * Validiert einen oder mehrere Korpus-Slices.
 * @param {Array<{name: string, cases: any[]}>} slices
 */
export function validateCorpus(slices) {
  const errors = [];
  const seen = new Set();
  const cases = [];
  for (const slice of slices) {
    if (!slice || !Array.isArray(slice.cases)) {
      errors.push(`Slice ${slice && slice.name ? slice.name : "?"}: keine cases-Liste`);
      continue;
    }
    if (slice.cases.length === 0) {
      errors.push(`Slice ${slice.name}: leer`);
    }
    for (const entry of slice.cases) {
      errors.push(...validateCase(entry, seen));
      if (entry && typeof entry.id === "string") seen.add(entry.id);
      if (entry && typeof entry === "object") cases.push({ ...entry, slice: slice.name });
    }
  }
  return { errors, cases };
}

function maxRank(hits) {
  return hits.reduce((acc, h) => Math.max(acc, SEVERITY_RANK[h.severity] || 0), 0);
}

function rankToSeverity(rank) {
  return rank >= 3 ? "hoch" : rank >= 2 ? "mittel" : rank >= 1 ? "niedrig" : null;
}

/** Fuehrt einen Fall durch die Engine und bewertet ihn. */
export function evaluateCase(entry) {
  const text = entry.text;
  let analysis = null;
  let error = null;
  try {
    analysis = analyzeContract(text);
  } catch (err) {
    error = err && err.message ? err.message : String(err);
  }

  const findings = analysis ? analysis.findings : [];
  const hits = findings.map((f) => ({ type: f.type, severity: f.severity }));
  const types = new Set(hits.map((h) => h.type));

  const result = {
    id: entry.id,
    slice: entry.slice || null,
    detector: entry.detector ?? null,
    nearestDetector: entry.nearestDetector ?? null,
    variant: entry.variant,
    technique: entry.technique,
    expectedSeverity: entry.severity ?? null,
    riskScore: analysis ? analysis.riskScore : null,
    riskLevel: analysis ? analysis.riskLevel : null,
    findingCount: findings.length,
    hits,
    types: [...types],
    error,
    pass: false,
    reason: null,
    text,
    note: entry.note,
    alsoExpect: entry.alsoExpect || [],
  };

  if (error) {
    result.reason = `Engine-Fehler: ${error}`;
    return result;
  }

  if (entry.variant === "compliant") {
    const loud = hits.filter((h) => (SEVERITY_RANK[h.severity] || 0) >= 2);
    result.falsePositives = loud;
    result.pass = loud.length === 0;
    result.reason = loud.length === 0 ? null : `False Positive: ${loud.map((h) => `${h.type}/${h.severity}`).join(", ")}`;
    return result;
  }

  // variant === "deceptive"
  const own = hits.filter((h) => h.type === entry.detector);
  const detected = own.length > 0;
  const ownSeverity = rankToSeverity(maxRank(own));
  const severityOk = detected && maxRank(own) >= (SEVERITY_RANK[entry.severity] || 1);
  const missingAlso = result.alsoExpect.filter((d) => !types.has(d));

  result.detected = detected;
  result.detectedSeverity = ownSeverity;
  result.severityOk = severityOk;
  result.missingAlso = missingAlso;
  result.silentMiss = !detected && findings.length === 0;
  result.wrongTypeOnly = !detected && findings.length > 0;

  const problems = [];
  if (!detected) problems.push("Zieldetektor nicht getroffen");
  else if (!severityOk) problems.push(`Schwere zu niedrig (${ownSeverity} < ${entry.severity})`);
  if (missingAlso.length > 0) problems.push(`Zusatzdetektoren fehlen: ${missingAlso.join(", ")}`);
  result.pass = problems.length === 0;
  result.reason = problems.length === 0 ? null : problems.join("; ");
  return result;
}

/** Fuehrt den kompletten Korpus aus und liefert Einzelergebnisse + Aggregate. */
export function runCorpus(cases) {
  const results = cases.map(evaluateCase);
  return { results, stats: aggregate(results) };
}

export function aggregate(results) {
  const stats = {
    total: results.length,
    passed: results.filter((r) => r.pass).length,
    failed: results.filter((r) => !r.pass).length,
    deceptive: { total: 0, passed: 0, detected: 0, severityOk: 0, silentMiss: 0, wrongTypeOnly: 0 },
    compliant: { total: 0, clean: 0, falsePositiveCases: 0, falsePositiveFindings: 0 },
    byDetector: {},
    byTechnique: {},
    byCompliantScope: {},
    directDeceptive: { total: 0, detected: 0, passed: 0, detectionRate: 1 },
    errors: results.filter((r) => r.error).length,
  };
  stats.passRate = stats.total ? stats.passed / stats.total : 1;

  for (const r of results) {
    const bucket = r.variant === "compliant" ? stats.compliant : stats.deceptive;
    bucket.total++;
    if (r.variant === "compliant") {
      if (r.pass) bucket.clean++;
      else {
        bucket.falsePositiveCases++;
        bucket.falsePositiveFindings += r.falsePositives ? r.falsePositives.length : 0;
      }
    } else {
      if (r.detected) bucket.detected++;
      if (r.severityOk) bucket.severityOk++;
      if (r.silentMiss) bucket.silentMiss++;
      if (r.wrongTypeOnly) bucket.wrongTypeOnly++;
      if (r.pass) bucket.passed++;
    }

    // Detektor- und Technikbuckets werden nur ueber taueschende Faelle gebildet.
    // Compliant-Faelle laufen bewusst alle unter technique "direct" und wuerden die
    // Technikstatistik sonst verwaessern; sie werden ueber byCompliantScope gefuehrt.
    if (r.variant === "deceptive") {
      const d = (stats.byDetector[r.detector || "unknown"] ||= { total: 0, passed: 0, failed: 0, failedIds: [] });
      d.total++;
      if (r.pass) d.passed++;
      else {
        d.failed++;
        d.failedIds.push(r.id);
      }

      const t = (stats.byTechnique[r.technique] ||= { total: 0, passed: 0, failed: 0, failedIds: [] });
      t.total++;
      if (r.pass) t.passed++;
      else {
        t.failed++;
        t.failedIds.push(r.id);
      }

      if (r.technique === "direct") {
        stats.directDeceptive.total++;
        if (r.detected) stats.directDeceptive.detected++;
        if (r.pass) stats.directDeceptive.passed++;
      }
    } else {
      const c = (stats.byCompliantScope[r.nearestDetector || "unknown"] ||= { total: 0, passed: 0, failed: 0, failedIds: [] });
      c.total++;
      if (r.pass) c.passed++;
      else {
        c.failed++;
        c.failedIds.push(r.id);
      }
    }
  }

  stats.deceptive.detectionRate = stats.deceptive.total ? stats.deceptive.detected / stats.deceptive.total : 1;
  stats.directDeceptive.detectionRate = stats.directDeceptive.total ? stats.directDeceptive.detected / stats.directDeceptive.total : 1;
  stats.deceptive.severityRate = stats.deceptive.total ? stats.deceptive.severityOk / stats.deceptive.total : 1;
  stats.compliant.cleanRate = stats.compliant.total ? stats.compliant.clean / stats.compliant.total : 1;
  return stats;
}

export function percent(x) {
  return `${Math.round(x * 1000) / 10}%`;
}

/** Alle nicht bestandenen Falle ausserhalb der Compliant-Slices = Blindspots. */
export function blindspots(results) {
  return results.filter((r) => !r.pass && r.variant === "deceptive");
}

export function falsePositiveCases(results) {
  return results.filter((r) => !r.pass && r.variant === "compliant");
}

function truncate(text, max = 260) {
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length > max ? `${flat.slice(0, max - 1)}…` : flat;
}

/** Rendert den Markdown-Report. */
export function formatReport(run, meta = {}) {
  const { results, stats } = run;
  const misses = blindspots(results);
  const fps = falsePositiveCases(results);
  const lines = [];

  lines.push("# ClauseLens Stress-Korpus — Detektionsbericht");
  lines.push("");
  lines.push(`Quelle: ${meta.source || "test/stress/corpus/*.js"} · Engine: src/analyze.js (47 Detektoren)`);
  lines.push("");
  lines.push(`Falle gesamt: **${stats.total}** · bestanden: **${stats.passed}** · Fehlschläge: **${stats.failed}** (${percent(stats.passRate)} bestanden)`);
  lines.push("");
  lines.push("## Aggregate");
  lines.push("");
  lines.push("| Metrik | Wert |");
  lines.push("|---|---|");
  lines.push(`| Täuschende Klauseln | ${stats.deceptive.total} |`);
  lines.push(`| davon Zieldetektor getroffen | ${stats.deceptive.detected} (${percent(stats.deceptive.detectionRate)}) |`);
  lines.push(`| davon ausreichend scharf (≥ erwartet) | ${stats.deceptive.severityOk} (${percent(stats.deceptive.severityRate)}) |`);
  lines.push(`| stille Blindspots (kein einziger Treffer) | ${stats.deceptive.silentMiss} |`);
  lines.push(`| nur Fremdtreffer (falscher Detektor) | ${stats.deceptive.wrongTypeOnly} |`);
  lines.push(`| harmlose Klauseln | ${stats.compliant.total} |`);
  lines.push(`| davon sauber (kein mittel/hoch) | ${stats.compliant.clean} (${percent(stats.compliant.cleanRate)}) |`);
  lines.push(`| False-Positive-Fälle | ${stats.compliant.falsePositiveCases} |`);
  lines.push(`| Engine-Fehler | ${stats.errors} |`);
  lines.push("");

  lines.push("## Trefferquote je Detektor (nur täuschende Fälle)");
  lines.push("");
  lines.push("| Detektor | Fälle | bestanden | fehlend | Fehlschläge |");
  lines.push("|---|---|---|---|---|");
  for (const [key, d] of Object.entries(stats.byDetector).sort((a, b) => b[1].failed - a[1].failed || a[0].localeCompare(b[0]))) {
    lines.push(`| \`${key}\` | ${d.total} | ${d.passed} | ${d.failed} | ${d.failedIds.slice(0, 4).join(", ")}${d.failedIds.length > 4 ? ", …" : ""} |`);
  }
  lines.push("");

  lines.push("## Trefferquote je Evasionstechnik (nur täuschende Fälle)");
  lines.push("");
  lines.push("| Technik | Fälle | bestanden | fehlend | Bestehensquote |");
  lines.push("|---|---|---|---|---|");
  for (const [key, t] of Object.entries(stats.byTechnique).sort((a, b) => b[1].failed - a[1].failed || a[0].localeCompare(b[0]))) {
    lines.push(`| \`${key}\` | ${t.total} | ${t.passed} | ${t.failed} | ${percent(t.total ? t.passed / t.total : 1)} |`);
  }
  lines.push("");
  lines.push("### Kontrollgruppe: unverschleierte Klauseln (technique \`direct\`)");
  lines.push("");
  lines.push(`Von ${stats.directDeceptive.total} im Klartext formulierten Fallen wurden ${stats.directDeceptive.detected} erkannt (${percent(stats.directDeceptive.detectionRate)}). Diese Quote muss nahe 100 % liegen — Verluste hier sind reine Regellücken, keine Formulierungsblindspots.`);
  lines.push("");

  lines.push("## Harmlose Klauseln je Themenbereich");
  lines.push("");
  lines.push("| Bereich | Fälle | sauber | False Positives |");
  lines.push("|---|---|---|---|");
  for (const [key, c] of Object.entries(stats.byCompliantScope).sort((a, b) => b[1].failed - a[1].failed || a[0].localeCompare(b[0]))) {
    lines.push(`| \`${key}\` | ${c.total} | ${c.passed} | ${c.failed} |`);
  }
  lines.push("");

  lines.push(`## Blindspots (${misses.length})`);
  lines.push("");
  if (misses.length === 0) lines.push("_Keine — alle täuschenden Fälle wurden erkannt._");
  for (const r of misses) {
    lines.push(`### ${r.id} — \`${r.detector}\` (${r.technique})`);
    lines.push("");
    lines.push(`- Erwartet: ${r.expectedSeverity}${r.alsoExpect.length ? ` + ${r.alsoExpect.join(", ")}` : ""}`);
    lines.push(`- Befund: ${r.reason}`);
    lines.push(`- Treffer: ${r.hits.length ? r.hits.map((h) => `${h.type}/${h.severity}`).join(", ") : "keine"}`);
    lines.push(`- Risiko-Score: ${r.riskScore} (${r.riskLevel})`);
    lines.push(`- Lücke: ${r.note}`);
    lines.push("");
    lines.push("```text");
    lines.push(r.text.trim());
    lines.push("```");
    lines.push("");
  }

  lines.push(`## False Positives auf harmlosen Klauseln (${fps.length})`);
  lines.push("");
  if (fps.length === 0) lines.push("_Keine._");
  for (const r of fps) {
    lines.push(`### ${r.id} — ${r.reason}`);
    lines.push("");
    lines.push(`- Bereich: \`${r.nearestDetector || "unknown"}\` (${r.technique})`);
    lines.push(`- Hinweis: ${r.note}`);
    lines.push("");
    lines.push("```text");
    lines.push(r.text.trim());
    lines.push("```");
    lines.push("");
  }

  return `${lines.join("\n")}\n`;
}

/** Maschinenlesbare Version des Laufs. */
export function serializeReport(run, meta = {}) {
  const { results, stats } = run;
  return {
    schemaVersion: "1.0.0",
    source: meta.source || "test/stress/corpus/*.js",
    stats,
    results: results.map((r) => ({
      id: r.id,
      slice: r.slice,
      variant: r.variant,
      detector: r.detector,
      nearestDetector: r.nearestDetector,
      technique: r.technique,
      expectedSeverity: r.expectedSeverity,
      detected: r.detected ?? null,
      detectedSeverity: r.detectedSeverity ?? null,
      severityOk: r.severityOk ?? null,
      silentMiss: r.silentMiss ?? null,
      wrongTypeOnly: r.wrongTypeOnly ?? null,
      missingAlso: r.missingAlso ?? [],
      falsePositives: r.falsePositives ?? [],
      pass: r.pass,
      reason: r.reason,
      riskScore: r.riskScore,
      riskLevel: r.riskLevel,
      hits: r.hits,
      note: r.note,
    })),
  };
}

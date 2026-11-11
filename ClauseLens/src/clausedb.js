"use strict";

/**
 * ClauseLens Klauseldatenbank — Loader, Validator, Aggregation und Export.
 * ======================================================================
 *
 * Die Datenbank liegt als JSONL vor: eine Klausel pro Zeile in
 * `data/clauses/*.jsonl`. `data/manifest.json` ist der massgebliche Register-
 * eintrag: er listet jede Slice-Datei, und der Loader verweigert eine Slice,
 * die auf der Platte liegt, aber nicht registriert ist.
 *
 * Der Datensatzvertrag steht in `schema/clause.schema.json`; die Felderliste
 * hier muss mit dessen `required` uebereinstimmen (test/clause_db.test.js
 * vergleicht beides) und die Detektorliste kommt direkt aus der Regelmaschine
 * (src/analyze.js), damit Schema und Engine nicht auseinanderlaufen.
 *
 * Keine Laufzeitabhaengigkeiten ausser node:fs/node:path — wie der Rest des
 * Projekts.
 */

import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { analyzeContract, RULES } from "./analyze.js";

export const RECORD_FIELDS = Object.freeze([
  "id",
  "docFamily",
  "clauseType",
  "clauseText",
  "why",
  "legalBasis",
  "caseLaw",
  "otherStatutes",
  "jurisdiction",
  "language",
  "severity",
  "detector",
  "detectorStatus",
  "evasion",
]);

export const DOC_FAMILIES = Object.freeze([
  "arbeitsvertrag",
  "mietvertrag",
  "werkvertrag_kaufvertrag",
  "b2b_agb",
  "b2c_agb",
  "gesellschaftsvertrag",
  "lizenz_saas_nda",
  "aufhebungs_abfindung",
  "dach_varianten",
  "en_law_msa_sow_dpa",
  "handelsvertretervertrag",
  "franchisevertrag",
  "leasingvertrag",
  "darlehen_verbraucherkredit",
  "versicherungsvertrag",
  "anteilskauf_spa",
  "reisevertrag",
  "telekommunikationsvertrag",
  "energieliefervertrag",
  "pflege_behandlungsvertrag",
  "arbeitnehmerueberlassung",
  "bausparvertrag",
  "kommission",
  "spedition_lager",
  "insolvenzplan_starug",
  "bautraegervertrag",
]);

export const JURISDICTIONS = Object.freeze(["DE", "AT", "CH", "DACH", "EU", "UK", "US"]);
export const LANGUAGES = Object.freeze(["de", "en", "bilingual"]);
export const SEVERITIES = Object.freeze(["hoch", "mittel", "niedrig"]);
/**
 * Drei Zustaende, weil zwei die Wirklichkeit nicht abbilden konnten:
 *
 *   mapped       — eine Regel deckt den Klauseltyp UND feuert auf diese Formulierung (gemessen).
 *   mapped_blind — eine Regel deckt den Klauseltyp, reagiert aber nicht auf diese Formulierung
 *                  (die Regel existiert; die Formulierung ist fuer sie unsichtbar).
 *   no_detector  — fuer den Klauseltyp existiert ueberhaupt keine Regel (gezaehlte Luecke).
 *
 * Nur `mapped` ist eine Messung. `mapped_blind` ist eine benannte, gezaehlte Schwaeche: sie darf
 * nicht als Abdeckung gelesen und nicht durch Absenken einer Sammelquote verdeckt werden.
 */
export const DETECTOR_STATUSES = Object.freeze(["mapped", "mapped_blind", "no_detector"]);
export const EVASION_TECHNIQUES = Object.freeze([
  "direct",
  "euphemism",
  "synonym",
  "english",
  "word-number",
  "split-sentence",
  "list-buried",
  "nominal-style",
  "cross-reference",
  "dach",
  "typo-evasion",
  "compound-multi",
]);

/** Detektornamen kommen aus der Engine selbst — keine zweite Liste pflegen. */
export const DETECTOR_TYPES = Object.freeze(RULES.map((rule) => rule.type));

export const LIMITS = Object.freeze({
  idMin: 3,
  idMax: 80,
  clauseTypeMin: 3,
  clauseTypeMax: 80,
  textMin: 60,
  textMax: 1200,
  whyMin: 25,
  whyMax: 400,
  minContainmentLength: 100,
  /** Wortmengen-Aehnlichkeit ab der zwei Klauseltexte als derselbe Text gelten. */
  nearDuplicateJaccard: 0.6,
  /** Unter dieser Wortzahl wird die Aehnlichkeit nicht geprueft (zu kurze Texte sind nicht aussagekraeftig). */
  nearDuplicateMinTokens: 8,
  /** Wortfenster, das zwei Klauseln nicht gemeinsam haben duerfen, auch wenn der Rest abweicht. */
  sharedPhraseWords: 16,
});

const ID_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

const STATUS_RANK = Object.freeze({ mapped: 2, mapped_blind: 1, no_detector: 0 });

/** Traegt ein Klauseltyp mehrere Zustaende, gilt der staerkste (mapped > mapped_blind > no_detector). */
export function strongestStatus(current, next) {
  if (current === undefined) return next;
  return (STATUS_RANK[next] ?? -1) > (STATUS_RANK[current] ?? -1) ? next : current;
}

/** Normalform fuer Duplikat- und Containment-Pruefung. */
export function normalizeClauseText(text) {
  return String(text)
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[^a-z0-9äöüß]+/g, " ")
    .trim();
}

/** Wortmenge eines Klauseltextes fuer die Aehnlichkeitspruefung. */
export function clauseTokens(text) {
  return normalizeClauseText(text).split(" ").filter((word) => word.length > 3);
}

/**
 * Findet Textnaehe jenseits byte-exakter Gleichheit.
 *
 * Zwei Klauseln sind derselbe Klauseltyp, wenn sie dieselbe Wortmenge verwenden
 * (Jaccard >= LIMITS.nearDuplicateJaccard) oder wenn sie ein zusammenhaengendes
 * Wortfenster von LIMITS.sharedPhraseWords Woertern teilen. Beides ist ein
 * Buildfehler, kein Audit-Ergebnis: ein umformulierter Zwilling traegt keine neue
 * Klausel bei, sondern nur Redundanz.
 */
export function findNearDuplicates(records) {
  const tokens = new Map();
  const sets = new Map();
  for (const record of records) {
    if (typeof record.clauseText !== "string") continue;
    const words = normalizeClauseText(record.clauseText).split(" ");
    tokens.set(record.id, words);
    sets.set(record.id, new Set(clauseTokens(record.clauseText)));
  }

  const similar = [];
  const list = records.filter((record) => sets.has(record.id));
  for (let i = 0; i < list.length; i++) {
    const a = list[i];
    const sa = sets.get(a.id);
    for (let j = i + 1; j < list.length; j++) {
      const b = list[j];
      const sb = sets.get(b.id);
      if (sa.size < LIMITS.nearDuplicateMinTokens && sb.size < LIMITS.nearDuplicateMinTokens) continue;
      const ratio = Math.min(sa.size, sb.size) / Math.max(sa.size, sb.size);
      if (ratio < 0.4) continue; // eine Jaccard-Aehnlichkeit von 0.6 setzt aehnliche Wortmengen voraus
      let intersection = 0;
      for (const word of sa) if (sb.has(word)) intersection++;
      const jaccard = intersection / (sa.size + sb.size - intersection);
      if (jaccard >= LIMITS.nearDuplicateJaccard) {
        similar.push({ a, b, kind: "jaccard", score: Math.round(jaccard * 1000) / 1000 });
      }
    }
  }

  const windowSize = LIMITS.sharedPhraseWords;
  const windows = new Map();
  for (const record of list) {
    const words = tokens.get(record.id);
    for (let start = 0; start + windowSize <= words.length; start++) {
      const key = words.slice(start, start + windowSize).join(" ");
      if (!windows.has(key)) windows.set(key, []);
      windows.get(key).push(record);
    }
  }
  const seen = new Set(similar.map((pair) => `${pair.a.id}|${pair.b.id}`));
  const phrases = [];
  for (const [phrase, sharers] of windows) {
    if (sharers.length < 2) continue;
    for (let i = 0; i < sharers.length; i++) {
      for (let j = i + 1; j < sharers.length; j++) {
        const key = `${sharers[i].id}|${sharers[j].id}`;
        if (seen.has(key)) continue;
        seen.add(key);
        phrases.push({ a: sharers[i], b: sharers[j], kind: "phrase", score: `${windowSize} Woerter`, phrase });
      }
    }
  }
  return { pairs: similar, phrases };
}

function isStringArray(value) {
  return Array.isArray(value) && value.every((v) => typeof v === "string" && v.trim().length > 0);
}

function lengthError(label, value, min, max) {
  if (typeof value !== "string") return `${label} muss ein String sein`;
  if (value.length < min) return `${label} zu kurz (${value.length} < ${min})`;
  if (value.length > max) return `${label} zu lang (${value.length} > ${max})`;
  return null;
}

/** Validiert einen Datensatz gegen den Vertrag aus schema/clause.schema.json. */
export function validateRecord(record, slice = null) {
  const errors = [];
  if (!record || typeof record !== "object" || Array.isArray(record)) return ["kein Objekt"];

  const keys = Object.keys(record);
  const missing = RECORD_FIELDS.filter((k) => !keys.includes(k));
  const extra = keys.filter((k) => !RECORD_FIELDS.includes(k));
  if (missing.length) errors.push(`fehlende Felder: ${missing.join(", ")}`);
  if (extra.length) errors.push(`unerlaubte Zusatzfelder: ${extra.join(", ")}`);
  if (missing.length) return errors;

  if (!ID_RE.test(record.id) || record.id.length < LIMITS.idMin || record.id.length > LIMITS.idMax) {
    errors.push(`id "${record.id}" ist nicht kebab-case oder hat unzulaessige Laenge`);
  }
  if (slice && slice.prefix && !record.id.startsWith(`${slice.prefix}-`)) {
    errors.push(`id "${record.id}" verletzt das Slice-Praefix "${slice.prefix}-"`);
  }
  if (!DOC_FAMILIES.includes(record.docFamily)) errors.push(`unbekannte docFamily "${record.docFamily}"`);
  const familyError = lengthError("clauseType", record.clauseType, LIMITS.clauseTypeMin, LIMITS.clauseTypeMax);
  if (familyError) errors.push(familyError);
  const textError = lengthError("clauseText", record.clauseText, LIMITS.textMin, LIMITS.textMax);
  if (textError) errors.push(textError);
  const whyError = lengthError("why", record.why, LIMITS.whyMin, LIMITS.whyMax);
  if (whyError) errors.push(whyError);

  if (!Array.isArray(record.legalBasis) || record.legalBasis.length === 0) {
    errors.push("legalBasis braucht mindestens eine Norm");
  } else if (!isStringArray(record.legalBasis)) {
    errors.push("legalBasis enthaelt leere oder nicht-textuelle Eintraege");
  }
  if (!Array.isArray(record.caseLaw) || !isStringArray(record.caseLaw)) {
    if (!(Array.isArray(record.caseLaw) && record.caseLaw.length === 0)) {
      errors.push("caseLaw muss ein String-Array sein (leer erlaubt)");
    }
  }
  if (!Array.isArray(record.otherStatutes) || !isStringArray(record.otherStatutes)) {
    if (!(Array.isArray(record.otherStatutes) && record.otherStatutes.length === 0)) {
      errors.push("otherStatutes muss ein String-Array sein (leer erlaubt)");
    }
  }

  if (!JURISDICTIONS.includes(record.jurisdiction)) errors.push(`unbekannte jurisdiction "${record.jurisdiction}"`);
  if (!LANGUAGES.includes(record.language)) errors.push(`unbekannte language "${record.language}"`);
  if (!SEVERITIES.includes(record.severity)) errors.push(`unbekannte severity "${record.severity}"`);
  if (!DETECTOR_STATUSES.includes(record.detectorStatus)) {
    errors.push(`unbekannter detectorStatus "${record.detectorStatus}"`);
  }
  if (record.evasion !== null && !EVASION_TECHNIQUES.includes(record.evasion)) {
    errors.push(`unbekannte evasion "${record.evasion}"`);
  }

  if (record.detectorStatus === "mapped" || record.detectorStatus === "mapped_blind") {
    if (typeof record.detector !== "string") {
      errors.push(`detectorStatus "${record.detectorStatus}" verlangt einen detector-String`);
    } else if (!DETECTOR_TYPES.includes(record.detector)) {
      errors.push(`unbekannter detector "${record.detector}" (nicht in src/analyze.js)`);
    }
  }
  if (record.detectorStatus === "no_detector" && record.detector !== null) {
    errors.push('detectorStatus "no_detector" verlangt detector: null');
  }

  return errors;
}

/** Laedt Manifest, Slices und Datensaetze und prueft alle Invarianten. */
export function loadDatabase(root = process.cwd()) {
  const errors = [];
  const manifestPath = path.join(root, "data", "manifest.json");
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch (err) {
    return { manifest: null, records: [], errors: [`data/manifest.json nicht lesbar: ${err.message}`] };
  }

  const clausesDir = path.join(root, manifest.clausesDirectory);
  const registered = new Set(manifest.slices.map((slice) => slice.file));
  let onDisk = [];
  try {
    onDisk = readdirSync(clausesDir).filter((f) => f.endsWith(".jsonl"));
  } catch (err) {
    errors.push(`${manifest.clausesDirectory} nicht lesbar: ${err.message}`);
  }
  for (const file of onDisk) {
    if (!registered.has(file)) {
      errors.push(`nicht im Manifest registrierte Slice-Datei "${file}" — das Manifest besitzt die Datenbank`);
    }
  }

  const records = [];
  for (const slice of manifest.slices) {
    const file = path.join(clausesDir, slice.file);
    let content;
    try {
      content = readFileSync(file, "utf8");
    } catch (err) {
      errors.push(`${slice.file}: nicht lesbar (${err.message})`);
      continue;
    }
    const lines = content.split(/\r?\n/);
    lines.forEach((line, index) => {
      if (!line.trim()) return;
      let record;
      try {
        record = JSON.parse(line);
      } catch (err) {
        errors.push(`${slice.file}:${index + 1}: kein gueltiges JSON (${err.message})`);
        return;
      }
      for (const error of validateRecord(record, slice)) {
        errors.push(`${slice.file}:${index + 1}: ${error}`);
      }
      records.push({ ...record, slice: slice.file });
    });
  }

  const byId = new Map();
  for (const record of records) {
    if (byId.has(record.id)) {
      errors.push(`doppelte id "${record.id}" (${byId.get(record.id).slice} und ${record.slice})`);
    } else {
      byId.set(record.id, record);
    }
  }

  const byText = new Map();
  for (const record of records) {
    if (typeof record.clauseText !== "string") continue;
    const key = normalizeClauseText(record.clauseText);
    if (byText.has(key)) {
      errors.push(`identischer Klauseltext: "${record.id}" (${record.slice}) und "${byText.get(key).id}" (${byText.get(key).slice})`);
    } else {
      byText.set(key, record);
    }
  }

  const entries = [...byText.entries()].sort((a, b) => a[0].length - b[0].length);
  for (let i = 0; i < entries.length; i++) {
    const [shortKey, shortRecord] = entries[i];
    if (shortKey.length < LIMITS.minContainmentLength) continue;
    for (let j = i + 1; j < entries.length; j++) {
      if (entries[j][0].includes(shortKey)) {
        errors.push(`Text-Containment: "${shortRecord.id}" ist vollstaendig in "${entries[j][1].id}" enthalten`);
      }
    }
  }

  const nearDuplicates = findNearDuplicates(records);
  for (const pair of nearDuplicates.pairs) {
    errors.push(
      `nahezu identischer Klauseltext (Wortmengen-Aehnlichkeit ${pair.score}, Grenze ${LIMITS.nearDuplicateJaccard}): ` +
        `"${pair.a.id}" (${pair.a.slice}) und "${pair.b.id}" (${pair.b.slice}) — eine der beiden Klauseln muss entfallen oder ein anderer Klauseltyp werden`
    );
  }
  for (const pair of nearDuplicates.phrases) {
    errors.push(
      `identische Passage ueber ${pair.score}: "${pair.a.id}" (${pair.a.slice}) und "${pair.b.id}" (${pair.b.slice})` +
        ` — gemeinsame Passage: "${pair.phrase.slice(0, 120)}…"`
    );
  }

  return { manifest, records, errors };
}

/**
 * Der veroeffentlichte Datensatzvertrag: genau die Felder aus dem Schema.
 * Interne Felder (z.B. die Herkunfts-Slice) gehoeren nicht in Exporte.\n */
export function publicRecord(record) {
  const output = {};
  for (const field of RECORD_FIELDS) output[field] = record[field];
  return output;
}

/** Zaehlt Datensaetze und Abdeckung. */
export function summarize(records) {
  const summary = {
    total: records.length,
    perFamily: {},
    perDetector: {},
    perSeverity: { hoch: 0, mittel: 0, niedrig: 0 },
    perStatus: { mapped: 0, mapped_blind: 0, no_detector: 0 },
    perFamilyStatus: {},
    perJurisdiction: {},
    perLanguage: {},
    clauseTypes: 0,
    mappedClauseTypes: 0,
    blindClauseTypes: 0,
    gapClauseTypes: 0,
  };
  const clauseTypeSeen = new Map();
  for (const record of records) {
    summary.perFamily[record.docFamily] = (summary.perFamily[record.docFamily] || 0) + 1;
    const familyStatus = (summary.perFamilyStatus[record.docFamily] = summary.perFamilyStatus[record.docFamily] || {
      mapped: 0,
      mapped_blind: 0,
      no_detector: 0,
    });
    if (familyStatus[record.detectorStatus] !== undefined) familyStatus[record.detectorStatus]++;
    summary.perDetector[record.detector || "(kein Detektor)"] = (summary.perDetector[record.detector || "(kein Detektor)"] || 0) + 1;
    if (summary.perSeverity[record.severity] !== undefined) summary.perSeverity[record.severity]++;
    if (summary.perStatus[record.detectorStatus] !== undefined) summary.perStatus[record.detectorStatus]++;
    summary.perJurisdiction[record.jurisdiction] = (summary.perJurisdiction[record.jurisdiction] || 0) + 1;
    summary.perLanguage[record.language] = (summary.perLanguage[record.language] || 0) + 1;
    const key = `${record.docFamily}::${record.clauseType}`;
    clauseTypeSeen.set(key, strongestStatus(clauseTypeSeen.get(key), record.detectorStatus));
  }
  summary.clauseTypes = clauseTypeSeen.size;
  summary.gapClauseTypes = [...clauseTypeSeen.values()].filter((s) => s === "no_detector").length;
  summary.blindClauseTypes = [...clauseTypeSeen.values()].filter((s) => s === "mapped_blind").length;
  summary.mappedClauseTypes = summary.clauseTypes - summary.gapClauseTypes - summary.blindClauseTypes;
  return summary;
}

/**
 * Baut die Klauseltyp-zu-Detektor-Matrix. Zeile = Klauseltyp, Spalte = Detektor
 * oder Abdeckungsluecke; damit sind die Luecken norm- und typweise zaehlbar.
 */
export function buildCoverage(records) {
  const rows = new Map();
  for (const record of records) {
    const key = `${record.docFamily}::${record.clauseType}`;
    if (!rows.has(key)) {
      rows.set(key, {
        docFamily: record.docFamily,
        clauseType: record.clauseType,
        detector: record.detector,
        detectorStatus: record.detectorStatus,
        statuses: { mapped: 0, mapped_blind: 0, no_detector: 0 },
        count: 0,
        jurisdictions: new Set(),
        severities: new Set(),
        ids: [],
      });
    }
    const row = rows.get(key);
    row.count++;
    if (row.statuses[record.detectorStatus] !== undefined) row.statuses[record.detectorStatus]++;
    row.detectorStatus = strongestStatus(row.detectorStatus, record.detectorStatus);
    if (record.detectorStatus !== "no_detector") row.detector = record.detector;
    row.jurisdictions.add(record.jurisdiction);
    row.severities.add(record.severity);
    row.ids.push(record.id);
  }
  const list = [...rows.values()]
    .map((row) => ({
      ...row,
      jurisdictions: [...row.jurisdictions].sort(),
      severities: [...row.severities].sort(),
      ids: row.ids.sort(),
    }))
    .sort((a, b) => {
      const ad = a.detector || "~";
      const bd = b.detector || "~";
      return ad.localeCompare(bd) || a.docFamily.localeCompare(b.docFamily) || a.clauseType.localeCompare(b.clauseType);
    });
  const totalsByDetector = {};
  for (const row of list) {
    const key = row.detector || "(kein Detektor)";
    totalsByDetector[key] = (totalsByDetector[key] || 0) + 1;
  }
  const withRule = list.filter((row) => row.detectorStatus !== "no_detector");
  return {
    rows: list,
    totalsByDetector,
    gapClauseTypes: list.filter((row) => row.detectorStatus === "no_detector").length,
    blindClauseTypes: list.filter((row) => row.detectorStatus === "mapped_blind").length,
    coveredDetectors: withRule.reduce((set, row) => set.add(row.detector), new Set()).size,
  };
}

/** Konsolenausgabe: Fallzahlen je Dokumentfamilie und Abdeckungsmatrix. */
export function renderCounts(summary, coverage, verification = null) {
  const lines = [];
  lines.push(`Klauseldatenbank: ${summary.total} Datensaetze · ${summary.clauseTypes} Klauseltypen`);
  lines.push(
    `  verifiziert zugeordnet: ${summary.perStatus.mapped} Datensaetze / ${summary.mappedClauseTypes} Klauseltypen`
  );
  lines.push(
    `  zugeteilt, aber blind: ${summary.perStatus.mapped_blind} Datensaetze / ${summary.blindClauseTypes} Klauseltypen · ` +
      `ohne Regel: ${summary.perStatus.no_detector} Datensaetze / ${summary.gapClauseTypes} Klauseltypen`
  );
  lines.push(`  Schwere: hoch=${summary.perSeverity.hoch} mittel=${summary.perSeverity.mittel} niedrig=${summary.perSeverity.niedrig}`);
  lines.push("");
  lines.push("Datensaetze je Dokumentfamilie");
  lines.push("  Familie                          n  verifiziert      blind   ohne Regel");
  for (const family of DOC_FAMILIES) {
    const count = summary.perFamily[family] || 0;
    const status = summary.perFamilyStatus[family] || { mapped: 0, mapped_blind: 0, no_detector: 0 };
    lines.push(
      `  ${family.padEnd(32)}${String(count).padStart(4)}${String(status.mapped).padStart(12)}` +
        `${String(status.mapped_blind).padStart(11)}${String(status.no_detector).padStart(13)}`
    );
  }
  lines.push("");
  lines.push("Abdeckung: Klauseltypen je Detektor");
  for (const [key, count] of Object.entries(coverage.totalsByDetector).sort((a, b) => b[1] - a[1])) {
    lines.push(`  ${key.padEnd(34)}${String(count).padStart(3)} Klauseltypen`);
  }
  lines.push("");
  lines.push(`Abgedeckte Detektoren: ${coverage.coveredDetectors}/${DETECTOR_TYPES.length} · Klauseltypen ohne Regel: ${coverage.gapClauseTypes}`);
  if (verification) {
    lines.push("");
    lines.push("Engine-Verifikation der Zuordnung");
    lines.push(
      `  verifiziert: ${verification.mapped} Klauseln, bei denen die beanspruchte Regel feuert ` +
        `(Abweichungen: ${verification.notFiring.length} — muss 0 sein)`
    );
    lines.push(
      `  blind: ${verification.blind} Klauseln mit Regel fuer den Typ, die auf diese Formulierung nicht reagiert ` +
        `(${Math.round(verification.blindShare * 1000) / 10}% der Klauseln mit beanspruchter Regel)`
    );
    lines.push(`  ohne Regel: ${verification.gaps} Klauseln, davon ${verification.gapHits.length} mit Fremdtreffer`);
  }
  return lines.join("\n");
}

/** Markdown-Report der Abdeckungsmatrix. */
export function renderCoverageMarkdown(summary, coverage, verification = null) {
  const lines = [];
  lines.push("# Klauseldatenbank — Abdeckungsmatrix");
  lines.push("");
  lines.push(
    `Quelle: data/clauses/*.jsonl (Manifest data/manifest.json) · ${summary.total} Datensaetze · ` +
      `${summary.clauseTypes} Klauseltypen · verifiziert zugeordnet ${summary.mappedClauseTypes} · ` +
      `zugeteilt aber blind ${summary.blindClauseTypes} · ohne Regel ${summary.gapClauseTypes}`
  );
  lines.push("");
  lines.push(
    "Drei Zuordnungszustaende, weil zwei die Wirklichkeit nicht abbilden: **verifiziert** (eine Regel deckt " +
      "den Klauseltyp und feuert auf die Formulierung), **blind** (eine Regel deckt den Klauseltyp, reagiert aber " +
      "nicht auf die Formulierung) und **ohne Regel** (fuer den Klauseltyp existiert keine Regel). Der blinde " +
      "Zustand ist eine benannte Schwaeche des Regelwerks, keine Abdeckung."
  );
  lines.push("");
  lines.push("## Klauseltypen ohne Detektor");
  lines.push("");
  lines.push("| Dokumentfamilie | Klauseltyp | Datensaetze |");
  lines.push("|---|---|---|");
  for (const row of coverage.rows.filter((r) => r.detectorStatus === "no_detector")) {
    lines.push(`| \`${row.docFamily}\` | ${row.clauseType} | ${row.ids.join(", ")} |`);
  }
  lines.push("");
  lines.push("## Zuordnung Klauseltyp zu Detektor");
  lines.push("");
  lines.push("| Detektor | Dokumentfamilie | Klauseltyp | Datensaetze |");
  lines.push("|---|---|---|---|");
  for (const row of coverage.rows.filter((r) => r.detectorStatus === "mapped" || r.detectorStatus === "mapped_blind")) {
    lines.push(`| \`${row.detector}\` | \`${row.docFamily}\` | ${row.clauseType} | ${row.count} |`);
  }
  lines.push("");
  if (verification) {
    lines.push("## Zugeteilt, aber blind — Regel vorhanden, reagiert nicht auf die Formulierung");
    lines.push("");
    lines.push(
      `Diese ${verification.blind} Klauseln (${Math.round(verification.blindShare * 1000) / 10}% aller Klauseln mit ` +
        "beanspruchter Regel) benennen eine Regel fuer ihren Klauseltyp, die auf ihre konkrete Formulierung nicht " +
        "reagiert. Sie zaehlen nicht als abgedeckt."
    );
    lines.push("");
    lines.push("| id | Dokumentfamilie | Klauseltyp | beanspruchter Detektor | Fremdtreffer |");
    lines.push("|---|---|---|---|---|");
    for (const item of verification.blindRecords) {
      lines.push(`| \`${item.id}\` | \`${item.docFamily}\` | ${item.clauseType} | \`${item.detector}\` | ${item.found.join(", ") || "—"} |`);
    }
    lines.push("");
    if (verification.notFiring.length > 0) {
      lines.push("## Fehler: als verifiziert gefuehrt, aber die Regel feuert nicht");
      lines.push("");
      for (const item of verification.notFiring) {
        lines.push(`- \`${item.id}\` (${item.clauseType}) beansprucht \`${item.detector}\`, gefunden: ${item.found.join(", ") || "nichts"}`);
      }
      lines.push("");
    }
    lines.push("## Als Abdeckungsluecke gefuehrt, aber mit Fremdtreffer");
    lines.push("");
    lines.push("| id | Dokumentfamilie | Klauseltyp | Fremdtreffer |");
    lines.push("|---|---|---|---|---|");
    for (const item of verification.gapHits) {
      lines.push(`| \`${item.id}\` | \`${item.docFamily}\` | ${item.clauseType} | ${item.found.join(", ")} |`);
    }
    lines.push("");
  }
  return `${lines.join("\n")}\n`;
}

function csvCell(value) {
  const text = String(value).replace(/"/g, '""');
  return /[",\n]/.test(text) ? `"${text}"` : text;
}

/**
 * Erzeugt alle Exportdateien deterministisch (Sortierung nach id, keine
 * Zeitstempel): JSON, JSONL und CSV sowie die Abdeckungsmatrix.
 */
export function buildExports(database) {
  const { records, errors } = database;
  const sorted = [...records].sort((a, b) => a.id.localeCompare(b.id));
  const summary = summarize(sorted);
  const coverage = buildCoverage(sorted);
  const verification = verifyMappings(sorted);
  const exportDir = "data/export";

  const published = sorted.map((record) => publicRecord(record));
  const json = `${JSON.stringify({ schemaVersion: "1.0.0", recordSchema: "schema/clause.schema.json", stats: summary, records: published }, null, 2)}\n`;
  const jsonl = `${published.map((record) => JSON.stringify(record)).join("\n")}\n`;

  const csvColumns = [
    "id",
    "docFamily",
    "clauseType",
    "severity",
    "jurisdiction",
    "language",
    "detector",
    "detectorStatus",
    "evasion",
    "legalBasis",
    "caseLaw",
    "otherStatutes",
    "why",
    "clauseText",
  ];
  const arrayCell = (value) => (Array.isArray(value) ? value.join(" | ") : value === null ? "" : value);
  const csvRows = [csvColumns.join(",")];
  for (const record of sorted) {
    csvRows.push(csvColumns.map((column) => csvCell(arrayCell(record[column]))).join(","));
  }
  const csv = `${csvRows.join("\n")}\n`;

  const coverageJson = `${JSON.stringify(
    {
      schemaVersion: "1.0.0",
      stats: summary,
      totalsByDetector: coverage.totalsByDetector,
      coveredDetectors: coverage.coveredDetectors,
      detectorTypesInEngine: DETECTOR_TYPES.length,
      blindClauseTypes: coverage.blindClauseTypes,
      gapClauseTypes: coverage.gapClauseTypes,
      rows: coverage.rows.map((row) => ({ ...row, severities: row.severities.join(" | ") })),
      verification: {
        mapped: verification.mapped,
        mappedVerified: verification.mappedVerified,
        mappedVerifiedRate: Math.round(verification.mappedVerifiedRate * 1000) / 1000,
        notFiring: verification.notFiring,
        blind: verification.blind,
        blindShare: Math.round(verification.blindShare * 1000) / 1000,
        blindRecords: verification.blindRecords,
        blindFiring: verification.blindFiring,
        gaps: verification.gaps,
        gapHits: verification.gapHits,
      },
    },
    null,
    2
  )}\n`;

  return {
    summary,
    coverage,
    verification,
    files: {
      [`${exportDir}/clauses.json`]: json,
      [`${exportDir}/clauses.jsonl`]: jsonl,
      [`${exportDir}/clauses.csv`]: csv,
      [`${exportDir}/coverage.json`]: coverageJson,
      [`${exportDir}/coverage.md`]: renderCoverageMarkdown(summary, coverage, verification),
    },
    errorCount: errors.length,
  };
}

/**
 * Prueft die Detektorzuordnung gegen die echte Engine und trennt die drei Zustaende.
 *
 * `mapped` behauptet, dass eine Regel den Klauseltyp abdeckt UND auf diese Formulierung
 * feuert — das ist eine Messung und muss fuer jeden einzelnen Datensatz gelten, nicht im
 * Durchschnitt. `mapped_blind` benennt den Fall dazwischen: die Regel existiert fuer den
 * Klauseltyp, reagiert aber nicht auf die konkrete Formulierung. `no_detector` heisst: fuer
 * den Klauseltyp existiert keine Regel (gezaehlte Luecke).
 */
export function verifyMappings(records) {
  const mapped = records.filter((record) => record.detectorStatus === "mapped");
  const blind = records.filter((record) => record.detectorStatus === "mapped_blind");
  const gaps = records.filter((record) => record.detectorStatus === "no_detector");

  const notFiring = [];
  for (const record of mapped) {
    const found = analyzeContract(record.clauseText).findings.map((finding) => finding.type);
    if (!found.includes(record.detector)) {
      notFiring.push({ id: record.id, detector: record.detector, clauseType: record.clauseType, docFamily: record.docFamily, found });
    }
  }

  const blindRecords = [];
  const blindFiring = [];
  for (const record of blind) {
    const found = analyzeContract(record.clauseText).findings.map((finding) => finding.type);
    const item = { id: record.id, detector: record.detector, clauseType: record.clauseType, docFamily: record.docFamily, found };
    blindRecords.push(item);
    if (found.includes(record.detector)) blindFiring.push(item);
  }

  const gapHits = [];
  for (const record of gaps) {
    const found = analyzeContract(record.clauseText).findings.map((finding) => finding.type);
    if (found.length > 0) {
      gapHits.push({ id: record.id, clauseType: record.clauseType, docFamily: record.docFamily, found });
    }
  }

  const claimed = mapped.length + blind.length;
  return {
    mapped: mapped.length,
    mappedVerified: mapped.length - notFiring.length,
    mappedVerifiedRate: mapped.length ? (mapped.length - notFiring.length) / mapped.length : 1,
    notFiring,
    blind: blind.length,
    blindRecords,
    blindFiring,
    blindShare: claimed ? blind.length / claimed : 0,
    claimed,
    gaps: gaps.length,
    gapHits,
  };
}

/** Abfrageoberflaeche: Filter auf der Datenbank. */
export function query(records, filter = {}) {
  return records.filter((record) => {
    if (filter.family && record.docFamily !== filter.family) return false;
    if (filter.detector && record.detector !== filter.detector) return false;
    if (filter.severity && record.severity !== filter.severity) return false;
    if (filter.jurisdiction && record.jurisdiction !== filter.jurisdiction) return false;
    if (filter.gaps && record.detectorStatus !== "no_detector") return false;
    if (filter.mapped && record.detectorStatus !== "mapped") return false;
    if (filter.blind && record.detectorStatus !== "mapped_blind") return false;
    if (filter.type && !record.clauseType.toLowerCase().includes(filter.type.toLowerCase())) return false;
    if (filter.id && record.id !== filter.id) return false;
    return true;
  });
}

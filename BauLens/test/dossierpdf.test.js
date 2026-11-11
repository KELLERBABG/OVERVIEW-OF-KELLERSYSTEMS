import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runChecks } from "../src/engine.js";
import { computeReportHash } from "../src/dossier.js";
import { getRuleById } from "../src/rules/index.js";
import { A4, inspectXref } from "../src/pdf.js";
import { buildDossierPdf, buildPlanSheet, renderDossierPdf, writeDossierPdf } from "../src/dossierpdf.js";

/* The dossier PDF is what leaves the office, so the tests have to see the
   engine's own facts inside the file rather than a plausible-looking layout.
   The runs below are real: they come from the same fixtures and the same
   `runChecks` the CLI and the sidecar use. */

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

const WINANSI_TO_TEXT = {
  0x80: "€", 0x82: "‚", 0x83: "ƒ", 0x84: "„", 0x85: "…", 0x86: "†", 0x87: "‡",
  0x88: "ˆ", 0x89: "‰", 0x8a: "Š", 0x8b: "‹", 0x8c: "Œ", 0x8e: "Ž", 0x91: "‘",
  0x92: "’", 0x93: "“", 0x94: "”", 0x95: "•", 0x96: "–", 0x97: "—", 0x98: "˜",
  0x99: "™", 0x9a: "š", 0x9b: "›", 0x9c: "œ", 0x9e: "ž", 0x9f: "Ÿ",
};
const WINANSI_TEXT = new Set(Object.values(WINANSI_TO_TEXT));

/* The writer spells a handful of glyphs out in ASCII rather than dropping them;
   an expected string has to pass through the same mapping before it is compared
   with what a reader will show. */
const ASCII_FALLBACK = {
  "↑": "^", "↓": "v", "→": "->", "←": "<-", "✓": "ok", "✗": "x", "·": "-",
  "−": "-", "≈": "~", "≤": "<=", "≥": ">=", "\u00a0": " ", "′": "'", "″": '"', "×": "x",
};

const decodeWinAnsi = (text) =>
  Array.from(text, (ch) => WINANSI_TO_TEXT[ch.charCodeAt(0)] ?? ch).join("");

function asPrinted(text) {
  let out = "";
  for (const ch of String(text)) {
    const code = ch.codePointAt(0);
    if (code < 0x80 || (code >= 0xa0 && code <= 0xff) || WINANSI_TEXT.has(ch)) {
      out += ch;
      continue;
    }
    out += ASCII_FALLBACK[ch] ?? "?";
  }
  return out;
}

/* Content streams are uncompressed, so the text a reader shows is the ordered
   sequence of `(literal)` strings in the file. Line breaks inside those are a
   layout decision, not part of the fact under test, which is why most
   comparisons run against a whitespace-free form. */
function pdfText(buffer) {
  const raw = buffer.toString("latin1");
  const strings = [];
  const re = /\((?:\\.|[^\\()])*\)/g;
  let match;
  while ((match = re.exec(raw)) !== null) {
    strings.push(match[0].slice(1, -1).replace(/\\([\\()])/g, "$1"));
  }
  return decodeWinAnsi(strings.join("\n"));
}

const squeeze = (text) => String(text).replace(/\s+/g, "");

/* writeDossierPdf may report failure by throwing or by rejecting; the test is
   about the failure, not about which of the two it picked. */
const settle = async (value) => (value && typeof value.then === "function" ? await value : value);
async function failure(fn) {
  try {
    await settle(fn());
  } catch (error) {
    return error;
  }
  assert.fail("expected the call to fail");
}

const STATUS_TITLE = (counts) =>
  counts.fail === 0
    ? (counts.warn === 0 ? "BESTANDEN (VOLLSTÄNDIG GENEHMIGUNGSFÄHIG)" : "MIT AUFLAGEN (BEFREIUNGSERFORDERNISSE)")
    : "NICHT BESTANDEN (MATERIELLE MÄNGEL)";

const ROW_LABEL = (result) =>
  result.findings.some((f) => f.status === "fail")
    ? "MANGEL"
    : result.findings.some((f) => f.status === "warn")
      ? "AUFLAGE"
      : "KONFORM";

/** A real run plus the project data the dossier is headed with. */
function dossierInputs(name, extra = {}) {
  const spec = fixture(name);
  const runResult = runChecks(spec);
  const projectData = {
    projekt: spec.projekt,
    aktenzeichen: "BV-2026-000123",
    flurstueck: "Flur 4, Flurstück 108/2",
    gemarkung: "Musterhausen",
    entwurfsverfasser: "Architekturbüro Dipl.-Ing. Muster",
    bauherr: "Projektentwicklung GmbH & Co. KG",
    ...spec,
    ...extra,
  };
  return {
    spec,
    runResult,
    projectData,
    hash: computeReportHash(projectData, runResult.verdict),
  };
}

const citation = (result) => result.para ?? getRuleById(result.id)?.para ?? null;

test("dossier pdf: builds a well-formed DIN A4 document for a passing and a failing run", () => {
  for (const name of ["good_spec.json", "bad_spec.json"]) {
    const { runResult, projectData, hash } = dossierInputs(name);
    const built = buildDossierPdf(projectData, runResult, { datum: "02.10.2026" });
    const raw = built.buffer.toString("latin1");

    assert.ok(Buffer.isBuffer(built.buffer), `${name}: a buffer, not a string`);
    assert.ok(raw.startsWith("%PDF-1.4"));
    assert.ok(raw.trimEnd().endsWith("%%EOF"));
    assert.equal(inspectXref(built.buffer).valid, true, `${name}: the xref table must resolve`);

    assert.ok(built.pages >= 1);
    assert.equal(built.pages, built.audit.pages.length, `${name}: the audit reports every page`);
    assert.equal(built.audit.overflow, false, `${name}: no text may leave the page`);
    for (const page of built.audit.pages) {
      assert.equal(page.overflow, false, `${name}: page ${page.index} overflowed`);
      assert.ok(page.ops > 0, `${name}: page ${page.index} drew nothing`);
    }

    /* The seal prints the same hash the HTML dossier computes, so the two
       artefacts can be checked against each other. */
    assert.match(built.hash, /^[0-9a-f]{64}$/);
    assert.equal(built.hash, hash, `${name}: the seal must carry the dossier hash`);
  }
});

test("dossier pdf: renders the engine's facts, not a placeholder", () => {
  const { runResult, projectData, hash } = dossierInputs("bad_spec.json");
  const built = buildDossierPdf(projectData, runResult, { datum: "02.10.2026" });
  const text = squeeze(pdfText(built.buffer));

  for (const phrase of [
    "Bauaufsichtlicher Vorprüfungsbefund",
    "Prüfergebnis je Modul",
    "Lageplan",
    "Legende",
    "Schriftfeld",
    "Prüfziffer (SHA-256)",
  ]) {
    assert.ok(text.includes(squeeze(phrase)), `missing fixed string: ${phrase}`);
  }
  assert.ok(text.includes(hash), "the seal must show the full hash, wrapped or not");

  for (const value of [
    projectData.projekt,
    projectData.aktenzeichen,
    projectData.flurstueck,
    projectData.entwurfsverfasser,
    projectData.bauherr,
  ]) {
    assert.ok(text.includes(squeeze(asPrinted(value))), `meta value not rendered: ${value}`);
  }

  /* One row per rule, each with its citation: the paragraph reference is the
     point of the whole document. */
  assert.ok(runResult.results.length >= 9, "the run covers the rule set");
  for (const result of runResult.results) {
    const para = citation(result);
    assert.ok(para, `rule ${result.id} has no citation to render`);
    assert.ok(text.includes(squeeze(asPrinted(para))), `missing citation ${para} (${result.id})`);
    assert.ok(text.includes(squeeze(asPrinted(result.title))), `missing module title ${result.title}`);
  }

  /* And the findings themselves, verbatim: a message that is truncated or
     dropped is a defect, not a cosmetic difference. */
  const failing = runResult.results.flatMap((result) =>
    result.findings.filter((f) => f.status === "fail").map((f) => f.message)
  );
  assert.ok(failing.length > 0, "bad_spec must produce at least one failure to render");
  for (const message of failing) {
    assert.ok(text.includes(squeeze(asPrinted(message))), `missing finding: ${message}`);
  }
});

test("dossier pdf: the site plan is its own landscape sheet with Massketten, Nordpfeil and legend", () => {
  const { runResult, projectData } = dossierInputs("good_spec.json");
  const built = buildDossierPdf(projectData, runResult, { datum: "02.10.2026" });
  const text = squeeze(pdfText(built.buffer));
  const sheets = built.audit.pages.filter((page) => page.width > page.height);

  /* Genau ein Blatt ist quer, und es ist das Zeichnungsblatt: ein Lageplan in
     einem Einreichungsordner liegt auf seinem eigenen Blatt. */
  assert.equal(sheets.length, 1, "exactly one Blatt is querformat");
  assert.equal(sheets[0].width, A4.height, "quer means A4 on its side");
  assert.equal(sheets[0].height, A4.width);
  assert.equal(sheets[0].ops > 0, true);
  assert.equal(sheets[0].overflow, false);
  assert.equal(sheets[0].index > 1, true, "the drawing sheet follows the findings");

  for (const phrase of [
    "Lageplan",
    "Abstandsflächen zum Grundstück · Maßstab 1:250 · Maße in Metern",
    "Legende",
    "Grundstücksgrenze / Bezugsgrenze",
    "Grenzabstand laut Prüfung",
    "Hauskörper",
    "Grundfläche 280 m² · GRZ 0,35 × 800 m²",
    "Abstand gewahrt",
    "Abstand unterschritten",
    "Vergleichswert 3,5 m (§6 MBO / LBO)",
    "Schriftfeld",
    "Blatt 2 – Lageplan (Abstandsflächen)",
    "1:250 · Maße in Metern",
  ]) {
    assert.ok(text.includes(squeeze(phrase)), `missing drawing-sheet string: ${phrase}`);
  }

  /* good_spec deklariert GRZ 0,35 und 800 m² Grundstück: der Hauskörper ist
     280 m² groß und wird mit dem angenommenen Verhältnis 4:3 zu 19,32 × 14,49 m,
     mit den geprüften Abständen das Grundstücksrechteck 27,32 × 23,49 m. */
  for (const mass of ["5,00", "4,00", "19,32", "14,49", "27,32", "23,49"]) {
    assert.ok(text.includes(mass), `missing Masszahl ${mass}`);
  }

  /* Der Nordpfeil steht auf dem Blatt, die Fußzeile zählt das Querblatt mit. */
  assert.ok(text.includes("N"), "the Nordpfeil carries its N");
  assert.ok(text.includes(`Seite${sheets[0].index}von${built.pages}`), "the quer sheet is numbered like every page");
});

test("dossier pdf: the sheet scale follows the data and the dimension chains stay on the paper", () => {
  /* Ein größeres Grundstück passt nicht mehr in 1:200: der Maßstab muss kleiner
     werden, statt die Maßketten über den Blattrand zu schieben. */
  const wide = {
    projekt: "Grosses Grundstück",
    grundstueck: { grenzabstaende: { norden: 21.0, sueden: 18.0, osten: 22.0, westen: 20.0 } },
  };
  const run = dossierInputs("good_spec.json").runResult;
  const built = buildDossierPdf(wide, run, { datum: "02.10.2026" });
  const text = squeeze(pdfText(built.buffer));

  assert.equal(built.audit.overflow, false, "a wide parcel must not push ink off the sheet");
  assert.ok(text.includes("Maßstab1:500"), "ein 50-m-Bezugsrechteck braucht einen kleineren Maßstab als 1:200");
  for (const mass of ["21,00", "18,00", "22,00", "20,00", "8,00", "6,00", "50,00", "45,00"]) {
    assert.ok(text.includes(mass), `missing Masszahl ${mass}`);
  }

  /* Ohne jede Grundflächenangabe bleibt nur das Bezugsrechteck - und das Blatt
     sagt das, statt erfundene Maße wie geprüfte aussehen zu lassen. */
  assert.ok(
    text.includes(squeeze("Grundfläche schematisch (keine Angabe in der Planung)")),
    "the schematic fallback must be labelled as such"
  );
});

test("dossier pdf: the Hauskörper follows the declared Grundfläche, not a placeholder", () => {
  const run = dossierInputs("good_spec.json").runResult;
  const setbacks = { norden: 4, sueden: 4, osten: 4, westen: 4 };

  const cases = [
    {
      /* 0,50 × 600 m² = 300 m², und mit 4:3 genau 20,00 × 15,00 m. */
      label: "GRZ × Grundstücksfläche",
      spec: { projekt: "GRZ-Weg", grundstueck: { flaeche_m2: 600, grenzabstaende: setbacks }, gebaeude: { grz_ist: 0.5 } },
      derivation: "Grundfläche 300 m² · GRZ 0,50 × 600 m²",
      masses: ["20,00", "15,00", "28,00", "23,00"],
    },
    {
      /* 240 m² Nutzfläche über 3 Geschosse: 80 m² je Geschoss. */
      label: "Nutzfläche / Geschosse",
      spec: { projekt: "Nutzflächenweg", grundstueck: { grenzabstaende: setbacks }, gebaeude: { nutzflaeche_m2: 240, geschosse_ueber_grund: 3 } },
      derivation: "Grundfläche 80 m² · Nutzfläche 240 m² / 3 Geschosse",
      masses: ["10,33", "7,75", "18,33", "15,75"],
    },
    {
      /* Ohne Geschossangabe ist die Nutzfläche die Grundfläche: genau so trägt
         der DXF-Import die gemessene überbaute Fläche ein. */
      label: "Nutzfläche ohne Geschossangabe",
      spec: { projekt: "Nur Nutzfläche", grundstueck: { grenzabstaende: setbacks }, gebaeude: { nutzflaeche_m2: 120 } },
      derivation: "Grundfläche 120 m² · Nutzfläche, Geschosszahl nicht angegeben",
      masses: ["12,65", "9,49", "20,65", "17,49"],
    },
  ];

  const drawn = [];
  for (const entry of cases) {
    const built = buildDossierPdf(entry.spec, run, { datum: "02.10.2026" });
    const text = squeeze(pdfText(built.buffer));

    assert.equal(built.audit.overflow, false, `${entry.label}: ink left the sheet`);
    assert.ok(
      text.includes(squeeze(entry.derivation)),
      `${entry.label}: the sheet must name the derivation, expected "${entry.derivation}"`
    );
    for (const mass of entry.masses) {
      assert.ok(text.includes(mass), `${entry.label}: missing Masszahl ${mass}`);
    }
    drawn.push(entry.masses[0]);
  }

  /* Die drei Wege liefern verschiedene Hauskörper - der Plan zeichnet die Daten,
     nicht ein festes Rechteck. */
  assert.equal(new Set(drawn).size, cases.length, "every route must size the footprint from its own data");
});

test("dossier pdf: the Blattübersicht names every Blatt and the footers number it", () => {
  const { runResult, projectData } = dossierInputs("bad_spec.json");
  const built = buildDossierPdf(projectData, runResult, { datum: "02.10.2026" });
  const text = squeeze(pdfText(built.buffer));
  const plan = built.audit.pages.find((page) => page.width > page.height);
  assert.ok(plan, "the dossier has a quer sheet to file");

  /* Die Blätter sind die Ablageeinheit: der Befund kann über mehrere Seiten
     laufen, der Lageplan ist genau ein Blatt, Unterschriften und Siegel eines. */
  const names = ["Befund", "Lageplan", "Unterschriften und Prüfsiegel"];
  const ranges = [
    { from: 1, to: plan.index - 1 },
    { from: plan.index, to: plan.index },
    { from: plan.index + 1, to: built.pages },
  ];

  assert.ok(text.includes(squeeze("Blattübersicht")), "the register is on the first sheet");
  for (const [index, name] of names.entries()) {
    assert.ok(text.includes(squeeze(`Blatt ${index + 1}/3`)), `missing Blatt ${index + 1}/3`);
    assert.ok(text.includes(squeeze(name)), `missing Blatt name ${name}`);
  }
  for (const range of ranges) {
    const label = range.from === range.to ? `Seite ${range.from}` : `Seiten ${range.from}–${range.to}`;
    assert.ok(text.includes(squeeze(label)), `missing page range ${label} in the register`);
  }
  assert.ok(
    text.includes(squeeze(`3 Blätter, ${built.pages} Seiten`)),
    "the register states the totals of the set"
  );

  /* Jede Seite nennt ihr Blatt: wer ein Blatt aus der Mappe zieht, weiß ohne die
     Übersicht, wohin es gehört. */
  for (let page = 1; page <= built.pages; page += 1) {
    const blatt = page < plan.index ? 0 : page === plan.index ? 1 : 2;
    const label = `Blatt ${blatt + 1}/3 · ${names[blatt]} · Seite ${page} von ${built.pages}`;
    assert.ok(text.includes(squeeze(label)), `page ${page} has no Blatt footer: ${label}`);
  }
});

test("dossier pdf: a Befund over several pages stays one Blatt, and the register follows the run", () => {
  const { runResult, projectData } = dossierInputs("bad_spec.json");
  const many = { ...runResult, results: Array.from({ length: 4 }, () => runResult.results).flat() };
  const built = buildDossierPdf(projectData, many, { datum: "02.10.2026" });
  const text = squeeze(pdfText(built.buffer));
  const plan = built.audit.pages.find((page) => page.width > page.height);

  assert.ok(plan.index > 2, "the stress run needs more than one Befund sheet for this to prove anything");
  assert.equal(built.pages > plan.index, true, "and sheets after the Lageplan for the seal");

  /* Alle Seiten vor dem Lageplan sind Blatt 1, egal wie viele es sind: die
     Bezeichnung zählt, nicht die Seitenzahl. */
  assert.ok(text.includes(squeeze(`Seiten 1–${plan.index - 1}`)), "the Befund range spans its sheets");
  assert.ok(text.includes(squeeze(`Blatt 1/3 · Befund · Seite 1 von ${built.pages}`)), "first sheet");
  assert.ok(text.includes(squeeze(`Seite ${plan.index} von ${built.pages}`)), "the plan keeps its page number");
  assert.ok(
    text.includes(squeeze(`Blatt 3/3 · Unterschriften und Prüfsiegel · Seite ${built.pages} von ${built.pages}`)),
    "the seal sheet closes the set"
  );
  assert.equal(built.audit.overflow, false);
});

test("dossier pdf: the verdict box and the row statuses follow the run", () => {
  const titles = [
    "BESTANDEN (VOLLSTÄNDIG GENEHMIGUNGSFÄHIG)",
    "MIT AUFLAGEN (BEFREIUNGSERFORDERNISSE)",
    "NICHT BESTANDEN (MATERIELLE MÄNGEL)",
  ];

  for (const name of ["good_spec.json", "bad_spec.json"]) {
    const { runResult, projectData } = dossierInputs(name);
    const text = squeeze(pdfText(buildDossierPdf(projectData, runResult, { datum: "02.10.2026" }).buffer));
    const expected = STATUS_TITLE(runResult.counts);

    assert.ok(text.includes(squeeze(expected)), `${name}: expected "${expected}"`);
    for (const other of titles) {
      if (other === expected) continue;
      assert.ok(!text.includes(squeeze(other)), `${name}: must not also claim "${other}"`);
    }

    const labels = new Set(runResult.results.map(ROW_LABEL));
    for (const label of labels) {
      assert.ok(text.includes(label), `${name}: every row status must render, "${label}" is missing`);
    }
  }

  const failing = dossierInputs("bad_spec.json");
  assert.equal(failing.runResult.counts.fail > 0, true, "the failing fixture still fails");
  assert.ok(
    squeeze(pdfText(buildDossierPdf(failing.projectData, failing.runResult, { datum: "02.10.2026" }).buffer)).includes(
      "NICHTBESTANDEN"
    )
  );
});

test("dossier pdf: the same input produces the same bytes, and the date comes from the options", () => {
  const { runResult, projectData } = dossierInputs("bad_spec.json");
  const first = renderDossierPdf(projectData, runResult, { datum: "02.10.2026" });
  const second = renderDossierPdf(projectData, runResult, { datum: "02.10.2026" });

  assert.ok(first.equals(second), "identical arguments, identical file");
  assert.ok(!first.toString("latin1").includes("CreationDate"), "no timestamp may sneak in");
  assert.ok(squeeze(pdfText(first)).includes("02.10.2026"), "the given date is printed");

  const otherDay = renderDossierPdf(projectData, runResult, { datum: "03.10.2026" });
  assert.ok(!first.equals(otherDay), "the date is part of the document");

  /* Without an explicit date the dossier still renders, and the date is German
     formatted. */
  const defaultDate = pdfText(renderDossierPdf(projectData, runResult, {}));
  assert.match(squeeze(defaultDate), /\d{2}\.\d{2}\.\d{4}/, "DD.MM.YYYY even with no date given");

  /* An unheaded project must not make the file different on every run: a random
     Aktenzeichen would break the determinism the seal depends on. */
  const unheaded = { projekt: "Bauvorhaben ohne Aktenzeichen" };
  const a = renderDossierPdf(unheaded, runResult, { datum: "02.10.2026" });
  const b = renderDossierPdf(unheaded, runResult, { datum: "02.10.2026" });
  assert.ok(a.equals(b), "a missing Aktenzeichen must not be invented per run");
  assert.ok(a.toString("latin1").startsWith("%PDF-1.4"));
});

test("dossier pdf: writeDossierPdf writes the file it promises and refuses a missing directory", async () => {
  const dir = mkdtempSync(join(tmpdir(), "baulens-pdf-"));
  try {
    const { runResult, projectData, hash } = dossierInputs("good_spec.json");
    const target = join(dir, "Befund.pdf");

    const written = await settle(writeDossierPdf(target, projectData, runResult, { datum: "02.10.2026" }));
    assert.ok(existsSync(target), "the file is on disk");
    assert.equal(written.path, target);

    const onDisk = readFileSync(target);
    assert.equal(onDisk.length, written.bytes);
    assert.ok(
      onDisk.equals(renderDossierPdf(projectData, runResult, { datum: "02.10.2026" })),
      "the file on disk is exactly what the renderer produces"
    );
    assert.equal(written.hash, hash);
    assert.ok(written.pages >= 1);

    const missingParent = join(dir, "gibt-es-nicht", "Befund.pdf");
    const error = await failure(() => writeDossierPdf(missingParent, projectData, runResult, {}));
    assert.equal(error.code, "missing_directory");
    assert.equal(existsSync(join(dir, "gibt-es-nicht")), false, "the writer must not create the tree");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("dossier pdf: every page carries its footer and stays inside the sheet", () => {
  const { runResult, projectData } = dossierInputs("bad_spec.json");

  /* Enough rows that the table, the plan and the seal cannot share one sheet:
     this is the case where a missing page break would draw over the footer. The
     rows are copies of the real ones -- same ids and citations -- so a renderer
     that resolves a citation by rule id still finds it. */
  const many = {
    ...runResult,
    results: Array.from({ length: 4 }, () => runResult.results).flat(),
  };

  for (const [label, run] of [["single", runResult], ["multi", many]]) {
    const built = buildDossierPdf(projectData, run, { datum: "02.10.2026" });
    const text = squeeze(pdfText(built.buffer));

    /* Das Lageplan-Blatt ist quer und trägt dieselbe Fußzeile; ohne die
       blatteigene Geometrie läge sie bei den Hochformat-Konstanten unterhalb
       des Papiers, und Blatt- wie Seitenzahl wären verschwunden. */
    const quer = built.audit.pages.filter((page) => page.width > page.height);
    assert.equal(quer.length, 1, `${label}: exactly one quer sheet`);
    const plan = quer[0].index;
    const names = ["Befund", "Lageplan", "Unterschriften und Prüfsiegel"];

    for (let page = 1; page <= built.pages; page++) {
      const blatt = page < plan ? 0 : page === plan ? 1 : 2;
      const footer = `Blatt ${blatt + 1}/3 · ${names[blatt]} · Seite ${page} von ${built.pages}`;
      assert.ok(text.includes(squeeze(footer)), `${label}: page ${page} has no Blatt footer, expected "${footer}"`);
    }
    for (const page of built.audit.pages) {
      assert.ok(page.maxTextBottom <= page.height, `${label}: page ${page.index} ends below the sheet`);
      assert.ok(page.maxTextX <= page.width, `${label}: page ${page.index} ends past the right edge`);
      assert.equal(page.overflow, false);
    }
  }

  assert.ok(
    buildDossierPdf(projectData, many, { datum: "02.10.2026" }).pages > 1,
    "the stress run must actually paginate, otherwise this test proves nothing"
  );
});

/* Optional, and skipped when the tool is absent: an independent reader is the
   only real proof that the file opens. poppler is not a dependency of this
   project, so only a genuinely missing binary skips the test -- any other
   failure of the reader is a failure of the file. */
test("dossier pdf: an external reader sees the title and the seal", async (t) => {
  const dir = mkdtempSync(join(tmpdir(), "baulens-pdftotext-"));
  try {
    const { runResult, projectData, hash } = dossierInputs("bad_spec.json");
    const target = join(dir, "Befund.pdf");
    await settle(writeDossierPdf(target, projectData, runResult, { datum: "02.10.2026" }));

    let text;
    try {
      /* -enc UTF-8 is explicit on purpose: both poppler and xpdf default to the
         platform ANSI codepage on Windows, where the umlauts in the heading come
         back as replacement characters and the comparison below fails for a
         reason that has nothing to do with the file. */
      text = execFileSync("pdftotext", ["-enc", "UTF-8", target, "-"], { encoding: "utf8", maxBuffer: 8 * 1024 * 1024 });
    } catch (error) {
      if (error.code === "ENOENT") {
        t.skip("poppler (pdftotext) is not installed; the suite stays dependency-free");
        return;
      }
      throw error;
    }

    assert.ok(squeeze(text).includes(squeeze("Bauaufsichtlicher Vorprüfungsbefund")), "the reader shows the title");
    assert.ok(squeeze(text).includes(hash), "the reader shows the seal hash");
    assert.ok(squeeze(text).includes(squeeze(projectData.projekt)), "the reader shows the project");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("dossier pdf: a geprüfter Abstand is drawn green, an unterschrittener red", () => {
  /* Die Richtung war einmal verdreht, und das ist genau die Art Fehler, die auf
     einem Blatt in einem Einreichungsordner niemand bemerkt: ein Plan mit 4,0 m
     Abstand - also gewahrt - wurde rot gezeichnet, während die Befundtabelle
     "ok" sagte, und bad_spec mit 2,5 m wurde grün. Die Prüfung selbst lautet
     `ist < erforderlich` ist der Mangel, also ist die geprüfte Seite grün. */
  const inkFor = (name) => {
    const { runResult, projectData } = dossierInputs(name);
    const plan = buildPlanSheet(projectData, runResult, { datum: "02.10.2026" });
    return plan.ops.filter((op) => op.layer === "ABSTANDSFLAECHE").map((op) => op.color);
  };

  /* good_spec: 5,0/4,0/4,0/4,0 m gegen den Vergleichswert 3,5 m - alles gewahrt. */
  assert.deepEqual([...new Set(inkFor("good_spec.json"))], ["#047857"], "ein gewahrter Abstand ist grün");

  /* bad_spec: Norden 3,0 m und Süden 2,5 m, beide unter 3,5 m; Osten und Westen
     sind nicht angegeben und fallen auf den Standardwert 4,0 m zurück. */
  const bad = inkFor("bad_spec.json");
  assert.equal(bad.filter((color) => color === "#dc2626").length, 2, "die unterschrittenen Seiten sind rot");
  assert.equal(bad.filter((color) => color === "#047857").length, 2, "die übrigen bleiben grün");

  /* Und dieselbe Farbe trägt die Maßzahl der Seite, sonst widerspräche die
     Beschriftung der Linie darunter. */
  const { runResult, projectData } = dossierInputs("bad_spec.json");
  const plan = buildPlanSheet(projectData, runResult, { datum: "02.10.2026" });
  const masszahlen = plan.ops.filter((op) => op.layer === "MASSZAHL" && (op.text === "3,00" || op.text === "2,50"));
  assert.equal(masszahlen.length, 4, "Norden und Süden stehen in beiden Ketten");
  assert.deepEqual([...new Set(masszahlen.map((op) => op.color))], ["#dc2626"]);
});

test("dossier pdf: the drawing sheet is exported from the recording the PDF page replays", () => {
  /* Der Lageplan verlässt die Mappe auch als eigene Datei (SVG für den Plotter,
     DXF in Metern für CAD). Beide Formate werden aus derselben Aufzeichnung
     geschrieben, die diese Seite abspielt - ein zweiter Zeichenweg wäre eine
     zweite Zeichnung, und die erste Korrektur landete nur in einer. */
  const { runResult, projectData } = dossierInputs("good_spec.json");
  const plan = buildPlanSheet(projectData, runResult, { datum: "02.10.2026" });

  assert.equal(plan.geometry.label, "1:250");
  assert.equal(plan.blatt, 2, "das Blatt ist das zweite der Mappe");
  assert.equal(plan.ops.length > 100, true, "die Aufzeichnung enthält die ganze Zeichnung");

  /* Die Aufzeichnung ist rein: zweimal aufgenommen kommt zweimal dasselbe
     heraus, sonst hinge der Export von der Reihenfolge der Aufrufe ab. */
  const again = buildPlanSheet(projectData, runResult, { datum: "02.10.2026" });
  assert.deepEqual(again.ops, plan.ops);
});

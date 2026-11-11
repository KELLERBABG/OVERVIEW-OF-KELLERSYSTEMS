/**
 * BauLens - the DIN-A4 "Bauaufsichtlicher Vorprüfungsbefund" as a real PDF.
 *
 * The desktop app used to hand this document to the OS print dialog, which
 * produced a printout with a filename nobody could choose and no file at all on
 * a machine without a printer. This module writes the file directly, from the
 * same `projectData`/`runResult` the HTML dossier in src/dossier.js renders.
 *
 * Two promises this file has to keep, both of them load-bearing for a document
 * that names paragraphs of law:
 *
 *   1. **The PDF and the HTML dossier cannot disagree.** Both take the same two
 *      arguments, both use the same fallbacks, and both print the same Prüfziffer
 *      from `computeReportHash` - so the two exports of one check are provably
 *      the same check.
 *   2. **Byte-determinism.** No Math.random, no Date.now(), no locale calls, no
 *      counter suffix: the same input produces the same bytes on every machine.
 *      That is why `aktenzeichen` has a fixed placeholder default instead of the
 *      random one src/dossier.js substitutes (a random identifier is not a fact
 *      the engine knows, and it would make the file - and its hash - differ on
 *      every run).
 *
 * Layout is written top-down in PostScript points and paginated here rather than
 * in src/pdf.js: only the renderer knows which blocks must stay together (a
 * table row, the signature block) and what the continuation header looks like.
 */

import { existsSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { A4, createPdf, resolveTextX, textWidth, wrapText } from "./pdf.js";
import { computeReportHash } from "./dossier.js";
import { DISCLAIMER } from "./report.js";
import { ENGINE_LABEL, VERSION } from "./version.js";
import { planSvg } from "./plansvg.js";
import { planDxf } from "./plandxf.js";

/* --------------------------------------------------------------------------
 * Page geometry. DIN 5008-ish: 20mm side margins, the footer band is reserved
 * out of the bottom margin so nothing can be written over it.
 * ------------------------------------------------------------------------ */

const MARGIN = { left: 56.7, right: 56.7, top: 42.5, bottom: 48 };
const CONTENT_LEFT = MARGIN.left;
const CONTENT_WIDTH = A4.width - MARGIN.left - MARGIN.right; // 481.88
const CONTENT_RIGHT = A4.width - MARGIN.right;
const FOOTER_TOP = A4.height - MARGIN.bottom; // 793.89
/** Content stops here, clear of the footer rule and text. */
const CONTENT_BOTTOM = FOOTER_TOP - 16;

/* --------------------------------------------------------------------------
 * Ink and status palette, matching the HTML dossier's colours so the two
 * exports of one check look like the same document.
 * ------------------------------------------------------------------------ */

const INK = "#0f172a";
const SOFT_INK = "#334155";
const MUTED = "#64748b";
const RULE = "#cbd5e1";
const HAIRLINE = "#e2e8f0";
const PANEL = "#f8fafc";
const HEAD_FILL = "#f1f5f9";
const ACCENT = "#0369a1";
const PASS = { color: "#059669", bg: "#ecfdf5", row: "#047857" };
const LAPSED = { color: "#b45309", bg: "#fffbeb", row: "#b45309" };
const FAIL = { color: "#dc2626", bg: "#fef2f2", row: "#b91c1c" };

/** The setback the HTML dossier compares each side against. */
const COMPARE_SETBACK_M = 3.5;

const TEXT = {
  authority: { font: "helvetica-bold", size: 7.5 },
  title: { font: "helvetica-bold", size: 18 },
  subtitle: { font: "helvetica", size: 9 },
  metaLabel: { font: "helvetica-bold", size: 7 },
  metaValue: { font: "helvetica", size: 9.5 },
  boxLabel: { font: "helvetica-bold", size: 7.5 },
  verdict: { font: "helvetica-bold", size: 14 },
  counts: { font: "courier", size: 9 },
  heading: { font: "helvetica-bold", size: 11 },
  para: { font: "courier", size: 8.5 },
  module: { font: "helvetica-bold", size: 8.5 },
  finding: { font: "helvetica", size: 8.5 },
  status: { font: "helvetica-bold", size: 8 },
  tableHead: { font: "helvetica-bold", size: 7 },
  small: { font: "helvetica", size: 8 },
  tiny: { font: "helvetica", size: 7 },
  seal: { font: "courier", size: 7.5 },
  sealNote: { font: "italic", size: 7.5 },
  footer: { font: "helvetica", size: 7.5 },
  /* Zeichnungsblatt (Lageplan): eigener Blattkopf, Maßzahlen, Legende und
     Schriftfeld. Die Maßzahlen sind bewusst klein - sie stehen zwischen zwei
     Maßlinien, nicht in einem Fließtext. */
  sheetTitle: { font: "helvetica-bold", size: 16 },
  sheetSub: { font: "helvetica", size: 9 },
  mass: { font: "helvetica", size: 7.5 },
  north: { font: "helvetica-bold", size: 11 },
  blockLabel: { font: "helvetica-bold", size: 6.5 },
  blockValue: { font: "helvetica", size: 8 },
};

/* --------------------------------------------------------------------------
 * Table geometry lives in one place, so the header row and the body rows
 * cannot drift apart.
 * ------------------------------------------------------------------------ */

const CELL_PAD_X = 5;
const ROW_PAD_Y = 5;
const ROW_LINE = 8.5 * 1.3;
const MESSAGE_GAP = 2;

/**
 * Column geometry for the findings table, built on demand so the module has no
 * top-level state (scripts/bundle.js wraps every module in a function and
 * promises there are no side effects at import time). The widths must still add
 * up to the content width exactly, or the last column walks past the margin.
 */
function tableColumns() {
  const definitions = [
    { label: "NORM", width: 78, ...TEXT.para },
    { label: "MODUL", width: 132, ...TEXT.module },
    { label: "BEFUND", width: CONTENT_WIDTH - 78 - 132 - 47.88, ...TEXT.finding },
    { label: "STATUS", width: 47.88, ...TEXT.status, align: "right" },
  ];

  let x = CONTENT_LEFT;
  return definitions.map((column) => {
    const placed = { ...column, x };
    x += column.width;
    return placed;
  });
}

const DEFAULT_AKTENZEICHEN = "BV-0000-000000";

/* --------------------------------------------------------------------------
 * Small value helpers. Every one of them has to survive absent data: a caller
 * may hand this a half-filled spec, and a dossier that throws is worse than a
 * dossier that says "nicht angegeben".
 * ------------------------------------------------------------------------ */

const text = (value, fallback = "nicht angegeben") => {
  if (value === undefined || value === null) return fallback;
  const string = String(value).trim();
  return string === "" ? fallback : string;
};

const dateString = (value) => {
  /* The date is printed, never generated from a locale call: a locale-dependent
     string would change the file between machines. `options.datum` is taken
     verbatim, because a caller re-rendering an old report must be able to keep
     its date. */
  if (typeof value === "string" && value.trim() !== "") return value.trim();
  const iso = new Date().toISOString().slice(0, 10); // YYYY-MM-DD
  const [year, month, day] = iso.split("-");
  return `${day}.${month}.${year}`;
};

const meters = (value) => {
  const number = Number(value);
  if (!Number.isFinite(number)) return "n. a.";
  return `${number.toFixed(1).replace(".", ",")} m`;
};

const defaultAuthority = (country) => {
  if (country === "AT") return "Zuständige Baubehörde (Magistrat / Gemeinde)";
  if (country === "CH") return "Zuständiges Bauinspektorat";
  return "Untere Bauaufsichtsbehörde / Bauamt";
};

function statusOf(findings) {
  if (!Array.isArray(findings) || findings.length === 0) return "ok";
  if (findings.some((f) => f && f.status === "fail")) return "fail";
  if (findings.some((f) => f && f.status === "warn")) return "warn";
  return "ok";
}

const ROW_STATUS = {
  fail: { label: "MANGEL", color: FAIL.row },
  warn: { label: "AUFLAGE", color: LAPSED.row },
  ok: { label: "KONFORM", color: PASS.row },
};

/**
 * Normalises both arguments into one display model. `projectData` and
 * `runResult` keep exactly the shape src/dossier.js documents; `options` may
 * override what is *printed* (a caller re-rendering an archived report sets the
 * date and the Aktenzeichen), but it is not part of the Prüfziffer - see
 * `computeReportHash(projectData, verdict)` below and the note in buildDossierPdf.
 */
function buildModel(projectData, runResult, options) {
  const project = projectData && typeof projectData === "object" ? projectData : {};
  const run = runResult && typeof runResult === "object" ? runResult : {};
  const opts = options && typeof options === "object" ? options : {};

  const verdict = text(run.verdict, "nicht bestanden");
  const counts = {
    ok: Number(run.counts?.ok) || 0,
    warn: Number(run.counts?.warn) || 0,
    fail: Number(run.counts?.fail) || 0,
  };
  const profile = run.profile ?? null;
  const municipality = run.municipality ?? null;
  const country = text(profile?.country, "DE");

  const pick = (key) => (opts[key] !== undefined ? opts[key] : project[key]);

  const isPass = counts.fail === 0;
  const status = isPass ? (counts.warn === 0 ? PASS : LAPSED) : FAIL;
  const statusTitle = isPass
    ? counts.warn === 0
      ? "BESTANDEN (VOLLSTÄNDIG GENEHMIGUNGSFÄHIG)"
      : "MIT AUFLAGEN (BEFREIUNGSERFORDERNISSE)"
    : "NICHT BESTANDEN (MATERIELLE MÄNGEL)";

  const setbacks = project.grundstueck?.grenzabstaende ?? {};

  return {
    projekt: text(pick("projekt"), "Bauvorhaben Neubau"),
    aktenzeichen: text(pick("aktenzeichen"), DEFAULT_AKTENZEICHEN),
    flurstueck: text(pick("flurstueck"), "Flur 4, Flurstück 108/2"),
    gemarkung: text(pick("gemarkung"), "Musterhausen"),
    entwurfsverfasser: text(pick("entwurfsverfasser"), "Architekturbüro Dipl.-Ing. Muster"),
    bauherr: text(pick("bauherr"), "Projektentwicklung GmbH & Co. KG"),
    datum: dateString(opts.datum ?? project.datum),
    country,
    authority: text(profile?.authorityTitle, defaultAuthority(country)),
    profileName: text(profile?.name, "DACH"),
    citationBase: text(profile?.citationBase, "Referenz"),
    profileCode: text(profile?.code, "MBO"),
    kommune: municipality
      ? `${text(municipality.name)} (Satzungen aktiv)`
      : country === "CH"
        ? "Gemeinde / BZO"
        : country === "AT"
          ? "Gemeinde / Bebauungsplan"
          : "Standard",
    verdict,
    counts,
    status,
    statusTitle,
    results: Array.isArray(run.results) ? run.results : [],
    setbacks: {
      norden: Number(setbacks.norden ?? 4.0),
      sueden: Number(setbacks.sueden ?? 4.0),
      osten: Number(setbacks.osten ?? 4.0),
      westen: Number(setbacks.westen ?? 4.0),
    },
    /* Die Grundfläche, aus der der Lageplan seinen Hauskörper zeichnet. */
    footprint: footprintEstimate(project),
    /* Computed on the raw argument, exactly as generateDossierHtml does it, so
       the HTML dossier and this PDF carry the same Prüfziffer. */
    hash: computeReportHash(project, verdict),
  };
}

/* --------------------------------------------------------------------------
 * Layout cursor and pagination.
 * ------------------------------------------------------------------------ */

class Layout {
  constructor(pdf) {
    this.pdf = pdf;
    this.pages = [];
    this.cursor = 0;
    this.newPage();
  }

  newPage() {
    const page = this.pdf.addPage();
    this.pages.push(page);
    this.cursor = MARGIN.top;
    return page;
  }

  /**
   * Beginnt ein eigenes Blatt im Querformat. Ein Lageplan ist ein Zeichnungs-
   * blatt und kein Absatz: er bekommt immer eine frische Seite, egal wie viel
   * auf der vorigen noch frei wäre - ein Plan, der halb auf einem Textblatt
   * sitzt, gehört in keinen Einreichungsordner.
   */
  newSheet() {
    const page = this.pdf.addPage({ width: SHEET.width, height: SHEET.height });
    this.pages.push(page);
    this.cursor = MARGIN.top;
    return page;
  }

  /** Breaks the page when `height` does not fit, then returns the cursor. */
  ensure(height) {
    if (this.cursor + height > CONTENT_BOTTOM) this.newPage();
    return this.cursor;
  }

  advance(height) {
    this.cursor += height;
    return this.cursor;
  }
}

/* --------------------------------------------------------------------------
 * Header, meta box and verdict box.
 * ------------------------------------------------------------------------ */

function drawHeader(layout, model) {
  const pdf = layout.pdf;
  pdf.drawText(`${model.country} · ${model.authority}`, CONTENT_LEFT, 42.5, { ...TEXT.authority, color: ACCENT });
  pdf.drawText("Bauaufsichtlicher Vorprüfungsbefund", CONTENT_LEFT, 55, { ...TEXT.title, color: INK });
  pdf.drawText(
    `Planungs- und bauordnungsrechtlicher Kriterienabgleich gem. ${model.citationBase}`,
    CONTENT_LEFT,
    79,
    { ...TEXT.subtitle, color: MUTED }
  );

  /* The right column is drawn with the same rules as the left one, so a long
     Aktenzeichen cannot creep past the margin. */
  pdf.drawText(`AZ: ${model.aktenzeichen}`, CONTENT_RIGHT - 160, 55, {
    ...TEXT.counts,
    color: SOFT_INK,
    align: "right",
    maxWidth: 160,
  });
  pdf.drawText(`Datum: ${model.datum}`, CONTENT_RIGHT - 160, 67, {
    ...TEXT.counts,
    color: SOFT_INK,
    align: "right",
    maxWidth: 160,
  });

  pdf.line(CONTENT_LEFT, 93, CONTENT_RIGHT, 93, { color: INK, width: 1.2 });
  layout.cursor = 105;
}

function metaPairs(model) {
  return [
    { label: "Bauvorhaben / Projekt", value: model.projekt },
    { label: "Lage / Parzelle", value: `${model.flurstueck}, Gem. ${model.gemarkung}` },
    { label: "Rechtsbereich", value: `${model.country} · ${model.profileName} (${model.citationBase})` },
    { label: "Entwurfsverfasser", value: model.entwurfsverfasser },
    { label: "Bauherrschaft", value: model.bauherr },
    { label: "Kommune / Satzungen", value: model.kommune },
  ];
}

function drawMetaBox(layout, model) {
  const pdf = layout.pdf;
  const pairs = metaPairs(model);
  const columnWidth = CONTENT_WIDTH / 2 - 16;
  const columns = [pairs.slice(0, 3), pairs.slice(3, 6)];

  const cellHeight = (pair) =>
    9 + wrapText(pair.value, columnWidth, TEXT.metaValue).length * 12.4;

  const columnHeight = Math.max(
    ...columns.map((column) => column.reduce((sum, pair) => sum + cellHeight(pair), 0))
  );
  const boxHeight = columnHeight + 20;

  layout.ensure(boxHeight + 14);
  const top = layout.cursor;
  pdf.rect(CONTENT_LEFT, top, CONTENT_WIDTH, boxHeight, { fill: PANEL, stroke: HAIRLINE, lineWidth: 0.7 });

  columns.forEach((column, index) => {
    const x = CONTENT_LEFT + 10 + index * (CONTENT_WIDTH / 2);
    let cursor = top + 12;
    for (const pair of column) {
      pdf.drawText(pair.label, x, cursor, { ...TEXT.metaLabel, color: MUTED });
      cursor += 9;
      cursor = pdf.drawParagraph(pair.value, x, cursor, {
        ...TEXT.metaValue,
        color: INK,
        width: columnWidth,
        leading: 1.3,
      });
      cursor += 3;
    }
  });

  layout.cursor = top + boxHeight + 14;
}

function drawVerdictBox(layout, model) {
  const pdf = layout.pdf;
  const height = 46;
  layout.ensure(height + 14);
  const top = layout.cursor;

  pdf.rect(CONTENT_LEFT, top, CONTENT_WIDTH, height, { fill: model.status.bg, stroke: model.status.color, lineWidth: 1.5 });
  /* The counts sit on the top line beside the small label, not beside the
     status title: "BESTANDEN (VOLLSTÄNDIG GENEHMIGUNGSFÄHIG)" at 14pt bold is
     about 355pt wide and would run under a right-hand count block placed on the
     same line. The label is a third of that, so the top line has room. */
  pdf.drawText(
    `${model.counts.ok} OK · ${model.counts.warn} Warnungen · ${model.counts.fail} Fehler`,
    CONTENT_RIGHT - 222,
    top + 9,
    { ...TEXT.counts, color: SOFT_INK, align: "right", maxWidth: 210 }
  );
  pdf.drawText("Gesamtergebnis der automatisierten Vorprüfung", CONTENT_LEFT + 12, top + 9, {
    ...TEXT.boxLabel,
    color: model.status.color,
  });
  pdf.drawText(model.statusTitle, CONTENT_LEFT + 12, top + 22, { ...TEXT.verdict, color: model.status.color });
  layout.cursor = top + height + 16;
}

/* --------------------------------------------------------------------------
 * Blattübersicht (Blattregister).
 *
 * The dossier is filed by Blatt the way a plan set is filed: the Befund is
 * Blatt 1, the Lageplan Blatt 2, signatures and seal Blatt 3. The register names
 * every Blatt and the pages it occupies, so somebody who pulls one sheet out of
 * the folder can put it back. Which pages belong to which Blatt is only known
 * once everything is laid out, so the rows are drawn here and the page ranges
 * are stamped at the end - the same reason the footers are stamped late.
 *
 * The register is PDF-only on purpose: the HTML dossier in src/dossier.js is one
 * continuous print flow without sheets, so numbering Blätter there would invent
 * a structure that document does not have.
 * ------------------------------------------------------------------------ */

/**
 * The Blätter of the dossier, in filing order. The order also defines the Blatt
 * numbers, so a new Blatt is added here and nowhere else.
 */
const BLATT_DEFINITIONS = Object.freeze([
  Object.freeze({
    id: "befund",
    name: "Befund",
    note: "Kopf, Projektdaten, Einordnung und Prüfergebnis je Modul",
  }),
  Object.freeze({
    id: "lageplan",
    name: "Lageplan",
    note: "Abstandsflächen mit Maßketten, Legende und Schriftfeld",
  }),
  Object.freeze({
    id: "siegel",
    name: "Unterschriften und Prüfsiegel",
    note: "Signaturfelder und SHA-256-Prüfziffer über Spec und Befund",
  }),
]);

/** Blattnummer eines Blattes aus seiner Kennung (1-basiert). */
const blattNumber = (id) => BLATT_DEFINITIONS.findIndex((definition) => definition.id === id) + 1;

const OVERVIEW_HEADING = 22;
const OVERVIEW_ROW_LINE = 13;
const OVERVIEW_NOTE_LINE = 11;
const OVERVIEW_ROW_GAP = 5;
const OVERVIEW_FOOTER = 22;

/** Beschreibungen werden umbrochen, damit der Kasten auch bei längerem Text hält. */
function overviewRows() {
  return BLATT_DEFINITIONS.map((definition) => ({
    ...definition,
    noteLines: wrapText(definition.note, CONTENT_WIDTH - 140, TEXT.tiny),
  }));
}

/** Höhe des Kastens: hängt nur an den festen Blattbezeichnungen, nicht am Umfang. */
function measureBlattOverview() {
  const rows = overviewRows().reduce(
    (sum, row) => sum + OVERVIEW_ROW_LINE + row.noteLines.length * OVERVIEW_NOTE_LINE + OVERVIEW_ROW_GAP,
    0
  );
  return OVERVIEW_HEADING + rows + OVERVIEW_FOOTER;
}

/**
 * Draws the register and everything that does not depend on pagination, and
 * returns the coordinates the late pass needs: the sheet object, the top of each
 * row, and the line that carries the totals.
 */
function drawBlattOverview(layout) {
  const pdf = layout.pdf;
  const height = measureBlattOverview();
  const top = layout.ensure(height + 12);

  pdf.rect(CONTENT_LEFT, top, CONTENT_WIDTH, height, { fill: PANEL, stroke: HAIRLINE, lineWidth: 0.7 });
  pdf.drawText("Blattübersicht", CONTENT_LEFT + 10, top + 8, { ...TEXT.boxLabel, color: MUTED });

  const stations = [];
  let cursor = top + OVERVIEW_HEADING;
  for (const [index, row] of overviewRows().entries()) {
    pdf.drawText(`Blatt ${index + 1}/${BLATT_DEFINITIONS.length}`, CONTENT_LEFT + 10, cursor, {
      ...TEXT.metaLabel,
      color: ACCENT,
    });
    pdf.drawText(row.name, CONTENT_LEFT + 70, cursor, { ...TEXT.status, color: INK });
    stations.push({ id: row.id, top: cursor });

    cursor += OVERVIEW_ROW_LINE;
    for (const line of row.noteLines) {
      pdf.drawText(line, CONTENT_LEFT + 70, cursor, { ...TEXT.tiny, color: MUTED });
      cursor += OVERVIEW_NOTE_LINE;
    }
    cursor += OVERVIEW_ROW_GAP;
  }

  layout.cursor = top + height + 12;
  return { page: pdf.page, stations, footerTop: top + height - 16, top, height };
}

/** "Seite 3" bzw. "Seiten 1–2" - die Seiten, auf denen ein Blatt liegt. */
function blattRangeLabel(blatt) {
  return blatt.firstPage === blatt.lastPage ? `Seite ${blatt.firstPage}` : `Seiten ${blatt.firstPage}–${blatt.lastPage}`;
}

/** Stamps the page ranges and the totals into the register box. */
function stampBlattOverview(pdf, overview, sheets, pageCount) {
  withPage(pdf, overview.page, () => {
    /* Looked up by id, not by position: a Blatt without pages must not shift the
       rows of the others. */
    for (const blatt of sheets) {
      const station = overview.stations.find((entry) => entry.id === blatt.id);
      if (!station) continue;
      pdf.drawText(blattRangeLabel(blatt), CONTENT_RIGHT - 70, station.top, {
        ...TEXT.tiny,
        color: SOFT_INK,
        align: "right",
        maxWidth: 60,
      });
    }

    /* `pageCount` is passed in, not read from `pdf.pages`: withPage has swapped
       that for the single sheet being stamped. */
    pdf.drawText(
      `${sheets.length} Blätter, ${pageCount} Seiten · abgelegt wird nach Blatt, gezählt nach Seite.`,
      CONTENT_LEFT + 10,
      overview.footerTop,
      { ...TEXT.tiny, color: MUTED }
    );
  });
}

/**
 * Groups the physical pages into Blätter. A Blatt is what gets filed: the Befund
 * may run over several sheets, the Lageplan is exactly one, signatures and seal
 * are one. That grouping - not the page count - is what the register and the
 * footer number.
 */
function blattPlan(pages, planPage) {
  const plan = pages.indexOf(planPage);
  const boundaries = [
    { from: 0, to: plan - 1, definition: BLATT_DEFINITIONS[blattNumber("befund") - 1] },
    { from: plan, to: plan, definition: BLATT_DEFINITIONS[blattNumber("lageplan") - 1] },
    { from: plan + 1, to: pages.length - 1, definition: BLATT_DEFINITIONS[blattNumber("siegel") - 1] },
  ];

  return boundaries
    .filter((entry) => entry.to >= entry.from && entry.from >= 0)
    .map((entry, index, list) => ({
      id: entry.definition.id,
      name: entry.definition.name,
      from: entry.from,
      to: entry.to,
      index: index + 1,
      total: list.length,
      firstPage: entry.from + 1,
      lastPage: entry.to + 1,
    }));
}

/* --------------------------------------------------------------------------
 * Findings table.
 * ------------------------------------------------------------------------ */

function measureRow(result) {
  const columns = tableColumns();
  const findings = Array.isArray(result.findings) ? result.findings.filter(Boolean) : [];
  const status = ROW_STATUS[statusOf(findings)];

  const para = wrapText(text(result.para, "–"), columns[0].width - 2 * CELL_PAD_X, TEXT.para);
  const module = wrapText(text(result.title, result.id ?? "Modul"), columns[1].width - 2 * CELL_PAD_X, TEXT.module);
  const messages = (findings.length ? findings.map((f) => text(f.message, "Ohne Begründung.")) : ["Keine Beanstandungen."]).map(
    (message) => wrapText(message, columns[2].width - 2 * CELL_PAD_X, TEXT.finding)
  );

  const findingLines = messages.reduce((sum, lines) => sum + lines.length, 0);
  const lines = Math.max(para.length, module.length, findingLines, 1);

  return {
    para,
    module,
    messages,
    status,
    /* Every row is kept whole: a Mangel split across two pages reads as two
       Mängel, which is exactly the kind of misreading this document exists to
       prevent. */
    height: 2 * ROW_PAD_Y + lines * ROW_LINE + Math.max(0, messages.length - 1) * MESSAGE_GAP,
  };
}

function drawTableHeader(layout) {
  const pdf = layout.pdf;
  const top = layout.cursor;
  const height = 16;
  pdf.rect(CONTENT_LEFT, top, CONTENT_WIDTH, height, { fill: HEAD_FILL });
  for (const column of tableColumns()) {
    if (column.align === "right") {
      pdf.drawText(column.label, column.x, top + 5, {
        ...TEXT.tableHead,
        color: MUTED,
        align: "right",
        maxWidth: column.width - CELL_PAD_X,
      });
    } else {
      pdf.drawText(column.label, column.x + CELL_PAD_X, top + 5, { ...TEXT.tableHead, color: MUTED });
    }
  }
  pdf.line(CONTENT_LEFT, top + height, CONTENT_RIGHT, top + height, { color: RULE, width: 0.7 });
  layout.cursor = top + height;
}

function drawRow(layout, row) {
  const pdf = layout.pdf;
  const top = layout.cursor;
  const columns = tableColumns();

  const drawColumn = (index, lines) => {
    const column = columns[index];
    lines.forEach((line, lineIndex) => {
      pdf.drawText(line, column.x + CELL_PAD_X, top + ROW_PAD_Y + lineIndex * ROW_LINE, {
        font: column.font,
        size: column.size,
        color: INK,
      });
    });
  };

  drawColumn(0, row.para);
  drawColumn(1, row.module);

  let cursor = top + ROW_PAD_Y;
  for (const message of row.messages) {
    message.forEach((line, lineIndex) => {
      pdf.drawText(line, columns[2].x + CELL_PAD_X, cursor + lineIndex * ROW_LINE, {
        ...TEXT.finding,
        color: SOFT_INK,
      });
    });
    cursor += message.length * ROW_LINE + MESSAGE_GAP;
  }

  pdf.drawText(row.status.label, columns[3].x, top + ROW_PAD_Y, {
    ...TEXT.status,
    color: row.status.color,
    align: "right",
    maxWidth: columns[3].width - CELL_PAD_X,
  });

  pdf.line(CONTENT_LEFT, top + row.height, CONTENT_RIGHT, top + row.height, { color: HAIRLINE, width: 0.5 });
  layout.cursor = top + row.height;
}

function drawFindingsTable(layout, model) {
  const pdf = layout.pdf;
  const HEADING_HEIGHT = 22;

  /* Heading, header row and roughly two rows must fit before the table starts;
     a header stranded alone at the foot of a page reads as an empty table. */
  layout.ensure(HEADING_HEIGHT + 16 + 30);
  pdf.drawText("Prüfergebnis je Modul", CONTENT_LEFT, layout.cursor, { ...TEXT.heading, color: INK });
  layout.advance(HEADING_HEIGHT);
  drawTableHeader(layout);

  for (const result of model.results) {
    const row = measureRow(result);
    if (layout.cursor + row.height > CONTENT_BOTTOM) {
      layout.newPage();
      pdf.drawText("Prüfergebnis je Modul (Fortsetzung)", CONTENT_LEFT, layout.cursor, {
        ...TEXT.heading,
        color: INK,
      });
      layout.advance(HEADING_HEIGHT);
      drawTableHeader(layout);
    }
    drawRow(layout, row);
  }

  layout.advance(6);
}

/* --------------------------------------------------------------------------
 * Vorher-/Nachher-Vergleich.
 *
 * Eine Heilungssimulation ist eine Entwurfsvariante, keine Bestandsaufnahme.
 * Das Blatt stellt Maße, Abstandsflächen und Mängelzahlen beider Stände neben-
 * einander und nennt die Modulergebnisse, die sich ändern. Der Befund bleibt der
 * des Bestands; die Variante bekommt ihre eigene Prüfziffer, damit das Blatt
 * eindeutig auf genau diese Simulation zeigt. Es steht im Befund, nicht als
 * eigenes Blatt, damit Blattregister und Blattnummern unverändert bleiben.
 * ------------------------------------------------------------------------ */

const COMPARE_HEADING = 22;
const COMPARE_ROW = 16;
const COMPARE_LINE = 11;
/** Spaltenbreiten des Vergleichs; die Änderung bekommt den Rest der Zeile. */
const COMPARE_COLUMNS = Object.freeze({ label: 190, before: 100, after: 100 });

/** Zahl oder `null`: fehlende Angaben werden nicht als 0 gelesen. */
const numberOrNull = (value) => {
  if (value === null || value === undefined || value === "") return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
};

const formatNumber = (value, digits, unit = "") => {
  if (value === null) return "n. a.";
  return `${value.toFixed(digits).replace(".", ",")}${unit ? ` ${unit}` : ""}`;
};

/**
 * Die Größen, die eine Heilungsvariante verändern kann. Gelesen wird aus dem
 * Stand (Modell und Spec), nicht aus der Variante selbst - so bleibt jede Zeile
 * dieselbe Frage, egal auf welcher Seite sie gestellt wird.
 */
const COMPARISON_ROWS = Object.freeze([
  { label: "Abstand Norden", read: (s) => s.model.setbacks.norden, digits: 2, unit: "m" },
  { label: "Abstand Osten", read: (s) => s.model.setbacks.osten, digits: 2, unit: "m" },
  { label: "Abstand Süden", read: (s) => s.model.setbacks.sueden, digits: 2, unit: "m" },
  { label: "Abstand Westen", read: (s) => s.model.setbacks.westen, digits: 2, unit: "m" },
  { label: "Gebäudehöhe", read: (s) => s.project.gebaeude?.hoehe_m, digits: 2, unit: "m" },
  { label: "GRZ (Ist)", read: (s) => s.project.gebaeude?.grz_ist, digits: 2, unit: "" },
  { label: "GFZ (Ist)", read: (s) => s.project.gebaeude?.gfz_ist, digits: 2, unit: "" },
  { label: "Mängel", read: (s) => s.model.counts.fail, digits: 0, unit: "" },
  { label: "Auflagen", read: (s) => s.model.counts.warn, digits: 0, unit: "" },
]);

/** Modulergebnisse, deren Status sich zwischen Bestand und Variante ändert. */
function moduleChanges(before, after) {
  const afterById = new Map(after.model.results.map((result) => [result.id, result]));
  const changes = [];
  for (const result of before.model.results) {
    const next = afterById.get(result.id);
    if (!next) continue;
    const from = statusOf(result.findings);
    const to = statusOf(next.findings);
    if (from === to) continue;
    changes.push(`${text(result.title, result.id)}: ${ROW_STATUS[from].label} wird ${ROW_STATUS[to].label}`);
  }
  return changes;
}

function drawComparison(layout, model, projectData, comparison) {
  const pdf = layout.pdf;
  const before = { model, project: projectData ?? {} };
  const after = { model: buildModel(comparison.projectData, comparison.runResult, {}), project: comparison.projectData ?? {} };

  const label = text(comparison.label, "Heilungsvariante");
  const intro = wrapText(
    `Die Variante „${label}“ ist eine Entwurfsvariante aus der Heilungssimulation, keine Bestandsaufnahme. ` +
      "Der Befund oben gilt unverändert für den Bestand.",
    CONTENT_WIDTH,
    TEXT.small
  );
  const changes = moduleChanges(before, after);
  const changeLines = (changes.length ? changes : ["Kein Modulergebnis ändert sich."]).flatMap((line) =>
    wrapText(line, CONTENT_WIDTH - 2 * CELL_PAD_X, TEXT.finding)
  );
  const note = wrapText(`Prüfziffer der Variante: ${after.model.hash}`, CONTENT_WIDTH, TEXT.para);

  const height =
    COMPARE_HEADING +
    intro.length * COMPARE_LINE +
    8 +
    16 +
    COMPARISON_ROWS.length * COMPARE_ROW +
    14 +
    16 +
    changeLines.length * COMPARE_LINE +
    8 +
    note.length * COMPARE_LINE +
    12;
  layout.ensure(height);

  pdf.drawText("Vorher-/Nachher-Vergleich", CONTENT_LEFT, layout.cursor, { ...TEXT.heading, color: INK });
  layout.advance(COMPARE_HEADING);
  for (const line of intro) {
    pdf.drawText(line, CONTENT_LEFT, layout.cursor, { ...TEXT.small, color: MUTED });
    layout.advance(COMPARE_LINE);
  }
  layout.advance(8);

  /* Kopfzeile wie in der Befundtabelle: gleiche Fläche, gleiche Linie. */
  const headTop = layout.cursor;
  pdf.rect(CONTENT_LEFT, headTop, CONTENT_WIDTH, 16, { fill: HEAD_FILL });
  pdf.drawText("Größe", CONTENT_LEFT + CELL_PAD_X, headTop + 5, { ...TEXT.tableHead, color: MUTED });
  pdf.drawText("Bestand", CONTENT_LEFT + COMPARE_COLUMNS.label, headTop + 5, {
    ...TEXT.tableHead,
    color: MUTED,
    align: "right",
    maxWidth: COMPARE_COLUMNS.before - CELL_PAD_X,
  });
  pdf.drawText("Variante", CONTENT_LEFT + COMPARE_COLUMNS.label + COMPARE_COLUMNS.before, headTop + 5, {
    ...TEXT.tableHead,
    color: MUTED,
    align: "right",
    maxWidth: COMPARE_COLUMNS.after - CELL_PAD_X,
  });
  pdf.drawText("Änderung", CONTENT_LEFT + COMPARE_COLUMNS.label + COMPARE_COLUMNS.before + COMPARE_COLUMNS.after + CELL_PAD_X, headTop + 5, {
    ...TEXT.tableHead,
    color: MUTED,
  });
  pdf.line(CONTENT_LEFT, headTop + 16, CONTENT_RIGHT, headTop + 16, { color: RULE, width: 0.7 });
  layout.advance(16);

  const changeX = CONTENT_LEFT + COMPARE_COLUMNS.label + COMPARE_COLUMNS.before + COMPARE_COLUMNS.after + CELL_PAD_X;
  for (const row of COMPARISON_ROWS) {
    const top = layout.cursor;
    const from = numberOrNull(row.read(before));
    const to = numberOrNull(row.read(after));
    const delta = from === null || to === null ? null : to - from;
    const changed = delta !== null && Math.abs(delta) > 1e-9;
    const change = delta === null ? "–" : changed ? `${delta > 0 ? "+" : "-"}${formatNumber(Math.abs(delta), row.digits, row.unit)}` : "unverändert";

    pdf.drawText(row.label, CONTENT_LEFT + CELL_PAD_X, top + 5, { ...TEXT.finding, color: INK });
    pdf.drawText(formatNumber(from, row.digits, row.unit), CONTENT_LEFT + COMPARE_COLUMNS.label, top + 5, {
      ...TEXT.finding,
      color: SOFT_INK,
      align: "right",
      maxWidth: COMPARE_COLUMNS.before - CELL_PAD_X,
    });
    pdf.drawText(formatNumber(to, row.digits, row.unit), CONTENT_LEFT + COMPARE_COLUMNS.label + COMPARE_COLUMNS.before, top + 5, {
      ...TEXT.finding,
      color: SOFT_INK,
      align: "right",
      maxWidth: COMPARE_COLUMNS.after - CELL_PAD_X,
    });
    pdf.drawText(change, changeX, top + 5, { ...TEXT.status, color: changed ? ACCENT : MUTED });
    pdf.line(CONTENT_LEFT, top + COMPARE_ROW, CONTENT_RIGHT, top + COMPARE_ROW, { color: HAIRLINE, width: 0.5 });
    layout.advance(COMPARE_ROW);
  }
  layout.advance(14);

  pdf.drawText("Geänderte Modulergebnisse", CONTENT_LEFT, layout.cursor, { ...TEXT.metaLabel, color: MUTED });
  layout.advance(10);
  for (const line of changeLines) {
    pdf.drawText(line, CONTENT_LEFT + CELL_PAD_X, layout.cursor, { ...TEXT.finding, color: SOFT_INK });
    layout.advance(COMPARE_LINE);
  }
  layout.advance(8);

  for (const line of note) {
    pdf.drawText(line, CONTENT_LEFT, layout.cursor, { ...TEXT.para, color: MUTED });
    layout.advance(COMPARE_LINE);
  }
  layout.advance(12);
}

/* --------------------------------------------------------------------------
 * Site plan as a drawing sheet.
 *
 * A Lageplan in a submission folder is a Blatt of its own, so it leaves the
 * portrait flow: a DIN-A4 landscape sheet with a frame, Maßketten in metres, a
 * Nordpfeil, a legend and a Schriftfeld. There is no image support in the writer
 * and no rasteriser in the project, so the drawing is vector geometry from lines
 * and rectangles - sharper than a raster image would have been anyway.
 *
 * The Hauskörper comes out of the planning data whenever the spec carries any:
 * `grz_ist × Grundstücksfläche` is the Grundfläche of this building by
 * definition, and `Nutzfläche / Geschosse` is its average floor plate when that
 * is all there is. The parcel is then the footprint plus the four checked
 * setbacks, so the built-over share the sheet draws is the one the finding
 * checked. Only without usable data does the plan fall back to a Bezugsrechteck
 * with invented dimensions - and then the sheet says so instead of dressing it
 * up. A drawing in a submission folder must not look more authoritative than
 * the data behind it.
 * ------------------------------------------------------------------------ */

/** Das Zeichnungsblatt selbst: DIN A4 quer, 297 mm breit und 210 mm hoch. */
const SHEET = Object.freeze({ width: A4.height, height: A4.width });
/** Blattrand des Zeichnungsblatts (Rahmen mit Schriftfeld, DIN 6771-nah). */
const SHEET_FRAME = 28;
/** Abstand zwischen Rahmen und Blattinhalt. */
const SHEET_PAD = 16;
/** Das Blatt ist quer; der Rahmen endet oberhalb der Fußzeile jeder Seite. */
const SHEET_BOTTOM = SHEET.height - MARGIN.bottom - 12;
const SHEET_INNER = Object.freeze({
  left: SHEET_FRAME + SHEET_PAD,
  right: SHEET.width - SHEET_FRAME - SHEET_PAD,
  top: SHEET_FRAME + SHEET_PAD,
  bottom: SHEET_BOTTOM - SHEET_PAD,
});
/** Rechte Spalte des Blattes: oben die Legende, unten das Schriftfeld. */
const SHEET_COLUMN = 250;
/** Platz zwischen Zeichnung und Blattrand für Maßlinien, Hilfslinien und Maßzahlen. */
const CHAIN_MARGIN = 64;
const SHEET_DRAW = Object.freeze({
  left: SHEET_INNER.left,
  right: SHEET_INNER.right - SHEET_COLUMN - 14,
  top: SHEET_INNER.top + 64,
  bottom: SHEET_INNER.bottom,
});

/** Maßstäbe nach DIN 1356-1, aufsteigend; der erste passende wird verwendet. */
const PLAN_SCALES = [100, 200, 250, 500, 1000];
/** Ein Millimeter Papier sind 2,83465 Punkt, 1 m im Maßstab 1:d also 2834,65/d. */
const ptPerMetre = (denominator) => 2834.6456692913385 / denominator;

/** Seitenverhältnis des Hauskörpers, wenn die Planung keine eigenen Maße nennt. */
const FOOTPRINT_ASPECT = 4 / 3;
/**
 * Mindestgröße für einen abgeleiteten Hauskörper. Darunter zeichnet das Blatt
 * das Bezugsrechteck: eine Grundfläche von 3 m² ergäbe ein Rechteck, in dem die
 * Beschriftung die Maßketten überdeckt, und das wäre keine Zeichnung mehr.
 */
const MIN_FOOTPRINT_AREA_M2 = 12;
/** Bezugsrechteck ohne jede Grundflächenangabe: so klein wie die Abstände erlauben. */
const PARCEL_MIN_WIDTH_M = 20;
const PARCEL_MIN_DEPTH_M = 15;
const FOOTPRINT_MIN_WIDTH_M = 8;
const FOOTPRINT_MIN_DEPTH_M = 6;

/** Eine Abstandsangabe als Zahl ≥ 0 in Metern; alles Unbrauchbare wird 0. */
const metres = (value) => {
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? number : 0;
};

/** Maßzahl: zwei Nachkommastellen, deutsches Komma, ohne Einheit. */
const masszahl = (value) => Number(value).toFixed(2).replace(".", ",");

/** Flächenangabe ohne unnötige Nachkommastelle, deutsches Komma. */
const areal = (value) => (Number.isInteger(value) ? String(value) : value.toFixed(1).replace(".", ","));

/**
 * Die Grundfläche, die das Blatt zeichnet - aus den Angaben der Planung, wenn
 * es welche gibt.
 *
 * Reihenfolge und warum: `grz_ist` ist per Definition der überbaute Anteil des
 * Grundstücks, also ist `grz_ist × Grundstücksfläche` die Grundfläche dieses
 * Bauvorhabens - und sie bleibt mit dem Grundstück stimmig, das dasselbe Blatt
 * daneben zeichnet. Ein allein aus der Nutzfläche abgeleiteter Hauskörper würde
 * eine GRZ zeigen, die der Befund gerade anders geprüft hat. Erst wenn dieser
 * Weg nicht geht, steht der mittlere Geschossgrundriss (`Nutzfläche /
 * Geschosse`) ein. Ohne Geschossangabe gilt die Nutzfläche als Grundfläche, denn
 * genau so trägt der DXF-Import die gemessene überbaute Fläche ein.
 *
 * Gibt es nichts Verwertbares, bleibt `areaM2` null und das Blatt zeichnet das
 * Bezugsrechteck - sichtbar als solches beschriftet.
 */
function footprintEstimate(project) {
  const building = project.gebaeude && typeof project.gebaeude === "object" ? project.gebaeude : {};
  const parcelArea = Number(project.grundstueck?.flaeche_m2);
  const coverage = Number(building.grz_ist);
  const usable = Number(building.nutzflaeche_m2);
  const storeys = Number(building.geschosse_ueber_grund);

  if (coverage > 0 && coverage <= 1 && parcelArea > 0) {
    return {
      areaM2: coverage * parcelArea,
      derivation: `GRZ ${masszahl(coverage)} × ${areal(parcelArea)} m²`,
    };
  }

  if (usable > 0) {
    const floors = Number.isFinite(storeys) && storeys >= 1 ? Math.floor(storeys) : 1;
    return {
      areaM2: usable / floors,
      /* `areal` steht schon vor der Zeile: bei einem Geschoss wäre "Nutzfläche
         120 m²" neben "Grundfläche 120 m²" nur eine Wiederholung - hier zählt,
         dass die Geschossangabe fehlt und die Zahl deshalb die Grundfläche ist. */
      derivation:
        floors > 1 ? `Nutzfläche ${areal(usable)} m² / ${floors} Geschosse` : "Nutzfläche, Geschosszahl nicht angegeben",
    };
  }

  return { areaM2: null, derivation: null };
}

/**
 * Geometrie des Blattes: Bezugsrechteck in Metern, Maßstab und die Rechtecke in
 * Punkten. Der Maßstab wird aus dem verfügbaren Platz gewählt, nicht geraten -
 * die Maßketten sollen lesbar bleiben und der Maßstabsbalken ehrlich sein.
 */
function planGeometry(model) {
  const setbacks = {
    norden: metres(model.setbacks.norden),
    sueden: metres(model.setbacks.sueden),
    osten: metres(model.setbacks.osten),
    westen: metres(model.setbacks.westen),
  };
  const footprint = planFootprint(model, setbacks);
  const widthM = footprint.parcelWidthM;
  const depthM = footprint.parcelDepthM;

  const availableWidth = SHEET_DRAW.right - SHEET_DRAW.left - 2 * CHAIN_MARGIN;
  const availableDepth = SHEET_DRAW.bottom - SHEET_DRAW.top - 2 * CHAIN_MARGIN;
  const denominator =
    PLAN_SCALES.find(
      (step) => widthM * ptPerMetre(step) <= availableWidth && depthM * ptPerMetre(step) <= availableDepth
    ) ?? PLAN_SCALES[PLAN_SCALES.length - 1];

  const unit = ptPerMetre(denominator);
  const width = widthM * unit;
  const depth = depthM * unit;
  const centreX = (SHEET_DRAW.left + SHEET_DRAW.right) / 2;
  const centreY = (SHEET_DRAW.top + SHEET_DRAW.bottom) / 2;

  return {
    denominator,
    label: `1:${denominator}`,
    unit,
    widthM,
    depthM,
    footprintWidthM: footprint.widthM,
    footprintDepthM: footprint.depthM,
    left: centreX - width / 2,
    top: centreY - depth / 2,
    right: centreX + width / 2,
    bottom: centreY + depth / 2,
    footprint,
    setbacks,
  };
}

/**
 * Hauskörper und Grundstücksrechteck in Metern. Der abgeleitete Weg liefert die
 * Grundfläche aus der Planung, der schematische das Bezugsrechteck - beide
 * Angaben sind hier zu finden, damit Legende und Maßketten dieselben Zahlen
 * benutzen, die gezeichnet werden.
 */
function planFootprint(model, setbacks) {
  const estimate = model.footprint;

  if (estimate.areaM2 >= MIN_FOOTPRINT_AREA_M2) {
    const widthM = Math.sqrt(estimate.areaM2 * FOOTPRINT_ASPECT);
    const depthM = Math.sqrt(estimate.areaM2 / FOOTPRINT_ASPECT);
    return {
      derived: true,
      areaM2: estimate.areaM2,
      derivation: estimate.derivation,
      widthM,
      depthM,
      parcelWidthM: widthM + setbacks.westen + setbacks.osten,
      parcelDepthM: depthM + setbacks.norden + setbacks.sueden,
    };
  }

  const parcelWidthM = Math.max(PARCEL_MIN_WIDTH_M, setbacks.westen + setbacks.osten + FOOTPRINT_MIN_WIDTH_M);
  const parcelDepthM = Math.max(PARCEL_MIN_DEPTH_M, setbacks.norden + setbacks.sueden + FOOTPRINT_MIN_DEPTH_M);
  return {
    derived: false,
    areaM2: null,
    derivation: null,
    widthM: parcelWidthM - setbacks.westen - setbacks.osten,
    depthM: parcelDepthM - setbacks.norden - setbacks.sueden,
    parcelWidthM,
    parcelDepthM,
  };
}

/**
 * One station of a Maßkette: the extension line from the object, and the
 * Normschrägstrich at 45 degrees that replaces the arrowhead in German
 * architectural dimensioning.
 */
function drawChainStation(pdf, axis, station, line, extend, tick, layer) {
  const direction = Math.sign(line - extend) || 1;
  const beyond = line + direction * tick;
  if (axis === "h") {
    pdf.line(station, extend, station, beyond, { color: RULE, width: 0.4, layer });
    pdf.line(station - tick, line + direction * tick, station + tick, line - direction * tick, {
      color: SOFT_INK,
      width: 0.9,
      layer,
    });
  } else {
    pdf.line(extend, station, beyond, station, { color: RULE, width: 0.4, layer });
    pdf.line(line - tick, station - tick, line + tick, station + tick, { color: SOFT_INK, width: 0.9, layer });
  }
}

/** Maßkette waagerecht: Maßlinie, Schrägstriche und die Maßzahlen über der Linie. */
function drawHorizontalChain(pdf, { stations, labels, colors, line, extend, tick = 3.4, layer = "MASZKETTE" }) {
  const direction = Math.sign(line - extend) || -1;
  pdf.line(stations[0], line, stations[stations.length - 1], line, { color: SOFT_INK, width: 0.5, layer });
  for (const station of stations) drawChainStation(pdf, "h", station, line, extend, tick, layer);

  labels.forEach((label, index) => {
    if (!label) return;
    const top = direction < 0 ? line - 4 - TEXT.mass.size : line + 4;
    pdf.drawText(label, stations[index], top, {
      ...TEXT.mass,
      color: colors[index] ?? MUTED,
      align: "center",
      maxWidth: stations[index + 1] - stations[index],
      layer: "MASSZAHL",
    });
  });
}

/**
 * Maßkette senkrecht. Die Maßzahlen stehen neben der Linie statt über ihr: der
 * Schreiber kann Text nicht drehen, und eine gedrehte Zahl wäre das Einzige, was
 * eine echte Zeichnung hier anders macht.
 */
function drawVerticalChain(pdf, { stations, labels, colors, line, extend, tick = 3.4, layer = "MASZKETTE" }) {
  const direction = Math.sign(line - extend) || -1;
  pdf.line(line, stations[0], line, stations[stations.length - 1], { color: SOFT_INK, width: 0.5, layer });
  for (const station of stations) drawChainStation(pdf, "v", station, line, extend, tick, layer);

  labels.forEach((label, index) => {
    if (!label) return;
    const centre = (stations[index] + stations[index + 1]) / 2;
    const top = centre - TEXT.mass.size / 2;
    if (direction < 0) {
      /* maxWidth 0 right-aligns the number on the line without a box around it. */
      pdf.drawText(label, line - 4, top, {
        ...TEXT.mass,
        color: colors[index] ?? MUTED,
        align: "right",
        maxWidth: 0,
        layer: "MASSZAHL",
      });
    } else {
      pdf.drawText(label, line + 4, top, { ...TEXT.mass, color: colors[index] ?? MUTED, layer: "MASSZAHL" });
    }
  });
}

/**
 * Nordpfeil: gefüllte Nordhälfte, offene Südhälfte, N darüber. Die Nordhälfte
 * wird zeilenweise gefüllt, weil der Schreiber nur Linien und Rechtecke kennt -
 * ein hohler Pfeil wäre als Nordpfeil nicht zu lesen.
 */
function drawNorthArrow(pdf, { x, tip, height = 44 }) {
  const halfWidth = 6.5;
  const base = tip + height;

  for (let y = 0; y <= height; y += 1) {
    const spread = (halfWidth * y) / height;
    pdf.line(x - spread, tip + y, x, tip + y, { color: INK, width: 1.05, layer: "NORDEN" });
  }
  pdf.line(x, tip, x + halfWidth, base, { color: INK, width: 0.9, layer: "NORDEN" });
  pdf.line(x + halfWidth, base, x - halfWidth, base, { color: INK, width: 0.9, layer: "NORDEN" });

  pdf.drawText("N", x - 12, tip - 13, {
    ...TEXT.north,
    color: INK,
    align: "center",
    maxWidth: 24,
    layer: "NORDEN",
  });
}

/**
 * Legende des Blattes: Linienarten, Farben, Herkunft der Grundfläche, Maßstab.
 * Die Zeile zur Grundfläche ist der Beleg für die Maßzahlen am Hauskörper: sie
 * nennt die Rechnung, aus der die gezeichneten Meter stammen.
 */
function planLegendRows(geometry) {
  return [
    { sample: "thin", color: ACCENT, text: "Grundstücksgrenze / Bezugsgrenze" },
    { sample: "dashed", color: SOFT_INK, text: "Grenzabstand laut Prüfung" },
    { sample: "filled", text: "Hauskörper" },
    { sample: "solid", color: PASS.row, text: "Abstand gewahrt" },
    { sample: "solid", color: FAIL.color, text: "Abstand unterschritten" },
    {
      sample: "none",
      text: geometry.footprint.derived
        ? `Grundfläche ${areal(geometry.footprint.areaM2)} m² · ${geometry.footprint.derivation}`
        : "Grundfläche schematisch (keine Angabe in der Planung)",
    },
    { sample: "solid", color: MUTED, text: `Vergleichswert ${meters(COMPARE_SETBACK_M)} (§6 MBO / LBO)` },
  ];
}

/**
 * Der Hinweis sagt, was am Blatt gerechnet und was angenommen ist. Ein Plan, der
 * eine abgeleitete Grundfläche zeigt, nennt die Annahme (Seitenverhältnis, Lage)
 * und nicht nur "schematisch" - nur so ist die Zahl nachprüfbar.
 */
function planLegendNote(geometry) {
  if (!geometry.footprint.derived) {
    return "Schematischer Auszug; verbindlich sind die Zahlenwerte der Befundtabelle. Kein amtlicher Lageplan.";
  }
  return (
    "Maße in Metern. Der Hauskörper folgt aus der angegebenen Grundfläche; Seitenverhältnis 4:3 und Lage im " +
    "Grundstück sind angenommen. Verbindlich sind die Zahlenwerte der Befundtabelle; kein amtlicher Lageplan."
  );
}

const LEGEND_ROW_HEIGHT = 13;
/** Balkenhöhe plus Beschriftung, so wie drawScaleBar sie zurückgibt. */
const SCALE_BAR_HEIGHT = 18;

/** Kastenmaße der Legende aus ihrem Inhalt, damit nichts über den Rand läuft. */
function legendLayout(geometry) {
  const rows = planLegendRows(geometry);
  const note = planLegendNote(geometry);
  const noteLines = wrapText(note, SHEET_COLUMN - 20, TEXT.tiny).length;
  const height =
    10 + 14 + rows.length * LEGEND_ROW_HEIGHT + 5 + 13 + SCALE_BAR_HEIGHT + 6 + noteLines * TEXT.tiny.size * 1.4 + 10;
  return { rows, note, height };
}

/**
 * Maßstabsbalken: alternierende Meterblöcke im Blattmaßstab, mit Zahlen an den
 * Hauptteilungen. Er ist der Beleg auf dem Blatt, dass die Zeichnung wirklich in
 * diesem Maßstab steht - ein reiner Textvermerk wäre nach dem Kopieren wertlos.
 */
function drawScaleBar(pdf, x, top, geometry) {
  const block = geometry.unit;
  const metresWide = Math.min(50, Math.max(2, Math.round(80 / block)));
  const height = 6;
  const labelStep = Math.max(1, Math.ceil(16 / block));

  for (let index = 0; index < metresWide; index += 1) {
    const segmentX = x + index * block;
    if (index % 2 === 0) pdf.rect(segmentX, top, block, height, { fill: INK, layer: "MASSSTAB" });
    pdf.rect(segmentX, top, block, height, { stroke: INK, lineWidth: 0.4, layer: "MASSSTAB" });
    if (index % labelStep === 0) {
      pdf.drawText(String(index), segmentX, top + height + 2, {
        ...TEXT.tiny,
        color: MUTED,
        align: "center",
        maxWidth: block,
        layer: "MASSSTAB",
      });
    }
  }

  const endX = x + metresWide * block;
  pdf.drawText(String(metresWide), endX, top + height + 2, {
    ...TEXT.tiny,
    color: MUTED,
    align: "center",
    maxWidth: block,
    layer: "MASSSTAB",
  });
  pdf.drawText("m", endX + 4, top, { ...TEXT.tiny, color: MUTED, layer: "MASSSTAB" });

  return top + height + 12;
}

function drawPlanLegend(pdf, geometry, layout) {
  const x = SHEET_INNER.right - SHEET_COLUMN;
  const top = SHEET_INNER.top + 4;
  pdf.rect(x, top, SHEET_COLUMN, layout.height, { stroke: RULE, lineWidth: 0.7, layer: "LEGENDE" });

  let cursor = top + 10;
  pdf.drawText("Legende", x + 10, cursor, { ...TEXT.boxLabel, color: MUTED, layer: "LEGENDE" });
  cursor += 14;

  for (const row of layout.rows) {
    const mid = cursor + TEXT.tiny.size / 2;
    if (row.sample === "filled") {
      pdf.rect(x + 10, cursor, 22, 7, { fill: "#e2e8f0", stroke: SOFT_INK, lineWidth: 0.6, layer: "LEGENDE" });
    } else if (row.sample === "dashed") {
      pdf.line(x + 10, mid, x + 32, mid, { color: row.color, width: 0.9, dash: [3, 3], layer: "LEGENDE" });
    } else if (row.sample === "solid" || row.sample === "thin") {
      pdf.line(x + 10, mid, x + 32, mid, {
        color: row.color,
        width: row.sample === "thin" ? 1.3 : 0.9,
        layer: "LEGENDE",
      });
    }
    pdf.drawText(row.text, x + 40, cursor, {
      ...TEXT.tiny,
      color: SOFT_INK,
      maxWidth: SHEET_COLUMN - 50,
      layer: "LEGENDE",
    });
    cursor += LEGEND_ROW_HEIGHT;
  }

  cursor += 5;
  pdf.drawText(`Maßstab ${geometry.label}`, x + 10, cursor, { ...TEXT.blockLabel, color: INK, layer: "LEGENDE" });
  cursor = drawScaleBar(pdf, x + 10, cursor + 13, geometry) + 6;

  pdf.drawParagraph(layout.note, x + 10, cursor, {
    ...TEXT.tiny,
    color: MUTED,
    width: SHEET_COLUMN - 20,
    leading: 1.4,
    layer: "LEGENDE",
  });
}

/** Schriftfeld: die Angaben, mit denen das Blatt abgelegt und zugeordnet wird. */
function titleBlockRows(model, geometry, blatt) {
  return [
    { label: "Bauvorhaben", value: model.projekt },
    { label: "Aktenzeichen", value: model.aktenzeichen },
    { label: "Grundstück", value: `${model.flurstueck}, Gem. ${model.gemarkung}` },
    { label: "Blattbezeichnung", value: `Blatt ${blatt} – Lageplan (Abstandsflächen)` },
    { label: "Maßstab", value: `${geometry.label} · Maße in Metern` },
    { label: "Stand", value: `${model.datum} · BauLens ${VERSION}` },
  ];
}

/**
 * Measured before it is drawn: a long Bauvorhaben name wraps, and the box has to
 * grow with it instead of pushing the text through its own bottom edge. Only the
 * height is dynamic - the box stays anchored to the sheet corner.
 */
function measureTitleBlock(model, geometry, blatt) {
  const rows = titleBlockRows(model, geometry, blatt).map((row) => ({
    ...row,
    lines: wrapText(row.value, SHEET_COLUMN - 20, TEXT.blockValue),
  }));
  const height = 12 + rows.reduce((sum, row) => sum + 9 + row.lines.length * TEXT.blockValue.size * 1.35 + 4, 0);
  return { rows, height };
}

function drawTitleBlock(pdf, measured) {
  const x = SHEET_INNER.right - SHEET_COLUMN;
  const top = SHEET_INNER.bottom - measured.height;
  pdf.rect(x, top, SHEET_COLUMN, measured.height, { stroke: INK, lineWidth: 1, layer: "SCHRIFTFELD" });
  pdf.drawText("Schriftfeld", x + 10, top + 6, { ...TEXT.boxLabel, color: MUTED, layer: "SCHRIFTFELD" });

  let cursor = top + 20;
  for (const row of measured.rows) {
    pdf.drawText(row.label, x + 10, cursor, { ...TEXT.blockLabel, color: MUTED, layer: "SCHRIFTFELD" });
    cursor = pdf.drawParagraph(row.value, x + 10, cursor + 9, {
      ...TEXT.blockValue,
      color: INK,
      width: SHEET_COLUMN - 20,
      leading: 1.35,
      layer: "SCHRIFTFELD",
    });
    cursor += 4;
  }
}

/** Die Zeichnung selbst: Bezugsrechteck, Abstandsflächen, Hauskörper, Maßketten. */
function drawPlanFigure(pdf, geometry) {
  const { left, top, right, bottom, unit, setbacks } = geometry;
  const footprint = {
    left: left + setbacks.westen * unit,
    top: top + setbacks.norden * unit,
    right: right - setbacks.osten * unit,
    bottom: bottom - setbacks.sueden * unit,
  };

  /* Grün, solange der Abstand den Vergleichswert erreicht; rot, wenn er ihn
     unterschreitet. Die Richtung ist die der Prüfung (§6 Abs. 2 MBO: `ist <
     erforderlich` ist der Mangel) - und sie war hier einmal verdreht: ein Blatt,
     das geprüfte Abstände rot zeichnet, während der Befund "ok" sagt, ist in
     einem Einreichungsordner schlimmer als gar keine Farbe. Ein Abstand genau
     auf dem Vergleichswert ist gewahrt, so wie die Regel `>=` prüft. */
  const setbackInk = (value) => (value >= COMPARE_SETBACK_M ? PASS.row : FAIL.color);
  const dashed = { width: 0.9, dash: [3.2, 2.6] };

  pdf.rect(left, top, right - left, bottom - top, { stroke: ACCENT, lineWidth: 1.3, layer: "GRENZE" });

  pdf.line(left, footprint.top, right, footprint.top, {
    ...dashed,
    color: setbackInk(setbacks.norden),
    layer: "ABSTANDSFLAECHE",
  });
  pdf.line(left, footprint.bottom, right, footprint.bottom, {
    ...dashed,
    color: setbackInk(setbacks.sueden),
    layer: "ABSTANDSFLAECHE",
  });
  pdf.line(footprint.left, top, footprint.left, bottom, {
    ...dashed,
    color: setbackInk(setbacks.westen),
    layer: "ABSTANDSFLAECHE",
  });
  pdf.line(footprint.right, top, footprint.right, bottom, {
    ...dashed,
    color: setbackInk(setbacks.osten),
    layer: "ABSTANDSFLAECHE",
  });

  pdf.rect(footprint.left, footprint.top, footprint.right - footprint.left, footprint.bottom - footprint.top, {
    fill: "#e2e8f0",
    stroke: SOFT_INK,
    lineWidth: 0.9,
    layer: "GEBAEUDE",
  });
  pdf.drawText("Hauskörper", footprint.left, (footprint.top + footprint.bottom) / 2 - 4, {
    ...TEXT.tiny,
    color: SOFT_INK,
    align: "center",
    maxWidth: footprint.right - footprint.left,
    layer: "GEBAEUDE",
  });

  /* Die Maßzahlen der Abstände sind geprüft, die des Hauskörpers abgeleitet;
     die des Bezugsrechtecks bleiben gedämpft - dessen Seitenverhältnis ist die
     einzige Annahme, die das Blatt noch trifft. */
  const massColour = geometry.footprint.derived ? SOFT_INK : MUTED;

  /* Waagerechte Ketten oben und unten: die Abstände West und Ost. */
  const horizontal = {
    stations: [left, footprint.left, footprint.right, right],
    labels: [masszahl(setbacks.westen), masszahl(geometry.footprintWidthM), masszahl(setbacks.osten)],
    colors: [setbackInk(setbacks.westen), massColour, setbackInk(setbacks.osten)],
  };
  drawHorizontalChain(pdf, { ...horizontal, line: top - 14, extend: top });
  drawHorizontalChain(pdf, { ...horizontal, line: bottom + 14, extend: bottom });
  drawHorizontalChain(pdf, {
    stations: [left, right],
    labels: [masszahl(geometry.widthM)],
    colors: [MUTED],
    line: top - 34,
    extend: top,
  });

  /* Senkrechte Ketten links und rechts: die Abstände Nord und Süd. */
  const vertical = {
    stations: [top, footprint.top, footprint.bottom, bottom],
    labels: [masszahl(setbacks.norden), masszahl(geometry.footprintDepthM), masszahl(setbacks.sueden)],
    colors: [setbackInk(setbacks.norden), massColour, setbackInk(setbacks.sueden)],
  };
  drawVerticalChain(pdf, { ...vertical, line: left - 14, extend: left });
  drawVerticalChain(pdf, { ...vertical, line: right + 14, extend: right });
  drawVerticalChain(pdf, {
    stations: [top, bottom],
    labels: [masszahl(geometry.depthM)],
    colors: [MUTED],
    line: left - 38,
    extend: left,
  });
}

/* --------------------------------------------------------------------------
 * The drawing sheet as a recording.
 *
 * The Lageplan is the one part of the dossier that is a deliverable in its own
 * right - it goes to a plotter or into an existing plan set, so it has to leave
 * this repository as DXF and SVG as well as PDF. All three exports have to be
 * the same drawing, and the cheapest way to guarantee that is to draw it once:
 * `recordPlanSheet` runs the drawing functions against a recorder that keeps the
 * primitives instead of writing them, and the PDF path then replays exactly
 * those primitives through src/pdf.js. A second drawing routine would be a
 * second drawing, and the first correction would land in only one of them.
 *
 * The ops are in sheet points, top-down - the same space the drawing functions
 * already work in. Each op names the CAD layer it belongs to, so the vector
 * exports can group by layer without re-deriving it from colours and dashes.
 * ------------------------------------------------------------------------ */

/**
 * Drop-in stand-in for the PDF writer that collects what would have been drawn.
 *
 * Only the four calls the sheet makes are implemented, on purpose: an interface
 * this narrow is easy to keep identical to src/pdf.js, and a method that does
 * not exist here would be a mistake in the drawing code rather than a silent
 * difference between the PDF and the exports.
 */
class PlanRecorder {
  constructor() {
    this.ops = [];
    this.font = "helvetica";
    this.size = 10;
    this.color = INK;
  }

  setFont(font) {
    this.font = font;
    return this;
  }

  setSize(size) {
    this.size = size;
    return this;
  }

  setColor(color) {
    this.color = color;
    return this;
  }

  width(text) {
    return textWidth(text, { font: this.font, size: this.size });
  }

  wrap(text, maxWidth) {
    return wrapText(text, maxWidth, { font: this.font, size: this.size });
  }

  blockHeight(text, maxWidth, { leading = 1.35 } = {}) {
    return this.wrap(text, maxWidth).length * this.size * leading;
  }

  drawText(text, x, top, { font = this.font, size = this.size, color = this.color, align = "left", maxWidth = null, layer = null } = {}) {
    const content = String(text ?? "");
    const left = resolveTextX(content, x, { font, size, align, maxWidth });
    this.ops.push({ kind: "text", layer, text: content, x: left, top, font, size, color });
    return { x: left, top, width: textWidth(content, { font, size }) };
  }

  drawParagraph(text, x, top, { width, font = this.font, size = this.size, color = this.color, leading = 1.35, align = "left", layer = null } = {}) {
    let cursor = top;
    for (const line of wrapText(text, width, { font, size })) {
      this.drawText(line, x, cursor, { font, size, color, align, maxWidth: width, layer });
      cursor += size * leading;
    }
    return cursor;
  }

  line(x1, y1, x2, y2, { color = this.color, width = 0.7, dash = null, layer = null } = {}) {
    this.ops.push({ kind: "line", layer, x1, y1, x2, y2, color, width, dash });
  }

  rect(x, top, width, height, { stroke = null, fill = null, lineWidth = 0.7, layer = null } = {}) {
    this.ops.push({ kind: "rect", layer, x, top, width, height, stroke, fill, lineWidth });
  }
}

/**
 * Replays recorded ops into the PDF writer, in the recorded order. This is what
 * makes the PDF and the exported files one drawing: any difference between them
 * has to come from the writers, never from the geometry.
 */
function replayPlanOnPdf(pdf, ops) {
  for (const op of ops) {
    if (op.kind === "text") {
      pdf.drawText(op.text, op.x, op.top, { font: op.font, size: op.size, color: op.color });
    } else if (op.kind === "line") {
      pdf.line(op.x1, op.y1, op.x2, op.y2, { color: op.color, width: op.width, dash: op.dash });
    } else if (op.kind === "rect") {
      pdf.rect(op.x, op.top, op.width, op.height, { stroke: op.stroke, fill: op.fill, lineWidth: op.lineWidth });
    }
  }
}

/**
 * Draws the whole Lageplan sheet onto the recorder: frame, Blattkopf, drawing,
 * Nordpfeil, legend and Schriftfeld. Blattkopf and Schriftfeld name the sheet,
 * because a printed sheet has to identify itself without the Mappe around it.
 */
function drawPlanFrame(pdf, model, geometry, blatt) {
  pdf.rect(SHEET_FRAME, SHEET_FRAME, SHEET.width - 2 * SHEET_FRAME, SHEET_BOTTOM - SHEET_FRAME, {
    stroke: INK,
    lineWidth: 1,
    layer: "BLATTRAHMEN",
  });

  pdf.drawText("Lageplan", SHEET_INNER.left, SHEET_INNER.top, {
    ...TEXT.sheetTitle,
    color: INK,
    layer: "BLATTKOPF",
  });
  pdf.drawText(
    `Abstandsflächen zum Grundstück · Maßstab ${geometry.label} · Maße in Metern`,
    SHEET_INNER.left,
    SHEET_INNER.top + 22,
    { ...TEXT.sheetSub, color: MUTED, layer: "BLATTKOPF" }
  );
  pdf.drawParagraph(
    `${model.projekt} · ${model.flurstueck}, Gem. ${model.gemarkung}`,
    SHEET_INNER.left,
    SHEET_INNER.top + 36,
    { ...TEXT.tiny, color: MUTED, width: SHEET_DRAW.right - SHEET_INNER.left, leading: 1.35, layer: "BLATTKOPF" }
  );

  drawPlanFigure(pdf, geometry);
  drawNorthArrow(pdf, { x: SHEET_DRAW.right - 26, tip: SHEET_DRAW.top + 16 });
  drawPlanLegend(pdf, geometry, legendLayout(geometry));
  drawTitleBlock(pdf, measureTitleBlock(model, geometry, blatt));
}

/**
 * Records the sheet instead of drawing it. The geometry comes back with the ops
 * because the vector exports need the scale (DXF coordinates are metres) and the
 * tests need something to assert against.
 */
function recordPlanSheet(model, blatt) {
  const canvas = new PlanRecorder();
  const geometry = planGeometry(model);
  drawPlanFrame(canvas, model, geometry, blatt);
  return { ops: canvas.ops, geometry, blatt };
}

/**
 * Puts a recorded sheet on its own page. The sheet object is returned so the
 * Blattregister can tell which pages are the Lageplan Blatt; the footer is
 * stamped later by `stampFooters`, because only then is the page total known.
 */
function drawPlanSheet(layout, plan) {
  const page = layout.newSheet();
  replayPlanOnPdf(layout.pdf, plan.ops);
  layout.cursor = SHEET_INNER.bottom;
  return page;
}

/* --------------------------------------------------------------------------
 * Signatures and the digital seal.
 * ------------------------------------------------------------------------ */

const SIGNATURE_HEADING = 24;
/** Blank paper above the rule that the signatory actually writes on. */
const SIGNATURE_SPACE = 34;

function signatureColumns(model) {
  const width = (CONTENT_WIDTH - 40) / 2;
  return [
    {
      x: CONTENT_LEFT,
      width,
      label: "Entwurfsverfasser",
      hint: "Unterschrift bauvorlageberechtigte(r) Entwurfsverfasser(in)",
      name: model.entwurfsverfasser,
    },
    {
      x: CONTENT_LEFT + width + 40,
      width,
      label: "Bauherrschaft",
      hint: "Ort, Datum / Unterschrift Bauherr(in)",
      name: model.bauherr,
    },
  ];
}

function measureSignatureBlock(model) {
  const tallest = Math.max(
    ...signatureColumns(model).map((column) => {
      const names = wrapText(column.name, column.width, TEXT.metaValue).length;
      const hints = wrapText(column.hint, column.width, TEXT.tiny).length;
      return SIGNATURE_SPACE + 5 + names * TEXT.metaValue.size * 1.3 + 3 + hints * TEXT.tiny.size * 1.4;
    })
  );
  return SIGNATURE_HEADING + tallest + 16;
}

function drawSignatureBlock(layout, model) {
  const pdf = layout.pdf;
  pdf.drawText("Unterschriften", CONTENT_LEFT, layout.cursor, { ...TEXT.heading, color: INK });
  layout.advance(SIGNATURE_HEADING);

  const ruleTop = layout.cursor + SIGNATURE_SPACE;
  for (const column of signatureColumns(model)) {
    pdf.drawText(column.label, column.x, layout.cursor, { ...TEXT.boxLabel, color: MUTED });
    pdf.line(column.x, ruleTop, column.x + column.width, ruleTop, { color: INK, width: 1 });

    /* Both lines are wrapped and measured, because a company name is data, not
       a constant: "Architekturbüro Dipl.-Ing. Muster" fits, but nothing
       guarantees the next one does. */
    const afterName = pdf.drawParagraph(column.name, column.x, ruleTop + 5, {
      ...TEXT.metaValue,
      color: INK,
      width: column.width,
      leading: 1.3,
    });
    pdf.drawParagraph(column.hint, column.x, afterName + 3, {
      ...TEXT.tiny,
      color: MUTED,
      width: column.width,
      leading: 1.4,
    });
  }

  layout.advance(measureSignatureBlock(model) - SIGNATURE_HEADING);
}

function sealLines(model) {
  return [
    { text: `Prüfziffer (SHA-256): ${model.hash}`, ...TEXT.seal, color: SOFT_INK },
    /* ENGINE_LABEL keeps the PDF and the HTML dossier consistent: both name the
       same engine build. */
    { text: `Erzeugt von BauLens ${VERSION} · Profil ${model.profileCode} (${model.citationBase}) · ${model.datum}`, ...TEXT.seal, color: MUTED },
    { text: `Prüfungsstand: DACH ${ENGINE_LABEL} · Berechnet deterministisch ohne externe Cloud-Dienste.`, ...TEXT.seal, color: MUTED },
    { text: "Deterministisch: gleicher Input ergibt dieselbe Prüfziffer.", ...TEXT.seal, color: MUTED },
    { text: DISCLAIMER, ...TEXT.sealNote, color: MUTED },
  ];
}

function measureSealBox(model) {
  const innerWidth = CONTENT_WIDTH - 24;
  const entries = sealLines(model).map((entry) => ({
    ...entry,
    lines: wrapText(entry.text, innerWidth, { font: entry.font, size: entry.size }),
  }));

  const contentHeight = entries.reduce((sum, entry) => sum + entry.lines.length * entry.size * 1.4, 0);
  return { entries, height: contentHeight + 20 };
}

/**
 * `measured` is passed in rather than computed here so the page break can be
 * decided for the signature block and the seal together: measure, then draw.
 */
function drawSealBox(layout, measured) {
  const pdf = layout.pdf;
  const { entries, height: boxHeight } = measured;
  const top = layout.cursor;
  pdf.rect(CONTENT_LEFT, top, CONTENT_WIDTH, boxHeight, { stroke: RULE, lineWidth: 0.7 });

  let cursor = top + 10;
  for (const entry of entries) {
    for (const line of entry.lines) {
      pdf.drawText(line, CONTENT_LEFT + 12, cursor, { font: entry.font, size: entry.size, color: entry.color });
      cursor += entry.size * 1.4;
    }
  }

  layout.cursor = top + boxHeight + 12;
}

/* --------------------------------------------------------------------------
 * Footers. "Seite 3 von 4" cannot be written while the page is being drawn -
 * the total is only known once everything is laid out - so the footers are
 * stamped at the end, onto each page object.
 * ------------------------------------------------------------------------ */

/**
 * Runs `draw` as if `page` were the current page. The writer draws against
 * "the last page added", which is what a top-down layout wants; stamping
 * footers is the one place that has to reach back to an earlier page.
 */
function withPage(pdf, page, draw) {
  const all = pdf.pages;
  pdf.pages = [page];
  try {
    draw();
  } finally {
    pdf.pages = all;
  }
}

/** Breite der Blatt-/Seitenangabe rechts in der Fußzeile (Platz für den Blattnamen). */
const FOOTER_LABEL_WIDTH = 232;

function stampFooters(pdf, pages, model, sheets) {
  const total = pages.length;
  pages.forEach((page, index) => {
    withPage(pdf, page, () => {
      /* Blattgeometrie je Seite: das Lageplan-Blatt ist quer, die übrigen sind
         hoch. Mit den Hochformat-Konstanten gesetzt, läge die Fußzeile des
         Querblattes unterhalb des Papiers - und die Blatt- und Seitenzahl
         verschwänden. */
      const left = MARGIN.left;
      const right = page.width - MARGIN.right;
      const footerTop = page.height - MARGIN.bottom;

      /* Jedes Blatt nennt sich selbst: wer eines aus der Mappe zieht, weiß ohne
         die Übersicht, wohin es gehört. Das Blatt ist die Ablageeinheit, die
         Seite die Zählung - deshalb stehen beide in der Fußzeile. */
      const blatt = sheets.find((entry) => index >= entry.from && index <= entry.to);
      const label = blatt
        ? `Blatt ${blatt.index}/${blatt.total} · ${blatt.name} · Seite ${index + 1} von ${total}`
        : `Seite ${index + 1} von ${total}`;

      pdf.line(left, footerTop, right, footerTop, { color: HAIRLINE, width: 0.5 });
      pdf.drawText(`BauLens – Vorprüfungsbefund · AZ ${model.aktenzeichen}`, left, footerTop + 6, {
        ...TEXT.footer,
        color: MUTED,
        maxWidth: right - left - FOOTER_LABEL_WIDTH - 10,
      });
      pdf.drawText(label, right - FOOTER_LABEL_WIDTH, footerTop + 6, {
        ...TEXT.footer,
        color: MUTED,
        align: "right",
        maxWidth: FOOTER_LABEL_WIDTH,
      });
    });
  });
}

/* --------------------------------------------------------------------------
 * Public API.
 * ------------------------------------------------------------------------ */

/**
 * Renders the dossier. Returns the document together with the layout audit, so
 * a caller (and the tests) can assert that no text left the printable area.
 */
export function buildDossierPdf(projectData = {}, runResult = {}, options = {}) {
  const model = buildModel(projectData, runResult, options);

  const pdf = createPdf({
    title: `Bauvorhaben ${model.projekt} – Bauaufsichtlicher Vorprüfungsbefund`,
    author: "BauLens",
    subject: `Befund ${model.aktenzeichen} (${model.verdict})`,
  });

  /* Recorded before anything is drawn: the sheet is a pure function of the
     model, and recording it first keeps the PDF a replay of the same ops the
     DXF and SVG exports are written from. */
  const plan = recordPlanSheet(model, blattNumber("lageplan"));

  const layout = new Layout(pdf);
  drawHeader(layout, model);
  drawMetaBox(layout, model);
  drawVerdictBox(layout, model);
  const overview = drawBlattOverview(layout);
  drawFindingsTable(layout, model);
  /* Bei aktiver Heilungssimulation folgt der Vergleich im Befund, vor dem Lageplan:
     so bleibt das Blattregister gleich, und der Lageplan zeigt den Bestand. */
  if (options.comparison) drawComparison(layout, model, projectData, options.comparison);
  const planPage = drawPlanSheet(layout, plan);

  /* The plan sheet is landscape; signatures and seal belong back on a portrait
     page. The break is explicit rather than left to `ensure` (which measures
     against the portrait content bottom and would happily draw the signature
     block onto the shorter landscape sheet). */
  layout.newPage();

  /* Signatures and seal travel together: a page with a signature rule but no
     Prüfziffer would suggest the document is complete when it is not. Both are
     measured first, so the break happens before either is drawn. */
  const seal = measureSealBox(model);
  layout.ensure(measureSignatureBlock(model) + 12 + seal.height);
  drawSignatureBlock(layout, model);
  drawSealBox(layout, seal);

  /* The Blätter are only known once every sheet is laid out, so the ranges in
     the register and the Blatt numbers in the footers are all stamped here. */
  const sheets = blattPlan(layout.pages, planPage);
  stampBlattOverview(pdf, overview, sheets, layout.pages.length);
  stampFooters(pdf, layout.pages, model, sheets);

  return {
    buffer: pdf.toBuffer(),
    pages: layout.pages.length,
    hash: model.hash,
    audit: pdf.audit(),
  };
}

/** Convenience wrapper: the file bytes only. */
export function renderDossierPdf(projectData, runResult, options) {
  return buildDossierPdf(projectData, runResult, options).buffer;
}

/* --------------------------------------------------------------------------
 * The Lageplan as a file of its own: DXF for the CAD side, SVG for the plotter
 * and for anything that only understands vectors.
 *
 * Both take the same `projectData`/`runResult` as the dossier, and both go
 * through the same recording, so an exported DXF is the sheet in the PDF - not
 * a second drawing that will drift the moment one of them is corrected.
 * ------------------------------------------------------------------------ */

/**
 * The recorded sheet plus the model behind it. The geometry is returned because
 * the DXF writes metres and needs the scale, and because the tests assert
 * against numbers rather than against byte counts.
 */
export function buildPlanSheet(projectData = {}, runResult = {}, options = {}) {
  const model = buildModel(projectData, runResult, options);
  return { ...recordPlanSheet(model, blattNumber("lageplan")), model };
}

/** Dieselbe Zeile, die Blattkopf und Schriftfeld nennen - Dateititel und SVG-Metadaten. */
function planDocumentTitle(model, geometry) {
  return `Lageplan – Abstandsflächen · ${model.projekt} · Maßstab ${geometry.label}`;
}

/**
 * Das Blatt in einem der beiden Vektorformate, zusammen mit Aufzeichnung und
 * Modell. Einmal gerendert, damit ein Aufrufer Inhalt und Kennzahlen (Maßstab,
 * Prüfziffer) nicht aus zwei Renderungen zusammensetzen muss - zwei Renderungen
 * wären zwei Gelegenheiten, sich zu unterscheiden.
 *
 * Der Maßstab geht ins DXF, weil die Punkte des Blattes Papier sind und eine
 * Zeichnungseinheit ein Meter ist; ohne ihn wären alle Maße um den Faktor
 * 1:250 zu groß.
 */
export function planDocument(projectData, runResult, options, format = "svg") {
  const sheet = buildPlanSheet(projectData, runResult, options);
  const { ops, geometry, model } = sheet;
  const common = { width: SHEET.width, height: SHEET.height };

  if (format === "dxf") {
    return {
      ...sheet,
      contents: planDxf(ops, {
        ...common,
        unitPtPerMetre: geometry.unit,
        comment: [
          planDocumentTitle(model, geometry),
          `BauLens ${VERSION} · ${model.aktenzeichen} · ${model.datum}`,
          "Zeichnungseinheit: 1 Einheit = 1 m (Metrisch)",
          `Maßstab des Blattes: 1:${geometry.denominator} · Blattkopf, Legende und Maßstabsbalken liegen auf den Ebenen BLATTRAHMEN, BLATTKOPF, LEGENDE, MASSSTAB und SCHRIFTFELD und lassen sich einfrieren`,
          "Gezeichnet aus den Angaben der Planung; wer diese Zeichnung wieder einliest, erhält die gezeichneten Maße, kein amtlicher Lageplan.",
        ],
      }),
    };
  }

  return {
    ...sheet,
    contents: planSvg(ops, {
      ...common,
      title: planDocumentTitle(model, geometry),
      description:
        `BauLens ${VERSION} · ${model.aktenzeichen} · ${model.datum} · 1:${geometry.denominator} · ` +
        `Abstände in Metern · kein amtlicher Lageplan`,
    }),
  };
}

/** Der Lageplan als SVG-Blatt (A4 quer, 1 Blatt = 1 Datei). */
export function renderPlanSvg(projectData, runResult, options) {
  return planDocument(projectData, runResult, options, "svg").contents;
}

/** Der Lageplan als DXF-Blatt, eine Zeichnungseinheit = ein Meter. */
export function renderPlanDxf(projectData, runResult, options) {
  return planDocument(projectData, runResult, options, "dxf").contents;
}

/**
 * Writes the dossier to disk.
 *
 * A missing parent directory is an error, not something to fix by creating the
 * tree: silently inventing `C:\Projekte\2026\Bauamt\` because of a typo in a
 * path is how files end up somewhere nobody looks for them. Same choice as
 * write_workspace_file in the shell.
 */
function assertWritable(filePath, format) {
  if (typeof filePath !== "string" || filePath.trim() === "") {
    const error = new Error(`Es wurde kein Zielpfad für das ${format} angegeben.`);
    error.code = "missing_path";
    throw error;
  }

  const directory = dirname(filePath);
  if (!existsSync(directory)) {
    const error = new Error(`Das Zielverzeichnis existiert nicht: ${directory}`);
    error.code = "missing_directory";
    throw error;
  }
}

function writeSheetFile(filePath, contents, { format, code }) {
  assertWritable(filePath, format);
  try {
    writeFileSync(filePath, contents);
  } catch (cause) {
    const error = new Error(`${filePath} konnte nicht geschrieben werden: ${cause.message}`);
    error.code = code;
    throw error;
  }
  return { path: filePath, bytes: Buffer.byteLength(contents) };
}

export function writeDossierPdf(filePath, projectData, runResult, options) {
  assertWritable(filePath, "PDF");

  const { buffer, pages, hash } = buildDossierPdf(projectData, runResult, options);
  const written = writeSheetFile(filePath, buffer, { format: "PDF", code: "pdf_write_failed" });
  return { ...written, hash, pages };
}

/**
 * Writes one vector Blatt to disk and reports what it is: bytes, Maßstab and the
 * Prüfziffer of the check it was drawn from. The seal travels with the file so an
 * exported drawing can be tied back to the Befund it belongs to - and the file is
 * written from the same single render the caller would have got inline.
 */
function writePlan(filePath, projectData, runResult, options, format, labels) {
  const document = planDocument(projectData, runResult, options, format);
  const written = writeSheetFile(filePath, document.contents, labels);
  return { ...written, hash: document.model.hash, scale: document.geometry.label };
}

/** Der Lageplan als SVG-Datei. */
export function writePlanSvg(filePath, projectData, runResult, options) {
  return writePlan(filePath, projectData, runResult, options, "svg", {
    format: "SVG-Export (Lageplan)",
    code: "svg_write_failed",
  });
}

/** Der Lageplan als DXF-Datei. */
export function writePlanDxf(filePath, projectData, runResult, options) {
  return writePlan(filePath, projectData, runResult, options, "dxf", {
    format: "DXF-Export (Lageplan)",
    code: "dxf_write_failed",
  });
}

import { PROFILE_CODES, getProfile } from "./profiles.js";
import { runChecks, VERDICT } from "./engine.js";
import { createPdf, wrapText } from "./pdf.js";

/**
 * Evaluates a building spec simultaneously across all 16 German federal states (+ MBO reference).
 * Essential for prefabricated housing manufacturers (Fertighaushersteller / Typengenehmigung)
 * and architects standardizing residential designs across DACH.
 *
 * @param {object} spec - Building specification object
 * @param {object} options - Options passed to runChecks (e.g. rules filter)
 * @returns {object} Aggregated matrix result
 */
export function runAllLandsMatrix(spec, options = {}) {
  const matrix = [];
  const countsByVerdict = {
    [VERDICT.BESTANDEN]: 0,
    [VERDICT.AUFLAGEN]: 0,
    [VERDICT.NICHT_BESTANDEN]: 0,
  };

  const filterCountry = options.country ? options.country.trim().toUpperCase() : null;

  for (const code of PROFILE_CODES) {
    const profile = getProfile(code);
    if (!profile) continue;

    const country = profile.country ?? "DE";
    if (filterCountry && filterCountry !== "DACH" && filterCountry !== "ALL" && country !== filterCountry) {
      continue;
    }

    // Run isolated check against this specific state
    const checkRes = runChecks(spec, {
      ...options,
      land: code,
    });

    const isBayBO = code === "BY";
    const isBW = code === "BW";
    const h = Number(spec.gebaeude?.hoehe_m) || 0;

    // Compute effective setback requirements for quick comparison
    let setbackReq = 0;
    if (isBayBO) {
      setbackReq = null; // No standard MBO §6 setback regime
    } else if (isBW) {
      setbackReq = Math.max(5.0, 0.5 * h);
    } else if (country === "AT") {
      const f = profile.ruleOverrides?.abstandsflaeche?.params?.factor ?? 0.5;
      const minM = profile.ruleOverrides?.abstandsflaeche?.params?.minM ?? 3.0;
      setbackReq = Math.max(minM, f * h);
    } else if (country === "CH") {
      setbackReq = Math.max(3.5, 0.5 * h);
    } else {
      setbackReq = Math.max(3.0, 0.5 * h);
    }

    const stateEntry = {
      code,
      name: profile.name,
      citationBase: profile.citationBase,
      regimeSummary: profile.regimeSummary,
      confidence: profile.confidence,
      verdict: checkRes.verdict,
      counts: checkRes.counts,
      requiredSetbackM: setbackReq,
      hasBayBOGrenz: isBayBO,
      fails: checkRes.results.flatMap((r) =>
        r.findings.filter((f) => f.status === "fail").map((f) => ({ id: r.id, para: r.para, message: f.message }))
      ),
      warns: checkRes.results.flatMap((r) =>
        r.findings.filter((f) => f.status === "warn").map((f) => ({ id: r.id, para: r.para, message: f.message }))
      ),
      rawResult: checkRes,
    };

    countsByVerdict[checkRes.verdict] += 1;
    matrix.push(stateEntry);
  }

  const totalStates = matrix.length;
  const passedStates = countsByVerdict[VERDICT.BESTANDEN] + countsByVerdict[VERDICT.AUFLAGEN];
  const allPassed = countsByVerdict[VERDICT.NICHT_BESTANDEN] === 0;

  return {
    matrix,
    totalStates,
    passedStates,
    allPassed,
    countsByVerdict,
  };
}

/**
 * Formats the matrix evaluation result as an aligned ASCII / Unicode CLI table.
 *
 * @param {object} matrixResult - Output from runAllLandsMatrix
 * @returns {string} Formatted table string
 */
export function formatLandsMatrixTable(matrixResult) {
  const { matrix, passedStates, totalStates } = matrixResult;

  const VERDICT_SYMBOL = {
    [VERDICT.BESTANDEN]: "✓ BESTANDEN",
    [VERDICT.AUFLAGEN]: "▲ MIT AUFLAGEN",
    [VERDICT.NICHT_BESTANDEN]: "✗ NICHT BESTANDEN",
  };

  const rows = [
    ["Land", "Code", "Region / Kanton", "Norm", "Ergebnis", "Fehler", "Warn", "Min. Abstand", "Besonderheit"],
  ];

  for (const m of matrix) {
    const abstandStr = m.requiredSetbackM === null ? "Grenzbebauung" : `${m.requiredSetbackM.toFixed(1)} m`;
    let note = "–";
    if (m.code === "BY") note = "§22 BayBO (Kein MBO §6)";
    else if (m.code === "BW") note = "§6 LBO BW (Mind. 5m)";
    else if (m.code === "BE") note = "Permissive Grenzbebauung";
    else if (m.code.startsWith("AT_")) note = "OIB-Richtlinien (AT)";
    else if (m.code.startsWith("CH_")) note = "IVHB / PBG (CH)";

    const flag = m.code.startsWith("AT_") ? "🇦🇹 AT" : m.code.startsWith("CH_") ? "🇨🇭 CH" : "🇩🇪 DE";

    rows.push([
      flag,
      m.code,
      m.name,
      m.citationBase,
      VERDICT_SYMBOL[m.verdict] ?? m.verdict,
      String(m.counts.fail),
      String(m.counts.warn),
      abstandStr,
      note,
    ]);
  }

  const widths = [0, 1, 2, 3, 4, 5, 6, 7, 8].map((i) =>
    Math.max(...rows.map((r) => r[i].length))
  );

  const fmt = (r) => r.map((c, i) => c.padEnd(widths[i])).join("  ");
  const header = fmt(rows[0]);
  const separator = widths.map((w) => "─".repeat(w)).join("  ");
  const body = rows.slice(1).map(fmt).join("\n");

  const summary = `\nTypenprüfung-Zusammenfassung: ${passedStates} von ${totalStates} Ländern genehmigungsfähig.`;

  return `${header}\n${separator}\n${body}\n${summary}`;
}

/* ------------------------------------------------------------------ *
 * PDF: the same matrix as a printable table. Landscape A4, because seven
 * columns of Rechtsbereich, Norm and Ergebnis do not fit a portrait page at a
 * readable size. The rows are the matrix rows as the UI draws them, so the
 * printed table and the drawer agree.
 * ------------------------------------------------------------------ */

const LANDSCAPE_A4 = Object.freeze({ width: 841.89, height: 595.28 });
const PAGE_MARGIN = 40;
const TITLE_TOP = PAGE_MARGIN;
const FIRST_TABLE_TOP = PAGE_MARGIN + 56;
const FOOTER_TOP = LANDSCAPE_A4.height - 30;
const TABLE_BOTTOM_LIMIT = LANDSCAPE_A4.height - 50;
const CELL_PAD = 4;
const CELL_SIZE = 8.5;
const CELL_LEADING = 11;
const HEAD_HEIGHT = 20;
const INK = "#1a1a1a";
const MUTED = "#5f6368";
const RULE = "#c9ced6";
const HEAD_FILL = "#e8ecf1";

const MATRIX_COLUMNS = [
  { title: "Rechtsbereich", width: 170, align: "left" },
  { title: "Code", width: 66, align: "left" },
  { title: "Norm", width: 230, align: "left" },
  { title: "Ergebnis", width: 110, align: "left" },
  { title: "Fehler", width: 52, align: "right" },
  { title: "Warn.", width: 52, align: "right" },
  { title: "Min. Abstand", width: 80, align: "right" },
];

const VERDICT_CELL = {
  [VERDICT.BESTANDEN]: { label: "Bestanden", color: "#1e7b34" },
  [VERDICT.AUFLAGEN]: { label: "Mit Auflagen", color: "#9a5b00" },
  [VERDICT.NICHT_BESTANDEN]: { label: "Nicht bestanden", color: "#b3261e" },
};

const SCOPE_LABEL = { DE: "Deutschland (DE)", AT: "Österreich (AT)", CH: "Schweiz (CH)" };

function matrixRowCells(entry) {
  const setback =
    entry.requiredSetbackM === null || entry.requiredSetbackM === undefined
      ? "–"
      : `${Number(entry.requiredSetbackM).toFixed(1).replace(".", ",")} m`;
  const verdict = VERDICT_CELL[entry.verdict] ?? { label: String(entry.verdict), color: INK };
  return [
    { text: entry.name || entry.code, color: INK },
    { text: entry.code, color: INK },
    { text: entry.citationBase || "–", color: INK },
    { text: verdict.label, color: verdict.color },
    { text: String(entry.counts?.fail ?? 0), color: INK },
    { text: String(entry.counts?.warn ?? 0), color: INK },
    { text: setback, color: INK },
  ];
}

/** Lays the rows out in pages before anything is drawn, so the footer can say how many pages there are. */
function paginateMatrixRows(matrixResult) {
  const rows = matrixResult.matrix.map((entry) => {
    const cells = matrixRowCells(entry);
    const lines = cells.map((cell, i) =>
      wrapText(cell.text, MATRIX_COLUMNS[i].width - 2 * CELL_PAD, { font: "helvetica", size: CELL_SIZE })
    );
    const height = Math.max(...lines.map((l) => l.length)) * CELL_LEADING + 2 * CELL_PAD;
    return { cells, lines, height };
  });

  const pages = [];
  let page = { tableTop: FIRST_TABLE_TOP, rows: [] };
  let cursor = FIRST_TABLE_TOP + HEAD_HEIGHT;
  for (const row of rows) {
    if (cursor + row.height > TABLE_BOTTOM_LIMIT && page.rows.length > 0) {
      pages.push(page);
      page = { tableTop: PAGE_MARGIN, rows: [] };
      cursor = PAGE_MARGIN + HEAD_HEIGHT;
    }
    page.rows.push(row);
    cursor += row.height;
  }
  pages.push(page);
  return pages;
}

function drawMatrixHeader(pdf, { projekt, scope, summary }) {
  pdf.drawText("DACH-Matrix · Projekt gegen Bauordnungen", PAGE_MARGIN, TITLE_TOP, {
    font: "helvetica-bold",
    size: 14,
    color: INK,
  });
  pdf.drawText(`Projekt: ${projekt} · Umfang: ${scope}`, PAGE_MARGIN, TITLE_TOP + 18, { size: 9, color: MUTED });
  pdf.drawText(summary, PAGE_MARGIN, TITLE_TOP + 32, { size: 9, color: INK });
}

function drawMatrixTable(pdf, sheet) {
  const width = MATRIX_COLUMNS.reduce((sum, col) => sum + col.width, 0);
  let top = sheet.tableTop;

  pdf.rect(PAGE_MARGIN, top, width, HEAD_HEIGHT, { fill: HEAD_FILL });
  let x = PAGE_MARGIN;
  for (const col of MATRIX_COLUMNS) {
    pdf.drawText(col.title, x + CELL_PAD, top + 6, {
      font: "helvetica-bold",
      size: CELL_SIZE,
      color: INK,
      align: col.align,
      maxWidth: col.width - 2 * CELL_PAD,
    });
    x += col.width;
  }
  top += HEAD_HEIGHT;
  pdf.line(PAGE_MARGIN, top, PAGE_MARGIN + width, top, { color: RULE, width: 0.7 });

  for (const row of sheet.rows) {
    let cellX = PAGE_MARGIN;
    row.cells.forEach((cell, i) => {
      const col = MATRIX_COLUMNS[i];
      row.lines[i].forEach((text, n) => {
        pdf.drawText(text, cellX + CELL_PAD, top + CELL_PAD + n * CELL_LEADING, {
          font: "helvetica",
          size: CELL_SIZE,
          color: cell.color,
          align: col.align,
          maxWidth: col.width - 2 * CELL_PAD,
        });
      });
      cellX += col.width;
    });
    top += row.height;
    pdf.line(PAGE_MARGIN, top, PAGE_MARGIN + width, top, { color: RULE, width: 0.5 });
  }
}

function drawMatrixFooter(pdf, number, total) {
  pdf.drawText(
    "Vorprüfung auf Grundlage der Angaben – kein Bauschein · Min. Abstand „–“: Grenzbebauung (kein Regelabstand)",
    PAGE_MARGIN,
    FOOTER_TOP,
    { size: 7.5, color: MUTED }
  );
  pdf.drawText(`Seite ${number} von ${total}`, LANDSCAPE_A4.width - PAGE_MARGIN - 160, FOOTER_TOP, {
    size: 7.5,
    color: MUTED,
    align: "right",
    maxWidth: 160,
  });
}

/**
 * Renders a runAllLandsMatrix result as a landscape A4 PDF.
 *
 * @param {object} matrixResult - Output from runAllLandsMatrix
 * @param {object} meta - { projekt, country } as the drawer and the IPC method know them
 * @returns {{ buffer: Buffer, pages: number, audit: object }}
 */
export function buildLandsMatrixPdf(matrixResult, { projekt = "–", country = "DACH" } = {}) {
  const scope = SCOPE_LABEL[country] ?? "DACH · alle Rechtsbereiche";
  const failing = matrixResult.matrix.filter((e) => e.verdict === VERDICT.NICHT_BESTANDEN).length;
  const summary =
    `${matrixResult.passedStates} von ${matrixResult.totalStates} Rechtsbereichen genehmigungsfähig · ` +
    `${failing} nicht bestanden`;

  const pdf = createPdf({
    title: `DACH-Matrix – ${projekt}`,
    subject: `${summary} (${scope})`,
  });

  const pages = paginateMatrixRows(matrixResult);
  pages.forEach((sheet, index) => {
    pdf.addPage({ width: LANDSCAPE_A4.width, height: LANDSCAPE_A4.height });
    if (index === 0) drawMatrixHeader(pdf, { projekt, scope, summary });
    drawMatrixTable(pdf, sheet);
    drawMatrixFooter(pdf, index + 1, pages.length);
  });

  return { buffer: pdf.toBuffer(), pages: pages.length, audit: pdf.audit() };
}

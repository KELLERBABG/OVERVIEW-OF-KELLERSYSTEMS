/**
 * BauLens - the Lageplan drawing sheet as SVG.
 *
 * Why a second vector format next to the PDF: a plotter takes vectors, and the
 * architect who receives the dossier wants the Blatt in the plan set they are
 * drawing in, not as page five of a PDF. SVG is the format every drawing
 * program, browser and office suite opens, and it is text - so the exported
 * sheet can be inspected and diffed like the rest of this repository's output.
 *
 * Deliberate choices, all of them for the same reason the PDF makes them:
 *
 *   - **User units are PostScript points.** `width`/`height` state the paper size
 *     in pt ("841.89pt"), the viewBox is the same rectangle, so every number in
 *     the file is the number the drawing code computed. Nothing is rescaled on
 *     the way out, and the file prints 1:1 without arithmetic. Text keeps its
 *     point size, which is what makes a Maßzahl on paper the size the sheet says.
 *   - **No embedded font.** Same limit as src/pdf.js: the writer names the
 *     Helvetica/Arial/Courier families the PDF's base-14 faces map onto and lets
 *     the reader substitute. A plotter does not need the outlines, and a font
 *     that fails to embed opens as garbage.
 *   - **No external reference, no timestamp.** One file, self-contained,
 *     byte-identical for identical input.
 *
 * The colours, widths and dashes are not decided here: the ops carry them, so
 * the SVG cannot invent a look of its own.
 */

import { BASELINE_RATIO } from "./pdf.js";

/** Font stacks the PDF's base-14 faces map onto; xmllint-proof quoted families. */
const FONT_STACKS = {
  helvetica: { family: "Helvetica, Arial, sans-serif", weight: null, style: null },
  "helvetica-bold": { family: "Helvetica, Arial, sans-serif", weight: "bold", style: null },
  courier: { family: "'Courier New', Courier, monospace", weight: null, style: null },
  "courier-bold": { family: "'Courier New', Courier, monospace", weight: "bold", style: null },
  italic: { family: "Helvetica, Arial, sans-serif", weight: null, style: "italic" },
};

/** XML text out of arbitrary project names: ampersands appear in company names. */
function escapeXml(value) {
  return String(value ?? "").replace(/[&<>"']/g, (character) => {
    if (character === "&") return "&amp;";
    if (character === "<") return "&lt;";
    if (character === ">") return "&gt;";
    if (character === '"') return "&quot;";
    return "&apos;";
  });
}

/** Numbers with at most three decimals, trailing zeros trimmed - readable and stable. */
function num(value) {
  const rounded = Math.round(Number(value) * 1000) / 1000;
  if (!Number.isFinite(rounded)) return "0";
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(3).replace(/0+$/, "").replace(/\.$/, "");
}

function stackFor(font) {
  return FONT_STACKS[font] ?? FONT_STACKS.helvetica;
}

/**
 * Groups consecutive ops of the same layer, so a drawing program that reads the
 * file by layer finds one group per layer instead of one per primitive. Runs are
 * kept in recorded order rather than merged: stacking order is part of the
 * drawing, and re-sorting by layer would move ink in front of or behind other
 * ink.
 */
function layerRuns(ops) {
  const runs = [];
  for (const op of ops) {
    const layer = op.layer ?? "0";
    const last = runs[runs.length - 1];
    if (last && last.layer === layer) last.ops.push(op);
    else runs.push({ layer, ops: [op], first: !runs.some((run) => run.layer === layer) });
  }
  return runs;
}

function lineElement(op) {
  const dash = op.dash ? ` stroke-dasharray="${op.dash.map(num).join(" ")}"` : "";
  return (
    `<line x1="${num(op.x1)}" y1="${num(op.y1)}" x2="${num(op.x2)}" y2="${num(op.y2)}" ` +
    `stroke="${op.color}" stroke-width="${num(op.width)}"${dash}/>`
  );
}

function rectElement(op) {
  const fill = op.fill ? `fill="${op.fill}"` : 'fill="none"';
  const stroke = op.stroke ? ` stroke="${op.stroke}" stroke-width="${num(op.lineWidth)}"` : "";
  return `<rect x="${num(op.x)}" y="${num(op.top)}" width="${num(op.width)}" height="${num(op.height)}" ${fill}${stroke}/>`;
}

function textElement(op) {
  const stack = stackFor(op.font);
  const weight = stack.weight ? ` font-weight="${stack.weight}"` : "";
  const style = stack.style ? ` font-style="${stack.style}"` : "";
  const y = op.top + op.size * BASELINE_RATIO;
  return (
    `<text x="${num(op.x)}" y="${num(y)}" font-family="${escapeXml(stack.family)}" ` +
    `font-size="${num(op.size)}"${weight}${style} fill="${op.color}">${escapeXml(op.text)}</text>`
  );
}

function elementFor(op) {
  if (op.kind === "line") return lineElement(op);
  if (op.kind === "rect") return rectElement(op);
  if (op.kind === "text") return textElement(op);
  return null;
}

/**
 * The sheet as one standalone SVG document.
 *
 * `width`/`height` are the sheet in points (A4 quer), `ops` the recorded
 * primitives from src/dossierpdf.js. The white background rectangle is not
 * decoration: on a plotter or in a viewer with a dark canvas, a drawing sheet
 * with a transparent background is unreadable, and the sheet is white paper.
 */
export function planSvg(ops, { width, height, title = "", description = "" } = {}) {
  const body = [];
  body.push(`<rect x="0" y="0" width="${num(width)}" height="${num(height)}" fill="#ffffff"/>`);

  for (const run of layerRuns(ops)) {
    const id = run.first ? ` id="layer-${escapeXml(run.layer)}"` : "";
    body.push(`<g${id} data-layer="${escapeXml(run.layer)}">`);
    for (const op of run.ops) {
      const element = elementFor(op);
      if (element) body.push(element);
    }
    body.push("</g>");
  }

  const head = [
    '<?xml version="1.0" encoding="UTF-8" standalone="no"?>',
    `<svg xmlns="http://www.w3.org/2000/svg" version="1.1" width="${num(width)}pt" height="${num(height)}pt" ` +
      `viewBox="0 0 ${num(width)} ${num(height)}">`,
    title ? `  <title>${escapeXml(title)}</title>` : null,
    description ? `  <desc>${escapeXml(description)}</desc>` : null,
  ].filter(Boolean);

  const indented = body.map((line) => `  ${line}`);
  return `${head.join("\n")}\n${indented.join("\n")}\n</svg>\n`;
}

/**
 * BauLens - minimal, dependency-free PDF writer.
 *
 * Why this file exists: the project ships with no dependencies at all (Node
 * standard library only, no install step), and the desktop app needs a real PDF
 * file instead of a print dialog. PDF is a text format, so a writer is a few
 * hundred lines rather than a dependency.
 *
 * Deliberate limits, all documented because they matter for a dossier:
 *
 *   - **Base-14 fonts.** Helvetica, Helvetica-Bold, Courier and Courier-Bold are
 *     part of every PDF reader, so nothing has to be embedded. The bundled IBM
 *     Plex webfonts are *not* used here: embedding them would mean converting
 *     woff2 to sfnt inside this repository, and a font that fails to embed opens
 *     as garbage. Base-14 with WinAnsiEncoding covers German text (umlauts,
 *     sharp s, section sign, euro, en/em dash, German quotes).
 *   - **Uncompressed content streams.** A dossier is tens of kilobytes; readable
 *     streams are worth more than the bytes saved, and they keep the output
 *     diffable and byte-reproducible.
 *   - **No timestamps.** `/CreationDate` is omitted and `/ID` is derived from the
 *     content, so the same input always produces the same bytes. That is the
 *     same promise the report makes: determinism, not just correctness.
 *
 * Coordinates: PDF counts from the bottom-left. This API is top-down, because
 * document layout is written top-down; `x` is measured from the left edge and
 * `top` from the top of the page.
 */

import { createHash } from "node:crypto";
import { METRICS } from "./pdfmetrics.js";

/** DIN A4 in PostScript points (210 x 297 mm). */
export const A4 = Object.freeze({ width: 595.28, height: 841.89 });

/**
 * How far the baseline sits below the top of the em box, as a fraction of the
 * font size. Layout hands this writer the top edge of a line, PDF and every
 * other vector format position text on its baseline, so the offset has to live
 * in exactly one place - the SVG and DXF exports use it too (src/plansvg.js,
 * src/plandxf.js), and text a fraction of a point apart in two exports of the
 * same sheet is a defect nobody would ever report.
 */
export const BASELINE_RATIO = 0.72;

/** The four base-14 faces this writer uses. */
const FONTS = {
  helvetica: { resource: "F1", base: "Helvetica" },
  "helvetica-bold": { resource: "F2", base: "Helvetica-Bold" },
  courier: { resource: "F3", base: "Courier" },
  "courier-bold": { resource: "F4", base: "Courier-Bold" },
  italic: { resource: "F5", base: "Helvetica-Oblique" },
};

export const FONT_NAMES = Object.keys(FONTS);


/** Characters that need a non-Latin-1 code point in WinAnsiEncoding. */
const WINANSI_HIGH = new Map(
  Object.entries({
    "€": 0x80, "‚": 0x82, "ƒ": 0x83, "„": 0x84, "…": 0x85, "†": 0x86,
    "‡": 0x87, "ˆ": 0x88, "‰": 0x89, "Š": 0x8a, "‹": 0x8b, "Œ": 0x8c,
    "Ž": 0x8e, "‘": 0x91, "’": 0x92, "“": 0x93, "”": 0x94, "•": 0x95,
    "–": 0x96, "—": 0x97, "˜": 0x98, "™": 0x99, "š": 0x9a, "›": 0x9b,
    "œ": 0x9c, "ž": 0x9e, "Ÿ": 0x9f,
  })
);

/**
 * Glyphs that have no WinAnsi code point at all, spelled out instead of dropped:
 * the dossier uses arrows and check marks, and "?" in a legal document is worse
 * than a plain-text arrow.
 */
const ASCII_FALLBACK = new Map(
  Object.entries({
    "↑": "^", "↓": "v", "→": "->", "←": "<-", "✓": "ok", "✗": "x",
    "·": "-", "−": "-", "≈": "~", "≤": "<=", "≥": ">=", "°": "°",
    "\u00a0": " ", "′": "'", "″": '"', "×": "x",
  })
);

/**
 * Maps a string onto WinAnsi bytes. Characters the encoding cannot carry become
 * "?", and control characters become a space: a raw 0x09 or 0x0C inside a text
 * string is not something a reader draws, and it would also make the measured
 * width of a line disagree with what appears on the page.
 */
export function toWinAnsi(text) {
  const bytes = [];
  for (const char of String(text ?? "")) {
    const code = char.codePointAt(0);
    if (code < 0x20 || code === 0x7f) {
      bytes.push(0x20);
      continue;
    }
    if (code < 0x80 || (code >= 0xa0 && code <= 0xff)) {
      bytes.push(code);
      continue;
    }
    if (WINANSI_HIGH.has(char)) {
      bytes.push(WINANSI_HIGH.get(char));
      continue;
    }
    const replacement = ASCII_FALLBACK.get(char);
    if (replacement !== undefined) {
      bytes.push(...toWinAnsi(replacement));
      continue;
    }
    // Emoji and everything else we cannot spell: never silently dropped, so the
    // reader can see that something was there.
    bytes.push(0x3f); // "?"
  }
  return Buffer.from(bytes);
}

/**
 * Advance width of one drawn byte, in 1/1000 em.
 *
 * Bytes below 0x20 cannot be produced by `toWinAnsi` (they become spaces), so
 * the `??` branch is only a safety net; it uses the width of "?" because that is
 * the glyph an unmappable character is drawn as.
 */
function byteWidth(byte, font) {
  const metrics = METRICS[font];
  if (metrics.mono !== undefined) return metrics.mono;
  if (byte >= 0x20 && byte <= 0x7e) return metrics.low[byte - 0x20];
  return metrics.high[byte] ?? metrics.low[0x3f - 0x20];
}

/**
 * Width of a string *as it will be drawn*.
 *
 * Measured on the encoded bytes rather than on the source characters, because
 * the encoding is not length-preserving: an arrow is spelled out as "->", and
 * measuring the two-character source would under-report the drawn width by a
 * whole glyph. Measuring exactly what is drawn is the invariant that keeps
 * wrapping and right-alignment honest.
 */
export function textWidth(text, { font = "helvetica", size = 10 } = {}) {
  if (!METRICS[font]) throw new Error(`unknown font "${font}"`);
  let units = 0;
  for (const byte of toWinAnsi(text)) units += byteWidth(byte, font);
  return (units * size) / 1000;
}

/** Splits text into lines that fit `maxWidth`, breaking on whitespace. */
export function wrapText(text, maxWidth, options = {}) {
  const lines = [];
  for (const rawLine of String(text ?? "").split(/\r?\n/)) {
    const words = rawLine.split(/\s+/).filter((w) => w !== "");
    if (words.length === 0) {
      lines.push("");
      continue;
    }
    let current = "";
    for (const word of words) {
      const candidate = current === "" ? word : `${current} ${word}`;
      if (textWidth(candidate, options) <= maxWidth || current === "") {
        current = candidate;
        continue;
      }
      lines.push(current);
      current = word;
    }
    lines.push(current);
  }
  return lines;
}

/**
 * Left edge a piece of text is drawn at, after `align`/`maxWidth` are applied.
 *
 * Exported because the sheet exporters (src/plansvg.js, src/plandxf.js) receive
 * pre-resolved positions from the recorder in src/dossierpdf.js. If alignment
 * were computed twice - once here, once there - the vector exports would drift
 * from the PDF by a fraction of a point, and a drawing sheet that disagrees with
 * its own PDF is worse than no export.
 */
export function resolveTextX(text, x, { font = "helvetica", size = 10, align = "left", maxWidth = null } = {}) {
  if (align === "left" || maxWidth === null) return x;
  const width = textWidth(text, { font, size });
  return align === "right" ? x + maxWidth - width : x + (maxWidth - width) / 2;
}

function escapeLiteral(bytes) {
  /* Only the string delimiters, the escape character and the two line breaks
     need escaping in a literal string; the bytes themselves are written
     verbatim. \\r and \\n are the spec's own escapes, so escaping them is
     lossless for both content strings and UTF-16BE metadata. */
  return bytes
    .toString("latin1")
    .replace(/[\\()]/g, (m) => `\\${m}`)
    .replace(/\r/g, "\\r")
    .replace(/\n/g, "\\n");
}

/**
 * Encodes a string as a PDF *text string* for the Info dictionary.
 *
 * This is not the same encoding as the page content. Content streams declare
 * WinAnsiEncoding through the font, but the Info dictionary has no font: its
 * strings are PDFDocEncoding, which differs from WinAnsi in exactly the range
 * this document uses (in PDFDocEncoding 0x96 is "Œ", not "–"). Writing the
 * WinAnsi bytes there makes every reader show mojibake in the file's title.
 * UTF-16BE with a byte-order mark is defined by PDF 1.4 for precisely this, and
 * it needs no table because it is a real Unicode encoding.
 */
function toPdfTextString(text) {
  const utf16 = Buffer.from(String(text ?? ""), "utf16le");
  utf16.swap16(); // utf16le -> utf16be; the length is always even
  return Buffer.concat([Buffer.from([0xfe, 0xff]), utf16]);
}

function hexToRgb(hex) {
  const match = /^#?([0-9a-f]{6})$/i.exec(String(hex ?? ""));
  if (!match) return null;
  const value = parseInt(match[1], 16);
  return [((value >> 16) & 0xff) / 255, ((value >> 8) & 0xff) / 255, (value & 0xff) / 255];
}

const num = (value) => {
  const rounded = Math.round(value * 1000) / 1000;
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(3).replace(/0+$/, "").replace(/\.$/, "");
};

class Page {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this.ops = [];
    this.maxTextX = 0;
    this.maxTextBottom = 0;
  }

  /* Top-down helpers: PDF's origin is bottom-left. */
  toPdfY(top, size) {
    return this.height - top - size * BASELINE_RATIO;
  }
}

class PdfDocument {
  constructor(meta = {}) {
    this.meta = {
      title: meta.title ?? "BauLens Dokument",
      author: meta.author ?? "BauLens",
      subject: meta.subject ?? "",
      creator: meta.creator ?? "BauLens",
    };
    this.pages = [];
    this.font = "helvetica";
    this.size = 10;
    this.color = "#000000";
  }

  addPage({ width = A4.width, height = A4.height } = {}) {
    const page = new Page(width, height);
    this.pages.push(page);
    return page;
  }

  get page() {
    if (this.pages.length === 0) this.addPage();
    return this.pages[this.pages.length - 1];
  }

  get pageCount() {
    return Math.max(1, this.pages.length);
  }

  setFont(font) {
    if (!FONTS[font]) throw new Error(`unknown font "${font}"`);
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

  /** Width of `text` in the current font and size. */
  width(text) {
    return textWidth(text, { font: this.font, size: this.size });
  }

  wrap(text, maxWidth) {
    return wrapText(text, maxWidth, { font: this.font, size: this.size });
  }

  /** Height of a wrapped block, so a page break can be decided before drawing. */
  blockHeight(text, maxWidth, { leading = 1.35 } = {}) {
    return this.wrap(text, maxWidth).length * this.size * leading;
  }

  /**
   * Draws one line of text with its top edge at `top`.
   * Returns `{ x, top, width }` for the drawn portion.
   */
  drawText(text, x, top, { font = this.font, size = this.size, color = this.color, align = "left", maxWidth = null } = {}) {
    const page = this.page;
    const content = String(text ?? "");
    const drawX = resolveTextX(content, x, { font, size, align, maxWidth });

    const bytes = toWinAnsi(content);
    const width = textWidth(content, { font, size });
    const rgb = hexToRgb(color) ?? [0, 0, 0];
    const y = page.toPdfY(top, size);

    page.ops.push(
      `BT ${num(rgb[0])} ${num(rgb[1])} ${num(rgb[2])} rg /${FONTS[font].resource} ${num(size)} Tf ` +
        `1 0 0 1 ${num(drawX)} ${num(y)} Tm (${escapeLiteral(bytes)}) Tj ET`
    );

    page.maxTextX = Math.max(page.maxTextX, drawX + width);
    page.maxTextBottom = Math.max(page.maxTextBottom, top + size);
    return { x: drawX, top, width };
  }

  /** Draws a wrapped paragraph. Returns the top offset for the next block. */
  drawParagraph(text, x, top, { width, font = this.font, size = this.size, color = this.color, leading = 1.35, align = "left" } = {}) {
    const lines = wrapText(text, width, { font, size });
    let cursor = top;
    for (const line of lines) {
      this.drawText(line, x, cursor, { font, size, color, align, maxWidth: width });
      cursor += size * leading;
    }
    return cursor;
  }

  line(x1, y1, x2, y2, { color = this.color, width = 0.7, dash = null } = {}) {
    const page = this.page;
    const rgb = hexToRgb(color) ?? [0, 0, 0];
    const ops = [
      `${num(rgb[0])} ${num(rgb[1])} ${num(rgb[2])} RG`,
      `${num(width)} w`,
      dash ? `[${dash.map(num).join(" ")}] 0 d` : "[] 0 d",
      `${num(x1)} ${num(page.height - y1)} m ${num(x2)} ${num(page.height - y2)} l S`,
    ];
    page.ops.push(ops.join(" "));
  }

  rect(x, top, width, height, { stroke = null, fill = null, lineWidth = 0.7 } = {}) {
    const page = this.page;
    const y = page.height - (top + height);
    if (fill) {
      const rgb = hexToRgb(fill) ?? [0, 0, 0];
      page.ops.push(`${num(rgb[0])} ${num(rgb[1])} ${num(rgb[2])} rg ${num(x)} ${num(y)} ${num(width)} ${num(height)} re f`);
    }
    if (stroke) {
      const rgb = hexToRgb(stroke) ?? [0, 0, 0];
      page.ops.push(
        `${num(rgb[0])} ${num(rgb[1])} ${num(rgb[2])} RG ${num(lineWidth)} w ${num(x)} ${num(y)} ${num(width)} ${num(height)} re S`
      );
    }
  }

  /**
   * Layout audit for tests and for the caller: the furthest right edge any drawn
   * text reached, per page, plus anything that left the page. A dossier with a
   * word over the page edge is a defect, not a rounding error.
   */
  audit() {
    return {
      pages: this.pages.map((page, index) => ({
        index: index + 1,
        width: page.width,
        height: page.height,
        ops: page.ops.length,
        maxTextX: Math.round(page.maxTextX * 100) / 100,
        maxTextBottom: Math.round(page.maxTextBottom * 100) / 100,
        overflow: page.maxTextX > page.width || page.maxTextBottom > page.height,
      })),
      overflow: this.pages.some((page) => page.maxTextX > page.width || page.maxTextBottom > page.height),
    };
  }

  /** Serialises the document. Deterministic: no timestamps, content-derived /ID. */
  toBuffer() {
    if (this.pages.length === 0) this.addPage();

    const objects = [];
    const push = (body) => {
      objects.push(body);
      return objects.length; // 1-based object number
    };

    const fontObjects = new Map();
    for (const [name, spec] of Object.entries(FONTS)) {
      fontObjects.set(
        name,
        push(`<< /Type /Font /Subtype /Type1 /BaseFont /${spec.base} /Encoding /WinAnsiEncoding >>`)
      );
    }

    /* /Pages is written after the pages so the kid references are known; the
       catalogue is written first and patched at the end. */
    const catalogNumber = push("");
    const pagesNumber = push("");

    const fontResources = [...fontObjects.values()].map((n, i) => `/F${i + 1} ${n} 0 R`).join(" ");

    const pageNumbers = [];
    for (const page of this.pages) {
      const content = page.ops.join("\n");
      const contentNumber = push(`<< /Length ${Buffer.byteLength(content, "latin1")} >>\nstream\n${content}\nendstream`);
      const pageNumber = push(
        `<< /Type /Page /Parent ${pagesNumber} 0 R /MediaBox [0 0 ${num(page.width)} ${num(page.height)}] ` +
          `/Resources << /Font << ${fontResources} >> >> /Contents ${contentNumber} 0 R >>`
      );
      pageNumbers.push(pageNumber);
    }

    objects[catalogNumber - 1] = `<< /Type /Catalog /Pages ${pagesNumber} 0 R >>`;
    objects[pagesNumber - 1] = `<< /Type /Pages /Kids [${pageNumbers.map((n) => `${n} 0 R`).join(" ")}] /Count ${pageNumbers.length} >>`;

    const infoNumber = push(
      `<< /Title (${escapeLiteral(toPdfTextString(this.meta.title))}) ` +
        `/Author (${escapeLiteral(toPdfTextString(this.meta.author))}) ` +
        `/Subject (${escapeLiteral(toPdfTextString(this.meta.subject))}) ` +
        `/Creator (${escapeLiteral(toPdfTextString(this.meta.creator))}) ` +
        `/Producer (${escapeLiteral(toPdfTextString(this.meta.creator))}) >>`
    );

    const chunks = [];
    const offsets = [];
    let offset = 0;
    const emit = (text) => {
      const buffer = Buffer.from(text, "latin1");
      chunks.push(buffer);
      offset += buffer.length;
    };

    emit("%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

    objects.forEach((body, index) => {
      offsets[index] = offset;
      emit(`${index + 1} 0 obj\n${body}\nendobj\n`);
    });

    const xrefOffset = offset;
    let xref = `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
    for (let i = 0; i < objects.length; i++) {
      xref += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
    }
    emit(xref);

    /* /ID from the content, so the same document gets the same identifier. */
    const digest = createHash("md5")
      .update(chunks.map((c) => c.toString("latin1")).join(""))
      .digest("hex");
    emit(
      `trailer\n<< /Size ${objects.length + 1} /Root ${catalogNumber} 0 R /Info ${infoNumber} 0 R ` +
        `/ID [<${digest}> <${digest}>] >>\nstartxref\n${xrefOffset}\n%%EOF\n`
    );

    return Buffer.concat(chunks);
  }
}

export function createPdf(meta = {}) {
  return new PdfDocument(meta);
}

/**
 * Reads back the object offsets a reader would use. Exported because "the file
 * opens in a reader" is the only claim that matters, and this is what makes that
 * claim checkable without a reader: every offset must point at "<n> 0 obj".
 */
export function inspectXref(buffer) {
  const text = buffer.toString("latin1");
  const startxref = /startxref\s+(\d+)/.exec(text);
  if (!startxref) return { valid: false, reason: "no startxref" };
  const start = Number(startxref[1]);
  const table = text.slice(start);
  if (!table.startsWith("xref")) return { valid: false, reason: "startxref does not point at xref" };

  /* The writer emits a single subsection; the header line names its first
     object number, which is what turns an entry index into an object number. */
  const lines = table.split("\n");
  if (lines[0].trim() !== "xref") return { valid: false, reason: "malformed xref keyword" };
  const header = /^(\d+)\s+(\d+)$/.exec((lines[1] ?? "").trim());
  if (!header) return { valid: false, reason: "malformed xref subsection header" };

  const firstObject = Number(header[1]);
  const declared = Number(header[2]);
  const problems = [];
  let count = 0;
  for (let i = 2; i < lines.length; i++) {
    const match = /^(\d{10}) (\d{5}) ([nf])\s*$/.exec(lines[i].trimEnd());
    if (!match) break;
    const objectNumber = firstObject + count;
    count++;
    if (match[3] === "f") continue;
    const at = Number(match[1]);
    if (!text.startsWith(`${objectNumber} 0 obj`, at)) {
      problems.push({ object: objectNumber, offset: at, found: text.slice(at, at + 16) });
    }
  }
  if (count !== declared) problems.push({ declared, found: count });
  return {
    valid: problems.length === 0 && count > 1 && text.trimEnd().endsWith("%%EOF"),
    entries: count,
    problems,
  };
}

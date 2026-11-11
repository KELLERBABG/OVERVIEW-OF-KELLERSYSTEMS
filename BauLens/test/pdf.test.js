import { test } from "node:test";
import assert from "node:assert/strict";
import { A4, FONT_NAMES, createPdf, inspectXref, textWidth, toWinAnsi, wrapText } from "../src/pdf.js";

/* The PDF writer is the piece the dossier depends on to produce a file at all,
   and it has no external verifier in this repository: there is no PDF library
   and no reader. So these tests check the two things a reader actually needs --
   an xref table whose offsets point at the objects it claims, and text encoded
   in bytes whose WinAnsiEncoding maps back to the glyphs that were written --
   plus the layout invariant that nothing is drawn outside the sheet. */

/* WinAnsi 0x80-0x9F are the code points that differ from Latin-1; a reader maps
   them to these characters, so a test decoding the output must do the same or a
   German quotation mark would be compared against a control character. */
const WINANSI_TO_TEXT = {
  0x80: "€", 0x82: "‚", 0x83: "ƒ", 0x84: "„", 0x85: "…", 0x86: "†", 0x87: "‡",
  0x88: "ˆ", 0x89: "‰", 0x8a: "Š", 0x8b: "‹", 0x8c: "Œ", 0x8e: "Ž", 0x91: "‘",
  0x92: "’", 0x93: "“", 0x94: "”", 0x95: "•", 0x96: "–", 0x97: "—", 0x98: "˜",
  0x99: "™", 0x9a: "š", 0x9b: "›", 0x9c: "œ", 0x9e: "ž", 0x9f: "Ÿ",
};

/* Bytes a page-oriented reader would reject because WinAnsiEncoding leaves them
   undefined. None of them may ever be produced. */
const UNDEFINED_WINANSI = [0x81, 0x8d, 0x8f, 0x90, 0x9d];

/* Acts as an independent reader: the bytes the writer produced are decoded here
   with a table written from the WinAnsiEncoding spec, not imported from the
   writer, so a wrong byte cannot hide behind a shared constant. Accepts both a
   string of latin1 characters and a Buffer, because the writer returns Buffer. */
const decodeWinAnsi = (text) => {
  const source = Buffer.isBuffer(text) ? text.toString("latin1") : String(text);
  return Array.from(source, (ch) => WINANSI_TO_TEXT[ch.charCodeAt(0)] ?? ch).join("");
};

/*
 * Reads an Info-dictionary string the way a reader does: UTF-16BE when it
 * carries a byte-order mark, PDFDocEncoding otherwise.
 *
 * A test that merely searched for the raw bytes of the title would pass while
 * the file was wrong. The Info dictionary has no font, so its strings are PDF
 * text strings and NOT WinAnsi: writing the WinAnsi bytes of an en dash put a
 * capital OE ligature in the file's title in every real reader. Decoding like a
 * reader is what catches that class of mistake.
 */
const infoString = (buffer, key) => {
  const match = new RegExp(`/${key} \\(([^)]*)\\)`).exec(buffer.toString("latin1"));
  assert.ok(match, `/${key} is present in the Info dictionary`);
  const raw = Buffer.from(match[1].replace(/\\([\\()])/g, "$1"), "latin1");
  if (raw[0] === 0xfe && raw[1] === 0xff) return raw.subarray(2).swap16().toString("utf16le");
  /* PDFDocEncoding agrees with Latin-1 across the printable range, so this
     fallback is enough to prove the ASCII cases. */
  return raw.toString("latin1");
};

const buildSample = () => {
  const doc = createPdf({ title: "BauLens PDF Writer", author: "BauLens" });
  doc.setFont("helvetica-bold").setSize(14);
  doc.drawText("Bauaufsichtlicher Vorprüfungsbefund", 56, 70);
  doc.setFont("helvetica").setSize(10);
  doc.drawParagraph("Die erforderliche Abstandsfläche beträgt 3,00 m zur Grenze.", 56, 100, { width: 300 });
  doc.line(56, 130, 539, 130, { color: "#334155", width: 0.8 });
  doc.rect(56, 140, 200, 40, { stroke: "#94a3b8", fill: "#f8fafc" });
  return doc;
};

test("pdf writer: a document is a well-formed PDF with a readable xref table", () => {
  assert.ok(Math.abs(A4.width - 595.28) < 0.01 && Math.abs(A4.height - 841.89) < 0.01, "DIN A4 in points");

  const buffer = buildSample().toBuffer();
  const text = buffer.toString("latin1");

  assert.ok(text.startsWith("%PDF-1.4"));
  assert.ok(text.trimEnd().endsWith("%%EOF"));
  assert.equal(infoString(buffer, "Title"), "BauLens PDF Writer", "the metadata travels with the file");
  assert.ok(text.includes("/BaseFont /Helvetica-Bold"));
  assert.ok(text.includes("/Encoding /WinAnsiEncoding"));

  const xref = inspectXref(buffer);
  assert.equal(xref.valid, true);
  assert.deepEqual(xref.problems, []);
  assert.ok(xref.entries > 1, "font, catalogue, pages, page, content and info objects");

  /* An untouched document still serialises: nothing may depend on a caller
     remembering to add a first page. */
  const empty = createPdf().toBuffer();
  assert.ok(empty.toString("latin1").startsWith("%PDF-1.4"));
  assert.equal(inspectXref(empty).valid, true);
});

test("pdf writer: metadata is a PDF text string, so non-ASCII survives a reader", () => {
  /* Regression: an en dash in the title used to be written as its WinAnsi byte
     0x96, which is "Œ" in PDFDocEncoding -- the encoding of Info strings. The
     file opened, and its title was wrong in every reader. */
  const doc = createPdf({ title: "Vorprüfung – Bauherr Müller GmbH & Co. KG", author: "BauLens" });
  doc.setFont("helvetica").setSize(10).drawText("Inhalt", 56, 70);
  const buffer = doc.toBuffer();

  assert.equal(infoString(buffer, "Title"), "Vorprüfung – Bauherr Müller GmbH & Co. KG");
  /* The BOM is what tells a reader to decode UTF-16BE; without it the bytes are
     read as PDFDocEncoding and the title is mojibake again. */
  const at = buffer.toString("latin1").indexOf("/Title (");
  assert.equal(buffer[at + 8], 0xfe, "the title carries a UTF-16BE BOM");
  assert.equal(buffer[at + 9], 0xff);
});

test("pdf writer: identical input produces identical bytes", () => {
  const first = buildSample().toBuffer();
  const second = buildSample().toBuffer();

  assert.equal(Buffer.compare(first, second), 0, "the writer must not leak a timestamp or a random id");
  assert.ok(first.equals(second));
  assert.ok(!first.toString("latin1").includes("CreationDate"), "no creation date, so no run-to-run drift");
});

test("pdf writer: German text becomes WinAnsi bytes a reader maps back", () => {
  assert.deepEqual(toWinAnsi("äöüß"), Buffer.from([0xe4, 0xf6, 0xfc, 0xdf]));
  assert.deepEqual(toWinAnsi("§"), Buffer.from([0xa7]));
  assert.deepEqual(toWinAnsi("€"), Buffer.from([0x80]));
  assert.deepEqual(toWinAnsi("„Zitat“"), Buffer.from([0x84, 0x5a, 0x69, 0x74, 0x61, 0x74, 0x93]));
  assert.deepEqual(toWinAnsi("–"), Buffer.from([0x96]));

  const german = "Größe 12 m² – „Abstandsfläche“ §6 MBO";
  assert.equal(decodeWinAnsi(toWinAnsi(german)), german, "every German glyph survives the round trip");

  /* Glyphs with no WinAnsi code point are spelled out instead of dropped, and
     anything else becomes "?" rather than throwing mid-render. */
  assert.equal(toWinAnsi("→").toString("latin1"), "->");
  assert.equal(toWinAnsi("← ↑").toString("latin1"), "<- ^");
  assert.equal(toWinAnsi("prüfen ✓").toString("latin1"), "prüfen ok");
  assert.equal(toWinAnsi("Haus 🏗").toString("latin1"), "Haus ?");
  assert.doesNotThrow(() => toWinAnsi("🏗️ ẞ Ǆ"));

  const nasty = "Größe 12 m² – „Abstandsfläche“ • € … 🏗 → ✓";
  for (const byte of toWinAnsi(nasty)) {
    assert.ok(
      !UNDEFINED_WINANSI.includes(byte),
      `byte 0x${byte.toString(16)} is undefined in WinAnsiEncoding`
    );
  }
});

test("pdf writer: metrics are Adobe's published values, one table per face", () => {
  /* These are the core-14 AFM values, not numbers this repository invented.
     Locking a few of them stops a plausible-looking edit of the tables from
     silently re-flowing every dossier ever produced: a wrong width is invisible
     in a PDF, it just moves text. */
  const at1000 = (text, font) => textWidth(text, { font, size: 1000 });

  assert.equal(at1000("A", "helvetica"), 667);
  assert.equal(at1000("M", "helvetica"), 833);
  assert.equal(at1000("i", "helvetica"), 222);
  assert.equal(at1000("0", "helvetica"), 556);
  assert.equal(at1000(" ", "helvetica"), 278);
  assert.equal(at1000("§", "helvetica"), 556);
  assert.equal(at1000("ß", "helvetica"), 611);
  assert.equal(at1000("ä", "helvetica"), 556);

  /* The characters that made a guessed fallback width fail: an em dash and an
     ellipsis are a full em wide, and under-measuring them by 400 units per
     glyph is how a finding message creeps over the page edge. */
  assert.equal(at1000("—", "helvetica"), 1000);
  assert.equal(at1000("…", "helvetica"), 1000);
  assert.equal(at1000("–", "helvetica"), 556);
  assert.equal(at1000("€", "helvetica"), 556);

  /* Bold is a real second table, not the regular one scaled: compared with
     Helvetica, Helvetica-Bold has a wider A and i and a *narrower* @. */
  assert.equal(at1000("A", "helvetica-bold"), 722);
  assert.equal(at1000("i", "helvetica-bold"), 278);
  assert.equal(at1000("@", "helvetica-bold"), 975);
  assert.ok(at1000("@", "helvetica-bold") < at1000("@", "helvetica"));

  /* Courier is monospaced: every glyph is 600 units wide, in both weights. */
  for (const font of ["courier", "courier-bold"]) {
    for (const glyph of ["i", "M", "0", " ", "§", "—"]) {
      assert.equal(at1000(glyph, font), 600, `${font} must be monospaced`);
    }
  }
});

test("pdf writer: a line is measured as it is drawn, not as it is written", () => {
  /* The encoding is not length-preserving: an arrow is spelled out as "->", and
     measuring the one-character source would under-report the drawn width by a
     glyph. Measuring the encoded bytes is what keeps right-alignment honest. */
  assert.equal(textWidth("→", { font: "helvetica", size: 1000 }), textWidth("->", { font: "helvetica", size: 1000 }));
  assert.equal(textWidth("← ↑", { font: "helvetica", size: 1000 }), textWidth("<- ^", { font: "helvetica", size: 1000 }));
  /* An unmappable character is drawn as "?", so it must also be measured as one. */
  assert.equal(textWidth("🏗", { font: "helvetica", size: 1000 }), textWidth("?", { font: "helvetica", size: 1000 }));
  /* Control characters are drawn as spaces; a raw 0x09 in a text string would be
     a byte no reader renders and a width the layout never accounted for. */
  assert.equal(textWidth("a\tb", { font: "helvetica", size: 1000 }), textWidth("a b", { font: "helvetica", size: 1000 }));
  assert.ok(!toWinAnsi("a\tb").includes(0x09));
});

test("pdf writer: width scales with size and wrapping respects the column", () => {
  const regular10 = textWidth("Abstandsfläche", { font: "helvetica", size: 10 });
  const regular20 = textWidth("Abstandsfläche", { font: "helvetica", size: 20 });
  assert.ok(Math.abs(regular20 - 2 * regular10) < 1e-9, "double the size, double the width");

  const bold10 = textWidth("Abstandsfläche", { font: "helvetica-bold", size: 10 });
  const bold20 = textWidth("Abstandsfläche", { font: "helvetica-bold", size: 20 });
  assert.ok(Math.abs(bold20 - 2 * bold10) < 1e-9);
  assert.ok(bold10 > regular10, "the bold face is wider than the regular one; over-estimating wraps early, not over the edge");

  assert.equal(textWidth("", { size: 10 }), 0);
  assert.throws(() => textWidth("x", { font: "comic-sans" }), /unknown font/);

  for (const font of ["helvetica", "helvetica-bold", "courier", "courier-bold", "italic"]) {
    assert.ok(FONT_NAMES.includes(font), `${font} must be usable`);
  }
  assert.equal(FONT_NAMES.length, 5);
  assert.throws(() => createPdf().setFont("comic-sans"), /unknown font/);

  const paragraph =
    "Die erforderliche Abstandsfläche beträgt 3,00 m zur nördlichen Grundstücksgrenze, " +
    "sofern die zulässige Grundflächenzahl nicht überschritten wird.";
  const column = 240;

  /* Precondition: every single word fits on its own. A word wider than the
     column is placed on its own line by design, so only the words that fit can
     be held to the column width. */
  for (const word of paragraph.split(/\s+/)) {
    assert.ok(textWidth(word, { font: "helvetica", size: 10 }) <= column, `word "${word}" must fit the column`);
  }

  for (const font of ["helvetica", "helvetica-bold", "courier"]) {
    const lines = wrapText(paragraph, column, { font, size: 10 });
    assert.ok(lines.length > 1, `${font} must wrap`);
    for (const line of lines) {
      assert.ok(
        textWidth(line, { font, size: 10 }) <= column + 1e-9,
        `${font}: "${line}" is wider than the column`
      );
    }
    assert.equal(lines.join(" "), paragraph, "wrapping must not lose or invent words");
  }

  /* Explicit line breaks are kept, and an empty paragraph is a single empty
     line rather than no line at all. */
  assert.deepEqual(wrapText("Zeile eins\nZeile zwei", 500, { size: 10 }), ["Zeile eins", "Zeile zwei"]);
  assert.deepEqual(wrapText("", 500, { size: 10 }), [""]);
});

test("pdf writer: the layout audit catches text drawn past the page edge", () => {
  const normal = createPdf();
  normal.setFont("helvetica").setSize(11);
  normal.drawText("Bauaufsichtlicher Vorprüfungsbefund", 56, 70);
  assert.equal(normal.audit().overflow, false);
  assert.equal(normal.audit().pages[0].overflow, false);
  assert.ok(normal.audit().pages[0].maxTextX <= A4.width);

  const offRight = createPdf();
  offRight.setSize(12);
  offRight.drawText("Text über den rechten Rand hinaus", A4.width - 20, 120);
  const rightAudit = offRight.audit();
  assert.equal(rightAudit.pages[0].overflow, true, "the page knows it overflowed");
  assert.equal(rightAudit.overflow, true, "and so does the document");
  assert.ok(rightAudit.pages[0].maxTextX > A4.width);

  const offBottom = createPdf();
  offBottom.setSize(10);
  offBottom.drawText("Fußzeile", 56, A4.height - 2);
  assert.equal(offBottom.audit().pages[0].overflow, true, "text below the bottom edge is an overflow too");
});

test("pdf writer: a multi-page document lists every page in the audit and the xref", () => {
  const doc = createPdf({ title: "Mehrseitig" });
  for (let page = 1; page <= 3; page++) {
    if (page > 1) doc.addPage();
    doc.setFont("helvetica").setSize(10);
    doc.drawText(`Seite ${page}`, 56, 60);
  }

  assert.equal(doc.pageCount, 3);
  const audit = doc.audit();
  assert.equal(audit.pages.length, 3);
  assert.deepEqual(audit.pages.map((page) => page.index), [1, 2, 3]);
  for (const page of audit.pages) {
    assert.ok(page.ops >= 1, `page ${page.index} drew nothing`);
    assert.equal(page.overflow, false);
    assert.equal(page.width, A4.width);
    assert.equal(page.height, A4.height);
  }

  const buffer = doc.toBuffer();
  assert.equal(inspectXref(buffer).valid, true);

  const text = buffer.toString("latin1");
  assert.equal((text.match(/\/Type \/Page\b/g) || []).length, 3, "one page object per page");
  assert.ok(text.includes("/Count 3"));
  for (let page = 1; page <= 3; page++) {
    assert.ok(text.includes(`(Seite ${page}) Tj`), `page ${page} content is in the file`);
  }
});

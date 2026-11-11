"use strict";

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { deflateRawSync, deflateSync } from "node:zlib";
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
import { pdfToText, PdfError } from "../src/pdfextract.js";
import { docxToText, DocxError } from "../src/docxextract.js";
import { extractText, detectFormat } from "../src/textio.js";
import { analyzeContract } from "../src/analyze.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const bin = join(root, "bin", "clauselens.js");

const KILLER =
  "Die Haftung des Anbieters ist ausgeschlossen. Der Kunde stellt den Anbieter von " +
  "allen Anspr\u00fcchen frei. Der Vertrag verl\u00e4ngert sich automatisch um weitere zw\u00f6lf Monate.";

function pdfEscape(s) {
  return s.replace(/\\/g, "\\\\").replace(/\(/g, "\\(").replace(/\)/g, "\\)");
}

function wrapLines(text, width = 72) {
  const words = text.split(" ");
  const lines = [];
  let cur = "";
  for (const w of words) {
    if ((cur + " " + w).trim().length > width) {
      lines.push(cur.trim());
      cur = w;
    } else {
      cur += ` ${w}`;
    }
  }
  if (cur.trim()) lines.push(cur.trim());
  return lines;
}

const CONTENT_LINES = wrapLines(KILLER);

function contentStream() {
  let y = 760;
  let out = "BT\n/F1 11 Tf\n14 TL\n50 800 Td\n";
  for (const l of CONTENT_LINES) {
    out += `(${pdfEscape(l)}) Tj\nT*\n`;
    y -= 14;
  }
  out += "ET\n";
  return out;
}

function minimalPdf(contentBuf, flate) {
  const streamData = flate ? deflateSync(Buffer.from(contentBuf, "latin1")) : Buffer.from(contentBuf, "latin1");
  const filterEntry = flate ? "/Filter /FlateDecode" : "";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    `<< /Length ${streamData.length} ${filterEntry} >>\nstream\n`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
  ];
  let out = "%PDF-1.4\n%\u00e2\u00e3\u00cf\u00d3\n";
  const offsets = [out.length];
  objects.forEach((obj, idx) => {
    if (idx === 3) {
      out += `4 0 obj\n${obj}`;
      out.write;
      out = out.slice(0, out.length);
      out += streamData.toString("latin1");
      out += "\nendstream\nendobj\n";
    } else {
      out += `${idx + 1} 0 obj\n${obj}\nendobj\n`;
    }
    offsets.push(out.length);
  });
  const xrefStart = out.length;
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (let i = 0; i < objects.length; i++) {
    out += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
  }
  out += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefStart}\n%%EOF\n`;
  return Buffer.from(out, "latin1");
}

test("PDF unkomprimiert: Extraktion liefert Klauseln", () => {
  const pdf = minimalPdf(contentStream(), false);
  const text = pdfToText(pdf);
  assert.match(text, /Haftung des Anbieters ist ausgeschlossen/);
});

test("PDF mit FlateDecode: Extraktion liefert Klauseln inkl. Umlaute", () => {
  const content = contentStream().replace("Anspruechen", "Anspr\u00fcchen");
  const pdf = minimalPdf(content, true);
  const text = pdfToText(pdf);
  assert.match(text, /verl\u00e4ngert sich\s+automatisch/);
  assert.match(text, /Anspr\u00fcken|Anspr\u00fcchen|Anspruechen/);
});

test("PDF-Analyse End-to-End: liability + auto_renewal erkannt", () => {
  const pdf = minimalPdf(contentStream(), true);
  const dir = mkdtempSync(join(tmpdir(), "clauselens-test-"));
  try {
    const p = join(dir, "vertrag.pdf");
    writeFileSync(p, pdf);
    const { text, format } = extractText(p);
    assert.equal(format, "pdf");
    const res = analyzeContract(text);
    const types = res.findings.map((f) => f.type);
    assert.ok(types.includes("liability"), `liability fehlt: ${types.join(",")}`);
    assert.ok(types.includes("auto_renewal"), `auto_renewal fehlt: ${types.join(",")}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("Verschluesseltes PDF -> klare Fehlermeldung", () => {
  const base = minimalPdf(contentStream(), false).toString("latin1");
  const withEncrypt = base.replace(
    "/Root 1 0 R",
    "/Encrypt 9 0 R /Root 1 0 R"
  );
  assert.throws(() => pdfToText(Buffer.from(withEncrypt, "latin1")), PdfError);
  assert.throws(
    () => pdfToText(Buffer.from(withEncrypt, "latin1")),
    /passwortgesch/
  );
});

test("Kein-PDF -> PdfError", () => {
  assert.throws(() => pdfToText(Buffer.from("Hallo Welt, kein PDF")), PdfError);
});

test("PDF mit TJ-Kerning-Arrays trennt Worte mit negativen Spacing-Werten", () => {
  const content = "BT /F1 11 Tf 50 800 Td [ (Die) -250 (Haftung) -300 (ist) -250 (ausgeschlossen) ] TJ ET";
  const pdf = minimalPdf(content, false);
  const text = pdfToText(pdf);
  assert.equal(text, "Die Haftung ist ausgeschlossen");
});

test("PDF mit Tm-Matrix-Positionierung trennt Zeilen sauber", () => {
  const content = "BT /F1 11 Tf 1 0 0 1 50 750 Tm (Klausel 1) Tj 1 0 0 1 50 720 Tm (Klausel 2) Tj ET";
  const pdf = minimalPdf(content, false);
  const text = pdfToText(pdf);
  assert.equal(text, "Klausel 1\nKlausel 2");
});

test("Scan-only PDF ohne Textstrom liefert praezise OCR-Fehlermeldung", () => {
  const imageStream = "<< /Type /XObject /Subtype /Image /Width 100 /Height 100 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode >>\nstream\nfakejpegdata\nendstream";
  const pdf = minimalPdf(imageStream, false);
  assert.throws(
    () => pdfToText(pdf),
    /PDF enthält nur Rasterbilder\/Scans|OCR/
  );
});

function u16(str) {
  return Buffer.from(str, "utf16le").swap16();
}

function crc32(buf) {
  let table = crc32.table;
  if (!table) {
    table = crc32.table = new Int32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c;
    }
  }
  let c = -1;
  for (let i = 0; i < buf.length; i++) c = table[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}

function dosDateTime() {
  return { time: 0x4800, date: 0x5c21 };
}

export function makeZip(members) {
  const parts = [];
  const central = [];
  let offset = 0;
  const { time, date } = dosDateTime();
  for (const [name, dataRaw] of members) {
    const nameBuf = Buffer.from(name, "utf8");
    const method = 8;
    const data = inflateRawStore(dataRaw, method);
    const localHeader = Buffer.alloc(30);
    localHeader.writeUInt32LE(0x04034b50, 0);
    localHeader.writeUInt16LE(20, 4);
    localHeader.writeUInt16LE(0x0800, 6);
    localHeader.writeUInt16LE(method, 8);
    localHeader.writeUInt16LE(time, 10);
    localHeader.writeUInt16LE(date, 12);
    localHeader.writeUInt32LE(crc32(data), 14);
    localHeader.writeUInt32LE(data.length, 18);
    localHeader.writeUInt32LE(dataRaw.length, 22);
    localHeader.writeUInt16LE(nameBuf.length, 26);
    localHeader.writeUInt16LE(0, 28);
    parts.push(localHeader, nameBuf, data);

    const cen = Buffer.alloc(46);
    cen.writeUInt32LE(0x02014b50, 0);
    cen.writeUInt16LE(20, 4);
    cen.writeUInt16LE(20, 6);
    cen.writeUInt16LE(0x0800, 8);
    cen.writeUInt16LE(method, 10);
    cen.writeUInt16LE(time, 12);
    cen.writeUInt16LE(date, 14);
    cen.writeUInt32LE(crc32(data), 16);
    cen.writeUInt32LE(data.length, 20);
    cen.writeUInt32LE(dataRaw.length, 24);
    cen.writeUInt16LE(nameBuf.length, 28);
    cen.writeUInt32LE(offset, 42);
    central.push(cen, nameBuf);
    offset += 30 + nameBuf.length + data.length;
  }
  const centralStart = offset;
  let centralSize = 0;
  for (const b of central) centralSize += b.length;
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(members.length, 8);
  eocd.writeUInt16LE(members.length, 10);
  eocd.writeUInt32LE(centralSize, 12);
  eocd.writeUInt32LE(centralStart, 16);
  return Buffer.concat([...parts, ...central, eocd]);
}

function inflateRawStore(raw, method) {
  return method === 8 ? deflateRawSync(raw) : raw;
}

function docxXml(paragraphs) {
  const body = paragraphs
    .map((p) => `<w:p><w:r><w:t xml:space="preserve">${p}</w:t></w:r></w:p>`)
    .join("");
  return (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' +
    '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">' +
    `<w:body>${body}</w:body></w:document>`
  );
}

const DOCX_MEMBERS = () => [
  ["[Content_Types].xml", Buffer.from('<?xml version="1.0"?><Types/>')],
  ["_rels/.rels", Buffer.from('<?xml version="1.0"?><Relationships/>')],
  ["word/document.xml", Buffer.from(docxXml(KILLER.split(". ")), "utf8")],
];

test("DOCX: Extraktion ueber selbstgebauten ZIP", () => {
  const zip = makeZip(DOCX_MEMBERS());
  const text = docxToText(zip);
  assert.match(text, /Haftung des Anbieters ist ausgeschlossen/);
  assert.match(text, /verl\u00e4ngert sich/);
});

test("DOCX Umlaute + Entities korrekt dekodiert", () => {
  const zip = makeZip([
    [
      "word/document.xml",
      Buffer.from(docxXml(["Vertragsstrafe in H\u00f6he von 2 Monatsmieten", "&lt;Test&gt; &amp; Co."]), "utf8"),
    ],
  ]);
  const text = docxToText(zip);
  assert.match(text, /Vertragsstrafe in H\u00f6he/);
  assert.ok(text.includes("<Test> & Co."));
});

test("Kein ZIP -> DocxError", () => {
  assert.throws(() => docxToText(Buffer.from("gar kein zip")), DocxError);
});

test("detectFormat snifft PDF und DOCX ohne Extension", () => {
  assert.equal(detectFormat(minimalPdf(contentStream(), true), "datei.bin"), "pdf");
  assert.equal(detectFormat(makeZip(DOCX_MEMBERS()), "datei.bin"), "docx");
});

test("CLI Smoke: analyse einer PDF-Datei endet mit Exit 0 und Befunden", () => {
  const dir = mkdtempSync(join(tmpdir(), "clauselens-cli-"));
  try {
    const pdfPath = join(dir, "agb.pdf");
    const jsonPath = join(dir, "out.json");
    writeFileSync(pdfPath, minimalPdf(contentStream(), true));
    execFileSync(process.execPath, [bin, "analyze", pdfPath, "--json", jsonPath], {
      encoding: "utf8",
      env: { ...process.env, CLAUSELENS_TEST_ACCOUNT: "1" },
    });
    const json = JSON.parse(readFileSync(jsonPath, "utf8"));
    assert.ok(json.riskScore > 0);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

"use strict";
import { deflateRawSync, deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const dir = join(here, "fixtures");
mkdirSync(dir, { recursive: true });

const L1 = "Die Haftung des Anbieters für Datenverlust ist ausgeschlossen.";
const SOL = "Ansprüche des Kunden verjähren innerhalb von sechs Monaten.";
const JU = "Ausschließlicher Gerichtsstand für alle Streitigkeiten ist der Sitz des Anbieters.";
const NP = "Der Anbieter kann mit einer Frist von drei Monaten kündigen, der Kunde erst mit einer Vorlaufzeit von 36 Monaten.";

function pdfLiteral(s) {
  let out = "";
  for (const ch of s) {
    const cp = ch.codePointAt(0);
    if (ch === "(" || ch === ")" || ch === "\\") out += "\\" + ch;
    else if (cp < 128) out += ch;
    else if (cp <= 255) out += "\\" + cp.toString(8).padStart(3, "0");
    else out += "?";
  }
  return out;
}

// UTF-16BE payload as PDF octal escapes, tested through the BOM path of unescapeLiteral.
function utf16beLiteral(s) {
  let out = "\\376\\377";
  for (let i = 0; i < s.length; i++) {
    const u = s.charCodeAt(i);
    out += "\\" + (u >> 8).toString(8).padStart(3, "0") + "\\" + (u & 0xff).toString(8).padStart(3, "0");
  }
  return out;
}

function utf16beHex(s) {
  let out = "FEFF";
  for (let i = 0; i < s.length; i++) out += s.charCodeAt(i).toString(16).padStart(4, "0").toUpperCase();
  return out;
}

function stream(dictExtra, data) {
  const body = Buffer.isBuffer(data) ? data : Buffer.from(data, "latin1");
  const head = Buffer.from("<< /Length " + body.length + (dictExtra ? " " + dictExtra : "") + " >>\nstream\n", "latin1");
  return Buffer.concat([head, body, Buffer.from("\nendstream\n", "latin1")]);
}

function pdf(streams, trailerExtra) {
  return Buffer.concat([
    Buffer.from("%PDF-1.7\n%\xe2\xe3\xcf\xd3\n", "latin1"),
    ...streams,
    Buffer.from("trailer\n<< /Root 1 0 R /Size 9" + (trailerExtra ? " " + trailerExtra : "") + " >>\n%%EOF\n", "latin1"),
  ]);
}

function show(clause) {
  return "BT /F1 12 Tf 72 720 Td (" + pdfLiteral(clause) + ") Tj ET";
}

const CRC_TABLE = (() => {
  const t = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c;
  }
  return t;
})();

function crc32(buf) {
  let c = -1;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}

function zip(entries) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const e of entries) {
    const name = Buffer.from(e.name, "utf8");
    const raw = Buffer.from(e.data);
    const method = e.method === undefined ? 0 : e.method;
    const stored = method === 8 ? deflateRawSync(raw) : raw;
    const crc = crc32(raw);
    const lh = Buffer.alloc(30);
    lh.writeUInt32LE(0x04034b50, 0);
    lh.writeUInt16LE(20, 4);
    lh.writeUInt16LE(method, 8);
    lh.writeUInt32LE(crc, 14);
    lh.writeUInt32LE(stored.length, 18);
    lh.writeUInt32LE(raw.length, 22);
    lh.writeUInt16LE(name.length, 26);
    const ch = Buffer.alloc(46);
    ch.writeUInt32LE(0x02014b50, 0);
    ch.writeUInt16LE(20, 4);
    ch.writeUInt16LE(20, 6);
    ch.writeUInt16LE(method, 10);
    ch.writeUInt32LE(crc, 16);
    ch.writeUInt32LE(stored.length, 20);
    ch.writeUInt32LE(raw.length, 24);
    ch.writeUInt16LE(name.length, 28);
    ch.writeUInt32LE(offset, 42);
    locals.push(lh, name, stored);
    centrals.push(ch, name);
    offset += lh.length + name.length + stored.length;
  }
  const cd = Buffer.concat(centrals);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(entries.length, 8);
  eocd.writeUInt16LE(entries.length, 10);
  eocd.writeUInt32LE(cd.length, 12);
  eocd.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, cd, eocd]);
}

const XML_HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' + "\n" +
  '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>';
const XML_TAIL = "</w:body></w:document>";
const CONTENT_TYPES = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' + "\n" +
  '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>';

function docx(parasXml, method) {
  return zip([
    { name: "[Content_Types].xml", data: CONTENT_TYPES, method: 0 },
    { name: "word/document.xml", data: XML_HEAD + parasXml + XML_TAIL, method: method === undefined ? 0 : method },
  ]);
}

const np = (s) => "<w:p><w:r><w:t>" + s + "</w:t></w:r></w:p>";
const utf8 = (s) => Buffer.from(s, "utf8");

const CASES = [];
function add(c) { CASES.push(c); }

const flateContent = utf8(show(L1));
const flateData = deflateSync(flateContent);

writeFileSync(join(dir, "flate.pdf"), pdf([stream("/Filter /FlateDecode", flateData)]));
add({ id: "pdf-flate-literal", file: "fixtures/flate.pdf", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "FlateDecode stream inflated through inflateSync" });

writeFileSync(join(dir, "tj-kerning.pdf"), pdf([stream("", utf8("BT /F1 12 Tf 1 0 0 1 72 720 Tm [(Die) -250 (Haftung) -300 (des) -250 (Anbieters) -250 (f\\374r) -260 (Datenverlust) -250 (ist) -250 (ausgeschlossen.)] TJ ET"))]));
add({ id: "pdf-tj-kerning", file: "fixtures/tj-kerning.pdf", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "TJ array, kerning offsets <= -120 become word spaces" });

writeFileSync(join(dir, "winansi.pdf"), pdf([stream("", utf8("BT /F1 8 Tf 72 720 Td (" + pdfLiteral(SOL) + ") Tj ET"))]));
add({ id: "pdf-winansi-octal", file: "fixtures/winansi.pdf", format: "auto", expectedText: SOL, expectedTypes: ["statute_of_limitations"], note: "WinAnsi umlauts as octal escapes, latin1 decode" });

writeFileSync(join(dir, "utf16be.pdf"), pdf([stream("", utf8("BT /F1 12 Tf 72 720 Td (" + utf16beLiteral(L1) + ") Tj T* <" + utf16beHex(JU) + "> Tj ET"))]));
add({ id: "pdf-utf16be-strings", file: "fixtures/utf16be.pdf", format: "auto", expectedText: L1 + "\n" + JU, expectedTypes: ["liability", "jurisdiction"], note: "literal and hex strings with UTF-16BE BOM" });

writeFileSync(join(dir, "multipage.pdf"), pdf([
  stream("", utf8(show(L1))),
  stream("/Filter /FlateDecode", deflateSync(utf8(show(JU)))),
]));
add({ id: "pdf-multipage", file: "fixtures/multipage.pdf", format: "auto", expectedText: L1 + "\n\n" + JU, expectedTypes: ["liability", "jurisdiction"], note: "two content streams joined with a blank line" });

const wrapContent = "BT /F1 12 Tf 72 720 Td (Die Haf-) Tj T* (tung des Anbieters f\\374r Datenverlust ist ausge-) Tj T* (schlossen.) Tj ET";
writeFileSync(join(dir, "linewrap.pdf"), pdf([stream("", utf8(wrapContent))]));
add({ id: "pdf-linewrap-dehyphenation", file: "fixtures/linewrap.pdf", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "Td/T*/Td line breaks and hyphen join at line end" });

const hidden = "BT /F1 12 Tf 72 720 Td (Haftung des Anbieters: unbeschr\\344nkt) Tj ET " +
  "BT 3 Tr /F1 12 Tf 72 700 Td (" + pdfLiteral(L1) + ") Tj ET";
writeFileSync(join(dir, "hidden-text.pdf"), pdf([stream("", utf8(hidden))]));
add({ id: "pdf-hidden-text-contradiction", file: "fixtures/hidden-text.pdf", format: "auto", expectedText: "Haftung des Anbieters: unbeschränkt\n" + L1, expectedTypes: ["liability"], defect: "invisible text (render mode 3) is extracted like visible text and the visible label contradicts the text layer; extraction cannot tell them apart", note: "visible label plus hidden clause in the text layer" });

const flateTrailing = Buffer.concat([flateData, Buffer.from([0x0d])]);
writeFileSync(join(dir, "flate-trailing.pdf"), pdf([stream("/Filter /FlateDecode", flateTrailing)]));
add({ id: "pdf-flate-trailing-byte", file: "fixtures/flate-trailing.pdf", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "trailing CR before endstream, inflateData falls back to dropping the last byte" });

writeFileSync(join(dir, "flate-broken.pdf"), pdf([stream("/Filter /FlateDecode", flateData.subarray(0, flateData.length - 9))]));
add({ id: "pdf-flate-corrupt", file: "fixtures/flate-broken.pdf", format: "auto", expectedErrorIncludes: "Kein Text im PDF gefunden", expectedErrorClass: "PdfError", note: "corrupt FlateDecode stream yields the no-text diagnosis" });

writeFileSync(join(dir, "scanned.pdf"), pdf([stream("/Type /XObject /Subtype /Image /Width 8 /Height 8 /Filter /DCTDecode", Buffer.from([0xff, 0xd8, 0xff, 0xd9]))]));
add({ id: "pdf-scanned-image-only", file: "fixtures/scanned.pdf", format: "auto", expectedError: "PDF enthält nur Rasterbilder/Scans (Bildseiten) und keine durchsuchbare Textebene. OCR-Texterkennung ist für gescannte PDFs erforderlich.", expectedErrorClass: "PdfError", note: "image-only PDF gets the scan diagnosis" });

writeFileSync(join(dir, "encrypted.pdf"), pdf([], "/Encrypt 5 0 R"));
add({ id: "pdf-encrypted", file: "fixtures/encrypted.pdf", format: "auto", expectedError: "PDF ist passwortgeschützt (verschlüsselt) und kann nicht gelesen werden.", expectedErrorClass: "PdfError", note: "encryption dictionary is detected before parsing" });

writeFileSync(join(dir, "no-endstream.pdf"), Buffer.concat([
  utf8("%PDF-1.7\n"),
  Buffer.from("<< /Length 60 >>\nstream\n", "latin1"),
  utf8(show(L1)),
]));
add({ id: "pdf-truncated-no-endstream", file: "fixtures/no-endstream.pdf", format: "auto", expectedErrorIncludes: "Kein Text im PDF gefunden", expectedErrorClass: "PdfError", note: "missing endstream yields the no-text diagnosis" });

writeFileSync(join(dir, "not-pdf.pdf"), utf8("Dies ist kein PDF, sondern eine Textdatei mit .pdf-Endung.\n"));
add({ id: "pdf-missing-signature", file: "fixtures/not-pdf.pdf", format: "auto", expectedError: "Datei ist kein PDF (fehlende %PDF-Signatur).", expectedErrorClass: "PdfError", note: "extension says pdf, bytes do not" });

const cmap = show(L1) + "\nbeginbfchar\n<0001> <0041>\nendbfchar\n";
writeFileSync(join(dir, "cmap.pdf"), pdf([stream("", utf8(cmap))]));
add({ id: "pdf-cmap-stream-skipped", file: "fixtures/cmap.pdf", format: "auto", expectedContains: [L1], expectedTypes: ["liability"], note: "ToUnicode CMap content stream retains text layer" });

writeFileSync(join(dir, "stored.docx"), docx(
  np("Die Haftung des Anbieters f&#252;r Datenverlust ist ausgeschlossen &amp; bleibt bestehen.") +
  np("Der Kunde tr&#228;gt<w:tab/>die Beweislast f&#xFC;r die Vollst&#228;ndigkeit aller Angaben."),
  0,
));
add({ id: "docx-stored-entities", file: "fixtures/stored.docx", format: "auto", expectedText: "Die Haftung des Anbieters für Datenverlust ist ausgeschlossen & bleibt bestehen.\nDer Kunde trägt\tdie Beweislast für die Vollständigkeit aller Angaben.", expectedTypes: ["liability"], note: "stored ZIP entry, decimal and hex entities, w:tab" });

writeFileSync(join(dir, "deflate.docx"), docx(np(NP), 8));
add({ id: "docx-deflate", file: "fixtures/deflate.docx", format: "auto", expectedText: NP, expectedTypes: ["notice_period"], note: "deflated document.xml" });

writeFileSync(join(dir, "deleted-text.docx"), docx(
  np("Die Parteien arbeiten vertrauensvoll zusammen.") +
  "<w:p><w:del><w:r><w:delText>" + L1 + "</w:delText></w:r></w:del></w:p>",
  0,
));
add({ id: "docx-deleted-text-included", file: "fixtures/deleted-text.docx", format: "auto", expectedText: "Die Parteien arbeiten vertrauensvoll zusammen.", expectedTypes: [], expectedNotContains: [L1], note: "tracked deletions (w:delText) are ignored" });

writeFileSync(join(dir, "footnote-member.docx"), zip([
  { name: "[Content_Types].xml", data: CONTENT_TYPES, method: 0 },
  { name: "word/document.xml", data: XML_HEAD + np("Die Parteien regeln ihre Zusammenarbeit in dieser Vereinbarung.") + XML_TAIL, method: 0 },
  { name: "word/footnotes.xml", data: XML_HEAD + np(L1) + XML_TAIL, method: 8 },
]));
add({ id: "docx-footnote-member-ignored", file: "fixtures/footnote-member.docx", format: "auto", expectedContains: ["Die Parteien regeln ihre Zusammenarbeit in dieser Vereinbarung.", L1], expectedTypes: ["liability"], note: "clauses in word/footnotes.xml are extracted and analysed" });

writeFileSync(join(dir, "no-eocd.docx"), Buffer.concat([Buffer.from("PK\x03\x04", "latin1"), Buffer.alloc(40, 0x41)]));
add({ id: "docx-no-eocd", file: "fixtures/no-eocd.docx", format: "auto", expectedError: "Kein gültiger ZIP-Abschluss gefunden.", expectedErrorClass: "DocxError", note: "PK header without end of central directory" });

writeFileSync(join(dir, "missing-member.docx"), zip([{ name: "word/other.xml", data: utf8(XML_HEAD + XML_TAIL), method: 0 }]));
add({ id: "docx-missing-member", file: "fixtures/missing-member.docx", format: "auto", expectedError: "word/document.xml nicht im Dokument gefunden.", expectedErrorClass: "DocxError", note: "valid ZIP without the main document part" });

writeFileSync(join(dir, "bad-method.docx"), zip([{ name: "word/document.xml", data: utf8(XML_HEAD + np(L1) + XML_TAIL), method: 12 }]));
add({ id: "docx-unsupported-method", file: "fixtures/bad-method.docx", format: "auto", expectedError: "Nicht unterstützte ZIP-Kompression (Methode 12).", expectedErrorClass: "DocxError", note: "compression method other than store/deflate" });

writeFileSync(join(dir, "not-zip.docx"), utf8("Dies ist kein ZIP-Container, sondern Text mit .docx-Endung.\n"));
add({ id: "docx-missing-pk-signature", file: "fixtures/not-zip.docx", format: "auto", expectedError: "Datei ist kein DOCX/ZIP (fehlende PK-Signatur).", expectedErrorClass: "DocxError", note: "extension says docx, bytes are not a ZIP" });

let huge = "";
while (huge.length < 120000) huge += L1 + " ";
writeFileSync(join(dir, "huge.txt"), utf8(huge));
add({ id: "txt-huge-single-line", file: "fixtures/huge.txt", format: "auto", expectedContains: [L1], expectedLengthMin: 120000, expectedTypes: ["liability"], note: "single line above 120k characters" });

writeFileSync(join(dir, "noise.txt"), utf8("Die Haf\u00adtung des\u00a0Anbieters f\u200bür Daten\u200dverlust ist ausgeschlossen."));
add({ id: "txt-nbsp-zero-width", file: "fixtures/noise.txt", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "soft hyphen, NBSP, zero width space and joiner" });

const utf16le = Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from(L1, "utf16le")]);
writeFileSync(join(dir, "utf16le.txt"), utf16le);
add({ id: "txt-utf16le-bom", file: "fixtures/utf16le.txt", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "UTF-16LE with BOM" });

const mojibake = Buffer.from(L1, "utf8").toString("latin1");
writeFileSync(join(dir, "mojibake.txt"), Buffer.from(mojibake, "latin1"));
add({ id: "txt-mojibake", file: "fixtures/mojibake.txt", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "UTF-8 read as latin1, repaired by repairMojibake" });

writeFileSync(join(dir, "translit.txt"), utf8("Die Haftung des Anbieters fuer Datenverlust ist ausgeschlossen."));
add({ id: "txt-transliteration", file: "fixtures/translit.txt", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "ASCII transliteration fuer becomes für" });

writeFileSync(join(dir, "wrap.txt"), utf8("Die Haf-\ntung des Anbieters für Datenverlust ist ausgeschlossen."));
add({ id: "txt-linewrap-dehyphenation", file: "fixtures/wrap.txt", format: "auto", expectedText: L1, expectedTypes: ["liability"], note: "hyphen at line end join after normalizeText" });

const calibration = [
  { id: "pdf-flate-literal", first: "expected text, got PdfError (no text)", actual: "L1 after fixing the fixture", classification: "bad fixture: PDF stream used raw deflate, src/pdfextract.js inflateSync needs a zlib wrapper (deflateSync)" },
  { id: "pdf-multipage", first: "expected L1 + blank line + JU, got L1 only", actual: "L1 + blank line + JU after fixing the fixture", classification: "bad fixture: second stream used raw deflate" },
  { id: "pdf-flate-truncated-one-byte", first: "expected recovery from a shortened stream, got the no-text error", actual: "recovery from a trailing CR byte", classification: "bad fixture: inflateData drops the last byte, so the trigger is an extra trailing byte, not a missing one" },
  { id: "pdf-tj-kerning", first: "L1 including the final period, got the string without it", actual: "period added inside the TJ literal", classification: "bad expectation: the fixture literal was missing the period" },
  { id: "txt-nbsp-zero-width", first: "L1, got fr where fuer was intended", actual: "L1 after adding the missing umlaut", classification: "bad fixture: typo dropped the umlaut" },
];
const out = {
  generatedBy: "build.mjs",
  cases: CASES,
  calibration,
};
writeFileSync(join(here, "cases.json"), JSON.stringify(out, null, 2) + "\n");
console.log("fixtures: " + CASES.length + " cases written to " + dir);

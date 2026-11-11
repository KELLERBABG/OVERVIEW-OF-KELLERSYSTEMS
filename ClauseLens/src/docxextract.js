"use strict";

import { inflateRawSync } from "node:zlib";

export class DocxError extends Error {}

function findEocd(buf) {
  for (let i = buf.length - 22; i >= Math.max(0, buf.length - 65558); i--) {
    if (buf.readUInt32LE(i) === 0x06054b50) return i;
  }
  return -1;
}

function readCentralDirectory(buf, eocdOffset) {
  const count = buf.readUInt16LE(eocdOffset + 10);
  let offset = buf.readUInt32LE(eocdOffset + 16);
  const entries = [];
  for (let n = 0; n < count; n++) {
    if (buf.readUInt32LE(offset) !== 0x02014b50) break;
    const method = buf.readUInt16LE(offset + 10);
    const compressedSize = buf.readUInt32LE(offset + 20);
    const nameLen = buf.readUInt16LE(offset + 28);
    const extraLen = buf.readUInt16LE(offset + 30);
    const commentLen = buf.readUInt16LE(offset + 32);
    const localOffset = buf.readUInt32LE(offset + 42);
    const name = buf.slice(offset + 46, offset + 46 + nameLen).toString("utf8");
    entries.push({ name, method, compressedSize, localOffset });
    offset += 46 + nameLen + extraLen + commentLen;
  }
  return entries;
}

function readEntry(buf, entry) {
  const lo = entry.localOffset;
  if (buf.readUInt32LE(lo) !== 0x04034b50) throw new DocxError("Ung\u00fcltiges ZIP (lokaler Header fehlt).");
  const nameLen = buf.readUInt16LE(lo + 26);
  const extraLen = buf.readUInt16LE(lo + 28);
  const dataStart = lo + 30 + nameLen + extraLen;
  const data = buf.slice(dataStart, dataStart + entry.compressedSize);
  if (entry.method === 0) return Buffer.from(data);
  if (entry.method === 8) {
    try {
      return inflateRawSync(data);
    } catch {
      throw new DocxError("DOCX-Teil konnte nicht dekomprimiert werden.");
    }
  }
  throw new DocxError(`Nicht unterst\u00fctzte ZIP-Kompression (Methode ${entry.method}).`);
}

export function extractDocxXml(buf, memberName) {
  if (!(buf instanceof Buffer)) buf = Buffer.from(buf);
  if (buf.readUInt32LE(0) !== 0x04034b50 && buf.subarray(0, 2).toString() !== "PK") {
    throw new DocxError("Datei ist kein DOCX/ZIP (fehlende PK-Signatur).");
  }
  const eocd = findEocd(buf);
  if (eocd === -1) throw new DocxError("Kein g\u00fcltiger ZIP-Abschluss gefunden.");
  const entries = readCentralDirectory(buf, eocd);
  const entry = entries.find((e) => e.name === memberName);
  if (!entry) throw new DocxError(`${memberName} nicht im Dokument gefunden.`);
  return readEntry(buf, entry);
}

const ENTITY_MAP = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'" };

function decodeEntities(s) {
  return s.replace(/&(#x?[0-9a-fA-F]+|\w+);/g, (m, body) => {
    if (body.startsWith("#x") || body.startsWith("#X")) {
      return String.fromCodePoint(parseInt(body.slice(2), 16));
    }
    if (body.startsWith("#")) {
      return String.fromCodePoint(parseInt(body.slice(1), 10));
    }
    return ENTITY_MAP[body.toLowerCase()] ?? m;
  });
}

export function docxXmlToText(xml) {
  let text = xml;
  text = text.replace(/<w:del\b[^>]*>[\s\S]*?<\/w:del>/gi, "");
  text = text.replace(/<w:delText\b[^>]*>[\s\S]*?<\/w:delText>/gi, "");
  text = text.replace(/<w:tab\b[^>]*\/?>/g, "\t");
  text = text.replace(/<w:br\b[^>]*\/?>/g, "\n");
  text = text.replace(/<\/w:p>/g, "\n");
  text = text.replace(/<[^>]+>/g, "");
  text = decodeEntities(text);
  return text
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .replace(/[ \t]{2,}/g, " ")
    .trim();
}

export function docxToText(buf) {
  if (!(buf instanceof Buffer)) buf = Buffer.from(buf);
  if (buf.readUInt32LE(0) !== 0x04034b50 && buf.subarray(0, 2).toString() !== "PK") {
    throw new DocxError("Datei ist kein DOCX/ZIP (fehlende PK-Signatur).");
  }
  const eocd = findEocd(buf);
  if (eocd === -1) throw new DocxError("Kein gültiger ZIP-Abschluss gefunden.");
  const entries = readCentralDirectory(buf, eocd);
  const docEntry = entries.find((e) => e.name === "word/document.xml");
  if (!docEntry) throw new DocxError("word/document.xml nicht im Dokument gefunden.");

  const parts = [];
  const mainText = docxXmlToText(readEntry(buf, docEntry).toString("utf8"));
  if (mainText) parts.push(mainText);

  // Auxiliaries: footnotes, endnotes, headers, footers
  const auxNames = ["word/footnotes.xml", "word/endnotes.xml"];
  for (const entry of entries) {
    if (/^word\/(?:header|footer)\d*\.xml$/i.test(entry.name)) {
      auxNames.push(entry.name);
    }
  }

  for (const name of auxNames) {
    const entry = entries.find((e) => e.name === name);
    if (!entry) continue;
    try {
      const auxText = docxXmlToText(readEntry(buf, entry).toString("utf8"));
      if (auxText && auxText.trim().length > 0) {
        parts.push(auxText.trim());
      }
    } catch {
      // ignore broken auxiliary parts
    }
  }

  return parts.join("\n\n");
}

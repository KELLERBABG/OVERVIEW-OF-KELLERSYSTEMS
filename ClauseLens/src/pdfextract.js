"use strict";

import { inflateSync } from "node:zlib";

export class PdfError extends Error {}

function findStreamChunks(buf) {
  const chunks = [];
  const latin = buf.toString("latin1");
  const re = /(<<[\s\S]{0,4000}?)>>\s*stream(\r\n|\n)/g;
  let m;
  while ((m = re.exec(latin))) {
    const dictEnd = m.index + m[1].length + 2;
    const dataStart = m.index + m[0].length;
    const endIdx = latin.indexOf("endstream", dataStart);
    if (endIdx === -1) break;
    chunks.push({
      dict: latin.slice(m.index, dictEnd),
      data: buf.subarray(dataStart, endIdx),
    });
    re.lastIndex = endIdx;
  }
  return chunks;
}

function inflateData(data) {
  try {
    return inflateSync(data);
  } catch {
    try {
      return inflateSync(data.subarray(0, data.length - 1));
    } catch {
      return null;
    }
  }
}

function isEncrypted(buf) {
  const latin = buf.toString("latin1");
  return /\/Encrypt\s+\d+\s+\d+\s+R/.test(latin);
}

function unescapeLiteral(s) {
  const bytes = [];
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch === "\\") {
      const n = s[++i];
      if (n === undefined) break;
      if (n >= "0" && n <= "7") {
        let oct = n;
        while (oct.length < 3 && s[i + 1] >= "0" && s[i + 1] <= "7") oct += s[++i];
        bytes.push(parseInt(oct, 8));
        continue;
      }
      const map = { n: "\n", r: "\r", t: "\t", b: "\b", f: "\f" };
      if (map[n] !== undefined) {
        bytes.push(map[n].codePointAt(0));
        continue;
      }
      if (n === "\r") {
        if (s[i + 1] === "\n") i++;
        continue;
      }
      if (n === "\n") continue;
      bytes.push(n.charCodeAt(0));
      continue;
    }
    bytes.push(ch.codePointAt(0) & 0xff);
  }

  // UTF-16BE BOM check
  if (bytes.length >= 2 && bytes[0] === 0xfe && bytes[1] === 0xff) {
    try {
      return Buffer.from(bytes.slice(2)).swap16().toString("utf16le");
    } catch {
      // Fallback to latin1
    }
  }

  let text = Buffer.from(bytes).toString("latin1");
  if (text.includes("\\") && /\\"|\\\)|\\\[/.test(text)) {
    text = text.replace(/\\(["'()\[\]\\])/g, "$1");
  }
  return text;
}

function decodeHexString(hex) {
  const clean = hex.replace(/[^0-9a-fA-F]/g, "");
  const padded = clean.length % 2 === 1 ? `${clean}0` : clean;
  const bytes = Buffer.from(padded, "hex");
  if (bytes.length >= 2 && bytes[0] === 0xfe && bytes[1] === 0xff) {
    return Buffer.from(bytes.subarray(2)).swap16().toString("utf16le");
  }
  return bytes.toString("latin1");
}

const SHOW_OPERATORS = /(?:Tj|TJ|'|")/;

function extractFromContent(content) {
  const text = content.toString("latin1");
  const out = [];
  let i = 0;
  let pendingBreak = false;

  const pushText = (t) => {
    if (!t) return;
    if (pendingBreak) {
      if (out.length > 0 && !out[out.length - 1].endsWith("\n")) {
        out.push("\n");
      }
      pendingBreak = false;
    }
    out.push(t);
  };

  while (i < text.length) {
    const ch = text[i];

    // Array operator for TJ kerning: [ (Hello) -250 (World) ] TJ
    if (ch === "[") {
      i++;
      while (i < text.length && text[i] !== "]") {
        const c = text[i];
        if (c === "(") {
          let depth = 1;
          let j = i + 1;
          const raw = [];
          while (j < text.length && depth > 0) {
            const ch2 = text[j];
            if (ch2 === "\\") {
              raw.push(ch2, text[j + 1] ?? "");
              j += 2;
              continue;
            }
            if (ch2 === "(") depth++;
            else if (ch2 === ")") {
              depth--;
              if (depth === 0) break;
            }
            raw.push(ch2);
            j++;
          }
          pushText(unescapeLiteral(raw.join("")));
          i = j + 1;
          continue;
        }

        if (c === "<" && text[i + 1] !== "<") {
          const close = text.indexOf(">", i);
          if (close !== -1) {
            pushText(decodeHexString(text.slice(i + 1, close)));
            i = close + 1;
            continue;
          }
        }

        // Numeric kerning offset: negative values denote inter-word spacing
        if (c === "-" || (c >= "0" && c <= "9")) {
          let numStr = c;
          let j = i + 1;
          while (j < text.length && ((text[j] >= "0" && text[j] <= "9") || text[j] === ".")) {
            numStr += text[j];
            j++;
          }
          const num = parseFloat(numStr);
          if (!isNaN(num) && num <= -120) {
            if (out.length > 0 && !out[out.length - 1].endsWith(" ")) {
              out.push(" ");
            }
          }
          i = j;
          continue;
        }

        i++;
      }
      if (i < text.length && text[i] === "]") {
        i++;
      }
      continue;
    }

    if (ch === "(") {
      let depth = 1;
      let j = i + 1;
      const raw = [];
      while (j < text.length && depth > 0) {
        const c = text[j];
        if (c === "\\") {
          raw.push(c, text[j + 1] ?? "");
          j += 2;
          continue;
        }
        if (c === "(") depth++;
        else if (c === ")") {
          depth--;
          if (depth === 0) break;
        }
        raw.push(c);
        j++;
      }
      pushText(unescapeLiteral(raw.join("")));
      i = j + 1;
      continue;
    }

    if (ch === "<" && text[i + 1] !== "<") {
      const close = text.indexOf(">", i);
      if (close !== -1) {
        pushText(decodeHexString(text.slice(i + 1, close)));
        i = close + 1;
        continue;
      }
    }

    if (ch === "T" || ch === "'" || ch === '"') {
      const rest = text.slice(i, i + 3);
      // Support Td, TD, T* and text matrix Tm for proper line breaks
      if (/^T[dD*m](?:\s|$)/.test(rest)) {
        pendingBreak = true;
        i += 2;
        continue;
      }
      if (/^ET(?:\s|$)/.test(rest)) {
        pendingBreak = true;
        i += 2;
        continue;
      }
      if (SHOW_OPERATORS.test(ch) && (i === 0 || /\s/.test(text[i - 1]) || ")]>".includes(text[i - 1]))) {
        if (ch === "'" || ch === '"') pendingBreak = true;
        i++;
        continue;
      }
    }

    i++;
  }

  return out
    .join("")
    .replace(/\u00ad/g, "") // soft-hyphen
    .replace(/\u00a0/g, " ") // non-breaking space
    .replace(/\u200b/g, "") // zero-width space
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .replace(/-\n(?=[a-z\u00e4\u00f6\u00fc])/g, "")
    .replace(/[ \t]{2,}/g, " ")
    .trim();
}

export function pdfToText(buf) {
  if (!(buf instanceof Buffer)) buf = Buffer.from(buf);
  const head = buf.subarray(0, 1024).toString("latin1");
  if (!head.startsWith("%PDF-") && !head.includes("%PDF-")) {
    throw new PdfError("Datei ist kein PDF (fehlende %PDF-Signatur).");
  }
  if (isEncrypted(buf)) {
    throw new PdfError("PDF ist passwortgesch\u00fctzt (verschl\u00fcsselt) und kann nicht gelesen werden.");
  }

  const parts = [];
  let foundImageOnly = false;
  let totalStreams = 0;

  for (const chunk of findStreamChunks(buf)) {
    totalStreams++;
    const dict = chunk.dict;
    if (/\/Subtype\s*\/Image|\/DCTDecode|\/JPXDecode|\/CCITTFaxDecode/.test(dict)) {
      foundImageOnly = true;
      continue;
    }

    let content = null;
    if (/\/FlateDecode/.test(dict)) {
      content = inflateData(chunk.data);
    } else if (!/\/Filter/.test(dict)) {
      content = chunk.data;
    }
    if (!content) continue;

    const asciiHead = content.subarray(0, Math.min(content.length, 600)).toString("latin1");
    const looksLikeContent =
      /\bBT\b/.test(asciiHead) ||
      /\bTj\b|\bTJ\b|\bTd\b|\bTD\b|\bTm\b/.test(asciiHead) ||
      /\bbeginbfchar\b|\bbeginbfrange\b/.test(asciiHead);
    if (!looksLikeContent) continue;
    if (/\bbeginbfchar\b|\bbeginbfrange\b/.test(asciiHead) && !/\b(?:BT|Tj|TJ)\b/.test(content.toString("latin1"))) {
      continue;
    }

    const extracted = extractFromContent(content);
    if (extracted.length > 0) {
      parts.push(extracted);
    }
  }

  const text = parts.join("\n\n").trim();
  if (text.length === 0) {
    if (foundImageOnly) {
      throw new PdfError(
        "PDF enthält nur Rasterbilder/Scans (Bildseiten) und keine durchsuchbare Textebene. OCR-Texterkennung ist für gescannte PDFs erforderlich."
      );
    }
    throw new PdfError(
      "Kein Text im PDF gefunden. Vermutlich ein Scan/Foto-PDF (nur Bilder) oder exotische Font-Kodierung – OCR wird noch nicht unterst\u00fctzt."
    );
  }
  return text;
}

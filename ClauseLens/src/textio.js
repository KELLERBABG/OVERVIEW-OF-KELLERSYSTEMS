"use strict";

import { readFileSync } from "node:fs";
import { pdfToText, PdfError } from "./pdfextract.js";
import { docxToText, DocxError } from "./docxextract.js";

const UTF8_BOM = [0xef, 0xbb, 0xbf];

// Mojibake entsteht, wenn UTF-8-Bytes als Latin-1 gelesen wurden ("MÃ¤ngel").
const MOJIBAKE_RE = /(?:Ã[\u0080-\u00bf]|Â[\u0080-\u00bf]|â€[\u0080-\u00bf])/g;

function repairMojibake(str) {
  const broken = (str.match(MOJIBAKE_RE) || []).length;
  if (broken === 0) return str;
  try {
    let bytes;
    if (typeof Buffer !== "undefined") {
      bytes = Buffer.from(str, "latin1");
    } else {
      bytes = new Uint8Array(str.length);
      for (let i = 0; i < str.length; i++) bytes[i] = str.charCodeAt(i) & 0xff;
    }
    const repaired = new TextDecoder("utf-8", { fatal: false }).decode(bytes);
    const stillBroken = (repaired.match(MOJIBAKE_RE) || []).length;
    return stillBroken < broken ? repaired : str;
  } catch {
    return str;
  }
}

// Typografische Ligaturen (ﬁ, ﬂ, ﬀ) tauchen aus PDF-Extraktion auf. U+FB01
// steht je nach Quelle für "fi" (echte PDF-Ligatur) oder nur für das "i"
// mit realem f davor/dahinter ("Schr" + ﬁ + "ftform", "Logf" + ﬁ + "les").
function repairFiLigatures(str) {
  return str
    .replace(/f\ufb01/gi, (m) => (/^F/.test(m) ? "Fi" : "fi"))
    .replace(/\ufb01f/gi, (m) => (/F$/.test(m) ? "iF" : "if"))
    .replace(/\ufb01/g, "fi")
    .replace(/\uFB01/g, "FI");
}

const LIGATURES = [
  [/\ufb00/g, "ff"],
  [/\ufb02/g, "fl"],
  [/\ufb03/g, "ffi"],
  [/\ufb04/g, "ffl"],
  [/\ufb05/g, "st"],
  [/\ufb06/g, "st"],
];

// Transliteration: fehlende Umlaute (aus ASCII-Quellen oder absichtlicher Evasion).
// Nur bekannte deutsche Wortstämme werden ersetzt, keine generische ae/oe/ue-Regel.
const TRANSLITERATIONS = [
  ["zwoelf", "zw\u00f6lf"],
  ["dreissig", "drei\u00dfig"],
  ["kuendigung", "k\u00fcndigung"],
  ["kuendig", "k\u00fcndig"],
  ["kuendbar", "k\u00fcndbar"],
  ["verjaehr", "verj\u00e4hr"],
  ["maengel", "m\u00e4ngel"],
  ["saemtlich", "s\u00e4mtlich"],
  ["kaeufer", "k\u00e4ufer"],
  ["behaelt", "beh\u00e4lt"],
  ["erhaelt", "erh\u00e4lt"],
  ["enthaelt", "enth\u00e4lt"],
  ["haelt", "h\u00e4lt"],
  ["erhoeh", "erh\u00f6h"],
  ["zurueck", "zur\u00fcck"],
  ["verlaenger", "verl\u00e4nger"],
  ["moeglich", "m\u00f6glich"],
  ["waehrend", "w\u00e4hrend"],
  ["fahrlaessig", "fahrl\u00e4ssig"],
  ["vorsaetzlich", "vors\u00e4tzlich"],
  ["ausschliess", "ausschlie\u00df"],
  ["schliess", "schlie\u00df"],
  ["geschaeft", "gesch\u00e4ft"],
  ["vertraege", "vertr\u00e4ge"],
  ["ansprueche", "anspr\u00fcche"],
  ["pruefung", "pr\u00fcfung"],
  ["pruef", "pr\u00fcf"],
  ["taetig", "t\u00e4tig"],
  ["naechst", "n\u00e4chst"],
  ["zaehl", "z\u00e4hl"],
  ["waehl", "w\u00e4hl"],
  ["faellig", "f\u00e4llig"],
  ["aender", "\u00e4nder"],
  ["ruege", "r\u00fcge"],
  ["rueck", "r\u00fcck"],
  ["groesse", "gr\u00f6\u00dfe"],
  ["stoesse", "st\u00f6\u00dfe"],
  ["ueber", "\u00fcber"],
  ["fuer", "f\u00fcr"],
];

function applyTransliterations(str) {
  let out = str;
  for (const [from, to] of TRANSLITERATIONS) {
    out = out.replace(new RegExp(from, "gi"), (m) => {
      const head = m[0];
      return head === head.toUpperCase() && head !== head.toLowerCase()
        ? to[0].toUpperCase() + to.slice(1)
        : to;
    });
  }
  return out;
}

// Evasion "G e r i c h t s s t a n d": >= 5 einzelne Buchstaben, durch
// Leerzeichen getrennt, werden wieder zusammengezogen.
function joinSpacedLetters(str) {
  return str.replace(
    /(?<![\p{L}\p{N}])(?:\p{L}\s){4,}\p{L}(?![\p{L}\p{N}])/gu,
    (m) => m.replace(/\s+/g, "")
  );
}

// Homoglyphe aus Kyrillisch/Griechisch, die lateinischen Buchstaben gleichen.
// In einem deutschsprachigen Vertrag ist ein solcher Codepoint immer eine Evasion.
const HOMOGLYPHS =
  /[\u0430\u0432\u0435\u043a\u043c\u043d\u043e\u0440\u0441\u0442\u0443\u0445\u0456\u0455\u0501\u0261\u03b1\u03b2\u03b5\u03b7\u03b9\u03ba\u03bc\u03bd\u03bf\u03c1\u03c3\u03c2\u03c4\u03c5\u03c7\u00b5\u0410\u0412\u0415\u041a\u041c\u041d\u041e\u0420\u0421\u0422\u0423\u0425\u0406\u0405\u0500\u0391\u0392\u0395\u0397\u0399\u039a\u039c\u039d\u039f\u03a1\u03a4\u03a5\u03a7]/g;
const HOMOGLYPH_MAP = {
  "\u0430": "a", "\u0432": "b", "\u0435": "e", "\u043a": "k", "\u043c": "m", "\u043d": "h",
  "\u043e": "o", "\u0440": "p", "\u0441": "c", "\u0442": "t", "\u0443": "y", "\u0445": "x",
  "\u0456": "i", "\u0455": "s", "\u0501": "d", "\u0261": "g",
  "\u03b1": "a", "\u03b2": "b", "\u03b5": "e", "\u03b7": "n", "\u03b9": "i", "\u03ba": "k",
  "\u03bc": "u", "\u03bd": "v", "\u03bf": "o", "\u03c1": "p", "\u03c3": "s", "\u03c2": "s",
  "\u03c4": "t", "\u03c5": "u", "\u03c7": "x", "\u00b5": "u",
  "\u0410": "A", "\u0412": "B", "\u0415": "E", "\u041a": "K", "\u041c": "M", "\u041d": "H",
  "\u041e": "O", "\u0420": "P", "\u0421": "C", "\u0422": "T", "\u0423": "Y", "\u0425": "X",
  "\u0406": "I", "\u0405": "S", "\u0500": "D",
  "\u0391": "A", "\u0392": "B", "\u0395": "E", "\u0397": "H", "\u0399": "I", "\u039a": "K",
  "\u039c": "M", "\u039d": "N", "\u039f": "O", "\u03a1": "P", "\u03a4": "T", "\u03a5": "Y",
  "\u03a7": "X",
};

function mapHomoglyphs(str) {
  return str.replace(HOMOGLYPHS, (ch) => HOMOGLYPH_MAP[ch] || ch);
}

// Fullwidth-Latein (U+FF01–U+FF5E) und Fullwidth-Ziffern auf ASCII zurückführen.
function narrowFullwidth(str) {
  return str.replace(/[\uff01-\uff5e]/g, (ch) => String.fromCharCode(ch.charCodeAt(0) - 0xfee0));
}

const JOIN_STOPWORDS = new Set([
  "und", "oder", "für", "von", "vom", "der", "die", "das", "dem", "den", "des", "ist",
  "sind", "wird", "werden", "gilt", "gelten", "nach", "bei", "mit", "auf", "aus", "im",
  "in", "zu", "zur", "zum", "auch", "sowie", "nicht", "nur", "ohne", "je", "pro", "als",
  "so", "ein", "eine", "einer", "eines", "einem", "einen",
]);

// PDF-Zeilenumbruch trennt Komposita ("Haf-\ntung", "voll-\numfänglich").
function dehyphenate(str) {
  return str.replace(/([A-Za-z\u00c0-\u017f]{2,})-\n[ \t]*([a-z\u00e0-\u00ff]{2,})/g, (m, head, tail) =>
    JOIN_STOPWORDS.has(tail) ? m : head + tail
  );
}

// Extraktion reißt gelegentlich Komposita an Leerzeichen/Tabs auseinander
// ("Vertrags strafe", "Haftungs beschränkung"). Nur bekannte Präfixe werden
// wieder zusammengeführt, Funktionswörter bleiben unangetastet.
const COMPOUND_PREFIX =
  /(Haftungs|Vertrags|Gerichts|Schrift|Mängel|Zugangs|Zurück|Beweis|Aufrechnungs|Konventional|Gewährleistungs|Kündigungs|Zahlungs|Widerrufs|Freistellungs|Schadens|Nutzungs|Vertraulichkeits|Leistungs|Vergütungs|Provisions|Abtretungs|Auftrags|Eigentums|Verjährungs|Verzugs|Zustimmungs|Genehmigungs|Benachrichtigungs|Informations|Auswahl|Erfüllungs|Anwendungs|Sicherheits|Verteidigungs|Schieds|Mandats|Projekt|Daten)/g;

function repairCompoundSplits(str) {
  return str.replace(
    new RegExp(COMPOUND_PREFIX.source + "\\s+([a-z\\u00e4\\u00f6\\u00fc\\u00df][a-z\\u00e4\\u00f6\\u00fc\\u00df-]{2,})", "g"),
    (m, prefix, tail) => (JOIN_STOPWORDS.has(tail) ? m : prefix + tail)
  );
}

// Eingeklebte Wörter ("SämtlicheAnsprüche") entstehen bei OCR ohne Wortgrenzen.
// Nur bei klarer Häufung (>= 3 Binnen-Großbuchstaben) wird gesplittet, damit
// normale Schreibweisen wie "eBay" unangetastet bleiben.
function splitGluedCaps(str) {
  const glued = str.match(/[a-z\u00e4\u00f6\u00fc\u00df][A-Z\u00c4\u00d6\u00dc]/g);
  if (!glued || glued.length < 3) return str;
  return str.replace(/([a-z\u00e4\u00f6\u00fc\u00df])([A-Z\u00c4\u00d6\u00dc])/g, "$1 $2");
}

export function normalizeText(str) {
  if (typeof str !== "string") return "";
  let s = repairMojibake(str);
  s = s
    .replace(/[\u00ad\u200b\u200c\u200d\ufeff]/g, "") // Soft-Hyphen und Zero-Width Zeichen entfernen
    .replace(/([\p{L}])\t+(?=\p{L})/gu, "$1") // Tab mitten im Wort ("ver\tlängert") entfernen
    .replace(/\t+/g, (m) => " ".repeat(m.length)); // restliche Tabs zu Leerzeichen
  s = repairFiLigatures(s);
  s = s.replace(/[\u2010\u2011\u2012]/g, "-"); // Hyphen-Varianten vereinheitlichen
  s = narrowFullwidth(s);
  for (const [re, out] of LIGATURES) s = s.replace(re, out);
  s = mapHomoglyphs(s);
  s = s.normalize("NFC");
  s = s.replace(/[\u00a0\u202f\u2007\u3000]/g, " "); // Geschützte Leerzeichen normalisieren
  // OCR-Verwechslungen: "Wi1mington" (1 statt l), "l2 Prozent" (l statt 1), "forrn".
  s = s.replace(/(?<=[A-Za-z\u00c4\u00d6\u00dc\u00e4\u00f6\u00fc\u00df])1(?=[A-Za-z\u00c4\u00d6\u00dc\u00e4\u00f6\u00fc\u00df])/g, "l");
  s = s.replace(/(?<![A-Za-z\u00c4\u00d6\u00dc\u00e4\u00f6\u00fc\u00df0-9])l(?=\d{1,3}\b)/g, "1");
  s = s.replace(/forrn\b/gi, "form");
  s = s.replace(/\[\d+\]/g, " "); // Fußnotenmarker ("gilt[1]") neutralisieren
  s = joinSpacedLetters(s);
  s = applyTransliterations(s);
  s = dehyphenate(s);
  s = repairCompoundSplits(s);
  s = splitGluedCaps(s);
  return s.replace(/\r\n|\r/g, "\n"); // Einheitliche Unix-Zeilenenden
}

export function decodeBuffer(buf) {
  let str = "";
  if (buf.length >= 2 && buf[0] === 0xff && buf[1] === 0xfe) {
    str = buf.slice(2).toString("utf16le");
  } else if (
    buf.length >= 3 &&
    buf[0] === UTF8_BOM[0] &&
    buf[1] === UTF8_BOM[1] &&
    buf[2] === UTF8_BOM[2]
  ) {
    str = buf.slice(3).toString("utf-8");
  } else {
    try {
      str = new TextDecoder("utf-8", { fatal: true }).decode(buf);
    } catch {
      str = buf.toString("latin1");
    }
  }
  return normalizeText(str);
}

export function readTextFile(filePath) {
  return decodeBuffer(readFileSync(filePath));
}

const FORMATS = new Set(["auto", "txt", "pdf", "docx"]);

export function detectFormat(buf, filePath) {
  const ext = (filePath ? filePath.toLowerCase().split(".").pop() : "").trim();
  if (buf.subarray(0, 5).toString("latin1").includes("%PDF-") || buf.subarray(0, 1024).toString("latin1").includes("%PDF-")) return "pdf";
  if (buf.length >= 2 && buf[0] === 0x50 && buf[1] === 0x4b) return "docx";
  if (ext === "pdf") return "pdf";
  if (ext === "docx") return "docx";
  return "txt";
}

export function extractTextFromBuffer(buf, formatHint = "auto", filename = "") {
  if (!Buffer.isBuffer(buf)) buf = Buffer.from(buf);
  if (!FORMATS.has(formatHint)) {
    throw new Error(`Unbekanntes Format "${formatHint}" (erlaubt: auto, txt, pdf, docx).`);
  }
  const format = formatHint === "auto" ? detectFormat(buf, filename) : formatHint;
  if (format === "pdf") {
    try {
      return { text: pdfToText(buf), format };
    } catch (err) {
      if (err instanceof PdfError) throw err;
      throw new PdfError(`PDF konnte nicht gelesen werden: ${err.message}`);
    }
  }
  if (format === "docx") {
    try {
      return { text: docxToText(buf), format };
    } catch (err) {
      if (err instanceof DocxError) throw err;
      throw new DocxError(`DOCX konnte nicht gelesen werden: ${err.message}`);
    }
  }
  return { text: decodeBuffer(buf), format };
}

export function extractText(filePath, formatHint = "auto") {
  const buf = readFileSync(filePath);
  return extractTextFromBuffer(buf, formatHint, filePath);
}

export function splitLines(text) {
  return text.split(/\r\n|\n|\r/);
}

export function lineStartsOf(text) {
  const starts = [0];
  for (let i = 0; i < text.length; i++) {
    if (text[i] === "\n") starts.push(i + 1);
  }
  return starts;
}

export function lineOf(lineStarts, offset) {
  let lo = 0;
  let hi = lineStarts.length - 1;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (lineStarts[mid] <= offset) lo = mid;
    else hi = mid - 1;
  }
  return lo + 1;
}

const TERMINATORS = ".!?;:";
const CLOSERS = "\"'\u2019\u201d\u00bb\u00bb)]\u203a";

const ABBREVIATIONS = [
  "z.b.", "z. b.", "d.h.", "d. h.", "u.a.", "u. a.", "usw.", "nr.", "abs.", "vgl.", "ggf.",
  "evtl.", "bzw.", "inkl.", "zzgl.", "bspw.", "insb.", "sog.", "ua.", "art.", "s.", "lit.",
  "ziff.", "gem.", "f.", "ff.", "i.v.m.", "i. v. m.", "e.g.", "i.e.",
];

function endsWithAbbreviation(slice) {
  const tail = slice.toLowerCase().replace(/\s+/g, " ").trim();
  if (
    ABBREVIATIONS.some(
      (abbr) =>
        tail === abbr ||
        tail.endsWith(" " + abbr) ||
        tail.endsWith("(" + abbr) ||
        tail.endsWith("[" + abbr)
    )
  )
    return true;
  // Einzelne isolierte Buchstaben mit Punkt ("S.", "f.", "i.", "V.", "m.") – auch direkt
  // nach einer Klammer, damit mehrteilige Abkürzungen wie "(z. B. ISACs)" nicht zerreissen.
  if (/(?:^|[\s(\[{])[a-z]\.$/i.test(tail)) return true;
  // Datums-/Zahlenmuster wie "am 1.3." oder "seit 12.05." sind keine Satzenden
  if (/\b\d{1,2}\.\d{1,2}\.?$/.test(tail)) return true;
  return false;
}

// Nur eine Klausel-Nummer ("1.", "§ 4", "(2)") ist noch kein Satz.
const ONLY_CLAUSE_NUMBER_RE =
  /^(?:\u00a7\s*\d+|\d+\.(?:\d+\.)*|\(\d+\)|[IVX]+\.)\s*$/;

const CLAUSE_START_RE =
  /^(?:\u00a7\s*\d+|\(\d+\)|\d+\.(?:\d+\.)*(?=\s)|[IVX]+\.(?=\s))/;

/**
 * Satzsegmentierung v0.2:
 * - Satzgrenzen an Satzzeichen + Whitespace (wie bisher)
 * - KEINE Grenzen nach bekannten Abkürzungen (z.B., Nr., Abs., ...)
 * - Zusätzliche Grenze an Zeilenenden, wenn die nächste Zeile mit einer
 *   Klausel-Markierung beginnt (§, 1., (1), I.) und die aktuelle Zeile
 *   ohne Satzendzeichen endet (Überschriften).
 */
export function segmentSentences(text) {
  const out = [];
  let start = 0;
  let i = 0;

  const push = (end) => {
    let s = start;
    while (s < end && /\s/.test(text[s])) s++;
    let e = end;
    while (e > s && /\s/.test(text[e - 1])) e--;
    if (e - s > 1) out.push({ start: s, end: e, text: text.slice(s, e) });
  };

  while (i < text.length) {
    const ch = text[i];
    if (TERMINATORS.includes(ch)) {
      let j = i + 1;
      while (j < text.length && CLOSERS.includes(text[j])) j++;
      const nextIsWs = j >= text.length || /\s/.test(text[j]);
      if (nextIsWs) {
        const soFar = text.slice(start, j);
        // "zum 1. Januar", "ab 1. Juli": Datumsangaben mit Monatsnamen sind keine Satzenden.
        const dateLike =
          ch === "." &&
          /\b\d{1,2}\.$/.test(soFar) &&
          /^(?:januar|februar|m(?:ä|ae)rz|april|mai|juni|juli|august|september|oktober|november|dezember)\b/i.test(
            text.slice(j, j + 40).trimStart()
          );
        if (!dateLike && !endsWithAbbreviation(soFar) && !ONLY_CLAUSE_NUMBER_RE.test(soFar.trim())) {
          push(j);
          i = j;
          while (i < text.length && /\s/.test(text[i])) i++;
          start = i;
          continue;
        }
        i = j;
        continue;
      }
    }
    if (ch === "\n") {
      let next = i + 1;
      while (next < text.length && /[ \t\r]/.test(text[next])) next++;
      const rest = text.slice(next, Math.min(next + 40, text.length));
      if (
        next < text.length &&
        CLAUSE_START_RE.test(rest) &&
        !endsWithAbbreviation(text.slice(start, i))
      ) {
        push(i);
        start = next;
      }
    }
    i++;
  }
  if (start < text.length) push(text.length);
  return out;
}

export function loadDocument(filePath) {
  const text = readTextFile(filePath);
  return { text, lines: splitLines(text) };
}

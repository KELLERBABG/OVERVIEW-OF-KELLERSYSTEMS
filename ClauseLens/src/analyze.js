"use strict";

import { segmentSentences, lineStartsOf, lineOf, normalizeText } from "./textio.js";
import { mergeConfig } from "./config.js";

export const SEVERITIES = Object.freeze(["niedrig", "mittel", "hoch"]);
export const SEVERITY_WEIGHT = Object.freeze({ niedrig: 2, mittel: 7, hoch: 15 });
const RANK = { niedrig: 1, mittel: 2, hoch: 3 };

const NUM_WORDS = {
  ein: 1, eine: 1, einem: 1, einen: 1, eines: 1, eins: 1,
  zwei: 2, drei: 3, vier: 4, "f\u00fcnf": 5, sechs: 6, sieben: 7,
  acht: 8, neun: 9, zehn: 10, elf: 11, "zw\u00f6lf": 12,
  dreizehn: 13, vierzehn: 14, "f\u00fcnfzehn": 15, sechzehn: 16,
  siebzehn: 17, achtzehn: 18, neunzehn: 19,
  zwanzig: 20, "drei\u00dfig": 30, dreissig: 30, vierzig: 40,
  "f\u00fcnfzig": 50, sechzig: 60, siebzig: 70, achtzig: 80, neunzig: 90,
  vierundzwanzig: 24, sechsunddrei\u00dfig: 36, sechsunddreissig: 36,
  zweiundzwanzig: 22, "f\u00fcnfundzwanzig": 25, sechsundzwanzig: 26,
  dreiundzwanzig: 23, einundzwanzig: 21, achtundvierzig: 48, zweiundsiebzig: 72,
};

const NUM_ONES = {
  ein: 1, eins: 1, zwei: 2, drei: 3, vier: 4, "f\u00fcnf": 5, sechs: 6,
  sieben: 7, acht: 8, neun: 9,
};
const NUM_TENS = {
  zwanzig: 20, "drei\u00dfig": 30, dreissig: 30, vierzig: 40,
  "f\u00fcnfzig": 50, sechzig: 60, siebzig: 70, achtzig: 80, neunzig: 90,
};
const NUM_TEENS = {
  zehn: 10, elf: 11, "zw\u00f6lf": 12, dreizehn: 13, vierzehn: 14,
  "f\u00fcnfzehn": 15, sechzehn: 16, siebzehn: 17, achtzehn: 18, neunzehn: 19,
};

/** Parst deutsche Zahlwörter (inkl. Komposita wie "sechsunddreißig"). */
export function parseGermanNumber(raw) {
  if (typeof raw !== "string") return undefined;
  const w = raw.toLowerCase();
  if (/^\d+$/.test(w)) return parseInt(w, 10);
  if (Object.prototype.hasOwnProperty.call(NUM_WORDS, w)) return NUM_WORDS[w];
  const m = w.match(/^([a-z\u00e4\u00f6\u00fc\u00df]+)und([a-z\u00e4\u00f6\u00fc\u00df]+)$/);
  if (m) {
    const one = NUM_ONES[m[1]] ?? NUM_TEENS[m[1]];
    const tens = NUM_TENS[m[2]];
    if (one && tens) return one + tens;
  }
  // Hunderter-Komposita: "einhundertachtzig", "zweihundert", "einhundert(und)zehn"
  const h = w.match(/^(ein|eins|zwei|drei|vier|f(?:ü|ue)nf|sechs|sieben|acht|neun)hundert([a-z\u00e4\u00f6\u00fc\u00df]*)$/);
  if (h) {
    const base = (NUM_ONES[h[1]] ?? 1) * 100;
    if (!h[2]) return base;
    const rest = parseGermanNumber(h[2].replace(/^und(?=.)/, ""));
    if (rest) return base + rest;
  }
  return undefined;
}

const UNIT_MONTH_FACTOR = (unit) => {
  const u = unit.toLowerCase();
  if (u.startsWith("monat")) return 1;
  if (u.startsWith("woch")) return 7 / 30.44;
  if (u.startsWith("jahr")) return 12;
  if (u.startsWith("tag")) return 1 / 30.44;
  return NaN;
};

/** Extrahiert Zeiträume ("sechs Monate", "4 Wochen") als Monatswerte. */
export function parseDurationsMonths(sentence) {
  const re =
    /(?:\b|\()(\d{1,3}|[a-z\u00e4\u00f6\u00fc\u00df]{3,20})\)?\s*(?:\([^\)]+\)\s*)?(?:kalender)?(monat(?:e|en|es|s)?|woc(?:he|hen)|jahr(?:e|es|en|s)?|tag(?:e|en|s)?)\b/gi;
  const out = [];
  let m;
  while ((m = re.exec(sentence))) {
    const n = parseGermanNumber(m[1]);
    if (!Number.isFinite(n)) continue;
    const months = n * UNIT_MONTH_FACTOR(m[2]);
    if (Number.isFinite(months)) out.push(months);
  }
  return out;
}

function fmtMonths(m) {
  return String(Math.round(m * 10) / 10).replace(".", ",");
}

// Einzelne Zeilenumbrüche (PDF-Zeilen, Seitenumbrüche) werden für das
// Muster-Matching wie Leerzeichen behandelt; Absatzgrenzen (\n\n) bleiben
// erhalten, damit Muster weiterhin nicht über Absätze hinweg greifen.
export function flattenLineBreaks(text) {
  return text
    .replace(/(?<!\n)\n(?!\n)/g, " ")
    .replace(/[ \t]+/g, " ");
}

// Muster greifen auch auf Formulierungen, die in sauberen Kontrollklauseln vorkommen
// (Zustimmungsvorbehalt, symmetrische Kuendigung, gesetzlich zwingende Haftung).
// Diese Ausschluesse verhindern den False Positive, ohne das Muster zu entfernen.
const MATCH_VETO = {
  jurisdiction: [
    /(?:sofern|wenn|ist|sind|f(?:ü|ue)r)\s+(?:der\s+)?(?:kunde|auftraggeber|besteller|mieter|leasingnehmer|partner)\s+(?:kaufmann|vollkaufmann)/i,
    /(?:kaufleute|vollkaufleute|kaufmann\b|vollkaufmann\b|handelsgesetzbuch|hgb)/i,
    /juristische\s+person\s+des\s+öffentlichen\s+rechts|öffentlich-rechtliche[s]?\s+sondervermögen/i,
    /§\s*38\s*(?:abs\.\s*1\s*)?zpo|§\s*29a\s*zpo/i,
    /gerichtsstand\s+der\s+belegenheit/i,
  ],
  penalty: [
    /(?:tats(?:ä|ae)chlich\s+entstandenen?\s+(?:bankgeb(?:ü|ue)hren|geb(?:ü|ue)hren|kosten|schaden)|nachweis\s+(?:eines\s+)?geringeren\s+schadens\s+bleibt)/i,
  ],
  subcontractor_uncontrolled: [
    /(?:nur\s+(?:mit|nach)|erst\s+nach)\s+(?:vorheriger\s+|schriftlicher\s+)?(?:zustimmung|einwilligung|genehmigung|freigabe|information|ank(?:ü|ue)ndigung)/i,
    /wenn\s+(?:er|sie|es)\s+(?:dies|den\s+Einsatz|die\s+Beauftragung)[^.!?;:\n]{0,70}?(?:mitteilt|informiert|anzeigt)/i,
  ],
  excessive_survival: [
    /(?:preise|konditionen|verg(?:ü|ue)tung|entgelte|bedingungen)[^.!?;:\n]{0,60}?(?:gelten|bleiben)[^.!?;:\n]{0,40}?unver(?:ä|ae)ndert/i,
  ],
  marketing_reference: [
    /(?:nicht|keine|keinen|ohne)[^.!?;:\n]{0,50}?(?:als\s+referenz|in\s+(?:pressemitteilungen|angeboten|fallstudien|werbung|kundenlisten)|referenzkunde)/i,
  ],
  notice_period: [
    /(?:beide\s+(?:parteien|seiten|vertragspartner)|jede\s+(?:partei|seite))[^.!?;:\n]{0,120}?(?:k(?:ü|ue)ndigen|beenden|beendigen)/i,
  ],
  liability: [
    /unbeschr(?:ä|ae)nkt[^.!?;:\n]{0,80}?(?:vorsatz|grobe\s+fahrl(?:ä|ae)ssigkeit)|(?:vorsatz|grobe\s+fahrl(?:ä|ae)ssigkeit)[^.!?;:\n]{0,80}?unbeschr(?:ä|ae)nkt/i,
    /unbeschr(?:ä|ae)nkt[^.!?;:\n]{0,80}?(?:leben|k(?:ö|oe)rper|gesundheit)|(?:leben|k(?:ö|oe)rper|gesundheit)[^.!?;:\n]{0,80}?unbeschr(?:ä|ae)nkt/i,
    /\bder\s+(?:anbieter|auftragnehmer|lizenzgeber|dienstleister|hersteller|verk(?:ä|ae)ufer|vermieter\w*|sponsor)\b[^.!?;:\n]{0,70}?(?:stellt|stellen|h(?:ä|ae)lt|halten)[^.!?;:\n]{0,70}?\bfrei\b/i,
    /(?:kardinalpflicht|wesentliche[nr]?\s+vertragspflicht)[^.!?;:\n]{0,80}?(?:vertragstypisch|vorhersehbar)(?![\s\S]{0,120}?(?:monatsverg(?:ü|ue)tung|monatsmiete|auftragssumme|\d+[\d.,]*\s*(?:eur|euro|chf)|\d+\s*(?:%|prozent)))/i,
    /(?:wiederherstellungsaufwand|typischen\s+wiederherstellungsaufwand)[^.!?;:\n]{0,80}?(?:beschr(?:ä|ae)nkt|begrenzt)/i,
    /(?:leicht|leichte|leichter|einfache|einfacher)\s+fahrl(?:ä|ae)ssig\w*[^.!?;:\n]{0,80}?nicht\s+wesentliche[nr]?\s+vertragspflicht[^.!?;:\n]{0,40}?ausgeschlossen/i,
    /(?:stellt[^.!?;:\n]{0,80}?zur\s+verf(?:ü|ue)gung|zur\s+verf(?:ü|ue)gung\s+(?:zu\s+)?stellen|vor\s+schadsoftware\s+zu\s+sch(?:ü|ue)tzen)/i,
  ],
  unilateral_change: [
    // Negierte einseitige Aenderung ist die Kontrollklausel, kein Aenderungsvorbehalt.
    /nicht\s+einseitig\w*[^.!?;:\n]{0,40}?(?:ge(?:ä|ae)ndert|ange(?:passt|glichen)|eingeschr(?:ä|ae)nkt|modifiziert)/i,
    /(?:anerkannten\s+regeln\s+der\s+technik|baustrom\s+und\s+bauwasser)/i,
  ],
  statute_of_limitations: [
    /(?:nach\s+den\s+gesetzlichen\s+vorschriften|gesetzlichen\s+vorschriften)[^.!?;:\n]{0,50}?(?:in\s+zwei\s+jahren|zwei\s+jahre|\b2\s+jahre)/i,
  ],
};

function collectPerSentence(rule, sentences, only = null, extra = null) {
  const out = [];
  const vetoes = MATCH_VETO[rule.type];
  sentences.forEach((s, i) => {
    if (only && !only.has(s)) return;
    const matchText = sentenceMatchText(s, extra);
    for (const re of rule.patterns || []) {
      const m = re.exec(matchText);
      if (!m) continue;
      if (vetoes && vetoes.some((v) => v.test(matchText))) continue;
      const sev =
        typeof rule.severity === "function" ? rule.severity(matchText, m) : rule.severity;
      if (sev) out.push({ i, severity: sev });
      break; // max. ein Treffer pro Regel und Satz -> Dedup auf Satzebene
    }
  });
  return out;
}

// Engine-Hotpath: Satztexte werden pro Regel erneut geglaettet. Die Memoisierung
// pro Satzobjekt (WeakMap) spart bei vielen Regeln x vielen Saetzen den Grossteil
// der String-Operationen, ohne das Matching-Verhalten zu veraendern.
const flattenedSentenceCache = new WeakMap();

function flattenSentence(sentence) {
  let cached = flattenedSentenceCache.get(sentence);
  if (cached === undefined) {
    cached = flattenLineBreaks(sentence.text);
    flattenedSentenceCache.set(sentence, cached);
  }
  return cached;
}

// Ein Satz kann seinen Tatbestand ueber den Absatzumbruch hinweg (continuation)
// oder ueber einen aufgeloesten Anlagenverweis (extra) tragen.
function sentenceMatchText(sentence, extra = null) {
  let text = flattenSentence(sentence);
  if (sentence.continuation) text += " " + sentence.continuation;
  if (extra && extra.has(sentence)) text += " " + extra.get(sentence);
  return text;
}

// Layoutumbrueche reissen Klauseln an Absatzgrenzen auf ("... wird eine," /
// "pauschale Abstandszahlung ..."). Ein Satz, der mit Komma endet und am
// Absatzende steht, wird fuer das Matching an den Folgeabsatz gehaengt; die
// Fundstelle bleibt der abgebrochene Satz.
function attachContinuations(sentences, text) {
  for (let i = 0; i + 1 < sentences.length; i++) {
    const cur = sentences[i];
    if (!/,\s*$/.test(cur.text)) continue;
    const gap = text.slice(cur.end, sentences[i + 1].start);
    if (!/\n[ \t]*\n/.test(gap)) continue;
    cur.continuation = flattenLineBreaks(sentences[i + 1].text);
  }
}

// Ein Verweis auf eine Anlage liefert den Tatbestand nicht im verweisenden Absatz
// ("Die in Anlage 2 Ziffer 1 geregelte Massnahme gilt ..."). Der Verweis wird fuer
// das Matching aufgeloest, damit die Fundstelle am verweisenden Satz steht.
const ANNEX_REF_RE =
  /(?:anlage|anhang|annex|nachtrag)\s*(\d+)[^.!?;:\n]{0,40}?(?:ziffer|ziff\.?|nr\.?|abschnitt|punkt)\s*(\d+)/i;

const ANNEX_ITEM_RE = (item) =>
  new RegExp(
    "(?:^|\\n)[ \\t]*(?:\\(\\s*" + item + "\\s*\\)|" + item + "\\s*[.)]\\s|ziffer\\s*" + item + "\\b)",
    "i"
  );

// Mehrere Vorkommen von "Anlage N" sind moeglich: der verweisende Absatz selbst und
// die Ueberschrift der Anlage. Der erste Kandidat, der einen echten Anlagenpunkt
// liefert (und nicht wieder ein Verweis ist), gewinnt.
function resolveAnnexItem(text, n, item) {
  const head = new RegExp("anlage\\s*" + n + "\\b", "gi");
  const itemRe = ANNEX_ITEM_RE(item);
  const starts = [];
  let m;
  while ((m = head.exec(text))) starts.push(m.index);
  // Von hinten suchen: die Anlage selbst steht meist nach ihren Verweisen.
  for (const at of starts.reverse()) {
    const rest = text.slice(at);
    const next = rest.slice(1).search(new RegExp("\\n[^\\n]{0,40}?anlage\\s*\\d+\\b", "i"));
    const region = next < 0 ? rest : rest.slice(0, next + 1);
    for (const para of region.split(/\n\s*\n/)) {
      if (!itemRe.test(para)) continue;
      const flat = flattenLineBreaks(para);
      if (ANNEX_REF_RE.test(flat)) continue;
      return flat;
    }
  }
  return null;
}

function attachCrossRefs(sentences, text) {
  for (const s of sentences) {
    const m = ANNEX_REF_RE.exec(flattenSentence(s));
    if (!m) continue;
    const item = resolveAnnexItem(text, m[1], m[2]);
    if (item) s.crossRefText = item;
  }
}

export const RULES = [
  {
    type: "auto_renewal",
    title(ctx) {
      const s = ctx?.sentence || "";
      if (/freigeschaltet|gebucht|kostenpflichtig|zusatzpaket|leistungspaket/i.test(s)) {
        return "Versteckte kostenpflichtige Zusatzleistung (Opt-out-Kostenfalle, § 312a Abs. 3 BGB)";
      }
      return "Automatische Vertragsverlängerung";
    },
    tags: ["bgb-307", "laufzeit", "kuendigung", "abo-falle"],
    patterns: [
      /verlängert\s+sich[^.!?;:\n]{0,120}/i,
      // Evasion-Swarm: Fortbestehen/Verlängerung ohne automatische Nennung ("setzt sich fort, sofern nicht beendet")
      /(?:vertrag|vertragsverhältnis|leistungsvereinbarung|leistungserbringung)[^.!?;:\n]{0,80}?(?:bleibt|besteht)[^.!?;:\n]{0,80}?(?:bis\s+zur\s+endgültigen\s+aufhebung|so\s+lange|fort|bestehen|während\s+der\s+laufzeit)[^.!?;:\n]{0,120}?(?:fortbestehen|wird\s+(?:das\s+vertragsverhältnis\s+)?fortgeführt|verlängert|setzt\s+sich\s+fort)/i,
      /(?:fortgesetzte\s+nutzung|weitere\s+inanspruchnahme)[^.!?;:\n]{0,80}?(?:gilt\s+als\s+annahme|annahme\s+der\s+weiterführung|vertragsverlängerung)/i,
      /(?:bei\s+ablauf\s+(?:der\s+vereinbarten\s+)?dauer|nach\s+ablauf\s+der\s+laufzeit)[^.!?;:\n]{0,80}?wird\s+(?:das\s+vertragsverhältnis\s+)?fortgeführt/i,
      /(?:vertrag|leistungsvereinbarung|leistungserbringung)[^.!?;:\n]{0,80}?(?:bleibt|besteht)[^.!?;:\n]{0,80}?(?:so\s+lange|fort|bestehen)[^.!?;:\n]{0,80}?(?:soweit|solange|sofern)[^.!?;:\n]{0,80}?(?:nicht|keine)[^.!?;:\n]{0,80}?(?:beend\w*|gekündigt)/i,
      /(?:eine\s+)?verlängerung\s+setzt\s+sich\s+fort[^.!?;:\n]{0,80}?(?:sofern|soweit|wenn)\s+nicht/i,
      /stillschweigend\w*\s+verlängerung/i,
      /automatisch\w*,?\s+(?:um\s+[^.!?;:\n]{0,40})?verlängerung/i,
      /verlängerung\w*\s+um\s+(?:jeweils\s+)?weitere/i,
      /(?:automatically\s+renew\w*|auto[\s-]renew\w*|renewal\s+term)[^.!?;:\n]{0,120}?(?:unless|notice|successive|\d+[\s-]month|further|period)/i,
      /(?:renew\w*\s+automatically|automatic\s+renewal)/i,
      /(?:vertrag|vereinbarung|vertragsverhältnis|laufzeit|abonnement|partnerschaft|systempartnerschaft|rahmenvertrag)[^;:\n]{0,80}?(?:setzt|setzen)\s+sich[^;:\n]{0,60}?fort/i,
      /(?:setzt|setzen)\s+sich[^;:\n]{0,80}?(?:jeweils\s+um|um\s+(?:\d+|ein|zwei|drei|sechs|zwölf|achtzehn|vierundzwanzig)|auf\s+unbestimmte\s+zeit)[^;:\n]{0,60}?fort/i,
      /(?:erneuert|erneuern)\s+sich[^;:\n]{0,80}/i,
      /verläng\w*(?:en|t)?\s+(?:sie\s+)?sich[^;:\n]{0,140}/i,
      // Versteckte kostenpflichtige Zusatzoptionen (Negative Option Billing / Opt-out Trap, § 312a Abs. 3 BGB)
      /(?:automatisch\s+(?:freigeschaltet|aktiviert|gebucht)[^.!?;:\n]{0,80}?(?:in\s+rechnung\s+gestellt|kostenpflichtig)|umfasst[^.!?;:\n]{0,80}?zusätzlich[^.!?;:\n]{0,80}?(?:ab\s+dem\s+\w+\s+vertragsmonat|nach\s+ablauf)[^.!?;:\n]{0,80}?(?:in\s+rechnung\s+gestellt|kostenpflichtig)|automatisch\s+freigeschaltet\s+und\s+in\s+rechnung\s+gestellt)/i,
      // Verschleierte Verlängerung: Fiktionsformulierung / Verweis auf andere Ziffer
      /keine\s+(?:beendigungs|kündigungs|widerspruchs|austritts)erklärung[^.!?;:\n]{0,160}?(?:tritt|greift|erfolgt|kommt|gilt)[^.!?;:\n]{0,80}?(?:verlängerung|verlängert)/i,
      /verlängerung\s+(?:des\s+)?(?:vertragsverh(?:ä|ae)ltnisses|vertrags|rahmenvertrags)[^.!?;:\n]{0,40}?um\s+(?:\d{1,3}|[a-zäöüß]{3,20}|ein\s+weiteres)/i,
      /(?:fortschreibung|fortbestand|verlängerung)[^.!?;:\n]{0,100}?greift[^.!?;:\n]{0,60}?(?:jeweils|um\s+ein\s+weiteres)[^.!?;:\n]{0,80}/i,
      /(?:fortschreibung|verlängerung)[^.!?;:\n]{0,140}?wenn\s+keine\s+partei[^.!?;:\n]{0,60}?widerspricht/i,
    ],
    severity(s) {
      // False-Positive-Schutz: ausdrücklicher Ausschluss der Automatik ("verlängert sich nicht automatisch")
      if (/(?:verläng\w*|erneuert)\s+(?:sich\s+)?nicht|nicht\s+automatisch\w*(?:\s+\w+){0,3}\s+(?:verläng\w*|erneuert)|keine\s+automatische\s+verl(?:ä|ae)ngerung/i.test(s)) {
        return null;
      }
      // False-Positive-Schutz: Fristverlaengerung fuer Liefer-/Leistungsfristen ist keine automatische Abo-Verlaengerung
      if (/(?:lieferfristen?|leistungsfristen?|bearbeitungsfristen?|fertigstellungsfristen?|reaktionszeiten?)/i.test(s) && !/(?:vertrag\w*|vereinbarung\w*|abonnement\w*|vertragsverhältnis|\b(?<!an)laufzeit\b)/i.test(s)) {
        return null;
      }
      // § 309 Nr. 9 lit. b BGB n.F.: Verlängerung auf unbestimmte Zeit mit Kündigungsfrist von max. 1 Monat ist gesetzeskonform
      if (
        /auf\s+unbestimmte\s+zeit/i.test(s) &&
        /(?:k(?:ü|ue)ndig\w*[^.!?;:\n]{0,80}?(?:frist\s+von\s+(?:einem\s+monat|vier\s+wochen|1\s+monat)|monatlich|jederzeit|zum\s+monatsende)|(?:frist\s+von\s+(?:einem\s+monat|vier\s+wochen|1\s+monat)|monatlich|jederzeit|zum\s+monatsende)[^.!?;:\n]{0,80}?k(?:ü|ue)ndig)/i.test(s)
      ) {
        return null;
      }
      if (/freigeschaltet|in\s+rechnung\s+gestellt|automatically\s+renew/i.test(s)) return "hoch";
      return /frist\s+von\s+(einem|einer)\s+(monat|woche)/i.test(s) ? "mittel" : "hoch";
    },
    why(ctx) {
      const s = ctx?.sentence || "";
      if (/freigeschaltet|gebucht|kostenpflichtig|zusatzpaket|leistungspaket/i.test(s)) {
        return "Versteckte Zusatzentgelte durch Opt-out (§ 312a Abs. 3 BGB, § 305c BGB): Kostenpflichtige Zusatzpakete dürfen nicht formularmäßig unterstellt und automatisch freigeschaltet werden, wenn der Kunde nicht widerspricht. Eine Vereinbarung über zusätzliche Zahlungen bedarf stets der ausdrücklichen vorherigen Zustimmung.";
      }
      return "Ihr Vertrag verlängert sich von selbst, wenn Sie nicht rechtzeitig kündigen. Wer die Frist verpasst, bleibt oft ein weiteres Jahr gebunden – auch ohne neues Interesse an der Leistung.";
    },
    suggestion(ctx) {
      const s = ctx?.sentence || "";
      if (/freigeschaltet|gebucht|kostenpflichtig|zusatzpaket|leistungspaket/i.test(s)) {
        return "Zusatzleistung ersatzlos streichen. Vereinbaren Sie, dass Zusatzoptionen ausschließlich durch aktives Opt-in (ausdrückliche Bestellung) hinzugebucht werden können.";
      }
      return "Fordern Sie: Verlängerung nur mit vorheriger schriftlicher Zustimmung oder Kündigung jederzeit zum Monatsende. Zusätzlich eine Erinnerungspflicht des Anbieters vor Ablauf vereinbaren.";
    },
  },
  {
    type: "notice_period",
    title: "Lange oder ungleiche Kündigungsfristen",
    tags: ["bgb-307", "kuendigungsfrist", "bindung"],
    suggestion:
      "Verhandeln Sie eine beidseitige Kündigungsfrist von höchstens vier Wochen bis drei Monaten.",
    why(ctx) {
      const note = ctx.note || {};
      if (note.activationTrap) {
        return "Verschleierter Laufzeitbeginn (§ 309 Nr. 9 lit. a BGB, BGH III ZR 8/25): Die Mindestlaufzeit wird künstlich nach hinten verschoben (z. B. auf den übernächsten Monat nach Auslieferung oder erst ab Freischaltung), was die gesetzliche Höchstbindungsdauer unzulässig verlängert.";
      }
      const s = ctx.sentence || "";
      if (note.termOnly) {
        return `Der Vertrag bindet Sie für mindestens ${note.months} Monate, ohne ein vorzeitiges Kündigungsrecht zu nennen. Eine so lange Mindestlaufzeit ist unangemessen (§ 309 Nr. 9 lit. b, § 307 BGB).`;
      }
      const renewal = s.match(/verl(?:ä|ae)ngert[^.!?;:\n]{0,20}?sich[^.!?;:\n]{0,60}?um\s+(?:weitere\s+|jeweils\s+)?(\d{1,3}|[a-z\u00e4\u00f6\u00fc\u00df]{3,20})\s*(?:kalender)?(monat|jahr|woche|tag)/i);
      if (renewal && /k(?:ü|ue)ndig/i.test(s)) {
        const n = parseGermanNumber(renewal[1]);
        const rm = Number.isFinite(n) ? n * UNIT_MONTH_FACTOR(renewal[2]) : NaN;
        return `Die Klausel koppelt eine Kündigungsfrist an eine automatische Verlängerung von ${Number.isFinite(rm) ? rm : note.months} Monaten: Ohne rechtzeitige Kündigung sind Sie jeweils erneut so lange gebunden. Diese Kopplung verlängert die Bindung faktisch immer weiter (§ 309 Nr. 9 lit. b, § 307 BGB).`;
      }
      let t = `Hier gilt eine Kündigungsfrist von etwa ${note.months} Monaten. So lange sind Sie gebunden, selbst wenn die Leistung nicht mehr passt.`;
      if (note.asymmetric) {
        t += " Zusätzlich sind die Fristen ungleich verteilt, eine Partei kann deutlich schneller raus als die andere.";
      }
      return t;
    },
    collect(sentences, _full, extra = null) {
      const flagged = [];
      const pool = [];
      const TERM_LINK =
        /(?<![a-z\u00e4\u00f6\u00fc\u00df])(?:l(?:ä|ae)uft|laufzeit|erstlaufzeit|festlaufzeit|mindestlaufzeit|vertragslaufzeit|gesamtlaufzeit|vertragsdauer|zeitraum|dauer|betr(?:ä|ae)gt|erstreckt)\s+(?:sich\s+)?(?:über\s+)?(?:eine[rsnm]?\s+|einen\s+|von\s+|f(?:ü|ue)r\s+|insgesamt\s+|zun(?:ä|ae)chst\s+|jeweils\s+|weitere[nrs]?\s+|um\s+)*$/i;
      const NOTICE_LINK =
        /(?:k(?:ü|ue)ndig|ank(?:ü|ue)ndigung|frist|beendig|vorlaufzeit|losl(?:ö|oe)sung|widerspruch)/i;
      sentences.forEach((s, i) => {
        const text = sentenceMatchText(s, extra);
        // Symmetrische Kuendigung mit kurzer Frist ist ein sauberer Kontrollmechanismus.
        if (
          /(?:beide\s+(?:parteien|seiten|vertragspartner)|jede\s+(?:partei|seite))[^.!?;:\n]{0,140}?(?:k(?:ü|ue)ndig|beendig|beenden)/i.test(text)
        ) {
          const durations = parseDurationsMonths(text);
          if (!durations.length || Math.max(...durations) <= 3) return;
        }
        const activationTrap =
          /(?:mit\s+der\s+freischaltung|ab\s+freischaltung|ab\s+bereitstellung|abweichend\s+vom\s+zeitpunkt\s+der\s+freischaltung)[^.!?;:\n]{0,120}?(?:zu\s+laufen\s+beginnen|beginnt\s+zu\s+laufen|beginnt)/i.test(text) ||
          /(?:mindestlaufzeit)[^.!?;:\n]{0,120}?(?:abweichend\s+vom\s+zeitpunkt|übernächsten\s+kalendermonat\s+beginnt)/i.test(text);
        if (activationTrap) {
          flagged.push({ i, months: 24, activationTrap: true });
          return;
        }

        // Reine Mindestlaufzeit ohne jedes Kündigungswort: eine Bindung von
        // mehr als 24 Monaten ist für sich genommen schon unangemessen lang.
        const termOnly =
          /(?:mindest(?:vertrags)?laufzeit|festlaufzeit|feste\s+laufzeit|erstlaufzeit|mindestdauer|grundlaufzeit)/i.test(text) &&
          !/(?:k(?:ü|ue)ndig|beendig|losl(?:ö|oe)sung|vorzeitig)/i.test(text);
        if (termOnly) {
          const mTerm = text.match(
            /(?:mindest(?:vertrags)?laufzeit|festlaufzeit|feste\s+laufzeit|erstlaufzeit|mindestdauer|grundlaufzeit)[^.!?;:\n]{0,40}?(\d{1,3}|[a-z\u00e4\u00f6\u00fc\u00df]{3,20})\s*(?:kalender)?(monat|jahr|woche|tag)/i
          );
          const termN = mTerm ? parseGermanNumber(mTerm[1]) : NaN;
          const termMonths = Number.isFinite(termN) ? termN * UNIT_MONTH_FACTOR(mTerm[2]) : NaN;
          if (Number.isFinite(termMonths) && termMonths > 24) {
            flagged.push({ i, months: termMonths, termOnly: true });
            return;
          }
        }

        // Grundbezug ist die Kündigung bzw. deren synonyme Umschreibung
        // ("Vorlaufzeit", "Beendigung", "Periodenende", "Loslösung").
        const kuendigungsbezug =
          /(?:k(?:ü|ue)ndig|beendig|vorlaufzeit|periodenende|vertrags(?:ende|ablauf)|losl(?:ö|oe)sung|auslaufen)/i.test(text);
        if (!kuendigungsbezug) return;

        const re =
          /(?:\b|\()(\d{1,3}|[a-z\u00e4\u00f6\u00fc\u00df]{3,20})\)?\s*(?:\([^\)]+\)\s*)?(?:kalender)?(monat(?:e|en|es|s)?|woc(?:he|hen)|jahr(?:e|es|en|s)?|tag(?:e|en|s)?)\b/gi;
        let m;
        const linked = [];
        while ((m = re.exec(text))) {
          const n = parseGermanNumber(m[1]);
          if (!Number.isFinite(n)) continue;
          const months = n * UNIT_MONTH_FACTOR(m[2]);
          if (!Number.isFinite(months)) continue;
          const before = text.slice(0, m.index);
          // "Kündigungsfrist beträgt 6 Monate" ist eine Frist, keine Laufzeit.
          const fristSubject = /(?:k(?:ü|ue)ndigungs|ank(?:ü|ue)ndigungs|beendigungs)frist[^.!?;:\n]{0,80}$/i.test(before);
          const waiver =
            /(?:verzicht\w*)[^.!?;:\n]{0,140}?(?:k(?:ü|ue)ndig|beendig|losl(?:ö|oe)sung)/i.test(text);
          const isTerm = TERM_LINK.test(before) && !fristSubject;
          if (isTerm && !waiver) continue; // reine Laufzeitangabe
          if (waiver && isTerm) { linked.push(months); continue; }
          const afterShort = text.slice(m.index + m[0].length, m.index + m[0].length + 80);
          // "kündigt ... spätestens sieben Tage im Voraus an" ist eine Ankündigungs-
          // praxis, keine Kündigungsfrist.
          if (/im\s+voraus/i.test(afterShort) && !/(?:k(?:ü|ue)ndigungs|beendigungs)frist/i.test(text)) continue;
          const linkedByBefore =
            NOTICE_LINK.test(before) ||
            /(?:k(?:ü|ue)ndigungs|ank(?:ü|ue)ndigungs|beendigungs)frist[^.!?;:\n]{0,120}$/i.test(before);
          const linkedByAfter =
            /(?:k(?:ü|ue)ndigungs|ank(?:ü|ue)ndigungs|beendigungs|vertragslaufzeit|periodenende|zum\s+(?:ablauf|ende))/i.test(
              afterShort
            );
          if (!linkedByBefore && !linkedByAfter) continue;
          linked.push(months);
        }
        // Bindungstraps: Mindestlaufzeit plus Ausschluss der vorzeitigen Kündigung.
        if (
          /(?:mindest(?:vertrags)?laufzeit|festlaufzeit|feste\s+laufzeit|erstlaufzeit|mindestdauer)[\s\S]{0,60}?(?:erstmals\s+zum|fr(?:ü|ue)hestens|nicht\s+vor\s+ablauf|vorzeitige?\s+k(?:ü|ue)ndigung\s+ist\s+ausgeschlossen|k(?:ü|ue)ndigung\s+ist\s+ausgeschlossen)/i.test(
            text
          )
        ) {
          const term = text.match(
            /(?:mindest(?:vertrags)?laufzeit|festlaufzeit|feste\s+laufzeit|erstlaufzeit|mindestdauer)[^.!?;:\n]{0,40}?(\d{1,3}|[a-z\u00e4\u00f6\u00fc\u00df]{3,20})\s*(?:kalender)?(monat|jahr|woche|tag)/i
          );
          if (term) {
            const n = parseGermanNumber(term[1]);
            if (Number.isFinite(n)) linked.push(n * UNIT_MONTH_FACTOR(term[2]));
          }
        }
        // Verlängerungstraps: Verlängerung um X bei Opt-out kurz vor Ablauf.
        const renewal = text.match(
          /verl(?:ä|ae)ngert\s+sich[^.!?;:\n]{0,40}?um\s+(\d{1,3}|[a-z\u00e4\u00f6\u00fc\u00df]{3,20})\s*(?:kalender)?(monat|jahr|woche|tag)/i
        );
        if (renewal && /vor\s+ablauf/i.test(text) && /k(?:ü|ue)ndig/i.test(text)) {
          const n = parseGermanNumber(renewal[1]);
          if (Number.isFinite(n)) linked.push(n * UNIT_MONTH_FACTOR(renewal[2]));
        }
        if (linked.length === 0) return;
        const max = Math.max(...linked);
        pool.push({ i, months: max });
        // Symmetrische Kündigungsrechte beider Parteien mit bis zu 6 Monaten
        // Frist sind B2B-üblich und gesetzeskonform.
        const symmetric = /(?:beiden\s+parteien|jede[rn]?\s+partei|beide\s+seiten|f(?:ü|ue)r\s+beide\s+parteien)/i.test(text);
        if (max > 3.05 && !(symmetric && max <= 6.05)) flagged.push({ i, months: max });
      });

      // Ungleiche Fristen: eine Partei wesentlich länger gebunden als die andere.
      const values = pool.map((p) => p.months);
      const min = values.length ? Math.min(...values) : 0;
      const max = values.length ? Math.max(...values) : 0;
      const asymmetric = pool.length >= 2 && max / Math.max(min, 0.01) >= 2.5;
      if (asymmetric) {
        const top = pool.find((p) => p.months === max);
        if (top && !flagged.some((f) => f.i === top.i)) {
          flagged.push({ i: top.i, months: top.months, asymmetric: true });
        }
      }

      return flagged.map((f) => ({
        i: f.i,
        severity: f.months >= 11.5 ? "hoch" : "mittel",
        note: { months: fmtMonths(f.months), asymmetric, activationTrap: f.activationTrap, termOnly: f.termOnly },
      }));
    },
  },
  {
    type: "liability",
    title(ctx) {
      const s = ctx?.sentence || "";
      if (/anfechtung|irrtum/i.test(s)) return "Pauschaler Verzicht auf Widerrufs- und Anfechtungsrechte (§ 307, § 119 BGB)";
      if (/widerruf/i.test(s)) return "Unwirksamer Ausschluss des Widerrufsrechts (§ 356, § 361 BGB)";
      if (/sammel-|musterfeststellung|verbandsklag/i.test(s)) return "Unwirksamer Verzicht auf Sammelklagen (§ 307 BGB, VDuG)";
      if (/bewertung|rezension/i.test(s)) return "Unzulässiges Bewertungsverbot (Gag-Klausel)";
      if (/leben,\s+körper|körper\s+und\s+gesundheit/i.test(s)) return "Unzulässiger Haftungsausschluss für Körperschäden (§ 309 Nr. 7 lit. a BGB)";
      return "Weitgehender Haftungsausschluss oder Freistellung";
    },
    tags: ["bgb-309-nr-7", "bgb-307", "haftungsausschluss", "freistellung", "sole-remedy"],
    patterns: [
      /(?:haftungshöhe|gesamthaftung|haftung)[^.!?;:\n]{0,120}?(?:auf\s+maximal\s+(?:\d+|fünfzig)\s*(?:%|prozent)|gedeckelt|begrenzt)/i,
      // Evasion-Swarm: Indirekter Ausschluss über ausschließlichen Rechtsbehelf / Freizeichnung von Folgewirkungen
      /(?:schadensersatzansprüche|folge-?\s*oder\s+schadensersatzansprüche|ersatzansprüche|verpflichtungen|leistungserbringung)[^.!?;:\n]{0,180}?(?:ausschließlicher\s+rechtsbehelf|vergütungsanpassung\s+als\s+ausschließlicher\s+rechtsbehelf|freizeichnung\s+von\s+folgewirkungen|risikoübernahme\s+durch\s+den\s+besteller)/i,
      /(?:ausschließlicher\s+rechtsbehelf|freizeichnung\s+von\s+folgewirkungen)[^.!?;:\n]{0,140}?(?:schaden|haftung|anspruch|ansprüche|besteller|risikoübernahme)/i,
      /(?:haftung|schadensersatz)[^.!?;:\n]{0,180}?(?:abbedungen|ausgeschlossen|erloschen|erlöschen)/i,
      /(?:service[\s-]level[\s-]credits?|service[\s-]credits?|gutschrift\w*)[^.!?;:\n]{0,100}?(?:ausschließlich|nur\s+auf\s+zukünftige|verfallen\s+ersatzlos)/i,
      /(?:haftung|schadensersatz|schadensersatzansprüche|ersatzansprüche)[^.!?;:\n]{0,120}?(?:abbedungen|ausgeschlossen|erloschen|erlöschen)/i,
      /(?:abbedungen|ausgeschlossen)[^.!?;:\n]{0,60}?(?:haftung|schadensersatz)/i,
      /(?:schließt|schließen)\s+[^.!?;:\n]{0,60}?haftung/i,
      /keine\s+haftung/i,
      /übernimmt\s+kein\w*\s+haftung/i,
      /haftet?\s+(?:nicht|keinesfalls|niemals|in\s+keinem\s+fall)/i,
      /freistell\w*/i,
      /stell[etn]+\s+[^.!?;:\n]{0,100}?\bfrei\b/i,
      /freizuhalten/i,
      // Red Team: Schadensersatzansprüche abbedungen / Summenbegrenzung / Invertierte Wortstellung
      /(?:schadensersatzansprüche|ersatzansprüche)[^.!?;:\n]{0,260}?(?:abbedungen|ausgeschlossen|erlöschen)/i,
      /(?:verpflichtung[^.!?;:\n]{0,80}?zum\s+ersatz\s+von\s+schäden|ersatz\s+von\s+schäden|schadensersatz|haftung)[^.!?;:\n]{0,320}?(?:der\s+höhe\s+nach\s+auf|auf\s+(?:die\s+höhe\s+des|den)|auf\s+die\s+summe)[^.!?;:\n]{0,120}?(?:beschränkt|begrenzt|gedeckelt|limitiert)/i,
      /(?:verpflichtung[^.!?;:\n]{0,80}?zum\s+ersatz\s+von\s+schäden|ersatz\s+von\s+schäden|schadensersatz|haftung)[^.!?;:\n]{0,320}?(?:auf\s+(?:die\s+höhe\s+des|den)[^.!?;:\n]{0,100}?(?:beschränkt|begrenzt|limitiert)|(?:beschränkt|begrenzt|limitiert)\s+auf)/i,
      /(?:beschränkt|begrenzt|gedeckelt)[^.!?;:\n]{0,80}?(?:auf\s+die\s+summe[^.!?;:\n]{0,80}?)?(?:in\s+den\s+letzten\s+(?:\d+|zwei|drei|sechs)\s+monaten|monatsvergütungen|monatsbeiträgen)/i,
      // Überspannte Mitverschuldenspflicht / Sham Data Loss Backup Trap (§ 307 BGB)
      /(?:für\s+datenverluste\s+haftet|haftung\s+für\s+datenverlust\w*)[^.!?;:\n]{0,140}?(?:nur\s+in\s+dem\s+umfang|nur\s+insoweit)[^.!?;:\n]{0,140}?(?:vollsicherung|arbeitstäglich\w*|manuell\w*)/i,
      /(?:arbeitstäglich\w*|manuell\w*)[^.!?;:\n]{0,80}?(?:extern\w*\s+)?vollsicherung/i,
      // Adversarial / Sole Remedy Trap (§ 307 Abs. 2 Nr. 1 BGB, § 314 BGB):
      // Service Credit / Gutschrift als abschließende/ausschließliche Kompensation, weitergehende Ansprüche (Minderung, Rücktritt, Schadensersatz) abgegolten
      /(?:service[\s-]credit|gutschrift)[^.!?;:\n]{0,120}?(?:abschließende|ausschließliche)\s+kompensation/i,
      /(?:abschließende|ausschließliche)\s+kompensation[^.!?;:\n]{0,160}?(?:weitergehende\s+ansprüche|abgegolten|minderung|rücktritt)/i,
      /weitergehende\s+ansprüche[^.!?;:\n]{0,120}?(?:minderungsrechte|minderung|rücktritt|schadensersatz)[^.!?;:\n]{0,80}?(?:abgegolten|ausgeschlossen)/i,
      // Haftung nur für Vorsatz (Aushebelung von grober Fahrlässigkeit und Kardinalpflichten)
      /(?:haftet?|haftung\s+besteht|haftung\s+erfolgt)\s+(?:nur|ausschließlich|lediglich)\s+(?:bei|für|im\s+falle\s+von)\s+vorsatz/i,
      /(?:haftung|schadensersatz)[^.!?;:\n]{0,60}?(?:nur\s+bei\s+vorsatz|nur\s+für\s+vorsatz)/i,
      /(?:haftung|schadensersatz)[^.!?;:\n]{0,120}?(?:auf\s+vorsatz\s+(?:und\s+grobe\s+fahrlässigkeit\s+)?beschränkt|beschränkt\s+auf\s+vorsatz)/i,
      /(?:rücktritt\s+und\s+minderung|minderung\s+und\s+rücktritt)[^.!?;:\n]{0,60}?(?:sind\s+ausgeschlossen|ausgeschlossen)/i,
      /(?:unbegrenzte\s+anzahl|beliebig\s+viele)\s+(?:von\s+)?nachbesserungsversuchen?/i,
      /(?:haftung|ersatzansprüche)[^.!?;:\n]{0,120}?(?:ausgeschlossen|beschränkt)[^.!?;:\n]{0,120}?(?:für\s+schäden\s+an\s+leben,\s+körper|leben,\s+körper\s+und\s+gesundheit)/i,
      /(?:für\s+schäden\s+an\s+leben,\s+körper\s+(?:und|oder)\s+gesundheit|dies\s+gilt\s+insbesondere\s+für\s+schäden[^.!?;:\n]{0,140}?(?:leben,\s+körper|körper\s+und\s+gesundheit))/i,
      // Verbraucher-Knebel: Widerrufsverzicht, Klageverzicht, Bewertungsverbot
      /(?:widerrufsrecht[^.!?;:\n]{0,120}?(?:vollständig\s+)?erlischt|verzicht\w*[^.!?;:\n]{0,80}?auf\s+(?:das\s+)?widerrufsrecht)/i,
      /(?:durch\s+(?:die\s+)?(?:aktivierung|anmeldung|inbetriebnahme|anschließen)[^.!?;:\n]{0,120}?verzichtet[^.!?;:\n]{0,80}?widerrufsrecht)/i,
      /(?:widerrufsbelehrung\s+verwiesen,\s+die\s+auf\s+anforderung|widerrufsbelehrung[^.!?;:\n]{0,60}?(?:nur\s+)?auf\s+anforderung)/i,
      /(?:verzicht\w*[^.!?;:\n]{0,80}?auf\s+(?:die\s+)?teilnahme\s+an\s+(?:sammel-|musterfeststellungsklagen|verbandsklagen)|ausschluss\s+von\s+(?:sammelklagen|verbandsklagen))/i,
      /(?:keine\s+öffentlichen\s+negativen\s+bewertungen|bewertungsverbot|verpflichtet\s+sich,[^.!?;:\n]{0,60}?keine\s+(?:negativen\s+)?bewertungen)/i,
      // Verzicht auf Ersatzansprüche sowie lang gestreckte Kategorie-Ausschlüsse
      // (typisch fuer Cyber-/MDR-MSAs, in denen Schadenskategorien aufgezaehlt werden):
      /verzicht\w*[^.!?;:\n]{0,60}?auf\s+(?:sämtliche\s+|alle\s+|jegliche\s+)?(?:ersatz-?\s*und\s*)?(?:ersatzansprüche|schadensersatzansprüche|schadensersatz|ansprüche)[^.!?;:\n]{0,140}?(?:betriebsunterbrechung|datenverlust|transaktionsausfall|schäden|maßnahmen|sicherheitsvorfall)/i,
      /verzicht\w*\s+(?:ausdrücklich\s+|hiermit\s+)?auf\s+(?:sämtliche\s+|alle\s+|jegliche\s+)?ersatzansprüche/i,
      /(?:sieht|sehen)[^;:\n]{0,60}?von\s+(?:sämtlichen\s+|allen\s+|jeglichen\s+)?(?:ansprüchen|ersatzansprüchen|schadensersatzansprüchen|haftungsansprüchen)\s+ab/i,
      /(?:haftung|gewähr|gewährleistung|garantie|einstandspflicht|schäden|vermögensschäden|ersatz)[^;:\n]{0,60}?(?:wird|werden)[^;:\n]{0,40}?nicht\s+(?:eingestanden|übernommen)\b/i,
      /(?:nur|lediglich)\s+bis\s+(?:zur\s+)?(?:höhe|summe)[^;:\n]{0,90}?(?:entgelt|vergütung|gebühr|honorar|wert|auftragssumme)/i,
      /(?:die\s+)?haftung\s+für(?:(?!nicht\b)[^.!?;:\n]){0,300}?(?:vollumfänglich\s+|vollständig\s+|gänzlich\s+|ausdrücklich\s+)?ausgeschlossen/i,
      // English limitation of liability
      /(?:maximum\s+liability|aggregate\s+liability|total\s+liability)[^.!?;:\n]{0,80}?(?:shall\s+be\s+limited\s+to|is\s+limited\s+to|shall\s+not\s+exceed)/i,
      /(?:in\s+no\s+event\s+shall|shall\s+not\s+be\s+liable|disclaims\s+all\s+warranties|exclusive\s+remedy|limitation\s+of\s+liability)[^.!?;:\n]{0,140}?(?:indirect|consequential|incidental|punitive|lost\s+profits|all\s+damages|any\s+damages)/i,
      // Erfüllungsgehilfen-Ausschluss (§ 278 BGB, § 309 Nr. 7 lit. b BGB)
      /(?:haftung|verschulden|einstehen)[^.!?;:\n]{0,140}?erfüllungsgehilfen[^.!?;:\n]{0,100}?(?:ausgeschlossen|ausbedungen|abbedungen|nicht\s+gehaftet|haftet\s+nicht)/i,
      /(?:erfüllungsgehilfen|unterauftragnehmer)[^.!?;:\n]{0,60}?(?:ist|sind|wird|werden)[^.!?;:\n]{0,40}?(?:ausgeschlossen|nicht\s+gehaftet)/i,
      // Euphemismen & Synonyme für Haftungsausschluss / -deckel
      /(?:ansprüche|schadensersatz|ersatz|haftung|ersatzpflicht)[^.!?;:\n]{0,160}?(?:finden\s+ihre\s+abschließende\s+erledigung|abschließende\s+erledigung|sind\s+hiermit\s+abschließend\s+abgegolten)/i,
      /(?:ansprüche|schadensersatz|ersatzansprüche|ersatz|haftung)[^.!?;:\n]{0,120}?(?:wirtschaftlich|praktisch|faktisch|rechtlich)\s+nicht\s+(?:vorgesehen|gewollt|erfasst|gewünscht|beabsichtigt)/i,
      /(?:haftung|einstandspflicht|gewährleistung|ersatzpflicht)[^.!?;:\n]{0,100}?(?:ist|wird|wurde|sind|werden)[^.!?;:\n]{0,30}?(?:ausbedungen|aufgehoben|abbedungen)/i,
      /(?:mittelbare|unmittelbare|entgangene)[^.!?;:\n]{0,120}?(?:nachteile|schäden|folgeschäden|gewinn)[^.!?;:\n]{0,80}?(?:ausbedungen|aufgehoben|abbedungen|nicht\s+zu\s+vertreten|entfällt)/i,
      // Haftungsdeckel erfasst ausdrücklich auch Kardinalpflichten
      /(?:haftungsbeschränkung|haftungsbegrenzung|haftung[^.!?;:\n]{0,40}?(?:begrenzt|beschränkt|gedeckelt))[^.!?;:\n]{0,120}?(?:auftrags|einzelauftrags|netto)?summe[^.!?;:\n]{0,140}?(?:gilt\s+auch\s+f(?:ü|ue)r|umfasst\s+auch|erfasst\s+auch)[^.!?;:\n]{0,80}?kardinalpflichten/i,
      // Monatsdeckel relativ zur anspruchsbegründenden Pflichtverletzung
      /(?:ersatz|haftung|ersatzpflicht|deckel)[^.!?;:\n]{0,140}?(?:monate\s+vor\s+der\s+anspruchsbegründenden|vor\s+der\s+anspruchsbegründenden\s+pflichtverletzung)/i,
      // Negierte Verantwortungsübernahme / Haftung nur bei Vorsatz (Split-Satz)
      /(?:keine\s+verantwortung|verantwortung)[^.!?;:\n]{0,40}?nicht\s+(?:übernommen|getragen)/i,
      /(?:haftung|verantwortung)[^.!?;:\n]{0,120}?(?:ausschließlich|nur)\s+(?:bei|für)\s+vorsätzlichem\s+handeln/i,
    ],
    severity(s) {
      // Gesetzeskonforme Klausel: einfache/leichte Fahrlässigkeit begrenzt auf vertragstypischen
      // Schaden, Kardinalpflichten ausdrücklich einbezogen, kein Ausschluss/Deckel.
      if (
        /(?:einfache|leichte|leicht)\s+fahrl(?:ä|ae)ssig/i.test(s) &&
        /(?:kardinalpflicht|wesentliche\s+vertragspflicht|wichtige\s+vertragspflicht|vertragstypische\w*[^.!?;:\n]{0,30}?vorhersehbare\w*\s+schaden)/i.test(s) &&
        !/(?:ausgeschlossen|abbedungen|keine\s+haftung|vollumf(?:ä|ae)nglich|g(?:ä|ae)nzlich|monatsverg(?:ü|ue)tung|monatsentgelt|in\s+den\s+letzten\s+(?:\d+|zwei|drei|sechs|zw(?:ö|oe)lf)\s+monaten|auftragssumme|einzelauftragssumme|\d+\s*(?:%|prozent)|haftungsbeschr(?:ä|ae)nkung|gedeckelt)/i.test(s) &&
        !/vorsatz/i.test(s)
      ) {
        return null;
      }
      // BGH-konforme Begrenzung bei Datenverlust auf den typischen Wiederherstellungsaufwand
      if (/(?:wiederherstellungsaufwand|typischen\s+wiederherstellungsaufwand)/i.test(s)) {
        return null;
      }
      // Englisches MSA mit gesetzlich zulässigen Carve-outs (Tod/Körper/Fraud/Vorsatz)
      if (
        /(?:death|personal\s+injury|fraud|intentional\s+misconduct|wilful\s+misconduct|willful\s+misconduct)/i.test(s) &&
        /(?:except\s+for|not\s+(?:to\s+be\s+)?limited|are\s+not\s+limited|is\s+not\s+limited|remain\s+unaffected|cannot\s+be\s+waived|liability\s+is\s+limited\s+for)/i.test(s)
      ) {
        return null;
      }
      // Marktüblicher Jahresgebühren-Cap mit Verweis auf vorstehende Carve-out-Regel
      if (
        /subject\s+to\s+the\s+(?:preceding|foregoing)/i.test(s) &&
        /fees\s+(?:paid\s+or\s+payable|paid)[^.!?;:\n]{0,80}?(?:twelve|12)\s+months/i.test(s)
      ) {
        return null;
      }
      if (/(?:widerruf|musterfeststellung|sammelklag|verbandsklag|bewertung)/i.test(s)) return "hoch";
      // Haftungsdeckel/Verantwortungsverzicht (auch auf Kardinalpflichten)
      if (
        /haftungsbeschr(?:ä|ae)nkung|haftungsbegrenzung|(?:auftrags|einzelauftrags|netto)summe|anspruchsbegr(?:ü|ue)ndenden\s+pflichtverletzung|verantwortung[^.!?;:\n]{0,50}?nicht\s+(?:übernommen|getragen)|vorsätzlichem\s+handeln|abschließende\s+erledigung|ausbedungen|abbedungen|abgegolten|erfüllungsgehilfen|unterauftragnehmer[^.!?;:\n]{0,60}?(?:ausgeschlossen|nicht\s+gehaftet)/i.test(
          s
        )
      ) {
        return "hoch";
      }
      if (/(?:haftet?|haftung\s+besteht)\s+(?:nur|ausschließlich|lediglich)\s+(?:bei|für)\s+vorsatz|auf\s+vorsatz\s+beschränkt|beschränkt\s+auf\s+vorsatz|unbegrenzte\s+anzahl|rücktritt\s+und\s+minderung\s+sind\s+ausgeschlossen/i.test(s)) return "hoch";
      if (/in\s+no\s+event\s+shall|shall\s+not\s+be\s+liable|limitation\s+of\s+liability|maximum\s+liability|aggregate\s+liability|total\s+liability/i.test(s)) return "hoch";
      // Ausdrücklicher Verzicht auf Ersatzansprüche (z. B. für Abwehr-/Isolationsmaßnahmen):
      if (/verzicht\w*[^.!?;:\n]{0,60}?auf\s+(?:sämtliche\s+|alle\s+|jegliche\s+)?(?:ersatzansprüche|schadensersatzansprüche|schadensersatz)/i.test(s)) return "hoch";
      if (/von\s+(?:sämtlichen\s+|allen\s+|jeglichen\s+)?(?:ansprüchen|ersatzansprüchen|schadensersatzansprüchen)\s+ab|(?:haftung|gewähr|schäden)[^.!?;:\n]{0,60}?nicht\s+(?:eingestanden|übernommen)|bis\s+(?:zur\s+)?(?:höhe|summe)[^.!?;:\n]{0,90}?(?:entgelt|vergütung|gebühr|honorar)/i.test(s)) return "hoch";

      // Unzulässige Beschränkungen von Kardinalpflichten (z. B. auf 2 Monatsvergütungen) oder Schein-Haftung für Datenverlust:
      const lowCapOrDataLoss =
        /(?:in\s+den\s+letzten\s+(?:\d+|zwei|drei|sechs)\s+monaten|arbeitstäglich|vollsicherung)/i.test(s);
      if (lowCapOrDataLoss) return "hoch";

      // Haftungsdeckel auf eine kleine Bemessungsgrundlage (Monatsmiete, Monatspauschale,
      // Prozentsatz des Kaufpreises, Honorar) verkuerzt die Haftung fuer Kardinalpflichten
      // der Hoehe nach und ist nach § 307 Abs. 2 Nr. 2 BGB unwirksam.
      if (
        /(?:haftung|einstandspflicht|ersatzpflicht)[^.!?;:\n]{0,90}?(?:auf|bis\s+(?:zur|zu))[^.!?;:\n]{0,40}?(?:eine?[mn]?\s+monatsmiete\w*|(?:zwei|drei|vier|f(?:ü|ue)nf|sechs|zehn|zwanzig|\d{1,3})\s+monatsmieten|monatspauschale\w*|monatsverg(?:ü|ue)tung\w*|monatsentgelt\w*|jahresverg(?:ü|ue)tung\w*|h(?:ö|oe)he\s+des\s+\w*honorars?|(?:zehn|zwanzig|\d{1,3})\s*(?:%|prozent)\s+des\s+kaufpreises)[^.!?;:\n]{0,40}?(?:begrenzt|beschr(?:ä|ae)nkt|gedeckelt|limitiert)/i.test(s)
      ) {
        return "hoch";
      }

      // Wenn die Klausel den BGB-Katalog sauber enthält ("Kardinalpflichten", "Leben, Körper, Gesundheit") ohne künstlichen Cap:
      if (/leben|körper|gesundheit/i.test(s) && /kardinalpflicht|wesentliche\s+vertragspflicht/i.test(s)) {
        return null; // Saubere gesetzliche Klausel -> kein False Positive!
      }
      if (/unbeschr(?:ä|ae)nkt/i.test(s) && /(?:leben|k(?:ö|oe)rper|gesundheit|vorsatz|grobe\s+fahrl)/i.test(s) && !/(?:ausgeschlossen|abbedungen|keine\s+haftung)/i.test(s)) {
        return null;
      }
      if (/leben,\s+körper|körper\s+und\s+gesundheit/i.test(s)) return "hoch";
      const soleRemedy =
        /(?:abschließende|ausschließliche)\s+kompensation|ausschließlicher\s+rechtsbehelf|freizeichnung\s+von\s+folgewirkungen|weitergehende\s+ansprüche[^.!?;:\n]{0,100}?(?:minderung|rücktritt|schadensersatz).*?abgegolten/i.test(s);
      const strictIndemnity =
        /freistell\w*[^.!?;:\n]{0,60}?verschuldensunabhängig|verschuldensunabhängig[^.!?;:\n]{0,60}?freistell\w*/i.test(s);
      const exclusion =
        /ausgeschlossen|keine\s+haftung|haftet?\s+(?:nicht|keinesfalls|niemals)|nicht\s+gehaftet|ist\s+nicht\s+er(?:ö|oe)ffnet|abbedungen|der\s+höhe\s+nach\s+auf|auf\s+(?:die\s+höhe|den|die\s+summe)[^.!?;:\n]{0,100}?(?:beschränkt|begrenzt|limitiert)|auf\s+(?:die\s+)?(?:monatspauschale|lizenzgeb(?:ü|ue)hr|jahresgeb(?:ü|ue)hr|\d[\d.,]*\s*(?:eur|euro|chf))[^.!?;:\n]{0,40}?(?:beschr(?:ä|ae)nkt|begrenzt|gedeckelt|limitiert)|(?:beschränkt|begrenzt|limitiert)\s+auf|nur\s+in\s+dem\s+umfang/i.test(
          s
        );
      const indemnity =
        /freistell|freizuhalten|\b(?:hält|halten|stellt|stellen)\b[^.!?;:\n]{0,120}?\bfrei\b/i.test(s) &&
        !/elternzeit|freistellung\s+(?:von\s+der\s+arbeit|auf\s+die\s+zeit|von\s+grundpfandrechten)/i.test(s);
      const broad =
        /sämtliche[rns]?|jegliche[rns]?|alle[rns]?|uneingeschränkt|unbegrenzt|aller\s+art|welcher\s+art|gleich\s+aus/i.test(
          s
        );
      if (soleRemedy || strictIndemnity) return "hoch";
      if (exclusion) return "hoch";
      if (indemnity && broad) return "hoch";
      if (indemnity) return "mittel";
      return null;
    },
    why(ctx) {
      const s = ctx?.sentence || "";
      if (/anfechtung|irrtum/i.test(s)) {
        return "Ein formularmäßiger Generalverzicht auf gesetzliche Gestaltungsrechte, hier auf Widerruf und auf die Anfechtung wegen Irrtums, nimmt dem Vertragspartner Rechte, die ihm das Gesetz ohne Ausnahme zubilligt. Ein solcher Pauschalverzicht benachteiligt ihn unangemessen (§ 307 BGB, § 119 BGB, § 123 BGB).";
      }
      if (/widerruf/i.test(s)) {
        return "Ein formularmäßiger Verzicht auf das gesetzliche Widerrufsrecht oder die Ausdehnung des Erlöschens von Dienstleistungen auf gekoppelte Hardware ist nach § 356 Abs. 4 BGB und § 361 BGB unwirksam.";
      }
      if (/sammel-|musterfeststellung|verbandsklag/i.test(s)) {
        return "Der formularmäßige Ausschluss von gesetzlichen Verbands- und Musterfeststellungsklagen (§ 307 BGB, VDuG) benachteiligt Kunden unangemessen und ist unwirksam.";
      }
      if (/bewertung|rezension/i.test(s)) {
        return "Klauseln, die sachliche oder wahrheitsgemäße Bewertungen untersagen oder an unzumutbare Vorab-Bedingungen koppeln, verstoßen gegen Art. 5 GG und § 307 BGB.";
      }
      if (/leben,\s+körper|körper\s+und\s+gesundheit/i.test(s)) {
        return "Ein Haftungsausschluss für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit ist nach § 309 Nr. 7 lit. a BGB absolut unzulässig.";
      }
      if (/in\s+den\s+letzten\s+(?:\d+|zwei|drei|sechs)\s+monaten/i.test(s)) {
        return "Aushöhlung von Kardinalpflichten (§ 307 Abs. 2 Nr. 2 BGB): Die Haftung für vertragswesentliche Pflichten darf der Höhe nach nicht auf Entgelte der letzten Monate gedeckelt werden. Dies untergräbt das Synallagma und führt im Schadensfall zu einer unzureichenden Absicherung.";
      }
      if (/arbeitstäglich|vollsicherung|datenverlust/i.test(s)) {
        return "Überspannte Datensicherungspflicht & faktischer Haftungsausschluss (§ 307 BGB): Die Haftung für Datenverluste wird an weltfremde Pflichten (wie arbeitstägliche manuelle externe Vollsicherung) gekoppelt. Da ein Datenverlust bei lückenloser Vollsicherung kaum eintritt, läuft dies auf einen verdeckten Haftungsausschluss hinaus.";
      }
      if (/(?:service[\s-]credit|gutschrift|abschließende|ausschließliche\s+kompensation|abgegolten)/i.test(s)) {
        return "Sole Remedy Trap (§ 307 Abs. 2 Nr. 1, § 314 BGB): Eine Klausel, die Service Credits oder Gutschriften zur ausschließlichen Kompensation erklärt und gesetzliche Minderungs-, Rücktritts- oder Kündigungsrechte abbedingt, höhlt Kardinalpflichten unzulässig aus.";
      }
      if (/verschuldensunabhängig/i.test(s)) {
        return "Gefahrverlagerung & verschuldensunabhängige Freistellung (§ 307 Abs. 1, 2 BGB): Der Kunde wird ohne eigenes Verschulden zur Freistellung und Kostenübernahme für Systemstörungen verpflichtet.";
      }
      return "Der Anbieter will Verantwortung für Fehler fast vollständig abgeben bzw. Sie umgekehrt für fast alles haftbar machen. Im Schadensfall stehen Sie dann allein da – auch bei groben Fehlern des Anbieters.";
    },
    suggestion(ctx) {
      const s = ctx?.sentence || "";
      if (/anfechtung|irrtum/i.test(s)) return "Verzicht auf Widerrufs- und Anfechtungsrechte streichen. Gesetzliche Gestaltungsrechte (§ 119, § 123 BGB) müssen uneingeschränkt offenstehen.";
      if (/widerruf/i.test(s)) return "Gesetzliches 14-tägiges Widerrufsrecht für Hardware und unvollständig erbrachte Services uneingeschränkt aufrechterhalten.";
      if (/sammel-|musterfeststellung|verbandsklag/i.test(s)) return "Ausschluss ersatzlos streichen. Gesetzliche Rechtswege und kollektiver Rechtsschutz müssen uneingeschränkt offenstehen.";
      if (/bewertung|rezension/i.test(s)) return "Bewertungsverbote ersatzlos streichen. Kunden haben das Recht auf freie, sachliche Meinungsäußerung.";
      if (/leben,\s+körper|körper\s+und\s+gesundheit/i.test(s)) return "Haftung für Verletzungen von Leben, Körper und Gesundheit unbeschränkt erhalten.";
      return "Haftung mindestens für Vorsatz, grobe Fahrlässigkeit und Verletzung wesentlicher Vertragspflichten erhalten. Freistellungen auf eigenes grobes Verschulden begrenzen und gesetzliche Minderungs-/Kündigungsrechte unberührt lassen.";
    },
  },
  {
    type: "penalty",
    title: "Vertragsstrafe oder pauschalierter Schadensersatz",
    tags: ["bgb-309-nr-6", "bgb-309-nr-5", "vertragsstrafe", "poenale"],
    patterns: [
      /(?:vorhalte-?\s*(?:und\s+)?leerlaufkosten|bereitstellungskosten)[^.!?;:\n]{0,80}?(?:nach\s+(?:den\s+)?(?:regulären\s+)?tagessätzen|vollumfänglich\s+zu\s+erstatten)/i,
      // Evasion-Swarm: Bearbeitungsgebühr für Pflichtabweichungen / Verschuldensunabhängige Sondergebühr
      /(?:bearbeitungsgebühr|sondergebühr|bearbeitungspauschale)\s+(?:für|bei|im\s+falle\s+einer?)\s+(?:pflichtabweichungen?|vertragsverletzungen?|pflichtverletzungen?)[^.!?;:\n]{0,140}?(?:euro|eur|\d+|verstoß|vertragswerts?|%)/i,
      /(?:beträgt|in\s+höhe\s+von)\s+(?:\d+[\d,.]*\s*(?:euro|eur|%)|\d+\s*%(?:[^.!?;:\n]{0,30}?vertragswerts?)?)[^.!?;:\n]{0,80}?(?:pro|je|für\s+die|bei)\s+(?:verstoß|pflichtabweichung|abweichung|bearbeitung)/i,
      /(?:pflichtabweichung|vertragsabweichung)[^.!?;:\n]{0,80}?(?:pauschale\s+bearbeitungsgebühr|mindestens\s+\d+[\d,.]*\s*(?:euro|eur))/i,
      /vertragsstrafe|vertragsbu(?:ß|ss)e|konventionalstrafe|pönale|poenale/i,
      /strafe\s+(?:in\s+höhe|höhe)\s+von/i,
      // Getarnte Vertragsstrafe: "verwirkt eine ... in Höhe von", "Aufwandspauschale"
      /verwirkt[^.!?;:\n]{0,80}?(?:bu(?:ß|ss)e|strafe|pauschale|konventionalstrafe)/i,
      /(?:überschreitung|versp(?:ä|ae)tung|verzug|zuwiderhandlung)[^.!?;:\n]{0,120}?(?:aufwands-?\s*)?pauschale[^.!?;:\n]{0,120}?(?:prozent|%|eur|euro|f(?:ä|ae)llig)/i,
      /nachweis[^.!?;:\n]{0,140}?nicht\s+(?:zugelassen|gestattet|zul(?:ä|ae)ssig|m(?:ö|oe)glich)\b/i,
      // Red Team: Pauschalierter Schadensersatz mit Ausschluss des Gegenbeweises (§ 309 Nr. 5 lit. b BGB)
      /schadenspauschale/i,
      /pauschaliert\w*\s+schadensersatz/i,
      /(?:rücklastschrift[^.!?;:\n]{0,60}?(?:bearbeitungsgebühr|gebühr|pauschale)|mahnungen\s+werden\s+mit[^.!?;:\n]{0,30}?(?:berechnet|erhoben)|mahngebühr\s+von\s+(?:\d+|zehn|fünfzehn)\s*eur)/i,
      /(?:fehlende\w*\s+originalverpackung\w*[^.!?;:\n]{0,40}?pauschale|nutzungsentschädigung\s+in\s+höhe\s+eines\s+vollen\s+monatsentgelts)/i,
      /(?:pauschale\s+von\s+\d+[\d,.]*\s*(?:eur|euro)[^;:\n]{0,30}?(?:je|pro)\s*(?:werktag|tag|kalendertag|monat)|wird\s+eine\s+pauschale\s+von)/i,
      // Getarnte Pauschalierung / Bereitstellungsvergütung (§ 309 Nr. 5 lit. b BGB):
      /(?:pauschaliert\w*\s+)?bereitstellungsvergütung/i,
      /reservierten\s+(?:entwickler-?\s*)?kapazitäten[^.!?;:\n]{0,80}?(?:abzurechnen|abrechnen|pauschal)/i,
      /(?:nachweis|der\s+nachweis)\s+(?:eines\s+)?(?:geringeren|niedrigeren|höheren|entstandenen)\s+schadens[^;:\n]{0,60}?(?:nicht\s+zulässig|ist\s+nicht\s+zulässig|ausgeschlossen|nicht\s+gestattet|entfällt)/i,
      /(?:bleibt|ist|wird|sind)[^;:\n]{0,40}?unabhängig\s+davon\s+geschuldet/i,
      /unabhängig\s+davon,\s+ob[^;:\n]{0,80}?(?:schaden|aufwand)[^;:\n]{0,40}?(?:entstanden|angefallen)/i,
      /(?:fehlbetrag|differenzbetrag|mindererlös|abstandszahlung|abstandssumme)[^;:\n]{0,80}?(?:in\s+höhe\s+von|als\s+(?:ausgleich|pauschale)|geschuldet)/i,
      /(?:kosten|aufwendungen)[^.!?;:\n]{0,80}?(?:pauschal\s+berechnen|pauschalieren|pauschal\s+in\s+rechnung\s+stellen)/i,
      /(?:pauschal\s+berechnen|pauschal\s+in\s+rechnung\s+stellen)/i,
      /(?:änderung\s+der\s+bankverbindung|rechnung\s+in\s+papierform)[^.!?;:\n]{0,60}?(?:\d+[\d,.]*\s*eur|gebühr|pauschale)/i,
      /(?:nachweis\s+eines\s+geringeren\s+aufwands\s+nicht\s+gestattet)/i,
    ],
    collect(sentences, _full, extra = null) {
      const wholePara = sentences.map((s) => s.text).join(" ");
      const isExcluded =
        /nicht\s+(?:gestattet|möglich|zulässig|eingeräumt|erlaubt|zugelassen)|ausgeschlossen|entfällt/i.test(wholePara) ||
        /nachweis[^.!?;:\n]{0,90}?nicht\s+(?:vorbehalten|gestattet|zugelassen|m(?:ö|oe)glich)/i.test(wholePara);
      const hasMitigation =
        !isExcluded &&
        (/nachweis[^.!?;:\n]{0,80}?(?:wesentlich\s+geringer\w*|geringer\w*)[^.!?;:\n]{0,60}?(?:vorbehalten|gestattet|möglich)/i.test(wholePara) ||
        (/nachweis[^.!?;:\n]{0,80}?(?:vorbehalten|gestattet|möglich)/i.test(wholePara) && !/überhaupt\s+kein|keinerlei/i.test(wholePara)));
      if (hasMitigation) return [];
      return collectPerSentence(this, sentences, null, extra);
    },
    severity(s) {
      // Unzulässiger Ausschluss des Gegenbeweises (§ 309 Nr. 5 lit. b BGB):
      if (
        /nicht\s+(?:gestattet|möglich|zulässig|eingeräumt|erlaubt|zugelassen)|ausgeschlossen|entfällt/i.test(s) &&
        /nachweis/i.test(s)
      ) {
        return "hoch";
      }
      // Nachweis-Knebel: Verlangt unzulässigerweise Vollbeweis des völligen Nichtentstehens von Aufwand/Schaden (§ 309 Nr. 5 lit. b BGB)
      if (/überhaupt\s+kein\w*|keinerlei\s+(?:aufwand|schaden)/i.test(s)) {
        return "hoch";
      }
      // Gesetzmäßiger Vorbehalt: Ausdrücklicher Nachweis eines wesentlich geringeren Schadens/Aufwands ist gestattet
      if (/nachweis[^.!?;:\n]{0,80}?(?:wesentlich\s+geringer\w*|geringer\w*)[^.!?;:\n]{0,60}?(?:vorbehalten|gestattet|möglich)/i.test(s)) {
        return null;
      }
      if (/bereitstellungsvergütung|reservierten\s+entwicklerkapazitäten/i.test(s)) {
        return "hoch";
      }
      if (/nachweis[^.!?;:\n]{0,80}?(?:vorbehalten|gestattet|möglich)/i.test(s)) return null; // Gesetzeskonformer Vorbehalt!
      return "hoch";
    },
    why(ctx) {
      const s = ctx.sentence || "";
      if (/überhaupt\s+kein|bereitstellungsvergütung/i.test(s)) {
        return "Unzulässige Pauschalierung & Nachweis-Knebel (§ 309 Nr. 5 lit. b BGB): Eine pauschalierte Vergütung/Entschädigung ist unwirksam, wenn dem Kunden nicht der Nachweis eines wesentlich geringeren Schadens/Aufwands offengehalten wird, sondern gefordert wird zu beweisen, dass 'überhaupt kein' Aufwand entstand.";
      }
      return "Bei Verstößen droht eine fest vereinbarte Geldstrafe oder ein pauschalierter Schadensersatz ohne Möglichkeit des Gegenbeweises (§ 309 Nr. 5 lit. b / Nr. 6 BGB).";
    },
    suggestion:
      "Vertragsstrafen streichen oder stark begrenzen: nur klare Ausnahmefälle, Obergrenze in Euro und ausdrückliches Recht auf Nachweis eines geringeren Schadens.",
  },
  {
    type: "jurisdiction",
    title(ctx) {
      const s = ctx?.sentence || "";
      if (/(?:es\s+gilt|anwendbar|unterliegt|governed\s+by)[^.!?;:\n]{0,80}?(?:recht|law)/i.test(s) && !/gerichtsstand|gerichte/i.test(s)) {
        return "Anwendbares Recht im Ausland (Rechtswahl)";
      }
      return "Gerichtsstand beim Anbieter oder im Ausland";
    },
    tags: ["bgb-305c", "zpo-38", "gerichtsstand", "kostenrisiko"],
    patterns: [
      /ausschließliche?r?\s+gerichtsstand/i,
      /gerichtsstand\s+ist[^.!?;:\n]{0,80}/i,
      /als\s+gerichtsstand\s+gilt/i,
      /gerichtsstand\s+[^.!?;:\n]{0,40}(vereinbart|bestimmt|gewählt)/i,
      // Red Team: Forum-Formulierungen ohne das Wort 'Gerichtsstand'
      /ausschließlich\s+vor\s+den\s+[^.!?;:\n]{0,80}?gerichten\s+am\s+sitz\s+des\s+(?:anbieters|dienstleisters)/i,
      /(?:unterliegt|anwendbar\s+ist|gilt)[^.!?;:\n]{0,80}?(?:recht\s+des\s+staates|delaware|schweizer\s+recht|englischem?\s+recht|recht\s+der\s+vereinigten\s+staaten|us-amerikanischem\s+recht|recht\s+der\s+republik|recht\s+von)/i,
      /(?:governed\s+by|construed\s+in\s+accordance\s+with)\s+(?:the\s+)?laws\s+of/i,
      /(?:exclusive\s+jurisdiction|exclusive\s+venue)\s+(?:of|in)\s+(?:the\s+courts\s+of)?/i,
      /(?:unter\s+ausschluss\s+des\s+un[\s-]kaufrechts|cisg)/i,
      /gerichte?\s+am\s+(?:geschäfts?sitz|sitz)[^;:\n]{0,60}?(?:zuständig|berufen|vereinbart|ausschließlich)/i,
      /(?:zuständigkeit|gerichtsstand)[^;:\n]{0,80}?(?:am\s+)?(?:geschäfts?sitz|sitz)\s+(?:des|der)\s+(?:anbieters?|dienstleisters?|auftragnehmers?|systemzentrale|verkäufers?)/i,
      /(?:sind\s+)?ausschließlich\s+die\s+gerichte[^;:\n]{0,60}?(?:zuständig|berufen)/i,
      // Zweiter Satzteil nach Semikolon: "gilt der gesetzliche Gerichtsstand"
      /gerichtsstand\s+(?:ist|gilt|bleibt|wird)[^.!?;:\n]{0,80}/i,
      // Forum ohne das Wort Gerichtsstand: "ausschließlich vor den staatlichen Gerichten in X"
      /ausschließ\w*[^.!?;:\n]{0,60}?\s(?:vor\s+den\s+|die\s+|den\s+)?(?:\w+\s+){0,2}?gerichten?\s+(?:in|bei|am|vor)\s+[A-ZÄÖÜ][\wäöüß-]+/i,
      // Red Team: Forumsklauseln in Euphemismus-/Synonym-/Nominal-/English-Syntax
      /streitigkeiten[^.!?;:\n]{0,80}?einheitlicher\s+ort/i,
      /(?:ort|gerichtsstand)\s+ist\s+(?:der\s+|das\s+)?(?:sitz|hauptsitz|niederlassung)\s+(?:des|der)\s+(?:anbieters?|anbieterin|dienstleisters?|verk(?:ä|ae)ufers?)/i,
      /(?:sitz|niederlassung|hauptsitz)\s+(?:des|der)\s+(?:anbieters?|anbieterin|dienstleisters?|auftragnehmers?|verk(?:ä|ae)ufers?)[^.!?;:\n]{0,80}?gerichts?stand/i,
      // Anbietersitz plus ausschliessliche Zustaendigkeit ohne die Woerter 'Gerichtsstand'/'Sitz des Anbieters' (Welle D):
      /(?:unternehmenssitz|gesch(?:ä|ae)ftssitz|hauptsitz|sitz|niederlassung)\s+(?:des|der)\s+(?:anbieters?|anbieterin|dienstleisters?|dienstleisterin|auftragnehmers?|auftragnehmerin|verk(?:ä|ae)ufers?)[^.!?;:\n]{0,80}?(?:ausschlie(?:ß|ss)lich\s+)?zust(?:ä|ae)ndig/i,
      /ort\s+der\s+streitbeilegung[^.!?;:\n]{0,140}?(?:hauptsitz|sitz|niederlassung)/i,
      /streitigkeiten[^.!?;:\n]{0,120}?bei\s+der\s+niederlassung[^.!?;:\n]{0,80}?zur\s+entscheidung\s+gebracht/i,
      /(?:niederlassung|hauptsitz)\s+(?:der\s+|des\s+)?(?:anbieterin|anbieters)[^.!?;:\n]{0,60}?in\s+[A-ZÄÖÜ][\wäöüß-]+/i,
      /ausschlie(?:ß|ss)\w*[^.!?;:\n]{0,40}?(?:der\s+)?gerichts?stand/i,
      /venue\s+shall\s+lie/i,
      /courts?\s+of\s+the\s+provider['\u2019]?s?\s+(?:seat|principal\s+place)/i,
      /submits?\s+to\s+(?:that|the)\s+venue/i,
      /bring\s+claims?\s+before\s+any\s+other\s+court/i,
      /subject\s+to\s+the\s+laws?\s+of/i,
      /(?:convention\s+on\s+contracts\s+for\s+the\s+international\s+sale\s+of\s+goods|waive[^.!?;:\n]{0,100}?international\s+sale\s+of\s+goods)/i,
      /verhandlungs-?\s*und\s+entscheidungsort/i,
      /(?:anspr(?:ü|ue)che|klagen)[^.!?;:\n]{0,80}?bei\s+(?:jedem|jeglichem)\s+(?:beliebigen\s+)?gericht/i,
    ],
    severity(s) {
      const exclusive =
        /ausschließliche?r?\s+gerichtsstand|ausschließlich\s+vor\s+den|ausschließ\w*[^.!?;:\n]{0,60}?gerichten?|ausschlie(?:ß|ss)\w*[^.!?;:\n]{0,40}?(?:der\s+)?gerichts?stand|ausschlie(?:ß|ss)\w*[^.!?;:\n]{0,60}?zust(?:ä|ae)ndig|venue\s+shall\s+lie|einheitlicher\s+ort/i.test(
          s
        );
      const providerSeat =
        /(sitz|niederlassung|hauptsitz)\s+(?:des\s+|der\s+)?(anbieters?|anbieterin|verkäufers?|verkäuferin|dienstleisters?|dienstleisterin|auftragnehmers?|systemzentrale|lieferanten?)|am\s+sitz\s+des\s+anbieters/i.test(
          s
        );
      const foreign =
        /delaware|california|new\s+york|usa|united\s+states|singapur|singapore|schweiz|switzerland|london|uk|england|ausland|wilmington|malta|zypern|cyprus|luxemburg|luxembourg|irland|ireland|gibraltar|(?:ö|oe)sterreich|wien|z(?:ü|ue)rich|\bzug\b/i.test(
          s
        );
      // Reine gesetzliche Zuständigkeit bzw. rein deutsche Rechtswahl ist unproblematisch.
      if (/gesetzliche\w*\s+gerichtsstand/i.test(s) && !foreign && !providerSeat) return null;
      if (
        /(?:ausschlie(?:ß|ss)lich\s+)?(?:deutsches\s+recht|recht\s+der\s+bundesrepublik\s+deutschland|laws?\s+of\s+(?:the\s+federal\s+republic\s+of\s+)?germany|german\s+law)/i.test(s) &&
        !foreign &&
        !providerSeat &&
        !/(?:ausschlie(?:ß|ss)lich\w*[^.!?;:\n]{0,30}?(?:gerichtsstand|zust(?:ä|ae)ndig|gerichte)|ausschlie(?:ß|ss)licher\s+gerichtsstand)/i.test(s)
      ) {
        return null;
      }
      return (exclusive && (providerSeat || foreign)) || foreign ? "hoch" : "mittel";
    },
    why(ctx) {
      const s = ctx.sentence || "";
      if (/(?:es\s+gilt|anwendbar|unterliegt|governed\s+by)[^.!?;:\n]{0,80}?(?:recht|law)/i.test(s) && !/gerichtsstand|gerichte/i.test(s)) {
        return "Es soll ausländisches Recht gelten. Dann richten sich Ihre Rechte nach einer Rechtsordnung, die Sie und Ihr Anwalt erst prüfen müssen, und zwingende deutsche Schutzvorschriften können verdrängt werden (§ 307 BGB, Art. 6 Rom I-VO).";
      }
      return "Streitigkeiten müssen vor Gericht am Sitz des Anbieters oder im Ausland geklärt werden. Für Sie bedeutet das: weite Wege, fremde Anwälte und deutlich höhere Kosten, falls Sie Ihre Rechte durchsetzen wollen.";
    },
    suggestion(ctx) {
      const s = ctx.sentence || "";
      if (/(?:es\s+gilt|anwendbar|unterliegt|governed\s+by)[^.!?;:\n]{0,80}?(?:recht|law)/i.test(s) && !/gerichtsstand|gerichte/i.test(s)) {
        return "Deutsches Recht und einen Gerichtsstand an Ihrem Firmensitz vereinbaren; gegenüber Verbrauchern darf die Rechtswahl den Schutz des Aufenthaltsstaats nicht entziehen.";
      }
      return "Gerichtsstand an Ihrem Firmensitz oder einen neutralen Ort aushandeln; zusätzlich außergerichtliche Mediation vorsehen.";
    },
  },
  {
    type: "non_compete",
    title: "Wettbewerbsverbot",
    tags: ["hgb-74", "bgb-138", "wettbewerbsverbot", "existenzrisiko"],
    patterns: [
      /(?:nach|bei)\s+(?:beendigung|ablauf|ende)\s+(?:des\s+|der\s+)?vertrags?\w*[^.!?;:\n]{0,150}?(?:konkurrierend|wettbewerbs?tätig|nicht\s+tätig|beratend|vermittelnd|operativ\s+tätig)/i,
      // Evasion-Swarm: Exklusivitätspartnerschaftliche Zusammenarbeit / Interessenswahrung als getarntes Wettbewerbsverbot
      /(?:exklusivitätspartnerschaft\w*|interessenswahrung\s+und\s+kundenschutz|interessenwahrung|geschäftsinteressen)[^.!?;:\n]{0,180}?(?:wettbewerbsverhältnisse\s+zu\s+wahren|wettbewerbsverhältnisse\s+zu\s+vermeiden|gegenseitig\s+zu\s+verbieten|keine\s+ähnlichen\s+geschäftsaktivitäten|nach\s+vertragsende\s+verpflichtend\s+bleibt|im\s+wettbewerb\s+zu\s+beeinträchtigen|keine\s+handlungen\s+zu\s+unternehmen)/i,
      /(?:erfolg\s+des\s+auftraggebers\s+beeinträchtigen|gegenstand\s+der\s+zusammenarbeit\s+zu\s+untergraben)[^.!?;:\n]{0,100}?(?:nach\s+beendigung|nach\s+vertragsende)/i,
      /(?:keine\s+handlungen\s+zu\s+unternehmen[^.!?;:\n]{0,80}?beeinträchtigen[^.!?;:\n]{0,80}?nach\s+beendigung)/i,
      /(?:durch\s+eigenständige\s+handlungen\s+im\s+wettbewerb\s+zu\s+beeinträchtigen|gegenstand\s+der\s+zusammenarbeit\s+zu\s+untergraben)/i,
      /(?:exklusivitätspartnerschaft\w*|interessenswahrung)[^.!?;:\n]{0,180}?(?:wettbewerbs?ähnliche\s+tätigkeiten|ausgeschlossen)/i,
      /(?:keine\s+ähnlichen\s+geschäftsaktivitäten|keine\s+konkurrierenden\s+geschäfte)[^.!?;:\n]{0,120}?(?:im\s+gleichen\s+oder\s+verwandten\s+bereich|im\s+geschäftsbereich)/i,
      /wettbewerbsverhältnisse\s+zu\s+vermeiden/i,
      /wettbewerbsverbot|konkurrenzverbot/i,
      /(?:nach|bei)\s+(?:beendigung|ablauf|ende)\s+(?:des\s+|der\s+)?(?:vertrags|vertragsverhältnisses)[^.!?;:\n]{0,150}?(?:konkurrierend|wettbewerbs?tätig|nicht\s+tätig|beratend|vermittelnd|operativ\s+tätig)/i,
      /(?:konkurrierend\w*|wettbewerbliche\w*)\s+tätigkeit\w*[^.!?;:\n]{0,60}?(?:nach|bei)\s+(?:beendigung|vertragsende|ablauf)/i,
      // Red Team: Fortwirkende Treuepflicht / Untersagung der Tätigkeit
      /verbot\s+jedweder\s+tätigkeit[^.!?;:\n]{0,80}?gilt\s+über\s+die\s+beendigung/i,
      /(?:karenzentschädigung|ausgleichszahlung)[^.!?;:\n]{0,60}?(?:nicht|ausgeschlossen|abbedungen|entfällt)/i,
      /(?:betreibt|betreiben|unterhält|unterhalten)[^;:\n]{0,140}?(?:kein|keine|nicht)[^;:\n]{0,60}?(?:system|geschäft|konkurrenz|wettbewerb|niederlassung)/i,
      /(?:enthält|enthalten)\s+sich[^;:\n]{0,60}?(?:jeder\s+)?(?:wettbewerbs?tätigkeit|konkurrenztätigkeit|konkurrenz)/i,
      // Getarnte nachvertragliche Tätigkeitsverbote (Synonym-/Euphemismus-Evasion)
      /(?:für|während|innerhalb)\s+(?:die\s+dauer\s+von\s+)?(?:\d{1,3}|[a-z\u00e4\u00f6\u00fc\u00df]{3,20})\s+monate[n]?\s+(?:nach|ab)\s+(?:auslaufen|ende|beendigung|ablauf)[^.!?;:\n]{0,160}?(?:keine\s+t(?:ä|ae)tigkeit|keine\s+besch(?:ä|ae)ftigung|unt(?:ä|ae)tig|nicht\s+t(?:ä|ae)tig)/i,
      /(?:keine\s+besch(?:ä|ae)ftigung\s+oder\s+gelegenheit|nicht\s+durch\s+gleichartige\s+angebote\s+dritter)/i,
      /(?:t(?:ä|ae)tigkeit|besch(?:ä|ae)ftigung)[^.!?;:\n]{0,80}?(?:im\s+gesch(?:ä|ae)ftsbereich|im\s+wirtschaftsraum)[^.!?;:\n]{0,60}?(?:aus(?:ü|ue)ben|untersagt|verboten|keine)/i,
      /keine\s+t(?:ä|ae)tigkeit\s+im\s+gesch(?:ä|ae)ftsbereich[^.!?;:\n]{0,60}?aus/i,
      /unt(?:ä|ae)tig\s+(?:zu\s+bleiben|bleibt)/i,
      /erwerbsersatzleistung[^.!?;:\n]{0,60}?nicht\s+(?:zugesagt|vorgesehen|gezahlt)/i,
      /(?:artikel\s*418|karenzentsch(?:ä|ae)digung|ausgleichszahlung)[^.!?;:\n]{0,60}?(?:nicht\s+(?:zugesprochen|zugesagt|geschuldet|vorgesehen)|ausdr(?:ü|ue)cklich\s+ausgeschlossen)/i,
      /(?:nachwirkung|bindungswirkungen)[^.!?;:\n]{0,140}?(?:weitergelten|fortgelten|weitergelten)/i,
    ],
    collect(sentences, _full, extra = null) {
      const whole = sentences.map(flattenSentence).join(" ");
      // Gesetzeskonform: Karenzentschädigung ist ausdrücklich vereinbart oder
      // wird gezahlt (§ 74 Abs. 2 HGB).
      const hasKarenz = /karenzentsch(?:ä|ae)digung/i.test(whole);
      // § 74 Abs. 2 HGB: mindestens die Hälfte der zuletzt bezogenen Vergütung.
      const paidPercent = whole.match(
        /(?:^|[^\d])(\d{1,3})\s*(?:%|prozent)[^.!?;:\n]{0,80}?(?:leistungen|bez(?:ü|ue)ge|gehalt|honorar|verg(?:ü|ue)tung)/i
      );
      const karenzPaid =
        (paidPercent !== null && Number(paidPercent[1]) >= 50) ||
        /§\s*74\s*(?:abs\.?\s*2)?\s*hgb/i.test(whole);
      const karenzDenied =
        /(?:keine|ohne)\s+karenzentsch(?:ä|ae)digung|karenzentsch(?:ä|ae)digung[^.!?;:\n]{0,80}?nicht\s+(?:zugesagt|gezahlt|vorgesehen|geschuldet)|(?:karenzentsch(?:ä|ae)digung|ausgleich)[^.!?;:\n]{0,60}?(?:ausgeschlossen|abbedungen|entf(?:ä|ae)llt)/i.test(
          whole
        );
      if (hasKarenz && karenzPaid && !karenzDenied) return [];
      return collectPerSentence(this, sentences, null, extra);
    },
    severity(s) {
      const flat = s.replace(/\s+/g, " ");
      const postContractual =
        /(?:nach|bei)\s+(?:der\s+|dem\s+|des\s+)?(?:beendigung|ablauf|ende)\b|vertrags(?:ende|ablauf)|\über\s+(?:das\s+)?vertragsende|über\s+die\s+beendigung|nach\s+auslaufen|sodann|nachvollzugsphase/i.test(
          flat
        );
      return postContractual ? "hoch" : "mittel";
    },
    why(ctx) {
      if (ctx.severity === "hoch") {
        return "Sie dürfen auch nach Vertragsende nicht für Konkurrenten arbeiten. Ohne Ausgleichszahlung ist das ein Existenzrisiko – besonders für Gründer und Freelancer.";
      }
      return "Im Vertrag wird ein Wettbewerbsverbot thematisiert. Prüfen Sie genau, wann es greift und ob es über das Vertragsende hinauswirkt.";
    },
    suggestion:
      "Nachvertragliche Verbote streichen oder deutlich entschärfen: maximal ein Jahr, sachlich eng begrenzt und nur mit Karenzentschädigung.",
  },
  {
    type: "data_grant",
    title: "Sehr weitreichende Nutzungsrechte an Ihren Daten",
    tags: ["urhg-31", "dsgvo", "geschgehg", "nutzungsrechte", "ip-transfer", "ai-scraping"],
    patterns: [
      /(?:nutzungsrecht[^.!?;:\n]{0,120}?(?:zwecke\s+des\s+trainings|finetuning|machine[\s-]learning|trainieren\s+von|algorithmen|vorhersagemodelle))/i,
      /(?:räumt|gewährt)[^.!?;:\n]{0,100}?(?:weltweites|unwiderrufliches|übertragbares)[^.!?;:\n]{0,60}?nutzungsrecht/i,
      /(?:training|finetuning|weiterentwicklung)[^.!?;:\n]{0,60}?(?:interner\s+algorithmen|machine[\s-]learning|modelle\w*)/i,
      /überträg\w*[^.!?;:\n]{0,100}?\balle\s+(?:ggf\.?\s+)?(?:urheberrechtlichen\s+)?rechte/i,
      /(?:räumt|einräumt|gewährt)[^.!?;:\n]{0,80}?(?:ausschließlich\w*|exklusiv\w*|uneingeschränkt\w*|unbegrenzt\w*)/i,
      /nutzungsrecht\w*[^.!?;:\n]{0,80}?(?:uneingeschränkt|unbegrenzt|ausschließlich|exklusiv|ohne\s+grenzen)/i,
      /(?:zeitlich|räumlich|inhaltlich)\s*,?\s*(?:(?:und\s+)?(?:zeitlich|räumlich|inhaltlich)\s*,?\s*)*uneingeschränkt\w*/i,
      // Red Team: Skript-/Vorlagen-Enteignung & Telemetrie-Drittverwertung
      /(?:gehen|fällt|fallen)[^.!?;:\n]{0,120}?(?:in\s+das\s+)?(?:unbeschränkte\s+)?(?:geistige\s+eigentum|alleineigentum)\s+des\s+(?:anbieters|betreibers)/i,
      /(?:berechtigt|befugt)[^.!?;:\n]{0,80}?(?:telemetriedaten|feedback|workflows?)[^.!?;:\n]{0,120}?(?:kommerziell|dritten|veräußern|lizenzieren|verwerten)/i,
      /(?:telemetriedaten|diagnoseprotokoll\w*|kennzahlen)[^.!?;:\n]{0,140}?(?:weltweit|unbegrenzt|kommerziell)[^.!?;:\n]{0,100}?(?:verwerten|verwertet|nutzen|veräußern)/i,
      // IP-Abschöpfung / KI-Modelltraining / Verdeckte Datenverwertung:
      /(?:mathematische\s+gewichtungen|strukturelle\s+muster|systemparameter|semantische\s+analyse)[^.!?;:\n]{0,160}?(?:dauerhaft|unentgeltlich|über\s+das\s+vertragsende\s+hinaus|modellierungszwecke|entwicklungszwecke)/i,
      /(?:entwicklungs-|modellierungszwecke|trainieren|modelltraining)[^.!?;:\n]{0,120}?(?:dauerhaft|unentgeltlich|über\s+das\s+vertragsende\s+hinaus)/i,
      // Konzern- & Kontokorrentvorbehalt bei IP & Nutzungsrechten (§ 307 BGB):
      /(?:urheber-?\s*(?:und\s+)?nutzungsrechte|nutzungsrechte|arbeitsergebniss\w*|eigentum)[^.!?;:\n]{0,140}?(?:gesamten\s+bestehenden\s+geschäftsbeziehung|allen\s+forderungen\s+aus\s+der\s+geschäftsverbindung)/i,
      /(?:ausgleich|erfüllung)[^.!?;:\n]{0,80}?(?:aller|sämtlicher)[^.!?;:\n]{0,80}?(?:forderungen|ansprüche)[^.!?;:\n]{0,80}?aus\s+der\s+gesamten\s+(?:bestehenden\s+)?geschäftsbeziehung/i,
      /aus\s+der\s+gesamten\s+bestehenden\s+geschäftsbeziehung/i,
      /(?:auswertungen|nutzungsdaten|erkenntnisse|betriebsdaten)[^.!?;:\n]{0,80}?(?:in\s+eigener\s+verantwortung\s+verwerten|eigenständig\s+verwerten|für\s+eigene\s+zwecke\s+verwerten)/i,
      /(?:verfahrenskenntnisse|strukturmodelle|erkenntnisse)[^.!?;:\n]{0,80}?(?:zeitlich\s+unbeschränkt|ohne\s+vergütung|für\s+eigene\s+zwecke\s+nutzen)/i,
      // DSGVO-Verstöße: Gekoppelte Einwilligung, Profilbildung für Werbung, Sanktionierung des Widerrufs (Art. 7 Abs. 3, 4 DSGVO)
      /(?:einwilligung\s+ist\s+voraussetzung\s+für\s+die\s+erbringung\s+der\s+leistungen|widerruf\s+der\s+einwilligung\s+gilt\s+als[^.!?;:\n]{0,60}?kündigung)/i,
      /(?:zu\s+zwecken\s+der\s+werbung,\s+marktforschung\s+und\s+profilbildung\s+verarbeitet)/i,
      // Red Team: Euphemismus-, Nominal-, Listen- und English-Syntax der Datengrätsche
      /merkmalsprofile[^.!?;:\n]{0,100}?(?:verwenden|nutzen|verwerten)/i,
      /verg(?:ü|ue)tung\s+oder\s+beteiligung\s+des\s+kunden[^.!?;:\n]{0,60}?nicht\s+vorgesehen/i,
      /(?:qualit(?:ä|ae)tssicherung|anomalieerkennung)[^.!?;:\n]{0,120}?datenstr(?:ö|oe)me[^.!?;:\n]{0,60}?ausgewertet/i,
      /grants?[^.!?;:\n]{0,140}?(?:perpetual|irrevocable|worldwide|royalty-free)[^.!?;:\n]{0,80}?licen[cs]e/i,
      /licen[cs]e[^.!?;:\n]{0,180}?(?:right\s+to\s+train|train\s+machine\s+learning|machine\s+learning\s+models)/i,
      /erfolgt\s+eine\s+(?:ü|ue)bertragung\s+s(?:ä|ae)mtlicher[^.!?;:\n]{0,100}?befugnisse/i,
      /beschr(?:ä|ae)nkung\s+der\s+verwertung[^.!?;:\n]{0,60}?nicht\s+vorgesehen/i,
      /(?:auswertungen|erkenntnisse)[^.!?;:\n]{0,100}?(?:unentgeltlich|zeitlich\s+unbegrenzt)[^.!?;:\n]{0,100}?(?:eigene[rn]?\s+zwecken?\s+)?(?:nutzen|verwenden|verwerten)/i,
      /(?:beteiligung\s+des\s+(?:kunden|auftraggebers|bestellers)\s+an\s+(?:den\s+)?erl(?:ö|oe)sen|erl(?:ö|oe)se[^.!?;:\n]{0,60}?(?:beteiligung|verg(?:ü|ue)tung)\s+des\s+(?:kunden|auftraggebers|bestellers))[^.!?;:\n]{0,80}?nicht\s+vorgesehen/i,
      /(?:auswertungen|erkenntnisse)[^.!?;:\n]{0,140}?auch\s+f(?:ü|ue)r\s+eigene\s+zwecke\s+nutzen/i,
    ],
    severity(s) {
      if (
        /einwilligung\s+ist\s+voraussetzung|widerruf\s+der\s+einwilligung|profilbildung|merkmalsprofile|(?:ü|ue)bertragung\s+s(?:ä|ae)mtlicher|beschr(?:ä|ae)nkung\s+der\s+verwertung|unentgeltlich[^.!?;:\n]{0,80}?nutzen|verg(?:ü|ue)tung\s+oder\s+beteiligung[^.!?;:\n]{0,60}?nicht\s+vorgesehen|beteiligung\s+des\s+(?:kunden|auftraggebers|bestellers)\s+an|perpetual|irrevocable|worldwide|royalty-free|machine\s+learning|zeitlich\s+unbegrenzt|eigene\s+zwecke/i.test(
          s
        )
      )
        return "hoch";
      if (/gesamten\s+bestehenden\s+geschäftsbeziehung|geschäftsverbindung/i.test(s)) {
        return "hoch";
      }
      // Red Team Schutz vor False Positives: "nicht ausschließlich" ist Standard und sauber!
      if (/nicht\s+ausschließl\w*|kein\s+ausschließl\w*/i.test(s)) {
        return null;
      }
      return /alle\s+rechte|uneingeschränkt|unbegrenzt|ausschließlich|exklusiv|geistige\s+eigentum|kommerziell|mathematische\s+gewichtungen|modellierungszwecke|über\s+das\s+vertragsende\s+hinaus|in\s+eigener\s+verantwortung|zeitlich\s+unbeschränkt/i.test(s)
        ? "hoch"
        : "mittel";
    },
    why(ctx) {
      const s = ctx.sentence || "";
      if (/gesamten\s+bestehenden\s+geschäftsbeziehung/i.test(s)) {
        return "Konzern- & Kontokorrentvorbehalt bei Nutzungsrechten (§ 307 BGB): Die Übertragung von Nutzungsrechten an Arbeitsergebnissen wird vom Ausgleich sämtlicher Forderungen aus der gesamten Geschäftsbeziehung abhängig gemacht. Dieser Vorbehalt weicht das synallagmatische Austauschverhältnis unzulässig auf.";
      }
      if (/mathematische\s+gewichtungen|strukturelle\s+muster|modellierungszwecke|semantische\s+analyse/i.test(s)) {
        return "IP-Abschöpfung & verdecktes KI-Training (GeschGehG, DSGVO, § 307 BGB): Der Anbieter sichert sich dauerhafte, unentgeltliche Nutzungsrechte an Mustern und Gewichten aus Ihren Geschäftsdaten auch über das Vertragsende hinaus.";
      }
      return "Sie geben praktisch alle Rechte an Ihren Inhalten oder Daten ab – zeitlich, räumlich und inhaltlich unbegrenzt. Der Anbieter könnte sie danach beliebig verwerten, sogar weiterverkaufen.";
    },
    suggestion:
      "Nutzungsrechte nur im Umfang des konkreten Betriebsauftrags einräumen. Jede dauerhafte Ableitung von Parametern, KI-Modelltraining oder Weiterverwendung nach Vertragsende strikt untersagen.",
  },
  {
    type: "default_interest",
    title: "Überhöhte Verzugszinsen",
    tags: ["bgb-288", "verzugszinsen", "finanzrisiko"],
    patterns: [
      /verzugszins\w*/i,
      // Red Team: Zinsmarge / Basispunkte / Leitzins
      /(?:basispunkten|zinsmarge)/i,
      /(?:verzugsfall|zahlungsverzug)[^.!?;:\n]{0,80}?(?:leitzins|zins)/i,
      // OCR-/Tabellenform: "Zinssatz zwölf Prozent pro Jahr"
      /(?:zinssatz|zinsen|sollzins|verzinsung)[^.!?;:\n]{0,60}?(?:\d{1,2}(?:[.,]\d+)?|zehn|elf|zw(?:ö|oe)lf|dreizehn|vierzehn|f(?:ü|ue)nfzehn|sechzehn|siebzehn|achtzehn|neunzehn|zwanzig)\s*(?:prozentpunkten?|prozent|%)/i,
    ],
    severity(s) {
      // Red Team: 1200 Basispunkte = 12%, 850 Basispunkte = 8,5%
      const bpMatch = s.match(/(\d{3,4})\s*basispunkten/i);
      let bpValue = null;
      if (bpMatch) {
        const bp = parseInt(bpMatch[1], 10);
        if (bp >= 900) return "hoch";
        bpValue = bp / 100;
      }

      const PCT_WORDS = { acht: 8, neun: 9, zehn: 10, elf: 11, "zwölf": 12, dreizehn: 13, vierzehn: 14, "fünfzehn": 15, sechzehn: 16, siebzehn: 17, achtzehn: 18, neunzehn: 19, zwanzig: 20 };
      let value = bpValue;
      const pm = s.match(/(\d{1,2}(?:[.,]\d+)?)\s*(?:prozentpunkt\w*|prozent|%|punkten?(?=\s*(?:über|ueber)))/i);
      if (pm) value = parseFloat(pm[1].replace(",", "."));
      if (value === null) {
        const wm = s.match(/(zehn|elf|zw(?:ö|oe)lf|dreizehn|vierzehn|f(?:ü|ue)nfzehn|sechzehn|siebzehn|achtzehn|neunzehn|zwanzig|acht|neun)\s*(?:prozentpunkt\w*|prozent|%)/i);
        if (wm) {
          const key = wm[1].toLowerCase().replace("oe", "ö").replace("ue", "ü");
          value = PCT_WORDS[key] ?? null;
        }
      }
      if (value === null) {
        const zug = s.match(/(\d{1,2})\s*punkten?\s*(?:über|ueber)\s+dem\s+basiszinssatz/i);
        if (zug) value = parseInt(zug[1], 10);
      }
      if (value === null) return null; // nur gesetzliche Verweise sind unkritisch
      // Red Team: "je monat", "je angefangenem monat", "monatlich" (auch nachgestellt
      // oder als Adjektiv: "monatliche Bearbeitungspauschale", "1,75 % monatlich")
      const perShortPeriod = /(?:pro|je)\s+(?:angefangenem\s+)?(?:monat|woche|tag)|monatlich|w(?:ö|oe)chentlich|t(?:ä|ae)glich/i.test(s);
      if (perShortPeriod || value > 9) return "hoch";
      if (value > 5.05) return "mittel";
      return "niedrig";
    },
    why(ctx) {
      const pm = ctx.sentence.match(/(\d{1,2}(?:[.,]\d+)?)\s*(?:prozentpunkt\w*|prozent|%)/i);
      const value = pm ? pm[1].replace(".", ",") : null;
      if (value) {
        return `Verzugszinsen von ${value} Punkten über dem Basiszinssatz liegen deutlich über dem gesetzlichen Maß (regelmäßig 5 Punkte). Bei einmaligen Zahlproblemen wird es schnell sehr teuer.`;
      }
      return "Unüblich hohe Verzugszinsen: Bei kurzem Zahlungsverzug summieren sich die Kosten unverhältnismäßig schnell.";
    },
    suggestion:
      "Auf die gesetzlichen Verzugszinsen beschränken (§ 288 BGB) und keine zusätzlichen Mahn- oder Pauschalgebühren akzeptieren.",
  },
  {
    type: "unilateral_change",
    title: "Einseitiges Änderungsrecht & Zustimmungsfiktion",
    tags: ["bgb-307", "bgb-308-nr-4", "bgb-308-nr-5", "aenderungsvorbehalt", "preisrisiko"],
    patterns: [
      /(?:gilt|gelten)[^.!?;:\n]{0,40}?(?:die\s+)?(?:budgeterweiterung|budget|kostenerhöhung|mehrkosten)[^.!?;:\n]{0,30}?als\s+(?:genehmigt|freigegeben)/i,
      /(?:schätzrahmen|budget|kostendeckel)[^.!?;:\n]{0,80}?(?:zu\s+überschreiten|um\s+bis\s+zu\s+\d+\s*%)[^.!?;:\n]{0,80}?(?:ohne\s+(?:dass\s+es\s+einer\s+)?(?:gesonderten\s+)?zustimmung)/i,
      /(?:widerspricht\s+der\s+kunde[^.!?;:\n]{0,40}?nicht|kein\s+widerspruch)[^.!?;:\n]{0,120}?(?:gilt|gelten)[^.!?;:\n]{0,40}?(?:als\s+(?:unwiderruflich\s+)?(?:genehmigt|akzeptiert|vereinbart|angenommen))/i,
      /(?:änderungen|anpassungen|geänderten?\s+bedingungen)[^.!?;:\n]{0,120}?gelten\s+als\s+(?:genehmigt|akzeptiert|vereinbart)[^.!?;:\n]{0,80}?(?:nicht|widerspricht)/i,
      /(?:widerspricht\s+der\s+kunde\s+nicht|sofern\s+kein\s+widerspruch)[^.!?;:\n]{0,80}?gelten[^.!?;:\n]{0,60}?(?:als\s+genehmigt|als\s+akzeptiert|als\s+angenommen)/i,
      /(?:die\s+)?zustimmung\s+(?:des\s+kunden|des\s+nutzers|des\s+auftraggebers)?\s*gilt\s+als\s+(?:erteilt|gegeben|genehmigt)/i,
      /änderungsvorbehalt/i,
      /einseitig\w*,?\s+(?:und\s+)?(?:ohne\s+zustimmung\s+)?(?:zu\s+)?(?:ändern|ergänzen|anpassen)/i,
      /recht\w*,?[^.!?;:\n]{0,80}?einseitig[^.!?;:\n]{0,60}?(?:ändern|anpassen|ergänzen)/i,
      /behält\s+sich\s+vor,?[^.!?;:\n]{0,200}?(?:änder|passen|ergänz|anheb)/i,
      /berechtigt[^.!?;:\n]{0,200}?(?:anpassen|anzupassen|entfernen|einzustellen|streichen)/i,
      /(?:umfang\s+der\s+services|leistungsumfang|services|leistungen)[^.!?;:\n]{0,80}?(?:ohne\s+angabe\s+von\s+gründen|jederzeit)[^.!?;:\n]{0,60}?(?:zu\s+ändern|einzuschränken|einzustellen|ändern)/i,
      // Evasion-Swarm: Anpassung von SLAs, Leistungsumfängen und Preisen "jederzeit und ohne Rücksprache"
      /(?:leistungsumfänge|leistungsumfang|slas?|preisangaben|preise)[^.!?;:\n]{0,100}?(?:jederzeit|ohne\s+rücksprache|nach\s+eigenem\s+ermessen)[^.!?;:\n]{0,80}?(?:anpassen|ändern|modifizieren)/i,
      /(?:anpassen|ändern)\s+kann,?\s+sofern\s+dies\s+zur\s+sicherung[^.!?;:\n]{0,60}?(?:dienstleistung|leistung)/i,
      /(?:bedingungen\s+sowie\s+deren\s+änderungen|änderungen)[^.!?;:\n]{0,80}?(?:ohne\s+beanstandung|vorbehaltlos)\s+(?:umzusetzen|zu\s+akzeptieren|anzunehmen)/i,
      /änderungen\s+werden\s+durch\s+veröffentlichung[^.!?;:\n]{0,80}?wirksam/i,
      /(?:durch[^.!?;:\n]{0,60}?(?:bloßes\s+)?schweigen\s+annehmen|annahme\s+durch\s+schweigen|durch\s+schweigen\s+angenommen)/i,
      /(?:angebot\s+ab,\s+an\s+das\s+er|an\s+sein\s+angebot\s+gebunden)[^.!?;:\n]{0,80}?(?:vier\s+wochen|\d+\s+wochen|\d+\s+tage)/i,
      /(?:lieferung\s+(?:angemessen,\s+höchstens\s+jedoch\s+um|zu\s+verschieben)|lieferfristen[^.!?;:\n]{0,40}?unverbindlich)/i,
      /(?:ohne\s+dass\s+dem\s+kunden\s+hieraus\s+rechte\s+erwachsen)/i,
      /(?:keine\s+ansprüche|ohne\s+anspruch|ausschluss\s+von\s+ansprüchen|ohne\s+dass\s+dem\s+kunden)[^.!?;:\n]{0,80}?(?:minderung|rückabwicklung|herabsetzung)/i,
      /minderung\s+(?:oder\s+rückabwicklung\s+)?ist\s+ausgeschlossen/i,
      /sonderkündigungsrecht[^.!?;:\n]{0,80}?(?:erst\s+(?:dann|bei)|übersteigt)/i,
      /(?:ausschließen|ausgeschlossen|nicht\s+berechtigt)[^.!?;:\n]{0,80}?(?:lösung\s+vom\s+vertrag|sonderkündigung)/i,
      /lösung\s+vom\s+vertrag[^.!?;:\n]{0,40}?ausschließen/i,
      /(?:widerspricht[^.!?;:\n]{0,60}?nicht[^.!?;:\n]{0,80}?)?gilt\s+(?:die\s+)?(?:fortgesetzte\s+nutzung|weiterführung)[^.!?;:\n]{0,60}?als\s+(?:unwiderrufliche\s+)?(?:genehmigung|zustimmung)/i,
      /als\s+(?:unwiderrufliche|stillschweigende)\s+genehmigung\s+der\s+geänderten\s+bedingungen/i,
      // Red Team: BGH XI ZR 26/20 Schweigen als Annahme & Login als Fiktion
      /gilt\s+(?:das\s+|sein\s+)?schweigen[^.!?;:\n]{0,50}?als\s+(?:annahme|zustimmung|genehmigung)/i,
      /(?:zugriff|anmeldung|login|nutzung|weiternutzung)[^.!?;:\n]{0,80}?steht[^.!?;:\n]{0,30}?(?:dem\s+)?schweigen[^.!?;:\n]{0,20}?gleich/i,
      /(?:loggt|zugriff|anmeldung)[^.!?;:\n]{0,180}?begründet[^.!?;:\n]{0,80}?(?:einverständnis|zustimmung|annahme)/i,
      /modifiziert\s+der\s+(?:provider|anbieter)[^.!?;:\n]{0,80}?(?:module|komponenten|funktionen)/i,
      // Billiges Ermessen bei Architektur/Leistungsanpassung (§ 307, § 308 Nr. 4 BGB):
      /berechtigt[^.!?;:\n]{0,120}?(?:angleichen|anzugleichen|anpassen|modifizieren)/i,
      /nach\s+eigenem\s+(?:billigem\s+)?ermessen\s+(?:anpassen|anzugleichen|angleichen|ändern)/i,
      // Zustimmungsfiktion zu Mehrkosten & Budgetüberschreitung (§ 308 Nr. 4, 5 BGB):
      /gelten[^.!?;:\n]{0,80}?(?:damit\s+verbundenen\s+)?kosten\s+als\s+freigegeben/i,
      /(?:kosten|mehraufwände|aufwände)[^.!?;:\n]{0,80}?als\s+(?:freigegeben|genehmigt|akzeptiert)/i,
      /(?:budget|vergütung)[^.!?;:\n]{0,80}?um\s+(?:nicht\s+mehr\s+als\s+)?\d+\s*%\s*überschreiten/i,
      // Zustimmungsfiktion zu Notfall-/Mehrstundenabrechnung (§ 308 Nr. 5 BGB):
      /gilt\s+(?:der\s+kunde|die\s+kundin|der\s+auftraggeber)\s+als\s+einverstanden[^.!?;:\n]{0,200}?(?:abrechnung|mehrstunden|kosten|sätze|vergütung|zuschlag)/i,
      /(?:gelten|gilt)[^.!?;:\n]{0,80}?(?:mehrstunden|mehraufwände|sonderstunden|notfallmaßnahmen)[^.!?;:\n]{0,80}?als\s+(?:genehmigt|einverstanden|freigegeben|akzeptiert)/i,
      /(?:anspruch|recht)\s+auf\s+(?:beibehaltung|unveränderte\s+fortsetzung|unveränderte\s+weitergeltung)[^;:\n]{0,60}?besteht\s+nicht/i,
      /(?:erhebt|erheben|widerspricht|widersprechen)[^;:\n]{0,60}?(?:keinen?\s+|nicht\s+)?(?:einspruch|widerspruch)[^;:\n]{0,80}?(?:wird|gilt|ist|sind)[^;:\n]{0,60}?(?:wirksam|verbindlich|vereinbart|erteilt|angenommen|maßgeblich)/i,
      /(?:erhebt|widerspricht)[^;:\n]{0,80}?(?:nicht\s+binnen|nicht\s+innerhalb|nicht\s+vor)[^;:\n]{0,60}?(?:wird|gilt)[^;:\n]{0,50}?(?:die\s+)?(?:änderung|anpassung|festsetzung|erhöhung)[^;:\n]{0,40}?(?:wirksam|verbindlich|vereinbart)/i,
      /(?:nimmt|nehmen)\s+(?:der\s+\w+\s+)?(?:die\s+)?(?:festsetzung|änderung|anpassung|neufestsetzung)[^;:\n]{0,60}?hin\b/i,
      // Vertragsschlussfiktionen (Öffnen der Verpackung, Schweigen, Einbeziehungsfiktion, § 305 Abs. 2, § 309 Nr. 12 lit. b BGB):
      /(?:zustande\s+kommt|abschluss\s+des\s+vertrages)[^.!?;:\n]{0,100}?(?:erstmalige\s+nutzung|öffnen\s+der\s+verpackung|unterlassen\s+eines\s+widerspruchs)/i,
      /(?:unterlassen\s+eines\s+widerspruchs)[^.!?;:\n]{0,80}?(?:zustande\s+kommt|gilt\s+als\s+abgeschlossen)/i,
      /(?:vollständig\s+zur\s+kenntnis\s+genommen\s+und\s+verstanden|verzicht\w*[^.!?;:\n]{0,60}?auf\s+(?:etwaige\s+)?einwendungen\s+gegen\s+(?:deren\s+)?einbeziehung)/i,
      /(?:hält|halten)\s+(?:es|das\s+(?:systemhandbuch|handbuch|richtlinie|regelwerk))[^;:\n]{0,60}?(?:laufend\s+aktuell|auf\s+dem\s+laufenden|jederzeit\s+angepasst)/i,
      // Red Team: verkappte Anpassungsrechte ohne Standardbegriffe (Syntax-Evasion)
      /nachjustierungen[^.!?;:\n]{0,140}?(?:marktgegebenheiten|angepasst)/i,
      /als\s+kaufm(?:ä|ae)nnisch\s+mitgetragen/i,
      /berechtigt[^.!?;:\n]{0,240}?fortzuentwickeln/i,
      /(?:module|leistungen|funktionen)[^.!?;:\n]{0,80}?aus\s+dem\s+portfolio\s+(?:zu\s+)?nehmen/i,
      /minderungsbegehren[^.!?;:\n]{0,120}?herabgesetzt/i,
      /anpassungen[^.!?;:\n]{0,160}?kann\s+der\s+anbieter\s+mit\s+wirkung\s+f(?:ü|ue)r\s+die\s+zukunft[^.!?;:\n]{0,40}?vornehmen/i,
      /fortsetzung\s+der\s+nutzung[^.!?;:\n]{0,80}?neufassung[^.!?;:\n]{0,40}?zugrunde\s+gelegt/i,
      /anpassungen\s+der\s+verg(?:ü|ue)tungsbestandteile[^.!?;:\n]{0,140}?f(?:ü|ue)r\s+erforderlich\s+h(?:ä|ae)lt/i,
      /als\s+betriebswirtschaftlich\s+erforderlich\s+und\s+damit\s+akzeptiert\s+behandelt/i,
      /anpassungen[^.!?;:\n]{0,120}?akzeptiert\s+behandelt\s+werden/i,
    ],
    collect(sentences, _full, extra = null) {
      const hits = collectPerSentence(this, sentences, null, extra);
      // DSGVO-konforme Subunternehmer-Regelung (Art. 28 Abs. 2 DSGVO)
      const DSGVO_CTX = /(?:dsgvo|datenschutz|auftragsverarbeit|unterauftragsverh|verantwortliche[nr]?|subunternehmer|unterauftragnehmer)/i;
      return hits.filter((h) => {
        const own = flattenLineBreaks(sentences[h.i]?.text || "");
        if (!/widerspruch/i.test(own) || !/(?:einsatz|bestellung|beauftragung)/i.test(own)) return true;
        const ctx = [sentences[h.i - 1], sentences[h.i], sentences[h.i + 1]]
          .map((s) => (s ? flattenLineBreaks(s.text) : ""))
          .join(" ");
        return !DSGVO_CTX.test(ctx);
      });
    },
    severity(s) {
      // DSGVO-konforme Subunternehmer-Regelung (Art. 28 Abs. 2 DSGVO)
      if (
        /(?:dsgvo|datenschutz|auftragsverarbeit|art\.?\s*28)/i.test(s) &&
        /widerspruch/i.test(s) &&
        /(?:einsatz|unterauftragsverh|weitere[n]?\s+auftragsverarbeiter|subunternehmer|unterauftragnehmer)/i.test(s)
      ) {
        return null;
      }
      return /ohne\s+zustimmung|nach\s+eigenem\s+(?:billigem\s+)?ermessen|jederzeit|widerruflich|als\s+einverstanden|einspruch|beibehaltung|laufend\s+aktuell|festsetzung|keine\s+ansprüche|fortgesetzte\s+nutzung|als\s+unwiderrufliche\s+genehmigung|schweigen[^.!?;:\n]{0,40}?als\s+(?:annahme|zustimmung|genehmigung)|steht[^.!?;:\n]{0,30}?(?:dem\s+)?schweigen[^.!?;:\n]{0,20}?gleich|begründet[^.!?;:\n]{0,80}?einverständnis|lösung\s+vom\s+vertrag[^.!?;:\n]{0,40}?ausschließen|kosten\s+als\s+freigegeben|budget[^.!?;:\n]{0,60}?überschreiten|nachjustierungen|mitgetragen|fortzuentwickeln|herabgesetzt|portfolio|betriebswirtschaftlich\s+erforderlich|akzeptiert\s+behandelt|neufassung/i.test(
        s
      )
        ? "hoch"
        : "mittel";
    },
    why(ctx) {
      const s = ctx.sentence || "";
      if (/kosten\s+als\s+freigegeben|budget.*?überschreiten/i.test(s)) {
        return "Kosten-Freigabefiktion & einseitige Budgeterhöhung (§ 308 Nr. 4, 5 BGB): Mehrkosten durch einseitige Leistungsanpassungen gelten bis zu einer prozentualen Schwelle (z. B. 20 %) automatisch als freigegeben. Dies umgeht das Erfordernis gesonderter Change Requests und bürdet dem Kunden unkalkulierbare Mehrkosten auf.";
      }
      if (/billigem\s+ermessen/i.test(s)) {
        return "Einseitiges Leistungsbestimmungsrecht (§ 308 Nr. 4 BGB): Das Recht, Softwarearchitektur oder Schnittstellen nach eigenem billigem Ermessen anzugleichen, gefährdet die Kompatibilität zu Kundensystemen ohne berechtigte Mitbestimmung.";
      }
      if (/minderung|rückabwicklung|entfernen|herabsetzung/i.test(s)) {
        return "Der Anbieter darf Leistungsumfang und Funktionen einseitig entkernen oder streichen, ohne dass Sie mindern dürfen. Das weicht das synallagmatische Verhältnis von Leistung und Entgelt einseitig auf (§ 307 Abs. 1, 2 BGB i. V. m. § 308 Nr. 4 BGB).";
      }
      if (/fortgesetzte\s+nutzung|als\s+unwiderrufliche\s+genehmigung|widerspricht|schweigen\s+als\s+annahme|begründet[^.!?;:\n]{0,30}?einverständnis/i.test(s)) {
        return "Eine fingierte Zustimmung zu wesentlichen AGB-Änderungen durch bloßes Schweigen, Login oder Weiternutzung ist nach ständiger BGH-Rechtsprechung (u. a. XI ZR 26/20) sowie § 308 Nr. 5 BGB unwirksam.";
      }
      if (/anheben|preiserhöhung|sonderkündigung|lösung\s+vom\s+vertrag/i.test(s)) {
        return "Preisanpassungsklauseln müssen transparent sein, auch Kostensenkungen berücksichtigen und dem Kunden bei jeder spürbaren Erhöhung ein sofortiges Sonderkündigungsrecht gewähren (§ 307 Abs. 1 BGB).";
      }
      return "Der Anbieter darf Vertragsinhalte im Nachhinein einseitig ändern – Sie müssen nur noch widersprechen oder gelten per Weiternutzung als einverstanden. Damit kann er Preise und Leistungen laufend verschieben.";
    },
    suggestion:
      "Änderungen nur mit Ihrer vorherigen ausdrücklichen Zustimmung zulassen. Bei Entfall von Features oder Preiserhöhungen ein sofortiges Minderungs- und Sonderkündigungsrecht ohne Schwellenwert vereinbaren.",
  },
  {
    type: "setoff_restriction",
    title: "Pauschales Aufrechnungsverbot oder Einbehaltssperre",
    tags: ["bgb-309-nr-3", "bgb-309-nr-2", "bgb-307", "aufrechnung", "zurueckbehaltung", "zahlungsverkehr"],
    patterns: [
      /(?:nicht\s+berechtigt|kein\s+recht)[^.!?;:\n]{0,80}?(?:zur\s+aufrechnung|aufzurechnen|zahlungen\s+zurückzuhalten|zurückzubehalten)/i,
      /(?:zur\s+aufrechnung|zurückzubehalten|zurückzuhalten)[^.!?;:\n]{0,40}?(?:nicht\s+berechtigt|nicht\s+befugt)/i,
      /(?:gegenansprüche|gegenforderungen)[^.!?;:\n]{0,80}?(?:berechtigen\s+nicht\s+zur\s+aufrechnung|schließen\s+aufrechnung\s+aus|berechtigen\s+nicht\s+zu)/i,
      /(?:zurückbehaltungsrecht|einbehalt|leistungsverweigerungsrecht)[^.!?;:\n]{0,80}?(?:ausgeschlossen|abbedungen|steht\s+[^.!?;:\n]{0,40}?nicht\s+zu|ist\s+ausgeschlossen)/i,
      /(?:without\s+(?:any\s+)?(?:set[\s-]?off|deduction|withholding)|no\s+right\s+of\s+set[\s-]?off)/i,
      /aufrechnung[^.!?;:\n]{0,80}?(?:ausgeschlossen|unzulässig|nicht\s+gestattet|auszuschließen)/i,
      /(?:ausschluss|ausschließen|ausschließt|ausschließlich)[^.!?;:\n]{0,60}?aufrechnung/i,
      /aufrechnung\s+(?:mit\s+gegenansprüchen\s+)?ist\s+(?:vollumfänglich|ausdrücklich|gänzlich|grundsätzlich)?\s*ausgeschlossen/i,
      /verrechnung[^.!?;:\n]{0,80}?(?:ausgeschlossen|unzulässig|nicht\s+gestattet|auszuschließen)/i,
      // Zurückbehaltungsrecht abbedungen, künstlich fragmentiert oder eingeschränkt (§ 309 Nr. 2 lit. b BGB, § 307 BGB)
      /zurückbehaltungsrecht[^.!?;:\n]{0,60}?(?:abbedungen|ausgeschlossen)/i,
      /ohne\s+jeden\s+abzug\s+oder\s+einbehalt/i,
      /(?:zurückbehaltungsrecht|einrede\s+des\s+nicht\s+erfüllten\s+vertrages)[^.!?;:\n]{0,100}?(?:steht\s+dem\s+kunden\s+nur\s+zu,\s+sofern|nur\s+zu,\s+sofern)/i,
      /(?:aus\s+demselben\s+konkreten\s+monatsrechnungsabschnitt|demselben\s+rechnungsabschnitt)/i,
      /zurückbehaltungsrecht[es\s]+wegen\s+mängeln[^.!?;:\n]{0,80}?(?:ausgeschlossen|abbedungen)/i,
      // Vertraglicher Kill-Switch & Aushebelung von Zurückbehaltungsrechten (§ 320, § 307 BGB):
      /(?:ruhen\s+(?:sämtliche|die|bereits)|ruhen\s+der\s+nutzungsrechte)[^.!?;:\n]{0,100}?(?:berechtigungen|nutzungsrechte|ausführung|lizenz)/i,
      /ruhen\s+sämtliche\s+bereits\s+vorläufig\s+erteilten\s+berechtigungen/i,
      /(?:zurückbehaltungsrecht\w*|leistungsverweigerungsrecht\w*)[^.!?;:\n]{0,120}?(?:lässt[^.!?;:\n]{0,60}?unberührt|ausgeschlossen|abbedungen)/i,
      /lässt\s+das\s+ruhen\s+der\s+nutzungsrechte\s+unberührt/i,
      /(?:gegenforderungen|einbehalte|gegenansprüche)[^.!?;:\n]{0,100}?(?:in\s+ansatz\s+gebracht|geltend\s+gemacht|verrechnet)[^.!?;:\n]{0,80}?(?:nur\s+nach|nur\s+mit|vorherig\w*\s+(?:schriftlich\w*\s+)?(?:bestätigung|zustimmung))/i,
      /(?:gegenforderungen|gegenansprüche)[^.!?;:\n]{0,80}?(?:nur\s+nach|nur\s+mit)[^.!?;:\n]{0,80}?(?:bestätigung|zustimmung)[^.!?;:\n]{0,80}?(?:in\s+ansatz|geltend\s+gemacht)/i,
      /(?:kann\s+gegen\s+forderungen[^.!?;:\n]{0,80}?nicht\s+aufrechnen|kein\s+zurückbehaltungsrecht\s+geltend\s+machen)/i,
      /(?:aufrechn\w*|gegenforderung\w*)[^.!?;:\n]{0,120}?(?:unabhängig\s+davon,\s+ob|auch\s+wenn|selbst\s+wenn|unbeachtlich)[^.!?;:\n]{0,80}?(?:unbestritten|rechtskräftig)|(?:unabhängig\s+davon,\s+ob|auch\s+wenn|selbst\s+wenn|unbeachtlich)[^.!?;:\n]{0,80}?(?:gegenforderung\w*|aufrechn\w*)[^.!?;:\n]{0,80}?(?:unbestritten|rechtskräftig)/i,
      /(?:einbehalte)[^.!?;:\n]{0,80}?(?:sind\s+nur|nur\s+mit|nur\s+zulässig)/i,
      // Red Team: Euphemismen und faktische Sperren ohne Negationswort
      /(?:einwendungen|einreden|gegenanspr(?:ü|ue)che|gegenrechte)[^.!?;:\n]{0,60}?bleiben[^.!?;:\n]{0,60}?au(?:ß|ss)er\s+betracht/i,
      /(?:verrechnung|aufrechnung)[^.!?;:\n]{0,80}?(?:vorherige[nrs]?\s+)?zustimmung[^.!?;:\n]{0,40}?(?:erforderlich|notwendig|bedarf)/i,
      /(?:geltendmachung|ber(?:ü|ue)cksichtigung|aus(?:ü|ue)bung)[^.!?;:\n]{0,60}?(?:einreden|einwendungen|gegenanspr(?:ü|ue)che|gegenrechte)[^.!?;:\n]{0,80}?(?:ausgeklammert|ausgeblendet|ausgeschlossen|nicht\s+zugelassen|nicht\s+m(?:ö|oe)glich)/i,
      /(?:geltendmachung|aus(?:ü|ue)bung)[^.!?;:\n]{0,60}?(?:gegenrechte|einreden|gegenanspr(?:ü|ue)che|zur(?:ü|ue)ckbehaltungsanspr(?:ü|ue)che|aufrechnung)[^.!?;:\n]{0,120}?(?:richtet\s+sich\s+nach|gilt|erfolgt\s+nach)\s+(?:§\s*\d[^!?;:\n]{0,40}?(?:dieses\s+vertrages|des\s+vertrages|dieser\s+vereinbarung|dieses\s+vertrags)|ziffer|ziff\.?|abschnitt|nr\.?)/i,
      /free\s+of\s+any\s+(?:counterclaim|deduction|retention|set[\s-]?off)/i,
      /(?:bleiben|bleibt)\s+bei\s+der\s+abrechnung\s+unber(?:ü|ue)cksichtigt/i,
    ],
    severity(s) {
      if (/ruhen|lässt.*?unberührt/i.test(s)) return "hoch";
      if (/in\s+ansatz|einbehalte.*?nur\s+mit/i.test(s)) return "hoch";
      // Wenn die Klausel künstliche Einschränkungen (wie Beschränkung auf denselben Monatsabschnitt) vorschreibt, ist sie unwirksam!
      if (/demselben\s+konkreten\s+monatsrechnungsabschnitt|demselben\s+rechnungsabschnitt|mängeln\s+an\s+vorgelagerten.*?ausgeschlossen/i.test(s)) {
        return "hoch";
      }
      // Bösartige Invertierung: Aufrechnung auch/selbst bei unbestrittenen oder rechtskräftigen Forderungen verboten (§ 309 Nr. 3 BGB)
      if (/(?:unabhängig\s+davon,\s+ob|auch\s+wenn|selbst\s+wenn|unbeachtlich)[^.!?;:\n]{0,80}?(?:gegenforderung\w*|aufrechn\w*|forderung\w*)[^.!?;:\n]{0,80}?(?:unbestritten|rechtskräftig)|(?:nicht\s+aufrechnen)[^.!?;:\n]{0,80}?(?:unbestritten|rechtskräftig)/i.test(s)) {
        return "hoch";
      }
      // Einseitige Verbote gegenüber dem Kunden bleiben unwirksam, auch wenn die gesetzliche Ausnahme zitiert wird;
      // die saubere Ausnahme trägt nur, wenn sie nicht mit einem einseitigen Verbot des Kunden verbunden ist.
      if (/unbestritten|rechtskräftig/i.test(s)) {
        const oneSided = /(?:dem\s+kunden|der\s+kunde|auftraggeber|besteller|mieter|k(?:ä|ae)ufer)[^.!?;:\n]{0,70}?(?:untersagt|nicht\s+berechtigt|nicht\s+befugt|kein\s+recht|verwehrt|ausgeschlossen)/i.test(s);
        if (!oneSided) return null;
        return "hoch";
      }
      return "hoch";
    },
    why(ctx) {
      const s = ctx.sentence || "";
      if (/ruhen|lässt.*?unberührt/i.test(s)) {
        return "Vertraglicher Lizenzentzug (Kill-Switch) & Aushöhlung des Zurückbehaltungsrechts (§ 320, § 307 BGB): Die Berechtigung zur Nutzung der Software soll bei Zahlungsverzug automatisch ruhen, selbst wenn der Kunde ein berechtigtes Zurückbehaltungsrecht wegen Mängeln ausübt. Dies schneidet das gesetzliche Druckmittel unzulässig ab.";
      }
      if (/zurückbehaltungsrecht/i.test(s)) {
        if (/monatsrechnungsabschnitt|bestimmte\s+module/i.test(s)) {
          return "Unzulässige Einschränkung von Zurückbehaltungsrechten (§ 309 Nr. 2 lit. b, § 307 BGB): Gesetzliche Zurückbehaltungs- und Leistungsverweigerungsrechte (§ 273, § 320 BGB) dürfen nicht künstlich auf einzelne Monatsabschnitte oder bestimmte Module beschnitten werden.";
        }
        return "Ausschluss des Zurückbehaltungsrechts (§ 309 Nr. 2 lit. b, § 307 BGB): Das gesetzliche Leistungsverweigerungsrecht aus § 273 und § 320 BGB darf formularmäßig nicht pauschal abbedungen werden. Der Ausschluss ist auch im B2B unangemessen.";
      }
      return "Ein pauschaler Ausschluss der Aufrechnung benachteiligt den Vertragspartner unangemessen und verstößt gegen § 309 Nr. 3 BGB (sowie § 307 BGB im B2B). Die Aufrechnung mit unbestrittenen oder rechtskräftig festgestellten Forderungen darf niemals verwehrt werden.";
    },
    suggestion:
      "Gesetzliche Zurückbehaltungsrechte (§ 273, § 320 BGB) uneingeschränkt aufrechterhalten und Aufrechnung mit unbestrittenen oder rechtskräftigen Gegenforderungen stets zulassen.",
  },
  {
    type: "deemed_acceptance",
    title: "Fingierte Abnahme, Mängelgenehmigung & Rügefrist",
    tags: ["bgb-640", "bgb-308-nr-5", "bgb-307", "abnahmefiktion", "ruegefrist", "gewaehrleistung"],
    patterns: [
      /(?:gilt|gelten)[^.!?;:\n]{0,100}?als\s+(?:vollständig\s+|vorbehaltlos\s+)?(?:und\s+)?(?:mangelfrei\s+)?abgenommen/i,
      /(?:unter\s+benennung\s+von\s+fehlern\s+der\s+schweregrade?|showstopper)/i,
      /(?:produktiv\s+nutzt|in\s+eine\s+(?:für\s+dritte\s+zugängliche\s+)?produktionsumgebung\s+überführt)/i,
      /(?:minderungsanspruch|mängel|ansprüche)[^.!?;:\n]{0,80}?(?:binnen|innerhalb|spätestens)\s+(?:\d+|zweiundsiebzig|achtundvierzig)\s+stunden[^.!?;:\n]{0,60}?(?:anmeld\w*|anzumelden|zu\s+melden|anzuzeigen|r(?:ü|ue)gen|geltend)/i,
      /verfallen\s+jedoch\s+ersatzlos/i,
      /(?:gilt|gelten)\s+[^.!?;:\n]{0,80}?als\s+(?:vollständig\s+|vorbehaltlos\s+)?(?:mangelfrei\s+)?genehmigt/i,
      /(?:nicht\s+(?:fristgerecht|rechtzeitig)\s+angezeigte\s+mängel\s+gelten\s+als\s+genehmigt)/i,
      /mängelrüge[^.!?;:\n]{0,60}?(?:binnen|innerhalb|spätestens)[^.!?;:\n]{0,40}?(?:\d+|zwei|drei|vier|fünf)\s+(?:tag|kalendertag|werktag|stunde)/i,
      /spätestens\s+jedoch\s+innerhalb\s+von\s+(?:\d+|zwei|drei|vier|fünf)\s+(?:kalendertagen|tagen|werktagen|stunden)[^.!?;:\n]{0,80}?anzeigen/i,
      /unterbleibt\s+(?:die\s+)?(?:formgerechte\s+|rechtzeitige\s+)?(?:anzeige|rüge)[^.!?;:\n]{0,80}?gilt[^.!?;:\n]{0,60}?genehmigt/i,
      // Red Team: 48 Stunden Ausschlussfrist / binnen fünf Werktagen
      /ausschlussfrist\s+von\s+(?:\d+|achtundvierzig)\s+stunden/i,
      /widrigenfalls\s+gilt[^.!?;:\n]{0,60}?genehmigt/i,
      // Getarnte Abnahmefiktion & Vorbehaltlose Abnahme (§ 640 Abs. 2 S. 2 BGB):
      /gilt\s+[^.!?;:\n]{0,120}?als\s+(?:vorbehaltlos\s+|vollständig\s+)?abgenommen/i,
      /als\s+vorbehaltlos\s+abgenommen/i,
      /(?:vorsysteme|testabläufe)[^.!?;:\n]{0,100}?gilt[^.!?;:\n]{0,60}?abgenommen/i,
      /lässt[^.!?;:\n]{0,60}?frist\s+verstreichen[^.!?;:\n]{0,150}?gilt[^.!?;:\n]{0,60}?abgenommen/i,
      /(?:wird|gelten?)\s+(?:die\s+)?(?:leistung|teilleistung|werk|lieferung)[^.!?;:\n]{0,60}?als\s+(?:vertragsgemäß|genehmigt|abgenommen)\s+(?:behandelt|angesehen|fingiert)/i,
      /(?:als\s+vertragsgemäß\s+behandelt|als\s+vertragsgemäß\s+angesehen)/i,
      // Verfall/Verwirkung von Ansprüchen (nicht: verfallende Kontingente/Stunden)
      /(?:anspr(?:ü|ue)che|m(?:ä|ae)ngelanspr(?:ü|ue)che|gew(?:ä|ae)hrleistungsanspr(?:ü|ue)che)[^.!?;:\n]{0,60}?verfallen[^.!?;:\n]{0,40}?(?:ersatzlos|endg(?:ü|ue)ltig)/i,
      /(?:andernfalls|widrigenfalls)[^.!?;:\n]{0,80}?(?:verfallen|verwirk\w*|erl(?:ö|oe)sch\w*)/i,
      /(?:unterbleibt|unterl(?:ä|ae)sst|erfolgt\s+keine)[^.!?;:\n]{0,80}?(?:m(?:ä|ae)ngelr(?:ü|ue)ge|r(?:ü|ue)ge|anzeige|beanstandung)[^.!?;:\n]{0,90}?(?:verwirk\w*|verfall\w*|erl(?:ö|oe)sch\w*|verloren|ausgeschlossen)/i,
      /(?:anspr(?:ü|ue)che|gew(?:ä|ae)hrleistungsanspr(?:ü|ue)che|m(?:ä|ae)ngelanspr(?:ü|ue)che)[^.!?;:\n]{0,50}?(?:sind|werden|gelten\s+als)?\s*verwirkt/i,
      /(?:m(?:ä|ae)ngelr(?:ü|ue)gen?|m(?:ä|ae)ngel|anspr(?:ü|ue)che)[^.!?;:\n]{0,90}?(?:binnen|innerhalb|sp(?:ä|ae)testens)\s+(?:von\s+)?(?:\d+|ein|zwei|drei|vier|f(?:ü|ue)nf|sechs|sieben|acht|zehn|vierzehn)\s+(?:kalender)?(?:tagen|werktagen|stunden)[^.!?;:\n]{0,60}?(?:zu\s+r(?:ü|ue)gen|zu\s+erheben|anzuzeigen|geltend\s+zu\s+machen)/i,
      // Red Team: Euphemismus-/Split-/Listen-Evasion der Abnahmefiktion
      /als\s+verbindlich\s+(?:übernommen|akzeptiert|angesehen)/i,
      /(?:gilt|gelten)\s+(?:das\s+|die\s+|der\s+)?(?:profil|kandidatenprofil|stellenprofil)[^.!?;:\n]{0,40}?als\s+(?:verbindlich\s+)?(?:übernommen|akzeptiert|angesehen|genehmigt)/i,
      /nicht\s+angezeigte\s+abweichungen[^.!?;:\n]{0,80}?(?:nicht\s+mehr\s+zug(?:ä|ae)nglich|ausgeschlossen|verwirkt|entf(?:ä|ae)llt)/i,
      /(?:lieferung|werk|sache|leistung|artefakte?)[^.!?;:\n]{0,60}?verbleibt\s+endg(?:ü|ue)ltig\s+beim\s+kunden/i,
      /unterbleibt\s+eine\s+meldung[^.!?;:\n]{0,100}?(?:freigegeben|genehmigt|abgenommen|verbindlich)/i,
      /(?:artefakte?|lieferung|leistung)[^.!?;:\n]{0,80}?abschlie(?:ß|ss)end\s+freigegeben/i,
      /(?:widrigenfalls|andernfalls)[^.!?;:\n]{0,120}?als\s+(?:vollst(?:ä|ae)ndig|vertragsgem(?:ä|ae)(?:ß|ss)|mangelfrei|ordnungsgem(?:ä|ae)(?:ß|ss))[^.!?;:\n]{0,60}?(?:angesehen|betrachtet|behandelt)/i,
    ],
    collect(sentences, _full, extra = null) {
      const hits = collectPerSentence(this, sentences, null, extra);
      // DSGVO-konforme Subunternehmer-Regelung (Art. 28 Abs. 2 DSGVO): Der
      // Satz steht ggf. verkürzt ("gilt der Einsatz als genehmigt") nach einem
      // Semikolon, daher Nachbarsatz-Kontext prüfen.
      const DSGVO_CTX = /(?:dsgvo|datenschutz|auftragsverarbeit|unterauftragsverh|verantwortliche[nr]?|subunternehmer|unterauftragnehmer)/i;
      return hits.filter((h) => {
        const own = flattenLineBreaks(sentences[h.i]?.text || "");
        if (!/widerspruch/i.test(own) || !/(?:einsatz|bestellung|beauftragung)/i.test(own)) return true;
        const ctx = [sentences[h.i - 1], sentences[h.i], sentences[h.i + 1]]
          .map((s) => (s ? flattenLineBreaks(s.text) : ""))
          .join(" ");
        return !DSGVO_CTX.test(ctx);
      });
    },
    severity(s) {
      if (/(?:budget|budgeterweiterung|mehrkosten|schätzrahmen|entgelterhöhung|preisanpassung)/i.test(s)) {
        return null;
      }
      // DSGVO-konforme Subunternehmer-Regelung (Art. 28 Abs. 2 DSGVO):
      // Vorabinformation + Widerspruchsrecht sind gerade das Schutzmodell.
      if (
        /(?:dsgvo|datenschutz|auftragsverarbeit|art\.?\s*28)/i.test(s) &&
        /widerspruch/i.test(s) &&
        /(?:einsatz|unterauftragsverh|weitere[n]?\s+auftragsverarbeiter|subunternehmer|unterauftragnehmer)/i.test(s)
      ) {
        return null;
      }
      return "hoch";
    },
    why(ctx) {
      const s = ctx.sentence || "";
      if (/abgenommen/i.test(s)) {
        return "Getarnte Abnahmefiktion (§ 640 Abs. 2 Satz 2 BGB): Eine Abnahmefiktion durch Fristablauf ist nur wirksam, wenn der Anbieter den Kunden bei Fristbeginn ausdrücklich auf die gesetzlichen Folgen hinweist. Zudem darf das bloße Testen oder Einbinden in Vorsysteme nicht als vorbehaltlose Abnahme fingiert werden.";
      }
      if (
        /(?:preis|entgelt|verg(?:ü|ue)tung|bedingungen|bedingungswerk)/i.test(s) &&
        /(?:genehmigt|als\s+vereinbart|akzeptiert)/i.test(s)
      ) {
        return "Fingierte Zustimmung zu Preis- oder Vertragsänderungen (§ 308 Nr. 5, § 307 BGB): Änderungen gelten durch bloßes Veröffentlichen oder durch Schweigen als genehmigt. Eine solche Zustimmungsfiktion ist unwirksam, weil sie den Vertrag ohne Ihre Erklärung ändert.";
      }
      return "Eine Frist von wenigen Tagen oder Stunden zur Mängelanzeige mit anschließender Genehmigungsfiktion auch für Folgemängel ist nach § 308 Nr. 5 BGB und § 307 BGB unzulässig. Sie entwertet das Gewährleistungsrecht und kehrt die Beweislast unfair um.";
    },
    suggestion:
      "Verhandeln Sie eine praxisnahe Rügefrist von mindestens 14 Tagen ab Entdeckung und streichen Sie die Genehmigungsfiktion für Folgemängel ersatzlos.",
  },
  {
    type: "statute_of_limitations",
    title: "Pauschale Verjährungsverkürzung oder Verfall",
    tags: ["bgb-309-nr-7", "bgb-307", "verjaehrung", "haftung"],
    patterns: [
      /(?:verjährungsfrist|gewährleistungsfrist)[^.!?;:\n]{0,80}?(?:beträgt|von)[^.!?;:\n]{0,40}?(?:\d+|sechs|zwölf)\s*(?:\(\d+\)\s*)?(?:monat|woche|tag)/i,
      /verjähren\s+(?:innerhalb|nach|mit\s+ablauf\s+von)[^.!?;:\n]{0,50}?(?:\d+|ein|zwei|drei|sechs|180|90)\s+(?:monat|woche|tag|kalendertag)/i,
      /verjährungsfrist[^.!?;:\n]{0,60}?(?:beträgt|verkürzt|von)[^.!?;:\n]{0,40}?(?:\d+|ein|zwei|drei|sechs|180|90)\s+(?:monat|woche|tag|kalendertag)/i,
      /ansprüche[^.!?;:\n]{0,80}?verjähren[^.!?;:\n]{0,50}?(?:innerhalb|binnen|nach|mit\s+ablauf\s+von)\s+(?:\d+|sechs|zwölf|180)\s+(?:monat|tag|kalendertag)/i,
      // Red Team: Verfallsklauseln / Erlöschen
      /ansprüche[^.!?;:\n]{0,80}?(?:treten\s+in\s+den\s+verfall|erlöschen\s+endgültig)[^.!?;:\n]{0,60}?(?:frist\s+von|innerhalb\s+von)\s+(?:\d+|drei|sechs)\s+monat/i,
      /(?:erlöschen|erlischt|verfallen|verfällt|entfallen)[^;:\n]{0,60}?(?:\d+|ein|zwei|drei|sechs|neun|zwölf)\s*(?:\(\d+\)\s*)?(?:monate?|wochen?|jahre?)(?:\s+nach|\s+ab|\s+seit)/i,
      /(?:ansprüche|mängelansprüche|gewährleistungsansprüche)[^;:\n]{0,60}?(?:erlöschen|verfallen|entfallen)[^;:\n]{0,60}?(?:monate?|wochen?|jahre?)/i,
      // Verkürzung sprachlich als bloße "Geltendmachung" umschrieben + Erfüllungsfiktion:
      /(?:anspr(?:ü|ue)che|m(?:ä|ae)ngelanspr(?:ü|ue)che|gew(?:ä|ae)hrleistungsanspr(?:ü|ue)che)[^.!?;:\n]{0,120}?(?:ausschlie(?:ß|ss)lich\s+|nur\s+)?innerhalb\s+von\s+(?:\d+|ein|zwei|drei|sechs|neun|zw(?:ö|oe)lf)\s*(?:\(\d+\)\s*)?monaten[^.!?;:\n]{0,100}?(?:geltend\s+gemacht|geltend\s+zu\s+machen|zu\s+erheben|erhoben\s+werden)/i,
      /(?:anspr(?:ü|ue)che|m(?:ä|ae)ngelanspr(?:ü|ue)che|gew(?:ä|ae)hrleistungsanspr(?:ü|ue)che)[^.!?;:\n]{0,100}?verwirken[^.!?;:\n]{0,80}?(?:mit\s+ablauf|nach|innerhalb|binnen|frist)/i,
      /verwirken\s+mit\s+ablauf\s+von[^.!?;:\n]{0,60}?(?:monat|woche|tag|jahr)/i,
      /geltendmachung[^.!?;:\n]{0,80}?(?:nach\s+fristablauf|nach\s+ablauf\s+der\s+frist)[^.!?;:\n]{0,60}?ausgeschlossen/i,
      // Englische Verjährungsverkürzung (common law-style limitation clauses):
      /(?:any\s+)?(?:claim|action|proceeding|suit)[^.!?;:\n]{0,100}?(?:must\s+be|shall\s+be|has\s+to\s+be)\s+(?:asserted|brought|commenced|filed|instituted)[^.!?;:\n]{0,60}?within\s+(?:\d+|one|two|three|six|nine|twelve)\s*(?:\(\d+\)\s*)?(?:months?|years?|weeks?)/i,
      /(?:forever\s+barred|time[-\s]?barred|irrevocably\s+waive[^.!?;:\n]{0,60}?claims?)/i,
    ],
    severity(s) {
      const hasExclusion = /körper|leben|gesundheit|vorsatz|grobe\s+fahrlässigkeit/i.test(s);
      return !hasExclusion ? "hoch" : "mittel";
    },
    why:
      "Eine pauschale Verkürzung der Verjährung oder Verfallsklausel für Ansprüche ohne ausdrückliche Ausnahme für Körperschäden, Vorsatz und grobe Fahrlässigkeit verstößt gegen § 309 Nr. 7 lit. a, b BGB sowie § 307 BGB.",
    suggestion:
      "Mindestens die gesetzliche Regelverjährung (§ 195 BGB: 3 Jahre) erhalten oder eine Verkürzung frühestens auf 12 Monate beschränken, mit zwingender Ausnahme für Körperschäden und grobes Verschulden.",
  },
  {
    type: "written_form_trap",
    title: "Unwirksame Formklausel & Kündigungserschwernis",
    tags: ["bgb-309-nr-13", "bgb-305b", "schriftform", "formfalle", "kuendigung"],
    patterns: [
      /(?:schriftlich\s+per\s+eingeschriebenem\s+brief|per\s+eingeschriebenem\s+brief|eingeschriebenen?\s+brief)/i,
      /(?:kündigung\w*|erklärungen)[^.!?;:\n]{0,80}?(?:bedarf|bedürfen|muss|müssen)[^.!?;:\n]{0,80}?(?:der\s+)?(?:schriftform|eigenhändig|einschreiben)/i,
      /(?:per|mittels|durch)\s+(?:einschreiben|eingeschriebenen?\s+brief)/i,
      /(?:änderungen|ergänzungen)[^.!?;:\n]{0,80}?(?:bedürfen|vereinbarungen)[^.!?;:\n]{0,50}?(?:der\s+)?schriftform/i,
      /doppelte\s+schriftformklausel/i,
      /(?:mündliche\s+abreden|nebenabreden)[^.!?;:\n]{0,80}?(?:sind\s+unwirksam|ausgeschlossen|haben\s+keine\s+geltung)/i,
      /(?:mündliche|fernmündliche)?[^.!?;:\n]{0,60}?(?:nebenabreden|zusicherungen|zusagen)[^.!?;:\n]{0,120}?(?:keinerlei\s+wirkung|keine\s+wirkung|sind\s+unwirksam|ausgeschlossen|haben\s+keine\s+geltung|unbeachtlich|entfalten\s+keine)/i,
      // Qualifizierte / doppelte Schriftformklausel (§ 305b BGB):
      /erfordernis\s+der\s+schriftform\s+kann\s+seinerseits\s+nur\s+schriftlich\s+abbedungen/i,
      /(?:konkludente\s+oder\s+mündliche\s+abbedingung|abbedingung\s+dieses\s+formerfordernisses)/i,
      // Erschwerte Kündigung (§ 309 Nr. 13 BGB):
      // Ausschluss von Textform / elektronischer Form, Einschreiben mit Rückschein
      /kündigung[^.!?;:\n]{0,80}?bedarf[^.!?;:\n]{0,80}?(?:schriftform|eigenhändig)/i,
      /(?:eingeschriebenem\s+brief\s+mit\s+rückschein|einschreiben\s+mit\s+rückschein)/i,
      /(?:elektronische\s+form[^.!?;:\n]{0,60}?sowie\s+textform[^.!?;:\n]{0,60}?ausgeschlossen|textform[^.!?;:\n]{0,60}?ausdrücklich\s+ausgeschlossen)/i,
      /(?:notariell\s+(?:beglaubigt|beurkundet)|öffentlich\s+beglaubigt|eigenhändig\s+(?:unterzeichnet|unterschrieben))/i,
      // Red Team: Formfalle ohne die Standardbegriffe (Synonym-/Nominalstil-Evasion)
      /handschriftlich[^.!?;:\n]{0,60}?(?:unterzeichn\w*|schreiben)/i,
      /schreiben\s+in\s+papierform/i,
      /unterschriftsbeglaubigt\w*/i,
      /(?:elektronisch\w*\s+weg|telefax|kundenportal|e-?mail)[^.!?;:\n]{0,120}?(?:keine\s+rechtswirkung|ohne\s+bedeutung|nicht\s+wirksam)/i,
      /(?:wirksamkeit|wirksam)[^.!?;:\n]{0,100}?vorgeschaltet[^.!?;:\n]{0,120}?unterzeichneten\s+originaldokuments/i,
      /einreichung\s+eines\s+unterzeichneten\s+originaldokuments/i,
      /(?:vertragsbeendigung|beendigung|k(?:ü|ue)ndigung)[^.!?;:\n]{0,120}?nur\s+durch[^.!?;:\n]{0,140}?(?:papierform|handschriftlich)/i,
      /textform\s+ist\s+(?:ausdr(?:ü|ue)cklich\s+)?ausgeschlossen/i,
    ],
    severity(s) {
      // Wenn Textform (E-Mail) ausdrücklich als möglich/zulässig benannt ist, ist es unkritisch!
      if (
        /(?:textform\s+gen(?:ü|ue)gt|in\s+textform\s+(?:m(?:ö|oe)glich|zul(?:ä|ae)ssig|ausreichend)|per\s+e-?mail\s+(?:m(?:ö|oe)glich|zul(?:ä|ae)ssig|ausreichend|gen(?:ü|ue)gt))/i.test(
          s
        ) &&
        !/ausgeschlossen|ohne\s+bedeutung|keine\s+(?:rechts)?wirkung/i.test(s)
      )
        return null;
      // Strenge/doppelte Schriftform, Erschwerung per Einschreiben-Rückschein oder Ausschluss von Mündlichkeit/Textform ist HOCH riskant
      const strict =
        /doppelte|aufhebung\s+des\s+schriftformerfordernisses|seinerseits\s+nur\s+schriftlich|abbedingung\s+dieses\s+formerfordernisses|rückschein|ausgeschlossen|mündliche\s+abreden|nebenabreden|eigenhändig|notariell|beglaubigt|beurkundet|handschriftlich|papierform|elektronisch\w*\s+weg|ohne\s+bedeutung|keine\s+rechtswirkung|handelsregister|gesch(?:ä|ae)ftsanschrift/i.test(
          s
        );
      return strict ? "hoch" : "mittel";
    },
    why(ctx) {
      const s = ctx.sentence || "";
      if (/rückschein|kündigung.*?bedarf|eigenhändig|textform.*?ausgeschlossen/i.test(s)) {
        return "Erschwerte Kündigungsform (§ 309 Nr. 13 BGB): Für Kündigungen und Erklärungen darf formularmäßig keine strengere Form als die Textform (§ 126b BGB, z. B. E-Mail) verlangt werden. Die Pflicht zu eigenhändiger Unterschrift oder Einschreiben mit Rückschein ist unwirksam.";
      }
      if (/seinerseits\s+nur\s+schriftlich|abbedingung/i.test(s)) {
        return "Unwirksame doppelte Schriftformklausel (§ 305b BGB): Eine Klausel, nach der die Schriftform nur schriftlich abbedungen werden kann, ist nach ständiger BGH-Rechtsprechung unwirksam, da sie den Vorrang mündlicher Individualabreden aushöhlt.";
      }
      return "Eine Klausel, die für Vertragsänderungen die strenge Schriftform vorschreibt und Textform (E-Mail) ausschließt, ist nach § 309 Nr. 13 BGB unwirksam. Zudem haben individuelle mündliche Absprachen nach § 305b BGB stets Vorrang vor Formular-AGB.";
    },
    suggestion:
      "Auf Textform (z. B. E-Mail) umstellen: Kündigungen und Änderungen bedürfen der Textform (§ 126b BGB). Mündliche Individualabreden haben stets Vorrang.",
  },
  {
    type: "burden_of_proof",
    title: "Unzulässige Beweislastumkehr & Gefahrverlagerung",
    tags: ["bgb-309-nr-12", "bgb-307", "beweislast", "fiktion"],
    patterns: [
      /beweislast[^.!?;:\n]{0,100}?(?:obliegt|trägt|beim|beim\s+auftraggeber|beim\s+kunden)/i,
      /(?:bestätigt|erklärt)[^.!?;:\n]{0,80}?(?:mangelfrei|fehlerfrei|vollständig\s+übergeben)/i,
      /beweislast[^.!?;:\n]{0,60}?(?:beim\s+kunden|obliegt\s+dem\s+kunden|trägt\s+der\s+kunde)/i,
      /nachweis[^.!?;:\n]{0,140}?tr(?:ä|ae)gt\s+der\s+(?:kunde|auftraggeber|besteller)/i,
      /gilt\s+als\s+zugegangen[^.!?;:\n]{0,60}?(?:\d+|zwei|drei)\s+tagen?\s+nach\s+(?:absendung|aufgabe)/i,
      /zugangsfiktion/i,
      // Negative Beweislast / Verschuldensumkehr:
      /weist\s+lückenlos\s+nach,\s+dass\s+die\s+ursache\s+ausschließlich/i,
      /es\s+sei\s+denn,\s+der\s+kunde\s+weist[^.!?;:\n]{0,80}?nach/i,
      /obliegt\s+es\s+dem\s+kunden[^.!?;:\n]{0,80}?(?:darzulegen\s+und\s+nachzuweisen|nachzuweisen|beweisen|darzulegen)/i,
      /(?:nachzuweisen|beweis\w*)[^.!?;:\n]{0,100}?dass\s+die\s+ursache\s+nicht\s+auf/i,
      /dass\s+die\s+ursache\s+nicht\s+auf[^.!?;:\n]{0,100}?beruht/i,
      // Beweislast ausdrücklich beim Kunden (auch in Voranstellung: "Der Kunde trägt die Beweislast"):
      /(?:kunde|auftraggeber|besteller|mieter|bewohner|versicherungsnehmer)\s+trägt\s+(?:die\s+|sämtliche\s+|die\s+alleinige\s+)?(?:volle\s+)?beweislast/i,
      /beweislast\s+dafür,\s+dass[^.!?;:\n]{0,200}?nicht\s+auf[^.!?;:\n]{0,100}?beruht/i,
      /zurückzuführen\s+(?:ist|sind)\s+und\s+nicht\s+auf/i,
      /(?:darlegungs-?\s*und\s*beweisbelastet|beweisbelastet|darlegungsbelastet)/i,
      /(?:trägt|tragen)[^;:\n]{0,40}?(?:darlegungs-?\s*und\s*)?(?:beweis|nachweis)last/i,
      /(?:beweis|darlegungs)last[^;:\n]{0,40}?(?:liegt|trifft|obliegt)\s+(?:ausschließlich\s+)?(?:beim|dem|den)\s+(?:kunden|auftraggeber|besteller)/i,
      // Vorab-Anerkenntnis der Mangelfreiheit ohne die Keywords "bestätigt ... mangelfrei":
      /(?:anerkennt|erkennt\s+an|best(?:ä|ae)tigt)[^.!?;:\n]{0,120}?(?:als\s+ordnungsgem(?:ä|ae)(?:ß|ss)|als\s+vollst(?:ä|ae)ndig|in\s+jeder\s+hinsicht\s+vollst(?:ä|ae)ndig)/i,
      /nachweis\s+abweichender\s+umst(?:ä|ae)nde[^.!?;:\n]{0,100}?(?:von\s+ihm\s+zu\s+erbringen|obliegt\s+(?:ihm|dem\s+kunden)|trifft\s+ihn)/i,
      // Negativbeweis-Pflicht in untypischer Syntax ("Es obliegt dem Auftraggeber, den Nachweis ... zu führen"):
      /obliegt\s+dem\s+(?:kunden|auftraggeber|besteller)[^.!?;:\n]{0,80}?den\s+nachweis[^.!?;:\n]{0,80}?zu\s+f(?:ü|ue)hren/i,
      // Umgekehrte Wortstellung: "Der Nachweis, dass ... obliegt dem Auftraggeber" (Welle D):
      /(?:nachweis|beweis)[^.!?;:\n]{0,160}?obliegt\s+dem\s+(?:kunden|auftraggeber|besteller)/i,
      /(?:nachweis|beweis)[^.!?;:\n]{0,60}?nicht\s+auf\s+(?:gegebenheiten|umst(?:ä|ae)nde|vorg(?:ä|ae)nge)[^.!?;:\n]{0,80}?(?:seines\s+eigenen\s+verantwortungsbereichs|eigenen\s+verantwortungsbereich)/i,
      // Beweismonopol des Anbieters + Fiktion der ordnungsgemäßen Leistung (Split-Sentence):
      /(?:entscheiden|ma(?:ß|ss)geblich|verbindlich)\s+(?:sind\s+)?(?:die\s+|ausschlie(?:ß|ss)lich\s+die\s+)?(?:aufzeichnungen|protokollierung|protokolle|dokumentation)\s+des\s+(?:anbieters|auftragnehmers|dienstleisters)/i,
      /aufzeichnungen\s+des\s+(?:anbieters|auftragnehmers|dienstleisters)[^.!?;:\n]{0,60}?entscheiden[^.!?;:\n]{0,40}?abschlie(?:ß|ss)end/i,
      /gelingt\s+dem\s+(?:kunden|auftraggeber|besteller)[^.!?;:\n]{0,40}?der\s+gegenbeweis\s+nicht/i,
      /gegenbeweis\s+nicht[^.!?;:\n]{0,80}?gilt\s+die\s+leistung\s+als\s+(?:ordnungsgem(?:ä|ae)(?:ß|ss)\s+erbracht|erf(?:ü|ue)llt|abgenommen)/i,
      /(?:m(?:ä|ae)ngelrechte|m(?:ä|ae)ngelanspr(?:ü|ue)che|gew(?:ä|ae)hrleistungsanspr(?:ü|ue)che)[^.!?;:\n]{0,60}?entfallen(?!\s+nicht)/i,
    ],
    severity: "hoch",
    why(ctx) {
      const s = ctx.sentence || "";
      if (/ursache\s+nicht\s+auf|obliegt\s+es\s+dem\s+kunden/i.test(s)) {
        return "Unzulässige Beweislastumkehr & Negativbeweis bei Mängeln (§ 309 Nr. 12 BGB, § 307 BGB): Dem Kunden wird die Beweislast dafür aufgebürdet, dass Fehlfunktionen nicht auf Drittfaktoren, Systemkonfigurationen oder Fehlbedienungen beruhen. Die formularmäßige Pflicht zu einem solchen Negativbeweis ist unwirksam.";
      }
      if (/weist\s+lückenlos\s+nach/i.test(s)) {
        return "Negative Beweislastumkehr (§ 309 Nr. 12 BGB, § 307 BGB): Dem Kunden wird die Pflicht auferlegt, einen lückenlosen Negativbeweis für fremde Systemfehler zu führen. Dies begründet eine unzulässige Gefährdungshaftung.";
      }
      if (/tr(?:ä|ae)gt[^.!?;:\n]{0,60}?die\s+beweislast|beweislast[^.!?;:\n]{0,60}?(?:tr(?:ä|ae)gt|obliegt)/i.test(s)) {
        return "Auferlegung der Beweislast für das Verhalten des Anbieters (§ 309 Nr. 12 BGB): Der Kunde müsste im Streitfall beweisen, dass der Anbieter seine Pflichten nicht erfüllt hat. Das kehrt die gesetzliche Beweislastverteilung zu seinen Lasten um.";
      }
      if (/(?:zur\s+kenntnis\s+genommen|s(?:ä|ae)mtliche\s+hinweise)/i.test(s)) {
        return "Vorab-Bestätigung der Kenntnisnahme (§ 309 Nr. 12 BGB): Der Kunde soll pauschal bestätigen, alle Hinweise, Leistungsbeschreibungen und Bedingungen erhalten und verstanden zu haben. Diese Beweislastumkehr ist unwirksam, weil sie dem Anbieter den Nachweis erspart, was Vertragsinhalt geworden ist.";
      }
      return "Klauseln, die den Kunden vorab bestätigen lassen, dass eine Software fehlerfrei ist, oder den Zugang von Rechnungen/Kündigungen fingieren, verstoßen gegen § 309 Nr. 12 BGB (unzulässige Beweislastumkehr zu Lasten des Vertragspartners).";
    },
    suggestion:
      "Klausel streichen. Gesetzliche Beweislastverteilung nach BGB beibehalten.",
  },
  {
    type: "dynamic_url_referral",
    title: "Intransparenter dynamischer URL-Verweis (BGH III ZR 59/24)",
    tags: ["bgh-iii-zr-59-24", "bgb-307", "transparenzgebot", "url-verweis"],
    patterns: [
      /abrufbar\s+unter\s+https?:\/\/[^\s]+(?:\s+in\s+ihrer\s+jeweils\s+gültigen\s+fassung|\s+in\s+der\s+jeweils)/i,
      /(?:anlagen?\s+)?abrufbar\s+unter:?\s+(?:https?:\/\/|www\.)[^\s]+/i,
      /(?:abrufbar\s+unter|downloadbar\s+unter|einsehbar\s+unter)(?::|\b)/i,
      /(?:https?:\/\/|www\.)[^\s]+.*?(?:anlagen|agb|bedingungen|archiv)/i,
      /(?:jeweils\s+aktuellen|jederzeit\s+abrufbaren)\s+(?:agb|richtlinien|leistungsbeschreibungen|bedingungen)[^.!?;:\n]{0,80}?(?:unter|abrufbar|website|url)/i,
      /es\s+gelten\s+die\s+(?:jeweils\s+)?(?:aktuellen\s+)?(?:nutzungsbedingungen|agb|richtlinien|bedingungen)[^.!?;:\n]{0,80}?(?:https?:\/\/|www\.)/i,
      /(?:jeweils\s+aktuellen|jederzeit\s+abrufbaren)\s+agb[^.!?;:\n]{0,80}?(?:unter|abrufbar\s+unter|website|url)/i,
      /es\s+gelten\s+die\s+(?:jeweils\s+)?(?:aktuellen\s+)?(?:nutzungsbedingungen|agb)[^.!?;:\n]{0,80}?(?:https?:\/\/|www\.)/i,
      /abrufbar\s+unter[^.!?;:\n]{0,80}?(?:in\s+ihrer\s+jeweils\s+gültigen\s+fassung)/i,
      /(?:in\s+ihrer\s+jeweiligen\s+fassung|in\s+der\s+jeweils\s+aktuellen\s+fassung)[^.!?;:\n]{0,80}?(?:abrufbar\s+unter|unter)\s+(?:https?:\/\/|www\.)/i,
      /(?:abrufbar\s+unter|unter)\s+(?:https?:\/\/|www\.)[^\s]+[^.!?;:\n]{0,80}?(?:in\s+ihrer\s+jeweiligen\s+fassung)/i,
      /(?:ergänzend\s+)?gelten\s+die\s+[^.!?;:\n]{0,80}?(?:nutzungsbedingungen|agb|bedingungen|richtlinien)[^.!?;:\n]{0,100}?(?:https?:\/\/|www\.)/i,
      // Red Team: dynamische Verweise ohne URL / mit invertierter Syntax
      /(?:auf\s+der\s+website|auf\s+deren?\s+website)[^.!?;:\n]{0,90}?ver(?:ö|oe)ffentlichten?\s+(?:nutzungsbedingungen|agb|bedingungen|richtlinien|policies)/i,
      /in\s+der\s+(?:dort\s+)?jeweils\s+(?:abrufbaren|g(?:ü|ue)ltigen)\s+fassung/i,
      /online-?(?:fassung|version)[^.!?;:\n]{0,100}?(?:servicebedingungen|bedingungen|agb|richtlinien|policies)/i,
      /(?:anbieter|auftragnehmer)[^.!?;:\n]{0,60}?(?:jederzeit\s+)?fortschreib\w*/i,
      /(?:kundenportal|portal)[^.!?;:\n]{0,80}?hinterlegten\s+(?:policies|richtlinien|bedingungen|agb|regelungen)/i,
      /deren\s+wortlaut[^.!?;:\n]{0,60}?fortschreiben/i,
      /unter\s+(?:https?:\/\/|www\.)[^\s,;]+[^.!?;:\n]{0,90}?abrufbaren?\s+(?:regelungen|bedingungen|richtlinien|agb|fassung|version)/i,
      /(?:regelungen|bedingungen|richtlinien)[^.!?;:\n]{0,60}?die\s+der\s+anbieter\s+jeweils\s+aktualisiert/i,
      /terms[^.!?;:\n]{0,90}?published\s+at\s+[^\s,;]+/i,
      /published\s+at\s+[^\s,;]+[^.!?;:\n]{0,90}?(?:as\s+amended|amended|updated|revised)/i,
      /as\s+amended\s+by\s+the\s+(?:vendor|provider|company)/i,
      /(?:as\s+amended|updated)\s+from\s+time\s+to\s+time/i,
    ],
    severity: "hoch",
    why:
      "Ein bloßer dynamischer Link auf Online-AGB ohne Nennung einer konkreten, unveränderlichen Versionsnummer (z. B. Stand 01.01.2026) ist nach ständiger Rechtsprechung des BGH (u. a. Urteil vom 10.07.2025 – III ZR 59/24) wegen Verstoßes gegen das Transparenzgebot (§ 307 Abs. 1 S. 2 BGB) unwirksam.",
    suggestion:
      "Statisches Dokument mit eindeutiger Versionsbezeichnung und Datumsstand als feste Anlage zum Vertrag beifügen.",
  },
  {
    type: "vicarious_agents",
    title: "Haftungsausschluss für Erfüllungsgehilfen & Subunternehmer",
    tags: ["bgb-278", "bgb-307", "erfuellungsgehilfen", "subunternehmer", "auswahlverschulden"],
    patterns: [
      /(?:erfüllungsgehilf\w*|subunternehmer\w*|unterauftragnehmer\w*)[^.!?;:\n]{0,120}?(?:nur\s+bei\s+(?:eigenem\s+)?auswahlverschulden|beschränkt\s+auf\s+auswahlverschulden|auswahlverschulden\s+beschränkt)/i,
      /(?:haftet|haftung)[^.!?;:\n]{0,80}?(?:für\s+das\s+verschulden\s+von\s+erfüllungsgehilfen|für\s+erfüllungsgehilfen)[^.!?;:\n]{0,80}?(?:ausgeschlossen|nur\s+bei\s+vorsatz)/i,
      /(?:für\s+handlungen\s+(?:und\s+unterlassungen\s+)?von\s+subunternehmern|unterauftragnehmern)[^.!?;:\n]{0,80}?(?:übernimmt\s+der\s+anbieter\s+keine\s+haftung|haftet\s+der\s+anbieter\s+nicht)/i,
      /(?:schließt|schließen)\s+(?:jegliche\s+)?haftung\s+für[^.!?;:\n]{0,80}?(?:erfüllungsgehilfen|subunternehmer)[^.!?;:\n]{0,30}?aus/i,
      /(?:haftung|schadensersatz)[^.!?;:\n]{0,80}?(?:für\s+(?:handlungen\s+(?:oder\s+unterlassungen\s+)?von\s+|das\s+verschulden\s+von\s+)?(?:seiner\s+)?(?:erfüllungsgehilfen|subunternehmer))/i,
      // Reduktion auf Auswahlverschulden ("haftet ... nur im Rahmen der sorgfältigen Auswahl"):
      /nur\s+im\s+rahmen\s+der\s+sorgf(?:ä|ae)ltigen\s+(?:auswahl|auswahl\s+und\s+(?:überwachung|anweisung))/i,
      /(?:beschr(?:ä|ae)nkt\s+auf|reduziert\s+auf|nur\s+bei)\s+(?:einem\s+)?(?:eigenen\s+)?auswahlverschulden/i,
      // "keine Verantwortung"/"keine Haftung" für herangezogene Dritte:
      /(?:für\s+das\s+verhalten|f(?:ü|ue)r\s+handlungen\s+(?:und\s+unterlassungen\s+)?)[^.!?;:\n]{0,100}?(?:von\s+ihm\s+)?(?:beauftragten|herangezogenen|eingesetzten)[^.!?;:\n]{0,60}?(?:nachunternehmer|unterauftragnehmer|dritten|subunternehmer|erfüllungsgehilfen)[^.!?;:\n]{0,80}?(?:keine\s+verantwortung|keine\s+haftung|(?:übernimmt|trifft)\s+(?:der\s+)?(?:anbieter|auftragnehmer|dienstleister)\s+keine)/i,
      // Vollständiger Ausschluss der Gehilfenhaftung (Klartext, schließt acc-direct-24):
      /(?:haftung|einstandspflicht)[^.!?;:\n]{0,80}?(?:f(?:ü|ue)r\s+das\s+verschulden\s+(?:seiner\s+|der\s+)?|f(?:ü|ue)r\s+(?:das\s+verhalten\s+seiner\s+)?)(?:erfüllungsgehilfen|unterauftragnehmer|subunternehmer|nachunternehmer)[^.!?;:\n]{0,80}?(?:ausgeschlossen|abbedungen|ist\s+nicht\s+gegeben|besteht\s+nicht)/i,
      // Cross-Reference: Einstandspflicht für Nachunternehmer "gilt Ziffer/§ X ... entsprechend":
      /(?:einstandspflicht|haftung|verantwortlichkeit)[^.!?;:\n]{0,100}?(?:nachunternehmer|unterauftragnehmer|subunternehmer|eingesetzten\s+dritten)[^.!?;:\n]{0,100}?gilt[^.!?;:\n]{0,40}?(?:ziffer|§|\d)/i,
      // Englisch: Ausschluss der Gehilfenhaftung ("not responsible for the acts ... of its subcontractors"):
      /(?:shall\s+not\s+be\s+(?:responsible|liable)|not\s+be\s+responsible)[^.!?;:\n]{0,100}?(?:for\s+(?:the\s+)?acts?|subcontractors?|agents?)/i,
      /(?:acts?\s+or\s+omissions|acts?\s+and\s+omissions)[^.!?;:\n]{0,100}?(?:subcontractors?|agents?)[^.!?;:\n]{0,140}?(?:excluded|disclaimed|no\s+liability)/i,
    ],
    severity: "hoch",
    why:
      "Unzulässiger Ausschluss der Erfüllungsgehilfenhaftung (§ 278 BGB, § 307 Abs. 2 Nr. 1 BGB): Der Anbieter versucht, seine gesetzliche Einstandspflicht für eingesetzte Dritte und Subunternehmer auf bloßes 'Auswahlverschulden' zu reduzieren oder ganz abzubedingen. In AGB ist eine solche Haftungsflucht unwirksam.",
    suggestion:
      "Erfüllungsgehilfenklausel streichen. Der Anbieter muss für das Verschulden von Subunternehmern und Gehilfen wie für eigenes Verschulden einstehen (§ 278 BGB).",
  },
  {
    type: "unreasonable_audit",
    title: "Übergriffiges Audit- und Kontrollrecht",
    tags: ["bgb-307", "audit", "kontrollrecht", "datenschutz", "hausrecht"],
    patterns: [
      /(?:vorankündigung\s+von|frist\s+von)\s+(?:\d+|zwei|drei|vier|fünf|sieben)\s+(?:werktagen?|tagen?)[^.!?;:\n]{0,100}?(?:geschäftsräume|systeme|inspizieren|prüfen|audit)/i,
      /(?:geschäftsräume|it-systeme|nutzerkonten)[^.!?;:\n]{0,80}?(?:zu\s+inspizieren|zu\s+prüfen|auditieren)/i,
      /(?:audit|kontrollrecht|vor-ort-prüfung|betriebsprüfung)[^.!?;:\n]{0,120}?(?:ohne\s+(?:vorherige\s+)?ankündigung|jederzeit|ohne\s+anlass)/i,
      /(?:geschäftsräume|systeme\s+des\s+kunden)[^.!?;:\n]{0,80}?(?:jederzeit\s+zu\s+betreten|ohne\s+vorankündigung\s+zu\s+betreten)/i,
      /(?:kosten\s+des\s+audits|auditkosten)[^.!?;:\n]{0,80}?(?:trägt\s+(?:in\s+jedem\s+fall\s+)?der\s+kunde|sind\s+vom\s+kunden\s+zu\s+tragen)/i,
      /(?:auditieren|überprüfen|kontrollieren)[^.!?;:\n]{0,80}?(?:ohne\s+vorankündigung|jederzeit|ohne\s+anlass)/i,
      /(?:jederzeit|ohne\s+(?:vorherige\s+)?(?:vor)?ankündigung)[^.!?;:\n]{0,80}?(?:auditieren|überprüfen|kontrollieren|audits?\s+durchzuführen)/i,
      // Red Team: Euphemismus-/Nominal-/Listen-/Cross-Reference-Evasion
      /zutritt\s+zu\s+seinen\s+betriebsst(?:ä|ae)tten/i,
      /einsichtnahme[^.!?;:\n]{0,60}?anlassbezogen/i,
      /ein\s+vorlauf\s+entf(?:ä|ae)llt/i,
      /kosten\s+der\s+ma(?:ß|ss)nahme\s+tr(?:ä|ae)gt\s+der\s+kunde/i,
      /einsichtnahme\s+in\s+die\s+prozessrelevanten\s+systeme\s+des\s+kunden/i,
      /zeitpunkt[^.!?;:\n]{0,60}?im\s+ermessen\s+des\s+anbieters/i,
      /vorabinformation[^.!?;:\n]{0,80}?nicht\s+vorgesehen/i,
      /pr(?:ü|ue)fungen\s+werden\s+unmittelbar\s+durchgef(?:ü|ue)hrt/i,
      /eine\s+terminabstimmung\s+entf(?:ä|ae)llt/i,
      /aufwendungen\s+f(?:ü|ue)r\s+pr(?:ü|ue)fungshandlungen[^.!?;:\n]{0,60}?zulasten\s+des\s+kunden/i,
      /zugang\s+zu\s+den\s+gesch(?:ä|ae)ftsr(?:ä|ae)umen[^.!?;:\n]{0,40}?inspektion/i,
      /(?:kontrollrechte|pr(?:ü|ue)fungsrechte)[^.!?;:\n]{0,140}?unabh(?:ä|ae)ngig\s+von\s+einem\s+konkreten\s+anlass/i,
    ],
    severity(s) {
      return /ohne\s+vorankündigung|jederzeit|kosten.*?der\s+kunde|unmittelbar\s+durchgef(?:ü|ue)hrt|terminabstimmung\s+entf(?:ä|ae)llt|zulasten\s+des\s+kunden|anlassbezogen/i.test(
        s
      )
        ? "hoch"
        : "mittel";
    },
    why:
      "Unangemessenes Audit- und Kontrollrecht (§ 307 BGB): Unangekündigte Vor-Ort-Audits oder pauschale Kostenabwälzungen auf den Kunden verletzen das Hausrecht und Geschäftsgeheimnisse. Audits müssen zwingend mit Vorankündigungsfrist (mind. 14 Tage) und zu üblichen Geschäftszeiten erfolgen.",
    suggestion:
      "Auditrecht einschränken: Mindestens 14 Tage schriftliche Vorankündigung, nur zu regulären Geschäftszeiten, Schutz von Betriebsgeheimnissen und Kostenübernahme durch den Anbieter, sofern kein gravierender Verstoß vorliegt.",
  },
  {
    type: "data_lockin",
    title: "Daten-Lock-in & Entgelte für Datenherausgabe",
    tags: ["dsgvo-art-20", "bgb-307", "datenportabilitaet", "lock-in"],
    patterns: [
      /(?:nutzungslizenz|lizenz|nutzungsrecht)[^.!?;:\n]{0,80}?(?:welche\s+mit\s+beendigung|erlischt\s+mit\s+beendigung|mit\s+vertragsende\s+automatisch\s+erlischt|widerruflich)/i,
      /(?:mit\s+beendigung\s+dieses\s+(?:rahmen)?vertrags)[^.!?;:\n]{0,60}?(?:automatisch\s+erlischt|endet|widerrufen)/i,
      /(?:export|migration|herausgabe)[^.!?;:\n]{0,80}?(?:kundendaten|daten)[^.!?;:\n]{0,80}?(?:herausgabegebühr|gesonderte\s+gebühr|kostenpflichtig|gesonderte\s+vergütung)/i,
      /herausgabegebühr/i,
      /(?:herausgabe\s+der\s+daten|datenherausgabe|datenexport|rückgabe\s+der\s+daten)[^.!?;:\n]{0,100}?(?:kostenpflichtig|gesonderte\s+aufwandspauschale|gesonderte\s+vergütung|nach\s+aufwand\s+abgerechnet)/i,
      /(?:kein\s+anspruch\s+auf\s+herausgabe|anspruch\s+auf\s+bereitstellung)[^.!?;:\n]{0,80}?(?:in\s+einem\s+gängigen|in\s+einem\s+maschinenlesbaren|standardformat)[^.!?;:\n]{0,40}?(?:ausgeschlossen|besteht\s+nicht)/i,
      // Nur Pauschalen mit Datenbezug: "Bearbeitungspauschale" allein ist eine Mahn- oder
      // Rücksendegebühr und kein Daten-Lock-in. Erst der Bezug auf Datenbestand, Datenträger
      // oder Export/Herausgabe macht daraus ein Herausgabeentgelt (Art. 20 DSGVO, § 307 BGB).
      /(?:daten|datenbestand|datenbestände|datenbestands|bestand|bestände|datenträger|export|herausgabe|überlassung|übertragung|rückgabe|sicherung)[^;:\n]{0,120}?pauschale/i,
      /(?:herausgabe|rückgabe|überlassung|übertragung|export)[^;:\n]{0,80}?(?:daten|unterlagen|bestände)[^;:\n]{0,60}?(?:pauschale|gebühr|entgelt|kostenpflichtig|gesonderte\s+vergütung)/i,
      // Red Team: Euphemismus-/Synonym-/Wortzahl-/Nominal-Evasion
      /gegen\s+erstattung\s+des[^.!?;:\n]{0,80}?aufwands/i,
      /(?:daten|bestandsdaten)[^.!?;:\n]{0,140}?gegen\s+erstattung/i,
      /r(?:ü|ue)ckf(?:ü|ue)hrung[^.!?;:\n]{0,100}?(?:bestands)?daten[^.!?;:\n]{0,100}?nach\s+gesondertem\s+angebot/i,
      /maschinenlesbarer\s+export[^.!?;:\n]{0,80}?nicht\s+bestandteil/i,
      /fr(?:ü|ue)hestens\s+(?:neunzig|\d+)\s+kalendertage[^.!?;:\n]{0,120}?(?:k(?:ü|ue)ndigung|wirksamwerden)/i,
      /mit\s+(?:f(?:ü|ue)nf|\d+)\s*(?:prozent|%)\s+des\s+letzten\s+jahresvertragswerts\s+verg(?:ü|ue)tet/i,
      /(?:offenen?\s+standardformat|standardformat)[^.!?;:\n]{0,80}?nicht\s+vorgesehen/i,
      /herstellereigenen\s+bin(?:ä|ae)rformat/i,
    ],
    severity: "hoch",
    why:
      "Daten-Lock-in & Herausgabesperre (Art. 20 DSGVO, § 307 BGB): Der Anbieter erschwert den Wechsel zu anderen Dienstleistern, indem die Herausgabe der eigenen Unternehmensdaten kostenpflichtig gemacht oder in proprietären Formaten verweigert wird.",
    suggestion:
      "Kostenfreie Datenherausgabe in einem gängigen, offenen und maschinenlesbaren Format (z. B. CSV, JSON, SQL) innerhalb von 14 Tagen nach Vertragsende vereinbaren.",
  },
  {
    type: "third_party_indemnity",
    title: "Verschuldensunabhängige Drittfreistellung",
    tags: ["bgb-307", "freistellung", "haftungsverlagerung", "drittansprueche"],
    patterns: [
      /(?:verpflichtet\s+sich[^.!?;:\n]{0,60}?)?(?:freizustellen|freistellen)[^.!?;:\n]{0,80}?(?:von\s+sämtlichen\s+ansprüchen|von\s+allen\s+ansprüchen|ansprüchen[^.!?;:\n]{0,40}?dritter|bußgeldern)/i,
      /(?:von\s+sämtlichen\s+|von\s+allen\s+)?(?:ansprüchen|forderungen|schäden|bußgeldern)[^.!?;:\n]{0,60}?(?:dritter|aufsichtsbehörden)[^.!?;:\n]{0,60}?(?:vollumfänglich\s+)?freizustellen/i,
      /stellt[^.!?;:\n]{0,100}?von\s+(?:sämtlichen\s+|allen\s+)?ansprüchen\s+dritter\s+frei[^.!?;:\n]{0,100}?(?:unabhängig\s+von|ohne\s+rüchsicht\s+auf|ohne\s+eigenes\s+verschulden)/i,
      /(?:unabhängig\s+von\s+(?:einem\s+)?verschulden|verschuldensunabhängig)[^.!?;:\n]{0,80}?(?:freizustellen|stellt[^.!?;:\n]{0,40}?frei)/i,
      /freistellung\s+(?:im\s+innenverhältnis\s+)?gilt\s+verschuldensunabhängig/i,
      /stellt[^.!?;:\n]{0,100}?(?:von\s+sämtlichen\s+|allen\s+)?ansprüchen\s+dritter[^.!?;:\n]{0,80}?(?:verschuldensunabhängig\s+frei|ohne\s+eigenes\s+verschulden\s+frei)/i,
      /(?:schad-?\s*und\s*)?klaglos\s+zu\s+stellen|klaglos\s+halten/i,
      /(?:stellt|stellen|hat)[^.!?;:\n]{0,80}?(?:klaglos\s+zu\s+stellen|klaglos\s+halten)/i,
      /(?:auf\s+erstes\s+anfordern\s+(?:hin\s+)?freizustellen|stellt[^.!?;:\n]{0,80}?auf\s+erstes\s+anfordern\s+(?:hin\s+)?frei|auf\s+erstes\s+anfordern\s+hin\s+frei)/i,
      /stellt[^.!?;:\n]{0,100}?von\s+(?:sämtlichen\s+|allen\s+)?ansprüchen\s+dritter[^.!?;:\n]{0,250}?(?:unabhängig\s+davon,\s*ob|verschuldensunabhängig|auf\s+erstes\s+anfordern)/i,
      /(?:unabhängig\s+davon,\s*ob[^.!?;:\n]{0,60}?verschulden\s+trifft)[^.!?;:\n]{0,80}?(?:freizustellen|stellt[^.!?;:\n]{0,40}?frei)/i,
      /(?:indemnify|hold\s+harmless|defend,\s+indemnify)[^.!?;:\n]{0,100}?(?:claims|damages|liabilities|losses|provider)/i,
      /(?:defend\s+and\s+hold\s+harmless|shall\s+defend|shall\s+indemnify)/i,
      // "schadlos halten“/"stellt ... frei" in beliebiger Wortstellung (acc-euphemism-27, acc-synonym-28):
      /(?:auftraggeber|kunde|besteller)[^.!?;:\n]{0,60}?hält[^.!?;:\n]{0,100}?(?:anbieter|auftragnehmer|dienstleister|verk(?:ä|ae)ufer)[^.!?;:\n]{0,80}?schadlos/i,
      /(?:kunde|auftraggeber|besteller)[^.!?;:\n]{0,60}?stellt[^.!?;:\n]{0,100}?(?:den\s+)?(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,100}?frei/i,
      /(?:kunde|auftraggeber|besteller)[^.!?;:\n]{0,60}?(?:schadlos\s+zu\s+halten|von\s+anspr(?:ü|ue)chen\s+dritter\s+freizustellen)/i,
    ],
    severity(s) {
      // Richtungs-Guard: Freistellung zugunsten des Kunden ist kein Kundenrisiko.
      if (
        /(?:anbieter|auftragnehmer|dienstleister|verk(?:ä|ae)ufer)[^.!?;:\n]{0,60}?hält[^.!?;:\n]{0,80}?(?:den\s+)?(?:kunden|auftraggeber|besteller)[^.!?;:\n]{0,40}?schadlos/i.test(s) ||
        /(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,60}?stellt[^.!?;:\n]{0,80}?(?:den\s+)?(?:kunden|auftraggeber|besteller)[^.!?;:\n]{0,80}?frei/i.test(s)
      ) {
        return null;
      }
      return "hoch";
    },
    why:
      "Unzulässige verschuldensunabhängige Drittfreistellung (§ 307 BGB): Der Kunde wird verpflichtet, den Anbieter von Ansprüchen Dritter freizustellen, selbst wenn ihn überhaupt kein eigenes Verschulden trifft. Dies verlagert das unternehmerische Rechtsrisiko einseitig auf den Kunden.",
    suggestion:
      "Freistellungsverpflichtungen zwingend auf Fälle beschränken, in denen der Kunde den Anspruch durch eine nachweisbare schuldhafte Pflichtverletzung selbst verursacht hat.",
  },
  {
    type: "asymmetric_termination",
    title(ctx) {
      if (ctx && /übertrag|uebertrag|dritte[n]?|vertragsübernahme/i.test(ctx.sentence)) {
        return "Vertragspartnerwechsel / Vertragsübernahme ohne Zustimmung (§ 309 Nr. 10 BGB)";
      }
      return "Ausschluss des Rechts zur außerordentlichen Kündigung";
    },
    titlePlainGerman(ctx) {
      if (ctx && /übertrag|uebertrag|dritte[n]?|vertragsübernahme/i.test(ctx.sentence)) {
        return "Vertragspartnerwechsel / Vertragsübernahme ohne Zustimmung (§ 309 Nr. 10 BGB)";
      }
      return "Ausschluss des Rechts zur außerordentlichen Kündigung";
    },
    tags: ["bgb-314", "bgb-309-10", "bgb-307", "kuendigung", "knebelung", "vertragsuebernahme"],
    patterns: [
      /(?:ordentliche\s+kündigung)[^.!?;:\n]{0,80}?(?:durch\s+den\s+(?:kunden|auftraggeber)\s+)?ist\s+ausgeschlossen/i,
      /(?:kontrollwechsel|change\s+of\s+control)[^;:\n]{0,100}?(?:außerordentlich\s+zu\s+kündigen|mit\s+sofortiger\s+wirkung|sicherheitsleistung)/i,
      /(?:ohne\s+zustimmung\s+des\s+kunden)[^;:\n]{0,80}?(?:auf\s+verbundene\s+unternehmen|auf\s+einen\s+dritten|zu\s+übertragen|übertragbar)/i,
      /(?:rechte\s+und\s+pflichten)[^;:\n]{0,80}?(?:ohne\s+zustimmung)[^;:\n]{0,80}?(?:auf\s+einen\s+dritten|auf\s+dritte|zu\s+übertragen)/i,
      /(?:rechte\s+und\s+pflichten)[^;:\n]{0,120}?(?:auf\s+(?:einen\s+dritten|dritte|dritte\s+personen)\s+(?:zu\s+)?übertragen)[^;:\n]{0,80}?(?:ohne\s+(?:dass\s+es\s+der\s+)?zustimmung|ohne\s+zustimmung)/i,
      /(?:außerordentliche\s+kündigung|kündigung\s+aus\s+wichtigem\s+grund)[^.!?;:\n]{0,120}?(?:ausgeschlossen|abbedungen|ist\s+nicht\s+zulässig|beschränkt|eingeschränkt)/i,
      /(?:recht\s+zur\s+außerordentlichen\s+kündigung)[^.!?;:\n]{0,120}?(?:beschränkt|eingeschränkt)/i,
      /recht\s+zur\s+fristlosen\s+kündigung[^.!?;:\n]{0,60}?(?:ausgeschlossen|abbedungen|verzichtet)/i,
      /verzichtet\s+(?:unwiderruflich\s+)?auf\s+(?:jedes\s+recht\s+zur|das\s+recht\s+zur)\s+außerordentlichen\s+kündigung/i,
      // Verneintes Sonder-/Außerordentliches Kündigungsrecht (§ 314 BGB):
      /(?:steht|besteht)\s+(?:dem\s+kunden\s+|ihm\s+)?kein\s+(?:außerordentliches\s+)?(?:sonder)?kündigungsrecht\s+zu/i,
      /kein\s+außerordentliches\s+sonderkündigungsrecht/i,
      /kein\s+sonderkündigungsrecht\s+(?:wegen|bei|im\s+falle)[^.!?;:\n]{0,60}?(?:zu|besteht)/i,
      /(?:loslösung|vorzeitige\s+beendigung|beendigung)[^.!?;:\n]{0,80}?(?:vor\s+ablauf|vorzeitig)[^.!?;:\n]{0,80}?(?:besteht\s+nur|nur\s+wenn|nur\s+zulässig)/i,
      // "Lösung des Vertragsverhältnisses" statt "Kündigung" (bind-euphemism-13):
      /recht\s+zur\s+l(?:ö|oe)sung\s+des\s+vertragsverh(?:ä|ae)ltnisses[^.!?;:\n]{0,80}?besteht\s+nicht/i,
      /(?:k(?:ö|oe)nnen|kann)\s+den\s+vertrag\s+ausschlie(?:ß|ss)lich\s+einvernehmlich\s+beenden/i,
      // Passiv: "Auf das Recht zur vorzeitigen Beendigung ... wird ... verzichtet"; § 314 BGB ausgeschlossen (bind-synonym-14):
      /(?:verzicht\w*|entsag\w*)[^.!?;:\n]{0,100}?recht\s+zur\s+(?:vorzeitigen\s+beendigung|au(?:ß|ss)erordentlichen\s+k(?:ü|ue)ndigung)/i,
      /auf\s+das\s+recht\s+zur\s+vorzeitigen\s+beendigung[^.!?;:\n]{0,80}?(?:verzichtet|entsagt)/i,
      /§\s*314\s*bgb[^.!?;:\n]{0,60}?(?:findet\s+keine\s+anwendung|ist\s+nicht\s+anwendbar|ausgeschlossen|gilt\s+nicht)/i,
      /vertragsaufl(?:ö|oe)sung[^.!?;:\n]{0,60}?ist\s+ausgeschlossen/i,
      /fristlose\s+vertragsaufl(?:ö|oe)sung[^.!?;:\n]{0,60}?ausgeschlossen/i,
      /m(?:ä|ae)ngel\s+berechtigen\s+den\s+kunden\s+ausschlie(?:ß|ss)lich/i,
      // Umgekehrte Wortstellung: "kann den Vertrag nicht aus wichtigem Grund kündigen" (bind-split-sentence-16):
      /kann\s+den\s+vertrag\s+nicht\s+aus\s+wichtigem\s+grund\s+k(?:ü|ue)ndigen/i,
      /(?:beendigung|k(?:ü|ue)ndigung)[^.!?;:\n]{0,100}?ausschlie(?:ß|ss)lich\s+nach\s+ablauf\s+der\s+(?:fest)?laufzeit/i,
      // Englisch (bind-english-17):
      /(?:neither\s+party|no\s+party)[^.!?;:\n]{0,100}?(?:entitled|right|may|shall)[^.!?;:\n]{0,60}?terminate[^.!?;:\n]{0,60}?(?:for\s+cause|for\s+breach|for\s+material)/i,
      /termination\s+shall\s+require\s+the\s+mutual\s+(?:written\s+)?consent/i,
    ],
    severity: "hoch",
    why(ctx) {
      if (ctx && /übertrag|uebertrag|dritte[n]?|vertragsübernahme/i.test(ctx.sentence)) {
        return "Unwirksame Vertragsübernahme (§ 309 Nr. 10 BGB, § 307 BGB): Eine Klausel, nach der der Verwender alle Rechte und Pflichten ohne Zustimmung des Kunden auf einen Dritten übertragen darf, ist unzulässig. Dem Kunden darf kein beliebiger Dritter als Vertragspartner aufgedrängt werden, ohne dass der Dritte konkret benannt ist oder dem Kunden ein fristloses Lösungsrecht zusteht.";
      }
      return "Unwirksamer Ausschluss der fristlosen Kündigung (§ 314 Abs. 1, 3 BGB, § 307 BGB): Das gesetzliche Recht zur außerordentlichen Kündigung aus wichtigem Grund ist zwingendes Recht und kann formularmäßig weder ausgeschlossen noch unverhältnismäßig erschwert werden.";
    },
    suggestion(ctx) {
      if (ctx && /übertrag|uebertrag|dritte[n]?|vertragsübernahme/i.test(ctx.sentence)) {
        return "Klausel streichen oder anpassen: Vertragsübernahme nur mit vorheriger ausdrücklicher Zustimmung des Kunden ODER automatisches Sonderkündigungsrecht für den Kunden bei Übertragung vorsehen.";
      }
      return "Klausel streichen. Das Recht beider Vertragsparteien zur fristlosen Kündigung aus wichtigem Grund (§ 314 BGB) muss uneingeschränkt bestehen bleiben.";
    },
  },
  // 21. Assignment Restriction (§ 354a HGB, § 307 BGB)
  {
    type: "assignment_restriction",
    title: "Abtretungsverbot von Geldforderungen",
    tags: ["hgb-354a", "bgb-399", "bgb-307", "abtretungsverbot", "factoring"],
    patterns: [
      /(?:nicht\s+berechtigt|kein\s+recht)[^;:\n]{0,140}?(?:rechte\s+und\s+pflichten|ansprüche)[^;:\n]{0,160}?(?:abzutreten|zu\s+übertragen)/i,
      /(?:rechte\s+und\s+pflichten)[^;:\n]{0,80}?(?:dürfen|können)\s+nicht\s+(?:abgetreten|übertragen)/i,
      /abtretung\s+(?:von\s+forderungen|von\s+ansprüchen|sämtlicher\s+forderungen|seiner\s+ansprüche|von\s+rechten)[^.!?;:\n]{0,80}?(?:ausgeschlossen|bedarf|unwirksam|unzulässig|untersagt|nicht\s+gestattet)/i,
      /(?:forderungen|ansprüche)[^.!?;:\n]{0,60}?(?:dürfen|können)\s+nicht\s+abgetreten\s+werden/i,
      /abtretungsverbot/i,
      /abtretung\s+von\s+(?:geld)?forderungen[^.!?;:\n]{0,80}?(?:ohne\s+vorherige\s+schriftliche\s+zustimmung|ausgeschlossen)/i,
      /(?:ohne\s+vorherige\s+schriftliche\s+zustimmung|ohne\s+zustimmung)[^.!?;:\n]{0,60}?(?:nicht\s+abgetreten|nicht\s+übertragen|unwirksam)/i,
      // Red Team: Euphemismus-/Synonym-/English-Evasion
      /anspr(?:ü|ue)che[^.!?;:\n]{0,60}?sind\s+nicht\s+(?:abtretbar|übertragbar)/i,
      /nicht\s+(?:abtretbar|übertragbar)[^.!?;:\n]{0,80}?(?:abtretung|factoring)/i,
      /(?:abtretung\s+an\s+finanzierungsinstitute|im\s+wege\s+des\s+factorings)/i,
      /zession[^.!?;:\n]{0,100}?bedarf\s+der\s+vorherigen\s+zustimmung/i,
      /zession\s+der\s+verg(?:ü|ue)tungsanspr(?:ü|ue)che/i,
      /may\s+not\s+assign\s+or\s+transfer[^.!?;:\n]{0,140}?(?:right\s+to\s+payment|rights\s+hereunder)/i,
      /without\s+(?:provider|supplier|vendor)['\u2019]?s?\s+prior\s+written\s+consent/i,
    ],
    severity(s) {
      // § 399 BGB: höchstpersönliche Ansprüche sind ohnehin nicht abtretbar.
      if (/höchstpersönlich/i.test(s)) return null;
      if (
        /ausgeschlossen|unwirksam|unzulässig|untersagt|ohne\s+(?:vorherige\s+)?(?:schriftliche\s+)?zustimmung|nicht\s+(?:abtretbar|übertragbar)|factoring|right\s+to\s+payment|prior\s+written\s+consent/i.test(
          s
        )
      )
        return "hoch";
      return "mittel";
    },
    why: "Abtretungsverbot von Geldforderungen (§ 354a HGB, § 307 BGB): Ein formularmäßiges Verbot, Geldforderungen aus beiderseitigen Handelsgeschäften abzutreten, schränkt die Liquidität ein und behindert Factoring oder Vorfinanzierung.",
    suggestion: "Abtretung von Geldforderungen ausdrücklich gestatten (§ 354a HGB). Klausel auf höchstens höchstpersönliche Leistungsansprüche beschränken.",
  },

  // 22. Force Majeure One-Sided (§ 307 BGB)
  {
    type: "force_majeure_asymmetry",
    title: "Einseitige Klausel über höhere Gewalt (Force Majeure)",
    tags: ["bgb-307", "force-majeure", "hoehere-gewalt", "asymmetrie"],
    patterns: [
      /(?:höhere\s+gewalt|höherer\s+gewalt|force\s+majeure|betriebsstörung\w*)[^.!?;:\n]{0,120}?(?:befreit|entbindet|suspendiert)[^.!?;:\n]{0,80}?(?:den\s+anbieter|leistungspflicht)/i,
      /(?:höhere\s+gewalt|höherer\s+gewalt|force\s+majeure)[^.!?;:\n]{0,140}?(?:zahlungspflicht\w*[^.!?;:\n]{0,60}?(?:bleibt\s+unberührt|fortbesteht)|nicht\s+die\s+zahlungspflicht|während\s+die\s+vergütung)/i,
      /(?:ereignisse\s+höherer\s+gewalt|höhere\s+gewalt)[^.!?;:\n]{0,80}?(?:kein\s+recht\s+zur\s+kündigung|kündigungsrecht\s+des\s+kunden\s+ausgeschlossen)/i,
      // Subjektstellung "der Anbieter ... befreit" (bind-split-sentence-37, bind-cross-reference-41):
      /(?:h(?:ö|oe)here(?:r)?\s+gewalt(?:f(?:ä|ae)lle)?|force\s+majeure)[^.!?;:\n]{0,140}?(?:der\s+anbieter|der\s+auftragnehmer|der\s+dienstleister)[^.!?;:\n]{0,80}?(?:von\s+(?:seiner|der)\s+leistungspflicht\s+|von\s+der\s+leistung\s+)?(?:befreit|entbunden|entbindet|suspendiert)/i,
      /(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,80}?(?:entbunden|befreit)[^.!?;:\n]{0,120}?ohne\s+dass\s+sich\s+die\s+verg(?:ü|ue)tungspflicht\s+des\s+kunden/i,
      // "Zahlungsverpflichtungen ... unverändert zu erfüllen" (bind-euphemism-39):
      /zahlungsverpflichtungen\s+des\s+kunden[^.!?;:\n]{0,80}?(?:unver(?:ä|ae)ndert|weiterhin|vollumf(?:ä|ae)nglich)[^.!?;:\n]{0,60}?(?:zu\s+erf(?:ü|ue)llen|bleiben|bestehen)/i,
      /verg(?:ü|ue)tungspflicht\w*\s+des\s+kunden[^.!?;:\n]{0,80}?(?:bleiben\s+unber(?:ü|ue)hrt|bestehen\s+unver(?:ä|ae)ndert|entfallen\s+nicht)/i,
      /recht\s+zur\s+(?:vertragsaufl(?:ö|oe)sung|k(?:ü|ue)ndigung)[^.!?;:\n]{0,80}?besteht[^.!?;:\n]{0,40}?(?:f(?:ü|ue)r\s+den\s+kunden\s+)?nicht/i,
      // Englisch (bind-english-40):
      /force\s+majeure[^.!?;:\n]{0,140}?excuse[^.!?;:\n]{0,100}?(?:provider|supplier|vendor)[^.!?;:\n]{0,60}?(?:performance|obligations)/i,
      /(?:customer|client)\s+remains[^.!?;:\n]{0,100}?(?:obligated|liable|responsible)[^.!?;:\n]{0,60}?(?:pay|payment)/i,
      /(?:provider|supplier|vendor)\s+(?:is\s+|shall\s+be\s+)?(?:excused|relieved|released)[^.!?;:\n]{0,60}?from\s+performance/i,
    ],
    severity(s) {
      // Symmetrische Ausgestaltung (beide Parteien befreit) ist keinseitig benachteiligend.
      if (
        /beide\s+parteien[^.!?;:\n]{0,80}?befreit/i.test(s) ||
        /(?:auch|ebenfalls|gleicherma(?:ß|ss)en)[^.!?;:\n]{0,60}?(?:kunde|auftraggeber)[^.!?;:\n]{0,60}?befreit/i.test(s)
      ) {
        return null;
      }
      return "hoch";
    },
    why: "Einseitige Force-Majeure-Klausel (§ 307 BGB): Der Anbieter entlastet sich bei Betriebsstörungen von seiner Leistungspflicht, verpflichtet den Kunden aber zur uneingeschränkten Fortzahlung der Vergütung oder schließt dessen Kündigungsrecht aus.",
    suggestion: "Beidseitige Suspendierung der Hauptleistungspflichten bei höherer Gewalt sowie Kündigungsrecht beider Parteien nach spätestens 30 bis 60 Tagen ununterbrochener Störung.",
  },

  // 23. Customer Staff Non-Solicitation (§ 307 BGB, § 75f HGB)
  {
    type: "non_solicitation",
    title: "Unangemessenes Abwerbeverbot & Einstellungsverbot",
    tags: ["bgb-307", "hgb-75f", "abwerbeverbot", "mitarbeiter", "vertragsstrafe"],
    patterns: [
      /(?:abwerbung|abzuwerben|abwerben)[^.!?;:\n]{0,80}?(?:von\s+mitarbeitern|von\s+angestellten|untersagt|verboten)/i,
      /(?:mitarbeiter|angestellte|personal)[^.!?;:\n]{0,80}?(?:abzuwerben|einzustellen|zu\s+beschäftigen)/i,
      /(?:untersagt|verboten)[^.!?;:\n]{0,60}?(?:mitarbeiter|personal)[^.!?;:\n]{0,60}?(?:abzuwerben|einzustellen)/i,
      /(?:einstellungsverbot|beschäftigungsverbot)[^.!?;:\n]{0,60}?(?:für\s+mitarbeiter|hinsichtlich\s+mitarbeitern)/i,
      /abwerbeverbot/i,
      /(?:keine\s+)?(?:fachkräfte|mitarbeiter|personal|arbeitnehmer|angestellte)[^;:\n]{0,60}?(?:abwerben|abzuwerben|einzustellen|zu\s+beschäftigen)/i,
      /(?:personen|mitarbeiter)[^;:\n]{0,80}?tätig\s+(?:waren|sind)[^;:\n]{0,80}?(?:ziel|absicht|abzuwerben|abwerben|einzustellen)/i,
      /(?:ehemalige[nr]?\s+|fr(?:ü|ue)here[nr]?\s+)(?:mitarbeiter|berater|besch(?:ä|ae)ftigte|arbeitnehmer)[^.!?;:\n]{0,100}?(?:einstellen|einzustellen|zu\s+besch(?:ä|ae)ftigen|abzuwerben|zu\s+(?:ü|ue)bernehmen)/i,
      // Red Team: Euphemismus-/Synonym-/English-Evasion des Abwerbeverbots
      /verzichtet[^.!?;:\n]{0,120}?abzuwerben/i,
      /besch(?:ä|ae)ftigte[^.!?;:\n]{0,80}?(?:abzuwerben|anzusprechen|zu\s+(?:ü|ue)berf(?:ü|ue)hren)/i,
      // Nachvertragliches Beschaeftigungsverbot mit temporalem Anker (Welle D):
      /(?:kunde|auftraggeber|besteller)[^.!?;:\n]{0,60}?verpflichtet\s+sich[^.!?;:\n]{0,140}?(?:f(?:ü|ue)r\s+(?:\d{1,3}|[a-zäöüß]{3,20})\s+monate?\s+nach|nach\s+(?:vertragsende|beendigung|vertragsablauf|auslaufen))[^.!?;:\n]{0,120}?(?:keine\s+|nicht\s+)?(?:besch(?:ä|ae)ftigten|mitarbeiter|personal|arbeitnehmer|angestellte)[^.!?;:\n]{0,80}?(?:zu\s+besch(?:ä|ae)ftigen|einzustellen|abzuwerben|anzusprechen)/i,
      /(?:unterl(?:ä|ae)sst|verpflichtet\s+sich)[^.!?;:\n]{0,120}?personalgewinnung/i,
      /personalgewinnung[^.!?;:\n]{0,100}?(?:reihen|direktansprache|soziale[n]?\s+netzwerke)/i,
      /jegliche\s+personalgewinnung/i,
      /ausgleichszahlung[^.!?;:\n]{0,80}?(?:monatsgeh(?:ä|ae)lter|f(?:ä|ae)llig|zuwiderhandelns)/i,
      /shall\s+not\s+(?:solicit|hire|engage|poach)[^.!?;:\n]{0,140}?(?:personnel|employees|staff)/i,
      /solicit[^.!?;:\n]{0,80}?provider['\u2019]?s?\s+(?:personnel|employees|staff)/i,
    ],
    severity(s) {
      // GeschGehG-konforme Abwerbeverbote (nur unlautere Abwerbung unter Ausnutzung
      // vertraulicher Informationen/Geschaeftsgeheimnisse) sind zulaessig (Welle D).
      if (
        /unter\s+Verwendung\s+vertraulicher\s+Informationen|unter\s+Versto(?:ß|ss)\s+gegen\s+die\s+(?:Geheimhaltung|Vertraulichkeit|Verschwiegenheit)|unter\s+Ausnutzung\s+(?:von\s+)?vertraulicher|unter\s+Ausnutzung\s+von\s+Gesch(?:ä|ae)ftsgeheimnissen/i.test(
          s
        )
      )
        return null;
      if (
        /vertragsstrafe|pönale|schadensersatz|ausgleichszahlung|zuwiderhandelns/i.test(s) ||
        /nach\s+beendigung|nachvertraglich|nach\s+(?:vertragsende|vertragsablauf|auslaufen)|f(?:ü|ue)r\s+(?:\d{1,3}|[a-zäöüß]{3,20})\s+monate?\s+nach/i.test(s)
      )
        return "hoch";
      return "mittel";
    },
    why: "Pauschales Abwerbe- und Einstellungsverbot (§ 307 BGB, § 75f HGB analog): Vereinbarungen, die dem Kunden verbieten, wechselwillige Mitarbeiter des Vertragspartners einzustellen, verletzen die Berufsfreiheit der Beschäftigten und sind bei überlangen Fristen unwirksam.",
    suggestion: "Einstellungsverbote streichen; allenfalls gezieltes, aktives und unlauteres Abwerben auf maximal 6 Monate nach Projektabschluss beschränken.",
  },

  // 24. Unilateral Right to Terminate Convenience (§ 307 BGB)
  {
    type: "unilateral_convenience_termination",
    title: "Einseitiges ordentliches Kündigungsrecht",
    tags: ["bgb-307", "kuendigung", "asymmetrie", "bindung"],
    patterns: [
      /(?:ordentliche\s+kündigung|jederzeitige\s+kündigung)[^.!?;:\n]{0,80}?(?:steht\s+nur\s+dem\s+anbieter\s+zu|ist\s+dem\s+kunden\s+verwehrt|nur\s+durch\s+den\s+anbieter)/i,
      /(?:nur\s+der\s+anbieter|einzig\s+der\s+anbieter)\s+ist\s+berechtigt,[^.!?;:\n]{0,60}?ordentlich\s+zu\s+kündigen/i,
      /(?:anbieter|anbieterin|auftragnehmer|dienstleister|lizenzgeber)\s+(?:kann|ist\s+berechtigt)[^.!?;:\n]{0,60}?(?:jederzeit\s+)?mit\s+einer\s+frist\s+von[^.!?;:\n]{0,40}?(?:ohne\s+angabe\s+von\s+gründen|ohne\s+grund)[^.!?;:\n]{0,40}?kündigen/i,
      /ordentliche\s+kündigungsrecht\s+des\s+kunden\s+(?:ist\s+)?ausgeschlossen/i,
      // "kann die Vereinbarung ... beenden" ohne Grundangabe (bind-euphemism-19):
      /(?:der\s+)?(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,50}?kann[^.!?;:\n]{0,120}?(?:unter\s+einhaltung\s+einer\s+frist|mit\s+einer\s+frist)[^.!?;:\n]{0,100}?beenden/i,
      // Starre Bindung des Kunden an die Mindestlaufzeit (bind-euphemism-19):
      /(?:kunde|auftraggeber)[^.!?;:\n]{0,60}?(?:ist\s+)?f(?:ü|ue)r\s+die\s+dauer[^.!?;:\n]{0,60}?gebunden/i,
      // Cross-Reference/Nominalstil (bind-cross-reference-20):
      /(?:ordentliche\s+beendigung|ordentliches\s+k(?:ü|ue)ndigungsrecht)[^.!?;:\n]{0,80}?(?:durch\s+den\s+kunden)?[^.!?;:\n]{0,40}?(?:nicht\s+vorgesehen|ist\s+ausgeschlossen)/i,
      /beendigungsrecht\s+des\s+kunden[^.!?;:\n]{0,80}?nach\s+(?:ziffer|§|\d)/i,
    ],
    collect(sentences, _full, extra = null) {
      const whole = sentences.map(flattenSentence).join(" ");
      // Symmetrie-Guard: Kündigungsrechte beider Parteien bzw. auch des Kunden sprechen
      // gegen ein einseitiges Anbieter-Kündigungsrecht.
      const customerRight =
        /(?:beide\s+parteien|beidseitig|gegenseitig)[^.!?;:\n]{0,100}?k(?:ü|ue)ndigen/i.test(whole) ||
        /(?:auch\s+der\s+kunde|der\s+kunde\s+(?:kann|ist\s+berechtigt))[^.!?;:\n]{0,80}?k(?:ü|ue)ndigen/i.test(whole) ||
        /k(?:ü|ue)ndigungsrecht\s+(?:steht\s+auch\s+)?dem\s+kunden\s+zu/i.test(whole);
      // Starre Bindung des Kunden trotz symmetrischer Grundfrist: das einseitige
      // Zusatz-Kündigungsrecht des Anbieters bleibt dann unausgeglichen.
      const boundCustomer =
        /(?:kunde|auftraggeber|besteller)[^.!?;:\n]{0,60}?an\s+die[^.!?;:\n]{0,40}?laufzeit\s+gebunden/i.test(whole) ||
        /(?:kunde|auftraggeber|besteller)[^.!?;:\n]{0,60}?f(?:ü|ue)r\s+die\s+dauer[^.!?;:\n]{0,60}?gebunden/i.test(whole);
      if (customerRight && !boundCustomer && !/steht\s+nur\s+dem\s+anbieter|nur\s+der\s+anbieter|ist\s+dem\s+kunden\s+verwehrt/i.test(whole)) {
        return [];
      }
      return collectPerSentence(this, sentences, null, extra);
    },
    severity: "hoch",
    why: "Einseitiges ordentliches Kündigungsrecht (§ 307 BGB): Eine Klausel, nach der sich nur eine Partei jederzeit grundlos vom Vertrag lösen kann, während die andere an eine starre Laufzeit gebunden bleibt, benachteiligt den gebundenen Partner grob unangemessen.",
    suggestion: "Gleiche Kündigungsrechte und identische Kündigungsfristen für beide Parteien vereinbaren.",
  },

  // 25. Survival & Post-Contractual Restraints (§ 307 BGB)
  {
    type: "excessive_survival",
    title: "Exzessive fortwirkende Pflichten (Survival-Klausel)",
    tags: ["bgb-307", "survival", "fortwirkung", "nachvertraglich"],
    patterns: [
      /(?:pflichten|verpflichtungen|bindungen)[^.!?;:\n]{0,80}?(?:gelten\s+zeitlich\s+unbegrenzt\s+fort|bestehen\s+zeitlich\s+unbeschränkt\s+fort|wirken\s+zeitlich\s+unbegrenzt)/i,
      /(?:geheimhaltung|verschwiegenheit|treuepflicht)[^.!?;:\n]{0,80}?(?:gilt\s+zeitlich\s+unbeschränkt|ohne\s+zeitliche\s+begrenzung\s+fort)/i,
      /(?:nachvertragliche\s+pflichten|bestimmungen\s+dieses\s+vertrages)[^.!?;:\n]{0,80}?(?:überdauern\s+das\s+vertragsende\s+auf\s+unbestimmte\s+zeit|gelten\s+fortdauernd)/i,
      /(?:geheimhaltung\w*|verschwiegenheit\w*|treuepflicht\w*|pflichten)[^.!?;:\n]{0,100}?(?:gelten\s+(?:zeitlich\s+)?(?:unbegrenzt|unbefristet)|bestehen\s+(?:zeitlich\s+)?unbeschränkt|auf\s+unbestimmte\s+zeit|über\s+die\s+beendigung\s+hinaus)/i,
      /(?:geheimhaltung\w*|verschwiegenheit\w*|vertraulichkeit\w*|pflichten|verpflichtungen)[^.!?;:\n]{0,80}?(?:besteht\s+auf\s+dauer|bestehen\s+auf\s+dauer|gilt\s+auf\s+dauer|gelten\s+auf\s+dauer|end(?:et|en)\s+nicht\s+mit\s+der\s+beendigung)/i,
      /(?:endet\s+nicht\s+mit\s+der\s+beendigung)[^.!?;:\n]{0,60}?(?:sondern\s+besteht|sondern\s+gilt)/i,
      // "unbefristete Nachwirkung"; kein Endzeitpunkt (bind-euphemism-23):
      /unbefristete\s+nachwirkung/i,
      /(?:regelungen|pflichten|verpflichtungen|bestimmungen)[^.!?;:\n]{0,100}?entfalten[^.!?;:\n]{0,60}?nachwirkung/i,
      /endzeitpunkt[^.!?;:\n]{0,60}?wird\s+nicht\s+vereinbart/i,
      // "überdauert die Vertragsbeendigung ohne zeitliche Beschränkung" (bind-synonym-24):
      /überdauert\s+die\s+vertragsbeendigung\s+ohne\s+zeitliche\s+beschr(?:ä|ae)nkung/i,
      /(?:verschwiegenheit|geheimhaltung|vertraulichkeit)\w*[^.!?;:\n]{0,120}?überdauert[^.!?;:\n]{0,80}?ohne\s+zeitliche\s+beschr(?:ä|ae)nkung/i,
      /endet\s+erst\s+mit\s+der\s+aufnahme\s+der\s+information[^.!?;:\n]{0,60}?(?:in\s+das\s+)?(?:öffentliche\s+wissen|allgemeinheit)/i,
    ],
    severity(s) {
      if (/zeitlich\s+unbegrenzt|auf\s+unbestimmte\s+zeit|nachvertraglich|auf\s+dauer|unbefristet|ohne\s+zeitliche\s+beschr(?:ä|ae)nkung|nachwirkung|end(?:et|en)\s+nicht\s+mit\s+der\s+beendigung/i.test(s)) return "hoch";
      return "mittel";
    },
    why: "Exzessive fortwirkende Pflichten nach Vertragsende (§ 307 BGB): Pflichten und Beschränkungen, die unbegrenzt fortgelten, schaffen unkalkulierbare Dauerhaftungsrisiken nach Beendigung der Zusammenarbeit.",
    suggestion: "Nachvertragliche Geheimhaltungs- und Treuepflichten auf 2 bis 3 Jahre nach Vertragsende befristen; Ausnahmen für offenkundige Informationen aufnehmen.",
  },

  // 26. Price Indexation / Escalation (§ 307 BGB, PrKG)
  {
    type: "price_escalation",
    title: "Einseitige Preiserhöhungsautomatik & Indexfalle",
    tags: ["bgb-307", "prkg", "preiserhoehung", "preisgleitklausel"],
    patterns: [
      /um\s+bis\s+zu\s+(?:\d+|fünfzehn|zehn)\s*(?:%|prozent)/i,
      /(?:behält\s+sich[^.!?;:\n]{0,60}?vor|berechtigt)[^.!?;:\n]{0,140}?(?:entgelte|preise|verg(?:ü|ue)tung|geb(?:ü|ue)hr\w*)[^;:\n]{0,120}?(?:anzupassen|zu\s+erhöhen)/i,
      /(?:vergütung|entgelte|preise)[^.!?;:\n]{0,80}?(?:erhöhen\s+sich\s+automatisch|steigen\s+automatisch)[^.!?;:\n]{0,60}?(?:um\s+\d+\s*%|jährlich)/i,
      /preisanpassung[^.!?;:\n]{0,80}?(?:nach\s+billigem\s+ermessen|ohne\s+ankündigung|ohne\s+sonderkündigungsrecht)/i,
      /(?:kostensteigerungen|preissteigerungen)[^.!?;:\n]{0,80}?(?:können\s+vollumfänglich|werden\s+vollständig)[^.!?;:\n]{0,60}?(?:an\s+den\s+kunden\s+weitergegeben|weitergereicht)/i,
      /(?:preise|vergütung|entgelte)[^.!?;:\n]{0,120}?(?:nach\s+billigem\s+ermessen|einseitig\s+zu\s+erhöhen|anzupassen|erhöhen)/i,
      // Unbegrenzte Notfall-/Mehrstundensätze ohne Budgetobergrenze (Incident-Response-Retainer):
      /(?:betragsmäßige\s+)?(?:obergrenze|deckelung|budget\s*cap)[^;:\n]{0,120}?besteht\s+nicht/i,
      /(?:obergrenze|deckelung|budget\s*cap)[^;:\n]{0,120}?(?:wird\s+)?(?:ausdrücklich\s+)?ausgeschlossen/i,
      // Kein Kostendeckel ausdruecklich vereinbart (Welle D):
      /(?:obergrenze|deckelung|budget\s*cap)[^;:\n]{0,120}?wird\s+(?:ausdr(?:ü|ue)cklich\s+)?nicht\s+(?:vereinbart|vorgesehen|geregelt)/i,
      /(?:notfall|mehrstunden|sonderstunden)[^.!?;:\n]{0,140}?(?:faktor\s+\d|\d+\s*(?:%|prozent)\s*(?:aufschlag|zuschlag)?|wochenend-?\s*und\s*nachtzuschlag|zuschlag\s+von\s+\d+\s*%)/i,
      /(?:an\s+die|an\s+den|an\s+das|der\s+entwicklung)[^;:\n]{0,60}?(?:preissteigerung|preisindex|index(?:es)?|konsumentenpreise|kostenentwicklung|marktbedingungen|(?:energie|personal)?kosten)[^;:\n]{0,60}?(?:angepasst|anzupassen|anpassen|angelehnt)/i,
      /(?:preise|entgelte|vergütung|beiträge|prämien)[^;:\n]{0,80}?nach\s+billigem\s+ermessen[^;:\n]{0,80}?(?:an(?:zu)?passen|anzuheben|zu\s+erhöhen|erhöhen)/i,
      /nach\s+billigem\s+ermessen[^;:\n]{0,80}?(?:an(?:zu)?passen|anpassen|anheben|erhöhen)/i,
      /(?:stundens(?:ä|ae)tze|preise|entgelte|verg(?:ü|ue)tung)[^.!?;:\n]{0,80}?(?:j(?:ä|ae)hrlich\s+neu|jederzeit\s+neu|einseitig\s+neu|neu\s+festgesetzt)[^.!?;:\n]{0,60}?nach\s+billigem\s+ermessen/i,
      /(?:beitragsanpassung|prämienanpassung)[^;:\n]{0,100}?(?:kann\s+abgesehen\s+werden|nach\s+ermessen|vorbehalten|einseitig)/i,
      /an\s+sich\s+verändernde\s+marktbedingungen/i,
      /durch\s+einseitige\s+leistungsbestimmung[^;:\n]{0,80}?anzupassen/i,
      /(?:fees?)\s+(?:may\s+be\s+changed|may\s+change)\s+at\s+any\s+time|may\s+(?:change|increase|modify|adjust)\s+(?:the\s+)?(?:fees|prices|rates|charges)\s+(?:at\s+any\s+time|upon\s+notice)/i,
      // Red Team: ausgeschriebene Jahreserhöhung ("Grundpreis erhöht sich ... um acht Prozent")
      /(?:preis|grundpreis|entgelt\w*|geb(?:ü|ue)hr\w*|verg(?:ü|ue)tung)[^.!?;:\n]{0,80}?(?:erh(?:ö|oe)ht\s+sich|erh(?:ö|oe)hen\s+sich|steigt|steigen)(?!\s+nicht)[^.!?;:\n]{0,140}?um\s+(?:bis\s+zu\s+)?(?:\d{1,2}(?:[.,]\d+)?|acht|zehn|elf|zw(?:ö|oe)lf|f(?:ü|ue)nfzehn|zwanzig)\s*(?:%|prozent)/i,
      // Eupehmismus: marktgerechte Fortschreibung der Grundgebühr o. Ä.
      /(?:fortschreibung|weiterentwicklung|fortschreiben)[^.!?;:\n]{0,60}?(?:grundgeb(?:ü|ue)hr|entgelt|preis|verg(?:ü|ue)tung|beitrag)/i,
      // Red Team: englische Preisanpassungsautomatik ohne Kündigungsrecht
      /may\s+(?:adjust|increase|change|modify)\s+(?:the\s+|its\s+)?[\w\s]{0,50}?(?:fees|prices|rates|charges)\b/i,
      /(?:fees|prices|rates|charges)\s+(?:may|shall)\s+be\s+(?:adjusted|increased|changed)\s+annually/i,
      /without\s+(?:any\s+)?right\s+of\s+termination\s+for\s+the\s+customer/i,
      /adjust\s+the\s+subscription\s+fees\s+annually/i,
    ],
    severity(s) {
      // False-Positive-Schutz: ausdrückliches Kündigungsrecht des Kunden bei Preisanpassung.
      if (
        /(?:right\s+of\s+termination\s+for\s+the\s+customer|sonderk(?:ü|ue)ndigungsrecht\s+des\s+kunden|kunde\s+kann[^.!?;:\n]{0,80}?sonderk(?:ü|ue)ndigen|each\s+party\s+may\s+terminate)/i.test(s) &&
        !/without\s+(?:any\s+)?right\s+of\s+termination/i.test(s)
      )
        return null;
      if (/ohne\s+sonderkündigungsrecht|nach\s+billigem\s+ermessen|besteht\s+nicht|budget\s*cap|keine\s+obergrenze|preissteigerung|marktbedingungen|beitragsanpassung|einseitige\s+leistungsbestimmung|erh(?:ö|oe)ht\s+sich[^.!?;:\n]{0,100}?(?:prozent|%)|fortschreibung|entgeltbestandteile/i.test(s)) return "hoch";
      return "mittel";
    },
    collect(sentences, _full, extra = null) {
      const hits = collectPerSentence(this, sentences, null, extra);
      if (hits.length > 0) return hits;
      // Split-Sentence-/Cross-Reference-Evasion: die Indexkopplung, die fehlende
      // Kostensenkungsweitergabe und der Sonderkündigungsausschluss stehen in
      // verschiedenen Sätzen. § 307 BGB, Preisklauselgesetz.
      const whole = sentences.map(flattenSentence).join(" ");
      const indexClause =
        /(?:verbraucherpreisindex|preisindex|inflationsrate|statistische[sn]?\s+bundesamt|kostenentwicklung)/i.test(whole) &&
        /(?:anpassung|an(?:zu)?passen|angepasst|passt\s+[^.!?;:\n]{0,40}?an\b|erh(?:ö|oe)ht\s+sich)/i.test(whole);
      const aggravator =
        /(?:kostensenkung\w*[^.!?;:\n]{0,80}?nicht|nicht\s+ber(?:ü|ue)cksichtigt|kein\w*\s+sonderk(?:ü|ue)ndigungsrecht|sonderk(?:ü|ue)ndigungsrecht[^.!?;:\n]{0,60}?nicht|ohne\s+sonderk(?:ü|ue)ndigungsrecht)/i.test(
          whole
        );
      if (!indexClause || !aggravator) return hits;
      const idx = sentences.findIndex((s) =>
        /(?:verbraucherpreisindex|preisindex|inflationsrate|statistische[sn]?\s+bundesamt|kostenentwicklung)/i.test(flattenSentence(s))
      );
      return [{ i: idx >= 0 ? idx : 0, severity: "hoch" }];
    },
    why: "Preiserhöhungsautomatik ohne Kündigungsrecht (§ 307 BGB, Preisklauselgesetz): Klauseln, die automatische Preiserhöhungen vorschreiben, ohne dass der Kunde Kostensenkungen einfordern oder den Vertrag sonderkündigen kann, sind unwirksam.",
    suggestion: "Preiserhöhungen an transparente Indizes (z. B. Verbraucherpreisindex Destatis) koppeln, Begrenzung auf max. 3-5 % p.a. und Sonderkündigungsrecht bei jeder Steigerung vereinbaren.",
  },

  // 27. Severability with Discretion / Salvatorische Klausel (§ 306 BGB)
  {
    type: "salvatorische_klausel_falle",
    title: "Unwirksame geltungserhaltende Reduktion (Salvatorische Klausel)",
    tags: ["bgb-306", "bgb-307", "salvatorische-klausel", "ersetzung"],
    patterns: [
      /(?:anstelle\s+der\s+unwirksamen|an\s+die\s+stelle\s+der\s+unwirksamen)[^.!?;:\n]{0,80}?(?:gilt|tritt|treten)?[^.!?;:\n]{0,40}?(?:diejenige|eine\s+solche|eine\s+wirksame)/i,
      /(?:gilt|treten?)\s+(?:anstelle|an\s+die\s+stelle)[^.!?;:\n]{0,80}?(?:diejenige|eine\s+solche|eine\s+wirksame)/i,
      /(?:gilt\s+eine\s+solche\s+regelung\s+als\s+vereinbart|gilt\s+als\s+vereinbart)[^.!?;:\n]{0,120}?(?:wirtschaftlich\w*|zweck)[^.!?;:\n]{0,80}?am\s+nächsten/i,
      /(?:dem\s+wirtschaftlichen\s+zweck|wirtschaftlich\w*)[^.!?;:\n]{0,80}?am\s+nächsten\s+kommt/i,
      /sollte[n]?\s+(?:eine|einzelne)\s+bestimmung(?:en)?[^.!?;:\n]{0,100}?(?:unwirksam|nichtig)[^.!?;:\n]{0,120}?(?:so\s+gilt|nach\s+billigem\s+ermessen|wirtschaftlich)/i,
      /(?:unwirksame\s+bestimmung|nichtige\s+klausel)[^.!?;:\n]{0,80}?(?:durch\s+eine\s+solche\s+zu\s+ersetzen|automatisch\s+ersetzt)/i,
      /wirtschaftlich\s+am\s+nächsten\s+kommt[^.!?;:\n]{0,80}?(?:bestimmt\s+der\s+anbieter|nach\s+ermessen\s+des\s+anbieters)/i,
      /gerade\s+noch\s+zulässig/i,
      // Red Team: Euphemismus-/Synonym-/English-Evasion
      /an\s+die\s+stelle\s+(?:der\s+)?(?:nichtiger|unwirksamer|unwirksamen)\s+(?:klauseln|bestimmungen|regelungen|vereinbarungen)[^.!?;:\n]{0,100}?(?:tritt|gilt)[^.!?;:\n]{0,40}?(?:diejenige|eine\s+solche|eine\s+wirksame|den\s+parteien|die\s+den)/i,
      /(?:nichtig|unwirksam)[^.!?;:\n]{0,120}?wirtschaftlich\s+am\s+n(?:ä|ae)chsten\s+steht/i,
      /(?:gestaltung|regelung|bestimmung)[^.!?;:\n]{0,80}?den\s+parteien\s+wirtschaftlich\s+am\s+n(?:ä|ae)chsten\s+steht/i,
      /unwirksame\s+regelungen\s+werden\s+durch[^.!?;:\n]{0,80}?ersetzt/i,
      /ersatzregelung[^.!?;:\n]{0,120}?(?:nach\s+billigem\s+ermessen|nach\s+ermessen\s+des\s+anbieters)/i,
      /interessengerechte\s+ersatzregelung/i,
      /replace\s+it\s+with\s+a\s+valid\s+provision[^.!?;:\n]{0,140}?comes\s+closest/i,
      /comes\s+closest\s+to\s+the\s+commercial\s+intent/i,
      /(?:deemed\s+to\s+be\s+replaced\s+by|shall\s+be\s+replaced\s+by)[^.!?;:\n]{0,100}?(?:closest|most\s+appropriate)/i,
    ],
    severity: "hoch",
    why: "Verstoß gegen das Verbot der geltungserhaltenden Reduktion (§ 306 Abs. 2 BGB): In AGB ist eine Klausel, die vorsieht, dass unwirksame Bestimmungen automatisch durch eine andere wirksame Regelung ersetzt werden, unzulässig. An die Stelle unwirksamer Klauseln tritt zwingend das BGB.",
    suggestion: "Streichen. Gesetzliche Rechtsfolge nach § 306 Abs. 2 BGB (Geltung der gesetzlichen Vorschriften) greifen lassen.",
  },

  // 28. Retention of Title Group / Erweiterter Eigentumsvorbehalt (§ 449, § 307 BGB)
  {
    type: "extended_retention_of_title",
    title: "Übersichernder Kontokorrent-Eigentumsvorbehalt",
    tags: ["bgb-449", "bgb-307", "eigentumsvorbehalt", "kontokorrent"],
    patterns: [
      /(?:zurückbehaltungsrecht|eigentumsvorbehalt)[^.!?;:\n]{0,140}?(?:bereits\s+vollständig\s+bezahlt\w*|bezahlter\s+vorangegangener\s+meilensteine|bezahlten\s+leistungen)/i,
      /(?:gilt\s+auch\s+für\s+solche\s+liefergegenstände|erstreckt\s+sich\s+auch\s+auf)[^.!?;:\n]{0,80}?(?:bereits\s+vollständig\s+bezahlt\w*|bezahlte)/i,
      /(?:bleibt\s+eigentum|bleiben\s+im\s+eigentum|vorbehalten)[^.!?;:\n]{0,120}?(?:ausgleich|begleichung)[^.!?;:\n]{0,80}?(?:aller\s+forderungen|sämtlicher\s+verbindlichkeiten|gesamten\s+bestehenden\s+geschäftsbeziehung)/i,
      /(?:eigentumsvorbehalt|kontokorrentvorbehalt)[^.!?;:\n]{0,80}?(?:erstreckt\s+sich|gesamte\s+geschäftsbeziehung|alle\s+offenen\s+forderungen)/i,
      /(?:ausgleich|erfüllung)\s+sämtlicher\s+(?:verbindlichkeiten|forderungen)[^.!?;:\n]{0,60}?(?:aus\s+der\s+(?:gesamten\s+)?geschäfts)/i,
      // Red Team: Euphemismus-/Synonym-/Cross-Reference-/English-Evasion
      /bleibt\s+unser\s+eigentum[^.!?;:\n]{0,140}?(?:alle\s+verbindlichkeiten|s(?:ä|ae)mtliche\s+verbindlichkeiten)/i,
      /bis\s+der\s+kunde\s+alle\s+verbindlichkeiten[^.!?;:\n]{0,120}?beglichen\s+hat/i,
      /(?:ü|ue)bereignung[^.!?;:\n]{0,120}?aufschiebenden\s+bedingung[^.!?;:\n]{0,100}?(?:erf(?:ü|ue)llung|bezahlung|begleichung)/i,
      // Kontokorrentvorbehalt in Nominalstil: Eigentum bis zur Begleichung aller Verbindlichkeiten aus der Geschaeftsverbindung (Welle D):
      /(?:eigentum|eigentumsvorbehalt|vorbehalten|(?:ü|ue)bereignung)[^.!?;:\n]{0,140}?(?:begleichung|bezahlung|erf(?:ü|ue)llung|ausgleich)[^.!?;:\n]{0,40}?(?:aller|s(?:ä|ae)mtlicher)\s+verbindlichkeiten[^.!?;:\n]{0,60}?gesch(?:ä|ae)fts(?:verbindung|beziehung)/i,
      /vollst(?:ä|ae)ndigen\s+erf(?:ü|ue)llung\s+aller\s+anspr(?:ü|ue)che\s+aus\s+der\s+gesch(?:ä|ae)ftsverbindung/i,
      /s(?:ä|ae)mtliche\s+forderungen\s+aus\s+der\s+gesch(?:ä|ae)ftsbeziehung/i,
      /(?:s(?:ä|ae)mtliche\s+anspr(?:ü|ue)che|alle\s+anspr(?:ü|ue)che)[^.!?;:\n]{0,40}?aus\s+der\s+gesamten\s+gesch(?:ä|ae)ftsbeziehung[^.!?;:\n]{0,60}?ausgeglichen/i,
      /eigentum\s+an\s+den\s+liefergegenst(?:ä|ae)nden[^.!?;:\n]{0,120}?ziffer/i,
      /regelung\s+in\s+ziffer[^.!?;:\n]{0,100}?s(?:ä|ae)mtliche\s+forderungen/i,
      /title\s+to[^.!?;:\n]{0,100}?shall\s+remain\s+with\s+(?:provider|supplier|vendor)/i,
      /until\s+payment\s+in\s+full\s+of\s+all\s+amounts\s+owed/i,
      /all\s+amounts\s+owed\s+by\s+customer\s+under\s+any\s+agreement/i,
    ],
    severity: "hoch",
    why: "Übersichernder Kontokorrent-Eigentumsvorbehalt (§ 449, § 307 BGB): Waren oder Arbeitsergebnisse verbleiben im Eigentum des Anbieters, selbst wenn die konkrete Rechnung bezahlt wurde, solange noch andere Forderungen aus der Geschäftsbeziehung offen sind.",
    suggestion: "Eigentumsvorbehalt strikt auf den konkreten Einzelauftrag und dessen Bezahlung beschränken (einfacher Eigentumsvorbehalt).",
  },

  // 29. Subcontracting Without Notice or Consent (§ 307 BGB, Art. 28 DSGVO)
  {
    type: "subcontractor_uncontrolled",
    title: "Freie und unangekündigte Subunternehmerbeauftragung",
    tags: ["dsgvo-art-28", "bgb-307", "subunternehmer", "datenschutz"],
    patterns: [
      /(?:anbieter\s+ist\s+berechtigt|berechtigt)[^.!?;:\n]{0,100}?(?:subunternehmer|nachunternehmer|nachunternehmen|dritte|unterauftragnehmer)[^.!?;:\n]{0,80}?einzusetzen/i,
      /(?:einsatz\s+von\s+subunternehmern|subunternehmer\w*|unterauftragnehmer\w*)[^.!?;:\n]{0,80}?(?:ohne\s+vorherige\s+ankündigung|ohne\s+zustimmung|jederzeit|bedarf\s+keiner\s+zustimmung)/i,
      /(?:drittanbieter|unterauftragnehmer|subunternehmer)[^.!?;:\n]{0,60}?(?:heranziehen|beauftragen)[^.!?;:\n]{0,80}?(?:ohne\s+(?:dies\s+)?(?:dem\s+[^.!?;:\n]{0,30}?)?(?:vorher\s+)?anzuzeigen|ohne\s+vorherige\s+anzeige|ohne\s+zustimmung)/i,
      // Red Team: Euphemismus-/DSGVO-Nominal-/English-Evasion
      /(?:kann|darf|ist\s+berechtigt)[^.!?;:\n]{0,100}?auf\s+(?:partnerunternehmen|verbundene\s+gesellschaften|verbundene\s+unternehmen)[^.!?;:\n]{0,50}?zur(?:ü|ue)ckgreifen/i,
      /(?:zustimmung|einwilligung)\s+des\s+(?:auftraggebers|kunden)[^.!?;:\n]{0,60}?bedarf\s+es\s+nicht/i,
      /(?:verarbeitung\s+personenbezogener\s+daten|personenbezogene\s+daten)[^.!?;:\n]{0,140}?auf\s+(?:verbundene\s+unternehmen|externe\s+dienstleister|dritte)[^.!?;:\n]{0,50}?(?:ü|ue)bertragen/i,
      /(?:subunternehmer|unterauftragnehmer|drittanbieter|auftragsverarbeiter)[^.!?;:\n]{0,160}?widerspruchsrecht[^.!?;:\n]{0,60}?besteht\s+nicht/i,
      /provider\s+may\s+appoint[^.!?;:\n]{0,120}?(?:subcontractors|affiliates|third\s+parties)/i,
      /at\s+its\s+sole\s+discretion[^.!?;:\n]{0,100}?without\s+prior\s+notice/i,
    ],
    severity(s) {
      if (/personenbezogen|daten/i.test(s)) return "hoch";
      return "mittel";
    },
    why: "Unkontrollierter Subunternehmereinsatz (§ 307 BGB, Art. 28 Abs. 2 DSGVO): Der Anbieter darf Leistungen oder sensible Daten ohne Information oder Widerspruchsrecht des Kunden an unbekannte Dritte weiterdelegieren.",
    suggestion: "Vorab-Informationspflicht (mindestens 14 Tage vor Einsatz) und Widerspruchsrecht aus wichtigem Grund vereinbaren; vertragliche Pflichten vollständig auf Subunternehmer überbinden.",
  },

  // 30. Unilateral Audit Log / Modification by Vendor (§ 307 BGB)
  {
    type: "audit_log_manipulation",
    title: "Beweismonopol durch Protokolle des Anbieters",
    tags: ["bgb-307", "bgb-309-nr-12", "logdaten", "beweismonopol"],
    patterns: [
      /(?:systemprotokolle|serverlogs|logfiles|aufzeichnungen\s+des\s+anbieters)[^.!?;:\n]{0,80}?(?:gelten\s+als\s+vollbeweis|liefern\s+den\s+vollständigen\s+beweis|sind\s+maßgeblich\s+und\s+unwiderlegbar)/i,
      /protokolle\s+des\s+anbieters[^.!?;:\n]{0,60}?(?:begründen\s+die\s+widerlegliche\s+vermutung\s+nicht|sind\s+vom\s+kunden\s+anzuerkennen)/i,
      /(?:systemprotokolle|serverlogs|logfiles|aufzeichnungen|protokolldateien)[^.!?;:\n]{0,100}?(?:gelten\s+als\s+(?:unwiderlegbar\w*\s+)?(?:vollbeweis|beweis|maßgeblich))/i,
      // "unwiderlegbarer Nachweis" + Unterwerfungserklärung (acc-synonym-32):
      /aufzeichnungen\s+des\s+(?:dienstleisters|anbieters|auftragnehmers)[^.!?;:\n]{0,120}?unwiderlegbar\w*/i,
      /unwiderlegbar\w*\s+(?:nachweis|beweis|feststellung)[^.!?;:\n]{0,80}?(?:zu|f(?:ü|ue)r\s+(?:umfang|art|g(?:ü|ue)te))/i,
      /(?:kunde|auftraggeber)\s+unterwirft\s+sich[^.!?;:\n]{0,60}?(?:feststellung|festsetzung|ermittlung|bewertung)/i,
      // Nominalstil: "Protokollierung des Auftragnehmers maßgeblich" + "abweichende Nachweise ohne Bedeutung" (acc-nominal-style-33):
      /(?:ausschlie(?:ß|ss)lich\s+die\s+)?protokollierung\s+des\s+(?:auftragnehmers|anbieters|dienstleisters)[^.!?;:\n]{0,80}?ma(?:ß|ss)geblich/i,
      /abweichende\s+(?:nachweise|aufzeichnungen|protokolle|belege|daten)[^.!?;:\n]{0,60}?(?:bleiben\s+ohne\s+bedeutung|sind\s+unbeachtlich|werden\s+nicht\s+ber(?:ü|ue)cksichtigt|sind\s+ohne\s+bedeutung)/i,
      // Englisch: "logs shall constitute conclusive and binding evidence" + "waives any right to challenge" (acc-english-34):
      /(?:server\s+)?logs?[^.!?;:\n]{0,60}?(?:shall\s+constitute|constitute|are)\s+(?:conclusive|binding|irrefutable|unrebuttable|final)/i,
      /(?:shall\s+constitute|constitute)\s+(?:conclusive|irrefutable|unrebuttable)[^.!?;:\n]{0,60}?evidence/i,
      /waives?\s+(?:any\s+|all\s+)?right\s+to\s+(?:challenge|contest|dispute|rebut|question)/i,
    ],
    severity: "hoch",
    why: "Beweismonopol & unwiderlegbare Vermutung (§ 309 Nr. 12 BGB, § 307 BGB): Logdateien des Anbieters werden als unwiderlegbarer Beweis für Fehlerfreiheit oder Zugriffszahlen festgeschrieben. Dies nimmt dem Kunden jede Möglichkeit des Gegenbeweises.",
    suggestion: "Klausel streichen. Logdaten dürfen allenfalls als widerlegbares Indiz herangezogen werden.",
  },

  // 31. Suspension of Service (Cut-Off Trap) (§ 307, § 320 BGB)
  {
    type: "service_suspension",
    title: "Unverhältnismäßige Dienstsperre (Leistungseinstellung)",
    tags: ["bgb-320", "bgb-307", "sperre", "leistungseinstellung"],
    patterns: [
      /(?:ohne\s+(?:vorherige\s+)?abmahnung|ohne\s+mahnung)[^.!?;:\n]{0,100}?(?:zugang|plattform|dienste?)[^.!?;:\n]{0,50}?(?:zu\s+sperren|sperren)/i,
      /(?:den\s+zugang\s+[^.!?;:\n]{0,80}?zu\s+sperren)[^.!?;:\n]{0,100}?(?:gebühren\s+hiervon\s+unberührt|zahlungspflicht\s+unberührt)/i,
      /(?:anbieter\s+ist\s+berechtigt|berechtigt)[^.!?;:\n]{0,120}?(?:ohne\s+vorankündigung|sofort|fristlos)[^.!?;:\n]{0,80}?(?:den\s+zugang\s+zu\s+sperren|die\s+leistung\s+einzustellen|den\s+dienst\s+abzuschalten|den\s+zugang\s+sofort\s+zu\s+sperren)/i,
      /(?:dienstsperre|kontosperrung|zugangssperre|sperrung\s+des\s+zugangs|den\s+zugang\s+zu\s+sperren)[^.!?;:\n]{0,80}?(?:ohne\s+vorankündigung|bereits\s+bei\s+geringfügigem|ohne\s+mahnung|lässt\s+die\s+zahlungspflicht\s+unberührt)/i,
      /(?:sperrung|gesperrt)[^.!?;:\n]{0,80}?(?:lässt\s+die\s+zahlungspflicht\s+unberührt|ohne\s+vorankündigung)/i,
      /(?:zugang|konto|dienst|leistung|cloud-infrastruktur)[^.!?;:\n]{0,80}?(?:sofort|fristlos|ohne\s+vorankündigung|ohne\s+mahnung)[^.!?;:\n]{0,40}?(?:zu\s+sperren|einzustellen|abzuschalten|sperren)/i,
      /(?:berechtigt|vorbehalten)[^.!?;:\n]{0,80}?(?:den\s+zugang\s+[^.!?;:\n]{0,140}?(?:zu\s+sperren|sperren))/i,
      /(?:zahlungsrückstand\s+von\s+mehr\s+als[^.!?;:\n]{0,40}?zu\s+sperren|ohne\s+vorherige\s+ankündigung\s+zu\s+sperren)/i,
      /(?:entsperrung\s+wird\s+eine\s+gebühr|entsperrgebühr)/i,
      /(?:unterbricht|unterbrechen|stellt|stellen|setzt|setzen)\s+(?:der\s+\w+\s+)?(?:die\s+)?(?:leistungserbringung|leistung|erbringung|versorgung|dienste?)[^;:\n]{0,60}?(?:ohne\s+vorherige\s+(?:ankündigung|information|abmahnung)|ohne\s+abmahnung|ohne\s+vorwarnung|sofort|unverzüglich|mit\s+sofortiger\s+wirkung)/i,
      /(?:may\s+immediately\s+suspend|may\s+suspend\s+all\s+or\s+part)[^;:\n]{0,90}/i,
      // "aussetzen" + Weiterzahlungspflicht (bind-euphemism-28):
      /(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,60}?kann[^.!?;:\n]{0,60}?(?:die\s+)?(?:leistungserbringung|leistung|dienste|bereitstellung|versorgung)[^.!?;:\n]{0,50}?aussetzen/i,
      /(?:verg(?:ü|ue)tung|entgelt|zahlung)[^.!?;:\n]{0,100}?(?:weiterhin|unver(?:ä|ae)ndert|uneingeschr(?:ä|ae)nkt|vollumf(?:ä|ae)nglich)[^.!?;:\n]{0,60}?(?:zu\s+entrichten|zu\s+zahlen|geschuldet|f(?:ä|ae)llig)/i,
      /f(?:ü|ue)r\s+den\s+zeitraum\s+der\s+(?:aussetzung|sperrung)[^.!?;:\n]{0,80}?(?:verg(?:ü|ue)tung|entgelt|zahlung)/i,
      // "jederzeit ... einstellen" + "nicht erstattet" (bind-synonym-29):
      /(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,50}?darf[^.!?;:\n]{0,100}?(?:bereitstellung|leistungserbringung|dienste|leistung)[^.!?;:\n]{0,100}?(?:jederzeit|sofort|mit\s+sofortiger\s+wirkung)[^.!?;:\n]{0,60}?(?:einstellen|einzustellen|aussetzen|abzuschalten)/i,
      /bereits\s+gezahlte\s+entgelte[^.!?;:\n]{0,100}?nicht\s+erstattet/i,
      /(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,80}?(?:erbringung|leistungserbringung|leistungen|dienste?|bereitstellung)[^.!?;:\n]{0,60}?sofort\s+(?:einzustellen|einstellen|zu\s+sperren|auszusetzen|abzuschalten)/i,
      /(?:verg(?:ü|ue)tung|entgelt|zahlung)[^.!?;:\n]{0,100}?l(?:ä|ae)uft[^.!?;:\n]{0,80}?(?:unver(?:ä|ae)ndert\s+weiter|w(?:ä|ae)hrend\s+der\s+sperre)/i,
      // Englisch: "may suspend ... immediately without notice" + "remain liable for all fees" (bind-english-30):
      /may\s+(?:immediately\s+)?suspend[^.!?;:\n]{0,120}?(?:immediately|with\s+immediate\s+effect|without\s+(?:prior\s+)?notice|for\s+any\s+breach)/i,
      /(?:customer|client)\s+shall\s+remain\s+(?:liable|obligated)[^.!?;:\n]{0,80}?(?:fees|charges|payment)/i,
      /remain\s+liable\s+for\s+all\s+fees[^.!?;:\n]{0,60}?during/i,
    ],
    severity: "hoch",
    why: "Unverhältnismäßige Dienstsperre (§ 307, § 320 BGB): Der Anbieter darf den Dienst ohne Vorwarnung oder Mahnung sofort sperren, verlangt aber gleichzeitig die volle Vergütung für die Ausfallzeit.",
    suggestion: "Sperrung erst nach Mahnung mit mindestens 14 Tagen Zahlungsfrist und einem Zahlungsrückstand von mindestens zwei Monatsbeiträgen zulassen.",
  },

  // 32. Insolvency / Termination on Financial Distress (§ 119 InsO)
  {
    type: "insolvency_termination",
    title: "Unwirksame insolvenzabhängige Lösungsklausel",
    tags: ["inso-119", "bgb-307", "insolvenz", "kuendigung"],
    patterns: [
      /(?:vertrag\s+endet\s+automatisch|berechtigt\s+zur\s+fristlosen\s+kündigung|kündigungsrecht)[^.!?;:\n]{0,120}?(?:antrag\s+auf\s+eröffnung\s+(?:eines\s+)?insolvenzverfahrens|insolvenzantrag|eröffnung\s+des\s+insolvenzverfahrens|drohender\s+zahlungsunfähigkeit)/i,
      /(?:vertrag\s+endet\s+automatisch|berechtigt\s+zur\s+fristlosen\s+kündigung)[^.!?;:\n]{0,80}?(?:bei\s+antrag\s+auf\s+eröffnung\s+eines\s+insolvenzverfahrens|bei\s+drohender\s+zahlungsunfähigkeit)/i,
      /insolvenzantrag[^.!?;:\n]{0,80}?(?:berechtigt\s+zur\s+außerordentlichen\s+kündigung|führt\s+zum\s+automatischen\s+erlöschen)/i,
      /(?:bei\s+(?:einem\s+)?insolvenzverfahren|im\s+falle\s+(?:der\s+)?insolvenz|bei\s+insolvenz)[^.!?;:\n]{0,80}?(?:endet\s+der\s+vertrag|erlischt\s+der\s+vertrag|ist\s+der\s+vertrag\s+beendet)[^.!?;:\n]{0,40}?(?:ohne\s+weiteres|automatisch|sofort)/i,
      /(?:endet\s+der\s+vertrag|erlischt\s+der\s+vertrag)[^.!?;:\n]{0,40}?(?:ohne\s+weiteres|automatisch)[^.!?;:\n]{0,80}?(?:insolvenz|insolvenzverfahren)/i,
      // Umgekehrte Wortstellung: "berechtigt, den Vertrag fristlos zu kündigen" (bind-direct-32, bind-word-number-36):
      /berechtigt[^.!?;:\n]{0,80}?(?:den\s+vertrag\s+)?fristlos\s+zu\s+k(?:ü|ue)ndigen/i,
      /insolvenz\w*[^.!?;:\n]{0,140}?(?:zur\s+fristlosen\s+k(?:ü|ue)ndigung|fristlos\s+zu\s+k(?:ü|ue)ndigen|fristlose\s+k(?:ü|ue)ndigung)/i,
      /insolvenzantrag[^.!?;:\n]{0,120}?berechtigt/i,
      // Blosse Zahlungsunfähigkeit / Überschuldung als Beendigungsgrund (bind-synonym-33):
      /(?:zahlungsunf(?:ä|ae)higkeit|(?:ü|ue)berschuldung)[^.!?;:\n]{0,100}?(?:endet\s+der\s+vertrag|endet\s+das\s+vertragsverh(?:ä|ae)ltnis|ist\s+der\s+vertrag\s+beendet)/i,
      /endet\s+der\s+vertrag[^.!?;:\n]{0,60}?(?:zahlungsunf(?:ä|ae)higkeit|(?:ü|ue)berschuldung|insolvenz)/i,
      // Passive Rechtsfolge: "führt zum automatischen Erlöschen ... ohne dass es einer Kündigung bedarf" (bind-nominal-style-34):
      /(?:er(?:ö|oe)ffnung\s+des\s+insolvenzverfahrens|insolvenzverfahren)[^.!?;:\n]{0,120}?(?:f(?:ü|ue)hrt\s+zum\s+automatischen|zum\s+automatischen\s+erl(?:ö|oe)schen|erl(?:ö|oe)schen\s+s(?:ä|ae)mtlicher)/i,
      /insolvenz\w*[^.!?;:\n]{0,140}?ohne\s+dass\s+es\s+einer\s+k(?:ü|ue)ndigung\s+bedarf/i,
    ],
    severity: "hoch",
    why: "Unwirksame insolvenzabhängige Lösungsklausel (§ 119 InsO, BGH IX ZR 169/11): Vertragsklauseln, die an den bloßen Antrag auf Eröffnung eines Insolvenzverfahrens Kündigungsrechte knüpfen, sind unwirksam, da sie die Fortführung des Unternehmens im Insolvenzverfahren vereiteln.",
    suggestion: "Klausel streichen. Kündigung nur nach den allgemeinen insolvenzrechtlichen Regeln des Insolvenzverwalters zulassen.",
  },

  // 33. Reverse Charge & Tax Indemnity Shifting (§ 307 BGB)
  {
    type: "tax_indemnity_shift",
    title: "Einseitige Steuer- und Nachforderungsabwälzung",
    tags: ["bgb-307", "ustg", "steuern", "haftungsverlagerung"],
    patterns: [
      /(?:steuernachforderungen|steuer-?\s*(?:oder\s+)?quellensteuernachforderungen)[^.!?;:\n]{0,80}?(?:sind\s+vollumfänglich\s+vom\s+kunden|vom\s+kunden\s+zu\s+erstatten|vom\s+kunden\s+zu\s+tragen)/i,
      /(?:sämtliche\s+steuern|umsatzsteuer|quellennachforderungen)[^.!?;:\n]{0,80}?(?:gehen\s+zu\s+lasten\s+des\s+kunden|sind\s+vom\s+kunden\s+nachzuentrichten|stellt\s+von\s+allen\s+steuerforderungen\s+frei)/i,
      /steuernachforderungen[^.!?;:\n]{0,80}?(?:auch\s+rückwirkend|verschuldensunabhängig\s+vom\s+kunden)/i,
      /(?:s(?:ä|ae)mtliche\s+steuern|steuern\s+und\s+abgaben)[^.!?;:\n]{0,180}?tr(?:ä|ae)gt\s+(?:diese\s+)?der\s+kunde/i,
      // Steuerabwaelzung in Aufzaehlung mit Satzende-Subjekt (Welle D):
      /(?:steuern|abgaben)\s*(?:,|und)\s+(?:abgaben|steuern|geb(?:ü|ue)hren|sonstige)[^.!?;:\n]{0,160}?tr(?:ä|ae)gt\s+(?:diese\s+|s(?:ä|ae)mtliche\s+|alle\s+)?(?:der\s+)?(?:kunde|auftraggeber|besteller)/i,
      // Freistellung des Anbieters von Steuernachforderungen durch den Kunden (Welle D):
      /(?:stellt|befreit)[^.!?;:\n]{0,60}?(?:den\s+)?(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,100}?von\s+(?:s(?:ä|ae)mtlichen\s+|allen\s+)?steuernachforderungen[^.!?;:\n]{0,80}?frei/i,
      // Red Team: Synonym-/Euphemismus-/Cross-Reference-/English-Syntax
      /(?:steuerliche\w*\s+)?mehrbelastungen[^.!?;:\n]{0,140}?(?:dem\s+kunden\s+belastet|vollumf(?:ä|ae)nglich)/i,
      /vollumf(?:ä|ae)nglich\s+dem\s+kunden\s+belastet/i,
      /unabh(?:ä|ae)ngig\s+davon,\s+ob\s+den\s+kunden\s+ein\s+verschulden\s+trifft/i,
      /nachtr(?:ä|ae)gliche\s+abgabenbelastungen[^.!?;:\n]{0,100}?tr(?:ä|ae)gt\s+(?:diese\s+)?der\s+kunde/i,
      /(?:steuerlichen\s+w(?:ü|ue)rdigung)[^.!?;:\n]{0,140}?tr(?:ä|ae)gt\s+(?:diese\s+)?der\s+kunde/i,
      /(?:nachbelastungen|s(?:ä|ae)umniszuschl(?:ä|ae)ge|haftungsbetr(?:ä|ae)ge)[^.!?;:\n]{0,140}?richtet\s+sich\s+nach[^.!?;:\n]{0,40}?(?:ziffer|§|\d)/i,
      /(?:customer|client)\s+shall\s+(?:bear|pay|reimburse)[^.!?;:\n]{0,160}?(?:taxes|assessments)/i,
      /(?:withholding|turnover|transfer)\s+taxes[^.!?;:\n]{0,140}?(?:interest|penalties|retrospective)/i,
      /retrospective\s+assessments/i,
      /kunde\s+stellt[^.!?;:\n]{0,80}?anbieter[^.!?;:\n]{0,100}?steuerlichen\s+nachforderungen/i,
      /(?:steuerlichen?\s+nachforderungen|s(?:ä|ae)umniszuschl(?:ä|ae)ge)[^.!?;:\n]{0,140}?frei/i,
      /von\s+finanzbeh(?:ö|oe)rden[^.!?;:\n]{0,140}?geltend\s+gemacht\s+werden/i,
    ],
    severity: "mittel",
    why: "Einseitige Steuerabwälzung (§ 307 BGB): Risiken fehlerhafter steuerlicher Einordnung des Anbieters (z. B. Umsatzsteuer, Quellensteuer) werden pauschal und rückwirkend auf den Kunden abgewälzt.",
    suggestion: "Steuernachforderungen nur bei fehlerhaften Angaben oder Verschulden des Kunden zulassen.",
  },

  // 34. Confidentiality Asymmetry (§ 307 BGB, GeschGehG)
  {
    type: "asymmetric_confidentiality",
    title: "Einseitige Geheimhaltungspflicht nur für den Kunden",
    tags: ["geschgehg", "bgb-307", "geheimhaltung", "nda", "asymmetrie"],
    patterns: [
      /(?:kunde\s+verpflichtet\s+sich|auftraggeber\s+verpflichtet\s+sich)[^.!?;:\n]{0,140}?(?:vertraulich\w*|stillschweigen\w*|geheimzuhalten|geheimhaltung)/i,
      /für\s+den\s+anbieter\s+gilt\s+dies\s+nicht/i,
      /(?:kunde\s+verpflichtet\s+sich|der\s+auftraggeber\s+verpflichtet\s+sich)[^.!?;:\n]{0,140}?(?:vertraulich|stillschweigen|geheimzuhalten)/i,
      /(?:streng\s+vertraulich\s+zu\s+behandeln|stillschweigen\s+zu\s+bewahren)[^.!?;:\n]{0,80}?(?:obliegt\s+dem\s+kunden|durch\s+den\s+kunden)/i,
      /geheimhaltungspflicht\w*[^.!?;:\n]{0,80}?(?:trifft\s+nur\s+den\s+kunden|gilt\s+ausschließlich\s+für\s+den\s+kunden)/i,
      // Einseitige Durchbrechung der Vertraulichkeit (Threat-Intel-Sharing, Behördenherausgabe ohne Information):
      /ungeachtet\s+der\s+(?:wechselseitigen\s+)?(?:verpflichtung\s+zur\s+)?(?:geheimhaltung|vertraulichkeit)[^;:\n]{0,260}?(?:berechtigt|befugt)[^;:\n]{0,420}?(?:zu\s+teilen|weiterzugeben|offenzulegen|herauszugeben|zu\s+übermitteln)/i,
      /ohne\s+vorherige\s+information\s+des\s+kunden[^.!?;:\n]{0,120}?(?:nachzukommen|nachkommen|herauszugeben|offenzulegen|auskunft)/i,
      /(?:behördlicher\s+oder\s+gerichtlicher\s+seite|staatliche\s+stellen|aufsichtsbehörden)[^.!?;:\n]{0,160}?(?:herausgabe|offenlegung|auskunft|datensätze)[^.!?;:\n]{0,120}?ohne\s+(?:vorherige\s+)?(?:information|benachrichtigung|zustimmung|einwilligung)/i,
      // Red Team: Pflichtenträger-Syntax / Passiv / English
      /verschwiegenheits(?:verpflichtung|pflicht)[^.!?;:\n]{0,80}?obliegt\s+ausschlie(?:ß|ss)lich\s+dem\s+(?:besteller|kunden|auftraggeber)/i,
      /ausschlie(?:ß|ss)lich\s+dem\s+(?:besteller|kunden|auftraggeber)[^.!?;:\n]{0,80}?(?:verschwiegenheits|vertraulichkeits|geheimhaltungs)/i,
      /den\s+(?:auftragnehmer|anbieter)\s+trifft\s+lediglich\s+die\s+gesetzliche\s+pflicht/i,
      /customer\s+shall\s+keep[^.!?;:\n]{0,80}?(?:confidential|secret)/i,
      /provider\s+assumes\s+no\s+confidentiality\s+obligation/i,
      /no\s+confidentiality\s+obligation[^.!?;:\n]{0,80}?(?:benchmarking|product\s+improvement)/i,
      /durch\s+den\s+(?:auftraggeber|kunden|besteller)[^.!?;:\n]{0,60}?streng\s+vertraulich\s+zu\s+behandeln/i,
      /(?:pflicht|verpflichtung)\s+des\s+(?:anbieters|auftragnehmers)[^.!?;:\n]{0,120}?besteht\s+nicht/i,
    ],
    collect(sentences, _full, extra = null) {
      const whole = sentences.map(flattenSentence).join(" ");
      // Symmetrie-Guard: verpflichtet der Vertrag auch den Anbieter ausdrücklich, ist er beidseitig.
      const providerObligated =
        /(?:anbieter|auftragnehmer)\s+verpflichtet\s+sich[^.!?;:\n]{0,80}?(?:vertraulich|stillschweigen|geheim)/i.test(whole) ||
        /(?:auch\s+)?den\s+(?:anbieter|auftragnehmer)\s+trifft\s+(?:eine|die)\s+(?:entsprechende\s+)?(?:verschwiegenheits|vertraulichkeits|geheimhaltungs)/i.test(whole) ||
        /(?:anbieter|auftragnehmer)[^.!?;:\n]{0,60}(?:zur\s+geheimhaltung|zur\s+vertraulichkeit)\s+verpflichtet/i.test(whole);
      const providerExempt =
        /(?:pflicht|verpflichtung)\s+des\s+(?:anbieters|auftragnehmers)[^.!?;:\n]{0,120}?besteht\s+nicht/i.test(whole) ||
        /anbieter\s+assumes\s+no\s+confidentiality/i.test(whole);
      if (providerObligated && !providerExempt) return [];
      return collectPerSentence(this, sentences, null, extra);
    },
    severity(s) {
      // Wenn der Vertrag auch den Anbieter zur Geheimhaltung verpflichtet, ist es beidseitig
      if (/beide\s+parteien|gegenseitig\s+verpflichten|verpflichten\s+sich\s+beide/i.test(s)) return null;
      return "mittel";
    },
    why: "Einseitige Geheimhaltungsverpflichtung (§ 307 BGB, GeschGehG): Nur der Kunde wird zur Geheimhaltung verpflichtet, während der Anbieter Kundendaten und Betriebsgeheimnisse ohne vergleichbaren Schutz behandeln darf.",
    suggestion: "Geheimhaltungsverpflichtung strikt wechselseitig ausgestalten (beidseitige Verschwiegenheitspflicht).",
  },

  // 35. Warranty Disclaimer on "As Is" / Software-Überlassung (§ 309 Nr. 8 lit. b BGB)
  {
    type: "as_is_software_disclaimer",
    title: "Vollständiger Ausschluss der Sachmängelhaftung (As-Is-Falle)",
    tags: ["bgb-309-nr-8", "bgb-307", "as-is", "gewaehrleistungsausschluss"],
    patterns: [
      /(?:ausschließlich\s+als\s+dienstvertrag|reiner\s+dienstvertrag)[^;:\n]{0,80}?(?:kein\s+erfolg|erfolg\s+nicht\s+geschuldet|keine\s+erzielung\s+eines\s+erfolgs)/i,
      /(?:erzielung[^.!?;:\n]{0,100}?erfolgs?\s+ist\s+nicht\s+geschuldet|kein\s+erfolg\s+geschuldet)/i,
      /(?:software|leistung|dienstleistung)\s+wird\s+(?:„wie\s+gesehen“|wie\s+gesehen|"as\s+is"|ohne\s+mängelgewähr)\s+bereitgestellt/i,
      /sachmängelhaftung[^.!?;:\n]{0,60}?(?:vollständig\s+ausgeschlossen|gänzlich\s+abbedungen)/i,
      /(?:keine\s+gewährleistung|gewährleistung\s+ist\s+ausgeschlossen)[^.!?;:\n]{0,60}?(?:für\s+sachmängel|für\s+funktionsfähigkeit)/i,
      /(?:software|leistung|dienstleistung|service|platform)[^.!?;:\n]{0,80}?(?:im\s+zustand\s+)?(?:wie\s+gesehen|"as\s+is"|as\s+is|ohne\s+mängelgewähr)/i,
      /(?:software|service|product|platform)\s+is\s+provided\s+as\s+is/i,
      /without\s+warranty\s+of\s+any\s+kind/i,
      // Funktions-, Erkennungs- und Garantieausschluss in Dienstleistungsverträgen (MDR/SOC/SLA):
      /(?:garantiert|gewährleistet)\s+(?:daher\s+|ausdrücklich\s+|hiermit\s+)?weder[^.!?;:\n]{0,200}?noch/i,
      /(?:nichtanschlagen|ausbleiben\s+einer\s+(?:sicherheitswarnung|warnung|meldung))[^.!?;:\n]{0,160}?(?:begründet|stellt)\s+keinen?\s+mangel/i,
      /keine\s+pflichtverletzung\s+des\s+(?:sicherheitsdienstleisters|anbieters|dienstleisters|auftragnehmers)/i,
      /(?:wird|es\s+wird|wird\s+hiermit)\s+keine\s+zusage[^;:\n]{0,60}?(?:übernommen|gemacht|erteilt|abgegeben)/i,
      /keine\s+zusage\s+(?:über|hinsichtlich|dafür|für|bezüglich)/i,
      /keine\s+(?:gewähr|gewährleistung)\s+(?:dafür|dafür|für|über|hinsichtlich)/i,
      // Red Team: Euphemismus-/Synonym-/Split-/English-Evasion
      /ohne\s+(?:jede\s+|jegliche\s+|besondere\s+|ausdr(?:ü|ue)ckliche\s+)?zusicherung[^.!?;:\n]{0,120}?(?:beschaffenheit|eignung|einsatzzweck)/i,
      /(?:bereitstellung|überlassung|software|anwendung)[^.!?;:\n]{0,60}?im\s+ist-?zustand/i,
      /anspruch\s+auf\s+beseitigung[^.!?;:\n]{0,120}?beste(?:ht|hen)\s+nicht/i,
      /anpassung\s+an\s+einen\s+bestimmten\s+einsatzzweck[^.!?;:\n]{0,60}?beste(?:ht|hen)\s+nicht/i,
      /in\s+its\s+current\s+state[^.!?;:\n]{0,140}?(?:with\s+all\s+faults|accepts?\s+it\s+as\s+such)/i,
      /with\s+all\s+faults\s+and\s+defects/i,
      /accepts?\s+it\s+as\s+such/i,
      /wie\s+besehen/i,
      /zusicherungen\s+zu[^.!?;:\n]{0,120}?nicht\s+bestandteil\s+dieses\s+vertrages/i,
    ],
    severity: "hoch",
    why: "Vollständiger Gewährleistungsausschluss bei Neuleistungen (§ 309 Nr. 8 lit. b BGB, § 307 BGB): Die Klausel 'wie gesehen' oder der pauschale Ausschluss der Mängelhaftung für Standardfunktionen entkernt die Leistungspflicht des Anbieters.",
    suggestion: "Gesetzliche Sachmängelhaftung (§ 437 / § 634 BGB) vereinbaren: Nacherfüllung binnen angemessener Frist, Minderung und Rücktritt bei Fehlschlagen.",
  },

  // 36. Entire Agreement Trap / Ausschluss mündlicher Absprachen (§ 305b BGB)
  {
    type: "entire_agreement",
    title: "Vollständigkeitsklausel & Ausschluss mündlicher Zusagen",
    tags: ["bgb-305b", "bgb-307", "vollstaendigkeitsklausel", "entire-agreement", "individualabrede"],
    patterns: [
      /(?:ungeachtet|trotz)\s+(?:etwaiger\s+|entgegenstehender\s+)?nebenabreden/i,
      /nebenabreden[^.!?;:\n]{0,60}?(?:zwingend\s+vorrang|stets\s+vorrang|keine\s+geltung)/i,
      /(?:frühere|bisherige|vorherige|vorvertragliche|bisherigen|früheren)\s+(?:absprachen|zusagen|leistungsbeschreibungen|angebote|vereinbarungen|verhandlungen)[^.!?;:\n]{0,120}?(?:verlieren[^.!?;:\n]{0,50}?(?:ihre\s+gültigkeit|jede\s+gültigkeit)|treten\s+außer\s+kraft|sind\s+unverbindlich|werden\s+aufgehoben|gelten\s+nicht\s+mehr)/i,
      /(?:entire\s+agreement|whole\s+agreement|supersedes\s+all\s+prior|sole\s+and\s+entire\s+agreement)/i,
      /(?:mündliche|mündlichen?)\s+(?:nebenabreden|absprachen|zusagen|vereinbarungen)\s+(?:bestehen\s+nicht|sind\s+nicht\s+getroffen|haben\s+keine\s+gültigkeit|sind\s+unwirksam|ausgeschlossen)/i,
      // Muendliche Vereinbarungen/Verhandlungen fuer unverbindlich erklaert (Welle D):
      /m(?:ü|ue)ndliche\s+(?:vereinbarungen|absprachen|abreden|zusagen|verhandlungen)[^.!?;:\n]{0,160}?sind\s+unverbindlich/i,
      /(?:frühere|bisherige)\s+(?:absprachen|zusagen|leistungsbeschreibungen|angebote|vereinbarungen|verhandlungen)[^.!?;:\n]{0,90}?(?:verlieren\s+ihre\s+gültigkeit|treten\s+außer\s+kraft|sind\s+unverbindlich|werden\s+aufgehoben)/i,
      /dieser\s+vertrag\s+enthält\s+alle\s+vereinbarungen[^.!?;:\n]{0,80}?(?:frühere|mündliche|weitere)/i,
      /keine\s+mündlichen\s+nebenabreden/i,
      /mündliche\s+zusagen[^.!?;:\n]{0,60}?(?:bedürfen\s+der\s+schriftlichen\s+bestätigung|sind\s+nicht\s+bindend)/i,
      // Red Team: Euphemismus-/Synonym-/English-/Listen-Evasion
      /zuvor\s+kommunizierte[n]?\s+(?:leistungsbeschreibungen|servicebeschreibungen|kostensch(?:ä|ae)tzungen|angebote)/i,
      /treten\s+s(?:ä|ae)mtliche\s+zuvor\s+kommunizierten[^.!?;:\n]{0,160}?au(?:ß|ss)er\s+kraft/i,
      /m(?:ü|ue)ndliche\s+(?:erkl(?:ä|ae)rungen|zusagen)[^.!?;:\n]{0,100}?(?:begr(?:ü|ue)nden\s+keine|sind\s+f(?:ü|ue)r\s+die\s+auslegung[^.!?;:\n]{0,60}?nicht\s+heranzuziehen)/i,
      /(?:erkl(?:ä|ae)rungen|zusagen)\s+von\s+vertriebsmitarbeitern/i,
      /complete\s+and\s+final\s+understanding/i,
      /replaces\s+any\s+earlier[^.!?;:\n]{0,100}?(?:quotation|representation|warranty)/i,
      /(?:zusagen|nebenabreden|vertriebszusicherungen)[^.!?;:\n]{0,160}?entfalten\s+keine\s+rechtliche\s+wirkung/i,
      /weitergehende\s+zusagen[^.!?;:\n]{0,120}?keine\s+(?:rechtliche\s+)?wirkung/i,
    ],
    severity(s) {
      // § 305b BGB: der Vorrang individueller Abreden ist gerade gesetzeskonform.
      if (
        /(?:individual(?:abreden?|vereinbarung\w*|absprachen|zusagen)|individuell\s+ausgehandelt|individually\s+negotiated|signed\s+by\s+both\s+parties)[^.!?;:\n]{0,100}?(?:vorrang|verbindlich|bleiben|remain|unber(?:ü|ue)hrt)/i.test(
          s
        ) ||
        /(?:vorrang)[^.!?;:\n]{0,60}?(?:individual|individuell|m(?:ü|ue)ndliche)/i.test(s) ||
        /(?:individually\s+negotiated|individual\s+agreement)[^.!?;:\n]{0,80}?(?:binding|prevail|supersede)/i.test(s)
      ) {
        return null;
      }
      return "mittel";
    },
    suggestion: "Klarstellen, dass individuelle Absprachen, Verhandlungsprotokolle und vorvertragliche Leistungszusagen des Vertriebs verbindlicher Vertragsbestandteil bleiben.",
  },

  // 37. Marketing & Logo Rights Referral Trap (§ 307 BGB, MarkenG)
  {
    type: "marketing_reference",
    title: "Ungefragte Referenznennung & Markennutzung zu Werbezwecken",
    tags: ["bgb-307", "markeng", "referenzkunde", "logo-nutzung", "werberechte"],
    patterns: [
      /(?:kundennamen|firmennamen|unternehmenskennzeichen|bildmarken|logos?)[^.!?;:\n]{0,160}?(?:referenzangaben|marketingunterlagen|referenzzwecken|pressemitteilungen|werbung)/i,
      /(?:rahmen\s+von\s+referenzangaben|als\s+referenz\s+unentgeltlich)/i,
      /(?:als\s+referenz\s+zu\s+nennen|als\s+referenzkunden?\s+zu\s+führen|als\s+referenz\s+benennen)/i,
      /(?:namen|firmennamen|unternehmensnamen|logo|marken?)\s+(?:des\s+kunden|des\s+auftraggebers)[^.!?;:\n]{0,100}?(?:zu\s+referenzzwecken|zu\s+marketingzwecken|für\s+werbezwecke|auf\s+der\s+website|in\s+pressemitteilungen)/i,
      /(?:berechtigt|behält\s+sich\s+das\s+recht\s+vor)[^.!?;:\n]{0,80}?(?:den\s+kunden\s+als\s+referenz|das\s+logo\s+des\s+kunden\s+zu\s+verwenden|kunden\s+als\s+referenzkunden)/i,
      /referenzkunden?[^.!?;:\n]{0,80}?(?:unentgeltlich|ohne\s+gesonderte\s+zustimmung|zeitlich\s+und\s+räumlich\s+unbeschränkt)/i,
      /als\s+kunden\s+(?:namentlich\s+)?(?:zu\s+)?nennen/i,
      /(?:nennt|nennen|verwendet|verwenden)[^;:\n]{0,60}?(?:als\s+kunden|als\s+referenz)/i,
      /(?:dessen|deren)\s+(?:zeichen|marken?|logos?|firmenname)[^;:\n]{0,60}?(?:verwenden|nutzen|nennen)/i,
      // Red Team: Euphemismus-/Synonym-/Listen-Evasion
      /kundenbezeichnungen[^.!?;:\n]{0,100}?(?:pr(?:ä|ae)sentationen|website|angeboten|werbung)/i,
      /bildmarken[^.!?;:\n]{0,140}?(?:website|pr(?:ä|ae)sentationen|werbung|angeboten|aufgef(?:ü|ue)hrt)/i,
      /auftragnehmer\s+darf\s+den\s+auftraggeber[^.!?;:\n]{0,140}?als\s+kunden\s+f(?:ü|ue)hren/i,
      /in\s+(?:verzeichnissen|fallstudien|auss(?:c|ch)reibungsunterlagen)[^.!?;:\n]{0,80}?als\s+kunden/i,
      /(?:nennung\s+des\s+kunden|referenzkunde)[^.!?;:\n]{0,90}?(?:f(?:ü|ue)nf\s+jahren|\d+\s+jahre)[^.!?;:\n]{0,50}?zul(?:ä|ae)ssig/i,
      /gesonderte\s+zustimmung\s+des\s+kunden[^.!?;:\n]{0,60}?nicht\s+erforderlich/i,
    ],
    severity(s) {
      // Gesetzeskonform: Nutzung nur mit Einzelfreigabe und Widerrufsvorbehalt.
      if (
        /zustimmung/i.test(s) &&
        /(?:vorherig|einzelfall|widerruf)/i.test(s)
      ) {
        return null;
      }
      return "mittel";
    },
    why: "Ungefragte Referenzwerbung & Logo-Nutzung (§ 307 BGB, MarkenG): Der Anbieter räumt sich formularmäßig das Recht ein, Ihre Marke, Ihren Firmennamen und Ihr Logo ohne gesonderte Freigabe oder Vergütung für eigene Marketing- und Werbekampagnen zu nutzen.",
    suggestion: "Referenznennung und Logo-Nutzung nur mit vorheriger schriftlicher Zustimmung (Einzelfreigabe) und mit jederzeitiger Widerrufsmöglichkeit vereinbaren.",
  },

  // 38. Feedback & Enhancement IP Grab (§ 307 BGB, UrhG)
  {
    type: "feedback_ip_grab",
    title: "Kostenlose Abtretung von Kunden-Feedback, Ideen und Anpassungen",
    tags: ["bgb-307", "urhg", "feedback-klausel", "ip-grab", "ideenklau"],
    patterns: [
      /(?:feedback|anregungen|verbesserungsvorschläge|feature[\s-]requests?|ideen)[^.!?;:\n]{0,120}?(?:gehen\s+unentgeltlich|unentgeltlich\s+in\s+das\s+eigentum|ausschließliche\s+eigentum\s+des\s+anbieters|abgetreten|übertragen)/i,
      /(?:überträgt|tritt\s+ab)[^.!?;:\n]{0,80}?(?:alle\s+rechte\s+an|sämtliche\s+rechte\s+an)[^.!?;:\n]{0,80}?(?:feedback|verbesserungsvorschlägen?|anpassungsideen?)/i,
      /anregungen\s+und\s+vorschläge[^.!?;:\n]{0,80}?(?:ohne\s+vergütung|unentgeltlich|dauerhaft\s+und\s+kostenlos)/i,
      /feedback[^.!?;:\n]{0,60}?(?:geht\s+in\s+das\s+geistige\s+eigentum|ausschließlich\s+dem\s+anbieter\s+zustehen)/i,
      /(?:vorschläge|anregungen|feedback|ideen)[^.!?;:\n]{0,100}?(?:in\s+seine\s+produkte\s+einfließen\s+lassen|in\s+produkte\s+einfließen)/i,
      // Red Team: Euphemismus-/Synonym-/Nominal-/Listen-Evasion
      /(?:ideen|anregungen)[^.!?;:\n]{0,100}?darf\s+der\s+anbieter\s+frei\s+verwerten/i,
      /(?:anspr(?:ü|ue)che\s+des\s+kunden\s+auf\s+beteiligung|beteiligung\s+oder\s+verg(?:ü|ue)tung)[^.!?;:\n]{0,60}?(?:bestehen\s+nicht|nicht\s+vorgesehen)/i,
      /(?:anregungen|konzeptionelle|anforderungsskizzen|verbesserungsvorschl(?:ä|ae)ge)[^.!?;:\n]{0,160}?bestandteil\s+des\s+geistigen\s+eigentums\s+des\s+(?:auftragnehmers|anbieters)/i,
      /erfolgt\s+deren\s+unentgeltliche\s+(?:ü|ue)bertragung\s+aller\s+daran\s+bestehenden\s+rechte/i,
      /mit\s+(?:ü|ue)bermittlung\s+einer\s+idee[^.!?;:\n]{0,100}?unentgeltliche\s+(?:ü|ue)bertragung/i,
    ],
    severity: "mittel",
    why: "Einseitige Feedback-Enteignung (§ 307 BGB): Vorschläge, Anforderungen, Workflows oder Ideen des Kunden sollen automatisch und kostenlos in das geistige Eigentum des Anbieters übergehen, wodurch der Kunde eigene innovative Ideen an den Anbieter verliert.",
    suggestion: "Klarstellen, dass Feedback unverbindlich ist und keine Übertragung von gewerblichen Schutzrechten oder urheberrechtlichen Nutzungsrechten an eigenen Erfindungen beinhaltet.",
  },

  // 39. Fictitious Receipt / Zugangsfiktion (§ 308 Nr. 5 BGB)
  {
    type: "fictitious_receipt",
    title: "Fingierter Zugang von Erklärungen & Rechnungen (Zugangsfiktion)",
    tags: ["bgb-308-nr-5", "bgb-307", "zugangsfiktion", "fristenfalle"],
    patterns: [
      /(?:sie\s+)?gelten\s+als\s+(?:vorbehaltlos\s+)?(?:genehmigt|freigegeben)[^.!?;:\n]{0,80}?(?:innerhalb|binnen)\s+(?:von\s+)?(?:\d+|sieben)\s*(?:\(\d+\)\s*)?(?:kalendertagen|tagen|werktagen)/i,
      /(?:leistungsnachweise|timesheets|stundenzettel)[^.!?;:\n]{0,100}?(?:gelten\s+als\s+vorbehaltlos\s+genehmigt|als\s+freigegeben|als\s+genehmigt)/i,
      /(?:mitteilungen|erklärungen|rechnungen|mahnungen|schreiben)[^.!?;:\n]{0,60}?gelten[^.!?;:\n]{0,80}?(?:nach\s+absendung|nach\s+aufgabe\s+zur\s+post|nach\s+versand)[^.!?;:\n]{0,60}?(?:als\s+dem\s+kunden\s+zugegangen|als\s+zugegangen|als\s+bewirkt)/i,
      /(?:zugang\s+einer\s+erklärung|erklärungen|mitteilungen)[^.!?;:\n]{0,80}?(?:gilt|gelten)[^.!?;:\n]{0,80}?(?:nach\s+absendung|nach\s+versand)[^.!?;:\n]{0,40}?(?:als\s+bewirkt|als\s+zugegangen)/i,
      /(?:gilt|gelten)[^.!?;:\n]{0,60}?(?:am\s+tag\s+nach\s+(?:ihrer\s+)?absendung|am\s+folgetag|mit\s+dem\s+dritten\s+tag|am\s+(?:dritten|\d+\.)\s+(?:werk)?tag|nach\s+\d+\s+tagen)[^.!?;:\n]{0,60}?(?:nach\s+absendung|nach\s+versand)?[^.!?;:\n]{0,60}?(?:als\s+dem\s+kunden\s+zugegangen|als\s+bewirkt|als\s+zugegangen)/i,
      /(?:unabhängig\s+davon,\s+ob\s+und\s+wann\s+die\s+erklärung\s+den\s+kunden\s+tatsächlich\s+erreicht|unabhängig\s+vom\s+tatsächlichen\s+zugang)/i,
      /zugangsfiktion/i,
      /(?:mitteilungen|erklärungen|schreiben|rechnungen)[^.!?;:\n]{0,40}?gelten\s+als\s+zugegangen[^.!?;:\n]{0,100}?(?:sobald|wenn|nach\s+einstellung|elektronische\w*\s+postfach)/i,
      /(?:gelten|gilt)\s+als\s+zugegangen[^.!?;:\n]{0,100}?(?:sobald|in\s+das\s+[^.!?;:\n]{0,40}?postfach\s+eingestellt)/i,
      // Red Team: Euphemismus-/Synonym-/Split-/Nominal-/Typo-Evasion
      /mit\s+der\s+einstellung[^.!?;:\n]{0,80}?(?:sind\s+mitteilungen|gelten\s+mitteilungen)[^.!?;:\n]{0,60}?bekannt/i,
      /auf\s+den\s+tats(?:ä|ae)chlichen\s+abruf\s+kommt\s+es\s+nicht\s+an/i,
      /(?:einstellung\s+in\s+das\s+portal|einstellung\s+in\s+das\s+benachrichtigungsportal)[^.!?;:\n]{0,100}?bekannt/i,
      /mit\s+(?:ü|ue)bergabe\s+an\s+den\s+versanddienstleister[^.!?;:\n]{0,100}?gelangt\s+anzusehen/i,
      /in\s+den\s+herrschaftsbereich\s+des\s+kunden\s+gelangt/i,
      /(?:verz(?:ö|oe)gerungen|verluste)[^.!?;:\n]{0,80}?auf\s+dem\s+transportweg[^.!?;:\n]{0,60}?zu\s+lasten\s+des\s+kunden/i,
      /entfalten\s+ihre\s+wirkung[^.!?;:\n]{0,90}?unabh(?:ä|ae)ngig\s+vom\s+empfang/i,
      /unabh(?:ä|ae)ngig\s+vom\s+empfang\s+und\s+vom\s+verbleib\s+der\s+nachricht/i,
      /zustellung\s+ist\s+mit\s+absendung\s+bewirkt/i,
      /gelten?\s+mit\s+absendung\s+als\s+zugegangen/i,
      // Typo-tolerant (OCR 'A[bs|ss]endung' nach Cleanup der Zero-Width-/Soft-Hyphen-Tarnung)
      /gelten?\s+mit\s+a\w*endung\s+als\s+zugegangen/i,
    ],
    severity: "mittel",
    why: "Unzulässige Zugangsfiktion (§ 308 Nr. 5 BGB): Erklärungen, Preiserhöhungen oder Rechnungen sollen nach wenigen Tagen automatisch als zugegangen gelten, selbst wenn sie im Spam-Filter hängen bleiben oder auf dem Postweg verloren gehen.",
    suggestion: "Zugangsfiktion ersatzlos streichen. Maßgeblich muss stets der tatsächliche Zugang beim Empfänger (§ 130 BGB) sein.",
  },

  // 40. Illusory SLAs & Availability Disclaimers (§ 307 BGB)
  {
    type: "unverbindliche_sla_falle",
    title: "Scheingarantie & unverbindliche Service-Levels (SLA-Entkernung)",
    tags: ["bgb-307", "sla", "verfuegbarkeit", "scheingarantie", "leistungsentkernung"],
    patterns: [
      /(?:bemüht\s+sich|nach\s+besten\s+(?:wirtschaftlichen\s+)?kräften|commercially\s+reasonable\s+efforts)[^.!?;:\n]{0,100}?(?:verfügbarkeit|uptime|erreichbarkeit)/i,
      /(?:mittlere|angestrebte|durchschnittliche)\s+verfügbarkeit[^.!?;:\n]{0,80}?(?:zu\s+gewährleisten|anzustreben|als\s+ziel)/i,
      /(?:verfügbarkeit|verfügbarkeitsangaben?|service[\s-]levels?|sla)[^.!?;:\n]{0,120}?(?:(?:ist|sind)\s+unverbindlich|unverbindlich|stellen\s+keine\s+beschaffenheitsgarantie|bloße\s+zielgröße|begründen?\s+keine\s+ansprüche)/i,
      /(?:angestrebte|durchschnittliche)\s+verfügbarkeit[^.!?;:\n]{0,80}?(?:unverbindlich|ohne\s+rechtsanspruch|begründet\s+kein\s+minderungsrecht)/i,
      /keine\s+gewähr\s+für\s+(?:unterbrechungsfreie|durchgehende|jederzeitige)\s+verfügbarkeit/i,
      /(?:ansprüche|ersatzansprüche|minderung)[^.!?;:\n]{0,80}?(?:unterschreitung|nichteinhaltung|ausfall)[^.!?;:\n]{0,80}?(?:verfügbarkeit|service[\s-]levels?)[^.!?;:\n]{0,50}?(?:bestehen\s+nicht|ausgeschlossen)/i,
      /(?:unterschreitung\s+(?:dieser\s+)?verfügbarkeit)[^.!?;:\n]{0,60}?(?:bestehen\s+nicht|ausgeschlossen)/i,
      /(?:strebt|streben)\s+(?:eine\s+)?verfügbarkeit[^.!?;:\n]{0,60}?\ban\b/i,
      // Red Team: Euphemismus-/Synonym-/English-/Nominal-Evasion
      /verf(?:ü|ue)gbarkeitswerte[^.!?;:\n]{0,100}?(?:serviceziele|zielgr(?:ö|oe)(?:ß|ss)en|richtwerte)/i,
      // Kennzahlen zur Erreichbarkeit als rein interne Steuerung entkraeftet (Welle D):
      /(?:kennzahlen|kennwerte|werte|angaben)[^.!?;:\n]{0,80}?(?:erreichbarkeit|verf(?:ü|ue)gbarkeit|uptime|antwortzeit\w*)[^.!?;:\n]{0,140}?(?:sind\s+unverbindlich|dienen\s+(?:ausschlie(?:ß|ss)lich\s+|lediglich\s+)?der\s+internen|ohne\s+rechtsanspruch|begr(?:ü|ue)nden?\s+keine\s+anspr(?:ü|ue)che)/i,
      /(?:serviceziele|ziels?|richtwerte)[^.!?;:\n]{0,100}?keine\s+einklagbaren/i,
      /(?:angaben\s+zur\s+erreichbarkeit|erreichbarkeit\s+der\s+dienste)[^.!?;:\n]{0,120}?(?:unverbindliche\s+richtwerte|richtwerte\s+ohne\s+rechtsanspruch)/i,
      /richtwerte\s+ohne\s+rechtsanspruch/i,
      /(?:aspirational\s+targets|targets\s+only)[^.!?;:\n]{0,120}?(?:do\s+not\s+constitute\s+a\s+guarantee|give\s+rise\s+to)/i,
      /do\s+not\s+constitute\s+a\s+guarantee/i,
      /(?:betriebsbereitschaft|verf(?:ü|ue)gbarkeit|uptime)[^.!?;:\n]{0,80}?wird\s+nicht\s+zugesichert/i,
      /uptime-?werte[^.!?;:\n]{0,100}?kapazit(?:ä|ae)tsplanung/i,
      /(?:interne[n]?\s+kapazit(?:ä|ae)tsplanung|ausschlie(?:ß|ss)lich\s+der\s+internen\s+kapazit(?:ä|ae)tsplanung)/i,
    ],
    severity: "hoch",
    why: "SLA-Entkernung & Scheingarantie (§ 307 BGB): Der Anbieter wirbt mit hohen Verfügbarkeitswerten (z. B. 99,9 %), stuft diese im Kleingedruckten jedoch als rechtlich unverbindliche Zielgrößen ein, sodass bei tagelangen Ausfällen weder Schadensersatz noch Kündigungsrechte greifen.",
    suggestion: "Verbindliche Service-Levels mit konkreten Pönalen oder Gutschriften bei Nichteinhaltung sowie Sonderkündigungsrecht bei Unterschreitung von Mindestschwellen (z. B. unter 98 %) vereinbaren.",
  },

  // 41. Handelsvertreterprovision fuer Direktgeschaefte (§ 87 Abs. 2 HGB)
  {
    type: "agency_commission_exclusion",
    title: "Ausschluss von Provisionen auf Direktgeschäfte",
    tags: ["hgb-87", "handelsvertreter", "provision", "gebietsschutz"],
    patterns: [
      /(?:direktauftrag|direktgeschäft|direktverkauf)[^.!?;:\n]{0,150}(?:keine(?:r|n)?\s+provision|provision[^.!?;:\n]{0,70}(?:nicht|ausgeschlossen)|steht[^.!?;:\n]{0,50}keine\s+provision)/i,
      /(?:keine(?:r|n)?\s+provision|provision[^.!?;:\n]{0,70}(?:nicht|ausgeschlossen))[^.!?;:\n]{0,120}(?:direktauftrag|direktgeschäft|direktverkauf)/i,
    ],
    severity(s) {
      // HGB § 87 Abs. 2 betrifft zugewiesene Gebiete/Kundenkreise; eine ausdrücklich
      // auf Geschäfte außerhalb des zugewiesenen Gebiets begrenzte Ausnahme ist kein Treffer.
      if (/außerhalb.{0,60}(?:zugewiesenen|übertragenen)\s+(?:gebiet|bezirk|kundenkreis)/i.test(s)) return null;
      return "hoch";
    },
    why: "Die Klausel schließt Provisionen für bestimmte Direktgeschäfte aus. Bei Handelsvertretern kann ein Provisionsanspruch für Geschäfte im zugewiesenen Bezirk oder Kundenkreis auch ohne eigene Mitwirkung bestehen (§ 87 Abs. 2 HGB); ob das hier greift, hängt von Vertragsrolle und Zuweisung ab.",
    suggestion: "Prüfen und verhandeln Sie eine klare Provisionsregel für Direktgeschäfte im zugewiesenen Gebiet oder Kundenkreis sowie eine nachvollziehbare Abrechnung.",
  },
  // 42. Vorabverzicht auf den Handelsvertreterausgleich (§ 89b Abs. 4 HGB)
  {
    type: "agency_goodwill_waiver",
    title: "Ausschluss eines Ausgleichsanspruchs nach Vertragsende",
    tags: ["hgb-89b", "handelsvertreter", "kundenstamm", "ausgleich"],
    patterns: [
      /(?:kein(?:e|en)?\s+anspruch|anspruch[^.!?;:\n]{0,50}(?:besteht|entsteht)\s+nicht|verzicht\w*)[^.!?;:\n]{0,130}(?:ausgleich|kundenstamm|kundschaft)/i,
      /(?:ausgleich|kundenstamm|kundschaft)[^.!?;:\n]{0,130}(?:kein(?:e|en)?\s+anspruch|anspruch[^.!?;:\n]{0,40}(?:besteht|entsteht)\s+nicht|verzicht\w*)/i,
    ],
    severity: "hoch",
    why: "Die Klausel scheint einen Ausgleich für den vom Handelsvertreter aufgebauten Kundenstamm auszuschließen. Ein Anspruch nach § 89b HGB ist an Voraussetzungen gebunden und kann nicht im Voraus ausgeschlossen werden; die konkrete Berechtigung hängt vom Einzelfall ab.",
    suggestion: "Lassen Sie den Ausschluss streichen und vereinbaren Sie, dass ein möglicher gesetzlicher Ausgleichsanspruch nach Vertragsende unberührt bleibt.",
  },
  // 43. Datenverarbeitung oder Datenzugriff außerhalb EU/EWR — Prüfhinweis, kein Rechtsurteil
  {
    type: "international_data_processing",
    title: "Datenverarbeitung außerhalb von EU/EWR",
    tags: ["gdpr-chapter-v", "international-transfer", "data-location"],
    patterns: [
      /(?:bestands-,\s*nutzungs-|erhobenen?\s+daten|personenbezogen\w*\s+daten|kundendaten|messdaten|telemetriedaten|nutzungsdaten|gerätedaten|betriebsdaten|bewegungs-,\s+anwesenheits)[^.!?;:\n]{0,380}(?:außerhalb\s+(?:der\s+)?(?:(?:schweiz\s+und\s+(?:der\s+)?)?(?:europäischen\s+union|eu|ewr))|drittland\w*)/i,
      /(?:außerhalb\s+(?:der\s+)?(?:(?:schweiz\s+und\s+(?:der\s+)?)?(?:europäischen\s+union|eu|ewr))|drittland\w*)[^.!?;:\n]{0,380}(?:bestands-,\s*nutzungs-|erhobenen?\s+daten|personenbezogen\w*\s+daten|kundendaten|messdaten|telemetriedaten|nutzungsdaten|gerätedaten|betriebsdaten)/i,
    ],
    severity: "mittel",
    why: "Der Vertrag sieht eine Verarbeitung oder Nutzung von Daten außerhalb der EU/des EWR oder in einem Drittland vor. Das ist ein Prüfhinweis, keine automatische Rechtswidrigkeit: Es kommt unter anderem auf Datenart, Empfängerland und einen anwendbaren Übermittlungsmechanismus nach Art. 44–49 DSGVO an.",
    suggestion: "Klären Sie Datenarten, Empfänger, Speicher- und Zugriffsort sowie den Übermittlungsmechanismus (z. B. Angemessenheitsbeschluss oder geeignete Garantien) und regeln Sie Informations- und Widerspruchsrechte.",
  },
  // 44. Sachverständigenverfahren als Zugangsschranke zum Rechtsweg
  {
    type: "mandatory_expert_determination",
    title: "Sachverständigenverfahren als Voraussetzung für den Rechtsweg",
    tags: ["dispute-resolution", "expert-determination", "court-access"],
    patterns: [
      /(?:sachverständigenverfahren|gutachterliches?\s+verfahren)[\s\S]{0,700}(?:kosten[^.!?;:\n]{0,50}verfahren|verfahrenskosten)[\s\S]{0,160}(?:kunden|besteller)[\s\S]{0,100}(?:vorzustrecken|vorauszuzahlen)[\s\S]{0,500}(?:rechtsweg|gerichtlichen\s+rechtsweg|klage)[\s\S]{0,100}(?:erst\s+nach|erst\s+wenn|nach\s+abschluss)/i,
    ],
    collect(sentences, _full, extra = null) {
      const hits = [];
      const anchored = new Set();
      const anchorRe = /sachverst(?:ä|ae)ndig|gutachter|verfahrenskosten|rechtsweg|vorauszuzahlen|vorzustrecken/i;
      for (let i = 0; i < sentences.length; i++) {
        for (let end = i + 1; end < Math.min(i + 4, sentences.length); end++) {
          const window = sentences.slice(i, end + 1).map((sentence) => sentenceMatchText(sentence, extra)).join(" ");
          if (!this.patterns.some((p) => p.test(window))) continue;
          // Der Treffer muss auf dem Satz mit dem Sachverständigen-/Kostenbezug liegen, nicht auf
          // dem davor liegenden Satz, sonst wandert die Fundstelle in eine harmlose Klausel.
          let anchor = end;
          for (let k = end; k >= i; k--) {
            if (anchorRe.test(sentenceMatchText(sentences[k], extra))) { anchor = k; break; }
          }
          if (!anchored.has(anchor)) {
            anchored.add(anchor);
            hits.push({ i: anchor, severity: "mittel" });
          }
          break;
        }
      }
      return hits;
    },
    severity: "mittel",
    why: "Der Vertrag koppelt den Zugang zu staatlichen Gerichten an ein vorheriges Sachverständigenverfahren und legt dessen Vorfinanzierung dem Kunden auf. Ein gestuftes Verfahren ist nicht automatisch unzulässig, sollte aber hinsichtlich Kosten, Fristen, Auswahl und Eilrechtsschutz geprüft werden.",
    suggestion: "Begrenzen Sie das Verfahren auf klar definierte technische Fragen, teilen Sie die Kosten angemessen und erhalten Sie den Zugang zu Gerichten für Eilmaßnahmen sowie bei Verzögerung oder Verfahrensstillstand.",
  },
  // 45. Verfall von Service-Gutschriften wegen verspaeteter Sicherheitsupdates
  {
    type: "service_credit_forfeiture",
    title: "Verfall von Service-Gutschriften wegen Update-Frist",
    tags: ["service-level", "service-credit", "security-update", "customer-duty"],
    patterns: [
      /(?:kritisch\w*[^.!?;:\n]{0,35}aktualisier\w*|aktualisier\w*[^.!?;:\n]{0,45}kritisch\w*|sicherheitspatch\w*|security\s+patch\w*)[^.!?;:\n]{0,160}(?:entfällt|verfällt|erlischt|verliert)[^.!?;:\n]{0,90}(?:anspruch|gutschrift\w*|service[\s-]?credit\w*)/i,
      /(?:gutschrift\w*|service[\s-]?credit\w*)[^.!?;:\n]{0,100}(?:entfällt|verfällt|erlischt)[^.!?;:\n]{0,100}(?:aktualisier\w*|sicherheitspatch\w*|security\s+patch\w*)/i,
    ],
    severity: "mittel",
    why: "Service-Gutschriften können bei versäumten Sicherheitsupdate-Fristen entfallen, auch wenn sie andere Leistungsausfälle betreffen. Prüfen Sie Frist, Verantwortlichkeit, Abhängigkeiten und ob bereits verdiente Gutschriften betroffen sind.",
    suggestion: "Legen Sie Update-Fristen nach Kritikalität fest, berücksichtigen Sie vom Anbieter oder Dritten verursachte Verzögerungen und gewähren Sie eine angemessene Nachfrist, bevor Gutschriften verfallen.",
  },
  // 46. Pauschale Zahlung bei vorzeitiger Vertragsbeendigung
  {
    type: "early_termination_charge",
    title: "Pauschale Zahlung bei vorzeitiger Beendigung",
    tags: ["termination", "lump-sum", "mitigation", "bgb-307"],
    patterns: [
      /(?:vorzeitig\w*[^.!?;:\n]{0,55}(?:beendigung|auflösung|kündigung)|(?:beendigung|auflösung|kündigung)[^.!?;:\n]{0,55}vorzeitig\w*)[^.!?;:\n]{0,170}(?:pauschal\w*[^.!?;:\n]{0,45}(?:abstandszahl\w*|beendigungsentgelt|storno\w*gebühr)|(?:abstandszahl\w*|beendigungsentgelt|storno\w*gebühr)[^.!?;:\n]{0,80}pauschal\w*)/i,
    ],
    severity: "mittel",
    why: "Bei vorzeitiger Beendigung wird eine pauschale Zahlung verlangt. Prüfen Sie, ob sie unabhängig von Kündigungsgrund, ersparten Aufwendungen und tatsächlichem Schaden anfällt oder eine Doppelkompensation ermöglicht; die Wirksamkeit ist kontextabhängig.",
    suggestion: "Begrenzen Sie die Zahlung auf einen nachvollziehbaren, nicht doppelt ersatzfähigen Betrag und ermöglichen Sie den Nachweis geringeren oder fehlenden Schadens, soweit anwendbar.",
  },
  // 47. Abschaltung eines Systems wegen fehlender Wartungsvereinbarung
  {
    type: "maintenance_conditional_shutdown",
    title: "Abschaltung bei fehlendem Wartungsvertrag",
    tags: ["service-continuity", "maintenance", "shutdown", "termination"],
    patterns: [
      /(?:anlage|system|software|dienst|zugang|betrieb)[^.!?;:\n]{0,150}(?:ab(?:zu)?schalt\w*|stillleg\w*|außer\s+betrieb\s+setz\w*)[^.!?;:\n]{0,100}(?:wenn|sofern|falls|mangels)[^.!?;:\n]{0,80}(?:wartungsvertrag\w*|wartungsvereinbarung\w*|maintenance\s+agreement)/i,
      /(?:wartungsvertrag\w*|wartungsvereinbarung\w*|maintenance\s+agreement)[^.!?;:\n]{0,130}(?:ab(?:zu)?schalt\w*|stillleg\w*|außer\s+betrieb\s+setz\w*)/i,
    ],
    severity: "mittel",
    why: "Der Anbieter kann ein System oder eine Anlage abschalten, wenn kein Wartungsvertrag besteht. Prüfen Sie, ob Wartung wirklich Voraussetzung für einen sicheren Betrieb ist und ob die Abschaltung verhältnismäßig, angekündigt und mit Datenzugriff oder Übergangshilfe verbunden ist.",
    suggestion: "Vereinbaren Sie eine angemessene Vorankündigung und Nachfrist, klare Sicherheitsgründe für eine Abschaltung sowie einen sicheren Datenexport und eine Übergangslösung.",
  },
];

// Red-Team-Nachtrag: englischsprachige Klauseltexte, Synonyme und
// umschriebene Formen aus dem Vertraege-Korpus, die das Register umgehen.
const PATTERN_SUPPLEMENTS = {
  penalty: [
    /(?:contractual\s+penalt(?:y|ies)|liquidated\s+damages|penalt(?:y|ies)\s+of\s+[\d.,]+\s*(?:eur|usd|euro|dollars?))/i,
    /nachweis[^.!?;:\n]{0,140}?nicht\s+vorbehalten/i,
    /(?:gegenbeweis|nachweis)[^.!?;:\n]{0,120}?(?:geringeren|niedrigeren|wesentlich\s+geringeren)\s+(?:schadens|aufwands)[^.!?;:\n]{0,60}?nicht/i,
    /(?:r(?:ü|ue)cklastschrift|lastschrift|mahnung|verzug)[^.!?;:\n]{0,80}?(?:bearbeitungspauschale|mahngeb(?:ü|ue)hr|bearbeitungsgeb(?:ü|ue)hr)[^.!?;:\n]{0,60}?\d+[\d,.]*\s*(?:eur|euro)/i,
  ],
  jurisdiction: [
    /(?:courts?|venue|submits?)[^.!?;:\n]{0,90}?exclusive\s+jurisdiction/i,
    /exclusive\s+jurisdiction\s+(?:of|in)\s+(?:the\s+courts?|any\s+court)/i,
    /(?:brought|submitted|resolved|litigated)[^.!?;:\n]{0,70}?exclusively\s+before\s+the\s+courts?/i,
    /exclusively\s+before\s+the\s+courts?\s+(?:at|of|in)\s+the\s+(?:provider|supplier|vendor)/i,
  ],
  non_compete: [
    /shall\s+not[^.!?;:\n]{0,60}?(?:engage\s+in|carry\s+on|be\s+involved\s+in|participate\s+in)[^.!?;:\n]{0,60}?compet(?:ing|itive)/i,
    /competing\s+business|\bnon[\s-]?compete\b/i,
    /(?:untersagen\s+sich|verpflichtet\s+sich)[^.!?;:\n]{0,60}?jede[^.!?;:\n]{0,40}?werbende\s+t(?:ä|ae)tigkeit/i,
  ],
  excessive_survival: [
    /survive[^.!?;:\n]{0,80}?(?:indefinitely|in\s+perpetuity|without\s+(?:any\s+)?(?:time\s+)?limitation)/i,
  ],
  liability: [
    /wird\s+nicht\s+gehaftet|haftung[^.!?;:\n]{0,60}?ist\s+nicht\s+er(?:ö|oe)ffnet/i,
    /(?:liability|damages)[^.!?;:\n]{0,60}?(?:is\s+|shall\s+be\s+)?(?:excluded|disclaimed|not\s+assumed)/i,
    /shall\s+not\s+be\s+liable\s+for\s+(?:any\s+)?(?:loss|damages|claims|costs|indirect|consequential)/i,
    /all\s+risk\s+of\s+the\s+services\s+remains\s+with\s+the\s+customer/i,
    /widerrufsrecht[^.!?;:\n]{0,120}?(?:verzicht\w*|unwiderruflich|ausgeschlossen|besteht\s+nicht|entf\u00e4llt)/i,
    /(?:verzicht\w*|entsag\w*)[^.!?;:\n]{0,120}?widerrufsrecht/i,
    /(?:unwiderruflich\w*|ausdr\u00fccklich\w*)\s+verzicht\w*[^.!?;:\n]{0,120}?(?:widerruf|widerrufsrecht)/i,
  ],
  setoff_restriction: [
    /gehindert[^.!?;:\n]{0,60}?(?:aufzurechnen|zur\s+aufrechnung|zur(?:ü|ue)ckzubehalten)/i,
    /(?:nicht\s+gestattet|untersagt|verwehrt|nicht\s+befugt)[^.!?;:\n]{0,60}?(?:aufzurechnen|zur(?:ü|ue)ckzubehalten)/i,
    /aufrechnung[^.!?;:\n]{0,80}?(?:untersagt|verwehrt|verweigert|nicht\s+gestattet|verboten)/i,
    /nur\s+mit\s+unbestrittenen\s+forderungen[^.!?;:\n]{0,80}?(?:verrechnen|aufrechnen)/i,
    /(?:anbieter|auftragnehmer|dienstleister)\s+bleibt\s+zur\s+(?:verrechnung|aufrechnung)[^.!?;:\n]{0,60}?befugt/i,
  ],
  auto_renewal: [
    /(?:l(?:ä|ae)uft|verl(?:ä|ae)ngert\s+sich|gilt)[^.!?;:\n]{0,90}?um\s+(?:\d{1,2}|zwei|drei|sechs|zw(?:ö|oe)lf|ein|eine)\s+weitere?[sn]?\s+(?:monate|monat|jahr|jahre|woche|wochen)/i,
    /nur\s+der\s+kunde\s+muss[^.!?;:\n]{0,90}?(?:beendigung|k(?:ü|ue)ndigung)[^.!?;:\n]{0,60}?(?:erkl(?:ä|ae)ren|aussprechen|mitteilen)/i,
    /andernfalls\s+gilt[^.!?;:\n]{0,60}?als\s+f(?:ü|ue)r\s+(?:ein|ein\s+weiteres|zw(?:ö|oe)lf|\d+)\s+(?:jahr|jahre|monate)/i,
    /automatisch\s+(?:freigeschaltet|aktiviert|gebucht|abgerechnet)[^.!?;:\n]{0,140}?(?:sofern|wenn|falls)[^.!?;:\n]{0,90}?widersprochen/i,
    /(?:kennenlernzeitraum|testphase|probezeitraum|kennenlernphase)[^.!?;:\n]{0,140}?automatisch[^.!?;:\n]{0,60}?(?:freigeschaltet|aktiviert|gebucht|abgerechnet)/i,
  ],
  data_grant: [
    /soweit\s+(?:es\s+)?(?:dem\s+anbieter\s+)?(?:zweckm(?:ä|ae)(?:ß|ss)ig|angemessen)\s+erscheint[^.!?;:\n]{0,90}?daten[^.!?;:\n]{0,60}?(?:nutzen|verwenden|heranziehen)/i,
    /daten[^.!?;:\n]{0,60}?f(?:ü|ue)r\s+(?:produktverbesserung|produktentwicklung|eigene\s+zwecke|eigene\s+nutzung)[^.!?;:\n]{0,60}?(?:nutzen|verwenden|heranziehen|verarbeiten)/i,
    /(?:datenverwendung|datennutzung)[^.!?;:\n]{0,90}?anlage[^.!?;:\n]{0,90}?(?:nutzung\s+zu\s+eigenen|gestattet|eigene\s+zwecke)/i,
    /(?:gesundheits|bewegungs|standort)[^.!?;:\n]{0,80}?daten[^.!?;:\n]{0,140}?(?:partnerunternehmen|datenh(?:ä|ae)ndler|werbedienstleister|versicherung)/i,
    /einwilligung[^.!?;:\n]{0,140}?ist\s+voraussetzung\s+f(?:ü|ue)r\s+die\s+vertragsdurchf(?:ü|ue)hrung/i,
    /einwilligung[^.!?;:\n]{0,160}?nur\s+zusammen\s+mit\s+dem\s+vertrag\s+widerrufen/i,
  ],
  vicarious_agents: [
    /(?:verantwortung|haftung|einstandspflicht)\s+nur\s+bei[^.!?;:\n]{0,50}?(?:nachweisbarer\s+)?(?:fehlauswahl|auswahlverschulden)/i,
    /(?:kunde|auftraggeber|besteller)\s+haftet\s+f(?:ü|ue)r\s+das\s+verhalten[^.!?;:\n]{0,80}?(?:erf(?:ü|ue)llungsgehilfen|subunternehmer|nachunternehmer)/i,
    /(?:beauftragten|eingesetzten)\s+(?:dienstleister|unternehmen)[^.!?;:\n]{0,90}?verantwortung[^.!?;:\n]{0,40}?nur\s+bei/i,
  ],
  deemed_acceptance: [
    /(?:gilt|gelten)\s+als\s+(?:genehmigt|anerkannt|abgenommen)[^.!?;:\n]{0,90}?(?:wenn|sofern|falls|es\s+sei\s+denn)[^.!?;:\n]{0,60}?nicht/i,
    /(?:keine|ohne)\s+(?:rechtzeitige\s+)?(?:r(?:ü|ue)ge|beanstandung|widerspruch)[^.!?;:\n]{0,90}?(?:gilt|gelten)\s+als\s+(?:genehmigt|anerkannt|abgenommen)/i,
    /(?:48|achtundvierzig|zweiundsiebzig|siebzig|vierzehn|zehn)\s+stunden[^.!?;:\n]{0,90}?(?:r(?:ü|ue)ge|beanstandung|widerspruch)/i,
    /(?:gilt|gelten)\s+als\s+anerkannt[^.!?;:\n]{0,60}?betriebskosten/i,
    /(?:preis(?:ä|ae)nderungen|preisanpassungen|entgelt(?:ä|ae)nderungen)[^.!?;:\n]{0,140}?gelten\s+als\s+(?:genehmigt|vereinbart|akzeptiert)/i,
    /durch\s+ver(?:ö|oe)ffentlichung[^.!?;:\n]{0,140}?gelten\s+als\s+(?:genehmigt|vereinbart|akzeptiert)/i,
    /(?:gilt|gelten)[^.!?;:\n]{0,140}?als\s+(?:ordnungsgem(?:ä|ae)(?:ß|ss)\s+und\s+)?(?:vollst(?:ä|ae)ndig\s+und\s+)?(?:mangelfrei\s+)?(?:abgenommen|genehmigt|anerkannt)/i,
    /(?:nicht\s+innerhalb\s+von\s+(?:\d+|zwei|drei|achtundvierzig|dreissig|30)\s*(?:minuten|stunden|werktagen|tagen)|ausschlussfrist\s+von\s+(?:\d+|zehn|f(?:ü|ue)nf)\s+werktagen)[^.!?;:\n]{0,120}?(?:gilt|gelten|genehmigt|abgenommen|beanstandung|m(?:ä|ae)ngel)/i,
  ],
  written_form_trap: [
    /(?:papierform|schriftform)[^.!?;:\n]{0,100}?w(?:ä|ae)hrend[^.!?;:\n]{0,90}?(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,80}?(?:app|nachricht|e-?mail|textform|m(?:ü|ue)ndlich)/i,
    /(?:m(?:ü|ue)ndliche|telefonische)\s+(?:abreden|vereinbarungen|zusagen)[^.!?;:\n]{0,60}?bed(?:ü|ue)rfen[^.!?;:\n]{0,40}?schriftform/i,
  ],
  burden_of_proof: [
    /(?:den\s+nachweis|die\s+beweislast)[^.!?;:\n]{0,60}?(?:f(?:ü|ue)hrt|tr(?:ä|ae)gt|obliegt)\s+(?:der\s+)?(?:kunde|besteller|mieter|k(?:ä|ae)ufer)/i,
    /der\s+(?:kunde|besteller|k(?:ä|ae)ufer)\s+hat[^.!?;:\n]{0,70}?nachzuweisen/i,
    /best(?:ä|ae)tigt[^.!?;:\n]{0,140}?zur\s+kenntnis\s+genommen/i,
    /(?:best(?:ä|ae)tigt|erkl(?:ä|ae)rt)[^.!?;:\n]{0,100}?(?:s(?:ä|ae)mtliche|alle)\s+(?:hinweise|bedingungen|unterlagen|informationen)[^.!?;:\n]{0,80}?erhalten/i,
    /(?:abweichend\s+von\s+(?:den\s+gesetzlichen\s+regeln|§\s*\d+)[^.!?;:\n]{0,80}?(?:tr(?:ä|ae)gt|obliegt)[^.!?;:\n]{0,60}?(?:beweislast|gefahr))/i,
    /(?:gefahr\s+des\s+zuf(?:ä|ae)lligen\s+untergangs|gefahr(?:ü|ue)bergang)[^.!?;:\n]{0,120}?(?:bereits\s+in\s+dem\s+zeitpunkt|fertigungshalle|vor\s+abnahme|vor\s+(?:ü|ue)bergabe)/i,
    /(?:tr(?:ä|ae)gt|obliegt)[^.!?;:\n]{0,60}?(?:ausschlie(?:ß|ss)lich\s+)?(?:der\s+)?(?:auftraggeber|kunde|besteller|mieter|lieferant)[^.!?;:\n]{0,40}?(?:die\s+)?(?:volle\s+)?beweislast/i,
  ],
  asymmetric_termination: [
    /(?:anbieter|vermieter|auftragnehmer)[^.!?;:\n]{0,60}?kann\s+den\s+vertrag\s+mit\s+sofortiger\s+wirkung\s+k(?:ü|ue)ndigen[^.!?;:\n]{0,100}?kunde/i,
    /dem\s+kunden\s+steht\s+ein\s+solches\s+recht\s+nur\s+bei/i,
    /(?:ordentliche\s+k(?:ü|ue)ndigung|beendigung)\s+steht\s+ausschlie(?:ß|ss)lich\s+(?:der\s+)?(?:anbieterin|anbieter|vermieterin|vermieter)\s+zu/i,
  ],
  force_majeure_asymmetry: [
    /ereignisse\s+h(?:ö|oe)herer\s+gewalt\s+befreien[^.!?;:\n]{0,60}?(?:den\s+anbieter|den\s+auftragnehmer)/i,
    /h(?:ö|oe)here[r]?\s+gewalt[^.!?;:\n]{0,140}?den\s+kunden\s+treffen\s+sie\s+ohne\s+entlastung/i,
    /h(?:ö|oe)here[r]?\s+gewalt[^.!?;:\n]{0,120}?verl(?:ä|ae)ngern\s+dessen\s+fristen[^.!?;:\n]{0,80}?zahlungspflichten[^.!?;:\n]{0,40}?laufen\s+unver(?:ä|ae)ndert/i,
    /(?:naturereignis|naturkatastrophe|unwetter)[^.!?;:\n]{0,140}?zahlungspflicht[^.!?;:\n]{0,60}?(?:bleibt|besteht)/i,
  ],
  extended_retention_of_title: [
    /eigentum[^.!?;:\n]{0,140}?(?:alle[nr]?|s(?:ä|ae)mtliche[nr]?)\s+forderungen\s+aus\s+der\s+gesch(?:ä|ae)ftsbeziehung/i,
    /eigentumsvorbehalt[^.!?;:\n]{0,130}?bis\s+(?:auch\s+)?(?:zur\s+)?(?:vollst(?:ä|ae)ndigen\s+)?(?:zahlung|begleichung)[^.!?;:\n]{0,90}?(?:aller|s(?:ä|ae)mtlicher)\s+forderungen/i,
  ],
  subcontractor_uncontrolled: [
    /(?:ohne\s+(?:vorherige\s+)?zustimmung|ohne\s+information|ohne\s+ank(?:ü|ue)ndigung|zustimmung\s+bedarf\s+es\s+nicht)[^.!?;:\n]{0,90}?(?:auf\s+dritte|auf\s+verbundene|auf\s+unternehmen|zu\s+(?:ü|ue)bertragen|zu\s+beauftragen)/i,
    /(?:setzt|greift|bedient\s+sich)[^.!?;:\n]{0,80}?(?:dienstleister|dritte|verbundene\s+gesellschaften|nachunternehmer)[^.!?;:\n]{0,90}?(?:ohne\s+(?:deren\s+)?(?:auswahl|zustimmung|information|mitteilung)|mitzuteilen)/i,
    /(?:ohne\s+(?:vorherige\s+)?(?:information|kenntnis|wissen)\s+oder\s+zustimmung|ohne\s+wissen\s+oder\s+zustimmung)[^.!?;:\n]{0,100}?(?:subunternehmer|nachunternehmer|dritte|externe|beliebige|weitervergeben|weiterzugeben|(?:ü|ue)bertragen)/i,
    /(?:berechtigt|darf)[^.!?;:\n]{0,120}?(?:leistungen|bauauftrag|objektbewachung|beratungsleistungen|reinigungs-?\s*(?:und\s+hausmeister)?leistungen)[^.!?;:\n]{0,120}?(?:ohne\s+(?:vorherige\s+)?(?:information|kenntnis|wissen)\s+oder\s+zustimmung|ohne\s+zustimmung)[^.!?;:\n]{0,80}?(?:an\s+beliebige|subunternehmer|nachunternehmer|dritte)/i,
  ],
  audit_log_manipulation: [
    /(?:protokoll|aufzeichnung|log)[^.!?;:\n]{0,80}?(?:nachtr(?:ä|ae)glich|r(?:ü|ue)ckwirkend|jederzeit)[^.!?;:\n]{0,60}?(?:anpassen|(?:ä|ae)ndern|berichtigen|korrigieren|l(?:ö|oe)schen)/i,
    /(?:nachtr(?:ä|ae)glich|r(?:ü|ue)ckwirkend)[^.!?;:\n]{0,60}?(?:anpassen|(?:ä|ae)ndern|berichtigen|korrigieren)[^.!?;:\n]{0,70}?(?:protokoll|aufzeichnung)/i,
    /(?:protokolldaten|aufzeichnungen)[^.!?;:\n]{0,90}?(?:ma(?:ß|ss)geblich|vollbeweis|gegenbeweis\s+ausgeschlossen)/i,
  ],
  service_suspension: [
    /(?:zugang|leistung|einfahrt|zufahrt)[^.!?;:\n]{0,90}?(?:unterbrechen|sperren|aussetzen|einstellen)[^.!?;:\n]{0,110}?(?:zweifel|zahlungsf(?:ä|ae)higkeit|zahlungsverzug)/i,
    /(?:sperrt|sperren|unterbricht|unterbrechen)[^.!?;:\n]{0,60}?(?:zugang|zufahrt|leistung)[^.!?;:\n]{0,90}?(?:ohne\s+(?:vorherige\s+)?(?:ank(?:ü|ue)ndigung|fristsetzung)|sofort)/i,
    /(?:anschluss|leistung)[^.!?;:\n]{0,60}?(?:sofort|ohne\s+vorherige\s+androhung)[^.!?;:\n]{0,60}?(?:vollst(?:ä|ae)ndig\s+)?zu\s+sperren/i,
  ],
  insolvency_termination: [
    /(?:insolvenz|insolvenzverfahren|konkurs)[^.!?;:\n]{0,110}?(?:beendet|endet|k(?:ü|ue)ndigen|kann\s+beendet)/i,
    /(?:antrag|er(?:ö|oe)ffnung)[^.!?;:\n]{0,70}?insolvenz[^.!?;:\n]{0,90}?(?:beendet|endet|gestellt|sofort)/i,
  ],
  as_is_software_disclaimer: [
    /(?:keine\s+haftung|ohne\s+gew(?:ä|ae)hrleistung)[^.!?;:\n]{0,80}?daf(?:ü|ue)r,\s+dass[^.!?;:\n]{0,100}?(?:entspricht|vorstellungen|erwartungen|unterbrechungsfrei)/i,
    /(?:wie\s+gesehen|ist-zustand)[^.!?;:\n]{0,90}?(?:ohne\s+gew(?:ä|ae)hrleistung|(?:ü|ue)berlassen|bereitgestellt)/i,
    /gew(?:ä|ae)hrleistung[^.!?;:\n]{0,90}?(?:ist\s+)?ausgeschlossen/i,
    /anspruch\s+auf\s+(?:beseitigung|nachbesserung|nacherf(?:ü|ue)llung)[^.!?;:\n]{0,60}?besteht\s+nicht/i,
  ],
  marketing_reference: [
    /(?:verweist|verweisen)[^.!?;:\n]{0,70}?(?:au(?:ß|ss)endarstellung|werbung|referenz)[^.!?;:\n]{0,90}?zusammenarbeit/i,
    /widerspruch\s+des\s+kunden[^.!?;:\n]{0,70}?(?:erst\s+nach|nur\s+nach)[^.!?;:\n]{0,50}?(?:ver(?:ö|oe)ffentlichung|nutzung)/i,
    /(?:als\s+)?referenzkunde\w*n?[^.!?;:\n]{0,90}?(?:f(?:ü|ue)hren|nennen|ver(?:ö|oe)ffentlichen)/i,
    /(?:zustimmung|einwilligung)\s+des\s+kunden[^.!?;:\n]{0,40}?nicht\s+erforderlich[^.!?;:\n]{0,60}?referenz/i,
  ],
  unverbindliche_sla_falle: [
    /(?:strebt|bem(?:ü|ue)ht\s+sich)[^.!?;:\n]{0,100}?(?:reaktionszeit|servicezeit|servicelevel|verf(?:ü|ue)gbarkeit)[^.!?;:\n]{0,130}?(?:keine\s+rechte|keine\s+anspr(?:ü|ue)che|unverbindlich|nicht\s+verbindlich)/i,
    /(?:ü|ue)berschreitung[^.!?;:\n]{0,80}?entstehen\s+dem\s+kunden\s+keine\s+rechte/i,
  ],
  agency_commission_exclusion: [
    /provision\w*[^.!?;:\n]{0,110}?(?:entfallen|verwirkt|nicht\s+geschuldet)[^.!?;:\n]{0,90}?(?:nachweis|frist|nicht\s+innerhalb|drei\s+monate)/i,
    /(?:provision|stornoreserve)\w*[^.!?;:\n]{0,90}?nicht\s+innerhalb\s+von\s+(?:\d+|drei|zwei|sechs)\s+monaten/i,
  ],
  service_credit_forfeiture: [
    /gutschrift\w*[^.!?;:\n]{0,120}?(?:innerhalb\s+(?:eines|einer|von)\s+\w+\s*(?:monats|monat|wochen|tagen)|\d+\s*(?:monaten|wochen|tagen))[^.!?;:\n]{0,100}?(?:anzumelden|geltend\s+zu\s+machen|mitzuteilen|entfallen)/i,
    /(?:entfallen|verfallen)\s+(?:sie|die\s+gutschrift|die\s+anspr(?:ü|ue)che)/i,
  ],
  early_termination_charge: [
    /(?:restentgelt|restverg(?:ü|ue)tung|restliche[nr]?\s+(?:raten|entgelte|zahlungen))[^.!?;:\n]{0,100}?(?:geschuldet|f(?:ä|ae)llig|zu\s+zahlen|abzugsfrei|ohne\s+abzug)/i,
    /(?:vorzeitig|vorzeitiger)[^.!?;:\n]{0,60}?(?:aufl(?:ö|oe)sung|beendigung|k(?:ü|ue)ndigung)[^.!?;:\n]{0,130}?(?:rest|ohne\s+abzug|abstand)[^.!?;:\n]{0,60}?(?:geschuldet|f(?:ä|ae)llig)/i,
    /(?:bearbeitung|abwicklung|ausspruch)[^.!?;:\n]{0,70}?k(?:ü|ue)ndigung[^.!?;:\n]{0,90}?(?:verwaltungsgeb(?:ü|ue)hr|bearbeitungsgeb(?:ü|ue)hr|pauschale|kosten|entgelt)/i,
    /geb(?:ü|ue)hr[^.!?;:\n]{0,60}?f(?:ü|ue)r\s+die\s+(?:bearbeitung|abwicklung)[^.!?;:\n]{0,40}?einer\s+k(?:ü|ue)ndigung/i,
    /(?:100\s*(?:%|prozent)|\d{2}\s*(?:%|prozent))[^.!?;:\n]{0,140}?(?:gesamtverg(?:ü|ue)tung|grundpreise|restlaufzeit|restlichen\s+raten|ausstehenden\s+raten|pauschalentsch(?:ä|ae)digung)[^.!?;:\n]{0,120}?(?:ohne\s+dass\s+ein\s+abzug|abzug\s+ersparter\s+aufwendungen\s+ausgeschlossen|ohne\s+abzug|sofort\s+(?:in\s+einer\s+summe\s+)?zur\s+zahlung\s+f(?:ä|ae)llig)/i,
    /(?<![a-z\u00e4\u00f6\u00fc\u00df])(?:pauschalentsch(?:ä|ae)digung|entsch(?:ä|ae)digung|schadensersatz)[^.!?;:\n]{0,80}?(?:in\s+h(?:ö|oe)he\s+von\s+)?(?:\d{2,3}|hundert)\s*(?:%|prozent)[^.!?;:\n]{0,80}?(?:gesamtverg(?:ü|ue)tung|werklohn|verg(?:ü|ue)tung)/i,
  ],
  maintenance_conditional_shutdown: [
    /(?:nach\s+verf(?:ü|ue)gbarkeit\s+(?:freier\s+)?(?:kapazit(?:ä|ae)ten|ressourcen)|ohne\s+(?:g(?:ü|ue)ltigen\s+|erweitertes\s+)?(?:wartungsvertrag|servicepaket|wartungspaket))/i,
    /(?:instandhaltungsvertrag|wartungsvertrag)[^.!?;:\n]{0,110}?(?:verpflichtend|voraussetzung|zwingend)/i,
  ],
  price_escalation: [
    /(?:kosten|preise|entgelte|verg(?:ü|ue)tung)[^.!?;:\n]{0,90}?(?:steigen|erh(?:ö|oe)hen\s+sich|erh(?:ö|oe)ht\s+sich)[^.!?;:\n]{0,110}?(?:vollumf(?:ä|ae)nglich|vollst(?:ä|ae)ndig|eins\s+zu\s+eins)[^.!?;:\n]{0,70}?weiter/i,
    /obergrenze[^.!?;:\n]{0,70}?ist\s+nicht\s+(?:vereinbart|vorgesehen|geregelt)/i,
    /(?:erh(?:ö|oe)ht\s+sich|steigt|steigen)[^.!?;:\n]{0,70}?(?:j(?:ä|ae)hrlich|zum\s+jahreswechsel|jeweils\s+zum)[^.!?;:\n]{0,90}?um\s+(?:bis\s+zu\s+)?(?:\d{1,2}(?:[.,]\d+)?|f(?:ü|ue)nf|zehn|acht|zw(?:ö|oe)lf|f(?:ü|ue)nfzehn)\s*(?:%|prozent)/i,
  ],
  tax_indemnity_shift: [
    /(?:grunderwerbsteuer|steuern|abgaben|steuerliche\s+pflichten)[^.!?;:\n]{0,110}?tr(?:ä|ae)gt\s+(?:wirtschaftlich\s+)?(?:der\s+)?(?:kunde|k(?:ä|ae)ufer|mieter|besteller)/i,
    /(?:nachtr(?:ä|ae)gliche\s+)?(?:belastungen|steuern|abgaben)[^.!?;:\n]{0,90}?(?:an\s+den\s+kunden\s+weitergegeben|dem\s+kunden\s+auferlegt)/i,
  ],
  unilateral_change: [
    /(?:werden|wird)[^.!?;:\n]{0,70}?mit\s+der\s+ver(?:ö|oe)ffentlichung\s+wirksam/i,
    /(?:kann|darf)[^.!?;:\n]{0,70}?(?:jederzeit|nach\s+billigem\s+ermessen)[^.!?;:\n]{0,70}?(?:(?:ä|ae)ndern|anpassen|neu\s+festsetzen|einseitig\s+bestimmen)/i,
    /(?:bedingungswerk|vertragsbedingungen|allgemeinen\s+bedingungen|agb)[^.!?;:\n]{0,100}?jederzeit\s+zu\s+(?:ä|ae)ndern/i,
    /ge(?:ä|ae)nderten?\s+(?:bedingungen|bedingungswerk)[^.!?;:\n]{0,100}?ver(?:ö|oe)ffentlicht/i,
    /gelten\s+mit\s+der?\s+ver(?:ö|oe)ffentlichung\s+als\s+(?:vereinbart|genehmigt|akzeptiert)/i,
    /austausch\s+von\s+(?:baustoffen|materialien|komponenten)|auszutauschen[^.!?;:\n]{0,80}?ohne\s+dass\s+sich\s+der[^.!?;:\n]{0,40}?werklohn/i,
  ],
  unreasonable_audit: [
    /(?:jederzeit|ohne\s+(?:vorherige\s+)?ank(?:ü|ue)ndigung|ohne\s+anlass)[^.!?;:\n]{0,90}?(?:inspizieren|betreten|pr(?:ü|ue)fen|einblick\s+nehmen|kontrollieren)/i,
    /(?:einblick\s+nehmen|inspizieren)[^.!?;:\n]{0,90}?soweit\s+(?:er|sie)\s+dies\s+f(?:ü|ue)r\s+angezeigt\s+h(?:ä|ae)lt/i,
    /(?:pr(?:ü|ue)fung|audit|kontrolle)[^.!?;:\n]{0,90}?(?:ohne\s+(?:vorherige\s+)?terminabstimmung|unmittelbar|jederzeit)/i,
  ],
  statute_of_limitations: [
    /verj(?:ä|ae)hr\w*[^.!?;:\n]{0,90}?(?:ausschlie(?:ß|ss)lich\s+innerhalb|innerhalb\s+von\s+(?:\d+|[a-z(?:ä|ae)(?:ö|oe)(?:ü|ue)]{4,20})\s*monaten)/i,
    /(?:achtzehn|zw(?:ö|oe)lf|sechsunddrei(?:ß|ss)ig|zwanzig|sechs|drei)\s+monaten[^.!?;:\n]{0,70}?verj(?:ä|ae)hr/i,
  ],
  entire_agreement: [
    /fr(?:ü|ue)here\s+(?:zusagen|absprachen|vereinbarungen|abreden)[^.!?;:\n]{0,90}?(?:gegenstandslos|unwirksam|treten\s+au(?:ß|ss)er\s+kraft)/i,
    /(?:m(?:ü|ue)ndliche|telefonische)\s+(?:zusagen|abreden|absprachen)[^.!?;:\n]{0,90}?(?:entfalten\s+keine\s+(?:rechtliche\s+)?wirkung|sind\s+unwirksam|gelten\s+nicht)/i,
    /(?:m(?:ü|ue)ndliche|telefonische)?\s*nebenabreden[^.!?;:\n]{0,90}?(?:bestehen\s+nicht|sind\s+nicht\s+wirksam)/i,
    /(?:zusagen|abreden)[^.!?;:\n]{0,60}?(?:von\s+)?(?:vertriebsmitarbeitern|mitarbeitern)[^.!?;:\n]{0,60}?entfalten\s+keine/i,
    /nebenabreden[^.!?;:\n]{0,110}?(?:nur\s+(?:dann|g(?:ü|ue)ltig)|bed(?:ü|ue)rfen\s+der\s+schriftform)/i,
    /(?:s(?:ä|ae)mtliche\s+(?:vorvertraglichen|im\s+rahmen\s+der|bei\s+der)[^.!?;:\n]{0,140}?(?:zusagen|ausk(?:ü|ue)nfte|absprachen|abreden|vereinbarungen|protokollnotizen)[^.!?;:\n]{0,120}?(?:treten|werden)[^.!?;:\n]{0,60}?(?:au(?:ß|ss)er\s+kraft|gegenstandslos|unwirksam))/i,
    /(?:enth(?:ä|ae)lt|geben|regelt)[^.!?;:\n]{0,80}?(?:s(?:ä|ae)mtliche\s+vereinbarungen|das\s+vertragsverh(?:ä|ae)ltnis)[^.!?;:\n]{0,50}?(?:abschlie(?:ß|ss)end|vollst(?:ä|ae)ndig\s+wieder)[^.!?;:\n]{0,160}?(?:au(?:ß|ss)er\s+kraft|gegenstandslos|unwirksam|keine\s+geltung)/i,
  ],
  assignment_restriction: [
    /(?:kann|k(?:ö|oe)nnen)[^.!?;:\n]{0,50}?(?:anspr(?:ü|ue)che|rechte|forderungen)[^.!?;:\n]{0,50}?nicht\s+abtreten/i,
    /(?:abtretung|abtretbar|(?:ü|ue)bertragung)[^.!?;:\n]{0,90}?nur\s+mit\s+(?:zustimmung|billigung|genehmigung)\s+des\s+anbieters/i,
    /(?:rechte|anspr(?:ü|ue)che)[^.!?;:\n]{0,70}?nur\s+mit\s+billigung[^.!?;:\n]{0,50}?(?:auf\s+dritte|(?:ü|ue)bergehen)/i,
  ],
  unilateral_convenience_termination: [
    /(?:nicht\s+mehr\s+wirtschaftlich|nicht\s+mehr\s+rentabel|aus\s+kalkulatorischen\s+gr(?:ü|ue)nden)/i,
    /(?:jederzeit|ordentliche\s+k(?:ü|ue)ndigung)[^.!?;:\n]{0,90}?ohne\s+(?:grund|angabe\s+von\s+gr(?:ü|ue)nden|begr(?:ü|ue)ndung)/i,
  ],
  feedback_ip_grab: [
    /(?:ü|ue)bertr(?:ä|ae)gt[^.!?;:\n]{0,70}?(?:jede\s+nutzung|alle\s+rechte|s(?:ä|ae)mtliche\s+rechte)[^.!?;:\n]{0,70}?(?:anregung|idee|vorschlag|r(?:ü|ue)ckmeldung)/i,
    /(?:anregung|idee|vorschlag|r(?:ü|ue)ckmeldung|verbesserungsvorschlag)\w*[^.!?;:\n]{0,90}?(?:gehen\s+auf\s+den\s+anbieter\s+(?:ü|ue)ber|darfs+ders+anbieter\s+frei\s+verwenden)/i,
  ],
  international_data_processing: [
    /(?:verarbeitung|(?:ü|ue)bermittlung|transfer)[^.!?;:\n]{0,110}?anlage\s*\d*[^.!?;:\n]{0,90}?(?:staaten|drittl(?:ä|ae)nder|l(?:ä|ae)nder)/i,
    /anlage\s*\d+[^.!?;:\n]{0,110}?drittland/i,
    /(?:nicht\s+als\s+sicher\s+eingestuft|ohne\s+angemessenheitsbeschluss)/i,
    /(?:datenh(?:ä|ae)ndler|werbedienstleister|partnerunternehmen)[^.!?;:\n]{0,120}?(?:drittstaat|drittl(?:ä|ae)nder|drittland|(?:ü|ue)bermittelt)/i,
    /drittstaat[^.!?;:\n]{0,120}?(?:daten|gesundheitsdaten|standortdaten)/i,
  ],
  mandatory_expert_determination: [
    /(?:gutachter|sachverst(?:ä|ae)ndiger|schiedsgutachter)[^.!?;:\n]{0,90}?abschlie(?:ß|ss)end/i,
    /(?:vom\s+anbieter\s+(?:benannter|bestellter))[^.!?;:\n]{0,60}?(?:gutachter|sachverst(?:ä|ae)ndiger)/i,
  ],
  salvatorische_klausel_falle: [
    /ersetzt[^.!?;:\n]{0,90}?die\s+der\s+anbieter\s+f(?:ü|ue)r\s+wirtschaftlich\s+angemessen/i,
    /(?:unwirksame|unwirksamer)\s+klauseln?[^.!?;:\n]{0,90}?ersetzt[^.!?;:\n]{0,90}?anbieter/i,
  ],
  data_lockin: [
    /(?:ü|ue)bertragung[^.!?;:\n]{0,100}?daten[^.!?;:\n]{0,100}?ist\s+nicht\s+vorgesehen/i,
    /(?:datenherausgabe|herausgabe\s+der\s+daten|(?:ü|ue)bertragung\s+der\s+daten)[^.!?;:\n]{0,100}?(?:nicht\s+vorgesehen|ausgeschlossen|nicht\s+geschuldet)/i,
  ],
  fictitious_receipt: [
    /(?:versand|absendung)[^.!?;:\n]{0,110}?(?:gilt\s+als\s+zugang|gilt\s+als\s+zugegangen|gen(?:ü|ue)gt)/i,
  ],
  default_interest: [
    /verzugszinsen[^.!?;:\n]{0,100}?(?:\d{1,2}(?:[.,]\d+)?)\s*(?:%|prozent)s*(?:p.?\s*a.?|pro\s+jahr|j(?:ä|ae)hrlich)/i,
  ],
};
// >>> mined patterns
// Muster aus dem Red-Team-Korpus, gemergt aus Verträge/proposals/patterns_*.json.
const MINED_SUPPLEMENTS = {
  agency_commission_exclusion: [
    new RegExp("direkt(?:verk(?:ä|ae)ufe|auftr(?:ä|ae)ge|gesch(?:ä|ae)ft\\w*|verkauf\\w*)[^.!?;:\\n]{0,80}?(?:l(?:ö|oe)sen|l(?:ö|oe)st)[^.!?;:\\n]{0,30}?kein\\w*\\s+(?:provision|provisionsanspruch|verg(?:ü|ue)tung|anspruch)", "i"),
    new RegExp("keine\\s+provision[^.!?;:\\n]{0,100}?(?:direkt\\s+verl(?:ä|ae)ngert|direkt\\s+abschlie(?:ß|ss)t|unmittelbar\\s+mit\\s+dem\\s+(?:versicherer|anbieter|hersteller))", "i"),
  ],
  agency_goodwill_waiver: [
    new RegExp("ausgleich[^.!?;:\\n]{0,80}?(?:kundenstamm|kundenkreis|geworbene\\s+kunden|aufbau\\s+des\\s+kundenstamms)[^.!?;:\\n]{0,50}?(?:ist\\s+ausgeschlossen|ausgeschlossen|entf(?:ä|ae)llt|wird\\s+nicht\\s+gew(?:ä|ae)hrt|besteht\\s+nicht)", "i"),
    new RegExp("(?:kundenstamm|kundenkreis)[^.!?;:\\n]{0,80}?steht[^.!?;:\\n]{0,50}?kein\\w*\\s+ausgleich\\s+zu", "i"),
    new RegExp("ausgleich\\w*[^.!?;:\\n]{0,80}?(?:ist|wird|sind|werden)[^.!?;:\\n]{0,30}?(?:ausgeschlossen|abbedungen|nicht\\s+geschuldet|verzichtet)", "i"),
    new RegExp("anspr(?:ü|ue)che\\s+wegen[^.!?;:\\n]{0,100}?(?:kundenbeziehungen|kundenstamm|kundschaft|kundenkontakte|übernommenen\\s+kunden)[^.!?;:\\n]{0,80}?bestehen[^.!?;:\\n]{0,40}?nicht", "i"),
    new RegExp("(?:kundenbeziehungen|kundenstamm|kundschaft|vermittelte\\s+kunden|übernommenen\\s+kunden)[^.!?;:\\n]{0,100}?(?:besteht|bestehen|entsteht|entstehen)[^.!?;:\\n]{0,60}?nach\\s+(?:der\\s+)?(?:beendigung|vertragsende|k(?:ü|ue)ndigung)[^.!?;:\\n]{0,40}?nicht", "i"),
  ],
  as_is_software_disclaimer: [
    new RegExp("(?:ohne\\s+m(?:ä|ae)ngelgew(?:ä|ae)hr|im\\s+ist-zustand|wie\\s+gesehen)[^.!?;:\\n]{0,80}?(?:ohne|und\\s+ohne|keine)\\s+(?:jegliche\\s+)?(?:zusicherung|gew(?:ä|ae)hrleistung|garantie|m(?:ä|ae)ngelhaftung)", "i"),
    new RegExp("(?:software|anwendung|leistung|dienstleistung|plattform)[^.!?;:\\n]{0,60}?wird\\s+ohne\\s+gew(?:ä|ae)hrleistung[^.!?;:\\n]{0,80}?bereitgestellt", "i"),
    new RegExp("ohne\\s+gew(?:ä|ae)hrleistung\\s+f(?:ü|ue)r\\s+(?:fehlerfreiheit|eignung|funktionsf(?:ä|ae)higkeit|beschaffenheit|m(?:ä|ae)ngelfreiheit)", "i"),
  ],
  assignment_restriction: [
    new RegExp("(?:kann|k(?:ö|oe)nnen|darf|d(?:ü|ue)rfen)[^.!?;:\\n]{0,80}?anspr(?:ü|ue)che[^.!?;:\\n]{0,80}?nur\\s+mit\\s+(?:vorheriger\\s+|schriftlicher\\s+)?zustimmung[^.!?;:\\n]{0,80}?(?:abtreten|abzutreten|(?:ü|ue)bertragen|zu\\s+(?:ü|ue)bertragen)", "i"),
    new RegExp("(?:abtretung|(?:ü|ue)bertragung|weitergabe|weiterreichen|zession|abtreten)[^.!?;:\\n]{0,80}?(?:bedarf|nur\\s+mit|nur\\s+nach|nicht\\s+ohne)[^.!?;:\\n]{0,40}?(?:zustimmung|einverst(?:ä|ae)ndnis|genehmigung|schriftlicher)", "i"),
  ],
  asymmetric_confidentiality: [
    new RegExp("(?:verschwiegenheits|geheimhaltungs|vertraulichkeits)pflicht[^.!?;:\\n]{0,60}?(?:obliegt|trifft|besteht)[^.!?;:\\n]{0,40}?(?:ausschlie(?:ß|ss)lich|allein|nur|lediglich)[^.!?;:\\n]{0,30}?(?:dem|den|der)\\s+(?:mieter|kunde|k(?:ä|ae)ufer|auftraggeber|p(?:ä|ae)chter|besteller|erwerber|eigent(?:ü|ue)mer|k(?:ä|ae)uferin)\\w*", "i"),
    new RegExp("(?:nur|allein|ausschlie(?:ß|ss)lich)\\s+(?:der|den|die|dem)?\\s*(?:kunde|auftraggeber|besteller|mieter|sponsor|lizenznehmer|partner|creator|auftragnehmer)[^.!?;:\\n]{0,80}?(?:vertraulich|geheimhaltung|verschwiegenheit|geheim)\\w*", "i"),
    new RegExp("(?:vertraulichkeits|geheimhaltungs|verschwiegenheits)pflicht[^.!?;:\\n]{0,60}?(?:trifft\\s+nur|obliegt\\s+nur|nur\\s+(?:den|der|dem))", "i"),
    new RegExp("f(?:ü|ue)r\\s+(?:den\\s+|die\\s+)?(?:anbieter|auftragnehmer|dienstleister|vermieter|lizenzgeber|agentur)[^.!?;:\\n]{0,50}?(?:besteht\\s+keine|gilt\\s+(?:dies|das)\\s+nicht|trifft\\s+keine)", "i"),
    new RegExp("(?:den\\s+|die\\s+)?(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\\n]{0,40}?trifft\\s+keine\\s+(?:entsprechende\\s+)?pflicht", "i"),
  ],
  asymmetric_termination: [
    new RegExp("(?:beendet|k(?:ü|ue)ndigt)\\s+der\\s+(?:anbieter|auftragnehmer|dienstleister|vermieter|verk(?:ä|ae)ufer)[^.!?;:\\n]{0,80}?(?:aus\\s+wichtigem\\s+grund|fristlos|mit\\s+sofortiger\\s+wirkung|jederzeit)[^.!?;:\\n]{0,100}?(?:entfallen|erl(?:ö|oe)schen|entf(?:ä|ae)llt|endet)[^.!?;:\\n]{0,50}?anspr(?:ü|ue)che\\s+des\\s+(?:kunden|auftraggebers|bestellers)", "i"),
    new RegExp("(?:der\\s+)?(?:kunde|auftraggeber|besteller)\\s+bleibt[^.!?;:\\n]{0,60}?bis\\s+zum\\s+ablauf[^.!?;:\\n]{0,40}?gebunden", "i"),
  ],
  audit_log_manipulation: [
    new RegExp("(?:aufzeichnungen|protokolle|systemprotokolle|b(?:ü|ue)cher)[^.!?;:\\n]{0,80}?gelten\\s+als[^.!?;:\\n]{0,50}?(?:unwiderlegbar\\w*\\s+nachweis|unwiderlegbar|vollbeweis|verbindlich|ma(?:ß|ss)geblich)", "i"),
    new RegExp("(?:gegenbeweis|gegenteilige[rn]?\\s+(?:nachweis|beweis)|abweichende\\s+nachweise?)[^.!?;:\\n]{0,60}?(?:ausgeschlossen|unzul(?:ä|ae)ssig|nicht\\s+zugelassen|ohne\\s+bedeutung)", "i"),
    new RegExp("(?:aufzeichnungen|protokolle|protokolldaten|logfiles|logs?)[^.!?;:\\n]{0,100}?(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\\n]{0,60}?gepflegt[^.!?;:\\n]{0,80}?(?:nachtr(?:ä|ae)glich|jederzeit|r(?:ü|ue)ckwirkend)[^.!?;:\\n]{0,40}?(?:berichtigt|ge(?:ä|ae)ndert|korrigiert|angepasst|gel(?:ö|oe)scht)", "i"),
    new RegExp("(?:aus\\s+technischen\\s+gr(?:ü|ue)nden|nach\\s+freiem\\s+ermessen|jederzeit)[^.!?;:\\n]{0,60}?nachtr(?:ä|ae)glich[^.!?;:\\n]{0,40}?(?:berichtigt|ver(?:ä|ae)ndert|korrigiert|angepasst|gel(?:ö|oe)scht)", "i"),
  ],
  burden_of_proof: [
    new RegExp("den\\s+(?:l(?:ü|ue)ckenlosen\\s+|vollst(?:ä|ae)ndigen\\s+)?nachweis[^.!?;:\\n]{0,70}?hat\\s+(?:der|die|das)\\s+(?:kunde|k(?:ä|ae)ufer|auftraggeber|besteller|mieter|p(?:ä|ae)chter|erwerber|auftragnehmer)[^.!?;:\\n]{0,30}?zu\\s+f(?:ü|ue)hren", "i"),
    new RegExp("nachweis\\w*[^.!?;:\\n]{0,80}?hat\\s+(?:der|die|das)\\s+(?:kunde|auftraggeber|besteller|k(?:ä|ae)ufer|mieter|auftragnehmer)[^.!?;:\\n]{0,40}?zu\\s+f(?:ü|ue)hren", "i"),
    new RegExp("aufzeichnungen\\s+des\\s+(?:anbieters|auftragnehmers|dienstleisters)[^.!?;:\\n]{0,80}?(?:ma(?:ß|ss)geblich|verbindlich|entscheidend|entscheiden)", "i"),
    new RegExp("(?:hat|haben)\\s+der\\s+(?:kunde|auftraggeber|besteller|mieter|k(?:ä|ae)ufer)[^.!?;:\\n]{0,60}?(?:nachzuweisen|zu\\s+beweisen|darzulegen)", "i"),
  ],
  data_grant: [
    new RegExp("r(?:ä|ae)umt[^.!?;:\\n]{0,60}?das\\s+recht\\s+ein[^.!?;:\\n]{0,100}?(?:daten|informationen)[^.!?;:\\n]{0,100}?(?:f(?:ü|ue)r\\s+(?:die\\s+)?(?:entwicklung|eigene\\s+zwecke|prognosemodelle)|zu\\s+verwenden|zu\\s+verwerten|zu\\s+nutzen)", "i"),
    new RegExp("(?:verp(?:ä|ae)chter|vermieter|anbieter|auftragnehmer|dienstleister|verk(?:ä|ae)ufer|auftraggeber|betreiber)[^.!?;:\\n]{0,60}?darf[^.!?;:\\n]{0,100}?(?:daten|informationen)[^.!?;:\\n]{0,80}?f(?:ü|ue)r\\s+eigene\\s+zwecke[^.!?;:\\n]{0,60}?(?:verwerten|nutzen|verwenden|auswerten|verarbeiten)", "i"),
    new RegExp("(?:erh(?:ä|ae)lt|erhalten|einger(?:ä|ae)umt)[^.!?;:\\n]{0,60}?(?:unwiderruflich|unbefristet|unentgeltlich|weltweit|perpetual|irrevocable)[^.!?;:\\n]{0,40}?(?:nutzungsrecht|licen[cs]e)", "i"),
  ],
  data_lockin: [
    new RegExp("(?:daten|datenbank|datenbestand)\\w*[^.!?;:\\n]{0,90}?nur\\s+gegen\\s+(?:eine[n]?\\s+)?(?:gesonderte\\w*\\s+)?geb(?:ü|ue)hr[^!?;:\\n]{0,90}?(?:(?:ü|ue)bergeben|herausgegeben|exportiert|bereitgestellt|(?:ü|ue)berlassen)", "i"),
    new RegExp("(?:export|herausgabe|(?:ü|ue)bertragung|bereitstellung)[^.!?;:\\n]{0,80}?(?:ist\\s+nicht\\s+bestandteil|nicht\\s+bestandteil|nicht\\s+vorgesehen|ausgeschlossen|erlischt|entf(?:ä|ae)llt)", "i"),
  ],
  deemed_acceptance: [
    new RegExp("(?:gilt|gelten)[^.!?;:\\n]{0,60}?mit\\s+(?:der\\s+)?(?:ü|ue)bernahme[^.!?;:\\n]{0,60}?als\\s+(?:mangelfrei|abgenommen|genehmigt|vertragsgem(?:ä|ae)(?:ß|ss))", "i"),
    new RegExp("m(?:ä|ae)ngel\\w*[^.!?;:\\n]{0,50}?nicht\\s+binnen[^.!?;:\\n]{0,40}?(?:ger(?:ü|ue)gt|angezeigt|gemeldet|beanstandet)[^.!?;:\\n]{0,70}?gilt[^.!?;:\\n]{0,40}?als\\s+(?:vertragsgem(?:ä|ae)(?:ß|ss)|abgenommen|genehmigt|freigegeben)", "i"),
    new RegExp("(?:schweigt|schweigen|widerspricht\\s+nicht|erhebt\\s+keine\\s+einw(?:ä|ae)nde)[^.!?;:\\n]{0,70}?gilt[^.!?;:\\n]{0,40}?als\\s+(?:freigegeben|abgenommen|genehmigt|vertragsgem(?:ä|ae)(?:ß|ss))", "i"),
    new RegExp("(?:nicht|nicht\\s+fristgerecht)[^.!?;:\\n]{0,40}?(?:innerhalb|binnen)[^.!?;:\\n]{0,30}?(?:von\\s+)?(?:\\d+|zwei|drei|vier|f(?:ü|ue)nf|zehn|vierzehn|sieben)\\s*(?:kalender)?(?:tagen|werktagen|wochen|stunden)[^.!?;:\\n]{0,40}?(?:antwortet|reagiert|antworten|erkl(?:ä|ae)rt)[^.!?;:\\n]{0,60}?(?:einverst(?:ä|ae)ndnis|zustimmung|genehmigung)[^.!?;:\\n]{0,40}?(?:unterstellt|gilt\\s+als\\s+erteilt|als\\s+erteilt)", "i"),
    new RegExp("(?:einverst(?:ä|ae)ndnis|zustimmung|genehmigung)\\s+(?:des\\s+kunden\\s+)?(?:wird|gilt\\s+als)[^.!?;:\\n]{0,30}?(?:unterstellt|erteilt|gegeben|fingiert)", "i"),
  ],
  default_interest: [
    new RegExp("(?:zinsen|zinssatz|verzinsung|verzugszins\\w*)[^.!?;:\\n]{0,60}?(?:\\d{1,2}(?:[.,]\\d+)?\\s*%|[a-zäöüß]{3,20}\\s+prozent|prozent)[^.!?;:\\n]{0,40}?(?:monatlich|pro\\s+monat|je\\s+monat|w(?:ö|oe)chentlich|t(?:ä|ae)glich)", "i"),
  ],
  dynamic_url_referral: [
    new RegExp("(?:pl(?:ä|ae)ne|dokumente|unterlagen|bedingungen|richtlinien|vorgaben|fassung)[^.!?;:\\n]{0,70}?(?:im|in\\s+dem|auf\\s+dem)\\s+[^.!?;:\\n]{0,40}?portal[^.!?;:\\n]{0,70}?(?:hinterlegt|abrufbar|bereitgestellt|eingestellt|ver(?:ö|oe)ffentlicht|gespeichert)[^.!?;:\\n]{0,90}?(?:fortlaufend|jeweils\\s+aktuell|laufend|st(?:ä|ae)ndig|regelm(?:ä|ae)(?:ß|ss)ig)[^.!?;:\\n]{0,40}?aktualisiert", "i"),
    new RegExp("(?:portal|plattform|onlinebereich)[^.!?;:\\n]{0,70}?(?:hinterlegt|bereitgestellt|abrufbar|eingestellt)[^.!?;:\\n]{0,90}?(?:fortlaufend|jeweils\\s+aktuell|laufend|st(?:ä|ae)ndig)[^.!?;:\\n]{0,40}?aktualisiert", "i"),
    new RegExp("gelten\\s+die[^.!?;:\\n]{0,80}?(?:abrufbar\\w*|hinterlegt\\w*|jeweils\\s+g(?:ü|ue)ltig\\w*|jeweils\\s+aktuell\\w*)[^.!?;:\\n]{0,60}?(?:fassung\\w*|bedingung\\w*|richtlinie\\w*|version\\w*|einkaufsbedingungen)", "i"),
  ],
  early_termination_charge: [
    new RegExp("(?:r(?:ü|ue)cktritt|r(?:ü|ue)ckzug|vorzeitige\\w*\\s+(?:beendigung|k(?:ü|ue)ndigung|vertragsaufl(?:ö|oe)sung))[^.!?;:\\n]{0,80}?(?:storno(?:geb(?:ü|ue)hr)?|abstandszahlung|beendigungsgeb(?:ü|ue)hr|pauschal\\w*\\s+(?:abstandszahlung|entsch(?:ä|ae)digung|verg(?:ü|ue)tung|geb(?:ü|ue)hr))[^.!?;:\\n]{0,60}?(?:geschuldet|f(?:ä|ae)llig|zu\\s+zahlen|zu\\s+entrichten|pauschal)", "i"),
    new RegExp("zahlt\\s+(?:er|sie|der\\s+kunde|der\\s+auftraggeber|der\\s+mieter)[^.!?;:\\n]{0,80}?(?:verg(?:ü|ue)tung|entgelt|raten|miete)[^.!?;:\\n]{0,60}?f(?:ü|ue)r\\s+die\\s+(?:verbleibenden|restlichen)\\s+(?:monate|laufzeit|mietmonate|raten)", "i"),
    new RegExp("(?:ohne\\s+anrechnung|ohne\\s+abzug)[^.!?;:\\n]{0,60}?(?:ersparter\\s+aufwendungen|ersparter\\s+kosten|ersparnisse|ersparter\\s+betriebskosten)", "i"),
  ],
  entire_agreement: [
    new RegExp("fr(?:ü|ue)here\\w*[^.!?;:\\n]{0,60}?(?:absprachen?|vereinbarungen?|zusagen?|regelungen?)[^.!?;:\\n]{0,60}?treten[^.!?;:\\n]{0,60}?au(?:ß|ss)er\\s+kraft", "i"),
    new RegExp("(?:baubeschreibung|leistungsbeschreibung|anlage|vereinbarung|absprache|planung|preisliste)[^!?;:\\n]{0,60}?tritt\\s+mit\\s+unterzeichnung[^.!?;:\\n]{0,60}?au(?:ß|ss)er\\s+kraft", "i"),
    new RegExp("(?:fr(?:ü|ue)here|bisherige|vorherige|m(?:ü|ue)ndliche)\\s+(?:absprachen|vereinbarungen|zusagen|abreden|abmachungen)[^.!?;:\\n]{0,60}?(?:treten|tritt|verlieren|verliert|werden|wird|sind)[^.!?;:\\n]{0,50}?(?:au(?:ß|ss)er\\s+kraft|unwirksam|gegenstandslos|aufgehoben|nicht\\s+mehr\\s+bestand)", "i"),
    new RegExp("nebenabreden[^.!?;:\\n]{0,60}?(?:entfalten|entfaltet|haben|hat)\\s+keine\\s+(?:rechtliche\\s+)?wirkung", "i"),
  ],
  excessive_survival: [
    new RegExp("(?:verkehrssicherungspflichten|sicherungspflichten|pflichten|verpflichtungen|gew(?:ä|ae)hrleistungs\\w*|haftung\\w*)[^.!?;:\\n]{0,60}?gelten\\s+zeitlich\\s+unbeschr(?:ä|ae)nkt\\s+fort", "i"),
    new RegExp("(?:haftungsfreistellungen?|freistellungen?|regelungen?|pflichten|verpflichtungen|vereinbarungen|bestimmungen)[^.!?;:\\n]{0,80}?wirken?[^.!?;:\\n]{0,60}?(?:über\\s+das\\s+vertragsende|über\\s+die\\s+vertragsbeendigung)\\s+hinaus[^.!?;:\\n]{0,60}?(?:ohne\\s+endzeitpunkt|ohne\\s+zeitliche\\s+beschr\\w*|zeitlich\\s+unbegrenzt|unbefristet|auf\\s+unbestimmte\\s+zeit)", "i"),
    new RegExp("(?:verschwiegenheits|vertraulichkeits|geheimhaltungs)pflicht\\w*[^.!?;:\\n]{0,80}?(?:bestehen|gelten|wirken)[^.!?;:\\n]{0,60}?über[^.!?;:\\n]{0,40}?(?:vertragsende|vertragsbeendigung|beendigung|ende)[^.!?;:\\n]{0,40}?hinaus[^.!?;:\\n]{0,40}?(?:unver(?:ä|ae)ndert|ohne\\s+(?:zeitliche\\s+)?(?:begrenzung|befristung)|fort)", "i"),
    new RegExp("(?:besteh\\w+|gilt|gelten|wirkt|wirken|wird|werden|ist|sind)[^.!?;:\\n]{0,60}?\\b(?:auf\\s+dauer|unbegrenzt|unbefristet|dauerhaft|zeitlich\\s+unbegrenzt)\\b", "i"),
    new RegExp("(?:ü|ue)berdauern[^.!?;:\\n]{0,70}?(?:beendigung|vertragsende|auslaufen|ende)", "i"),
    new RegExp("(?:vertraulichkeit|geheimhaltung|verschwiegenheit|wettbewerbsschutz|wettbewerbsverbot|nachvertragliche\\s+pflichten)[^.!?;:\\n]{0,80}?bestehen?\\s+noch\\s+(?:\\d+|f(?:ü|ue)nfzehn|zehn|zwanzig|drei(?:ß|ss)ig)\\s+jahre[^.!?;:\\n]{0,60}?fort", "i"),
    new RegExp("(?:noch|über)\\s+(?:f(?:ü|ue)nfzehn|zehn|zwanzig|drei(?:ß|ss)ig|\\d+)\\s+jahre[^.!?;:\\n]{0,60}?nach\\s+(?:dem\\s+)?(?:letzten\\s+)?(?:leistungstag|vertragsende|vertragsbeendigung)[^.!?;:\\n]{0,60}?(?:fort|bestehen|gelten|wirken)", "i"),
  ],
  extended_retention_of_title: [
    new RegExp("eigentum[^.!?;:\\n]{0,140}?bleibt\\s+beim\\s+(?:anbieter|auftragnehmer|lieferanten|hersteller|verk(?:ä|ae)ufer)[^.!?;:\\n]{0,120}?forderungen\\s+(?:verbundener\\s+unternehmen|eines\\s+verbundenen\\s+unternehmens|aus\\s+der\\s+gesamten\\s+gesch(?:ä|ae)ftsbeziehung|s(?:ä|ae)mtlicher\\s+vertr(?:ä|ae)ge)", "i"),
    new RegExp("(?:eigentum|eigentumsvorbehalt)[^.!?;:\\n]{0,140}?bis\\s+(?:auch\\s+)?(?:alle[nr]?\\s+|s(?:ä|ae)mtliche[nr]?\\s+)?forderungen[^.!?;:\\n]{0,80}?(?:verbundener\\s+unternehmen|aus\\s+(?:der\\s+)?gesch(?:ä|ae)ftsbeziehung|aus\\s+allen\\s+vertr(?:ä|ae)gen)", "i"),
  ],
  feedback_ip_grab: [
    new RegExp("(?:anregungen?|ideen|vorschl(?:ä|ae)ge|feedback|hinweise)[^.!?;:\\n]{0,100}?(?:darf|kann|d(?:ü|ue)rfen|k(?:ö|oe)nnen|werden|stehen)[^.!?;:\\n]{0,80}?ohne\\s+(?:eine\\s+)?(?:verg(?:ü|ue)tung|gegenleistung|beteiligung|entgelt|zustimmung)[^.!?;:\\n]{0,60}?(?:verwerten|nutzen|verwenden|ver(?:ö|oe)ffentlichen|(?:ü|ue)bertragen)", "i"),
    new RegExp("(?:verbesserungsvorschl(?:ä|ae)ge|anregungen|ideen|r(?:ü|ue)ckmeldungen)[^.!?;:\\n]{0,70}?(?:werden|gehen|sind|wird)[^.!?;:\\n]{0,60}?(?:unentgeltlich\\s+)?(?:eigentum|bestandteil)[^.!?;:\\n]{0,40}?(?:des|der)\\s+(?:auftragnehmers|anbieters|dienstleisters|agentur)", "i"),
  ],
  fictitious_receipt: [
    new RegExp("(?:gilt|gelten)[^.!?;:\\n]{0,70}?(?:absender(?:ü|ue)bergabe|absendung|(?:ü|ue)bergabe\\s+an\\s+(?:den\\s+)?(?:versanddienstleister|boten|kurier))[^.!?;:\\n]{0,70}?als\\s+zugegangen", "i"),
    new RegExp("(?:erkl(?:ä|ae)rungen|mitteilungen|k(?:ü|ue)ndigungen|anzeigen|schreiben)[^.!?;:\\n]{0,70}?gelten[^.!?;:\\n]{0,60}?als\\s+zugegangen", "i"),
    new RegExp("(?:f(?:ü|ue)r\\s+den\\s+zugang|als\\s+zugang|zugang\\s+(?:einer\\s+erkl(?:ä|ae)rung|von\\s+erkl(?:ä|ae)rungen))[^.!?;:\\n]{0,60}?gen(?:ü|ue)gt\\s+die\\s+absendung", "i"),
    new RegExp("(?:kunde|empf(?:ä|ae)nger)[^.!?;:\\n]{0,40}?tr(?:ä|ae)gt\\s+das\\s+risiko\\s+der\\s+nichtkenntnisnahme", "i"),
  ],
  force_majeure_asymmetry: [
    new RegExp("h(?:ö|oe)here[rn]?\\s+gewalt[^.!?;:\\n]{0,120}?(?:befreit|entbindet|entbunden|befreien)[^.!?;:\\n]{0,120}?(?:w(?:ä|ae)hrend|jedoch|dennoch|gleichwohl|unver(?:ä|ae)ndert|unber(?:ü|ue)hrt|ungek(?:ü|ue)rzt)[^.!?;:\\n]{0,120}?(?:zahlungspflicht|vorauszahlungspflicht|pachtzins|kaufpreisraten|verg(?:ü|ue)tungspflicht|entgelt|zahlung|zu\\s+leisten|zu\\s+entrichten)", "i"),
    new RegExp("(?:erf(?:ü|ue)llungspflichten|leistungspflichten|pflichten|lieferpflichten)[^.!?;:\\n]{0,60}?(?:entfallen|entf(?:ä|ae)llt|ruhen|werden\\s+ausgesetzt)[^.!?;:\\n]{0,80}?(?:ereignissen?\\s+|f(?:ä|ae)llen?\\s+)?h(?:ö|oe)herer\\s+gewalt[^.!?;:\\n]{0,80}?(?:zahlungspflicht|vorauszahlungspflicht|verg(?:ü|ue)tungspflicht|zahlungen?)[^.!?;:\\n]{0,60}?(?:besteht|bleibt|sind|ist|werden)[^.!?;:\\n]{0,40}?(?:unver(?:ä|ae)ndert|unber(?:ü|ue)hrt|ungek(?:ü|ue)rzt|fort|dennoch|weiter)", "i"),
    new RegExp("(?:überschwemmung|naturereignis|naturkatastrophe|tierseuche|epidemie|pandemie|streik|krieg)[^.!?;:\\n]{0,80}?(?:befreit|entbunden|freigestellt|entlastet)[^.!?;:\\n]{0,90}?(?:w(?:ä|ae)hrend|jedoch|dennoch|gleichwohl|unver(?:ä|ae)ndert|unber(?:ü|ue)hrt|ungek(?:ü|ue)rzt)[^.!?;:\\n]{0,120}?(?:pachtzins|miete|zahlungspflicht|verg(?:ü|ue)tung|entgelt|kaufpreis\\w*|raten?)[^.!?;:\\n]{0,60}?(?:zu\\s+entrichten|zu\\s+zahlen|zu\\s+leisten|besteht|bleibt|geschuldet)", "i"),
    new RegExp("h(?:ö|oe)here\\w?\\s+gewalt[^.!?;:\\n]{0,130}?(?:entbind\\w+|entf(?:ä|ae)llt|befreit|freigestellt)[^.!?;:\\n]{0,80}?(?:auftragnehmer|anbieter|dienstleister|veranstalter)", "i"),
    new RegExp("(?:ereignisse|umst(?:ä|ae)nde|vorg(?:ä|ae)nge)\\s+au(?:ß|ss)erhalb\\s+des\\s+(?:einfluss|verantwortungs|risiko|gefahren)bereichs\\s+des\\s+(?:anbieters|auftragnehmers|dienstleisters)[^.!?;:\\n]{0,80}?verl(?:ä|ae)ngern[^.!?;:\\n]{0,40}?(?:dessen\\s+)?(?:leistungs)?fristen", "i"),
    new RegExp("(?:zahlungspflichten|zahlungsverpflichtungen|verg(?:ü|ue)tungspflichten?)\\s+des\\s+(?:kunden|auftraggebers|bestellers)[^.!?;:\\n]{0,60}?(?:laufen|bleiben|bestehen)[^.!?;:\\n]{0,40}?unver(?:ä|ae)ndert", "i"),
  ],
  insolvency_termination: [
    new RegExp("(?:antrag\\s+auf\\s+)?insolvenz[^.!?;:\\n]{0,60}?(?:erlischt|endet|beendet|entf(?:ä|ae)llt)[^.!?;:\\n]{0,50}?(?:der\\s+vertrag|das\\s+vertragsverh(?:ä|ae)ltnis|automatisch|ohne\\s+weiteres)", "i"),
    new RegExp("stellt\\s+der\\s+(?:kunde|auftraggeber|besteller)[^.!?;:\\n]{0,80}?antrag\\s+auf\\s+er(?:ö|oe)ffnung\\s+eines\\s+(?:verfahrens|insolvenzverfahrens)[^.!?;:\\n]{0,120}?darf\\s+der\\s+(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\\n]{0,80}?(?:einstellen|beenden|k(?:ü|ue)ndigen)", "i"),
    new RegExp("antrag\\s+auf\\s+er(?:ö|oe)ffnung\\s+eines\\s+(?:verfahrens|insolvenzverfahrens)[^.!?;:\\n]{0,140}?(?:sofort|jederzeit|fristlos|ohne\\s+weiteres)[^.!?;:\\n]{0,60}?(?:einstellen|beenden|k(?:ü|ue)ndigen)", "i"),
  ],
  international_data_processing: [
    new RegExp("(?:daten|informationen)[^.!?;:\\n]{0,80}?au(?:ß|ss)erhalb\\s+der\\s+europ(?:ä|ae)ischen\\s+union[^.!?;:\\n]{0,80}?(?:(?:ü|ue)bermittelt|gespeichert|verarbeitet|(?:ü|ue)bertragen|gehostet)", "i"),
    new RegExp("(?:daten|informationen)[^.!?;:\\n]{0,80}?(?:auf|in)\\s+(?:servern?\\s+in\\s+einem\\s+drittland|servern?\\s+au(?:ß|ss)erhalb|einem\\s+drittland|ein\\s+drittland|drittl(?:ä|ae)ndern)[^.!?;:\\n]{0,60}?(?:gespeichert|verarbeitet|gehostet|(?:ü|ue)bermittelt)", "i"),
  ],
  jurisdiction: [
    new RegExp("ausschlie(?:ß|ss)liche\\s+(?:zust(?:ä|ae)ndigkeit|gerichtsbarkeit|gerichtsstand)[^.!?;:\\n]{0,60}?(?:liegt\\s+bei|beim|bei\\s+dem|obliegt|ist\\s+bei)[^.!?;:\\n]{0,60}?(?:handelsgericht|gericht|gerichtshof|schiedsgericht|kammer)", "i"),
  ],
  liability: [
    new RegExp("(?:haftung|schadensersatz|ersatzpflicht)[^.!?;:\\n]{0,100}?auf\\s+(?:(?:eine?|zwei|drei|vier|fünf|sechs|sieben|acht|neun|zehn)\\w*\\s+)?monatsmieten?[^.!?;:\\n]{0,40}?(?:begrenzt|gedeckelt|beschr(?:ä|ae)nkt|limitiert)", "i"),
    new RegExp("(?:haftung|schadensersatz|ersatzpflicht)[^.!?;:\\n]{0,100}?auf\\s+(?:die\\s+h(?:ö|oe)he\\s+(?:des|der)\\s+)?(?:(?:zehn|zwanzig|fünfzig|hundert|\\d{1,3})\\s*(?:%|prozent)|architektenhonorar\\w*|honorar\\w*|kaufpreis\\w*|\\d[\\d.,]*\\s*(?:EUR|Euro))[^.!?;:\\n]{0,60}?(?:begrenzt|gedeckelt|beschr(?:ä|ae)nkt|limitiert)", "i"),
    new RegExp("weitergehende\\s+anspr(?:ü|ue)che[^.!?;:\\n]{0,100}?(?:mit\\s+der\\s+(?:verbesserung|nachbesserung|nacherf(?:ü|ue)llung|mangelbeseitigung)|durch\\s+die\\s+(?:verbesserung|nachbesserung))[^.!?;:\\n]{0,60}?abgegolten", "i"),
    new RegExp("(?:haftung|einstandspflicht|ersatzpflicht)[^.!?;:\\n]{0,90}?(?:ist|wird|bleibt)[^.!?;:\\n]{0,40}?(?:auf|bis\\s+(?:zur|zu))[^.!?;:\\n]{0,40}?(?:eine?[mn]?\\s+monatsmiete\\w*|(?:zwei|drei|vier|f(?:ü|ue)nf|sechs|zehn|zwanzig|\\d{1,3})\\s+monatsmieten|monatspauschale\\w*|monatsverg(?:ü|ue)tung\\w*|monatsentgelt\\w*|jahresverg(?:ü|ue)tung\\w*|h(?:ö|oe)he\\s+des\\s+\\w*honorars?|(?:zehn|zwanzig|\\d{1,3})\\s*(?:%|prozent)\\s+des\\s+kaufpreises)[^.!?;:\\n]{0,40}?(?:begrenzt|beschr(?:ä|ae)nkt|gedeckelt|limitiert)", "i"),
    new RegExp("(?:haftung|schadensersatz|anspr(?:ü|ue)che)[^.!?;:\\n]{0,80}?(?:ist|sind|werden|wird)[^.!?;:\\n]{0,25}?auf\\s+(?:h(?:ö|oe)chstens\\s+|maximal\\s+)?\\d[\\d.,]*\\s*(?:EUR|Euro|CHF)[^.!?;:\\n]{0,20}?(?:begrenzt|beschr(?:ä|ae)nkt|gedeckelt)", "i"),
    new RegExp("(?:weitergehende|s(?:ä|ae)mtliche|alle)\\s+anspr(?:ü|ue)che[^.!?;:\\n]{0,80}?(?:sind|werden|ist)[^.!?;:\\n]{0,60}?(?:abgegolten|abgefunden|erledigt)", "i"),
  ],
  maintenance_conditional_shutdown: [
    new RegExp("mangels[^.!?;:\\n]{0,60}?(?:abgeschlossen\\w*\\s+)?wartungs(?:vertrag|vertrags|vereinbarung)\\w*[^.!?;:\\n]{0,90}?(?:abgeschaltet|abgeschalten|stillgelegt|au(?:ß|ss)er\\s+betrieb|gesperrt|eingestellt|nicht\\s+betrieben)", "i"),
    new RegExp("(?:mangels|ohne)\\s+(?:abgeschlossenen\\s+|g(?:ü|ue)ltigen\\s+|bestehenden\\s+)?wartungs(?:vertrag|vertrags|vereinbarung)\\w*[^.!?;:\\n]{0,80}?(?:wird|werden|darf|kann|ist|sind)[^.!?;:\\n]{0,50}?(?:stillgelegt|abgeschaltet|au(?:ß|ss)er\\s+betrieb|gesperrt|eingestellt|unterbrochen)", "i"),
    new RegExp("(?:wartungsarbeiten|wartung|instandhaltung)[^.!?;:\\n]{0,80}?nur[^.!?;:\\n]{0,40}?(?:ausgef(?:ü|ue)hrt|erbracht|durchgef(?:ü|ue)hrt)[^.!?;:\\n]{0,80}?solange[^.!?;:\\n]{0,60}?(?:abo|paket|wartungsvertrag|servicepaket|mitgliedschaft)[^.!?;:\\n]{0,60}?(?:abgeschlossen|aktiv|gebucht|vorhanden|g(?:ü|ue)ltig)", "i"),
    new RegExp("(?:wartungsarbeiten|wartung|instandhaltung|serviceleistungen)[^.!?;:\\n]{0,100}?nur\\s+(?:dann\\s+)?(?:erbracht|ausgef(?:ü|ue)hrt|durchgef(?:ü|ue)hrt|geleistet)[^.!?;:\\n]{0,60}?(?:wenn|solange|sofern)[^.!?;:\\n]{0,60}?(?:premium|business|enterprise|erweitertes?)\\s*-?\\s*(?:abo|paket|support)", "i"),
  ],
  mandatory_expert_determination: [
    new RegExp("(?:sachverst(?:ä|ae)ndigen(?:gutachten|verfahren)|gutachterliche\\w*\\s+verfahren)[\\s\\S]{0,220}?kosten[^.!?;:\\n]{0,60}?(?:besteller|kunde|auftraggeber|mieter)[^.!?;:\\n]{0,60}?(?:vorzustrecken|vorauszuzahlen)|(?:sachverst(?:ä|ae)ndigen(?:gutachten|verfahren)|gutachterliche\\w*\\s+verfahren)[\\s\\S]{0,260}?(?:gerichtliche\\w*\\s+rechtsweg|rechtsweg)[\\s\\S]{0,80}?(?:erst\\s+nach|erst\\s+wenn|nach\\s+abschluss)", "i"),
    new RegExp("verfahrenskosten[^.!?;:\\n]{0,60}?(?:kunde|besteller|auftraggeber|antragsteller)[^.!?;:\\n]{0,40}?(?:vorzustrecken|vorauszuzahlen|zu\\s+tragen)", "i"),
    new RegExp("vom\\s+(?:anbieter|auftragnehmer|dienstleister|verk(?:ä|ae)ufer)\\s+(?:benannter|bestellter|ausgew(?:ä|ae)hlter)[^.!?;:\\n]{0,40}?(?:gutachter|sachverst(?:ä|ae)ndiger|schiedsgutachter)", "i"),
    new RegExp("(?:gutachter|sachverst(?:ä|ae)ndiger|schiedsgutachter)[^.!?;:\\n]{0,50}?abschlie(?:ß|ss)end", "i"),
  ],
  marketing_reference: [
    new RegExp("(?:als\\s+referenz(?:kunden?)?|zu\\s+referenzzwecken)[^.!?;:\\n]{0,90}?ohne\\s+(?:dass\\s+es\\s+)?(?:einer|der)\\s+(?:gesonderten\\s+|vorherigen\\s+|ausdr(?:ü|ue)cklichen\\s+)?zustimmung[^.!?;:\\n]{0,40}?(?:bedarf|bed(?:ü|ue)rfe)", "i"),
    new RegExp("(?:darf|kann|d(?:ü|ue)rfen|k(?:ö|oe)nnen)[^.!?;:\\n]{0,60}?(?:den|die|das)\\s+(?:kunden|k(?:ä|ae)ufer|mieter|auftraggeber|besteller|erwerber|p(?:ä|ae)chter)\\w*[^.!?;:\\n]{0,80}?als\\s+referenzkunden?\\s+(?:f(?:ü|ue)hren|nennen|benennen)", "i"),
    new RegExp("(?:f(?:ü|ue)hrt|nennt|bezeichnet|verwendet|nutzt|erw(?:ä|ae)hnt|darf)[^.!?;:\\n]{0,70}?(?:den\\s+|die\\s+)?(?:auftraggeber|auftragnehmer|kunden|sponsor|lizenznehmer|lizenzgeber|partner|verein)[^.!?;:\\n]{0,80}?(?:als\\s+referenz\\w*|als\\s+kunden|in\\s+(?:angeboten|fallstudien|case\\s*studies|pressemitteilungen|marketingunterlagen|werbetr(?:ä|ae)gern|werbung|kundenlisten|prospekten))", "i"),
    new RegExp("(?:firmenname|bildmarke|marke|marken|logo)\\b[^.!?;:\\n]{0,70}?(?:in|auf)[^.!?;:\\n]{0,25}?(?:werbung|werbetr(?:ä|ae)gern|marketingunterlagen|pressemitteilungen|angeboten|fallstudien|case\\s*studies|website|kundenliste|prospekten)", "i"),
  ],
  non_compete: [
    new RegExp("(?:wird|werden|verpflichtet\\s+sich|verpflichtet)[^.!?;:\\n]{0,80}?f(?:ü|ue)r\\s+(?:\\d{1,2}|zwei|drei|vier|fünf|sechs|zehn)\\s+(?:jahre?|monate?)[^.!?;:\\n]{0,80}?keine?\\w*\\s+[^.!?;:\\n]{0,60}?(?:betreuen|betreiben|verkaufen|anbieten|t(?:ä|ae)tig\\s+werden|zu\\s+betreiben)", "i"),
    new RegExp("(?:betreibt|erzeugt|h(?:ä|ae)lt|unterh(?:ä|ae)lt|verkauft)[^.!?;:\\n]{0,60}?f(?:ü|ue)r\\s+die\\s+(?:laufzeit|vertragsdauer|dauer\\s+des\\s+vertrags)[^.!?;:\\n]{0,60}?keine?\\w*\\s+(?:eigene|weitere|andere)\\w*", "i"),
    new RegExp("(?:verkauft|betreibt|f(?:ü|ue)hrt)[^.!?;:\\n]{0,60}?(?:ausschlie(?:ß|ss)lich|nur)\\s+(?:textilien|marken\\w*|produkte\\s+des\\s+herstellers|sortiment\\w*)", "i"),
    new RegExp("nach\\s+(?:dem\\s+ende|beendigung|vertragsende|auslaufen|ablauf)[^.!?;:\\n]{0,60}?(?:bleibt|ist|wird|sind|werden)[^.!?;:\\n]{0,60}?(?:unt(?:ä|ae)tig|nicht\\s+(?:t(?:ä|ae)tig|beratend|werblich|vermittelnd|konkurrierend)|nicht\\s+mehr\\s+t(?:ä|ae)tig)", "i"),
  ],
  non_solicitation: [
    new RegExp("(?:stellt|stellen|wird|werden|darf|d(?:ü|ue)rfen)[^.!?;:\\n]{0,70}?keine\\s+(?:ehemaligen|fr(?:ü|ue)heren|aktuellen)?\\s*(?:mitarbeiter|besch(?:ä|ae)ftigte)[^.!?;:\\n]{0,40}?ein\\b", "i"),
  ],
  price_escalation: [
    new RegExp("(?:(?:beitr(?:ä|ae)ge|zins|preis|miete|entgelt|kosten|verg(?:ü|ue)tung)\\w*[^.!?;:\\n]{0,70}?(?:erh(?:ö|oe)h(?:t|en)\\s+sich|steigt|steigen)|(?:erh(?:ö|oe)h(?:t|en)\\s+sich|steigt|steigen)[^.!?;:\\n]{0,70}?(?:miete|entgelt|preis|beitr(?:ä|ae)ge|zins|kosten|verg(?:ü|ue)tung)\\w*)[^.!?;:\\n]{0,70}?um\\s+(?:bis\\s+zu\\s+)?(?:ein|zwei|drei|vier|fünf|sechs|sieben|acht|neun|zehn|elf|zw(?:ö|oe)lf|\\d{1,2}(?:[.,]\\d+)?)\\s*(?:%|prozent)", "i"),
    new RegExp("(?:miete|entgelt|preis|beitrag|zins|kosten|verg(?:ü|ue)tung)\\w*[^!?;:\\n]{0,80}?(?:wird|werden)[^!?;:\\n]{0,80}?um\\s+(?:bis\\s+zu\\s+)?(?:\\d{1,2}(?:[.,]\\d+)?|zwei|drei|vier|fünf|sechs|sieben|acht|neun|zehn|elf|zw(?:ö|oe)lf)\\s*(?:%|prozent)[^.!?;:\\n]{0,60}?(?:angehoben|erh(?:ö|oe)ht|an(?:zu)?heben)", "i"),
    new RegExp("beh(?:ä|ae)lt\\s+sich[^.!?;:\\n]{0,60}?vor[^.!?;:\\n]{0,90}?um\\s+bis\\s+zu\\s+(?:\\d{1,2}(?:[.,]\\d+)?|zwei|drei|vier|fünf|sechs|sieben|acht|neun|zehn|elf|zw(?:ö|oe)lf)[^.!?;:\\n]{0,60}?(?:zu\\s+erh(?:ö|oe)hen|anzuheben|anzupassen)", "i"),
    new RegExp("(?:kosten|preis)steigerungen[^.!?;:\\n]{0,90}?(?:vollumf(?:ä|ae)nglich|vollst(?:ä|ae)ndig|ungek(?:ü|ue)rzt)[^.!?;:\\n]{0,90}?weitergegeben", "i"),
    new RegExp("(?:entgelt\\w*|verg(?:ü|ue)tung\\w*|pauschale\\w*|miete\\b|raummiete|geb(?:ü|ue)hr\\w*|honorar\\w*|preis\\w*|monatspauschale)[^.!?;:\\n]{0,60}?(?:steigt|steigen|erh(?:ö|oe)ht\\s+sich|erh(?:ö|oe)hen\\s+sich|wird\\s+erh(?:ö|oe)ht|werden\\s+erh(?:ö|oe)ht)[^.!?;:\\n]{0,70}?(?:um\\s+(?:\\d|[a-zäöüß]{3,20})|j(?:ä|ae)hrlich|kalenderjahr|prozent)", "i"),
    new RegExp("sonderk(?:ü|ue)ndigungsrecht[^.!?;:\\n]{0,30}?(?:besteht\\s+nicht|ist\\s+ausgeschlossen|wird\\s+nicht\\s+gew(?:ä|ae)hrt|ist\\s+nicht\\s+vorgesehen)", "i"),
  ],
  service_credit_forfeiture: [
    new RegExp("(?:vers(?:ä|ae)umte?\\w*|fehlende?\\w*|unterlassene?\\w*)[^.!?;:\\n]{0,60}?(?:sicherheitsupdates?|kritische?\\s+(?:updates?|patches?|sicherheitsupdates?))[^.!?;:\\n]{0,60}?(?:entf(?:ä|ae)llt|verf(?:ä|ae)llt)[^.!?;:\\n]{0,40}?(?:die\\s+|der\\s+)?(?:gutschrift|service\\s?credits?|verg(?:ü|ue)tung|anspruch|ersatzlos)", "i"),
    new RegExp("(?:gutschrift\\w*|service[\\s-]?credits?|anspr(?:ü|ue)che)[^.!?;:\\n]{0,120}?(?:innerhalb|binnen)\\s+(?:von\\s+)?(?:\\d+|ein(?:er|em)?|zwei|drei|vier|f(?:ü|ue)nf|sechs|sieben|acht|zehn|vierzehn|drei(?:ß|ss)ig)\\s*(?:werktagen|kalendertagen|tagen|wochen|monaten)[^.!?;:\\n]{0,80}?(?:geltend\\s+zu\\s+machen|geltend\\s+macht|anzumelden|mitzuteilen|zu\\s+beantragen|beansprucht)", "i"),
    new RegExp("(?:gutschrift\\w*|service[\\s-]?credits?)[^.!?;:\\n]{0,60}?(?:verfallen|entfallen|erl(?:ö|oe)schen)", "i"),
  ],
  service_suspension: [
    new RegExp("(?:verp(?:ä|ae)chter|vermieter\\w*|auftragnehmer\\w*|anbieter\\w*|dienstleister\\w*|betreiber\\w*|unternehmer\\w*)[^.!?;:\\n]{0,60}?(?:darf|kann|ist\\s+berechtigt|sind\\s+berechtigt)[^.!?;:\\n]{0,60}?(?:die\\s+|den\\s+|der\\s+)?(?:nutzung|zufahrt|einfahrt|baustellenzufahrt|versorgung|belieferung|leistung\\w*)[^.!?;:\\n]{0,60}?ohne\\s+(?:vorherige\\s+|weitere\\s+)?(?:ank(?:ü|ue)ndigung|vorank(?:ü|ue)ndigung|abmahnung|vorwarnung)[^.!?;:\\n]{0,60}?(?:einstellen|sperren|unterbrechen|aussetzen|blockieren|entziehen|verweigern)", "i"),
    new RegExp("(?:sperrt|sperren|unterbricht|unterbrechen|stellt|stellen|setzt|setzen|stoppt|stoppen)[^.!?;:\\n]{0,60}?(?:den\\s+|die\\s+|das\\s+)?(?:zugang|betreuung|leistung|plakatierung|lieferung|service|belieferung)[^.!?;:\\n]{0,60}?(?:unverz(?:ü|ue)glich|ohne\\s+(?:vorank(?:ü|ue)ndigung|abmahnung)|fristlos|sofort|jederzeit|ohne\\s+grund)", "i"),
    new RegExp("(?:zugang|betreuung|leistung|plakatierung|lieferung|service|belieferung)[^.!?;:\\n]{0,60}?(?:ohne\\s+(?:vorank(?:ü|ue)ndigung|abmahnung)|fristlos|sofort)[^.!?;:\\n]{0,40}?(?:sperren|unterbrechen|einstellen|aussetzen|stoppen)", "i"),
  ],
  setoff_restriction: [
    new RegExp("(?:kann|k(?:ö|oe)nnen|darf|d(?:ü|ue)rfen)[^.!?;:\\n]{0,70}?mit\\s+(?:gegenforderungen|gegenanspr(?:ü|ue)chen|forderungen)[^.!?;:\\n]{0,70}?nur\\s+(?:nach|mit|gegen|im\\s+rahmen)\\s+[^.!?;:\\n]{0,60}?aufrechnen", "i"),
    new RegExp("(?:aufrechnung|verrechnung|einbehalt\\w*|zur(?:ü|ue)ckbehaltung\\w*|gegenforderung\\w*|gegenanspruch|gegenanspr(?:ü|ue)che)[^.!?;:\\n]{0,80}?(?:bedarf\\s+der|nur\\s+mit|ist\\s+nicht\\s+m(?:ö|oe)glich|sind\\s+nicht\\s+m(?:ö|oe)glich|ist\\s+ausgeschlossen|sind\\s+ausgeschlossen|bleiben[^.!?;:\\n]{0,40}?(?:unber(?:ü|ue)cksichtigt|au(?:ß|ss)er\\s+betracht)|kein\\s+zur(?:ü|ue)ckbehaltungsrecht|begr(?:ü|ue)nden\\s+kein|ist\\s+kein\\s+recht)", "i"),
    new RegExp("(?:kein\\s+recht\\s+(?:auf|zu|zum)\\s+einbehalt|f(?:ü|ue)r\\s+einbehalte\\s+steht[^.!?;:\\n]{0,40}?kein\\s+recht)", "i"),
    new RegExp("verzicht\\w*[^.!?;:\\n]{0,60}?(?:auf\\s+)?(?:die\\s+)?(?:aufrechnung|verrechnung|einbehalt|zur(?:ü|ue)ckbehaltung)", "i"),
  ],
  statute_of_limitations: [
    new RegExp("anspr(?:ü|ue)che[^.!?;:\\n]{0,60}?verj(?:ä|ae)hren[^.!?;:\\n]{0,60}?(?:innerhalb\\s+von|binnen|nach|in)\\s+(?:(?<!\\d)(?:[1-9]|1[0-8]|einem|einer|zwei|drei|sechs|zw(?:ö|oe)lf))\\s+(?:jahr|monat|woche)\\w*", "i"),
    new RegExp("anspr(?:ü|ue)che[^.!?;:\\n]{0,60}?verj(?:ä|ae)hren[^.!?;:\\n]{0,20}?(?<!\\d)(?:[1-9]|1[0-8]|ein|eine|zwei|drei|sechs|zw(?:ö|oe)lf|achtzehn)\\s*(?:jahr|monat|woche)\\w*", "i"),
    new RegExp("(?:verj(?:ä|ae)hr\\w*|verwirk\\w*)[^.!?;:\\n]{0,60}?(?<!\\d)(?:[1-9]|1[0-8]|ein|eine|einer|zwei|drei|sechs|zw(?:ö|oe)lf|achtzehn)\\s*monat\\w*", "i"),
    new RegExp("(?:verj(?:ä|ae)hr\\w*|verwirk\\w*)[^.!?;:\\n]{0,60}?(?<!\\d)(?:ein|eine|einer)\\s*jahr\\w*", "i"),
  ],
  subcontractor_uncontrolled: [
    new RegExp("(?:darf|d(?:ü|ue)rfen|kann|k(?:ö|oe)nnen|setzt|setzen|beauftragt|beauftragen)[^.!?;:\\n]{0,50}?ohne\\s+(?:vorherige\\s+|weitere\\s+)?(?:ank(?:ü|ue)ndigung|information|zustimmung)[^.!?;:\\n]{0,60}?(?:nachunternehmer\\w*|unterauftragnehmer\\w*|subunternehmer\\w*|dienstleister\\w*|systempartner\\w*)", "i"),
    new RegExp("(?:nachunternehmer\\w*|unterauftragnehmer\\w*|subunternehmer\\w*|dienstleister\\w*|systempartner\\w*)[^.!?;:\\n]{0,80}?ohne\\s+(?:vorherige\\s+|weitere\\s+)?(?:ank(?:ü|ue)ndigung|zustimmung|information)|(?:nachunternehmer\\w*|dienstleister\\w*)[^.!?;:\\n]{0,60}?ohne\\s+(?:den\\s+)?(?:besteller|auftraggeber|kunden|eigent(?:ü|ue)mer)\\w*\\s+zu\\s+informieren", "i"),
    new RegExp("(?:beauftragung|einsatz|hinzuziehung|auswahl|(?:ü|ue)bertragung)\\s+(?:von\\s+)?(?:nachunternehmern?|unterauftragnehmern?|subunternehmern?|systempartnern?|dritten|verbundenen\\s+gesellschaften)[^.!?;:\\n]{0,90}?(?:bedarf\\s+keiner?|ist\\s+nicht\\s+erforderlich|sind\\s+nicht\\s+erforderlich|erfolgt\\s+ohne)", "i"),
    new RegExp("(?:greift|greifen|bedient\\s+sich)[^.!?;:\\n]{0,50}?(?:auf\\s+)?(?:verbundene\\w*\\s+(?:gesellschaften|unternehmen)|konzernunternehmen|konzerngesellschaften|verbundene\\s+unternehmen)", "i"),
    new RegExp("(?:darf|d(?:ü|ue)rfen|ist\\s+berechtigt|sind\\s+berechtigt|kann|k(?:ö|oe)nnen)[^.!?;:\\n]{0,70}?(?:nachunternehmen|nachunternehmer|subunternehmer|medienpartner|partnerunternehmen|werbepl(?:ä|ae)tze|dritte)[^.!?;:\\n]{0,60}?(?:einsetzen|weiterverkaufen|beauftragen|(?:ü|ue)bertragen|einbeziehen)", "i"),
    new RegExp("(?:setzt|setzen|beauftragt|beauftragen|greift)[^.!?;:\\n]{0,60}?(?:nachunternehmen|nachunternehmer|subunternehmer|medienpartner|dritte)[^.!?;:\\n]{0,50}?(?:ohne\\s+(?:vorherige\\s+)?(?:ank(?:ü|ue)ndigung|zustimmung|information)|einer\\s+zustimmung\\s+bedarf\\s+es\\s+nicht)", "i"),
  ],
  tax_indemnity_shift: [
    new RegExp("nachtr(?:ä|ae)glich\\w*[^.!?;:\\n]{0,80}?(?:steuernachforderung\\w*|abgabenbelastung\\w*|belastung\\w*|steuer\\w*|abgabe\\w*)[^.!?;:\\n]{0,80}?tr(?:ä|ae)gt\\s+(?:der|die|das)\\s+\\w+[^.!?;:\\n]{0,40}?(?:vollumf(?:ä|ae)nglich|vollst(?:ä|ae)ndig|allein|zur\\s+g(?:ä|ae)nze)", "i"),
    new RegExp("stellt[^.!?;:\\n]{0,80}?von\\s+(?:sämtlichen\\s+|allen\\s+)?(?:steuernachforderungen|steuerforderungen|abgabennachforderungen|steuerschulden|steuern\\s+und\\s+abgaben)[^.!?;:\\n]{0,40}?frei", "i"),
    new RegExp("(?:k(?:ä|ae)ufer|mieter|p(?:ä|ae)chter|kunde|auftraggeber|besteller|erwerber)\\w*[^.!?;:\\n]{0,60}?stellt[^.!?;:\\n]{0,60}?(?:verk(?:ä|ae)ufer|vermiet|verp(?:ä|ae)chter|anbieter|auftragnehmer|dienstleister)\\w*[^.!?;:\\n]{0,80}?von[^.!?;:\\n]{0,40}?(?:steuern|abgaben)[^.!?;:\\n]{0,40}?frei", "i"),
  ],
  third_party_indemnity: [
    new RegExp("(?:hält|halten)[^.!?;:\\n]{0,80}?von\\s+(?:sämtlichen\\s+|allen\\s+)?anspr(?:ü|ue)chen\\s+dritter[^.!?;:\\n]{0,80}?schadlos", "i"),
    new RegExp("(?:stellt|stellen)[^.!?;:\\n]{0,80}?von\\s+(?:sämtlichen\\s+|allen\\s+)?anspr(?:ü|ue)chen\\s+dritter[^.!?;:\\n]{0,60}?(?:unabh(?:ä|ae)ngig\\s+(?:davon,?\\s+ob|vom\\s+verschulden)|verschuldensunabh(?:ä|ae)ngig|vollumf(?:ä|ae)nglich)[^.!?;:\\n]{0,80}?(?:frei|freizustellen|schadlos)", "i"),
    new RegExp("hält[^.!?;:\\n]{0,60}?(?:für|von|wegen)[^.!?;:\\n]{0,60}?(?:bußgeld\\w*|ansprüche\\w*|forderungen\\w*|schäden\\w*|belastungen\\w*|verbindlichkeiten\\w*|abgaben\\w*)[^.!?;:\\n]{0,60}?schadlos", "i"),
    new RegExp("(?:^|\\s)(?:k(?:ä|ae)ufer|mieter|p(?:ä|ae)chter|kunde|auftraggeber|besteller|erwerber|werbegesellschaft|nutzer|betreiber)[^.!?;:\\n]{0,40}?stellt[^.!?;:\\n]{0,80}?von\\s+(?:sämtlichen\\s+|allen\\s+)?ansprüchen\\s+dritter[^.!?;:\\n]{0,60}?frei", "i"),
  ],
  unilateral_change: [
    new RegExp("(?:vermieter|verp(?:ä|ae)chter|anbieter|auftragnehmer|dienstleister|verk(?:ä|ae)ufer|verwalter|betreiber)[^.!?;:\\n]{0,70}?(?:darf|kann|ist\\s+berechtigt)[^.!?;:\\n]{0,70}?(?:entgelte?|preise?|nebenkosten|kosten|bedingungen|leistungsbeschreibung|leistungsumfang|verg(?:ü|ue)tung|miete)[^.!?;:\\n]{0,70}?(?:nach\\s+billigem\\s+ermessen|einseitig|jederzeit|ohne\\s+zustimmung)[^.!?;:\\n]{0,50}?(?:an(?:zu)?gleichen|an(?:zu)?passen|zu\\s+ändern|zu\\s+erh(?:ö|oe)hen|festzusetzen|zu\\s+bestimmen)", "i"),
    new RegExp("(?:änderungen|anpassungen)[^.!?;:\\n]{0,60}?(?:leistungsbeschreibung|leistungsumfang|bedingungen|entgelte|preise|nebenkosten)[^.!?;:\\n]{0,60}?(?:werden\\s+mit\\s+der|mit\\s+der)[^.!?;:\\n]{0,60}?(?:ver(?:ö|oe)ffentlichung|einstellung|bereitstellung|bekanntgabe)[^.!?;:\\n]{0,60}?wirksam", "i"),
    new RegExp("(?:mehrstunden|mehraufw(?:ä|ae)nde|sonderstunden|zusatzstunden|mehrarbeit|mehrleistungen)[^.!?;:\\n]{0,80}?(?:gelten|gilt|sind)[^.!?;:\\n]{0,60}?als\\s+(?:genehmigt|freigegeben|akzeptiert|einverstanden|abgenommen|beauftragt|verbindlich\\s+vereinbart)", "i"),
    new RegExp("(?:einseitig\\w*|nach\\s+billigem\\s+ermessen)[^.!?;:\\n]{0,80}?(?:(?:ä|ae)nder\\w*|angleich\\w+|anpass\\w+|einschr(?:ä|ae)nk\\w+)", "i"),
    new RegExp("(?:(?:ä|ae)nder\\w*|angleich\\w+|anpass\\w+|einschr(?:ä|ae)nk\\w+)[^.!?;:\\n]{0,60}?(?:einseitig|nach\\s+billigem\\s+ermessen|ohne\\s+zustimmung)", "i"),
  ],
  unilateral_convenience_termination: [
    new RegExp("(?:nur|allein|ausschlie(?:ß|ss)lich|lediglich)\\s+(?:der|die|das)\\s+(?:vermieter\\w*|verp(?:ä|ae)chter\\w*|anbieter\\w*|auftragnehmer\\w*|dienstleister\\w*|verk(?:ä|ae)ufer\\w*|verwalter\\w*|bautr(?:ä|ae)ger\\w*|betreiber\\w*)[^.!?;:\\n]{0,60}?berechtigt[^.!?;:\\n]{0,40}?(?:zu\\s+k(?:ü|ue)ndigen|die\\s+ordentliche\\s+k(?:ü|ue)ndigung)", "i"),
    new RegExp("(?:ordentliche\\w*\\s+)?k(?:ü|ue)ndig(?:ung|ungsrecht)[^.!?;:\\n]{0,80}?steht\\s+(?:ausschlie(?:ß|ss)lich|allein|nur|lediglich)[^.!?;:\\n]{0,20}?(?:der|dem|die)\\s+(?:anbieter|auftragnehmer|dienstleister|vermiet|verp(?:ä|ae)cht|verk(?:ä|ae)uf|unternehmer|verwalter|bautr(?:ä|ae)ger|lizenzgeber|generalunternehmer|werbegesellschaft)\\w*", "i"),
  ],
  unreasonable_audit: [
    new RegExp("(?:pr(?:ü|ue)fung\\w*|audit\\w*|kontroll\\w*|einsicht\\w*|zutritt)[^.!?;:\\n]{0,80}?(?:ohne\\s+(?:anlass|vorank(?:ü|ue)ndigung|terminabstimmung)|anlasslos|jederzeit)", "i"),
  ],
  unverbindliche_sla_falle: [
    new RegExp("bem(?:ü|ue)ht\\s+sich[^.!?;:\\n]{0,80}?(?:verf(?:ü|ue)gbarkeit|verf(?:ü|ue)gungsquote|verf(?:ü|ue)gbarkeitsquote|quote|reaktionszeit|servicezeit|ziel(?:gr(?:ö|oe)(?:ß|ss)e)?|standards?)[^.!?;:\\n]{0,80}?ohne\\s+(?:eine\\s+)?(?:zusage|garantie|rechtsanspruch|verbindliche\\s+zusage|verbindlichkeit)", "i"),
    new RegExp("(?:reaktions|wiederherstellungs|antwort|service|reparatur)[\\w-]*(?:\\s+und\\s+[\\w-]+)?zeiten[^.!?;:\\n]{0,60}?(?:sind|stellen)[^.!?;:\\n]{0,30}?unverbindliche", "i"),
    new RegExp("(?:reaktionszeit|antwortzeit|wiederherstellungszeit|verf(?:ü|ue)gbarkeit|servicelevel|kennzahlen)[^.!?;:\\n]{0,80}?(?:unverbindlich|nicht\\s+verbindlich|blo(?:ß|ss)e\\s+(?:ziel|richt)werte|ohne\\s+rechtsanspruch)", "i"),
  ],
  vicarious_agents: [
    new RegExp("f(?:ü|ue)r\\s+(?:das\\s+verhalten|die\\s+handlungen|handlungen)[^.!?;:\\n]{0,60}?(?:beauftragter\\s+dritter|dritter|unterlizenznehmer\\w*|erf(?:ü|ue)llungsgehilfen|nachunternehmer\\w*)[^.!?;:\\n]{0,80}?wird\\s+keine\\s+(?:verantwortung|einstandspflicht|haftung)\\b", "i"),
    new RegExp("(?:verantwortung|einstandspflicht|haftung)[^.!?;:\\n]{0,50}?f(?:ü|ue)r\\s+(?:handlungen|das\\s+verhalten)[^.!?;:\\n]{0,60}?(?:beauftragter\\s+)?dritter[^.!?;:\\n]{0,60}?(?:wird\\s+nicht|ist\\s+nicht|keine)[^.!?;:\\n]{0,30}?(?:ü|ue)bernommen", "i"),
    new RegExp("(?:haftet|haften)\\s+(?:der\\s+(?:anbieter|auftragnehmer|dienstleister)\\s+)?nicht\\s+f(?:ü|ue)r\\s+handlungen\\s+(?:und\\s+unterlassungen\\s+)?von\\s+(?:nach|unter|sub)unternehmer", "i"),
  ],
  written_form_trap: [
    new RegExp("änderungen\\s+(?:des\\s+vertrags|dieses\\s+vertrags|der\\s+vereinbarung|wesentlicher\\s+vertragsbestandteile)[^.!?;:\\n]{0,60}?bed(?:ü|ue)rfen[^.!?;:\\n]{0,60}?(?:der\\s+)?(?:eigenh(?:ä|ae)ndigen\\s+unterzeichnung|notariellen\\s+beurkundung|schriftform|unterzeichnung\\s+beider\\s+parteien|pers(?:ö|oe)nlichen\\s+unterzeichnung)", "i"),
    new RegExp("m(?:ü|ue)ndliche\\s+(?:absprachen|abreden|vereinbarungen|zusagen)[^.!?;:\\n]{0,70}?(?:sind|ist|bleiben|entfalten)[^.!?;:\\n]{0,40}?(?:ohne\\s+rechtswirkung|unwirksam|unverbindlich|keine\\s+wirkung)", "i"),
    new RegExp("(?:nur\\s+g(?:ü|ue)ltig|nur\\s+wirksam|nur\\s+in\\s+schriftform)[^.!?;:\\n]{0,60}?in\\s+(?:papierform|schriftform)[^.!?;:\\n]{0,60}?(?:originalunterschrift|eigenh(?:ä|ae)ndig|handschriftlich)", "i"),
  ],
};
// <<< mined patterns
export const STATUTORY_CITATIONS = Object.freeze({
  auto_renewal: "§ 309 Nr. 9 lit. b BGB, § 307 Abs. 1 BGB",
  notice_period: "§ 309 Nr. 9 lit. c BGB, § 307 Abs. 1 BGB",
  liability: "§ 309 Nr. 7 lit. a, b BGB, § 307 Abs. 2 Nr. 2 BGB",
  penalty: "§ 309 Nr. 5, Nr. 6 BGB, § 307 Abs. 1 BGB",
  jurisdiction: "§ 38 ZPO, § 307 Abs. 1 BGB",
  non_compete: "§ 307 Abs. 1 BGB, § 138 BGB, § 90a HGB",
  data_grant: "§ 307 Abs. 1, Abs. 2 Nr. 1 BGB, § 31 Abs. 5 UrhG",
  default_interest: "§ 288 Abs. 1, Abs. 2 BGB, § 309 Nr. 5 BGB",
  unilateral_change: "§ 308 Nr. 4, Nr. 5 BGB, § 307 Abs. 1 BGB",
  setoff_restriction: "§ 309 Nr. 3 BGB, § 307 Abs. 1 BGB",
  deemed_acceptance: "§ 308 Nr. 5 BGB, § 309 Nr. 8 lit. b ee BGB, § 640 Abs. 2 BGB",
  statute_of_limitations: "§ 309 Nr. 8 lit. b ff BGB, § 202 Abs. 1 BGB, § 307 Abs. 1 BGB",
  written_form_trap: "§ 309 Nr. 13 BGB, § 305b BGB",
  burden_of_proof: "§ 309 Nr. 12 BGB, § 307 Abs. 2 Nr. 1 BGB",
  dynamic_url_referral: "§ 305 Abs. 2 BGB, § 307 Abs. 1 S. 2 BGB (BGH III ZR 59/24)",
  vicarious_agents: "§ 278 BGB, § 309 Nr. 7 BGB, § 307 Abs. 2 Nr. 2 BGB",
  unreasonable_audit: "§ 307 Abs. 1 S. 1, S. 2 BGB",
  data_lockin: "§ 307 Abs. 1, Abs. 2 Nr. 2 BGB, Art. 20, Art. 28 Abs. 3 lit. g DSGVO",
  third_party_indemnity: "§ 307 Abs. 1, Abs. 2 Nr. 1 BGB, § 276 BGB",
  asymmetric_termination: "§ 314 Abs. 1 BGB, § 626 Abs. 1 BGB, § 307 Abs. 2 Nr. 1 BGB",
  assignment_restriction: "§ 308 Nr. 9 BGB, § 354a Abs. 1 HGB",
  force_majeure_asymmetry: "§ 326 Abs. 1 S. 1 BGB, § 307 Abs. 2 Nr. 1 BGB",
  non_solicitation: "§ 307 Abs. 1 BGB, § 75f HGB, Art. 12 Abs. 1 GG",
  unilateral_convenience_termination: "§ 307 Abs. 1, Abs. 2 Nr. 1 BGB, § 308 Nr. 3 BGB",
  excessive_survival: "§ 307 Abs. 1 S. 1, S. 2 BGB",
  price_escalation: "§ 309 Nr. 1 BGB, § 307 Abs. 1 S. 1, S. 2 BGB",
  salvatorische_klausel_falle: "§ 306 Abs. 1, Abs. 2 BGB, § 307 Abs. 1 S. 2 BGB",
  extended_retention_of_title: "§ 307 Abs. 1 BGB, § 138 Abs. 1 BGB (BGH GSZ 1/97)",
  subcontractor_uncontrolled: "§ 307 Abs. 1 BGB, Art. 28 Abs. 2 DSGVO",
  audit_log_manipulation: "§ 309 Nr. 12 BGB, § 307 Abs. 1 BGB",
  service_suspension: "§ 320 Abs. 2 BGB, § 307 Abs. 1, Abs. 2 Nr. 1 BGB",
  insolvency_termination: "§ 119 InsO, § 103 InsO, § 307 Abs. 2 Nr. 1 BGB (BGH IX ZR 213/11)",
  tax_indemnity_shift: "§ 307 Abs. 1 S. 1, S. 2 BGB",
  asymmetric_confidentiality: "§ 307 Abs. 1 S. 1 BGB",
  as_is_software_disclaimer: "§ 309 Nr. 8 lit. b aa BGB, § 536d BGB, § 307 Abs. 2 Nr. 2 BGB",
  entire_agreement: "§ 305b BGB, § 307 Abs. 1 BGB, § 309 Nr. 12 lit. b BGB",
  marketing_reference: "§ 307 Abs. 1, Abs. 2 Nr. 1 BGB, Art. 6 Abs. 1 lit. a DSGVO",
  feedback_ip_grab: "§ 307 Abs. 1, Abs. 2 Nr. 1 BGB, § 31 Abs. 5 UrhG",
  fictitious_receipt: "§ 308 Nr. 6 BGB, § 130 Abs. 1 S. 1 BGB",
  unverbindliche_sla_falle: "§ 307 Abs. 1 S. 2, Abs. 2 Nr. 2 BGB",
  agency_commission_exclusion: "§ 87 Abs. 2 HGB, § 307 Abs. 2 Nr. 1 BGB",
  agency_goodwill_waiver: "§ 89b Abs. 4 S. 1 HGB",
  international_data_processing: "Art. 44–49 DSGVO, Art. 28 Abs. 3 DSGVO, § 307 Abs. 1 BGB",
  mandatory_expert_determination: "§ 307 Abs. 1 BGB, § 319 Abs. 1 BGB",
  service_credit_forfeiture: "§ 307 Abs. 1, Abs. 2 Nr. 2 BGB",
  early_termination_charge: "§ 309 Nr. 5 lit. a, b BGB, § 648 S. 2 BGB, § 307 Abs. 2 Nr. 1 BGB",
  maintenance_conditional_shutdown: "§ 307 Abs. 1, Abs. 2 Nr. 2 BGB",
});

for (const rule of RULES) {
  const extra = (PATTERN_SUPPLEMENTS[rule.type] || []).concat(MINED_SUPPLEMENTS[rule.type] || []);
  if (extra.length) rule.patterns = (rule.patterns || []).concat(extra);
  if (!rule.citation && STATUTORY_CITATIONS[rule.type]) {
    rule.citation = STATUTORY_CITATIONS[rule.type];
  }
}

function buildSummary(findings, counts, riskScore, riskLevel) {
  const n = findings.length;
  if (n === 0) {
    return "Keine auff\u00e4lligen Klauseln gefunden. Gesamtrisiko: niedrig (0/100).";
  }
  const head = `${n} ${n === 1 ? "Befund" : "Befunde"}: ${counts.hoch} \u00d7 hoch, ${counts.mittel} \u00d7 mittel, ${counts.niedrig} \u00d7 niedrig. Gesamtrisiko: ${riskLevel} (${riskScore}/100).`;
  const tail =
    counts.hoch > 0
      ? "Empfehlung: Diese Stellen vor Unterschrift dringend nachverhandeln und idealerweise anwaltlich pr\u00fcfen lassen."
      : "Empfehlung: Gehen Sie den kleineren Punkten nach, bevor Sie unterschreiben.";
  return `${head} ${tail}`;
}

/**
 * Analysiert einen Vertragstext (Deutsch) und liefert
 * { schemaVersion, findings, riskScore, riskLevel, counts, summary }.
 */
export function analyzeContract(text, options = {}) {
  if (typeof text !== "string") {
    throw new TypeError("analyzeContract erwartet einen String");
  }
  text = normalizeText(text);

  const config = options.config || mergeConfig(options.userConfig, options.profile);
  const activeWeights = config.weights || SEVERITY_WEIGHT;
  const activeThresholds = config.thresholds || { hoch: 70, mittel: 35 };

  let activeRules = [...RULES];
  if (Array.isArray(config.activeDetectors) && config.activeDetectors.length > 0) {
    const activeSet = new Set(config.activeDetectors);
    activeRules = activeRules.filter((r) => activeSet.has(r.type));
  }
  if (Array.isArray(config.customRules) && config.customRules.length > 0) {
    activeRules = activeRules.concat(config.customRules);
  }

  const sentences = segmentSentences(text);
  attachContinuations(sentences, text);
  attachCrossRefs(sentences, text);
  const lineStarts = lineStartsOf(text);

  // 1) Kandidaten sammeln (eine Regel max. einmal je Satz)
  const candidates = [];
  for (const rule of activeRules) {
    const hits = rule.collect ? rule.collect(sentences, text) : collectPerSentence(rule, sentences);
    for (const hit of hits) {
      candidates.push({
        rule,
        sentence: sentences[hit.i],
        severity: hit.severity,
        note: hit.note || null,
      });
    }
  }

  // Aufgeloeste Anlagenverweise werden erst geprueft, wenn der verweisende Satz
  // selbst keinen Treffer hat; so entstehen keine Doppelmeldungen.
  const uncoveredRefs = sentences.filter(
    (s) => s.crossRefText && !candidates.some((c) => c.sentence === s)
  );
  if (uncoveredRefs.length) {
    const only = new Set(uncoveredRefs);
    const extra = new Map(uncoveredRefs.map((s) => [s, s.crossRefText]));
    for (const rule of activeRules) {
      const hits = rule.collect
        ? rule.collect(sentences, text, extra)
        : collectPerSentence(rule, sentences, only, extra);
      for (const hit of hits) {
        const sentence = sentences[hit.i];
        // Collect-Regeln sehen alle Saetze; hier zaehlt nur der verweisende Satz.
        if (rule.collect && !only.has(sentence)) continue;
        candidates.push({
          rule,
          sentence,
          severity: hit.severity,
          note: hit.note || null,
        });
      }
    }
  }

  // 2) Dedup: gleichartige Treffer im selben Absatz zusammenfassen
  const byType = new Map();
  for (const cand of candidates) {
    if (!byType.has(cand.rule.type)) byType.set(cand.rule.type, []);
    byType.get(cand.rule.type).push(cand);
  }

  const groups = [];
  for (const list of byType.values()) {
    list.sort((a, b) => a.sentence.start - b.sentence.start);
    let current = [list[0]];
    for (let k = 1; k < list.length; k++) {
      const last = current[current.length - 1];
      const between = text.slice(last.sentence.end, list[k].sentence.start);
      if (!/\r?\n[ \t]*\r?\n/.test(between)) {
        current.push(list[k]); // gleicher Absatz -> zusammenfassen
      } else {
        groups.push(current);
        current = [list[k]];
      }
    }
    groups.push(current);
  }

  // 3) Befunde bauen
  const findings = groups.map((items) => {
    const first = items[0];
    const last = items[items.length - 1];
    const rule = first.rule;
    const severity = items.reduce(
      (acc, it) => (RANK[it.severity] > RANK[acc] ? it.severity : acc),
      "niedrig"
    );
    let excerpt = items.map((it) => it.sentence.text).join(" ").replace(/\s+/g, " ").trim();
    if (excerpt.length > 420) excerpt = `${excerpt.slice(0, 417)}\u2026`;
    const ctx = {
      sentence: first.sentence.text,
      severity,
      note: first.note,
      items,
    };
    const why = typeof rule.why === "function" ? rule.why(ctx) : rule.why;
    const title = typeof rule.title === "function" ? rule.title(ctx) : rule.title;
    const titlePlainGerman = typeof rule.titlePlainGerman === "function"
      ? rule.titlePlainGerman(ctx)
      : (rule.titlePlainGerman || title);
    const suggestion = typeof rule.suggestion === "function" ? rule.suggestion(ctx) : rule.suggestion;
    const citation = typeof rule.citation === "function"
      ? rule.citation(ctx)
      : (rule.citation || STATUTORY_CITATIONS[rule.type] || "§ 307 Abs. 1 BGB");
    const lineNumbers = [
      lineOf(lineStarts, first.sentence.start),
      lineOf(lineStarts, Math.max(first.sentence.start, last.sentence.end - 1)),
    ];
    const id = `cl-${rule.type}-${lineNumbers[0]}-${lineNumbers[1]}`;
    const confidence = rule.isCustom ? 0.9 : severity === "hoch" ? 0.98 : 0.92;
    const tags = Array.isArray(rule.tags) ? [...rule.tags] : [rule.type];

    return {
      id,
      type: rule.type,
      title,
      titlePlainGerman,
      severity,
      confidence,
      citation,
      excerpt,
      lineNumbers,
      why,
      suggestion,
      tags,
    };
  });

  findings.sort(
    (a, b) =>
      RANK[b.severity] - RANK[a.severity] || a.lineNumbers[0] - b.lineNumbers[0]
  );

  const counts = { hoch: 0, mittel: 0, niedrig: 0 };
  for (const f of findings) counts[f.severity]++;
  const riskScore = Math.min(
    100,
    findings.reduce((sum, f) => sum + (activeWeights[f.severity] ?? SEVERITY_WEIGHT[f.severity]), 0)
  );
  const riskLevel =
    riskScore >= activeThresholds.hoch
      ? "hoch"
      : riskScore >= activeThresholds.mittel
      ? "mittel"
      : "niedrig";
  const summary = buildSummary(findings, counts, riskScore, riskLevel);

  return {
    schemaVersion: "1.0.0",
    riskScore,
    riskLevel,
    counts,
    summary,
    findings,
  };
}

"use strict";
import fs from "node:fs";
import path from "node:path";

export function generateEngineBundle() {
  const textioSrc = fs.readFileSync("src/textio.js", "utf8");
  const analyzeSrc = fs.readFileSync("src/analyze.js", "utf8");

  // Part A: from MOJIBAKE_RE down to decodeBuffer
  const pAStart = textioSrc.indexOf("const MOJIBAKE_RE =");
  const pAEnd = textioSrc.indexOf("export function decodeBuffer");
  const partA = textioSrc.slice(pAStart, pAEnd).replace(/export\s+/g, "");

  // Part B: from splitLines down to loadDocument (includes segmentSentences!)
  const pBStart = textioSrc.indexOf("export function splitLines");
  const pBEnd = textioSrc.indexOf("export function loadDocument");
  const partB = textioSrc.slice(pBStart, pBEnd).replace(/export\s+/g, "");

  const configSnippet = `
const POLICY_PROFILES = {
  default: {
    name: "default",
    label: "Standard BGB/AGB",
    weights: { niedrig: 2, mittel: 7, hoch: 15 },
    thresholds: { hoch: 70, mittel: 35 },
  },
  strict: {
    name: "strict",
    label: "Streng (Kaufmännisch)",
    weights: { niedrig: 4, mittel: 10, hoch: 20 },
    thresholds: { hoch: 50, mittel: 25 },
  },
  lenient: {
    name: "lenient",
    label: "Milde Vorprüfung",
    weights: { niedrig: 1, mittel: 5, hoch: 10 },
    thresholds: { hoch: 80, mittel: 45 },
  }
};
const DEFAULT_CONFIG = {
  version: "1.0.0",
  profile: "default",
  weights: { ...POLICY_PROFILES.default.weights },
  thresholds: { ...POLICY_PROFILES.default.thresholds },
  activeDetectors: null,
  customRules: [],
};
function mergeConfig(userConfig = {}) {
  const profile = POLICY_PROFILES[userConfig.profile] || POLICY_PROFILES.default;
  return {
    ...DEFAULT_CONFIG,
    ...userConfig,
    weights: { ...profile.weights, ...(userConfig.weights || {}) },
    thresholds: { ...profile.thresholds, ...(userConfig.thresholds || {}) },
  };
}
`;

  const analyzeStart = analyzeSrc.indexOf("export const SEVERITIES =");
  let analyzeCode = analyzeSrc.slice(analyzeStart);
  analyzeCode = analyzeCode.replace(/export\s+const\s+/g, "const ");
  analyzeCode = analyzeCode.replace(/export\s+function\s+/g, "function ");
  analyzeCode = analyzeCode.replace(/export\s+class\s+/g, "class ");

  return [
    "    // ===== client engine: synchron zu src/textio.js + src/analyze.js =====",
    "    // (automatisch generiert aus Server-Quellen)",
    partA,
    partB,
    configSnippet,
    analyzeCode,
    ""
  ].join("\n");
}

export function syncFile(targetPath) {
  if (!fs.existsSync(targetPath)) return;
  const html = fs.readFileSync(targetPath, "utf8");
  const bundle = generateEngineBundle();

  const startMarker = "    // ===== client engine: synchron zu src/textio.js + src/analyze.js =====";
  const endMarker = "    function runClientAnalysis() {";

  const sIdx = html.indexOf(startMarker);
  const eIdx = html.indexOf(endMarker);

  if (sIdx === -1 || eIdx === -1) {
    throw new Error(`Markers not found in ${targetPath}! sIdx: ${sIdx}, eIdx: ${eIdx}`);
  }

  const clientAnalysisPart = html.slice(eIdx);
  const newHtml = html.slice(0, sIdx) + bundle + clientAnalysisPart;
  fs.writeFileSync(targetPath, newHtml, "utf8");
  console.log(`Successfully rebuilt and synced ${targetPath}! New length: ${newHtml.length}`);
}

export function syncAll() {
  syncFile("index.html");
  syncFile("studio.html");
}

if (process.argv[1] && process.argv[1].endsWith("sync_html_engine.mjs")) {
  syncAll();
}

"use strict";

import { readFileSync, existsSync } from "node:fs";
import path from "node:path";

export const POLICY_PROFILES = Object.freeze({
  default: {
    name: "default",
    label: "Standard BGB/AGB",
    description: "Allgemeine ausgewogene Risikobewertung nach §§ 305–310 BGB für Standardverträge.",
    weights: { niedrig: 2, mittel: 7, hoch: 15 },
    thresholds: { hoch: 70, mittel: 35 },
  },
  b2b_saas: {
    name: "b2b_saas",
    label: "SaaS & Cloud Software",
    description: "Verschärfte Prüfung auf einseitige Leistungsänderungen, Datenrechte und Vendor-Lock-in.",
    weights: { niedrig: 3, mittel: 8, hoch: 18 },
    thresholds: { hoch: 65, mittel: 30 },
  },
  freelance_consulting: {
    name: "freelance_consulting",
    label: "Freelancer & Beratung",
    description: "Kritischer Fokus auf nachvertragliche Wettbewerbsverbote, unfaire Haftung und Pönalen.",
    weights: { niedrig: 2, mittel: 9, hoch: 20 },
    thresholds: { hoch: 60, mittel: 30 },
  },
  procurement_strict: {
    name: "procurement_strict",
    label: "Einkauf & Großkunden",
    description: "Strikte Risikotoleranz für Einkaufsabteilungen mit schnellem Eskalationslevel.",
    weights: { niedrig: 4, mittel: 10, hoch: 22 },
    thresholds: { hoch: 55, mittel: 25 },
  },
});

export const DEFAULT_CONFIG = Object.freeze({
  version: "1.0.0",
  profile: "default",
  weights: { ...POLICY_PROFILES.default.weights },
  thresholds: { ...POLICY_PROFILES.default.thresholds },
  activeDetectors: null, // null means all
  customRules: [],
});

/**
 * Loads and merges configuration from a file path or known config locations.
 */
export function loadConfig(configPath = null) {
  let resolvedPath = configPath;

  if (!resolvedPath) {
    const candidates = [
      path.resolve(process.cwd(), "clauselens.config.json"),
      path.resolve(process.cwd(), ".clauselensrc.json"),
    ];
    for (const cand of candidates) {
      if (existsSync(cand)) {
        resolvedPath = cand;
        break;
      }
    }
  }

  if (resolvedPath) {
    if (!existsSync(resolvedPath)) {
      throw new Error(`Konfigurationsdatei nicht gefunden: ${resolvedPath}`);
    }
    try {
      const content = readFileSync(resolvedPath, "utf-8");
      const userConfig = JSON.parse(content);
      return mergeConfig(userConfig);
    } catch (err) {
      throw new Error(`Fehler beim Lesen von "${resolvedPath}": ${err.message}`);
    }
  }

  return { ...DEFAULT_CONFIG };
}

/**
 * Merges user configuration with policy profiles and defaults.
 */
export function mergeConfig(userConfig = {}, profileOverride = null) {
  const profileName = profileOverride || userConfig.profile || "default";
  const profile = POLICY_PROFILES[profileName] || POLICY_PROFILES.default;

  const weights = {
    ...profile.weights,
    ...(userConfig.weights || {}),
  };

  const thresholds = {
    ...profile.thresholds,
    ...(userConfig.thresholds || {}),
  };

  const customRules = Array.isArray(userConfig.customRules)
    ? userConfig.customRules.map((r) => compileCustomRule(r))
    : [];

  return {
    version: userConfig.version || "1.0.0",
    profile: profileName,
    profileLabel: profile.label,
    profileDescription: profile.description,
    weights,
    thresholds,
    activeDetectors: Array.isArray(userConfig.activeDetectors) ? userConfig.activeDetectors : null,
    customRules,
  };
}

/**
 * Compiles a JSON custom rule into an engine-compatible rule object.
 */
function compileCustomRule(ruleDef) {
  if (!ruleDef.type || !ruleDef.title || !ruleDef.patterns) {
    throw new Error(`Ungültige benutzerdefinierte Regel: "type", "title" und "patterns" erforderlich.`);
  }

  const regexes = (ruleDef.patterns || []).map((p) => (p instanceof RegExp ? p : new RegExp(p, "i")));
  const sev = ["hoch", "mittel", "niedrig"].includes(ruleDef.severity) ? ruleDef.severity : "mittel";

  return {
    type: ruleDef.type,
    title: ruleDef.title,
    patterns: regexes,
    severity: sev,
    why: ruleDef.why || "Benutzerdefinierte Risikoklausel laut Firmenrichtlinie.",
    suggestion: ruleDef.suggestion || "Prüfen Sie diesen Punkt gemäß Ihrer internen Compliance.",
    tags: ruleDef.tags || ["custom-rule"],
    isCustom: true,
  };
}

import { selectRules, getRuleById } from "./rules/index.js";
import { getProfile, resolveProfileCode, PROFILES } from "./profiles.js";
import { getMunicipality, resolveMunicipalityCode } from "./municipalities.js";

export const VERDICT = {
  BESTANDEN: "bestanden",
  AUFLAGEN: "mit Auflagen",
  NICHT_BESTANDEN: "nicht bestanden",
};

export function resolveProfile(spec, options = {}) {
  const code = resolveProfileCode(spec, options.land);
  const profile = getProfile(code);
  if (!profile) return null;
  return { code, ...profile };
}

export function runChecks(spec, options = {}) {
  const profile = resolveProfile(spec, options);
  if (!profile) return null;

  // Resolve municipality
  const muniCode = resolveMunicipalityCode(spec, options.stadt ?? options.kommune);
  const municipality = muniCode ? getMunicipality(muniCode) : null;

  let rules = options.rules ? selectRules(options.rules) : [...selectRules(null)];
  const filterIds = Array.isArray(options.rules) && options.rules.length > 0
    ? new Set(options.rules)
    : null;

  if (!filterIds) {
    for (const extraId of profile.extraRules) {
      const rule = getRuleById(extraId);
      if (rule) rules.push(rule);
    }
  }

  const results = [];
  for (const baseRule of rules) {
    const override = profile.ruleOverrides[baseRule.id];
    if (override?.enabled === false) continue;

    let para = override?.para ?? baseRule.para;
    let title = override?.title ?? baseRule.title;

    // Apply municipal statute citation overlay if present
    if (baseRule.id === "stellplaetze" && municipality?.satzungen?.stellplatz) {
      para = municipality.satzungen.stellplatz.para;
      title = `${baseRule.title} (${municipality.name})`;
    } else if (baseRule.id === "gruendach" && municipality?.satzungen?.gruendach) {
      para = municipality.satzungen.gruendach.para;
      title = `${baseRule.title} (${municipality.name})`;
    }

    const rule = { ...baseRule, para, title };
    const ctx = {
      land: profile.code,
      params: override?.params ?? {},
      municipality,
    };

    let findings;
    try {
      findings = rule.evaluate(spec, ctx) ?? [];
    } catch (err) {
      findings = [{ status: "warn", message: `Regel konnte nicht ausgewertet werden: ${err.message}` }];
    }
    results.push({ id: rule.id, para: rule.para, title: rule.title, findings });
  }

  const counts = { ok: 0, warn: 0, fail: 0 };
  for (const result of results) {
    for (const finding of result.findings) {
      if (finding.status in counts) counts[finding.status] += 1;
    }
  }
  counts.total = counts.ok + counts.warn + counts.fail;

  const verdict =
    counts.fail > 0 ? VERDICT.NICHT_BESTANDEN
    : counts.warn > 0 ? VERDICT.AUFLAGEN
    : VERDICT.BESTANDEN;

  return { results, counts, verdict, profile, municipality };
}

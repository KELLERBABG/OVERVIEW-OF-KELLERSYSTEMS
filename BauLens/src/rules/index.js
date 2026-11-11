import gebaeudeklasse from "./gebaeudeklasse.js";
import abstandsflaeche from "./abstandsflaeche.js";
import grz from "./grz.js";
import gfz from "./gfz.js";
import stellplaetze from "./stellplaetze.js";
import tageslicht from "./tageslicht.js";
import raumhoehen from "./raumhoehen.js";
import rettungswege from "./rettungswege.js";
import vollgeschosse from "./vollgeschosse.js";
import grenzbebauung from "./grenzbebauung.js";
import gruendach from "./gruendach.js";
import brandwand from "./brandwand.js";
import barrierefreiheit from "./barrierefreiheit.js";
import schallschutz from "./schallschutz.js";
import bplan_festsetzungen from "./bplan_festsetzungen.js";

export const ALL_RULES = [
  gebaeudeklasse,
  abstandsflaeche,
  grz,
  gfz,
  stellplaetze,
  tageslicht,
  raumhoehen,
  rettungswege,
  vollgeschosse,
];

export const EXTRA_RULES = [
  grenzbebauung,
  gruendach,
  brandwand,
  barrierefreiheit,
  schallschutz,
  bplan_festsetzungen,
];

const BY_ID = new Map([...ALL_RULES, ...EXTRA_RULES].map((r) => [r.id, r]));

export function getRuleById(id) {
  return BY_ID.get(id) ?? null;
}

export function selectRules(ids) {
  if (!Array.isArray(ids) || ids.length === 0) return ALL_RULES;
  const wanted = new Set(ids);
  return [...ALL_RULES, ...EXTRA_RULES].filter((rule) => wanted.has(rule.id));
}

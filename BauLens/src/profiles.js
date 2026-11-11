export const MBO_SETBACK = { factor: 0.5, minM: 3 };
export const BW_SETBACK = { factor: 0.5, minM: 5 };

function mboAehnlich(name, citationBase, notes = []) {
  return {
    name,
    citationBase,
    regimeSummary: "MBO-ähnlich (Default-Parameter)",
    ruleOverrides: {},
    extraRules: [],
    notes: [
      `Keine flächendeckend gesicherten Abweichungen hinterlegt – MBO-Defaultwerte werden verwendet.`,
      ...notes,
    ],
    confidence: "default",
  };
}

export const PROFILES = {
  MBO: {
    name: "Musterbauordnung (Referenz)",
    citationBase: "MBO",
    regimeSummary: "MBO 2002 (Fassung 2016) als Referenz ohne Landesabweichungen",
    ruleOverrides: {},
    extraRules: [],
    notes: [],
    confidence: "reference",
  },
  BW: {
    name: "Baden-Württemberg",
    citationBase: "LBO BW",
    regimeSummary: "Abstandsflächen mit erhöhtem Mindestmaß (5 m)",
    ruleOverrides: {
      abstandsflaeche: { params: BW_SETBACK, para: "§6 LBO BW", pruefen: true },
      raumhoehen: { para: "§46 LBO BW" },
      rettungswege: { para: "§11 LBO BW" },
      gebaeudeklasse: { para: "§2 LBO BW" },
      tageslicht: { para: "§49 LBO BW" },
      vollgeschosse: { para: "§16 BauNVO i.V.m. LBO BW" },
    },
    extraRules: [],
    notes: [
      "Mindestabstandsfläche von 5 m ist die häufig zitierte Auslegung des §6 LBO BW [prüfen].",
    ],
    confidence: "pruefen",
  },
  BY: {
    name: "Bayern",
    citationBase: "BayBO",
    regimeSummary: "Keine Abstandsflächen – stattdessen Grenzbebauung nach §22 BayBO",
    ruleOverrides: {
      abstandsflaeche: { enabled: false },
      gebaeudeklasse: { para: "§2 BayBO" },
      raumhoehen: { para: "§46 BayBO" },
      rettungswege: { para: "§11 BayBO" },
      tageslicht: { para: "§49 BayBO" },
      vollgeschosse: { para: "§16 BauNVO i.V.m. BayBO" },
    },
    extraRules: ["grenzbebauung"],
    notes: [
      "Die BayBO kennt kein Abstandsflächenregime nach §6 MBO; maßgeblich ist die Grenzbebauung nach §22 BayBO.",
      "Ob bis an die Grenze gebaut werden darf, hängt von Bebauungsplan, Innenbereichslage und Grenzwand-Anforderungen ab – im MVP nur Hinweis, keine Prüfung.",
    ],
    confidence: "hoch",
  },
  BE: {
    name: "Berlin",
    citationBase: "BauO Bln",
    regimeSummary: "Abstandsflächen wie MBO, zusätzlich großzügige Grenzbebauung möglich",
    ruleOverrides: {
      gebaeudeklasse: { para: "§2 BauO Bln" },
      raumhoehen: { para: "§46 BauO Bln" },
      rettungswege: { para: "§11 BauO Bln" },
      tageslicht: { para: "§49 BauO Bln" },
      vollgeschosse: { para: "§16 BauNVO i.V.m. BauO Bln" },
    },
    extraRules: ["grenzbebauung"],
    notes: [
      "Berlin erlaubt unter Bedingungen das Bauen an/bis zur Grundstücksgrenze; die Abstandsflächen-Defaults folgen der MBO [prüfen].",
    ],
    confidence: "hoch",
  },
  NW: {
    name: "Nordrhein-Westfalen",
    citationBase: "BauO NRW",
    regimeSummary: "Abstandsflächen mit 0,4 H (mind. 3 m) nach §6 BauO NRW 2021",
    ruleOverrides: {
      abstandsflaeche: { params: { factor: 0.4, minM: 3 }, para: "§6 Abs. 5 BauO NRW" },
      gebaeudeklasse: { para: "§2 Abs. 3 BauO NRW" },
      raumhoehen: { para: "§46 BauO NRW" },
      rettungswege: { para: "§33 BauO NRW" },
      tageslicht: { para: "§47 BauO NRW" },
      stellplaetze: { para: "§48 BauO NRW" },
      vollgeschosse: { para: "§2 Abs. 6 BauO NRW" },
    },
    extraRules: [],
    notes: [
      "In NRW beträgt die Tiefe der Abstandsfläche 0,4 H, mindestens jedoch 3,00 m (§6 Abs. 5 BauO NRW).",
      "In Gewerbe- und Industriegebieten genügt 0,2 H.",
    ],
    confidence: "hoch",
  },
  HE: {
    name: "Hessen",
    citationBase: "HBO",
    regimeSummary: "Abstandsflächen mit 0,4 H (mind. 3 m) nach §6 HBO",
    ruleOverrides: {
      abstandsflaeche: { params: { factor: 0.4, minM: 3 }, para: "§6 Abs. 5 HBO" },
      gebaeudeklasse: { para: "§2 Abs. 3 HBO" },
      raumhoehen: { para: "§46 HBO" },
      rettungswege: { para: "§33 HBO" },
      tageslicht: { para: "§47 HBO" },
      stellplaetze: { para: "§52 HBO" },
      vollgeschosse: { para: "§2 Abs. 4 HBO" },
    },
    extraRules: [],
    notes: [
      "Hessen wendet 0,4 H (mind. 3 m) für allgemeine Baugebiete an (§6 Abs. 5 HBO).",
    ],
    confidence: "hoch",
  },
  SN: {
    name: "Sachsen",
    citationBase: "SächsBO",
    regimeSummary: "Abstandsflächen mit 0,4 H (mind. 3 m) nach §6 SächsBO",
    ruleOverrides: {
      abstandsflaeche: { params: { factor: 0.4, minM: 3 }, para: "§6 Abs. 5 SächsBO" },
      gebaeudeklasse: { para: "§2 Abs. 3 SächsBO" },
      raumhoehen: { para: "§46 SächsBO" },
      rettungswege: { para: "§33 SächsBO" },
      tageslicht: { para: "§47 SächsBO" },
      stellplaetze: { para: "§49 SächsBO" },
      vollgeschosse: { para: "§2 Abs. 5 SächsBO" },
    },
    extraRules: [],
    notes: [
      "In Sachsen gilt der Abstandsflächenfaktor 0,4 H (mind. 3 m) gem. §6 Abs. 5 SächsBO.",
    ],
    confidence: "hoch",
  },
  TH: {
    name: "Thüringen",
    citationBase: "ThürBO",
    regimeSummary: "Abstandsflächen mit 0,4 H (mind. 3 m) nach §6 ThürBO",
    ruleOverrides: {
      abstandsflaeche: { params: { factor: 0.4, minM: 3 }, para: "§6 Abs. 5 ThürBO" },
      gebaeudeklasse: { para: "§2 Abs. 3 ThürBO" },
      raumhoehen: { para: "§46 ThürBO" },
      rettungswege: { para: "§33 ThürBO" },
      tageslicht: { para: "§47 ThürBO" },
      stellplaetze: { para: "§49 ThürBO" },
      vollgeschosse: { para: "§2 Abs. 5 ThürBO" },
    },
    extraRules: [],
    notes: [
      "Thüringen hat den Faktor auf 0,4 H (mind. 3 m) harmonisiert (§6 Abs. 5 ThürBO).",
    ],
    confidence: "hoch",
  },
  HH: {
    name: "Hamburg",
    citationBase: "HBauO",
    regimeSummary: "Abstandsflächen mit 0,4 H (mind. 3 m) nach §6 HBauO",
    ruleOverrides: {
      abstandsflaeche: { params: { factor: 0.4, minM: 3 }, para: "§6 Abs. 5 HBauO" },
      gebaeudeklasse: { para: "§2 Abs. 3 HBauO" },
      raumhoehen: { para: "§46 HBauO" },
      rettungswege: { para: "§33 HBauO" },
      tageslicht: { para: "§47 HBauO" },
      stellplaetze: { para: "§48 HBauO" },
      vollgeschosse: { para: "§2 Abs. 6 HBauO" },
    },
    extraRules: [],
    notes: [
      "Hamburg verwendet den Faktor 0,4 H (mind. 3 m) nach §6 Abs. 5 HBauO.",
    ],
    confidence: "hoch",
  },
  NI: {
    name: "Niedersachsen",
    citationBase: "NBauO",
    regimeSummary: "Abstandsflächen mit 0,5 H (mind. 3 m) nach §5 NBauO",
    ruleOverrides: {
      abstandsflaeche: { params: { factor: 0.5, minM: 3 }, para: "§5 Abs. 2 NBauO" },
      gebaeudeklasse: { para: "§2 Abs. 3 NBauO" },
      raumhoehen: { para: "§43 NBauO" },
      rettungswege: { para: "§33 NBauO" },
      tageslicht: { para: "§44 NBauO" },
      stellplaetze: { para: "§47 NBauO" },
      vollgeschosse: { para: "§2 Abs. 7 NBauO" },
    },
    extraRules: [],
    notes: [
      "Niedersachsen bemisst Abstandsflächen nach §5 NBauO (Grundregel 0,5 H, mind. 3 m).",
    ],
    confidence: "hoch",
  },
  BB: mboAehnlich("Brandenburg", "BbgBauO"),
  HB: mboAehnlich("Bremen", "BremLBO"),
  MV: mboAehnlich("Mecklenburg-Vorpommern", "LBauO MV"),
  RP: mboAehnlich("Rheinland-Pfalz", "LBauO RP"),
  SL: mboAehnlich("Saarland", "LBO-Saar"),
  ST: mboAehnlich("Sachsen-Anhalt", "BauO LSA"),
  SH: mboAehnlich("Schleswig-Holstein", "LBO SH"),
};

// Integrate Austria (AT) and Switzerland (CH) profiles
import { DACH_REGIONS, resolveCountry } from "./dach.js";

for (const [code, reg] of Object.entries(DACH_REGIONS)) {
  const isAT = reg.country === "AT";
  const isCH = reg.country === "CH";

  PROFILES[code] = {
    name: reg.name,
    citationBase: reg.citationBase,
    regimeSummary: reg.regimeSummary,
    authorityTitle: reg.authorityTitle,
    country: reg.country,
    ruleOverrides: {
      abstandsflaeche: {
        params: { factor: reg.setbackFactor, minM: reg.minSetbackM },
        para: isAT ? "§79 BO / OIB RL" : "IVHB / PBG",
        pruefen: true,
      },
      gebaeudeklasse: {
        para: isAT ? "OIB-Richtlinie 2 (Tabelle 1)" : "IVHB / PBG",
      },
      raumhoehen: {
        para: isAT ? "OIB-Richtlinie 3 (Pkt. 7)" : "PBG / IVHB",
      },
      rettungswege: {
        para: isAT ? "OIB-Richtlinie 2.2" : "VKF Brandschutznorm (CH)",
      },
      tageslicht: {
        para: isAT ? "OIB-Richtlinie 3 (Pkt. 8.1)" : "SIA 500 / PBG",
      },
      vollgeschosse: {
        para: isAT ? "ROG / Landesbauordnung" : "IVHB Art. 4.1",
      },
    },
    extraRules: [],
    notes: [
      `Geltungsbereich: ${reg.country === "AT" ? "Österreich" : "Schweiz"}.`,
      `Zuständige Behörde: ${reg.authorityTitle}.`,
    ],
    confidence: reg.confidence,
  };
}

export const PROFILE_CODES = Object.keys(PROFILES);

export function getProfile(code) {
  const key = typeof code === "string" ? code.trim().toUpperCase() : "";
  return PROFILES[key] ?? null;
}

export function resolveProfileCode(spec, cliLand) {
  if (cliLand) return cliLand.trim().toUpperCase();

  // Explicit region code (e.g. "AT_W", "CH_ZH", "BY")
  const explicit = spec?.bundesland || spec?.kanton || spec?.region;
  if (typeof explicit === "string" && explicit.trim() !== "") {
    const raw = explicit.trim().toUpperCase();
    if (PROFILES[raw]) return raw;
    if (PROFILES[`AT_${raw}`]) return `AT_${raw}`;
    if (PROFILES[`CH_${raw}`]) return `CH_${raw}`;
  }

  // Country-level fallback
  const country = resolveCountry(spec);
  if (country === "AT") return "AT_OIB";
  if (country === "CH") return "CH_IVHB";
  return "MBO";
}

export function formatLandsTable() {
  const rows = [["Code", "Land", "Regime", "Vertrauensstufe"]];
  for (const code of PROFILE_CODES) {
    const p = PROFILES[code];
    const trust =
      p.confidence === "reference" ? "Referenz"
      : p.confidence === "hoch" ? "hoch"
      : p.confidence === "pruefen" ? "prüfen"
      : "default";
    rows.push([code, p.name, p.regimeSummary, trust]);
  }
  const widths = [0, 1, 2, 3].map((i) => Math.max(...rows.map((r) => r[i].length)));
  const fmt = (r) => r.map((c, i) => c.padEnd(widths[i])).join("  ");
  return rows.map(fmt).join("\n");
}

/**
 * BauLens DACH Foundation Module
 * Country definitions, regional authorities, measurement frameworks and postal code resolution
 * for Germany (DE), Austria (AT), and Switzerland (CH).
 */

export const COUNTRIES = {
  DE: {
    code: "DE",
    name: "Deutschland",
    flag: "🇩🇪",
    authorityType: "Bauordnungsamt / Untere Bauaufsichtsbehörde",
    legalFramework: "MBO (Musterbauordnung) & 16 Landesbauordnungen (LBO)",
    planningLaw: "BauGB & BauNVO",
    ratios: {
      siteCoverage: "GRZ (Grundflächenzahl §19 BauNVO)",
      floorAreaRatio: "GFZ (Geschossflächenzahl §20 BauNVO)",
      buildingMass: "BMZ (Baumassenzahl §21 BauNVO)"
    },
    defaultSubdivision: "MBO"
  },
  AT: {
    code: "AT",
    name: "Österreich",
    flag: "🇦🇹",
    authorityType: "Baubehörde (Magistrat / Bezirkshauptmannschaft / Gemeinde)",
    legalFramework: "OIB-Richtlinien 1–6 & 9 Landesbauordnungen",
    planningLaw: "Raumordnungsgesetze (ROG der Länder) & Bauordnungen",
    ratios: {
      siteCoverage: "Bebauungsdichte / Bauplatzgröße",
      floorAreaRatio: "GFZ (Geschossflächenzahl)",
      buildingMass: "BMZ (Baumassenzahl)"
    },
    defaultSubdivision: "AT_OIB"
  },
  CH: {
    code: "CH",
    name: "Schweiz",
    flag: "🇨🇭",
    authorityType: "Bauinspektorat / Baubewilligungsbehörde",
    legalFramework: "IVHB & 26 Kantone (PBG)",
    planningLaw: "RPG & Kantonale Richtpläne",
    ratios: {
      siteCoverage: "ÜZ (Überbauungsziffer) / GRZ",
      floorAreaRatio: "AZ (Ausnützungsziffer) / GFZ",
      buildingMass: "BMZ (Baumassenziffer)"
    },
    defaultSubdivision: "CH_IVHB"
  }
};

export const DACH_REGIONS = {
  // --- Österreich (AT) Bundesländer ---
  AT_OIB: {
    country: "AT",
    code: "AT_OIB",
    name: "Österreich (OIB-Referenz)",
    citationBase: "OIB-Richtlinien 1–6 (Ausgabe 2023)",
    regimeSummary: "Österreichisches Institut für Bautechnik (OIB RL 1-6)",
    authorityTitle: "Österreichisches Institut für Bautechnik",
    setbackFactor: 0.5,
    minSetbackM: 3.0,
    confidence: "reference"
  },
  AT_W: {
    country: "AT",
    code: "AT_W",
    name: "Wien",
    citationBase: "BO für Wien",
    regimeSummary: "Wiener Bauordnung (Bauklassen I–VI, Wiener Abstandsbestimmungen §79)",
    authorityTitle: "Magistrat der Stadt Wien - MA 37 Baupolizei",
    setbackFactor: 0.5,
    minSetbackM: 3.0,
    confidence: "hoch"
  },
  AT_NOE: {
    country: "AT",
    code: "AT_NOE",
    name: "Niederösterreich",
    citationBase: "NÖ Bauordnung 2014",
    regimeSummary: "NÖ BauO (Bauwich mind. 3,0 m bzw. halbe Gebäudehöhe)",
    authorityTitle: "NÖ Baubehörde",
    setbackFactor: 0.5,
    minSetbackM: 3.0,
    confidence: "pruefen"
  },
  AT_OOE: {
    country: "AT",
    code: "AT_OOE",
    name: "Oberösterreich",
    citationBase: "Oö. Bauordnung",
    regimeSummary: "Oö. BauO (Mindestabstand zur Grenze 3,0 m)",
    authorityTitle: "Oö. Baubehörde",
    setbackFactor: 0.5,
    minSetbackM: 3.0,
    confidence: "pruefen"
  },
  AT_SBG: {
    country: "AT",
    code: "AT_SBG",
    name: "Salzburg",
    citationBase: "Salzburger BauPolG",
    regimeSummary: "Salzburger BauPolG (Mindestabstand 0,75 x H, mind. 4 m)",
    authorityTitle: "Salzburger Baubehörde",
    setbackFactor: 0.75,
    minSetbackM: 4.0,
    confidence: "pruefen"
  },
  AT_T: {
    country: "AT",
    code: "AT_T",
    name: "Tirol",
    citationBase: "Tiroler Bauordnung (TBO 2022)",
    regimeSummary: "TBO (Mindestabstand 0,6 x H, mind. 4,0 m / 3,0 m)",
    authorityTitle: "Tiroler Baubehörde",
    setbackFactor: 0.6,
    minSetbackM: 4.0,
    confidence: "pruefen"
  },
  AT_STMK: {
    country: "AT",
    code: "AT_STMK",
    name: "Steiermark",
    citationBase: "Stmk. BauG",
    regimeSummary: "Stmk. BauG (Grenzabstand mind. halbe Gebäudehöhe, mind. 3,0 m)",
    authorityTitle: "Steiermärkische Baubehörde",
    setbackFactor: 0.5,
    minSetbackM: 3.0,
    confidence: "pruefen"
  },
  AT_KTN: {
    country: "AT",
    code: "AT_KTN",
    name: "Kärnten",
    citationBase: "Kärntner Bauordnung (K-BO)",
    regimeSummary: "K-BO (Mindestabstand 0,6 x H, mind. 3,0 m)",
    authorityTitle: "Kärntner Baubehörde",
    setbackFactor: 0.6,
    minSetbackM: 3.0,
    confidence: "pruefen"
  },
  AT_VBG: {
    country: "AT",
    code: "AT_VBG",
    name: "Vorarlberg",
    citationBase: "Vorarlberger Baugesetz (BauG)",
    regimeSummary: "BauG Vorarlberg (Abstandsflächenberechnung nach Wandhöhe)",
    authorityTitle: "Vorarlberger Baubehörde",
    setbackFactor: 0.5,
    minSetbackM: 3.0,
    confidence: "pruefen"
  },
  AT_BGLD: {
    country: "AT",
    code: "AT_BGLD",
    name: "Burgenland",
    citationBase: "Bgld. BauG",
    regimeSummary: "Bgld. BauG (Mindestabstand 3,0 m)",
    authorityTitle: "Burgenländische Baubehörde",
    setbackFactor: 0.5,
    minSetbackM: 3.0,
    confidence: "pruefen"
  },

  // --- Schweiz (CH) Leitkantone & IVHB ---
  CH_IVHB: {
    country: "CH",
    code: "CH_IVHB",
    name: "Schweiz (IVHB-Konkordat)",
    citationBase: "IVHB / SIA 416",
    regimeSummary: "Interkantonale Vereinbarung über die Harmonisierung der Baubegriffe",
    authorityTitle: "Schweizerische Baubehörde (IVHB)",
    setbackFactor: 0.5,
    minSetbackM: 3.5,
    confidence: "reference"
  },
  CH_ZH: {
    country: "CH",
    code: "CH_ZH",
    name: "Zürich",
    citationBase: "PBG ZH",
    regimeSummary: "PBG ZH (Kleiner Grenzabstand mind. 3.5 m bis 5.0 m, Vollgeschosse/Ausnützung)",
    authorityTitle: "Amt für Baubewilligungen Stadt Zürich",
    setbackFactor: 0.5,
    minSetbackM: 3.5,
    confidence: "hoch"
  },
  CH_BE: {
    country: "CH",
    code: "CH_BE",
    name: "Bern",
    citationBase: "BauG BE",
    regimeSummary: "BauG BE (Kantonales Baugesetz & BauV)",
    authorityTitle: "Regierungsstatthalteramt / Bauinspektorat Bern",
    setbackFactor: 0.5,
    minSetbackM: 4.0,
    confidence: "pruefen"
  },
  CH_BS: {
    country: "CH",
    code: "CH_BS",
    name: "Basel-Stadt",
    citationBase: "BPG BS",
    regimeSummary: "BPG BS (Städtische Verdichtung, Grenzbaurechte)",
    authorityTitle: "Bau- und Gastgewerbeinspektorat Basel-Stadt",
    setbackFactor: 0.5,
    minSetbackM: 3.0,
    confidence: "pruefen"
  },
  CH_LU: {
    country: "CH",
    code: "CH_LU",
    name: "Luzern",
    citationBase: "PBG LU",
    regimeSummary: "Planungs- und Baugesetz Kanton Luzern",
    authorityTitle: "Bauamt Luzern",
    setbackFactor: 0.5,
    minSetbackM: 4.0,
    confidence: "pruefen"
  },
  CH_SG: {
    country: "CH",
    code: "CH_SG",
    name: "St. Gallen",
    citationBase: "PBG SG",
    regimeSummary: "Planungs- und Baugesetz Kanton St. Gallen",
    authorityTitle: "Baubewilligungskommission St. Gallen",
    setbackFactor: 0.5,
    minSetbackM: 4.0,
    confidence: "pruefen"
  },
  CH_GE: {
    country: "CH",
    code: "CH_GE",
    name: "Genf (Genève)",
    citationBase: "LCI GE",
    regimeSummary: "Loi sur les constructions et les installations diverses (LCI)",
    authorityTitle: "Office des autorisations de construire Genève",
    setbackFactor: 0.5,
    minSetbackM: 4.0,
    confidence: "pruefen"
  }
};

export function resolveCountry(spec = {}, profileCode = "") {
  if (typeof spec?.land === "string" || typeof spec?.country === "string" || typeof spec?.staat === "string") {
    const raw = (spec.land || spec.country || spec.staat).trim().toUpperCase();
    if (raw === "AT" || raw === "AUT" || raw === "OESTERREICH" || raw === "ÖSTERREICH") return "AT";
    if (raw === "CH" || raw === "CHE" || raw === "SCHWEIZ" || raw === "SUISSE") return "CH";
    if (raw === "DE" || raw === "DEU" || raw === "DEUTSCHLAND") return "DE";
  }

  const pCode = (profileCode || spec?.bundesland || spec?.kanton || "").trim().toUpperCase();
  if (pCode.startsWith("AT_") || pCode === "AT" || pCode === "OIB") return "AT";
  if (pCode.startsWith("CH_") || pCode === "CH" || pCode === "IVHB" || DACH_REGIONS[`CH_${pCode}`]) return "CH";
  if (DACH_REGIONS[pCode]) return DACH_REGIONS[pCode].country;

  const addr = (spec?.adresse || spec?.projekt || "").toLowerCase();
  if (addr.includes("österreich") || addr.includes("oesterreich") || addr.includes("austria") || addr.includes("wien") || addr.includes("graz") || addr.includes("linz") || addr.includes("salzburg") || addr.includes("innsbruck")) {
    return "AT";
  }
  if (addr.includes("schweiz") || addr.includes("switzerland") || addr.includes("suisse") || addr.includes("zürich") || addr.includes("zuerich") || addr.includes("zurich") || addr.includes("bern") || addr.includes("basel") || addr.includes("genf") || addr.includes("geneve") || addr.includes("luzern")) {
    return "CH";
  }

  return "DE";
}

export function getRegionsForCountry(countryCode = "DE") {
  const c = countryCode.trim().toUpperCase();
  if (c === "AT") {
    return Object.values(DACH_REGIONS).filter((r) => r.country === "AT");
  }
  if (c === "CH") {
    return Object.values(DACH_REGIONS).filter((r) => r.country === "CH");
  }
  return [];
}


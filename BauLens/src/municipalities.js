/**
 * BauLens Municipal Engine (Kommunale Satzungen)
 *
 * Implements local city statutes enacted under state empowerment clauses
 * (e.g. §89 BayBO, §74 LBO BW, §89 BauO NRW, §88 HBO).
 */

export const MUNICIPALITIES = {
  // --- Bayern ---
  MUENCHEN: {
    code: "MUENCHEN",
    name: "Landeshauptstadt München",
    bundesland: "BY",
    satzungen: {
      stellplatz: {
        title: "Münchner Stellplatz- und Fahrradabstellsatzung (StplS)",
        para: "§§ 2–5 StplS München i.V.m. Art. 81 BayBO",
        pkwFaktor: 1.0,
        radFaktor: 2.0,
        notes: "München fordert zwingend überdachte und barrierefrei erreichbare Fahrradstellplätze.",
      },
      gruendach: {
        title: "Münchner Gründachsatzung & Freiflächengestaltung",
        para: "§ 4 Freiflächengestaltungssatzung (FGS) München",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 10,
        mindestBegruenungProzent: 80,
        giltFuer: "Flachdächer und flach geneigte Dächer bis 15° bei Neubauten ab 100 m² Dachfläche",
      },
      milieuschutz: {
        aktiv: true,
        bezirke: ["Schwabing", "Maxvorstadt", "Isarvorstadt", "Haidhausen", "Sendling"],
      },
    },
  },
  NUERNBERG: {
    code: "NUERNBERG",
    name: "Stadt Nürnberg",
    bundesland: "BY",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Stadt Nürnberg",
        para: "§ 3 StplS Nürnberg",
        pkwFaktor: 1.0,
        radFaktor: 1.5,
      },
      gruendach: {
        title: "Nürnberger Gründachförderung & B-Plan-Vorschriften",
        para: "Örtliche Bauvorschrift Nürnberg",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },

  // --- Berlin ---
  BERLIN: {
    code: "BERLIN",
    name: "Berlin (Gesamtstadt)",
    bundesland: "BE",
    satzungen: {
      stellplatz: {
        title: "Bauordnung für Berlin (Abschaffung Kfz-Stellplatzpflicht)",
        para: "§ 49 Abs. 1 BauO Bln",
        pkwFaktor: 0.0,
        radFaktor: 2.0,
        notes: "In Berlin besteht seit der Novelle KEINE Kfz-Stellplatzpflicht mehr für Wohnungen. Fahrradabstellplätze sind jedoch nach §49 Abs. 2 BauO Bln Pflicht.",
      },
      gruendach: {
        title: "Berliner GründachPLUS & Biotopflächenfaktor (BFF)",
        para: "§ 8 BauO Bln i.V.m. Landschaftsprogramm Berlin",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 8,
        mindestBegruenungProzent: 60,
      },
      milieuschutz: {
        aktiv: true,
        bezirke: ["Kreuzberg", "Friedrichshain", "Neukölln", "Pankow", "Mitte"],
      },
    },
  },

  // --- Hamburg ---
  HAMBURG: {
    code: "HAMBURG",
    name: "Freie und Hansestadt Hamburg",
    bundesland: "HH",
    satzungen: {
      stellplatz: {
        title: "Hamburgische Stellplatzverordnung (HmbStplVO)",
        para: "§ 48 HBauO i.V.m. HmbStplVO",
        pkwFaktor: 0.6,
        radFaktor: 2.0,
        notes: "Hamburg wendet je nach ÖPNV-Güteklasse reduzierte Richtwerte an (0,4 bis 0,8 Pkw/WE).",
      },
      gruendach: {
        title: "Hamburger Gründachstrategie",
        para: "§ 9 Abs. 2 HBauO (Begrünung von Flachdächern)",
        pflichtAbGrad: 0,
        maxGrad: 12,
        minSubstratStaerkeCm: 10,
        mindestBegruenungProzent: 70,
        giltFuer: "Neubauten mit Flachdach ab 100 m² Dachfläche",
      },
    },
  },

  // --- Nordrhein-Westfalen ---
  KOELN: {
    code: "KOELN",
    name: "Stadt Köln",
    bundesland: "NW",
    satzungen: {
      stellplatz: {
        title: "Kölner Stellplatzsatzung",
        para: "§ 89 Abs. 1 Nr. 5 BauO NRW i.V.m. Stellplatzsatzung Köln",
        pkwFaktor: 0.8,
        radFaktor: 2.0,
        notes: "In Köln gelten für gut angebundene City-Zonen Reduzierungsoptionen.",
      },
      gruendach: {
        title: "Kölner Gründachsatzung",
        para: "§ 89 BauO NRW i.V.m. Gründachsatzung Köln",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 10,
        mindestBegruenungProzent: 70,
      },
    },
  },
  DUESSELDORF: {
    code: "DUESSELDORF",
    name: "Landeshauptstadt Düsseldorf",
    bundesland: "NW",
    satzungen: {
      stellplatz: {
        title: "Düsseldorfer Stellplatzsatzung",
        para: "§ 89 BauO NRW i.V.m. Stellplatzsatzung Düsseldorf",
        pkwFaktor: 0.9,
        radFaktor: 2.0,
      },
      gruendach: {
        title: "Düsseldorfer Satzung über Begrünung und Freiflächengestaltung",
        para: "Begrünungssatzung Düsseldorf",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 10,
      },
    },
  },

  // --- Hessen ---
  FRANKFURT: {
    code: "FRANKFURT",
    name: "Stadt Frankfurt am Main",
    bundesland: "HE",
    satzungen: {
      stellplatz: {
        title: "Stellplatz- und Ablösesatzung Frankfurt am Main (StPlS)",
        para: "§ 51 HBO i.V.m. StPlS Frankfurt",
        pkwFaktor: 0.7,
        radFaktor: 2.0,
        notes: "Frankfurt beschränkt den Stellplatzbau im Innenstadtbereich (Zone 1) zur Vermeidung von Mehrverkehr.",
      },
      gruendach: {
        title: "Frankfurter Gründachsatzung & Begrünungsvorgaben",
        para: "§ 91 Abs. 1 HBO i.V.m. Freiflächensicherungssatzung Ffm",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 12,
        mindestBegruenungProzent: 70,
        giltFuer: "Neubauten mit Flachdächern über 50 m²",
      },
    },
  },

  // --- Baden-Württemberg ---
  STUTTGART: {
    code: "STUTTGART",
    name: "Landeshauptstadt Stuttgart",
    bundesland: "BW",
    satzungen: {
      stellplatz: {
        title: "Stuttgarter Kraftfahrzeug- und Fahrradstellplatzsatzung",
        para: "§ 74 LBO BW i.V.m. Satzung Stuttgart",
        pkwFaktor: 1.0,
        radFaktor: 2.0,
        notes: "Besondere Vorgaben zur Ladeinfrastruktur bei Elektromobilität gem. Satzung.",
      },
      gruendach: {
        title: "Stuttgarter Klimaanpassungs- & Gründachprogramm",
        para: "Örtliche Bauvorschrift Stuttgart (Kessellage / Luftreinhaltung)",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 12,
        mindestBegruenungProzent: 80,
        giltFuer: "Alle ungenutzten Dachflächen bis 15° im Stadtgebiet zur Kaltluftförderung",
      },
    },
  },

  // --- Sachsen ---
  LEIPZIG: {
    code: "LEIPZIG",
    name: "Stadt Leipzig",
    bundesland: "SN",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Stadt Leipzig",
        para: "§ 89 SächsBO i.V.m. Stellplatzsatzung Leipzig",
        pkwFaktor: 0.8,
        radFaktor: 2.0,
      },
      gruendach: {
        title: "Leipziger Gründach-Richtlinie & Freiflächengestaltung",
        para: "Gründachleitfaden Leipzig",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },
  DRESDEN: {
    code: "DRESDEN",
    name: "Landeshauptstadt Dresden",
    bundesland: "SN",
    satzungen: {
      stellplatz: {
        title: "Dresdner Stellplatzablöse- und Herstellsatzung",
        para: "§ 89 SächsBO i.V.m. Satzung Dresden",
        pkwFaktor: 1.0,
        radFaktor: 1.5,
      },
      gruendach: {
        title: "Dresdner Begrünungssatzung",
        para: "Örtliche Bauvorschriften Dresden",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },

  // --- Niedersachsen ---
  HANNOVER: {
    code: "HANNOVER",
    name: "Landeshauptstadt Hannover",
    bundesland: "NI",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Landeshauptstadt Hannover",
        para: "§ 47 NBauO i.V.m. Stellplatzsatzung Hannover",
        pkwFaktor: 0.8,
        radFaktor: 2.0,
      },
      gruendach: {
        title: "Gründachsatzung Hannover",
        para: "Örtliche Bauvorschrift über die Begrünung baulicher Anlagen",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 10,
        mindestBegruenungProzent: 70,
      },
    },
  },

  // --- Bayern (Erweiterung) ---
  AUGSBURG: {
    code: "AUGSBURG",
    name: "Stadt Augsburg",
    bundesland: "BY",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Stadt Augsburg",
        para: "Art. 81 BayBO i.V.m. Stellplatzsatzung Augsburg",
        pkwFaktor: 1.0,
        radFaktor: 2.0,
      },
      gruendach: {
        title: "Augsburger Begrünungssatzung",
        para: "Örtliche Bauvorschrift Augsburg",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 8,
      },
    },
  },

  // --- Nordrhein-Westfalen (Erweiterung) ---
  ESSEN: {
    code: "ESSEN",
    name: "Stadt Essen",
    bundesland: "NW",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Stadt Essen",
        para: "§ 48 BauO NRW i.V.m. Stellplatzsatzung Essen",
        pkwFaktor: 0.8,
        radFaktor: 1.5,
      },
      gruendach: {
        title: "Essener Klimaanpassungs- und Gründachprogramm",
        para: "Grüne Hauptstadt Europas Essen",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },
  DORTMUND: {
    code: "DORTMUND",
    name: "Stadt Dortmund",
    bundesland: "NW",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Stadt Dortmund",
        para: "§ 48 BauO NRW i.V.m. Satzung Dortmund",
        pkwFaktor: 0.8,
        radFaktor: 2.0,
      },
      gruendach: {
        title: "Dortmunder Gründachförderung",
        para: "Örtliche Bauvorschrift Dortmund",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },
  BONN: {
    code: "BONN",
    name: "Bundesstadt Bonn",
    bundesland: "NW",
    satzungen: {
      stellplatz: {
        title: "Bonner Stellplatz- und Fahrradsatzung",
        para: "§ 48 BauO NRW i.V.m. Stellplatzsatzung Bonn",
        pkwFaktor: 0.7,
        radFaktor: 2.5,
        notes: "Bonn gewährt Reduktionen bei nachgewiesenem Carsharing- und ÖPNV-Angebot.",
      },
      gruendach: {
        title: "Bonner Solargründach- und Klimasatzung",
        para: "Örtliche Bauvorschriften Bonn",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 10,
        mindestBegruenungProzent: 75,
      },
    },
  },
  MUENSTER: {
    code: "MUENSTER",
    name: "Stadt Münster",
    bundesland: "NW",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Stadt Münster (Fahrradhauptstadt)",
        para: "§ 48 BauO NRW i.V.m. Stellplatzsatzung Münster",
        pkwFaktor: 0.6,
        radFaktor: 3.0,
        notes: "Münster setzt mit 3,0 Fahrradstellplätzen je WE bundesweit einen Spitzenwert.",
      },
      gruendach: {
        title: "Gründachsatzung Münster",
        para: "Klimaschutzkonzept Münster",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },

  // --- Baden-Württemberg (Erweiterung) ---
  MANNHEIM: {
    code: "MANNHEIM",
    name: "Stadt Mannheim",
    bundesland: "BW",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Stadt Mannheim",
        para: "§ 74 LBO BW i.V.m. Stellplatzsatzung Mannheim",
        pkwFaktor: 0.8,
        radFaktor: 2.0,
      },
      gruendach: {
        title: "Gründachprogramm Mannheim",
        para: "Klimaschutzleitfaden Mannheim",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },
  KARLSRUHE: {
    code: "KARLSRUHE",
    name: "Stadt Karlsruhe",
    bundesland: "BW",
    satzungen: {
      stellplatz: {
        title: "Karlsruher Stellplatzsatzung",
        para: "§ 74 LBO BW i.V.m. Satzung Karlsruhe",
        pkwFaktor: 0.7,
        radFaktor: 2.5,
      },
      gruendach: {
        title: "Gründach- und Freiflächensatzung Karlsruhe",
        para: "Örtliche Bauvorschrift Karlsruhe",
        pflichtAbGrad: 0,
        maxGrad: 15,
        minSubstratStaerkeCm: 10,
      },
    },
  },

  // --- Hessen (Erweiterung) ---
  WIESBADEN: {
    code: "WIESBADEN",
    name: "Landeshauptstadt Wiesbaden",
    bundesland: "HE",
    satzungen: {
      stellplatz: {
        title: "Stellplatzsatzung der Landeshauptstadt Wiesbaden",
        para: "§ 52 HBO i.V.m. Stellplatzsatzung Wiesbaden",
        pkwFaktor: 1.0,
        radFaktor: 2.0,
      },
      gruendach: {
        title: "Wiesbadener Begrünungssatzung",
        para: "Örtliche Bauvorschrift Wiesbaden",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },

  // --- Bremen ---
  BREMEN: {
    code: "BREMEN",
    name: "Freie Hansestadt Bremen",
    bundesland: "HB",
    satzungen: {
      stellplatz: {
        title: "Bremische Stellplatz- und Fahrradabstellplatzverordnung",
        para: "BremLBO i.V.m. BremStplVO",
        pkwFaktor: 0.8,
        radFaktor: 2.0,
      },
      gruendach: {
        title: "Bremer Gründachprogramm",
        para: "Begrünungsvorschrift Bremen",
        pflichtAbGrad: 0,
        maxGrad: 15,
      },
    },
  },
};

export const MUNICIPALITY_CODES = Object.keys(MUNICIPALITIES);

const MUNICIPALITY_ALIASES = {
  "MÜNCHEN": "MUENCHEN",
  "MUNCHEN": "MUENCHEN",
  "DE-MUC": "MUENCHEN",
  "MUC": "MUENCHEN",
  "NÜRNBERG": "NUERNBERG",
  "DE-BER": "BERLIN",
  "DE-HAM": "HAMBURG",
  "KÖLN": "KOELN",
  "DE-CGN": "KOELN",
  "DÜSSELDORF": "DUESSELDORF",
  "DE-DUS": "DUESSELDORF",
  "DE-FRA": "FRANKFURT",
  "FRANKFURT AM MAIN": "FRANKFURT",
  "DE-STR": "STUTTGART",
  "DE-LEJ": "LEIPZIG",
  "MÜNSTER": "MUENSTER",
};

/**
 * Returns municipality by code or city name (case-insensitive)
 */
export function getMunicipality(code) {
  if (!code || typeof code !== "string") return null;
  const upper = code.trim().toUpperCase();
  if (MUNICIPALITIES[upper]) return MUNICIPALITIES[upper];
  if (MUNICIPALITY_ALIASES[upper]) return MUNICIPALITIES[MUNICIPALITY_ALIASES[upper]] ?? null;
  return null;
}

/**
 * Resolves municipality code based on spec or explicit option
 */
export function resolveMunicipalityCode(spec, optionCode) {
  if (optionCode && typeof optionCode === "string") {
    const found = getMunicipality(optionCode);
    if (found) return found.code;
  }
  if (spec?.kommune && typeof spec.kommune === "string") {
    const found = getMunicipality(spec.kommune);
    if (found) return found.code;
  }
  if (spec?.stadt && typeof spec.stadt === "string") {
    const found = getMunicipality(spec.stadt);
    if (found) return found.code;
  }
  // Heuristic scan in project title or address
  const text = `${spec?.projekt ?? ""} ${spec?.adresse ?? ""}`.toLowerCase();
  if (text.includes("münchen") || text.includes("muenchen")) return "MUENCHEN";
  if (text.includes("berlin")) return "BERLIN";
  if (text.includes("hamburg")) return "HAMBURG";
  if (text.includes("köln") || text.includes("koeln")) return "KOELN";
  if (text.includes("frankfurt")) return "FRANKFURT";
  if (text.includes("stuttgart")) return "STUTTGART";
  if (text.includes("düsseldorf") || text.includes("duesseldorf")) return "DUESSELDORF";
  if (text.includes("leipzig")) return "LEIPZIG";
  if (text.includes("dresden")) return "DRESDEN";
  if (text.includes("nürnberg") || text.includes("nuernberg")) return "NUERNBERG";
  if (text.includes("hannover")) return "HANNOVER";
  if (text.includes("augsburg")) return "AUGSBURG";
  if (text.includes("essen")) return "ESSEN";
  if (text.includes("dortmund")) return "DORTMUND";
  if (text.includes("bonn")) return "BONN";
  if (text.includes("münster") || text.includes("muenster")) return "MUENSTER";
  if (text.includes("mannheim")) return "MANNHEIM";
  if (text.includes("karlsruhe")) return "KARLSRUHE";
  if (text.includes("wiesbaden")) return "WIESBADEN";
  if (text.includes("bremen")) return "BREMEN";

  return null;
}

/**
 * Get all cities for a given Bundesland code
 */
export function getMunicipalitiesForLand(landCode) {
  if (!landCode) return [];
  const upperLand = landCode.toUpperCase();
  return Object.values(MUNICIPALITIES).filter((m) => m.bundesland === upperLand);
}

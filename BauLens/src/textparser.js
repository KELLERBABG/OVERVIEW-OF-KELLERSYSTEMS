/**
 * BauLens - Text & Baubeschreibung Ingestion Parser
 * Extracts building and parcel parameters from unstructured German project text,
 * DIN 277 calculation sheets, or Bauantrags-Vorlagen.
 */

const STATE_KEYWORDS = {
  by: ["bayern", "bayerische", "baybo", "münchen", "nürnberg", "augsburg"],
  bw: ["baden-württemberg", "stuttgart", "karlsruhe", "freiburg", "lbo bw"],
  be: ["berlin", "bauo bln", "bauordnung berlin"],
  nw: ["nordrhein-westfalen", "nrw", "köln", "düsseldorf", "bauo nrw"],
  he: ["hessen", "frankfurt", "wiesbaden", "hbo"],
  hh: ["hamburg", "hbauo"],
  sn: ["sachsen", "dresden", "leipzig", "sächsbo"],
  ni: ["niedersachsen", "hannover", "nbo"],
  rp: ["rheinland-pfalz", "mainz", "lbauo rlp"],
  sh: ["schleswig-holstein", "kiel", "lbo sh"],
  st: ["sachsen-anhalt", "magdeburg"],
  th: ["thüringen", "erfurt", "thürbo"],
  bb: ["brandenburg", "potsdam", "bbgbo"],
  hb: ["bremen", "bremische"],
  mv: ["mecklenburg-vorpommern", "rostock", "lbauo m-v"],
  sl: ["saarland", "saarbrücken", "lbo saarland"],
};

function parseGermanFloat(str) {
  if (!str) return null;
  let clean = str.trim().replace(/[.,;:]+$/, "");
  if (clean.includes(",")) {
    clean = clean.replace(/\./g, "").replace(",", ".");
  }
  const num = parseFloat(clean);
  return isNaN(num) ? null : num;
}

export function parseBaubeschreibung(text) {
  if (typeof text !== "string" || !text.trim()) {
    return { spec: null, confidence: 0, extractedKeys: [] };
  }

  const spec = {
    grundstueck: {
      grenzabstaende: {},
    },
    gebaeude: {
      aufenthaltsraeume: [],
    },
    einstellungen: {},
  };

  const extractedKeys = [];
  const lower = text.toLowerCase();

  // 1. Bundesland Detection
  for (const [code, keywords] of Object.entries(STATE_KEYWORDS)) {
    if (keywords.some((kw) => lower.includes(kw))) {
      spec.bundesland = code.toUpperCase();
      extractedKeys.push("bundesland");
      break;
    }
  }

  // 2. Grundstücksfläche (m² / qm)
  const plotMatch = lower.match(/(?:grundst(?:[üu]|ue)cks?fl(?:[äa]|ae)che|grundst(?:[üu]|ue)ck|flurst(?:[üu]|ue)ck[^\d\n]*fl(?:[äa]|ae)che)[^\d\n]*(\d+(?:[.,]\d+)?)\s*(?:m[²2]|qm)/i);
  if (plotMatch) {
    const val = parseGermanFloat(plotMatch[1]);
    if (val && val > 0) {
      spec.grundstueck.flaeche_m2 = val;
      extractedKeys.push("grundstueck.flaeche_m2");
    }
  }

  // 3. Gebäudehöhe / Wandhöhe
  const heightMatch = lower.match(/(?:geb(?:[äa]|ae)udeh(?:[öo]|oe)he|wandh(?:[öo]|oe)he|h(?:[öo]|oe)he\s*(?:h|geb(?:[äa]|ae)ude))[^\d\n]*(\d+(?:[.,]\d+)?)\s*m/i);
  if (heightMatch) {
    const val = parseGermanFloat(heightMatch[1]);
    if (val && val > 0) {
      spec.gebaeude.hoehe_m = val;
      extractedKeys.push("gebaeude.hoehe_m");
    }
  }

  // 4. Vollgeschosse
  const storeysMatch = lower.match(/(\d+)\s*(?:vollgeschosse?|geschosse?\s*(?:[üu]|ue)ber\s*grund|og\s*\+\s*eg|stockwerke?)/i) ||
                       lower.match(/(?:vollgeschosse?|geschosse)[^\d\n]*(\d+)/i);
  if (storeysMatch) {
    const val = parseInt(storeysMatch[1], 10);
    if (!isNaN(val) && val > 0) {
      spec.gebaeude.geschosse_ueber_grund = val;
      extractedKeys.push("gebaeude.geschosse_ueber_grund");
    }
  }

  // 5. Wohnungsanzahl
  const unitsMatch = lower.match(/(\d+)\s*(?:wohneinheit(?:en)?|wohnung(?:en)?|we\b)/i) ||
                     lower.match(/(?:wohneinheit(?:en)?|wohnung(?:en)?)[^\d\n]*(\d+)/i);
  if (unitsMatch) {
    const val = parseInt(unitsMatch[1], 10);
    if (!isNaN(val) && val > 0) {
      spec.gebaeude.wohnungen_anzahl = val;
      extractedKeys.push("gebaeude.wohnungen_anzahl");
    }
  } else if (/\b(?:efh|einfamilienhaus|einfamilien-wohnhaus)\b/i.test(lower)) {
    spec.gebaeude.wohnungen_anzahl = 1;
    extractedKeys.push("gebaeude.wohnungen_anzahl");
  } else if (/\b(?:zfh|zweifamilienhaus)\b/i.test(lower)) {
    spec.gebaeude.wohnungen_anzahl = 2;
    extractedKeys.push("gebaeude.wohnungen_anzahl");
  }

  // 6. Nutzfläche / Wohnfläche
  const areaMatch = lower.match(/(?:nutzfl(?:[äa]|ae)che|wohnfl(?:[äa]|ae)che|bruttogrundfl(?:[äa]|ae)che|bgf)[^\d\n]*(\d+(?:[.,]\d+)?)\s*(?:m[²2]|qm)/i);
  if (areaMatch) {
    const val = parseGermanFloat(areaMatch[1]);
    if (val && val > 0) {
      spec.gebaeude.nutzflaeche_m2 = val;
      extractedKeys.push("gebaeude.nutzflaeche_m2");
    }
  }

  // 7. GRZ ist
  const grzMatch = lower.match(/grz[^\d\n]*(\d+(?:[.,]\d+)?)/i);
  if (grzMatch) {
    const val = parseGermanFloat(grzMatch[1]);
    if (val !== null && val >= 0 && val <= 1) {
      spec.gebaeude.grz_ist = val;
      extractedKeys.push("gebaeude.grz_ist");
    }
  }

  // 8. GFZ ist
  const gfzMatch = lower.match(/gfz[^\d\n]*(\d+(?:[.,]\d+)?)/i);
  if (gfzMatch) {
    const val = parseGermanFloat(gfzMatch[1]);
    if (val !== null && val >= 0) {
      spec.gebaeude.gfz_ist = val;
      extractedKeys.push("gebaeude.gfz_ist");
    }
  }

  // 9. Stellplätze (PKW & Fahrrad)
  const pkwMatch = lower.match(/(\d+)\s*(?:pkw[ -]?stellpl[äa]tze?|stellpl[äa]tze?\s*\(?pkw\)?|garagenpl[äa]tze?)/i) ||
                   lower.match(/stellpl[äa]tze?[^\d\n]*(\d+)/i);
  if (pkwMatch) {
    const val = parseInt(pkwMatch[1], 10);
    if (!isNaN(val) && val >= 0) {
      spec.gebaeude.stellplaetze_ist = val;
      extractedKeys.push("gebaeude.stellplaetze_ist");
    }
  }

  const radMatch = lower.match(/(\d+)\s*(?:fahrrad[ -]?stellpl[äa]tze?|fahrradabstellpl[äa]tze?|radabstellpl[äa]tze?)/i);
  if (radMatch) {
    const val = parseInt(radMatch[1], 10);
    if (!isNaN(val) && val >= 0) {
      spec.gebaeude.fahrradstellplaetze_ist = val;
      extractedKeys.push("gebaeude.fahrradstellplaetze_ist");
    }
  }

  // 10. Grenzabstände (Nord, Süd, Ost, West)
  const sides = [
    { key: "norden", regex: /(?:nord(?:en)?|abstand\s*nord)[^\d\n]*(\d+(?:[.,]\d+)?)\s*m/i },
    { key: "sueden", regex: /(?:s[üu]d(?:en)?|abstand\s*s[üu]d)[^\d\n]*(\d+(?:[.,]\d+)?)\s*m/i },
    { key: "osten", regex: /(?:ost(?:en)?|abstand\s*ost)[^\d\n]*(\d+(?:[.,]\d+)?)\s*m/i },
    { key: "westen", regex: /(?:west(?:en)?|abstand\s*west)[^\d\n]*(\d+(?:[.,]\d+)?)\s*m/i },
  ];

  for (const { key, regex } of sides) {
    const m = lower.match(regex);
    if (m) {
      const val = parseGermanFloat(m[1]);
      if (val !== null && val >= 0) {
        spec.grundstueck.grenzabstaende[key] = val;
        extractedKeys.push(`grundstueck.grenzabstaende.${key}`);
      }
    }
  }

  // Fallbacks if nothing in grenzabstaende
  if (Object.keys(spec.grundstueck.grenzabstaende).length === 0) {
    delete spec.grundstueck.grenzabstaende;
  }

  const confidence = Math.min(100, Math.round((extractedKeys.length / 8) * 100));

  return { spec, confidence, extractedKeys };
}

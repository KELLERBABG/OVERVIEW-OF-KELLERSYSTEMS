import { isNum } from "./util.js";

const KNOWN_SIDES = [
  "norden", "nordosten", "osten", "suedosten",
  "sueden", "suedwesten", "westen", "nordwesten",
];

const ROOT_KEYS = [
  "projekt", "adresse", "bundesland", "kommune", "stadt",
  "land", "country", "staat", "kanton", "region",
  "grundstueck", "gebaeude", "bplan", "einstellungen",
];

const GEBAEUDE_KEYS = [
  "hoehe_m", "firsthoehe_m", "traufhoehe_m", "geschosse_ueber_grund", "geschosse_unter_ground",
  "wohnungen_anzahl", "nutzflaeche_m2", "grz_ist", "gfz_ist",
  "stellplaetze_ist", "fahrradstellplaetze_ist",
  "aufenthaltsraeume", "zweiter_rettungsweg_vorhanden", "treppenraum_vorhanden",
  "dachform", "dachneigung_grad", "gruendach_vorhanden", "gruendach_substrat_cm", "dach",
  "aufzug_vorhanden", "eingang_stufenlos", "barrierefreie_wohnungen_anzahl", "rampen_neigung_prozent",
  "rettungsweg_lauflaenge_m", "waermepumpe_aussen_vorhanden", "abstand_waermepumpe_grenze_m", "schallschutz_laermpegelbereich",
  "umriss_polygon", "grenzgarage", "dachgeschoss",
];

const GRENZGARAGE_KEYS = [
  "vorhanden", "wandhoehe_m", "laenge_einzelgrenze_m", "laenge_gesamt_m", "grenzseite"
];

const DACHGESCHOSS_KEYS = [
  "vorhanden", "flaeche_ueber_2_30m_m2", "flaeche_unterliegend_m2", "ist_vollgeschoss"
];

const DACH_KEYS = [
  "form", "dachneigung_grad", "gruendach_vorhanden", "substrat_cm",
];

const BPLAN_KEYS = [
  "baugebiet", "grz_festsetzung", "gfz_festsetzung", "grz_max", "gfz_max", "bauweise",
  "baugrenze_eingehalten", "max_traufhoehe_m", "max_firsthoehe_m", "traufhoehe_max_m", "firsthoehe_max_m",
  "vollgeschosse_max", "zulaessige_dachformen", "dachneigung_min_grad", "dachneigung_max_grad",
  "baugrenze_polygon", "baulinie_segmente", "xplan_version", "plan_name", "rechtsstand",
];

function checkNumber(value, path, opts, errors, warnings) {
  const { min = -Infinity, max = Infinity, integer = false } = opts;
  if (value === undefined) {
    warnings.push(`${path} fehlt (optional) – zugehörige Prüfungen eingeschränkt.`);
    return;
  }
  if (!isNum(value)) {
    errors.push(`${path} muss eine Zahl sein (gefunden: ${typeof value}).`);
    return;
  }
  if (integer && !Number.isInteger(value)) {
    errors.push(`${path} muss eine ganze Zahl sein.`);
    return;
  }
  if (value < min) {
    errors.push(`${path} muss >= ${min} sein.`);
    return;
  }
  if (value > max) {
    errors.push(`${path} muss <= ${max} sein.`);
  }
}

function checkBool(value, path, errors, warnings) {
  if (value === undefined) {
    warnings.push(`${path} fehlt (optional) – Annahme: false. Bitte explizit angeben.`);
    return;
  }
  if (typeof value !== "boolean") {
    errors.push(`${path} muss true oder false sein (gefunden: ${typeof value}).`);
  }
}

function warnUnknownKeys(obj, allowed, label, warnings) {
  for (const key of Object.keys(obj)) {
    if (!allowed.includes(key)) {
      warnings.push(`Unbekanntes Feld "${label}.${key}" (Tippfehler?) – wird ignoriert.`);
    }
  }
}

export function validateSpec(spec) {
  const errors = [];
  const warnings = [];

  if (typeof spec !== "object" || spec === null || Array.isArray(spec)) {
    errors.push("Die Eingabe muss ein JSON-Objekt sein.");
    return { ok: false, errors, warnings };
  }

  warnUnknownKeys(spec, ROOT_KEYS, "spec", warnings);

  if (spec.projekt !== undefined && typeof spec.projekt !== "string") {
    warnings.push('"projekt" sollte eine Zeichenkette sein – wird für den Bericht ignoriert.');
  }
  if (spec.adresse !== undefined && typeof spec.adresse !== "string") {
    warnings.push('"adresse" sollte eine Zeichenkette sein.');
  }
  if (spec.kommune !== undefined && typeof spec.kommune !== "string") {
    errors.push('"kommune" muss eine Zeichenkette sein (z.B. "MUENCHEN").');
  }
  if (spec.stadt !== undefined && typeof spec.stadt !== "string") {
    errors.push('"stadt" muss eine Zeichenkette sein.');
  }

  // --- grundstueck ---
  if (spec.grundstueck === undefined) {
    warnings.push('"grundstueck" fehlt – Grenzabstands- und GRZ-Prüfung ohne Daten.');
  } else if (typeof spec.grundstueck !== "object" || spec.grundstueck === null || Array.isArray(spec.grundstueck)) {
    errors.push('"grundstueck" muss ein Objekt sein.');
  } else {
    const g = spec.grundstueck;
    checkNumber(g.flaeche_m2, "grundstueck.flaeche_m2", { min: 0 }, errors, warnings);
    if (g.grenzabstaende === undefined) {
      warnings.push("grundstueck.grenzabstaende fehlt – §6-MBO-Prüfung ohne Eingangsdaten.");
    } else if (typeof g.grenzabstaende !== "object" || g.grenzabstaende === null || Array.isArray(g.grenzabstaende)) {
      errors.push("grundstueck.grenzabstaende muss ein Objekt (Seitenname -> Meter) sein.");
    } else {
      for (const [side, val] of Object.entries(g.grenzabstaende)) {
        if (!KNOWN_SIDES.includes(side)) {
          warnings.push(`Unbekannte Himmelsrichtung "${side}" in grenzabstaende (erwartet: ${KNOWN_SIDES.join(", ")}).`);
        } else if (!isNum(val) || val < 0) {
          errors.push(`grundstueck.grenzabstaende.${side} muss eine Zahl >= 0 sein.`);
        }
      }
    }
    warnUnknownKeys(g, [
      "flaeche_m2", "grenzabstaende", "flurstueck", "gemarkung", "flur",
      "flurstuecksnummer", "grenzpunkte", "gebaeude_polygon"
    ], "grundstueck", warnings);
  }

  // --- gebaeude ---
  if (spec.gebaeude === undefined) {
    warnings.push('"gebaeude" fehlt – keine Regel kann ausgewertet werden.');
  } else if (typeof spec.gebaeude !== "object" || spec.gebaeude === null || Array.isArray(spec.gebaeude)) {
    errors.push('"gebaeude" muss ein Objekt sein.');
  } else {
    const b = spec.gebaeude;
    checkNumber(b.hoehe_m, "gebaeude.hoehe_m", { min: 0 }, errors, warnings);
    checkNumber(b.geschosse_ueber_grund, "gebaeude.geschosse_ueber_grund", { min: 0, integer: true }, errors, warnings);
    checkNumber(b.geschosse_unter_ground, "gebaeude.geschosse_unter_ground", { min: 0, integer: true }, errors, warnings);
    checkNumber(b.wohnungen_anzahl, "gebaeude.wohnungen_anzahl", { min: 1, integer: true }, errors, warnings);
    checkNumber(b.nutzflaeche_m2, "gebaeude.nutzflaeche_m2", { min: 0 }, errors, warnings);
    checkNumber(b.grz_ist, "gebaeude.grz_ist", { min: 0, max: 1 }, errors, warnings);
    checkNumber(b.gfz_ist, "gebaeude.gfz_ist", { min: 0 }, errors, warnings);
    checkNumber(b.stellplaetze_ist, "gebaeude.stellplaetze_ist", { min: 0, integer: true }, errors, warnings);
    checkNumber(b.fahrradstellplaetze_ist, "gebaeude.fahrradstellplaetze_ist", { min: 0, integer: true }, errors, warnings);

    // Dach & Begrünung
    if (b.dachneigung_grad !== undefined) {
      checkNumber(b.dachneigung_grad, "gebaeude.dachneigung_grad", { min: 0, max: 90 }, errors, warnings);
    }
    if (b.gruendach_vorhanden !== undefined && typeof b.gruendach_vorhanden !== "boolean") {
      errors.push("gebaeude.gruendach_vorhanden muss ein Boolean sein.");
    }
    if (b.gruendach_substrat_cm !== undefined) {
      checkNumber(b.gruendach_substrat_cm, "gebaeude.gruendach_substrat_cm", { min: 0 }, errors, warnings);
    }
    if (b.dach !== undefined) {
      if (typeof b.dach !== "object" || b.dach === null || Array.isArray(b.dach)) {
        errors.push("gebaeude.dach muss ein Objekt sein.");
      } else {
        warnUnknownKeys(b.dach, DACH_KEYS, "gebaeude.dach", warnings);
      }
    }

    if (b.aufenthaltsraeume === undefined) {
      warnings.push("gebaeude.aufenthaltsraeume fehlt – Tageslicht- und Raumhöhenprüfung ohne Daten.");
    } else if (!Array.isArray(b.aufenthaltsraeume)) {
      errors.push("gebaeude.aufenthaltsraeume muss ein Array sein.");
    } else {
      const ROOM_KEYS = ["name", "flaeche_m2", "fensterflaeche_m2", "raumhoehe_m", "etage"];
      b.aufenthaltsraeume.forEach((raum, i) => {
        const p = `gebaeude.aufenthaltsraeume[${i}]`;
        if (typeof raum !== "object" || raum === null || Array.isArray(raum)) {
          errors.push(`${p} muss ein Objekt sein.`);
          return;
        }
        if (raum.name !== undefined && typeof raum.name !== "string") {
          errors.push(`${p}.name muss eine Zeichenkette sein.`);
        }
        checkNumber(raum.flaeche_m2, `${p}.flaeche_m2`, { min: 0 }, errors, warnings);
        checkNumber(raum.fensterflaeche_m2, `${p}.fensterflaeche_m2`, { min: 0 }, errors, warnings);
        checkNumber(raum.raumhoehe_m, `${p}.raumhoehe_m`, { min: 0 }, errors, warnings);
        checkNumber(raum.etage, `${p}.etage`, { integer: true, min: -5, max: 200 }, errors, warnings);
        warnUnknownKeys(raum, ROOM_KEYS, p, warnings);
      });
    }

    checkBool(b.zweiter_rettungsweg_vorhanden, "gebaeude.zweiter_rettungsweg_vorhanden", errors, warnings);
    checkBool(b.treppenraum_vorhanden, "gebaeude.treppenraum_vorhanden", errors, warnings);

    // Specialist Engineering fields
    if (b.aufzug_vorhanden !== undefined && typeof b.aufzug_vorhanden !== "boolean") {
      errors.push("gebaeude.aufzug_vorhanden muss ein Boolean sein.");
    }
    if (b.eingang_stufenlos !== undefined && typeof b.eingang_stufenlos !== "boolean") {
      errors.push("gebaeude.eingang_stufenlos muss ein Boolean sein.");
    }
    if (b.barrierefreie_wohnungen_anzahl !== undefined) {
      checkNumber(b.barrierefreie_wohnungen_anzahl, "gebaeude.barrierefreie_wohnungen_anzahl", { min: 0, integer: true }, errors, warnings);
    }
    if (b.rampen_neigung_prozent !== undefined) {
      checkNumber(b.rampen_neigung_prozent, "gebaeude.rampen_neigung_prozent", { min: 0 }, errors, warnings);
    }
    if (b.rettungsweg_lauflaenge_m !== undefined) {
      checkNumber(b.rettungsweg_lauflaenge_m, "gebaeude.rettungsweg_lauflaenge_m", { min: 0 }, errors, warnings);
    }
    if (b.waermepumpe_aussen_vorhanden !== undefined && typeof b.waermepumpe_aussen_vorhanden !== "boolean") {
      errors.push("gebaeude.waermepumpe_aussen_vorhanden muss ein Boolean sein.");
    }
    if (b.abstand_waermepumpe_grenze_m !== undefined) {
      checkNumber(b.abstand_waermepumpe_grenze_m, "gebaeude.abstand_waermepumpe_grenze_m", { min: 0 }, errors, warnings);
    }

    // Grenzgarage / Nebengebäude-Privileg
    if (b.grenzgarage !== undefined) {
      if (typeof b.grenzgarage !== "object" || b.grenzgarage === null || Array.isArray(b.grenzgarage)) {
        errors.push("gebaeude.grenzgarage muss ein Objekt sein.");
      } else {
        if (b.grenzgarage.vorhanden !== undefined && typeof b.grenzgarage.vorhanden !== "boolean") {
          errors.push("gebaeude.grenzgarage.vorhanden muss ein Boolean sein.");
        }
        if (b.grenzgarage.wandhoehe_m !== undefined) {
          checkNumber(b.grenzgarage.wandhoehe_m, "gebaeude.grenzgarage.wandhoehe_m", { min: 0 }, errors, warnings);
        }
        if (b.grenzgarage.laenge_einzelgrenze_m !== undefined) {
          checkNumber(b.grenzgarage.laenge_einzelgrenze_m, "gebaeude.grenzgarage.laenge_einzelgrenze_m", { min: 0 }, errors, warnings);
        }
        if (b.grenzgarage.laenge_gesamt_m !== undefined) {
          checkNumber(b.grenzgarage.laenge_gesamt_m, "gebaeude.grenzgarage.laenge_gesamt_m", { min: 0 }, errors, warnings);
        }
        warnUnknownKeys(b.grenzgarage, GRENZGARAGE_KEYS, "gebaeude.grenzgarage", warnings);
      }
    }

    // Dachgeschoss-Vollgeschossprüfung
    if (b.dachgeschoss !== undefined) {
      if (typeof b.dachgeschoss !== "object" || b.dachgeschoss === null || Array.isArray(b.dachgeschoss)) {
        errors.push("gebaeude.dachgeschoss muss ein Objekt sein.");
      } else {
        if (b.dachgeschoss.vorhanden !== undefined && typeof b.dachgeschoss.vorhanden !== "boolean") {
          errors.push("gebaeude.dachgeschoss.vorhanden muss ein Boolean sein.");
        }
        if (b.dachgeschoss.flaeche_ueber_2_30m_m2 !== undefined) {
          checkNumber(b.dachgeschoss.flaeche_ueber_2_30m_m2, "gebaeude.dachgeschoss.flaeche_ueber_2_30m_m2", { min: 0 }, errors, warnings);
        }
        if (b.dachgeschoss.flaeche_unterliegend_m2 !== undefined) {
          checkNumber(b.dachgeschoss.flaeche_unterliegend_m2, "gebaeude.dachgeschoss.flaeche_unterliegend_m2", { min: 0 }, errors, warnings);
        }
        warnUnknownKeys(b.dachgeschoss, DACHGESCHOSS_KEYS, "gebaeude.dachgeschoss", warnings);
      }
    }

    warnUnknownKeys(b, GEBAEUDE_KEYS, "gebaeude", warnings);
  }

  // --- bplan (optional) ---
  if (spec.bplan !== undefined) {
    if (typeof spec.bplan !== "object" || spec.bplan === null || Array.isArray(spec.bplan)) {
      errors.push('"bplan" muss ein Objekt sein.');
    } else {
      warnUnknownKeys(spec.bplan, BPLAN_KEYS, "bplan", warnings);
    }
  }

  // --- landesprofil / kanton / country (optional) ---
  if (spec.bundesland !== undefined) {
    if (typeof spec.bundesland !== "string" || spec.bundesland.trim() === "") {
      errors.push('"bundesland" muss ein nicht-leerer Landescode sein (z. B. "BY", "BW", "AT_W").');
    }
  }
  if (spec.kanton !== undefined) {
    if (typeof spec.kanton !== "string" || spec.kanton.trim() === "") {
      errors.push('"kanton" muss ein nicht-leerer Kantons-Code sein (z. B. "ZH", "BE").');
    }
  }
  if (spec.land !== undefined && typeof spec.land !== "string") {
    errors.push('"land" muss ein Ländercode sein (z. B. "DE", "AT", "CH").');
  }

  // --- einstellungen (Konfiguration, z. B. GRZ-Limit) ---
  if (spec.einstellungen !== undefined) {
    if (typeof spec.einstellungen !== "object" || spec.einstellungen === null || Array.isArray(spec.einstellungen)) {
      errors.push('"einstellungen" muss ein Objekt sein.');
    } else {
      if (spec.einstellungen.grz_limit !== undefined &&
          (!isNum(spec.einstellungen.grz_limit) || spec.einstellungen.grz_limit <= 0 || spec.einstellungen.grz_limit > 1)) {
        errors.push("einstellungen.grz_limit muss eine Zahl > 0 und <= 1 sein.");
      }
      if (spec.einstellungen.gfz_limit !== undefined &&
          (!isNum(spec.einstellungen.gfz_limit) || spec.einstellungen.gfz_limit <= 0)) {
        errors.push("einstellungen.gfz_limit muss eine Zahl > 0 sein.");
      }
      if (spec.einstellungen.pkw_stellplatz_faktor !== undefined &&
          (!isNum(spec.einstellungen.pkw_stellplatz_faktor) || spec.einstellungen.pkw_stellplatz_faktor < 0)) {
        errors.push("einstellungen.pkw_stellplatz_faktor muss eine Zahl >= 0 sein.");
      }
      if (spec.einstellungen.rad_stellplatz_faktor !== undefined &&
          (!isNum(spec.einstellungen.rad_stellplatz_faktor) || spec.einstellungen.rad_stellplatz_faktor < 0)) {
        errors.push("einstellungen.rad_stellplatz_faktor muss eine Zahl >= 0 sein.");
      }
    }
  }

  return { ok: errors.length === 0, errors, warnings };
}

export function validateBuildingSpec(spec) {
  const res = validateSpec(spec);
  return { ...res, valid: res.ok };
}

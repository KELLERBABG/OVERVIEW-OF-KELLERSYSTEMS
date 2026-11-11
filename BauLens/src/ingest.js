/**
 * BauLens - plan-file ingestion for the engine.
 *
 * studio.html maps an uploaded drawing onto form fields inside the browser. The
 * desktop workhorse instead checks whole folders, so that mapping has to live in
 * the engine: one file shape is recognised once, tested once, and the CLI, the
 * RPC path and the UI all get the same answer for the same file.
 *
 * The mapping is deliberately conservative. Every value that a parser guessed
 * rather than measured is reported in `notes`, so a result built from a plot
 * boundary is never mistaken for one built from real room data.
 */
import { basename, extname } from "node:path";
import { parseDxf, parseIfc } from "./cadparser.js";
import { parseXPlanGml, parseBPlanGeoJson, parseBPlanText } from "./bplanparser.js";
import { parseBaubeschreibung } from "./textparser.js";

/** Kinds this module can turn into a spec. */
export const KINDS = [
  "spec",
  "ifc",
  "dxf",
  "xplan",
  "bplan_geojson",
  "bplan_text",
  "text",
  "unsupported",
];

const SPEC_MARKERS = ["grundstueck", "gebaeude", "bundesland", "einstellungen", "projekt", "aufenthaltsraeume"];
const BPLAN_MARKERS = /bebauungsplan|festsetzung|baugrenze|textteil|bauleitplan|b-plan/i;

export function ingestError(code, message) {
  const error = new Error(message);
  error.code = code;
  return error;
}

/**
 * Classifies a file by name and, where the extension is ambiguous, by content:
 * a `.json` is a spec when it carries spec keys and a B-Plan when it does not, a
 * `.txt` is a Textteil when it says so and a Baubeschreibung otherwise.
 */
export function detectKind(fileName, text = "") {
  const ext = extname(String(fileName ?? "")).toLowerCase();

  if (ext === ".ifc") return "ifc";
  if (ext === ".dxf") return "dxf";
  if (ext === ".gml" || ext === ".xml") return "xplan";
  if (ext === ".geojson") return "bplan_geojson";

  if (ext === ".json") {
    try {
      const parsed = JSON.parse(text);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        if (SPEC_MARKERS.some((key) => key in parsed)) return "spec";
      }
    } catch {
      /* Unreadable JSON is reported by the parser below, not here. */
    }
    return "bplan_geojson";
  }

  if (ext === ".txt" || ext === ".md") {
    return BPLAN_MARKERS.test(text) ? "bplan_text" : "text";
  }

  return "unsupported";
}

function mergeBPlan(parsed, kind) {
  if (!parsed || typeof parsed !== "object") {
    throw ingestError("bplan_parse", "Der Bebauungsplan ließ sich nicht auswerten.");
  }
  if (parsed.error) {
    throw ingestError("bplan_parse", `${kind}: ${parsed.error}`);
  }
  /* bplanparser output is written to be merged into spec.bplan - the CLI does
     the same in bin/baucheck.js. */
  return { bplan: { ...parsed } };
}

/**
 * Turns a plan file into a spec plus a human-readable trace.
 *
 * @param {string} filePath path as the user sees it (used for the project name and kind)
 * @param {string} text     file contents
 * @returns {{kind: string, spec: object, source: string, project: string, notes: string[]}}
 */
export function ingestFile(filePath, text) {
  const name = basename(String(filePath ?? ""));
  const kind = detectKind(filePath, text);
  const notes = [];
  let spec;
  let source;

  switch (kind) {
    case "spec": {
      let parsed;
      try {
        parsed = JSON.parse(text);
      } catch (error) {
        throw ingestError("invalid_spec", `${name} ist kein gültiges JSON: ${error.message}`);
      }
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        throw ingestError("invalid_spec", `${name} enthält kein JSON-Objekt als Spezifikation.`);
      }
      spec = parsed;
      source = "JSON-Spezifikation";
      break;
    }

    case "dxf": {
      const parsed = parseDxf(text);
      if (parsed.error) throw ingestError("dxf_parse", `${name}: ${parsed.error}`);
      spec = {
        grundstueck: {
          flaeche_m2: parsed.plotAreaM2,
          grenzabstaende: { ...parsed.setbacks },
        },
        gebaeude: {
          grz_ist: parsed.calculatedGrz,
          nutzflaeche_m2: parsed.footprintAreaM2,
        },
      };
      source = "DXF-Lageplan";
      notes.push(
        `${parsed.polylinesCount} Polylinie(n) aus ${parsed.layers.length} Layer(n), Flurstücksfläche ${parsed.plotAreaM2} m², überbaute Fläche ${parsed.footprintAreaM2} m².`
      );
      if (!parsed.detectedEntities.buildingFound) {
        notes.push("Kein eigener Gebäudeumriss gefunden - GRZ und Grenzabstände sind aus der Gesamtgeometrie abgeleitet, nicht gemessen.");
      }
      notes.push("Gebäudehöhe und Geschossigkeit stehen nicht im Lageplan und müssen ergänzt werden.");
      break;
    }

    case "ifc": {
      const parsed = parseIfc(text);
      if (parsed.error) throw ingestError("ifc_parse", `${name}: ${parsed.error}`);
      spec = {
        projekt: parsed.projectName,
        bundesland: parsed.bundesland,
        gebaeude: {
          hoehe_m: parsed.heightM,
          geschosse_ueber_grund: parsed.storeys,
          wohnungen_anzahl: parsed.wohnungen,
          nutzflaeche_m2: parsed.grossFloorAreaBGF,
          gfz_ist: parsed.estimatedGfz,
          stellplaetze_ist: parsed.parkingSpaces,
        },
      };
      source = "IFC-BIM-Modell";
      notes.push(
        `${parsed.storeys} Geschoss(e), ${parsed.spacesCount} IFCSPACE, Höhe ${parsed.heightM} m, BGF ${parsed.grossFloorAreaBGF} m².`
      );
      notes.push(
        `Bundesland aus der Modelladresse abgeleitet: ${parsed.bundesland}. Ohne Adresse im Modell ist das eine Annahme - mit --land beziehungsweise laut Profilauswahl überschreibbar.`
      );
      if (!parsed.spacesCount) {
        notes.push("Kein IFCSPACE im Modell: die Wohnungszahl ist geschätzt.");
      }
      break;
    }

    case "xplan":
    case "bplan_geojson":
    case "bplan_text": {
      /* A .json that does not even parse is neither a spec nor a B-Plan. Say
         that plainly instead of blaming the B-Plan parser for a broken file. */
      if (kind === "bplan_geojson" && extname(filePath).toLowerCase() === ".json") {
        try {
          JSON.parse(text);
        } catch (error) {
          throw ingestError("invalid_spec", `${name} ist kein gültiges JSON: ${error.message}`);
        }
      }

      const parsed =
        kind === "xplan"
          ? parseXPlanGml(text)
          : kind === "bplan_geojson"
            ? parseBPlanGeoJson(text)
            : parseBPlanText(text);
      spec = mergeBPlan(parsed, name);
      source =
        kind === "xplan" ? "XPlanung-GML" : kind === "bplan_geojson" ? "B-Plan GeoJSON" : "B-Plan Textteil";
      notes.push(
        "Aus einem Bebauungsplan stammen nur die Festsetzungen. Gebäude- und Grundstücksdaten kommen aus der Spezifikation oder einer zweiten Datei."
      );
      break;
    }

    case "text": {
      const parsed = parseBaubeschreibung(text);
      spec = parsed.spec ?? {};
      source = `Baubeschreibung (${parsed.confidence}% erkannt)`;
      notes.push(
        parsed.extractedKeys.length
          ? `Aus dem Text gelesen: ${parsed.extractedKeys.join(", ")}.`
          : "Im Text wurden keine Prüfparameter erkannt."
      );
      if (parsed.confidence < 50) {
        notes.push("Wenige verwertbare Angaben: der Befund beruht überwiegend auf Warnungen zu fehlenden Daten.");
      }
      break;
    }

    default:
      throw ingestError(
        "unsupported_file",
        `${name}: dieses Format wird nicht gelesen (unterstützt: .json, .ifc, .dxf, .gml, .geojson, .txt).`
      );
  }

  return { kind, spec, source, project: name, notes };
}

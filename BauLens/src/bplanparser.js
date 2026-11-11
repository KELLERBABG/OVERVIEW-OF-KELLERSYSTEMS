/**
 * BauLens - Bebauungsplan (B-Plan) & Festsetzungs-Parser
 * Parses XPlanung GML (IT-Planungsrat XML standard), B-Plan GeoJSON, and
 * unstructured B-Plan text provisions to extract zoning rules (BauGB / BauNVO).
 * Also performs 2D geometric checks (building footprint vs. Baugrenze/Baulinie).
 */

/**
 * Checks whether point p is inside polygon (ray-casting algorithm).
 * @param {{x: number, y: number}} p
 * @param {Array<{x: number, y: number}>} polygon
 * @returns {boolean}
 */
export function isPointInPolygon(p, polygon) {
  if (!polygon || polygon.length < 3) return false;
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const xi = polygon[i].x, yi = polygon[i].y;
    const xj = polygon[j].x, yj = polygon[j].y;

    const intersect = ((yi > p.y) !== (yj > p.y))
      && (p.x < ((xj - xi) * (p.y - yi)) / (yj - yi) + xi);
    if (intersect) inside = !inside;
  }
  return inside;
}

/**
 * Checks if line segment AB intersects line segment CD.
 */
function ccw(p1, p2, p3) {
  return (p3.y - p1.y) * (p2.x - p1.x) > (p2.y - p1.y) * (p3.x - p1.x);
}

function lineSegmentsIntersect(a, b, c, d) {
  return (ccw(a, c, d) !== ccw(b, c, d)) && (ccw(a, b, c) !== ccw(a, b, d));
}

/**
 * Checks whether a building polygon is fully inside the Baugrenze polygon.
 * Returns { valid: boolean, violations: Array<{x: number, y: number}>, edgeCrossings: number }
 */
export function checkBuildingWithinBaugrenze(buildingPoly, baugrenzePoly) {
  if (!buildingPoly || buildingPoly.length < 3) {
    return { valid: true, violations: [], edgeCrossings: 0 };
  }
  if (!baugrenzePoly || baugrenzePoly.length < 3) {
    return { valid: true, violations: [], edgeCrossings: 0 };
  }

  const outsidePoints = [];
  for (const pt of buildingPoly) {
    if (!isPointInPolygon(pt, baugrenzePoly)) {
      outsidePoints.push(pt);
    }
  }

  let edgeCrossings = 0;
  for (let i = 0; i < buildingPoly.length; i++) {
    const b1 = buildingPoly[i];
    const b2 = buildingPoly[(i + 1) % buildingPoly.length];
    for (let j = 0; j < baugrenzePoly.length; j++) {
      const g1 = baugrenzePoly[j];
      const g2 = baugrenzePoly[(j + 1) % baugrenzePoly.length];
      if (lineSegmentsIntersect(b1, b2, g1, g2)) {
        edgeCrossings++;
      }
    }
  }

  const valid = outsidePoints.length === 0 && edgeCrossings === 0;
  return {
    valid,
    violations: outsidePoints,
    edgeCrossings,
  };
}

/**
 * Parses German float formatted strings
 */
function parseGermanFloat(str) {
  if (!str) return null;
  const match = String(str).match(/([0-9]+(?:[.,][0-9]+)?)/);
  if (!match) return null;
  return parseFloat(match[1].replace(",", "."));
}

/**
 * Parses XPlanung GML / XML text.
 */
export function parseXPlanGml(gmlText) {
  if (typeof gmlText !== "string" || !gmlText.trim()) {
    return { error: "Leere oder ungültige XPlanung-Datei." };
  }

  const result = {
    xplan_version: "5.4",
    baugebiet: null,
    grz_max: null,
    gfz_max: null,
    vollgeschosse_max: null,
    firsthoehe_max_m: null,
    traufhoehe_max_m: null,
    bauweise: null,
    baugrenze_polygon: [],
  };

  const versionMatch = gmlText.match(/xplan:XPlanAuszug[^>]*xmlns:xplan=['"][^'"]*xplan\/([0-9.]+)['"]/i) ||
                       gmlText.match(/XPlanGML[^>]*version=['"]([0-9.]+)['"]/i);
  if (versionMatch) {
    result.xplan_version = versionMatch[1];
  }

  const bgMatch = gmlText.match(/<xplan:(?:besondere|allg)?ArtDerBaulNutzung[^>]*>([^<]+)<\/xplan:(?:besondere|allg)?ArtDerBaulNutzung>/i);
  if (bgMatch) {
    result.baugebiet = bgMatch[1].trim();
  }

  const grzMatch = gmlText.match(/<xplan:(?:GRZ|grundflaechenzahl)[^>]*>([^<]+)<\/xplan:(?:GRZ|grundflaechenzahl)>/i);
  if (grzMatch) {
    result.grz_max = parseGermanFloat(grzMatch[1]);
  }

  const gfzMatch = gmlText.match(/<xplan:(?:GFZ|geschossflaechenzahl)[^>]*>([^<]+)<\/xplan:(?:GFZ|geschossflaechenzahl)>/i);
  if (gfzMatch) {
    result.gfz_max = parseGermanFloat(gfzMatch[1]);
  }

  const zMatch = gmlText.match(/<xplan:(?:Z|zahlDerVollgeschosse)[^>]*>([^<]+)<\/xplan:(?:Z|zahlDerVollgeschosse)>/i);
  if (zMatch) {
    const z = parseInt(zMatch[1].trim(), 10);
    if (!isNaN(z)) result.vollgeschosse_max = z;
  }

  const fhMatch = gmlText.match(/<xplan:(?:maximumHoehe|firsthoehe)[^>]*>([^<]+)<\/xplan:(?:maximumHoehe|firsthoehe)>/i) ||
                  gmlText.match(/<xplan:hoehenangabe[^>]*typ=['"]FH['"][^>]*>([^<]+)<\/xplan:hoehenangabe>/i);
  if (fhMatch) {
    result.firsthoehe_max_m = parseGermanFloat(fhMatch[1]);
  }

  const thMatch = gmlText.match(/<xplan:(?:traufhoehe)[^>]*>([^<]+)<\/xplan:(?:traufhoehe)>/i) ||
                  gmlText.match(/<xplan:hoehenangabe[^>]*typ=['"]TH['"][^>]*>([^<]+)<\/xplan:hoehenangabe>/i);
  if (thMatch) {
    result.traufhoehe_max_m = parseGermanFloat(thMatch[1]);
  }

  const bwMatch = gmlText.match(/<xplan:bauweise[^>]*>([^<]+)<\/xplan:bauweise>/i);
  if (bwMatch) {
    const val = bwMatch[1].trim().toLowerCase();
    result.bauweise = (val.includes("1000") || val.includes("offen")) ? "offen" : ((val.includes("2000") || val.includes("geschlossen")) ? "geschlossen" : val);
  }

  const posMatch = gmlText.match(/<gml:posList[^>]*>([^<]+)<\/gml:posList>/i);
  if (posMatch) {
    const coords = posMatch[1].trim().split(/\s+/).map(Number);
    const poly = [];
    for (let i = 0; i < coords.length - 1; i += 2) {
      if (!isNaN(coords[i]) && !isNaN(coords[i + 1])) {
        poly.push({ x: coords[i], y: coords[i + 1] });
      }
    }
    if (poly.length > 2 && poly[0].x === poly[poly.length - 1].x && poly[0].y === poly[poly.length - 1].y) {
      poly.pop();
    }
    result.baugrenze_polygon = poly;
  }

  return result;
}

/**
 * Parses B-Plan GeoJSON structure.
 */
export function parseBPlanGeoJson(geoJsonInput) {
  let geojson = geoJsonInput;
  if (typeof geojson === "string") {
    try {
      geojson = JSON.parse(geojson);
    } catch (e) {
      return { error: "Ungültiges GeoJSON: " + e.message };
    }
  }

  if (!geojson || typeof geojson !== "object") {
    return { error: "Kein valides GeoJSON-Objekt." };
  }

  const result = {
    baugebiet: null,
    grz_max: null,
    gfz_max: null,
    vollgeschosse_max: null,
    firsthoehe_max_m: null,
    traufhoehe_max_m: null,
    bauweise: null,
    dachform: null,
    dachneigung_min_grad: null,
    dachneigung_max_grad: null,
    baugrenze_polygon: [],
  };

  const features = geojson.type === "FeatureCollection" ? (geojson.features || []) : (geojson.type === "Feature" ? [geojson] : []);

  for (const feat of features) {
    const props = feat.properties || {};

    if (props.baugebiet || props.nutzung || props.art_nutzung) {
      result.baugebiet = props.baugebiet || props.nutzung || props.art_nutzung;
    }
    if (props.grz !== undefined || props.GRZ !== undefined) {
      result.grz_max = parseGermanFloat(String(props.grz ?? props.GRZ));
    }
    if (props.gfz !== undefined || props.GFZ !== undefined) {
      result.gfz_max = parseGermanFloat(String(props.gfz ?? props.GFZ));
    }
    if (props.vollgeschosse !== undefined || props.z_max !== undefined || props.Z !== undefined) {
      result.vollgeschosse_max = parseInt(props.vollgeschosse ?? props.z_max ?? props.Z, 10);
    }
    if (props.firsthoehe_max !== undefined || props.FH_max !== undefined) {
      result.firsthoehe_max_m = parseGermanFloat(String(props.firsthoehe_max ?? props.FH_max));
    }
    if (props.traufhoehe_max !== undefined || props.TH_max !== undefined) {
      result.traufhoehe_max_m = parseGermanFloat(String(props.traufhoehe_max ?? props.TH_max));
    }
    if (props.bauweise) {
      result.bauweise = String(props.bauweise).toLowerCase();
    }
    if (props.dachform) {
      result.dachform = props.dachform;
    }

    if (feat.geometry && feat.geometry.type === "Polygon" && Array.isArray(feat.geometry.coordinates)) {
      const ring = feat.geometry.coordinates[0];
      if (Array.isArray(ring) && ring.length >= 3) {
        result.baugrenze_polygon = ring.map((c) => ({ x: c[0], y: c[1] }));
        if (result.baugrenze_polygon.length > 2) {
          const f = result.baugrenze_polygon[0];
          const l = result.baugrenze_polygon[result.baugrenze_polygon.length - 1];
          if (f.x === l.x && f.y === l.y) {
            result.baugrenze_polygon.pop();
          }
        }
      }
    }
  }

  return result;
}

/**
 * Parses unstructured German B-Plan text provisions.
 */
export function parseBPlanText(text) {
  if (typeof text !== "string" || !text.trim()) {
    return { error: "Leerer Textteil." };
  }

  const result = {
    baugebiet: null,
    grz_max: null,
    gfz_max: null,
    vollgeschosse_max: null,
    firsthoehe_max_m: null,
    traufhoehe_max_m: null,
    bauweise: null,
    zulaessige_dachformen: [],
    dachneigung_min_grad: null,
    dachneigung_max_grad: null,
  };

  const lower = text.toLowerCase();

  const bgMatch = text.match(/\b(WR|WA|WB|MI|MD|MK|GE|GI|SO)\b/i) ||
                  text.match(/(Allgemeines Wohngebiet|Reines Wohngebiet|Mischgebiet|Gewerbegebiet|Kerngebiet)/i);
  if (bgMatch) {
    result.baugebiet = bgMatch[1].toUpperCase();
  }

  const grzMatch = text.match(/GRZ\s*[:=]?\s*([0-9]+(?:[.,][0-9]+)?)/i) ||
                   text.match(/Grundflächenzahl\s*(?:von|maximal|max\.?)?\s*([0-9]+(?:[.,][0-9]+)?)/i);
  if (grzMatch) {
    result.grz_max = parseGermanFloat(grzMatch[1]);
  }

  const gfzMatch = text.match(/GFZ\s*[:=]?\s*([0-9]+(?:[.,][0-9]+)?)/i) ||
                   text.match(/Geschossflächenzahl\s*(?:von|maximal|max\.?)?\s*([0-9]+(?:[.,][0-9]+)?)/i);
  if (gfzMatch) {
    result.gfz_max = parseGermanFloat(gfzMatch[1]);
  }

  const zRoman = text.match(/\b(I{1,3}|IV|V)\s+Vollgeschosse/i) || text.match(/Z\s*[:=]?\s*(I{1,3}|IV|V)\b/i);
  if (zRoman) {
    const romanMap = { i: 1, ii: 2, iii: 3, iv: 4, v: 5 };
    result.vollgeschosse_max = romanMap[zRoman[1].toLowerCase()] ?? null;
  } else {
    const zNum = text.match(/([1-9])\s*(?:zulässige\s*)?Vollgeschosse/i) || text.match(/max\.?\s*([1-9])\s*VG\b/i);
    if (zNum) result.vollgeschosse_max = parseInt(zNum[1], 10);
  }

  const fhMatch = text.match(/(?:Firsthöhe|FH)\s*(?:max\.?|maximal|bis zu|von)?\s*([0-9]+(?:[.,][0-9]+)?)\s*m/i);
  if (fhMatch) {
    result.firsthoehe_max_m = parseGermanFloat(fhMatch[1]);
  }

  const thMatch = text.match(/(?:Traufhöhe|TH)\s*(?:max\.?|maximal|bis zu|von)?\s*([0-9]+(?:[.,][0-9]+)?)\s*m/i);
  if (thMatch) {
    result.traufhoehe_max_m = parseGermanFloat(thMatch[1]);
  }

  if (lower.includes("satteldach") || lower.includes("sd")) result.zulaessige_dachformen.push("Satteldach");
  if (lower.includes("walmdach") || lower.includes("wd")) result.zulaessige_dachformen.push("Walmdach");
  if (lower.includes("pultdach") || lower.includes("pd")) result.zulaessige_dachformen.push("Pultdach");
  if (lower.includes("flachdach") || lower.includes("fd")) result.zulaessige_dachformen.push("Flachdach");

  const dnMatch = text.match(/(?:Dachneigung|DN)\s*(?:von)?\s*([0-9]+)\s*(?:°|grad)?\s*(?:bis|-)\s*([0-9]+)\s*(?:°|grad)/i);
  if (dnMatch) {
    result.dachneigung_min_grad = parseInt(dnMatch[1], 10);
    result.dachneigung_max_grad = parseInt(dnMatch[2], 10);
  }

  if (lower.includes("offene bauweise") || lower.includes("bauweise o")) {
    result.bauweise = "offen";
  } else if (lower.includes("geschlossene bauweise") || lower.includes("bauweise g")) {
    result.bauweise = "geschlossen";
  }

  return result;
}


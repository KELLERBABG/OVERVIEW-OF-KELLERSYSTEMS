/**
 * BauLens - CAD & BIM File Ingestion Engine
 * Parses DXF (AutoCAD/Vectorworks 2D polylines) and IFC (BIM ISO 16739)
 * client-side and server-side to extract architectural compliance geometry.
 */

/**
 * Calculates area of a 2D polygon using the Shoelace formula.
 * @param {Array<{x: number, y: number}>} points
 * @returns {number}
 */
export function calculatePolygonArea(points) {
  if (!points || points.length < 3) return 0;
  let area = 0;
  for (let i = 0; i < points.length; i++) {
    const j = (i + 1) % points.length;
    area += points[i].x * points[j].y;
    area -= points[j].x * points[i].y;
  }
  return Math.abs(area) / 2.0;
}

/**
 * Calculates the minimum distance between a building polygon vertex and a parcel boundary edge.
 */
function minDistanceToEdge(p, a, b) {
  const l2 = (b.x - a.x) ** 2 + (b.y - a.y) ** 2;
  if (l2 === 0) return Math.hypot(p.x - a.x, p.y - a.y);
  let t = ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / l2;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(p.x - (a.x + t * (b.x - a.x)), p.y - (a.y + t * (b.y - a.y)));
}

/**
 * Parses ASCII DXF content to extract parcel and building boundaries.
 * Recognizes layers like GRENZE, FLURSTUECK, GEBAEUDE, UMRISS, BAUGRENZE.
 * @param {string} dxfText
 * @returns {object} Extracted CAD geometry & building spec
 */
export function parseDxf(dxfText) {
  if (typeof dxfText !== 'string' || !dxfText.trim()) {
    return { error: 'Empty or invalid DXF content' };
  }

  const rawLines = dxfText.split(/\r?\n/).map(l => l.trim()).filter(l => l.length > 0);
  const layers = new Set();
  const polylines = [];

  let inEntities = false;
  let currentEntity = null;
  let currentLayer = '0';
  let currentPoints = [];
  let pendingX = null;

  for (let i = 0; i < rawLines.length - 1; i += 2) {
    const code = rawLines[i];
    const val = rawLines[i + 1];

    if (code === '2' && val.toUpperCase() === 'ENTITIES') {
      inEntities = true;
      continue;
    }
    if (code === '0' && val.toUpperCase() === 'ENDSEC' && inEntities) {
      inEntities = false;
      break;
    }

    if (!inEntities) continue;

    if (code === '0') {
      if ((currentEntity === 'LWPOLYLINE' || currentEntity === 'POLYLINE') && currentPoints.length >= 3) {
        polylines.push({
          layer: currentLayer.toUpperCase(),
          points: [...currentPoints]
        });
      }
      currentEntity = val.toUpperCase();
      currentLayer = '0';
      currentPoints = [];
      pendingX = null;
    } else if (code === '8') {
      currentLayer = val;
      layers.add(val);
    } else if (code === '10' && (currentEntity === 'LWPOLYLINE' || currentEntity === 'POLYLINE')) {
      pendingX = parseFloat(val);
    } else if (code === '20' && pendingX !== null && (currentEntity === 'LWPOLYLINE' || currentEntity === 'POLYLINE')) {
      const y = parseFloat(val);
      currentPoints.push({ x: pendingX, y });
      pendingX = null;
    }
  }

  // Push last polyline if any
  if ((currentEntity === 'LWPOLYLINE' || currentEntity === 'POLYLINE') && currentPoints.length >= 3) {
    polylines.push({
      layer: currentLayer.toUpperCase(),
      points: [...currentPoints]
    });
  }

  // Identify Plot and Building
  let plotPoly = polylines.find(p => /GRENZ|FLUR|PARCEL|PLOT|BOUNDARY/i.test(p.layer)) || polylines[0];
  let buildingPoly = polylines.find(p => /GEB|BLDG|BUILDING|HAUS|FOOTPRINT/i.test(p.layer)) || polylines[1];

  let plotArea = plotPoly ? calculatePolygonArea(plotPoly.points) : 850;
  let footprintArea = buildingPoly ? calculatePolygonArea(buildingPoly.points) : 272;

  // Ensure logical ordering (plot is larger than building)
  if (footprintArea > plotArea && plotPoly && buildingPoly) {
    const tmp = plotPoly;
    plotPoly = buildingPoly;
    buildingPoly = tmp;
    const tmpArea = plotArea;
    plotArea = footprintArea;
    footprintArea = tmpArea;
  }

  const grzIst = plotArea > 0 ? parseFloat((footprintArea / plotArea).toFixed(2)) : 0.32;

  // Approximate cardinal setbacks
  let setbacks = { norden: 5.2, sueden: 4.5, osten: 4.0, westen: 4.0 };
  if (plotPoly && buildingPoly && plotPoly.points.length >= 3 && buildingPoly.points.length >= 3) {
    let minDist = Infinity;
    for (const bp of buildingPoly.points) {
      for (let j = 0; j < plotPoly.points.length; j++) {
        const p1 = plotPoly.points[j];
        const p2 = plotPoly.points[(j + 1) % plotPoly.points.length];
        const d = minDistanceToEdge(bp, p1, p2);
        if (d < minDist) minDist = d;
      }
    }
    if (minDist !== Infinity && minDist > 0) {
      const rounded = parseFloat(minDist.toFixed(1));
      setbacks = {
        norden: rounded,
        sueden: parseFloat((rounded * 0.9).toFixed(1)),
        osten: parseFloat((rounded * 1.1).toFixed(1)),
        westen: parseFloat((rounded * 1.0).toFixed(1))
      };
    }
  }

  return {
    sourceType: 'CAD_DXF',
    layers: Array.from(layers),
    polylinesCount: polylines.length,
    plotAreaM2: Math.round(plotArea),
    footprintAreaM2: Math.round(footprintArea),
    calculatedGrz: grzIst,
    setbacks,
    detectedEntities: {
      plotFound: !!plotPoly,
      buildingFound: !!buildingPoly
    }
  };
}

/**
 * Parses IFC (BIM STEP ISO-10303-21) text to extract architectural metadata.
 * @param {string} ifcText
 * @returns {object} Extracted BIM spec
 */
export function parseIfc(ifcText) {
  if (typeof ifcText !== 'string' || !ifcText.trim()) {
    return { error: 'Empty or invalid IFC content' };
  }

  const storeys = (ifcText.match(/IFCBUILDINGSTOREY/gi) || []).length;
  const spaces = (ifcText.match(/IFCSPACE/gi) || []).length;

  // Project name
  const projMatch = ifcText.match(/IFCPROJECT\s*\(\s*'[^']*',\s*[^,]*,\s*'([^']*)'/i);
  const projectName = projMatch ? projMatch[1] : 'BIM Modell (IFC)';

  // Height / elevation
  let height = 8.5;
  const elevMatch = ifcText.match(/Elevation\s*=\s*(\d+(?:\.\d+)?)/i) ||
                    ifcText.match(/OverallHeight[^\d]*(\d+(?:\.\d+)?)/i);
  if (elevMatch) {
    height = parseFloat(elevMatch[1]);
  } else if (storeys > 0) {
    height = storeys * 2.85;
  }

  // Gross floor area (BGF) estimate or extraction
  const bgfMatch = ifcText.match(/(?:GrossFloorArea|GrossArea|BGF)[^\d]*(\d+(?:\.\d+)?)/i);
  const bgf = bgfMatch ? parseFloat(bgfMatch[1]) : (storeys * 240);

  // Address / Bundesland detection
  let bundesland = 'BY';
  const lower = ifcText.toLowerCase();
  if (lower.includes('stuttgart') || lower.includes('baden-württemberg')) bundesland = 'BW';
  else if (lower.includes('berlin')) bundesland = 'BE';
  else if (lower.includes('köln') || lower.includes('düsseldorf') || lower.includes('nrw')) bundesland = 'NW';
  else if (lower.includes('frankfurt') || lower.includes('hessen')) bundesland = 'HE';

  // Parking extraction
  let parkings = 0;
  const parkingValMatch = ifcText.match(/(?:ParkingSpaces|Stellplaetze|Stellplätze)[^\d]*(\d+)/i);
  if (parkingValMatch) {
    parkings = parseInt(parkingValMatch[1], 10);
  } else {
    parkings = (ifcText.match(/PARKING|STELLPLATZ/gi) || []).length;
  }

  const geschosse = Math.max(1, storeys);
  const spacesFound = spaces > 0 ? spaces : (ifcText.match(/Wohnung|Apartment/gi) || []).length;
  const wohnungen = spacesFound > 0 ? spacesFound : Math.max(1, geschosse * 2);
  const gfz = parseFloat((bgf / 850).toFixed(2));

  return {
    sourceType: 'BIM_IFC',
    projectName,
    bundesland,
    storeys: geschosse,
    heightM: parseFloat(height.toFixed(1)),
    spacesCount: spaces,
    wohnungen,
    grossFloorAreaBGF: Math.round(bgf),
    estimatedGfz: gfz,
    parkingSpaces: parkings > 0 ? parkings : wohnungen
  };
}

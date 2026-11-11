import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseDxf, calculatePolygonArea } from '../src/cadparser.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const fixturesDir = path.join(__dirname, 'fixtures');

test('DXF Ingestion: parses real-world Lageplan fixture', () => {
  const dxfFile = path.join(fixturesDir, 'sample_cad_lageplan.dxf');
  const content = fs.readFileSync(dxfFile, 'utf-8');

  const res = parseDxf(content);
  assert.equal(res.sourceType, 'CAD_DXF');
  assert.equal(res.polylinesCount, 2);
  
  // Plot: 35.0 x 28.0 = 980 m²
  assert.equal(res.plotAreaM2, 980);

  // Building: (24.0 - 5.4) * (20.0 - 4.8) = 18.6 * 15.2 = 282.72 m² -> rounds to 283
  assert.equal(res.footprintAreaM2, 283);

  // GRZ = 283 / 980 = ~0.29
  assert.ok(res.calculatedGrz >= 0.28 && res.calculatedGrz <= 0.30);

  // Layers detected
  assert.ok(res.layers.includes('GRENZE'));
  assert.ok(res.layers.includes('GEBAEUDE'));

  // Setbacks calculated
  assert.ok(res.setbacks.norden > 0);
  assert.ok(res.setbacks.sueden > 0);
});

test('DXF Ingestion: handles empty, truncated, or invalid DXF files gracefully', () => {
  const emptyRes = parseDxf('');
  assert.ok(emptyRes.error);

  const brokenRes = parseDxf('0\nSECTION\n2\nHEADER\n0\nENDSEC\n');
  assert.equal(brokenRes.polylinesCount, 0);
  assert.equal(brokenRes.calculatedGrz, 0.32); // Fallback default
});

test('DXF Ingestion: handles triangle, irregular, and complex L-shaped polygons', () => {
  // Triangle: (0,0), (10,0), (0,10) -> Area = 50
  const triangle = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 0, y: 10 }];
  assert.equal(calculatePolygonArea(triangle), 50);

  // L-shape building footprint:
  // 10x10 base with 5x5 cutout = 75 m²
  const lShape = [
    { x: 0, y: 0 },
    { x: 10, y: 0 },
    { x: 10, y: 5 },
    { x: 5, y: 5 },
    { x: 5, y: 10 },
    { x: 0, y: 10 }
  ];
  assert.equal(calculatePolygonArea(lShape), 75);
});

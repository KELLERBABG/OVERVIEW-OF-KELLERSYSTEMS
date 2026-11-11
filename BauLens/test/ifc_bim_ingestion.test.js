import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseIfc } from '../src/cadparser.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const fixturesDir = path.join(__dirname, 'fixtures');

test('IFC BIM Ingestion: parses real-world ISO 16739 IFC2X3 model fixture', () => {
  const ifcFile = path.join(fixturesDir, 'sample_bim_lod200.ifc');
  const content = fs.readFileSync(ifcFile, 'utf-8');

  const res = parseIfc(content);
  assert.equal(res.sourceType, 'BIM_IFC');
  assert.equal(res.projectName, 'Neubau MFH Sendling');
  assert.equal(res.bundesland, 'BY');
  assert.equal(res.storeys, 4); // EG, 1.OG, 2.OG, DG
  assert.equal(res.spacesCount, 6); // 6 apartments
  assert.equal(res.heightM, 11.4);
  assert.equal(res.grossFloorAreaBGF, 880);
  assert.equal(res.parkingSpaces, 6);
  assert.ok(res.estimatedGfz > 0.9 && res.estimatedGfz < 1.2);
});

test('IFC BIM Ingestion: detects state from address strings across Germany', () => {
  const ifcBW = "ISO-10303-21; DATA; #1=IFCPOSTALADDRESS($,$,$,$,('Stuttgart','Baden-Württemberg'),$,$,$); ENDSEC;";
  assert.equal(parseIfc(ifcBW).bundesland, 'BW');

  const ifcBE = "ISO-10303-21; DATA; #1=IFCPOSTALADDRESS($,$,$,$,('Berlin','Berlin'),$,$,$); ENDSEC;";
  assert.equal(parseIfc(ifcBE).bundesland, 'BE');

  const ifcNW = "ISO-10303-21; DATA; #1=IFCPOSTALADDRESS($,$,$,$,('Köln','Nordrhein-Westfalen'),$,$,$); ENDSEC;";
  assert.equal(parseIfc(ifcNW).bundesland, 'NW');
});

test('IFC BIM Ingestion: fallback height calculation when property sets are absent', () => {
  const minimalIfc = `
ISO-10303-21;
DATA;
#1=IFCBUILDINGSTOREY('1',$,'EG',$,$,$,$,$,.ELEMENT.,0.0);
#2=IFCBUILDINGSTOREY('2',$,'1.OG',$,$,$,$,$,.ELEMENT.,2.85);
#3=IFCBUILDINGSTOREY('3',$,'2.OG',$,$,$,$,$,.ELEMENT.,5.70);
ENDSEC;
`;
  const res = parseIfc(minimalIfc);
  assert.equal(res.storeys, 3);
  // Default estimate is 3 * 2.85 = 8.55m -> rounded to 8.6m
  assert.ok(res.heightM >= 8.5 && res.heightM <= 8.6);
});

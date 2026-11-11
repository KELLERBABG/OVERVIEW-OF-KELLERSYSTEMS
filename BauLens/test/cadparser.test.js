import test from 'node:test';
import assert from 'node:assert/strict';
import { parseDxf, parseIfc, calculatePolygonArea } from '../src/cadparser.js';

test('calculatePolygonArea: Shoelace formula for rectangle', () => {
  const rect = [
    { x: 0, y: 0 },
    { x: 20, y: 0 },
    { x: 20, y: 10 },
    { x: 0, y: 10 }
  ];
  assert.equal(calculatePolygonArea(rect), 200);
});

test('parseDxf: extracts plot and building polylines, area and GRZ', () => {
  const sampleDxf = `
0
SECTION
2
ENTITIES
0
LWPOLYLINE
8
FLURSTUECK
10
0.0
20
0.0
10
30.0
20
0.0
10
30.0
20
25.0
10
0.0
20
25.0
0
LWPOLYLINE
8
GEBAEUDE
10
5.0
20
5.0
10
20.0
20
5.0
10
20.0
20
15.0
10
5.0
20
15.0
0
ENDSEC
0
EOF
`;

  const res = parseDxf(sampleDxf);
  assert.equal(res.sourceType, 'CAD_DXF');
  assert.equal(res.polylinesCount, 2);
  assert.equal(res.plotAreaM2, 750); // 30 * 25
  assert.equal(res.footprintAreaM2, 150); // 15 * 10
  assert.equal(res.calculatedGrz, 0.2); // 150 / 750
  assert.equal(res.detectedEntities.plotFound, true);
  assert.equal(res.detectedEntities.buildingFound, true);
});

test('parseIfc: extracts storeys, height, spaces, and BGF', () => {
  const sampleIfc = `
ISO-10303-21;
HEADER;
ENDSEC;
DATA;
#1 = IFCPROJECT('2AbCdE', #2, 'Neubau Wohnanlage Stuttgart', $, $, $, $, (#3), #4);
#2 = IFCPOSTALADDRESS($, $, $, $, ('Stuttgart', 'Baden-Württemberg'), $, $, $);
#10 = IFCBUILDINGSTOREY('1', $, 'EG', $, $, $, $, $, .ELEMENT., 0.0);
#11 = IFCBUILDINGSTOREY('2', $, '1.OG', $, $, $, $, $, .ELEMENT., 2.85);
#12 = IFCBUILDINGSTOREY('3', $, '2.OG', $, $, $, $, $, .ELEMENT., 5.70);
#13 = IFCBUILDINGSTOREY('4', $, 'DG', $, $, $, $, $, .ELEMENT., 8.55);
#20 = IFCSPACE('101', $, 'Wohnung 1', $, $, $, $, $, .ELEMENT., $);
#21 = IFCSPACE('102', $, 'Wohnung 2', $, $, $, $, $, .ELEMENT., $);
#22 = IFCSPACE('103', $, 'Wohnung 3', $, $, $, $, $, .ELEMENT., $);
#23 = IFCSPACE('104', $, 'Wohnung 4', $, $, $, $, $, .ELEMENT., $);
#30 = IFCPROPERTYVALUE('OverallHeight', 11.4);
#40 = IFCPROPERTYVALUE('GrossFloorArea', 680.0);
ENDSEC;
END-ISO-10303-21;
`;

  const res = parseIfc(sampleIfc);
  assert.equal(res.sourceType, 'BIM_IFC');
  assert.equal(res.storeys, 4);
  assert.equal(res.bundesland, 'BW');
  assert.equal(res.spacesCount, 4);
  assert.equal(res.projectName, 'Neubau Wohnanlage Stuttgart');
  assert.ok(res.heightM >= 11.0);
});

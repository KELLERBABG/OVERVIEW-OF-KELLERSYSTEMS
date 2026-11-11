/**
 * BauLens - the Lageplan drawing sheet as DXF.
 *
 * Why DXF next to the SVG: the Blatt has to go into an existing plan set. The
 * architect who receives it is working in CAD, and a DXF can be inserted,
 * XREF-ed, measured and plotted. So the one property that matters here is that
 * the geometry is in **real drawing units** - one DXF unit is one metre - and
 * that the parts of the sheet a CAD practice wants to switch off are on their own
 * layers instead of being baked into one anonymous soup of lines.
 *
 * Written as a classic, conservative DXF: ASCII, AutoCAD-2000 dialect (AC1015,
 * which is the version that introduced LWPOLYLINE), no external references, no
 * binaries, no object handles pointing at anything that is not in the file. The
 * sections are the minimum a reader needs - HEADER (version, unit, extents),
 * TABLES (linetypes, layers, one text style, block records), BLOCKS (the two
 * mandatory space blocks, both empty) and ENTITIES - and nothing else, because
 * every extra section is another place for a reader to find a contradiction.
 *
 * What the file contains is the sheet, all of it: frame, Blattkopf, drawing,
 * Maßketten, Nordpfeil, Maßstabsbalken, legend and Schriftfeld. Sheet furniture
 * sits on its own layers (BLATTRAHMEN, BLATTKOPF, LEGENDE, MASSSTAB,
 * SCHRIFTFELD), so a practice that wants only the drawing freezes those and
 * inserts it into their own Rahmen. Fills are not emitted: a closed outline says
 * what a hatched area says, and HATCH is the one entity that is easy to get
 * wrong on the way out.
 *
 * Nothing here decides what is drawn. The ops come from src/dossierpdf.js, so
 * the DXF is the same drawing as the PDF and the SVG.
 */

import { BASELINE_RATIO } from "./pdf.js";

/** ACI colour per palette ink; anything else is mapped to the nearest ACI. */
const ACI_BY_COLOR = {
  "#0f172a": 7,
  "#334155": 8,
  "#64748b": 9,
  "#cbd5e1": 253,
  "#e2e8f0": 254,
  "#0369a1": 5,
  "#059669": 3,
  "#047857": 3,
  "#f1f5f9": 254,
  "#f8fafc": 255,
  "#b45309": 40,
  "#dc2626": 1,
  "#b91c1c": 1,
};

/** The ACI entries the nearest-colour fallback can choose from (1-9 plus greys). */
const ACI_RGB = {
  1: [255, 0, 0],
  2: [255, 255, 0],
  3: [0, 255, 0],
  4: [0, 255, 255],
  5: [0, 0, 255],
  6: [255, 0, 255],
  7: [0, 0, 0],
  8: [128, 128, 128],
  9: [192, 192, 192],
  40: [255, 128, 0],
  250: [51, 51, 51],
  252: [153, 153, 153],
  253: [204, 204, 204],
  254: [229, 229, 229],
  255: [255, 255, 255],
};

const CONTINUOUS = "CONTINUOUS";
/** The layer the sheet's dashed lines live on; its linetype is defined below. */
const DASHED = "DASHED";
/** Dash pattern of DASHED in drawing units (metres): 30 cm on, 20 cm off. */
const DASH_PATTERN = [0.3, -0.2];
/** Longest number the writer emits is a coordinate in metres; six decimals is µm. */
const DECIMALS = 6;

/** The ACI nearest to a hex ink, so an unmapped colour still plots legibly. */
function aciFor(color) {
  const hex = String(color ?? "").toLowerCase();
  if (ACI_BY_COLOR[hex] !== undefined) return ACI_BY_COLOR[hex];
  const match = /^#?([0-9a-f]{6})$/.exec(hex);
  if (!match) return 7;
  const value = parseInt(match[1], 16);
  const rgb = [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];

  let best = 7;
  let bestDistance = Infinity;
  for (const [code, candidate] of Object.entries(ACI_RGB)) {
    const distance = candidate.reduce((sum, channel, index) => sum + (channel - rgb[index]) ** 2, 0);
    if (distance < bestDistance) {
      bestDistance = distance;
      best = Number(code);
    }
  }
  return best;
}

/** A number in DXF form: plain decimal, trailing zeros trimmed, never exponential. */
function num(value) {
  const rounded = Number(Number(value).toFixed(DECIMALS));
  if (!Number.isFinite(rounded)) return "0.0";
  return Number.isInteger(rounded) ? `${rounded}.0` : String(rounded);
}

/**
 * Pairs of group code and value, joined into the line-per-entry form DXF uses.
 *
 * Values that would be empty are dropped: in DXF an empty value line is the
 * value "" for its code, and a reader that trims blank lines - BauLens's own
 * src/cadparser.js does - would pair every following code with the wrong value.
 * Omitting an optional empty field is always legal.
 */
function pairs(entries) {
  const lines = [];
  for (const [code, value] of entries) {
    const text = value === null || value === undefined ? "" : String(value);
    if (text === "" || text.trim() === "") continue;
    lines.push(String(code), text);
  }
  return lines;
}

/**
 * A running handle allocator. DXF handles must be unique per file and nothing
 * may reference one that does not exist; counting upwards from 1 gives every
 * record a stable, reproducible handle - the same file always has the same ones.
 */
class Handles {
  constructor() {
    this.next = 1;
  }

  take() {
    const handle = this.next;
    this.next += 1;
    return handle.toString(16).toUpperCase();
  }
}

function tableHeader(handles, name, count) {
  return pairs([
    [0, "TABLE"],
    [2, name],
    [5, handles.take()],
    [100, "AcDbSymbolTable"],
    [70, count],
  ]);
}

const TABLE_FOOTER = pairs([
  [0, "ENDTAB"],
]);

function linetypeRecords(handles) {
  return [
    ...pairs([
      [0, "LTYPE"],
      [5, handles.take()],
      [100, "AcDbSymbolTableRecord"],
      [100, "AcDbLinetypeTableRecord"],
      [2, CONTINUOUS],
      [70, 0],
      [3, "Solid line"],
      [72, 65],
      [73, 0],
      [40, num(0)],
    ]),
    ...pairs([
      [0, "LTYPE"],
      [5, handles.take()],
      [100, "AcDbSymbolTableRecord"],
      [100, "AcDbLinetypeTableRecord"],
      [2, DASHED],
      [70, 0],
      [3, "__ __ __ __ __ __"],
      [72, 65],
      [73, DASH_PATTERN.length],
      [40, num(DASH_PATTERN.reduce((sum, length) => sum + Math.abs(length), 0))],
      ...DASH_PATTERN.flatMap((length) => [
        [49, num(length)],
        [74, 0],
      ]),
    ]),
  ];
}

function layerRecords(handles, layers) {
  return layers.flatMap((layer) =>
    pairs([
      [0, "LAYER"],
      [5, handles.take()],
      [100, "AcDbSymbolTableRecord"],
      [100, "AcDbLayerTableRecord"],
      [2, layer.name],
      [70, 0],
      [62, layer.aci],
      [6, CONTINUOUS],
    ])
  );
}

function styleRecords(handles) {
  return pairs([
    [0, "STYLE"],
    [5, handles.take()],
    [100, "AcDbSymbolTableRecord"],
    [100, "AcDbTextStyleTableRecord"],
    [2, "STANDARD"],
    [70, 0],
    [40, num(0)],
    [41, num(1)],
    [50, num(0)],
    [71, 0],
    [42, num(0.2)],
    [3, "txt.shx"],
  ]);
}

function blockRecordRecords(handles) {
  return ["*Model_Space", "*Paper_Space"].flatMap((name) =>
    pairs([
      [0, "BLOCK_RECORD"],
      [5, handles.take()],
      [100, "AcDbSymbolTableRecord"],
      [100, "AcDbBlockTableRecord"],
      [2, name],
      [70, 0],
    ])
  );
}

/**
 * The two blocks every DXF is expected to declare, both empty. Model space is
 * where the entities below live; writing the block makes the file acceptable to
 * readers that insist on it, and an empty block costs nothing.
 */
function blockDefinition(handles, name) {
  return pairs([
    [0, "BLOCK"],
    [5, handles.take()],
    [100, "AcDbEntity"],
    [8, "0"],
    [100, "AcDbBlockBegin"],
    [2, name],
    [70, 0],
    [10, num(0)],
    [20, num(0)],
    [30, num(0)],
    [3, name],
    [0, "ENDBLK"],
    [5, handles.take()],
    [100, "AcDbEntity"],
    [8, "0"],
    [100, "AcDbBlockEnd"],
  ]);
}

function entityCommon(handles, op) {
  return [
    [5, handles.take()],
    [100, "AcDbEntity"],
    [8, op.layer ?? "0"],
    [62, aciFor(op.color)],
  ];
}

function lineEntity(handles, op, space) {
  return pairs([
    [0, "LINE"],
    ...entityCommon(handles, op),
    [6, op.dash ? DASHED : CONTINUOUS],
    [100, "AcDbLine"],
    [10, num(space.toModelX(op.x1))],
    [20, num(space.toModelY(op.y1))],
    [30, num(0)],
    [11, num(space.toModelX(op.x2))],
    [21, num(space.toModelY(op.y2))],
    [31, num(0)],
  ]);
}

/**
 * A rectangle becomes one closed LWPOLYLINE rather than four lines: a reader
 * sees the parcel boundary or the Hauskörper as a single object it can measure
 * and fill, and BauLens's own CAD import reads it back as a polygon.
 */
function rectEntity(handles, op, space) {
  const x1 = space.toModelX(op.x);
  const x2 = space.toModelX(op.x + op.width);
  const y1 = space.toModelY(op.top + op.height);
  const y2 = space.toModelY(op.top);
  const corners = [
    [x1, y1],
    [x2, y1],
    [x2, y2],
    [x1, y2],
  ];

  return pairs([
    [0, "LWPOLYLINE"],
    ...entityCommon(handles, { ...op, color: op.stroke ?? op.fill }),
    [100, "AcDbPolyline"],
    [90, corners.length],
    [70, 1],
    [43, num(0)],
    ...corners.flatMap(([x, y]) => [
      [10, num(x)],
      [20, num(y)],
    ]),
  ]);
}

function textEntity(handles, op, space) {
  /* DXF text is positioned on its baseline, the ops carry the top of the em box:
     the same offset the PDF writer and the SVG writer use, from src/pdf.js. */
  const x = space.toModelX(op.x);
  const y = space.toModelY(op.top + op.size * BASELINE_RATIO);

  return pairs([
    [0, "TEXT"],
    ...entityCommon(handles, op),
    [100, "AcDbText"],
    [10, num(x)],
    [20, num(y)],
    [30, num(0)],
    [40, num(op.size / space.unitPtPerMetre)],
    [1, op.text],
    [50, num(0)],
    [41, num(1)],
    [7, "STANDARD"],
    [71, 0],
    [72, 0],
    [11, num(x)],
    [21, num(y)],
    [31, num(0)],
    [100, "AcDbText"],
    [73, 0],
  ]);
}

/**
 * Sheet points to drawing units. The sheet is paper; a DXF is terrain. Both the
 * scale (`unitPtPerMetre`, the same number the Maßstabsbalken draws) and the
 * y-axis convention live here: DXF counts upwards from the bottom, the layout
 * counts downwards from the top.
 */
function modelSpace({ width, height, unitPtPerMetre }) {
  if (!Number.isFinite(unitPtPerMetre) || unitPtPerMetre <= 0) {
    throw new Error("Der Maßstab (unitPtPerMetre) fehlt - ohne ihn wären die DXF-Maße falsch.");
  }
  return {
    unitPtPerMetre,
    toModelX: (x) => x / unitPtPerMetre,
    toModelY: (top) => (height - top) / unitPtPerMetre,
    width: width / unitPtPerMetre,
    height: height / unitPtPerMetre,
  };
}

/**
 * The sheet as one ASCII DXF. Returns a Buffer, because the file is written to
 * disk byte-for-byte and the caller should not have to decide an encoding: DXF
 * text is ANSI, and the drawing's German umlauts are written as UTF-8 for the
 * same reason the engine writes every file as UTF-8.
 */
export function planDxf(ops, { width, height, unitPtPerMetre, comment = [] } = {}) {
  const space = modelSpace({ width, height, unitPtPerMetre });
  const handles = new Handles();

  /* Layers in the order they first appear: derived from the drawing, so a layer
     can never be missing from the table (an entity on an undeclared layer is a
     defect some readers punish by dropping the entity). */
  const layers = [];
  for (const op of ops) {
    const name = op.layer ?? "0";
    if (layers.some((layer) => layer.name === name)) continue;
    layers.push({ name, aci: aciFor(op.stroke ?? op.fill ?? op.color) });
  }

  const content = [];
  for (const line of comment) content.push(...pairs([[999, line]]));

  content.push(
    ...pairs([
      [0, "SECTION"],
      [2, "HEADER"],
      [9, "$ACADVER"],
      [1, "AC1015"],
      [9, "$HANDSEED"],
      [5, "FFFF"],
      [9, "$INSUNITS"],
      [70, 6],
      [9, "$MEASUREMENT"],
      [70, 1],
      [9, "$EXTMIN"],
      [10, num(0)],
      [20, num(0)],
      [30, num(0)],
      [9, "$EXTMAX"],
      [10, num(space.width)],
      [20, num(space.height)],
      [30, num(0)],
      [0, "ENDSEC"],
    ])
  );

  content.push(
    ...pairs([
      [0, "SECTION"],
      [2, "TABLES"],
    ]),
    ...tableHeader(handles, "LTYPE", 2),
    ...linetypeRecords(handles),
    ...TABLE_FOOTER,
    ...tableHeader(handles, "LAYER", layers.length),
    ...layerRecords(handles, layers),
    ...TABLE_FOOTER,
    ...tableHeader(handles, "STYLE", 1),
    ...styleRecords(handles),
    ...TABLE_FOOTER,
    ...tableHeader(handles, "BLOCK_RECORD", 2),
    ...blockRecordRecords(handles),
    ...TABLE_FOOTER,
    ...pairs([[0, "ENDSEC"]])
  );

  content.push(
    ...pairs([
      [0, "SECTION"],
      [2, "BLOCKS"],
    ]),
    ...blockDefinition(handles, "*Model_Space"),
    ...blockDefinition(handles, "*Paper_Space"),
    ...pairs([[0, "ENDSEC"]])
  );

  content.push(
    ...pairs([
      [0, "SECTION"],
      [2, "ENTITIES"],
    ])
  );
  for (const op of ops) {
    if (op.kind === "line") content.push(...lineEntity(handles, op, space));
    else if (op.kind === "rect") content.push(...rectEntity(handles, op, space));
    else if (op.kind === "text") content.push(...textEntity(handles, op, space));
  }
  content.push(...pairs([[0, "ENDSEC"]]));
  content.push(...pairs([[0, "EOF"]]));

  /* CRLF: DXF is a DOS-era format and some readers still expect the pairs on
     Windows line endings; the trailing newline keeps the last pair complete. */
  return Buffer.from(`${content.join("\r\n")}\r\n`, "utf8");
}

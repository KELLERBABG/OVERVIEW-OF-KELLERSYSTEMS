import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runChecks } from "../src/engine.js";
import { computeReportHash } from "../src/dossier.js";
import { calculatePolygonArea, parseDxf } from "../src/cadparser.js";
import { A4 } from "../src/pdf.js";
import { planDxf } from "../src/plandxf.js";
import { planOutputNames, safeLabel } from "../src/planexport.js";
import {
  buildDossierPdf,
  buildPlanSheet,
  planDocument,
  renderPlanDxf,
  renderPlanSvg,
  writePlanDxf,
  writePlanSvg,
} from "../src/dossierpdf.js";

/* The Lageplan is exported three times - as a page of the dossier, as SVG and as
   DXF - and the promise is that all three are the *same drawing*. These tests
   attack that promise from the outside: they parse the exported files the way a
   reader would (a browser for the SVG, the project's own CAD importer for the
   DXF) and compare what comes back with the recorded primitives the PDF page is
   drawn from.

   A formatting test on its own would not catch the failure that matters - a DXF
   that has the right sections but the wrong metres - so the numbers are checked
   against the geometry, not against golden strings. */

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

function planInputs(name, extra = {}) {
  const spec = fixture(name);
  const runResult = runChecks(spec);
  const projectData = {
    projekt: spec.projekt,
    aktenzeichen: "BV-2026-000123",
    flurstueck: "Flur 4, Flurstück 108/2",
    gemarkung: "Musterhausen",
    entwurfsverfasser: "Architekturbüro Dipl.-Ing. Muster",
    bauherr: "Projektentwicklung GmbH & Co. KG",
    ...spec,
    ...extra,
  };
  return { spec, runResult, projectData, hash: computeReportHash(projectData, runResult.verdict) };
}

const options = { datum: "02.10.2026" };

/* ---------------------------------------------------------------------------
 * SVG
 * ------------------------------------------------------------------------ */

test("plan svg: a standalone DIN-A4-quer sheet whose elements are the recorded primitives", () => {
  const { runResult, projectData } = planInputs("good_spec.json");
  const sheet = buildPlanSheet(projectData, runResult, options);
  const svg = renderPlanSvg(projectData, runResult, options);

  assert.ok(svg.startsWith("<?xml version=\"1.0\" encoding=\"UTF-8\""), "the file declares its encoding");
  assert.match(
    svg,
    /<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" version="1\.1" width="841\.89pt" height="595\.28pt" viewBox="0 0 841\.89 595\.28">/,
    "the paper size is stated in points, so the file prints 1:1 without arithmetic"
  );
  assert.ok(svg.includes("<title>"), "the Blatt names itself in its metadata");
  assert.ok(!/NaN|undefined|Infinity/.test(svg), "no unresolved numbers reach the file");

  /* Ein Element je Aufzeichnung: das ist die Zusage, dass SVG und PDF dieselbe
     Zeichnung sind, und nicht zwei Zeichnungen, die sich ähnlich sehen. */
  const counts = {
    line: (svg.match(/<line /g) ?? []).length,
    rect: (svg.match(/<rect /g) ?? []).length,
    text: (svg.match(/<text /g) ?? []).length,
  };
  const recorded = { line: 0, rect: 0, text: 0 };
  for (const op of sheet.ops) recorded[op.kind] += 1;

  assert.equal(counts.line, recorded.line, "every recorded line is a <line>");
  assert.equal(counts.text, recorded.text, "every recorded text is a <text>");
  /* Ein Rechteck mehr: die weiße Blattfläche. Ein Lageplan auf durchsichtigem
     Grund ist auf einem Plotter nicht lesbar. */
  assert.equal(counts.rect, recorded.rect + 1, "the sheet adds exactly one paper rectangle");

  /* Jede Ebene bekommt eine Gruppe, und zwar in der Reihenfolge ihres ersten
     Auftretens: die Stapelreihenfolge ist Teil der Zeichnung. */
  const layers = [...new Set(sheet.ops.map((op) => op.layer))];
  for (const layer of layers) {
    assert.ok(
      svg.includes(`<g id="layer-${layer}" data-layer="${layer}">`),
      `missing layer group ${layer}`
    );
  }
  assert.equal((svg.match(/<g id="layer-/g) ?? []).length, layers.length, "one id per layer, runs merged only in order");
  assert.equal(layers.length, 11, "the sheet's eleven layers");
  assert.ok(layers.includes("GRENZE") && layers.includes("ABSTANDSFLAECHE") && layers.includes("SCHRIFTFELD"));

  /* Die Geometrie steht unverändert im Blatt: dieselben Punkte, dieselben
     Strichstärken. Gemessen wird gegen die Aufzeichnung, nicht gegen ein
     Abbild. */
  const firstLine = sheet.ops.find((op) => op.kind === "line");
  const firstLineElement = svg.match(/<line [^>]*\/>/)[0];
  for (const value of [firstLine.x1, firstLine.y1, firstLine.x2, firstLine.y2]) {
    assert.ok(firstLineElement.includes(`="${Number(value.toFixed(3))}"`), `line keeps its coordinate ${value}`);
  }

  /* Gestrichelte Linien bleiben gestrichelt, und Text behält seine Punktgröße -
     sonst wäre die Maßzahl auf Papier nicht die, die das Blatt nennt. */
  assert.ok(svg.includes('stroke-dasharray="3.2 2.6"'), "the Grenzabstand lines stay dashed");
  const firstText = sheet.ops.find((op) => op.kind === "text");
  assert.ok(svg.includes(`font-size="${firstText.size}"`), "text keeps its point size");
  assert.ok(svg.includes("&amp;") || !svg.includes("&"), "XML entities are escaped, never raw");

  /* Determinismus: dasselbe Projekt ergibt dieselben Bytes. */
  assert.equal(renderPlanSvg(projectData, runResult, options), svg, "the SVG is byte-reproducible");
});

test("plan svg: an ampersand in a project name cannot break the document", () => {
  const { runResult, projectData } = planInputs("good_spec.json", {
    projekt: "Wohnanlage Müller & Söhne <Bauabschnitt 2>",
  });
  const svg = renderPlanSvg(projectData, runResult, options);

  assert.ok(svg.includes("Wohnanlage Müller &amp; Söhne &lt;Bauabschnitt 2&gt;"), "the name is escaped");
  assert.ok(!/<Bauabschnitt/.test(svg), "no raw markup from data");
  /* Ein unbalanciertes Tag wäre der Fehler, den ein Betrachter nicht sieht und
     ein Plotter nicht meldet. */
  const open = (svg.match(/<g[ >]/g) ?? []).length;
  const close = (svg.match(/<\/g>/g) ?? []).length;
  assert.equal(open, close, "every group is closed");
  assert.ok(svg.endsWith("</svg>\n"), "the document is complete");
});

/* ---------------------------------------------------------------------------
 * DXF
 * ------------------------------------------------------------------------ */

test("plan dxf: a conservative AC1015 file in metres that the project's own CAD import reads back", () => {
  const { runResult, projectData } = planInputs("good_spec.json");
  const sheet = buildPlanSheet(projectData, runResult, options);
  const dxf = renderPlanDxf(projectData, runResult, options).toString("utf8");

  /* Aufbau: die vier Abschnitte, die ein Leser braucht, plus EOF. */
  for (const phrase of ["SECTION", "HEADER", "TABLES", "BLOCKS", "ENTITIES", "EOF"]) {
    assert.ok(dxf.includes(phrase), `missing DXF ${phrase}`);
  }
  assert.ok(dxf.startsWith("999\r\n"), "the file starts with its provenance comment");
  assert.ok(dxf.includes("$ACADVER\r\n1\r\nAC1015"), "the dialect is the one that introduced LWPOLYLINE");
  assert.ok(dxf.includes("$INSUNITS\r\n70\r\n6"), "$INSUNITS 6 is metres");
  assert.ok(dxf.includes("1 Einheit = 1 m"), "and the comment says so in words");

  /* Kein Paar darf einen leeren Wert haben: ein Leser, der Leerzeilen verwirft -
     der eigene Import tut das - würde sonst jeden folgenden Wert dem falschen
     Gruppencode zuordnen. */
  const lines = dxf.split("\r\n");
  assert.equal(lines[lines.length - 1], "", "the file ends with a line break");
  for (const [index, line] of lines.entries()) {
    if (index % 2 === 0 || line === "") continue;
    assert.notEqual(line.trim(), "", `value for the code in line ${index} is empty`);
  }

  /* Die Grenzen des Blattes in Metern: 841,89 pt bei 1:250 sind 74,25 m quer
     und 52,50 m hoch. */
  const extmax = /\$EXTMAX\r\n10\r\n([-\d.]+)\r\n20\r\n([-\d.]+)\r\n/.exec(dxf);
  assert.ok(extmax, "EXTMAX is written");
  assert.ok(Math.abs(Number(extmax[1]) - A4.height / sheet.geometry.unit) < 1e-4, "EXTMAX x is the sheet in metres");
  assert.ok(Math.abs(Number(extmax[2]) - A4.width / sheet.geometry.unit) < 1e-4, "EXTMAX y is the sheet in metres");

  /* Jede Entität liegt auf einer deklarierten Ebene - eine Entität auf einer
     unbekannten Ebene ist der Fehler, den manche Leser mit Verwerfen bestrafen. */
  const declared = new Set(
    [...dxf.matchAll(/\r\nLAYER\r\n[\s\S]*?\r\n2\r\n([A-Z0-9_]+)\r\n/g)].map((match) => match[1])
  );
  const used = new Set(sheet.ops.map((op) => op.layer));
  for (const layer of used) assert.ok(declared.has(layer), `layer ${layer} is used but not declared`);

  const counts = {
    LINE: (dxf.match(/\r\nLINE\r\n/g) ?? []).length,
    LWPOLYLINE: (dxf.match(/\r\nLWPOLYLINE\r\n/g) ?? []).length,
    TEXT: (dxf.match(/\r\nTEXT\r\n/g) ?? []).length,
  };
  const recorded = { LINE: 0, LWPOLYLINE: 0, TEXT: 0 };
  for (const op of sheet.ops) {
    if (op.kind === "line") recorded.LINE += 1;
    else if (op.kind === "rect") recorded.LWPOLYLINE += 1;
    else if (op.kind === "text") recorded.TEXT += 1;
  }
  assert.deepEqual(counts, recorded, "one entity per recorded primitive");

  /* Jedes Rechteck ist ein geschlossenes Vieleck aus vier Ecken, nicht vier
     einzelne Linien: nur so kann ein Leser Grundstücksgrenze und Hauskörper
     messen und füllen - der eigene Import liest sie als Polygon zurück. */
  const polygons = [...dxf.matchAll(/\r\nLWPOLYLINE\r\n([\s\S]*?)(?=\r\n0\r\n)/g)].map((match) => match[1]);
  assert.equal(polygons.length, recorded.LWPOLYLINE, "one LWPOLYLINE per rectangle");
  for (const body of polygons) {
    assert.match(body, /\r\n90\r\n4\r\n/, "four corners");
    assert.match(body, /\r\n70\r\n1\r\n/, "closed");
    assert.equal((body.match(/\r\n10\r\n/g) ?? []).length, 4, "four x coordinates");
    assert.equal((body.match(/\r\n20\r\n/g) ?? []).length, 4, "four y coordinates");
  }

  /* Der entscheidende Test: der eigene CAD-Import liest die Datei und findet in
     ihr die Flächen, die das Blatt zeichnet - das Grundstücksrechteck und den
     Hauskörper in echten Metern. */
  const parsed = parseDxf(dxf);
  assert.equal(parsed.detectedEntities.plotFound, true, "the import finds the parcel");
  assert.equal(parsed.detectedEntities.buildingFound, true, "the import finds the building");
  assert.equal(parsed.footprintAreaM2, Math.round(sheet.geometry.footprintWidthM * sheet.geometry.footprintDepthM));
  assert.equal(parsed.footprintAreaM2, 280, "GRZ 0,35 x 800 m² is 280 m² on the drawing too");
  assert.equal(parsed.plotAreaM2, Math.round(sheet.geometry.widthM * sheet.geometry.depthM));
  assert.equal(parsed.plotAreaM2, 642, "the drawn parcel is Hauskörper plus the four checked setbacks");

  /* Die Maßzahlen und das Schriftfeld reisen als Text mit, nicht als Linien: wer
     die Zeichnung bemaßen will, soll die Zahlen lesen können. */
  for (const mass of ["5,00", "4,00", "19,32", "14,49", "27,32", "23,49"]) {
    assert.ok(dxf.includes(`\r\n1\r\n${mass}\r\n`), `missing TEXT für die Maßzahl ${mass}`);
  }
  assert.ok(dxf.includes("\r\n1\r\nSchriftfeld\r\n"), "the Schriftfeld is text, not outlines");

  /* Deterministisch, und ein fehlender Maßstab ist ein Fehler statt eines
     stillschweigend falschen Maßes. */
  assert.equal(renderPlanDxf(projectData, runResult, options).toString("utf8"), dxf, "the DXF is byte-reproducible");
});

test("plan dxf: without a scale the writer refuses instead of emitting metres that are wrong", () => {
  /* Die Punkte des Blattes sind Papier; ohne Maßstab wäre jede Koordinate um den
     Faktor 1:250 falsch - und eine Zeichnung mit falschen Maßen fällt erst beim
     Bauen auf. Der Aufruf muss scheitern, nicht raten. */
  assert.throws(
    () => planDxf([{ kind: "line", layer: "GRENZE", x1: 0, y1: 0, x2: 10, y2: 0, color: "#0369a1", width: 1, dash: null }], {
      width: A4.height,
      height: A4.width,
    }),
    /Maßstab/
  );
});

test("plan dxf: the failing fixture keeps its own drawing and its own scale", () => {
  const { runResult, projectData } = planInputs("bad_spec.json");
  const sheet = buildPlanSheet(projectData, runResult, options);
  const parsed = parseDxf(renderPlanDxf(projectData, runResult, options).toString("utf8"));

  /* bad_spec: GRZ 0,52 × 600 m² = 312 m², Abstände Nord 3,0 und Süd 2,5 m. */
  assert.equal(parsed.footprintAreaM2, 312);
  assert.equal(parsed.plotAreaM2, Math.round(sheet.geometry.widthM * sheet.geometry.depthM));
  assert.equal(parsed.detectedEntities.plotFound, true);
  assert.equal(parsed.detectedEntities.buildingFound, true);
  assert.equal(sheet.geometry.label, "1:250");
});

/* ---------------------------------------------------------------------------
 * Zusammen mit dem PDF
 * ------------------------------------------------------------------------ */

test("plan export: the SVG and the DXF carry the drawing of the PDF page, to the primitive", () => {
  const { runResult, projectData, hash } = planInputs("good_spec.json");
  const sheet = buildPlanSheet(projectData, runResult, options);
  const built = buildDossierPdf(projectData, runResult, options);
  const quer = built.audit.pages.filter((page) => page.width > page.height);

  assert.equal(quer.length, 1, "the dossier has exactly one quer sheet");
  /* Die PDF-Seite ist genau diese Aufzeichnung, abgespielt - plus die drei
     Operationen der Fußzeile (Linie und zwei Texte), die erst am Ende gestempelt
     wird, wenn die Seitenzahl bekannt ist. Ein Rechteck mit Füllung und Rahmen
     sind im Inhalt zwei Operationen, in der Aufzeichnung eine primitive; deshalb
     wird hier gezählt, was der Schreiber wirklich zeichnet. Stimmt diese Zahl,
     dann ist das Querblatt kein zweites Bild der Zeichnung, sondern dieselbe. */
  const drawnOps = sheet.ops.reduce(
    (sum, op) => sum + (op.kind === "rect" && op.fill && op.stroke ? 2 : 1),
    0
  );
  assert.equal(quer[0].ops, drawnOps + 3, "the landscape page is the recorded sheet plus its footer");

  const document = planDocument(projectData, runResult, options, "svg");
  assert.equal(document.geometry.label, "1:250");
  assert.equal(document.model.hash, hash, "the Blatt carries the Prüfziffer of the check");
  assert.equal(document.ops.length, sheet.ops.length, "one recording, both formats");
  assert.equal(planDocument(projectData, runResult, options, "dxf").geometry.denominator, 250);
});

test("plan export: writes files and refuses to invent a directory", async (t) => {
  const dir = mkdtempSync(join(tmpdir(), "baulens-plan-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const { runResult, projectData, hash } = planInputs("good_spec.json");

  const svg = writePlanSvg(join(dir, "Lageplan.svg"), projectData, runResult, options);
  const dxf = writePlanDxf(join(dir, "Lageplan.dxf"), projectData, runResult, options);

  assert.equal(svg.path, join(dir, "Lageplan.svg"));
  assert.equal(dxf.path, join(dir, "Lageplan.dxf"));
  assert.equal(svg.bytes, readFileSync(svg.path).length, "the reported byte count is the file's");
  assert.equal(dxf.bytes, readFileSync(dxf.path).length);
  assert.equal(svg.hash, hash, "the seal travels with the export");
  assert.equal(dxf.hash, hash);
  assert.equal(svg.scale, "1:250");
  assert.ok(readFileSync(svg.path, "utf8").startsWith("<?xml"), "the SVG on disk is the document");
  assert.ok(readFileSync(dxf.path, "utf8").startsWith("999"), "the DXF on disk is the document");

  for (const write of [writePlanSvg, writePlanDxf]) {
    assert.throws(() => write("   ", projectData, runResult, options), (error) => error.code === "missing_path");
    assert.throws(
      () => write(join(dir, "gibt-es-nicht", "x"), projectData, runResult, options),
      (error) => error.code === "missing_directory"
    );
    assert.equal(existsSync(join(dir, "gibt-es-nicht")), false, "the directory is not created on the way");
  }

  /* Ein Pfad, der nicht zu schreiben ist, bleibt ein Fehler mit eigenem Code -
     derselbe Vertrag wie beim PDF, damit das Shell den Fall benennen kann. */
  writeFileSync(join(dir, "blocked"), "");
  assert.throws(
    () => writePlanSvg(join(dir, "blocked", "x.svg"), projectData, runResult, options),
    (error) => error.code === "svg_write_failed"
  );
  assert.throws(
    () => writePlanDxf(join(dir, "blocked", "x.dxf"), projectData, runResult, options),
    (error) => error.code === "dxf_write_failed"
  );
});

test("plan export: the areas in the DXF are the drawn geometry, not the declared parcel", () => {
  const { runResult, projectData } = planInputs("good_spec.json");
  const sheet = buildPlanSheet(projectData, runResult, options);
  const parsed = parseDxf(renderPlanDxf(projectData, runResult, options).toString("utf8"));

  /* Was die Zeichnung zeigt, ist das Bezugsrechteck - Hauskörper plus geprüfte
     Abstände -, nicht das 800 m² große Grundstück aus der Planung. Wer die DXF
     wieder einliest, erhält die gezeichneten Maße; die Prüfziffer des Befunds
     bleibt das Siegel über die geprüften. */
  assert.notEqual(parsed.plotAreaM2, 800);
  assert.ok(parsed.plotAreaM2 > parsed.footprintAreaM2, "the parcel still encloses the building");
  assert.equal(parsed.footprintAreaM2, Math.round(calculatePolygonArea([
    { x: 0, y: 0 },
    { x: sheet.geometry.footprintWidthM, y: 0 },
    { x: sheet.geometry.footprintWidthM, y: sheet.geometry.footprintDepthM },
    { x: 0, y: sheet.geometry.footprintDepthM },
  ])));
});

/* ---------------------------------------------------------------------------
 * Naming a whole workspace export
 * ------------------------------------------------------------------------ */

test("plan export: a Blatt is named after its Vorhaben and can never leave the target folder", () => {
  /* Der Name kommt aus einem Verzeichnisnamen auf irgendeiner Festplatte. Ein
     Trennzeichen darin würde woanders schreiben, und `..` würde aus dem Ordner
     klettern, den jemand im Dialog ausgewählt hat. */
  assert.equal(safeLabel("Wohnanlage Musterweg"), "Wohnanlage-Musterweg");
  assert.equal(safeLabel("../../etc/passwd"), "etc-passwd");
  assert.equal(safeLabel("C:\\Projekte\\Haus 3"), "C-Projekte-Haus-3");
  assert.equal(safeLabel("Haus/3/OG"), "Haus-3-OG");
  assert.equal(safeLabel("Bau:vorhaben"), "Bau-vorhaben");
  assert.equal(safeLabel("Haus*?<>|3"), "Haus-3");
  assert.equal(safeLabel(".versteckt"), "versteckt", "a leading dot would hide the Blatt");
  assert.equal(safeLabel("Haus . "), "Haus", "Windows drops trailing dots and spaces, so two names must not collapse");
  assert.equal(safeLabel(""), "Lageplan");
  assert.equal(safeLabel("   "), "Lageplan");
  assert.equal(safeLabel(null), "Lageplan");

  for (const label of ["../../etc/passwd", "C:\\Windows\\System32", "..", "/", "a/b", ".\\..\\..\\secret"]) {
    const { names } = planOutputNames([{ source: "x.dxf", name: label }], ["dxf"]);
    assert.ok(!/[\/]/.test(names[0].files.dxf), `separator survived in ${names[0].files.dxf}`);
    assert.ok(!names[0].files.dxf.includes(".."), `traversal survived in ${names[0].files.dxf}`);
  }
});

test("plan export: two Vorhaben never share a file, and the rename is reported", () => {
  const entries = [
    { source: "a/haus.dxf", name: "Wohnanlage Musterweg" },
    { source: "b/haus.dxf" },
    { source: "c/haus.dxf", name: "wohnanlage musterweg" },
  ];
  const { names, collisions } = planOutputNames(entries, ["dxf", "svg"]);

  assert.deepEqual(
    names.map((entry) => entry.label),
    ["Wohnanlage-Musterweg", "haus", "wohnanlage-musterweg-2"],
    "the third label collides case-insensitively and is suffixed in input order"
  );
  /* Ein Suffix gilt für alle Formate: sonst gehörten DXF und SVG eines Vorhabens
     zu verschiedenen Namen. */
  assert.equal(names[2].files.dxf, "wohnanlage-musterweg-2_Lageplan.dxf");
  assert.equal(names[2].files.svg, "wohnanlage-musterweg-2_Lageplan.svg");
  assert.equal(collisions.length, 1, "only the entry that had to change is reported");
  assert.equal(collisions[0].source, "c/haus.dxf");
  assert.equal(collisions[0].renamed, "wohnanlage-musterweg-2");
  assert.deepEqual(
    names.map((entry) => entry.files.dxf),
    ["Wohnanlage-Musterweg_Lageplan.dxf", "haus_Lageplan.dxf", "wohnanlage-musterweg-2_Lageplan.dxf"],
    "names are stable and in input order"
  );

  /* Ohne Vorhabennamen ist der Plandateiname das Etikett - und ohne Endung. */
  const plain = planOutputNames([{ source: "/tmp/BV/haus.dxf" }, { source: "/tmp/BV/grundstueck.ifc" }]);
  assert.deepEqual(
    plain.names.map((entry) => entry.files.svg),
    ["haus_Lageplan.svg", "grundstueck_Lageplan.svg"]
  );

  /* Nur die verlangten Formate, deterministisch. */
  const only = planOutputNames([{ source: "haus.dxf" }], ["svg"]);
  assert.deepEqual(Object.keys(only.names[0].files), ["svg"]);
  assert.deepEqual(planOutputNames(entries, ["dxf"]).names, planOutputNames(entries, ["dxf"]).names, "same input, same names");
});

test("plan export: a label is truncated to a length a filesystem accepts", () => {
  const long = "W".repeat(300);
  const { names } = planOutputNames([{ source: "haus.dxf", name: long }], ["dxf"]);
  assert.ok(names[0].label.length <= 80, `label is ${names[0].label.length} characters`);
  assert.ok(names[0].files.dxf.length < 100, "the file name plus suffix stays far below the limit");
});

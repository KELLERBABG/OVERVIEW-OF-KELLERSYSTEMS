import { test, beforeEach, afterEach } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { validateSpec } from "../src/schema.js";
import { runChecks } from "../src/engine.js";
import { handleRequest } from "../src/ipc.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const bin = join(root, "bin", "baucheck.js");
const goodFixture = join(root, "test", "fixtures", "good_spec.json");
const badFixture = join(root, "test", "fixtures", "bad_spec.json");

let tempDir;
beforeEach(() => {
  tempDir = mkdtempSync(join(tmpdir(), "baucheck-cli-"));
});
afterEach(() => {
  rmSync(tempDir, { recursive: true, force: true });
});

function cli(args) {
  try {
    const stdout = execFileSync(process.execPath, [bin, ...args], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { status: 0, stdout };
  } catch (err) {
    return {
      status: err.status ?? -1,
      stdout: String(err.stdout ?? ""),
      stderr: String(err.stderr ?? ""),
    };
  }
}

test("check good_spec -> Exit 0 und Urteil BESTANDEN", () => {
  const res = cli(["check", goodFixture]);
  assert.equal(res.status, 0);
  assert.ok(res.stdout.includes("BESTANDEN"));
});

test("check bad_spec -> Exit 1 und Urteil NICHT BESTANDEN", () => {
  const res = cli(["check", badFixture]);
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("NICHT BESTANDEN"));
});

test("--example gibt gueltiges JSON aus, das alle Pruefungen besteht", () => {
  const res = cli(["--example"]);
  assert.equal(res.status, 0);
  const spec = JSON.parse(res.stdout);
  assert.equal(validateSpec(spec).ok, true);
  assert.equal(runChecks(spec).verdict, "bestanden");
});

test("--example-bad gibt JSON mit mindestens 4 Regelverstoessen aus", () => {
  const res = cli(["--example-bad"]);
  assert.equal(res.status, 0);
  const spec = JSON.parse(res.stdout);
  assert.equal(validateSpec(spec).ok, true);
  const run = runChecks(spec);
  assert.equal(run.verdict, "nicht bestanden");
  assert.ok(run.counts.fail >= 4);
});

test("Fehlende Datei -> Exit 2", () => {
  const res = cli(["check", join(tempDir, "gibts-nicht.json")]);
  assert.equal(res.status, 2);
});

test("Ungueltiges JSON -> Exit 2", () => {
  const file = join(tempDir, "broken.json");
  writeFileSync(file, "{ kein json", "utf8");
  const res = cli(["check", file]);
  assert.equal(res.status, 2);
  assert.ok(res.stderr.includes("JSON"));
});

test("Schema-Verstoesspec (hoehe_m als Text) -> Exit 2", () => {
  const file = join(tempDir, "wrongtype.json");
  writeFileSync(file, JSON.stringify({ gebaeude: { hoehe_m: "hoch" } }), "utf8");
  const res = cli(["check", file]);
  assert.equal(res.status, 2);
  assert.ok(res.stderr.includes("hoehe_m"));
});

test("Keine Argumente -> Exit 2; --help -> Exit 0", () => {
  assert.equal(cli([]).status, 2);
  const help = cli(["--help"]);
  assert.equal(help.status, 0);
  assert.ok(help.stdout.includes("--rules"));
});

test("Unbekannte Option und unbekannte Regel-ID -> jeweils Exit 2", () => {
  assert.equal(cli(["check", goodFixture, "--unsinn"]).status, 2);
  const res = cli(["check", goodFixture, "--rules", "tageslicht,bogus"]);
  assert.equal(res.status, 2);
  assert.ok(res.stderr.includes("bogus"));
});

test("--rules-Filter im CLI laeuft nur die gewaehlte Regel", () => {
  const res = cli(["check", badFixture, "--rules", "raumhoehen"]);
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("raumhoehen"));
  assert.ok(!res.stdout.includes("abstandsflaeche"));
});

test("--report schreibt Markdown-Bericht mit §-Zitaten", () => {
  const out = join(tempDir, "bericht.md");
  const res = cli(["check", badFixture, "--report", out]);
  assert.equal(res.status, 1);
  const md = readFileSync(out, "utf8");
  assert.ok(md.includes("§6 Abs. 2 MBO"));
  assert.ok(md.includes("nicht bestanden"));
});

test("--pdf writes the same document the desktop path does, building data included", () => {
  /* The CLI and the sidecar build their own projectData, and both now hand the
     building data to the renderer because the Lageplan derives the Hauskörper
     from it. They must still seal and write one document: two exports of one
     plan that differ would be worse than no export at all. */
  const out = join(tempDir, "Befund.pdf");
  const res = cli(["check", goodFixture, "--pdf", out, "--datum", "02.10.2026"]);
  assert.equal(res.status, 0);

  const printed = /Prüfziffer ([0-9a-f]{64})/.exec(res.stdout);
  assert.ok(printed, `the CLI reports the Prüfziffer: ${res.stdout}`);

  const desktop = handleRequest({
    id: 40,
    method: "dossier_pdf",
    params: { file: goodFixture, datum: "02.10.2026" },
  });
  assert.equal(desktop.ok, true, desktop.error && desktop.error.message);
  assert.equal(desktop.result.hash, printed[1], "one plan, one Prüfziffer on both paths");
  assert.ok(
    readFileSync(out).equals(Buffer.from(desktop.result.base64, "base64")),
    "and one file: the CLI and the desktop must not produce different dossiers"
  );

  /* Und die Daten sind auch wirklich im Blatt: der Hauskörper aus GRZ 0,35 ×
     800 m² steht als Maßkette im PDF. */
  const bytes = readFileSync(out, "latin1");
  assert.ok(bytes.includes("(19,32)"), "the drawn Hauskörper comes from the declared GRZ");
  assert.ok(bytes.includes("GRZ 0,35"), "and the sheet names the derivation");
});

test("--svg and --dxf write the Lageplan sheet beside the dossier", () => {
  /* The Blatt leaves the dossier for the CAD side of a submission: an SVG for
     the plotter, a DXF in metres for the plan set. Both must be the sheet of the
     same check, so the CLI has to hand them the same projectData and the same
     run it printed the summary from. */
  const svgPath = join(tempDir, "Lageplan.svg");
  const dxfPath = join(tempDir, "Lageplan.dxf");
  const res = cli(["check", goodFixture, "--svg", svgPath, "--dxf", dxfPath, "--datum", "02.10.2026"]);
  assert.equal(res.status, 0);

  const svg = readFileSync(svgPath, "utf8");
  const dxf = readFileSync(dxfPath, "utf8");
  assert.match(svg, /^<\?xml version="1\.0" encoding="UTF-8"/);
  assert.match(svg, /viewBox="0 0 841\.89 595\.28"/);
  assert.match(dxf, /^999\r\n/);
  assert.ok(dxf.includes("$ACADVER\r\n1\r\nAC1015"));
  assert.ok(dxf.includes("\r\n1\r\n19,32\r\n"), "the drawn Hauskörper is in the export");

  assert.ok(res.stdout.includes(`Lageplan als SVG geschrieben: ${svgPath}`), res.stdout);
  assert.ok(res.stdout.includes("Maßstab 1:250"), "the report names the scale the Blatt is drawn in");

  /* Unter dem Strich dieselbe Zeichnung wie im PDF des Sidecars. */
  const desktop = handleRequest({ id: 41, method: "plan_dxf", params: { file: goodFixture, datum: "02.10.2026" } });
  assert.equal(desktop.ok, true, desktop.error && desktop.error.message);
  assert.equal(dxf, desktop.result.dxf, "one plan, one drawing on both paths");

  const bad = cli(["check", goodFixture, "--svg"]);
  assert.equal(bad.status, 2);
  assert.ok(bad.stderr.includes("--svg benötigt einen Zieldateipfad"));
});

test("--deviations writes official Abweichungs-/Befreiungsantrag for non-compliant plan", () => {
  const devPath = join(tempDir, "Abweichung.txt");
  const res = cli(["check", badFixture, "--land", "BY", "--stadt", "München", "--deviations", devPath]);
  assert.equal(res.status, 1);
  const text = readFileSync(devPath, "utf8");
  assert.match(text, /ANTRAG AUF ABWEICHUNG \/ BEFREIUNG/);
  assert.match(text, /München/);
  assert.ok(res.stdout.includes("Abweichungsanträge geschrieben"));
});

test("--batch scans a project directory and exports Lagepläne and PDF dossiers", () => {
  const projDir = join(tempDir, "workspace");
  const outPlans = join(tempDir, "plans");
  const outPdfs = join(tempDir, "pdfs");
  execFileSync(process.execPath, ["-e", `
    const fs = require("fs"), path = require("path");
    fs.mkdirSync(${JSON.stringify(projDir)}, { recursive: true });
    fs.mkdirSync(${JSON.stringify(outPlans)}, { recursive: true });
    fs.mkdirSync(${JSON.stringify(outPdfs)}, { recursive: true });
    fs.copyFileSync(${JSON.stringify(goodFixture)}, path.join(${JSON.stringify(projDir)}, "projekt_a.json"));
    fs.copyFileSync(${JSON.stringify(badFixture)}, path.join(${JSON.stringify(projDir)}, "projekt_b.json"));
  `]);

  const res = cli([
    "check",
    "--batch",
    projDir,
    "--land",
    "BY",
    "--export-all-plans",
    outPlans,
    "--export-all-pdf",
    outPdfs,
  ]);
  /* One passed, one failed -> exit 1 */
  assert.equal(res.status, 1);
  assert.match(res.stdout, /Stapelprüfung/);
  assert.match(res.stdout, /Stapel-Bilanz: 2 Dateien \| 1 bestanden\/Auflagen \| 1 nicht bestanden/);
  assert.match(res.stdout, /Lageplan-Stapel geschrieben/);
  assert.match(res.stdout, /-> PDF:/);
});


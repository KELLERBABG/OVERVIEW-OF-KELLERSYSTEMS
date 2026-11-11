import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { handleLine, handleRequest, METHODS } from "../src/ipc.js";
import { inspectXref } from "../src/pdf.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

/* This protocol is the desktop app's only way to reach the engine, so its shape
   is a contract: the Rust shell parses these fields by name. */

test("ipc: a bad request never throws, it returns an error object", () => {
  for (const input of [null, undefined, 42, "string", []]) {
    const response = handleRequest(input);
    assert.equal(response.ok, false);
    assert.equal(typeof response.error.message, "string");
    assert.ok(response.error.code);
  }
});

test("ipc: malformed JSON is reported as a parse error, not a crash", () => {
  const response = handleLine("{not json");
  assert.equal(response.ok, false);
  assert.equal(response.error.code, "parse_error");
  assert.equal(response.id, null);
});

test("ipc: the id is echoed back so responses can be matched to requests", () => {
  assert.equal(handleLine('{"id":7,"method":"ping"}').id, 7);
  assert.equal(handleLine('{"id":"abc","method":"ping"}').id, "abc");
  assert.equal(handleLine('{"method":"ping"}').id, null);
});

test("ipc: ping and describe answer without a spec", () => {
  const ping = handleLine('{"id":1,"method":"ping"}');
  assert.equal(ping.ok, true);
  assert.equal(ping.result.pong, true);

  const describe = handleLine('{"id":2,"method":"describe"}');
  assert.equal(describe.ok, true);
  /* All 15 modules, not the 9 core ones. */
  assert.equal(describe.result.rules.length, 15);
  assert.equal(describe.result.counts.core, 9);
  assert.equal(describe.result.counts.extra, 6);
  for (const rule of describe.result.rules) {
    assert.ok(rule.id && rule.title && rule.para, "each rule carries id, title and citation");
  }
  assert.deepEqual(describe.result.methods, METHODS);
});

test("ipc: an unknown method names itself and lists the valid ones", () => {
  const response = handleLine('{"id":3,"method":"explode"}');
  assert.equal(response.ok, false);
  assert.equal(response.error.code, "unknown_method");
  assert.deepEqual(response.error.details, METHODS);
});

test("ipc: check returns the verdict, counts and a flat finding list", () => {
  const spec = fixture("good_spec.json");
  const response = handleRequest({ id: 4, method: "check", params: { spec, land: "BY" } });

  assert.equal(response.ok, true);
  const result = response.result;

  /* The verdict itself is not asserted: it legitimately changes when rules are
     added. What must hold is that it is one of the three known values and that
     it agrees with the findings that accompany it. */
  assert.ok(["bestanden", "mit Auflagen", "nicht bestanden"].includes(result.verdict));
  assert.equal(result.profile.code, "BY");
  assert.ok(result.counts.total > 0);

  const failing = result.findings.filter((f) => f.status === "fail");
  const warning = result.findings.filter((f) => f.status === "warn");
  const expected = failing.length ? "nicht bestanden" : warning.length ? "mit Auflagen" : "bestanden";
  assert.equal(result.verdict, expected, "the verdict must follow from the findings");

  /* The flat list is what the UI renders, so every entry needs these fields. */
  assert.ok(Array.isArray(result.findings) && result.findings.length > 0);
  for (const finding of result.findings) {
    assert.ok(typeof finding.ruleId === "string" && finding.ruleId);
    assert.ok(["ok", "warn", "fail"].includes(finding.status));
    assert.ok(typeof finding.message === "string" && finding.message.length > 0);
  }
});

test("ipc: a violating spec reports the failure and the failing rule", () => {
  const spec = fixture("bad_spec.json");
  const response = handleRequest({ id: 5, method: "check", params: { spec } });
  assert.equal(response.ok, true);
  assert.notEqual(response.result.verdict, "bestanden");
  assert.ok(response.result.findings.some((f) => f.status === "fail"));
});

test("ipc: the heavy artefacts are opt-in", () => {
  const spec = fixture("good_spec.json");
  const plain = handleRequest({ id: 6, method: "check", params: { spec } }).result;

  /* A verdict alone must not carry a markdown report or a repair set. */
  assert.equal(plain.report, undefined);
  assert.equal(plain.repairs, undefined);
  assert.equal(plain.bcf, undefined);
  assert.equal(plain.xbau, undefined);

  const rich = handleRequest({
    id: 7,
    method: "check",
    params: { spec, report: true, repair: true, bcf: true, xbau: true },
  }).result;

  assert.equal(typeof rich.report, "string");
  assert.ok(rich.report.includes("Vorprüfungsbericht"));
  assert.ok(Array.isArray(rich.repairs));
  assert.ok(rich.bcf !== undefined);
  assert.ok(rich.xbau !== undefined);
});

test("ipc: rule selection narrows the findings and rejects unknown ids", () => {
  const spec = fixture("bad_spec.json");

  const all = handleRequest({ id: 8, method: "check", params: { spec } }).result;
  const subset = handleRequest({ id: 9, method: "check", params: { spec, rules: ["grz"] } }).result;
  assert.ok(subset.findings.length < all.findings.length);
  assert.deepEqual([...new Set(subset.findings.map((f) => f.ruleId))], ["grz"]);

  const bad = handleRequest({ id: 10, method: "check", params: { spec, rules: ["nope"] } });
  assert.equal(bad.ok, false);
  assert.equal(bad.error.code, "unknown_rule");
});

test("ipc: a missing or invalid spec is refused with a useful code", () => {
  const missing = handleRequest({ id: 11, method: "check", params: {} });
  assert.equal(missing.ok, false);
  assert.equal(missing.error.code, "missing_spec");

  const invalid = handleRequest({
    id: 12,
    method: "check",
    params: { spec: { gebaeude: { hoehe_m: "acht" } } },
  });
  assert.equal(invalid.ok, false);
  assert.equal(invalid.error.code, "invalid_spec");
  assert.ok(Array.isArray(invalid.error.details));
});

test("ipc: an unknown profile is refused rather than silently defaulting", () => {
  const spec = fixture("good_spec.json");
  const response = handleRequest({ id: 13, method: "check", params: { spec, land: "XX" } });
  assert.equal(response.ok, false);
  assert.equal(response.error.code, "unknown_profile");
});

/* The desktop app's PDF export goes through this method, and it has two shapes:
   a written file when the caller passes `out`, base64 when it does not, because
   the protocol is one JSON object per line and cannot carry raw bytes. Both
   desktop/inject/desktop.js and the Rust shell depend on those field names. */

test("ipc: dossier_pdf is advertised so the shell can feature-detect it", () => {
  assert.ok(METHODS.includes("dossier_pdf"));
  assert.ok(handleRequest({ id: 20, method: "describe" }).result.methods.includes("dossier_pdf"));
});

test("ipc: dossier_pdf writes a PDF file when given a path", () => {
  const dir = mkdtempSync(join(tmpdir(), "baulens-ipc-pdf-"));
  try {
    const out = join(dir, "Befund.pdf");
    const response = handleRequest({
      id: 21,
      method: "dossier_pdf",
      params: { spec: fixture("bad_spec.json"), out, datum: "02.10.2026", aktenzeichen: "BV-2026-000042" },
    });
    assert.equal(response.ok, true, response.error && response.error.message);
    const result = response.result;
    assert.equal(result.out, out);
    assert.equal(result.hash.length, 64, "a SHA-256 Prüfziffer");
    assert.ok(result.pages >= 1);
    assert.equal(result.base64, undefined, "a written file does not need to travel as base64");

    const bytes = readFileSync(out);
    assert.equal(bytes.length, result.bytes);
    assert.equal(bytes.subarray(0, 8).toString("latin1"), "%PDF-1.4");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("ipc: dossier_pdf answers with base64 when no path is given", () => {
  const response = handleRequest({ id: 22, method: "dossier_pdf", params: { spec: fixture("good_spec.json") } });
  assert.equal(response.ok, true, response.error && response.error.message);
  assert.equal(response.result.out, undefined);
  const buffer = Buffer.from(response.result.base64, "base64");
  assert.equal(buffer.length, response.result.bytes);
  assert.equal(buffer.subarray(0, 8).toString("latin1"), "%PDF-1.4");
});

test("ipc: dossier_pdf ingests a plan file itself, like check_file", () => {
  const response = handleRequest({
    id: 23,
    method: "dossier_pdf",
    params: { file: join(root, "test", "fixtures", "sample_cad_lageplan.dxf") },
  });
  assert.equal(response.ok, true, response.error && response.error.message);
  assert.ok(response.result.bytes > 1000);
});

test("ipc: the same plan is sealed the same whether it arrives as a spec or a file", () => {
  /* The Prüfziffer covers the project fields, so a difference here means two
     dossiers for one building that an auditor cannot match up. This caught the
     ingestion label (the file name) winning over the name the plan declares,
     which also put "bad_spec.json" in the header of the printed Befund. */
  const spec = fixture("bad_spec.json");
  assert.equal(typeof spec.projekt, "string", "the fixture must declare a project name for this to mean anything");

  const fromSpec = handleRequest({ id: 30, method: "dossier_pdf", params: { spec } });
  const fromFile = handleRequest({
    id: 31,
    method: "dossier_pdf",
    params: { file: join(root, "test", "fixtures", "bad_spec.json") },
  });

  assert.equal(fromSpec.ok, true, fromSpec.error && fromSpec.error.message);
  assert.equal(fromFile.ok, true, fromFile.error && fromFile.error.message);
  assert.equal(fromFile.result.hash, fromSpec.result.hash, "one plan, one Prüfziffer");

  /* Searched inside a single drawn word, so a different line break in the meta
     box cannot make this pass or fail for the wrong reason. */
  const pdf = Buffer.from(fromFile.result.base64, "base64").toString("latin1");
  assert.ok(pdf.includes("Hang"), "the dossier is headed with the project name from the plan");
  assert.ok(!pdf.includes("bad_spec"), "never with the file name of the fixture");
});

test("ipc: dossier_pdf refuses bad input with a specific code, not a generic one", () => {
  const noInput = handleRequest({ id: 24, method: "dossier_pdf", params: {} });
  assert.equal(noInput.ok, false);
  assert.equal(noInput.error.code, "missing_spec");

  const unreadable = handleRequest({
    id: 25,
    method: "dossier_pdf",
    params: { file: join(root, "test", "fixtures", "nope.ifc") },
  });
  assert.equal(unreadable.ok, false);
  assert.equal(unreadable.error.code, "file_unreadable");

  /* A missing output directory is the caller's mistake, and it must be named:
     creating the tree behind the user's back would hide a mistyped path. */
  const missingDir = join(root, "test", "definitely-not-here");
  const badOut = handleRequest({
    id: 26,
    method: "dossier_pdf",
    params: { spec: fixture("good_spec.json"), out: join(missingDir, "Befund.pdf") },
  });
  assert.equal(badOut.ok, false);
  assert.equal(badOut.error.code, "missing_directory");
  assert.ok(!existsSync(missingDir), "the writer must not create directories");
});

/* The DACH matrix is one request for all 34 Rechtsbereiche. The page sends only
   the path of the active plan; the engine reads the spec itself, so the file and
   an already-built spec must give the same answer. */

test("ipc: lands_matrix is advertised and answers one row per Rechtsbereich", () => {
  assert.ok(METHODS.includes("lands_matrix"));
  const goodPath = join(root, "test", "fixtures", "good_spec.json");
  const byFile = handleRequest({ id: 70, method: "lands_matrix", params: { file: goodPath } });
  assert.equal(byFile.ok, true, byFile.error && byFile.error.message);

  const result = byFile.result;
  assert.equal(result.totalStates, 34, "every Landes- and Kantonsbauordnung is checked");
  assert.equal(result.rows.length, 34);
  const sum = Object.values(result.countsByVerdict).reduce((a, b) => a + b, 0);
  assert.equal(sum, 34, "each state has exactly one verdict");
  assert.equal(
    result.passedStates,
    result.countsByVerdict["bestanden"] + result.countsByVerdict["mit Auflagen"]
  );
  for (const row of result.rows) {
    assert.ok(!("rawResult" in row), "the full per-rule results stay out of the response");
    assert.ok(row.code && row.name && row.citationBase, "a row names its state and its norm");
  }

  const bySpec = handleRequest({
    id: 71,
    method: "lands_matrix",
    params: { spec: fixture("good_spec.json") },
  });
  assert.deepEqual(bySpec.result.countsByVerdict, result.countsByVerdict, "file and spec agree");
});

test("ipc: lands_matrix filters rows by country (DE, AT, CH)", () => {
  const goodPath = join(root, "test", "fixtures", "good_spec.json");
  const expected = { DE: 17, AT: 10, CH: 7, DACH: 34 };
  const prefixOf = { AT: "AT_", CH: "CH_" };

  for (const [country, total] of Object.entries(expected)) {
    const response = handleRequest({ id: 80, method: "lands_matrix", params: { file: goodPath, country } });
    assert.equal(response.ok, true, response.error && response.error.message);
    assert.equal(response.result.totalStates, total, `${country} lists ${total} Rechtsbereiche`);
    assert.equal(response.result.rows.length, total);
    if (prefixOf[country]) {
      for (const row of response.result.rows) {
        assert.ok(row.code.startsWith(prefixOf[country]), `${row.code} belongs to ${country}`);
      }
    } else if (country === "DE") {
      for (const row of response.result.rows) {
        assert.ok(!/^(AT_|CH_)/.test(row.code), `${row.code} is not a German state`);
      }
    }
  }

  const lower = handleRequest({ id: 81, method: "lands_matrix", params: { file: goodPath, country: "ch" } });
  assert.equal(lower.result.totalStates, 7, "the country code is case-insensitive");
});

test("ipc: lands_matrix refuses a missing input, an unknown country and an unreadable file with a code", () => {
  const goodPath = join(root, "test", "fixtures", "good_spec.json");
  const none = handleRequest({ id: 72, method: "lands_matrix", params: {} });
  assert.equal(none.ok, false);
  assert.equal(none.error.code, "missing_spec");

  const country = handleRequest({
    id: 73,
    method: "lands_matrix",
    params: { file: goodPath, country: "XX" },
  });
  assert.equal(country.error.code, "invalid_country");

  const gone = handleRequest({
    id: 74,
    method: "lands_matrix",
    params: { file: join(root, "test", "fixtures", "does_not_exist.json") },
  });
  assert.equal(gone.ok, false);
  assert.equal(gone.error.code, "file_unreadable");
});

test("ipc: lands_matrix_pdf is advertised and returns a valid landscape table as base64", () => {
  assert.ok(METHODS.includes("lands_matrix_pdf"));
  const goodPath = join(root, "test", "fixtures", "good_spec.json");
  const response = handleRequest({ id: 90, method: "lands_matrix_pdf", params: { file: goodPath } });
  assert.equal(response.ok, true, response.error && response.error.message);

  const { base64, pages, country, totalStates, passedStates } = response.result;
  const buffer = Buffer.from(base64, "base64");
  assert.equal(buffer.subarray(0, 5).toString("latin1"), "%PDF-");
  assert.equal(inspectXref(buffer).valid, true, "the xref table must resolve");
  assert.equal(country, "DACH");
  assert.equal(totalStates, 34);
  assert.ok(passedStates <= totalStates);
  assert.ok(pages >= 1, "at least one page");
  const pageObjects = buffer.toString("latin1").match(/\/Type\s*\/Page\b(?!s)/g) || [];
  assert.equal(pageObjects.length, pages, "the reported page count matches the document");
});

test("ipc: lands_matrix_pdf writes the file it promises and filters by country", () => {
  const goodPath = join(root, "test", "fixtures", "good_spec.json");
  const dir = mkdtempSync(join(tmpdir(), "matrix-pdf-"));
  try {
    const out = join(dir, "DACH-Matrix.pdf");
    const written = handleRequest({ id: 91, method: "lands_matrix_pdf", params: { file: goodPath, country: "de", out } });
    assert.equal(written.ok, true, written.error && written.error.message);
    assert.equal(written.result.out, out);
    assert.equal(written.result.country, "DE");
    assert.equal(written.result.totalStates, 17, "the DE filter reaches the PDF");
    const buffer = readFileSync(out);
    assert.equal(buffer.subarray(0, 5).toString("latin1"), "%PDF-");
    assert.equal(written.result.bytes, buffer.length);
    assert.equal(inspectXref(buffer).valid, true);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("ipc: lands_matrix_pdf refuses a missing input, an unknown country and a missing directory with a code", () => {
  const goodPath = join(root, "test", "fixtures", "good_spec.json");
  const none = handleRequest({ id: 92, method: "lands_matrix_pdf", params: {} });
  assert.equal(none.error.code, "missing_spec");

  const country = handleRequest({ id: 93, method: "lands_matrix_pdf", params: { file: goodPath, country: "XX" } });
  assert.equal(country.error.code, "invalid_country");

  const nowhere = handleRequest({
    id: 94,
    method: "lands_matrix_pdf",
    params: { file: goodPath, out: join(root, "no-such-dir", "matrix.pdf") },
  });
  assert.equal(nowhere.ok, false);
  assert.equal(nowhere.error.code, "missing_directory");
});

/* The drawing sheet has its own two methods because it is a deliverable in its
   own right: it goes to a plotter or into an existing plan set. They are text,
   so no base64 is involved, and they must be sealed with the same Prüfziffer as
   the dossier - an exported Blatt that cannot be tied back to the Befund is of
   little use in a submission folder. */

test("ipc: plan_svg and plan_dxf are advertised so the shell can feature-detect them", () => {
  for (const method of ["plan_svg", "plan_dxf"]) {
    assert.ok(METHODS.includes(method), `${method} is not advertised`);
    assert.ok(handleRequest({ id: 60, method: "describe" }).result.methods.includes(method));
  }
});

test("ipc: the plan exports answer inline with text and the same seal as the dossier", () => {
  const spec = fixture("good_spec.json");
  const params = { spec, datum: "02.10.2026", aktenzeichen: "BV-2026-000123" };

  const svg = handleRequest({ id: 61, method: "plan_svg", params });
  const dxf = handleRequest({ id: 62, method: "plan_dxf", params });
  const pdf = handleRequest({ id: 63, method: "dossier_pdf", params });

  assert.equal(svg.ok, true, svg.error && svg.error.message);
  assert.equal(dxf.ok, true, dxf.error && dxf.error.message);

  assert.match(svg.result.svg, /^<\?xml version="1\.0" encoding="UTF-8"/);
  assert.match(dxf.result.dxf, /^999\r\n/);
  assert.equal(svg.result.base64, undefined, "text travels inline, not base64");
  assert.equal(svg.result.blatt, "Lageplan (Abstandsflächen)");
  assert.equal(svg.result.scale, "1:250");
  assert.equal(dxf.result.scale, svg.result.scale);
  assert.equal(svg.result.bytes, Buffer.byteLength(svg.result.svg, "utf8"));
  assert.equal(dxf.result.bytes, Buffer.byteLength(dxf.result.dxf, "utf8"));
  assert.equal(svg.result.hash, pdf.result.hash, "the Blatt carries the Prüfziffer of the Befund");
  assert.equal(dxf.result.hash, pdf.result.hash);
  assert.equal(dxf.result.verdict, pdf.result.verdict);
});

test("ipc: the plan exports write a file when given a path and refuse an invented directory", () => {
  const dir = mkdtempSync(join(tmpdir(), "baulens-ipc-plan-"));
  try {
    const out = join(dir, "Lageplan.dxf");
    const written = handleRequest({
      id: 64,
      method: "plan_dxf",
      params: { spec: fixture("bad_spec.json"), out, datum: "02.10.2026" },
    });
    assert.equal(written.ok, true, written.error && written.error.message);
    assert.equal(written.result.out, out);
    assert.equal(written.result.dxf, undefined, "a written file does not travel inline");
    assert.equal(readFileSync(out).length, written.result.bytes);

    const missing = handleRequest({
      id: 65,
      method: "plan_svg",
      params: { spec: fixture("bad_spec.json"), out: join(dir, "gibt-es-nicht", "x.svg") },
    });
    assert.equal(missing.ok, false);
    assert.equal(missing.error.code, "missing_directory");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

/* The workspace export is one call for a whole folder of Vorhaben, because the
   desktop app offers exactly one action: open a project folder, press "Alle
   Lagepläne exportieren", and the submission folder holds a Blatt per project.
   The loop is here and not in the shell because the naming, the scale and the
   Prüfziffer are engine facts. */

test("ipc: plan_export is advertised so the shell can feature-detect it", () => {
  assert.ok(METHODS.includes("plan_export"));
  assert.ok(handleRequest({ id: 70, method: "describe" }).result.methods.includes("plan_export"));
});

test("ipc: plan_export writes one Blatt per Vorhaben and lets a broken plan fail alone", () => {
  const dir = mkdtempSync(join(tmpdir(), "baulens-ipc-plans-"));
  try {
    const good = join(root, "test", "fixtures", "good_spec.json");
    const bad = join(root, "test", "fixtures", "bad_spec.json");
    const empty = join(root, "test", "fixtures", "definitely-missing.ifc");

    const response = handleRequest({
      id: 71,
      method: "plan_export",
      params: {
        outDir: dir,
        files: [
          { path: good, label: "Wohnanlage Musterweg" },
          { path: bad },
          { path: good, label: "Wohnanlage Musterweg" },
          { path: empty, label: "Fehlt" },
        ],
        datum: "02.10.2026",
        aktenzeichen: "BV-2026-000123",
      },
    });

    assert.equal(response.ok, true, response.error && response.error.message);
    const result = response.result;
    assert.deepEqual(result.formats, ["dxf", "svg"]);
    assert.deepEqual(result.counts, { plans: 4, written: 6, failed: 1, drawn: 3 });

    /* Ein unlesbarer Plan nimmt die anderen nicht mit - und wird benannt. */
    assert.equal(result.failed.length, 1);
    assert.equal(result.failed[0].source, empty);
    assert.equal(result.failed[0].code, "file_unreadable");

    /* Zwei Vorhaben mit demselben Namen bekommen getrennte Dateien, und das
       wird berichtet: ein Ordner mit einem Blatt weniger, als der Bericht
       behauptet, ist der Fall, den niemand bemerkt. */
    assert.equal(result.collisions.length, 1);
    assert.equal(result.collisions[0].renamed, "Wohnanlage-Musterweg-2");

    const written = readFileSync(join(dir, "Wohnanlage-Musterweg_Lageplan.dxf"), "utf8");
    assert.match(written, /^999\r\n/, "the DXF on disk is the document");
    assert.match(readFileSync(join(dir, "Wohnanlage-Musterweg_Lageplan.svg"), "utf8"), /^<\?xml/);
    assert.match(readFileSync(join(dir, "Wohnanlage-Musterweg-2_Lageplan.dxf"), "utf8"), /^999\r\n/);
    assert.match(readFileSync(join(dir, "bad_spec_Lageplan.svg"), "utf8"), /^<\?xml/);

    /* Jedes geschriebene Blatt trägt Maßstab, Prüfziffer und den Namen des
       Vorhabens, das es zeigt - die Zuordnung muss ohne Ordnerpfad gehen. */
    for (const entry of result.written) {
      assert.equal(entry.scale, "1:250");
      assert.equal(entry.hash.length, 64);
      assert.equal(typeof entry.projekt, "string");
      assert.equal(readFileSync(entry.out).length, entry.bytes, `${entry.out} is the reported size`);
    }
    assert.equal(result.written[0].label, "Wohnanlage-Musterweg");
    assert.equal(result.written[0].projekt, "Musterweg 12, 80331 München");

    /* Ein Verzeichnis wird nie erfunden: der Stapel schreibt in einen Ordner,
       den jemand ausgewählt hat. */
    const missing = handleRequest({
      id: 72,
      method: "plan_export",
      params: { outDir: join(dir, "gibt-es-nicht"), files: [good] },
    });
    assert.equal(missing.ok, false);
    assert.equal(missing.error.code, "missing_directory");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("ipc: plan_export validates its input instead of guessing", () => {
  const dir = mkdtempSync(join(tmpdir(), "baulens-ipc-plans-bad-"));
  try {
    const good = join(root, "test", "fixtures", "good_spec.json");
    const cases = [
      [{ files: [good] }, "missing_out_dir"],
      [{ outDir: "   ", files: [good] }, "missing_out_dir"],
      [{ outDir: dir, files: [] }, "missing_files"],
      [{ outDir: dir, files: [{ label: "ohne Pfad" }] }, "missing_files"],
      [{ outDir: join(dir, "nope"), files: [good] }, "missing_directory"],
    ];
    for (const [params, expected] of cases) {
      const response = handleRequest({ id: 73, method: "plan_export", params });
      assert.equal(response.ok, false, JSON.stringify(params));
      assert.equal(response.error.code, expected, JSON.stringify(params));
    }

    /* Ein unbekanntes Format wird benannt, nicht stillschweigend ignoriert:
       ein PDF-Blatt gibt es hier nicht, und ein leiser Rückfall auf DXF wäre
       eine Datei, die niemand erwartet. */
    const unknown = handleRequest({
      id: 74,
      method: "plan_export",
      params: { outDir: dir, files: [good], formats: ["pdf"] },
    });
    assert.equal(unknown.ok, false);
    assert.equal(unknown.error.code, "invalid_formats");
    assert.match(unknown.error.message, /pdf/);
    assert.match(unknown.error.message, /dxf/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("ipc: plan_export can write one format only", () => {
  const dir = mkdtempSync(join(tmpdir(), "baulens-ipc-plans-one-"));
  try {
    const good = join(root, "test", "fixtures", "good_spec.json");
    const response = handleRequest({
      id: 75,
      method: "plan_export",
      params: { outDir: dir, files: [{ path: good, label: "Nur DXF" }], formats: ["dxf"] },
    });
    assert.equal(response.ok, true, response.error && response.error.message);
    assert.deepEqual(response.result.formats, ["dxf"]);
    assert.equal(response.result.counts.written, 1);
    assert.deepEqual(readdirSync(dir), ["Nur-DXF_Lageplan.dxf"]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("ipc: statute lookup returns verbatim statutory text with regional adaptation", () => {
  const mbo = handleRequest({
    id: 80,
    method: "statute",
    params: { ref: "§6 Abs. 2 MBO" },
  });
  assert.equal(mbo.ok, true);
  assert.equal(mbo.result.found, true);
  assert.match(mbo.result.cite, /§\s*6 MBO/);
  assert.ok(mbo.result.text.length > 40);

  const bay = handleRequest({
    id: 81,
    method: "statute",
    params: { ref: "§6 Abs. 2 MBO", land: "BY" },
  });
  assert.equal(bay.ok, true);
  assert.equal(bay.result.found, true);
  assert.match(bay.result.cite, /BayBO/);

  const missing = handleRequest({
    id: 82,
    method: "statute",
    params: {},
  });
  assert.equal(missing.ok, false);
  assert.equal(missing.error.code, "missing_ref");
});

test("ipc: check_file returns repairs, model geometry, and optional deviations", () => {
  const bad = join(root, "test", "fixtures", "bad_spec.json");
  const response = handleRequest({
    id: 83,
    method: "check_file",
    params: { file: bad, land: "BY", stadt: "MUENCHEN", deviations: true },
  });
  assert.equal(response.ok, true, response.error && response.error.message);
  const res = response.result;
  assert.equal(res.profile.code, "BY");
  assert.equal(res.municipality.code, "MUENCHEN");
  assert.ok(Array.isArray(res.repairs) && res.repairs.length > 0, "check_file generates repairs by default");
  assert.ok(res.repairs[0].patchedSpec, "repairs carry patchedSpec for What-If simulation");
  assert.ok(res.model && res.model.height > 0, "result carries derived 3D model dimensions");
  assert.equal(typeof res.deviationsText, "string");
  assert.match(res.deviationsText, /ANTRAG AUF ABWEICHUNG/);
  assert.match(res.deviationsText, /München/);
});


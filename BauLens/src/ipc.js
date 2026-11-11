/**
 * Line-delimited JSON request handling, for the desktop app's sidecar.
 *
 * The desktop shell needs to run the *tested* engine, not a second copy of the
 * rules living in the UI. Rust cannot execute JavaScript, so the engine is
 * exposed as a child process speaking one JSON object per line on stdin and
 * stdout. That keeps `src/` the single implementation and keeps it under the
 * 146-test suite.
 *
 * Protocol
 *   request   {"id":<any>,"method":"check","params":{"spec":{...},"land":"BY",
 *              "stadt":"MUENCHEN","rules":["grz"],"repair":true,"report":true}}
 *   request   {"id":<any>,"method":"check_file","params":{"path":"...","land":"BY"}}
 *   request   {"id":<any>,"method":"dossier_pdf","params":{"spec":{...},"land":"BY",
 *              "aktenzeichen":"BV-2026-000123","datum":"2026-10-05",
 *              "out":"C:/Befund.pdf"}}
 *   request   {"id":<any>,"method":"plan_svg"|"plan_dxf","params":{"file":"C:/haus.dxf",
 *              "out":"C:/Lageplan.svg"}}
 *   request   {"id":<any>,"method":"plan_export","params":{"outDir":"C:/Einreichung",
 *              "formats":["dxf","svg"],
 *              "files":[{"path":"C:/BV/haus.dxf","label":"Wohnanlage Musterweg"}]}}
 *   response  {"id":<any>,"ok":true,"result":{...}}
 *             {"id":<any>,"ok":false,"error":{"message":"...","code":"..."}}
 *
 * `check` takes an already-built spec; `check_file` reads a plan file (IFC, DXF,
 * XPlanung, GeoJSON, Baubeschreibung or a JSON spec), ingests it through
 * src/ingest.js and then runs the same check. The workspace scan and the batch
 * run live in the shell, which has the filesystem; this method is what it calls
 * per file, so ingestion stays in the tested engine.
 *
 * `dossier_pdf` answers with a printable DIN-A4 Befund as a real PDF file, so the
 * desktop app can save a document instead of opening a print dialog. Given
 * `params.out` the engine writes the file and reports its path; without it the
 * bytes come back base64-encoded, because this protocol is one JSON object per
 * line and raw binary cannot travel in it. The layout lives in
 * src/dossierpdf.js -- this method only decides where the bytes end up.
 *
 * `plan_svg` and `plan_dxf` take the same input as `dossier_pdf` and export the
 * Lageplan sheet alone, because that is the one part of the dossier that is a
 * deliverable in its own right: it goes to a plotter or into an existing plan
 * set. Both are text, so without `params.out` they come back inline instead of
 * base64-encoded - a UI can preview the SVG or hand the DXF straight to a
 * download without decoding anything.
 *
 * `plan_export` is the same Blatt for a whole workspace: a list of plans and one
 * target folder, one Blatt per Vorhaben, named after it. The loop belongs to the
 * engine because the drawing, the naming and the Prüfziffer do - the shell only
 * scans the folder and forwards what it found.
 *
 * One line in, one line out, in order. A malformed line produces an error
 * response rather than killing the process, so a bad request from the UI cannot
 * take the app down.
 */
import { readFileSync } from "node:fs";
import { validateSpec } from "./schema.js";
import { ingestFile } from "./ingest.js";
import { runChecks } from "./engine.js";
import { generateMarkdownReport } from "./report.js";
import { getRuleById, ALL_RULES, EXTRA_RULES } from "./rules/index.js";
import { generateComplianceRepairs } from "./repair.js";
import { exportBcfCollection } from "./bcf.js";
import { generateXBauXml, generateAbweichungsPaket } from "./xbau.js";
import { getStatuteByQuery } from "./statutes.js";
import { PROFILES } from "./profiles.js";
import { MUNICIPALITIES } from "./municipalities.js";
import { existsSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import {
  buildDossierPdf,
  planDocument,
  writeDossierPdf,
  writePlanDxf,
  writePlanSvg,
} from "./dossierpdf.js";
import { PLAN_FORMATS, planOutputNames } from "./planexport.js";
import { buildLandsMatrixPdf, runAllLandsMatrix } from "./matrix.js";
import { VERSION, PRODUCT } from "./version.js";

/** Methods the sidecar answers. Anything else is rejected by name. */
export const METHODS = [
  "ping",
  "describe",
  "check",
  "check_file",
  "statute",
  "dossier_pdf",
  "plan_svg",
  "plan_dxf",
  "plan_export",
  "lands_matrix",
  "lands_matrix_pdf",
];

function describe() {
  /* All 15 modules, not just the 9 core ones: selectRules([]) returns ALL_RULES
     alone, and a UI listing the available checks needs the profile-activated
     extras too. */
  const everyRule = [...ALL_RULES, ...EXTRA_RULES];
  return {
    product: PRODUCT,
    version: VERSION,
    methods: METHODS,
    rules: everyRule.map((rule) => ({
      id: rule.id,
      para: rule.para,
      title: rule.title,
      core: ALL_RULES.includes(rule),
    })),
    profiles: Object.entries(PROFILES).map(([code, p]) => ({
      code,
      name: p.name,
      citationBase: p.citationBase,
      country: p.country || "DE",
    })),
    municipalities: Object.values(MUNICIPALITIES).map((m) => ({
      code: m.code,
      name: m.name,
      bundesland: m.bundesland,
    })),
    counts: { total: everyRule.length, core: ALL_RULES.length, extra: EXTRA_RULES.length },
  };
}

function check(params = {}) {
  const spec = params.spec;
  if (spec === undefined || spec === null) {
    const error = new Error("params.spec is required");
    error.code = "missing_spec";
    throw error;
  }

  const validation = validateSpec(spec);
  if (!validation.ok) {
    const error = new Error(`spec is not valid: ${validation.errors.join("; ")}`);
    error.code = "invalid_spec";
    error.details = validation.errors;
    throw error;
  }

  if (Array.isArray(params.rules) && params.rules.length) {
    const unknown = params.rules.filter((id) => !getRuleById(id));
    if (unknown.length) {
      const error = new Error(`unknown rule id(s): ${unknown.join(", ")}`);
      error.code = "unknown_rule";
      error.details = unknown;
      throw error;
    }
  }

  /* runChecks returns { results, counts, verdict, profile, municipality }, with
     findings nested per rule. The flat list below is what a UI renders. */
  const run = runChecks(spec, {
    land: params.land ?? undefined,
    stadt: params.stadt ?? undefined,
  });

  if (!run) {
    const error = new Error(
      `no rule profile resolved for land "${params.land ?? "(from spec)"}"`
    );
    error.code = "unknown_profile";
    throw error;
  }

  if (Array.isArray(params.rules) && params.rules.length) {
    const wanted = new Set(params.rules);
    run.results = run.results.filter((r) => wanted.has(r.id));
  }

  const resolvedLand = run.profile?.code ?? "MBO";
  const resolvedMuni = run.municipality?.code ?? null;

  const findings = run.results.flatMap((result) =>
    (result.findings ?? []).map((f) => {
      const para = result.para ?? getRuleById(result.id)?.para ?? null;
      const statute = getStatuteByQuery(para || result.id, resolvedLand, resolvedMuni);
      return {
        ruleId: result.id,
        title: result.title,
        para,
        status: f.status,
        message: f.message,
        statute: statute
          ? {
              id: statute.id,
              norm: statute.norm,
              title: statute.title,
              jurisdiction: statute.jurisdiction,
              wortlaut: statute.wortlaut,
              commentary: statute.commentary,
              remedy: statute.remedy,
            }
          : null,
      };
    })
  );

  /* Only report the shapes the UI asked for: a markdown report, a repair
     suggestion set or an XBau document is far heavier than a verdict. */
  const result = {
    verdict: run.verdict,
    counts: run.counts,
    profile: run.profile ? { code: run.profile.code, name: run.profile.name, citationBase: run.profile.citationBase } : null,
    municipality: run.municipality ? { code: run.municipality.code, name: run.municipality.name } : null,
    findings,
    warnings: validation.warnings ?? [],
  };

  if (params.repair) {
    /* repair wants the profile's setback parameter block (factor, minM) from
       ruleOverrides.abstandsflaeche.params, falling back to profile.params. */
    const paramsBlock =
      run.profile?.ruleOverrides?.abstandsflaeche?.params ??
      run.profile?.params ??
      {};
    result.repairs = generateComplianceRepairs(spec, findings, paramsBlock);
  }
  if (params.report) {
    result.report = generateMarkdownReport(run, { projekt: spec.projekt });
  }
  if (params.bcf) {
    result.bcf = exportBcfCollection(findings);
  }
  if (params.xbau) {
    result.xbau = generateXBauXml(spec, run);
  }
  if (params.abweichung || params.deviations) {
    const devDoc = generateAbweichungsPaket(spec, findings, {
      land: resolvedLand,
      stadt: run.municipality?.name ?? spec?.stadt ?? spec?.kommune,
    });
    result.abweichung = devDoc;
    result.deviationsText = devDoc;
  }

  return result;
}

/**
 * Looks up a statutory text & commentary in the legal knowledge base.
 */
function statuteLookup(params = {}) {
  const ref = params.ref ?? params.query;
  if (typeof ref !== "string" || !ref.trim()) {
    const error = new Error("params.ref is required");
    error.code = "missing_ref";
    throw error;
  }
  const entry = getStatuteByQuery(ref, params.land ?? "MBO", params.stadt ?? null);
  if (!entry) {
    return { found: false, ref };
  }
  return {
    found: true,
    id: entry.id,
    cite: entry.norm,
    norm: entry.norm,
    title: entry.title,
    source: entry.jurisdiction,
    jurisdiction: entry.jurisdiction,
    text: entry.wortlaut,
    wortlaut: entry.wortlaut,
    note: [entry.commentary, entry.remedy].filter(Boolean).join(" "),
    commentary: entry.commentary,
    remedy: entry.remedy,
  };
}

/**
 * Reads one plan file and ingests it.
 *
 * The file is read here rather than in the shell because ingestion is engine
 * logic (it maps parser output onto the spec schema) and belongs with the tests.
 * The shell only ever passes a path. Shared by `check_file` and `dossier_pdf` so
 * a file that ingests for one answers for the other too.
 */
function readSpec(filePath) {
  if (typeof filePath !== "string" || !filePath.trim()) {
    const error = new Error("params.path is required");
    error.code = "missing_path";
    throw error;
  }

  let text;
  try {
    text = readFileSync(filePath, "utf8");
  } catch (error) {
    const wrapped = new Error(`${filePath} ist nicht lesbar: ${error.message}`);
    wrapped.code = "file_unreadable";
    throw wrapped;
  }

  /* ingestFile throws with its own code (unsupported_file, invalid_spec,
     *_parse), which the error envelope below passes straight through. */
  return { filePath, ingested: ingestFile(filePath, text) };
}

function deriveModelFromSpec(spec = {}) {
  const b = spec.gebaeude ?? {};
  const g = spec.grundstueck ?? {};
  const a = b.abstaende_m ?? {};
  const flaeche = Number(g.flaeche_m2) || 800;
  const grz = Number(b.grz_ist) || 0.32;
  const footprint = Math.max(36, flaeche * grz);
  const width = +(Number(b.breite_m) || Math.sqrt(footprint * 1.45)).toFixed(2);
  const depth = +(Number(b.tiefe_m) || footprint / width).toFixed(2);
  const height = +(Number(b.hoehe_m) || 8.5).toFixed(2);
  const storeys = Math.max(1, Math.round(Number(b.geschosse_ueber_grund ?? b.geschosse) || 3));
  return {
    width,
    depth,
    height,
    storeys,
    flaeche_m2: flaeche,
    setbacks: {
      norden: Number(a.norden ?? a.nord ?? 3.0),
      osten: Number(a.osten ?? a.ost ?? 3.0),
      sueden: Number(a.sueden ?? a.sued ?? 3.0),
      westen: Number(a.westen ?? a.west ?? 3.0),
    },
  };
}

/**
 * Reads one plan file, ingests it, and runs the check on the resulting spec.
 */
function checkFile(params = {}) {
  const targetPath = params.path ?? params.file;
  const { filePath, ingested } = readSpec(targetPath);

  const result = check({ repair: true, ...params, spec: ingested.spec });
  const model = deriveModelFromSpec(ingested.spec);

  return {
    path: filePath,
    kind: ingested.kind,
    source: ingested.source,
    model,
    project: ingested.project,
    notes: ingested.notes,
    spec: ingested.spec,
    ...result,
  };
}

/**
 * Resolves what `dossier_pdf`, `plan_svg` and `plan_dxf` all need: the spec
 * (given or ingested), the full engine run, and the header data the document is
 * rendered from.
 *
 * This runs the *full* engine rather than reusing `check()` above: `check()`
 * returns a reduced answer with a flat, lossy finding list and a stripped
 * profile, and the documents need the complete per-module results (citation,
 * title, messages) plus `profile.country` and `profile.authorityTitle` for the
 * header. Running `runChecks` directly is the only way to get those.
 *
 * One implementation for all three outputs, because they must not disagree: the
 * plan sheet inside the PDF and the exported DXF are the same drawing only if
 * they come from the same run and the same `projectData`.
 *
 * Input is either an already-built spec or a plan file to ingest, so a UI that
 * just checked a file can produce its dossier without re-sending the spec.
 */
function resolveDossierInput(params = {}) {
  let spec = params.spec;
  let ingested = null;

  if (spec === undefined || spec === null) {
    if (params.file !== undefined && params.file !== null) {
      const read = readSpec(params.file);
      spec = read.ingested.spec;
      ingested = read.ingested;
    } else {
      const error = new Error("params.spec or params.file is required");
      error.code = "missing_spec";
      throw error;
    }
  }

  const validation = validateSpec(spec);
  if (!validation.ok) {
    const error = new Error(`spec is not valid: ${validation.errors.join("; ")}`);
    error.code = "invalid_spec";
    error.details = validation.errors;
    throw error;
  }

  const run = runChecks(spec, {
    land: params.land ?? undefined,
    stadt: params.stadt ?? undefined,
  });

  if (!run) {
    const error = new Error(
      `no rule profile resolved for land "${params.land ?? "(from spec)"}"`
    );
    error.code = "unknown_profile";
    throw error;
  }

  /* The header fields come from the request, then from the spec, then from the
     renderer's defaults -- the same precedence the HTML dossier uses.

     Order matters for `projekt`: ingestion's `project` is a label derived from
     the file name, and letting it win over the name the plan itself declares put
     "haus.dxf" in the header of a dossier whose spec says "Am Hang 3" -- and, as
     the seal covers these fields, gave the file and the spec two different
     Prüfziffern for the same building. The declared name wins; the label is only
     a fallback for a plan that carries no name at all. */
  const grundstueck = spec.grundstueck ?? null;
  const projectData = {
    projekt: params.projekt ?? spec.projekt ?? ingested?.project,
    aktenzeichen: params.aktenzeichen,
    entwurfsverfasser: params.entwurfsverfasser,
    bauherr: params.bauherr,
    ...(grundstueck
      ? {
          grundstueck,
          flurstueck: grundstueck.flurstueck,
          gemarkung: grundstueck.gemarkung,
        }
      : {}),
    /* Symmetrisch zur Kommandozeile: der Lageplan leitet den Hauskörper aus
       diesen Angaben ab, und die Prüfziffer deckt sie mit ab - sonst trügen zwei
       Vorhaben, die sich nur in der GRZ unterscheiden, dasselbe Siegel. */
    ...(spec.gebaeude ? { gebaeude: spec.gebaeude } : {}),
  };

  const options = {
    datum: params.datum,
    aktenzeichen: params.aktenzeichen,
    entwurfsverfasser: params.entwurfsverfasser,
    bauherr: params.bauherr,
  };

  return { spec, ingested, run, projectData, options };
}

/**
 * Renders the printable DIN-A4 Befund as a PDF: with `params.out` the file, and
 * without it the bytes base64-encoded, because this protocol is one JSON object
 * per line and raw binary cannot travel in it.
 */
function dossierPdf(params = {}) {
  const base = resolveDossierInput(params);
  const { run, projectData } = base;
  const options = withVariantComparison(base.options, params);

  const summarise = (built) => ({
    bytes: built.bytes ?? built.buffer.length,
    hash: built.hash,
    pages: built.pages,
    verdict: run.verdict,
    counts: run.counts,
  });

  if (typeof params.out === "string" && params.out.trim()) {
    const written = writeDossierPdf(params.out, projectData, run, options);
    return { out: written.path, ...summarise(written) };
  }

  const built = buildDossierPdf(projectData, run, options);
  return { base64: built.buffer.toString("base64"), ...summarise(built) };
}

/**
 * Adds the Vorher-/Nachher-Blatt when a Heilungsvariante is active. The variant
 * spec goes through the same resolution as the base spec (validation, engine
 * run, header fields), so both columns of the Blatt come from one pipeline. A
 * request without `variant` is unchanged.
 */
function withVariantComparison(options, params = {}) {
  const variant = params.variant;
  if (variant === undefined || variant === null) return options;
  if (typeof variant !== "object" || typeof variant.spec !== "object" || variant.spec === null) {
    const error = new Error("params.variant.spec is required when params.variant is given");
    error.code = "invalid_variant";
    throw error;
  }

  const next = resolveDossierInput({ ...params, spec: variant.spec, file: undefined, variant: undefined });
  return {
    ...options,
    comparison: {
      projectData: next.projectData,
      runResult: next.run,
      label: typeof variant.label === "string" ? variant.label : undefined,
    },
  };
}

/**
 * Writes the Lageplan sheet for every given plan into one folder - the
 * workspace export: open a project folder, press one button, and the submission
 * folder holds a Blatt per Vorhaben.
 *
 * The loop lives here rather than in the shell for the reason the whole sidecar
 * exists: the shell can walk a filesystem, but the drawing, the naming and the
 * Prüfziffer are engine facts, and a second implementation of "which plan goes
 * to which file" in Rust is a second thing to keep correct. The shell scans and
 * forwards the list; this method answers per entry.
 *
 * One broken plan must not lose the others (the same promise `batch_check`
 * makes): a file that cannot be ingested is reported with its error and the
 * export continues. A missing target directory is an error, never something
 * invented - `C:\Projekte\Einreichung\` with a typo in it is how a folder of
 * Blätter ends up where nobody looks for it.
 */
function planExport(params = {}) {
  /* `path` because that is the key the workspace scan already uses, so the shell
     can hand its own file list over without translating it; `label` is the
     Vorhaben name the Blatt is filed under, and stays optional - without it the
     plan's own file name is the label. `name` is deliberately not the key: in a
     scan descriptor `name` is the *file* name, and a Blatt called
     "haus.dxf_Lageplan.svg" is not what anybody wants to hand to an authority. */
  const rawFiles = Array.isArray(params.files) ? params.files : [];
  const entries = rawFiles
    .map((entry) => (typeof entry === "string" ? { path: entry } : entry))
    .filter((entry) => entry && typeof entry.path === "string" && entry.path.trim())
    .map((entry) => ({
      source: entry.path,
      name: typeof entry.label === "string" && entry.label.trim() ? entry.label : null,
    }));

  if (entries.length === 0) {
    const error = new Error("params.files is required and must name at least one plan");
    error.code = "missing_files";
    throw error;
  }

  const formats = Array.isArray(params.formats) && params.formats.length ? params.formats : PLAN_FORMATS;
  const unknown = formats.filter((format) => !PLAN_FORMATS.includes(format));
  if (unknown.length) {
    const error = new Error(`unknown Blattformat: ${unknown.join(", ")} (bekannt: ${PLAN_FORMATS.join(", ")})`);
    error.code = "invalid_formats";
    throw error;
  }

  if (typeof params.outDir !== "string" || !params.outDir.trim()) {
    const error = new Error("params.outDir is required");
    error.code = "missing_out_dir";
    throw error;
  }
  if (!existsSync(params.outDir)) {
    const error = new Error(`Das Zielverzeichnis existiert nicht: ${params.outDir}`);
    error.code = "missing_directory";
    throw error;
  }

  const { names, collisions } = planOutputNames(entries, formats);
  const written = [];
  const failed = [];

  entries.forEach((entry, index) => {
    try {
      /* One input per entry, resolved exactly as `dossier_pdf` resolves it: the
         same ingestion, the same run, the same `projectData`. The drawable data
         and the Prüfziffer therefore cannot differ between the dossier of a plan
         and the Blatt exported beside it. */
      const input = resolveDossierInput({
        file: entry.source,
        land: params.land ?? undefined,
        stadt: params.stadt ?? undefined,
        aktenzeichen: params.aktenzeichen,
        entwurfsverfasser: params.entwurfsverfasser,
        bauherr: params.bauherr,
        datum: params.datum,
      });

      for (const format of formats) {
        const target = join(params.outDir, names[index].files[format]);
        const result = format === "dxf"
          ? writePlanDxf(target, input.projectData, input.run, input.options)
          : writePlanSvg(target, input.projectData, input.run, input.options);
        written.push({
          source: entry.source,
          projekt: input.projectData.projekt ?? null,
          label: names[index].label,
          format,
          out: result.path,
          bytes: result.bytes,
          scale: result.scale,
          hash: result.hash,
        });
      }
    } catch (error) {
      failed.push({ source: entry.source, message: error.message, code: error.code ?? null });
    }
  });

  return {
    outDir: params.outDir,
    formats,
    written,
    failed,
    collisions,
    counts: {
      plans: entries.length,
      written: written.length,
      failed: failed.length,
      drawn: entries.length - failed.length,
    },
  };
}

/**
 * The Lageplan sheet as a file of its own, in DXF or SVG.
 *
 * Same input, same run and the same `projectData` as `dossier_pdf`, so the
 * exported Blatt is the Blatt in the PDF. The Prüfziffer travels with it: an
 * exported drawing that cannot be tied back to the check that produced it is of
 * little use in a submission folder.
 *
 * Both formats are text, so without `params.out` they are returned inline rather
 * than base64-encoded - unlike the PDF, where one JSON line has to survive
 * 30 KB of binary. `scale` is the Blatt's Maßstab ("1:250"): the UI shows it, and
 * for the DXF it is what the drawing units mean.
 */
function planSingle(params = {}, format = "svg") {
  const { run, projectData, options } = resolveDossierInput(params);
  const document = planDocument(projectData, run, options, format);
  const summary = {
    blatt: "Lageplan (Abstandsflächen)",
    scale: document.geometry.label,
    bytes: Buffer.byteLength(document.contents),
    hash: document.model.hash,
    verdict: run.verdict,
    counts: run.counts,
  };

  if (typeof params.out === "string" && params.out.trim()) {
    const written = format === "dxf"
      ? writePlanDxf(params.out, projectData, run, options)
      : writePlanSvg(params.out, projectData, run, options);
    return { out: written.path, ...summary };
  }

  return format === "dxf" ? { dxf: document.contents.toString("utf8"), ...summary } : { svg: document.contents, ...summary };
}

/* The DACH matrix: one spec against every profile at once. The rawResult of
   each run is dropped - a UI needs verdict, counts and the findings, and the
   full per-rule results would make the response several times larger. */
const LANDS_MATRIX_COUNTRIES = ["DE", "AT", "CH", "DACH", "ALL"];

/* Shared by lands_matrix and lands_matrix_pdf: the same spec, the same
   validation and the same country filter, so the PDF cannot disagree with
   the table the UI showed. */
function resolveLandsMatrixInput(params = {}) {
  let spec = params.spec;
  if (spec === undefined || spec === null) {
    if (params.file === undefined || params.file === null) {
      const error = new Error("params.spec or params.file is required");
      error.code = "missing_spec";
      throw error;
    }
    spec = readSpec(params.file).ingested.spec;
  }

  const validation = validateSpec(spec);
  if (!validation.ok) {
    const error = new Error(`spec is not valid: ${validation.errors.join("; ")}`);
    error.code = "invalid_spec";
    error.details = validation.errors;
    throw error;
  }

  const country = params.country ? String(params.country).trim().toUpperCase() : "DACH";
  if (!LANDS_MATRIX_COUNTRIES.includes(country)) {
    const error = new Error(`params.country must be one of ${LANDS_MATRIX_COUNTRIES.join(", ")}`);
    error.code = "invalid_country";
    throw error;
  }
  return { spec, country };
}

function landsMatrix(params = {}) {
  const { spec, country } = resolveLandsMatrixInput(params);
  const matrix = runAllLandsMatrix(spec, { country });
  return {
    totalStates: matrix.totalStates,
    passedStates: matrix.passedStates,
    allPassed: matrix.allPassed,
    countsByVerdict: matrix.countsByVerdict,
    rows: matrix.matrix.map(({ rawResult, ...row }) => row),
  };
}

/* The DACH matrix as a landscape A4 table. Same answer as lands_matrix, drawn
   by src/matrix.js. Given params.out the file is written and its path comes
   back; without it the bytes come back base64-encoded, as dossier_pdf does. */
function landsMatrixPdf(params = {}) {
  const { spec, country } = resolveLandsMatrixInput(params);
  const matrix = runAllLandsMatrix(spec, { country });
  const built = buildLandsMatrixPdf(matrix, { projekt: spec.projekt, country });
  const summary = {
    bytes: built.buffer.length,
    pages: built.pages,
    country,
    totalStates: matrix.totalStates,
    passedStates: matrix.passedStates,
  };

  if (typeof params.out === "string" && params.out.trim()) {
    const directory = dirname(params.out);
    if (!existsSync(directory)) {
      const error = new Error(`Das Zielverzeichnis existiert nicht: ${directory}`);
      error.code = "missing_directory";
      throw error;
    }
    try {
      writeFileSync(params.out, built.buffer);
    } catch (cause) {
      const error = new Error(`${params.out} konnte nicht geschrieben werden: ${cause.message}`);
      error.code = "pdf_write_failed";
      throw error;
    }
    return { out: params.out, ...summary };
  }

  return { base64: built.buffer.toString("base64"), ...summary };
}

const HANDLERS = {
  ping: () => ({ pong: true, version: VERSION }),
  lands_matrix: landsMatrix,
  lands_matrix_pdf: landsMatrixPdf,
  describe,
  check,
  check_file: checkFile,
  statute: statuteLookup,
  dossier_pdf: dossierPdf,
  plan_svg: (params) => planSingle(params, "svg"),
  plan_dxf: (params) => planSingle(params, "dxf"),
  plan_export: planExport,
};

/** Handles one already-parsed request. Never throws. */
export function handleRequest(request) {
  const id = request && typeof request === "object" ? request.id ?? null : null;

  try {
    if (!request || typeof request !== "object") {
      const error = new Error("request must be a JSON object");
      error.code = "bad_request";
      throw error;
    }
    const handler = HANDLERS[request.method];
    if (!handler) {
      const error = new Error(`unknown method "${request.method}"`);
      error.code = "unknown_method";
      error.details = METHODS;
      throw error;
    }
    return { id, ok: true, result: handler(request.params ?? {}) };
  } catch (error) {
    return {
      id,
      ok: false,
      error: {
        message: error && error.message ? error.message : String(error),
        code: (error && error.code) || "internal",
        ...(error && error.details ? { details: error.details } : {}),
      },
    };
  }
}

/** Parses and handles one raw line. Blank lines are ignored by the caller. */
export function handleLine(line) {
  let parsed;
  try {
    parsed = JSON.parse(line);
  } catch (error) {
    return { id: null, ok: false, error: { message: `invalid JSON: ${error.message}`, code: "parse_error" } };
  }
  return handleRequest(parsed);
}

/**
 * Runs the sidecar loop on the given streams. Kept separate from the CLI entry
 * point so it can be driven directly in tests.
 */
export function runRpcLoop({ input, output, onExit } = {}) {
  const stdin = input ?? process.stdin;
  const stdout = output ?? process.stdout;

  stdout.write(
    JSON.stringify({ id: null, ok: true, event: "ready", result: describe() }) + "\n"
  );

  let buffer = "";
  stdin.setEncoding("utf8");

  stdin.on("data", (chunk) => {
    buffer += chunk;
    let newline;
    while ((newline = buffer.indexOf("\n")) !== -1) {
      const line = buffer.slice(0, newline).trim();
      buffer = buffer.slice(newline + 1);
      if (!line) continue;
      stdout.write(JSON.stringify(handleLine(line)) + "\n");
    }
  });

  stdin.on("end", () => {
    if (buffer.trim()) stdout.write(JSON.stringify(handleLine(buffer.trim())) + "\n");
    if (onExit) onExit();
  });

  return () => stdin.destroy();
}

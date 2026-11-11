/**
 * Proves the single-file bundle is equivalent to the source CLI.
 *
 *   node scripts/smoke-bundle.mjs
 *
 * Compares stdout, stderr, exit code and any report file written, across the
 * fixture corpus and the informational flags. The bundle is only shippable if
 * every case matches byte for byte — a bundler that silently drops a module or
 * reorders an initialiser produces plausible-looking but wrong findings, which
 * is exactly the failure this guards against.
 */
import { execFileSync } from "node:child_process";
import { readFileSync, existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const BUNDLE = "dist/baulens.cjs";
const SOURCE = "bin/baucheck.js";

if (!existsSync(BUNDLE)) {
  console.error(`FAILED: ${BUNDLE} not found. Run: node scripts/bundle.js`);
  process.exit(1);
}

const FIXTURES = [
  "good_spec.json",
  "bad_spec.json",
  "real_efh_stuttgart_lbo.json",
  "real_mfh_muenchen_baybo.json",
  "real_gewerbe_koeln_bauonrw.json",
  "real_hochhaus_berlin_bauobln.json",
];

/** Runs a CLI and captures everything observable about the run. */
function run(entry, args, cwd) {
  try {
    const stdout = execFileSync(process.execPath, [entry, ...args], {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { code: 0, stdout, stderr: "" };
  } catch (error) {
    return {
      code: typeof error.status === "number" ? error.status : 1,
      stdout: error.stdout ? error.stdout.toString() : "",
      stderr: error.stderr ? error.stderr.toString() : "",
    };
  }
}

const cases = [
  ...FIXTURES.filter((f) => existsSync(join("test", "fixtures", f))).map((f) => ({
    name: `check ${f}`,
    args: ["check", join("test", "fixtures", f)],
  })),
  ...FIXTURES.filter((f) => existsSync(join("test", "fixtures", f))).map((f) => ({
    name: `check ${f} --dach`,
    args: ["check", join("test", "fixtures", f), "--dach"],
  })),
  { name: "--lands", args: ["--lands"] },
  { name: "--staedte", args: ["--staedte"] },
  { name: "--example", args: ["--example"] },
  { name: "--example-bad", args: ["--example-bad"] },
  { name: "--help", args: ["--help"] },
  { name: "unknown flag", args: ["--definitely-not-a-flag"] },
];

let failures = 0;
let checked = 0;

console.log("bundled vs source CLI\n");

for (const testCase of cases) {
  const a = run(SOURCE, testCase.args, process.cwd());
  const b = run(BUNDLE, testCase.args, process.cwd());
  checked++;

  const same =
    a.code === b.code && a.stdout === b.stdout && a.stderr === b.stderr;

  if (same) {
    console.log(`  ok   ${testCase.name.padEnd(42)} exit=${a.code}`);
    continue;
  }

  failures++;
  console.log(`  FAIL ${testCase.name}`);
  if (a.code !== b.code) console.log(`       exit: source=${a.code} bundle=${b.code}`);
  if (a.stdout !== b.stdout) {
    const sa = a.stdout.split("\n"), sb = b.stdout.split("\n");
    for (let i = 0; i < Math.max(sa.length, sb.length); i++) {
      if (sa[i] !== sb[i]) {
        console.log(`       stdout first diff at line ${i + 1}`);
        console.log(`         source: ${JSON.stringify(sa[i])}`);
        console.log(`         bundle: ${JSON.stringify(sb[i])}`);
        break;
      }
    }
  }
  if (a.stderr !== b.stderr) console.log(`       stderr differs`);
}

/* Report generation: the dossier hash must match, since it is a customer-facing
   integrity seal. Separate runs, separate output files. */
const work = mkdtempSync(join(tmpdir(), "baulens-smoke-"));
try {
  const fixture = join("test", "fixtures", "good_spec.json");
  if (existsSync(fixture)) {
    const srcReport = join(work, "source.md");
    const bndReport = join(work, "bundle.md");

    const s = run(SOURCE, ["check", fixture, "--report", srcReport], process.cwd());
    const b = run(BUNDLE, ["check", fixture, "--report", bndReport], process.cwd());
    checked++;

    const sText = existsSync(srcReport) ? readFileSync(srcReport, "utf8") : "";
    const bText = existsSync(bndReport) ? readFileSync(bndReport, "utf8") : "";
    const same = s.code === b.code && sText === bText && sText.length > 0;

    if (same) {
      console.log(`  ok   ${"--report dossier".padEnd(42)} ${sText.length} chars identical`);
    } else {
      failures++;
      console.log(`  FAIL --report dossier`);
      if (!sText.length) console.log("       source wrote no report");
      if (sText !== bText) {
        const sa = sText.split("\n"), sb = bText.split("\n");
        for (let i = 0; i < Math.max(sa.length, sb.length); i++) {
          if (sa[i] !== sb[i]) {
            console.log(`       first diff at line ${i + 1}`);
            console.log(`         source: ${JSON.stringify(sa[i])}`);
            console.log(`         bundle: ${JSON.stringify(sb[i])}`);
            break;
          }
        }
      }
    }
  }
} finally {
  rmSync(work, { recursive: true, force: true });
}

/*
 * The RPC surface, which the desktop app uses instead of the CLI.
 *
 * This section exists because of a real defect: the bundler bound node builtins
 * from a hand-maintained member list, `extname` was not on it, and the bundle
 * threw "extname is not defined" for `check_file` -- the method the shell calls
 * for every plan file a user opens. Every CLI case above still passed, because
 * the CLI never ingested a file, so the break would have shipped. An artifact is
 * only verified where it is actually used.
 */
/* The workspace export writes files, so the source run and the bundle run have to
   aim at the same target: the answer names every written path, and a fresh temp
   directory per run would make the comparison fail for a reason that has nothing
   to do with the bundle. Both runs write the same bytes, so the second one simply
   overwrites the first. */
const planOut = mkdtempSync(join(tmpdir(), "baulens-smoke-plans-"));

const rpcRequests = [
  { id: 1, method: "ping" },
  { id: 2, method: "describe" },
  { id: 3, method: "check_file", params: { path: join("test", "fixtures", "bad_spec.json") } },
  { id: 4, method: "check_file", params: { path: join("test", "fixtures", "definitely-missing.ifc") } },
  {
    id: 5,
    method: "dossier_pdf",
    params: { file: join("test", "fixtures", "good_spec.json"), datum: "02.10.2026" },
  },
  /* Die beiden Blattformate gehen denselben Weg wie das PDF: der Desktop rendert
     sie aus dem gebauten Bündel, also muss das Bündel sie auch beantworten -
     eine korrekt gebundelte `plan_svg`-Methode ohne ihre beiden Schreiber wäre
     genau der Fehler, den dieser Fall schon einmal gefunden hat. */
  {
    id: 6,
    method: "plan_svg",
    params: { file: join("test", "fixtures", "good_spec.json"), datum: "02.10.2026" },
  },
  {
    id: 7,
    method: "plan_dxf",
    params: { file: join("test", "fixtures", "good_spec.json"), datum: "02.10.2026" },
  },
  /* Der Stapel-Export ist der Weg, den die Desktop-Schaltfläche nimmt: eine
     Liste von Vorhaben, ein Zielordner. Er muss im gebündelten Artefakt ebenso
     arbeiten wie in der Quelle - inklusive des Teils, in dem ein unlesbarer Plan
     die anderen nicht mitreißt. */
  {
    id: 8,
    method: "plan_export",
    params: {
      outDir: planOut,
      formats: ["dxf", "svg"],
      datum: "02.10.2026",
      files: [
        { path: join("test", "fixtures", "good_spec.json"), label: "Wohnanlage Musterweg" },
        { path: join("test", "fixtures", "bad_spec.json") },
        { path: join("test", "fixtures", "definitely-missing.ifc"), label: "Fehlt" },
      ],
    },
  },
];

/** Feeds request lines to an entry point over `--rpc` and parses the answers. */
function runRpc(entry) {
  const input = rpcRequests.map((request) => JSON.stringify(request)).join("\n") + "\n";
  try {
    const stdout = execFileSync(process.execPath, [entry, "--rpc"], {
      input,
      encoding: "utf8",
      stdio: ["pipe", "pipe", "pipe"],
      timeout: 120000,
    });
    return { code: 0, messages: stdout.trim().split("\n").filter(Boolean).map((line) => JSON.parse(line)) };
  } catch (error) {
    return { code: typeof error.status === "number" ? error.status : 1, messages: [], error };
  }
}

if (existsSync(BUNDLE)) {
  const sourceRpc = runRpc(SOURCE);
  const bundleRpc = runRpc(BUNDLE);
  checked++;

  const sourceAnswers = sourceRpc.messages.filter((m) => m.id !== null);
  const bundleAnswers = bundleRpc.messages.filter((m) => m.id !== null);
  const problems = [];

  if (sourceRpc.messages.length === 0) problems.push("the source engine answered nothing, so this case proves nothing");
  if (JSON.stringify(sourceAnswers) !== JSON.stringify(bundleAnswers)) {
    problems.push("the bundle answers differently from the source engine");
    for (let i = 0; i < Math.max(sourceAnswers.length, bundleAnswers.length); i++) {
      if (JSON.stringify(sourceAnswers[i]) !== JSON.stringify(bundleAnswers[i])) {
        problems.push(`  id ${rpcRequests[i] ? rpcRequests[i].method : i}: source ${JSON.stringify(sourceAnswers[i]).slice(0, 160)}`);
        problems.push(`  id ${rpcRequests[i] ? rpcRequests[i].method : i}: bundle ${JSON.stringify(bundleAnswers[i]).slice(0, 160)}`);
        break;
      }
    }
  }

  /* Behaviour, not just agreement: two identically broken engines would match. */
  const byId = new Map(bundleAnswers.map((m) => [m.id, m]));
  const expect = (id, predicate, description) => {
    const answer = byId.get(id);
    if (!answer) problems.push(`${description}: no answer for id ${id}`);
    else if (!predicate(answer)) problems.push(`${description}: ${JSON.stringify(answer).slice(0, 200)}`);
  };

  expect(1, (a) => a.ok === true && a.result.pong === true, "ping must answer");
  expect(
    2,
    (a) =>
      a.ok === true &&
      ["dossier_pdf", "plan_svg", "plan_dxf"].every((method) => (a.result.methods ?? []).includes(method)),
    "describe must advertise the dossier and both Blattformate"
  );
  expect(
    3,
    (a) => a.ok === true && typeof a.result.verdict === "string" && Array.isArray(a.result.findings),
    "check_file must ingest a plan file (the `extname` regression)"
  );
  expect(4, (a) => a.ok === false && a.error.code === "file_unreadable", "a missing file must be reported, not crash the engine");
  expect(
    5,
    (a) =>
      a.ok === true &&
      a.result.pages >= 1 &&
      Buffer.from(a.result.base64 ?? "", "base64").subarray(0, 8).toString("latin1") === "%PDF-1.4",
    "dossier_pdf must return a real PDF"
  );
  expect(
    6,
    (a) => typeof a.result.svg === "string" && a.result.svg.startsWith("<?xml version=\"1.0\"") && a.result.scale === "1:250",
    "plan_svg must return the drawing sheet as SVG, with its Maßstab"
  );
  expect(
    7,
    (a) => typeof a.result.dxf === "string" && a.result.dxf.startsWith("999\r\n") && a.result.dxf.includes("$ACADVER\r\n1\r\nAC1015"),
    "plan_dxf must return the drawing sheet as a DXF in metres"
  );
  expect(
    8,
    (a) =>
      a.ok === true &&
      a.result.counts.plans === 3 &&
      a.result.counts.written === 4 &&
      a.result.counts.drawn === 2 &&
      a.result.counts.failed === 1 &&
      a.result.failed[0].code === "file_unreadable" &&
      a.result.written.every((entry) => entry.hash.length === 64 && entry.scale === "1:250"),
    "plan_export must write a Blatt per Vorhaben and let one broken plan fail alone"
  );

  if (problems.length) {
    failures++;
    console.log(`  FAIL ${`--rpc ${rpcRequests.length} requests`.padEnd(42)}`);
    for (const problem of problems) console.log(`       ${problem}`);
  } else {
    console.log(`  ok   ${`--rpc ${rpcRequests.length} requests`.padEnd(42)} answers identical to the source engine`);
  }

  rmSync(planOut, { recursive: true, force: true });
}

console.log(`\n${failures === 0 ? `BUNDLE VERIFIED (${checked} cases)` : `${failures} of ${checked} cases FAILED`}`);
if (failures > 0) process.exit(1);
process.exit(failures === 0 ? 0 : 1);

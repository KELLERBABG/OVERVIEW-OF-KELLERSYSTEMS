/**
 * Builds the engine the desktop app runs.
 *
 *   node scripts/build-desktop-sidecar.mjs
 *
 * The shell cannot execute JavaScript, and the alternative -- reimplementing the
 * rules in Rust -- is the drift this project has already been bitten by. So the
 * tested single-file build (dist/baulens.cjs, proven byte-identical to the
 * source CLI by scripts/smoke-bundle.mjs) is what ships.
 *
 * Two shapes matter:
 *   - the .cjs script, staged as a Tauri resource and run through Node. This is
 *     what a normal build produces, and it is the same artifact the GitHub
 *     Release already publishes, so there is one engine build, not two.
 *   - an optional single-executable, written next to the app binary if
 *     node --experimental-sea-config succeeds. A hook is provided but not run
 *     automatically, because a binary that has not been exercised is worse than
 *     no binary: the fallback through Node is known to work.
 */
import { readFileSync, writeFileSync, copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { join } from "node:path";

const BUNDLE = "dist/baulens.cjs";
const STAGE = "desktop/src-tauri/resources";

function fail(msg) {
  console.error(`FAILED: ${msg}`);
  process.exit(1);
}

/* Always rebuild, so the sidecar cannot silently lag the sources. */
execFileSync(process.execPath, ["scripts/bundle.js"], { stdio: ["ignore", "ignore", "inherit"] });

if (!existsSync(BUNDLE)) fail(`${BUNDLE} was not produced`);

const size = statSync(BUNDLE).size;
if (size < 50 * 1024) fail(`${BUNDLE} is only ${size} bytes; the bundle looks wrong`);

/* The engine must answer before it is shipped. */
const handshake = execFileSync(process.execPath, [BUNDLE, "--rpc"], {
  input: '{"id":1,"method":"ping"}\n',
  encoding: "utf8",
  timeout: 30000,
});

const lines = handshake.trim().split("\n").map((line) => JSON.parse(line));
const ready = lines.find((l) => l.event === "ready");
const pong = lines.find((l) => l.id === 1);

if (!ready) fail("the engine did not announce itself");
if (!pong || pong.ok !== true) fail("the engine did not answer ping");

console.log(`engine build verified`);
console.log(`  product : ${ready.result.product} ${ready.result.version}`);
console.log(`  modules : ${ready.result.counts.total}`);

/* Stage the resource the shell looks for. */
mkdirSync(STAGE, { recursive: true });
const staged = join(STAGE, "baulens.cjs");
copyFileSync(BUNDLE, staged);
console.log(`  staged  : ${staged} (${(statSync(staged).size / 1024).toFixed(0)} KiB)`);

/* --------------------------------------------------------------------------
 * Optional single-executable, for a fully self-contained installer.
 *
 * Not on by default. The Node SEA step is the fragile part of the release
 * pipeline, and a binary that has never been launched is not evidence that it
 * works -- so this only reports, it does not promote anything.
 * ------------------------------------------------------------------------ */
if (process.argv.includes("--sea")) {
  const name = process.platform === "win32" ? "baulens-sidecar.exe" : "baulens-sidecar";
  const target = join(STAGE, name);

  const config = join(STAGE, "sea-config.json");
  writeFileSync(
    config,
    JSON.stringify({ main: BUNDLE, output: join(STAGE, "sea-prep.blob"), disableExperimentalSEAWarning: true }),
    "utf8"
  );

  try {
    execFileSync(process.execPath, ["--experimental-sea-config", config], { stdio: "inherit" });
    copyFileSync(process.execPath, target);
    execFileSync(
      "npx",
      ["--yes", "postject", target, "NODE_SEA_BLOB", join(STAGE, "sea-prep.blob"),
       "--sentinel-fuse", "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2"],
      { stdio: "inherit", shell: process.platform === "win32" }
    );

    /* Prove it runs and reports the released version before claiming success. */
    const out = execFileSync(target, ["--help"], { encoding: "utf8", timeout: 60000 });
    const version = JSON.parse(readFileSync("package.json", "utf8")).version;
    if (!out.includes(`v${version}`)) fail(`${target} does not report v${version}`);
    console.log(`  binary  : ${target} (verified, reports v${version})`);
  } catch (error) {
    /* Not fatal: the Node fallback is the supported path. */
    console.warn(`  binary  : skipped (${error.message.split("\n")[0]})`);
    console.warn("            the app will use the staged baulens.cjs through Node instead");
  }
}

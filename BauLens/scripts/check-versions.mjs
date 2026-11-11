/**
 * Asserts every declared version matches the root package.json.
 *
 *   node scripts/check-versions.mjs
 *   node scripts/check-versions.mjs --sync   also rewrite the desktop copies
 *
 * The version is declared in five places because four different tools read it:
 * npm reads package.json, cargo reads Cargo.toml, tauri.conf.json can point at
 * package.json, and site.config.json feeds the generated pages. That is exactly
 * the shape of drift this project has already suffered from three times, so it
 * is checked rather than trusted.
 */
import { readFileSync, writeFileSync } from "node:fs";

const root = JSON.parse(readFileSync("package.json", "utf8")).version;
const sync = process.argv.includes("--sync");

const targets = [];

function jsonVersion(path, label, { nested } = {}) {
  const text = readFileSync(path, "utf8");
  targets.push({
    label,
    path,
    read: () => {
      const parsed = JSON.parse(text);
      return nested ? parsed[nested]?.version : parsed.version;
    },
    write: (v) => {
      /* site.config.json keeps the version under "product", so the match is
         anchored on the surrounding key rather than assuming a top-level one. */
      const pattern = nested
        ? new RegExp(`("${nested}"\\s*:\\s*\\{[\\s\\S]*?"version"\\s*:\\s*")[^"]*(")`)
        : /("version"\s*:\s*")[^"]*(")/;
      const updated = text.replace(pattern, `$1${v}$2`);
      if (updated === text) throw new Error(`no version field in ${path}`);
      writeFileSync(path, updated, "utf8");
    },
  });
}

jsonVersion("site.config.json", "site.config.json (generated pages)", { nested: "product" });
jsonVersion("desktop/package.json", "desktop/package.json");
tomlVersion("desktop/src-tauri/Cargo.toml", "desktop/src-tauri/Cargo.toml");

function tomlVersion(path, label) {
  const text = readFileSync(path, "utf8");
  targets.push({
    label,
    path,
    read: () => {
      const match = /^version\s*=\s*"([^"]+)"/m.exec(text);
      if (!match) throw new Error(`no version in ${path}`);
      return match[1];
    },
    write: (v) => {
      const updated = text.replace(/^version\s*=\s*"[^"]+"/m, `version = "${v}"`);
      writeFileSync(path, updated, "utf8");
    },
  });
}

/* tauri.conf.json is supposed to reference the root package.json rather than
   repeat the number, so it is checked separately from the literal targets. */
const tauriConfig = JSON.parse(readFileSync("desktop/src-tauri/tauri.conf.json", "utf8"));
const tauriVersion = tauriConfig.version;

console.log(`root package.json : ${root}\n`);

let problems = 0;
for (const target of targets) {
  let actual;
  try {
    actual = target.read();
  } catch (error) {
    console.log(`  FAIL  ${target.label}: ${error.message}`);
    problems++;
    continue;
  }

  if (actual === root) {
    console.log(`  ok    ${target.label.padEnd(38)} ${actual}`);
    continue;
  }

  if (sync) {
    target.write(root);
    console.log(`  fixed ${target.label.padEnd(38)} ${actual} -> ${root}`);
    continue;
  }

  console.log(`  FAIL  ${target.label.padEnd(38)} ${actual} (expected ${root})`);
  problems++;
}

/* tauri.conf.json may either reference package.json or repeat the number; a
   reference is preferred, a stale literal is a failure. */
const TAURI_REFERENCE = "../../package.json";
if (tauriVersion === TAURI_REFERENCE || tauriVersion === root) {
  const form = tauriVersion === TAURI_REFERENCE ? `reference to ${TAURI_REFERENCE}` : tauriVersion;
  console.log(`  ok    ${"tauri.conf.json".padEnd(38)} ${form}`);
} else {
  console.log(`  FAIL  ${"tauri.conf.json".padEnd(38)} ${tauriVersion} (expected ${root} or the reference)`);
  problems++;
}

console.log(
  problems === 0
    ? "\nVERSIONS CONSISTENT"
    : `\n${problems} version mismatch(es). Run: node scripts/check-versions.mjs --sync`
);
process.exit(problems === 0 ? 0 : 1);

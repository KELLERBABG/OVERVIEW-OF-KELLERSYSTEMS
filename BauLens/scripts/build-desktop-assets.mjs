/**
 * Stages the web assets the desktop app serves.
 *
 *   node scripts/build-desktop-assets.mjs
 *
 * Tauri serves exactly one frontendDist directory, and the pages sit at the
 * repository root next to assets/, legal/, pricing/ and downloads/. Pointing
 * frontendDist at the repository root would hand the webview the entire source
 * tree, including node_modules and src-tauri. Instead this copies only what the
 * app renders into desktop/app/, which is what tauri.conf.json points at.
 *
 * Two shapes matter:
 *   - index.html is the workhorse workspace (desktop/workhorse.html): the app
 *     opens the multi-project view, not the marketing page.
 *   - studio.html and docs.html come across unchanged, because the workhorse
 *     and the engine view share the same tested rule engine and the docs are
 *     worth having offline.
 *
 * The copies are byte-for-byte except for the desktop injection, which is added
 * to the staged files only.
 */
import { cpSync, mkdirSync, rmSync, existsSync, statSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { INJECT_PARTS, injectScript as assembleInjectScript } from "../desktop/inject/sources.mjs";

const OUT = "desktop/app";

/* Pages, plus the directories they reference at runtime. */
const FILES = ["studio.html", "docs.html"];
const DIRS = ["assets", "legal", "pricing", "checkout", "downloads"];

/* Staged as index.html: the workspace the window opens. */
const WORKHORSE = "desktop/workhorse.html";

/* Desktop-only behaviour, inlined into the staged copies below. The parts and
   their order come from desktop/inject/sources.mjs — the same list the test
   harness assembles from, so staged script and tested script cannot diverge. */
const INJECTED = ["studio.html", "index.html"];

/* The licensing core (src/licensing.js) is an ES module for the test suite and
   a plain script for the webview. This transform strips its `export` keywords
   and exposes the declared names as window.BauLensLicensing — one source for
   the entitlement rules, which is the whole point of having them tested. */
const LICENSING_MODULE = "src/licensing.js";

function licensingScript() {
  const source = readFileSync(LICENSING_MODULE, "utf8");
  const names = [];
  const stripped = source.replace(
    /^export (?:async )?(?:const|function) (\w+)/gm,
    (match, name) => {
      names.push(name);
      return match.slice("export ".length);
    }
  );
  if (/^export /m.test(stripped) || /^import /m.test(stripped)) {
    console.error(`FAILED: ${LICENSING_MODULE} carries an export form the transform does not understand`);
    process.exit(1);
  }
  if (!names.includes("entitlement") || !names.includes("validateLicenseKey")) {
    console.error(`FAILED: ${LICENSING_MODULE} no longer declares the names the desktop UI relies on`);
    process.exit(1);
  }
  return (
    "window.BauLensLicensing = (function () {\n\"use strict\";\n" +
    stripped +
    "\nreturn { " + names.join(", ") + " };\n})();"
  );
}

/* The paywall points at the same URLs the website uses, read once from
   site.config.json so the app and the site can never disagree. checkoutUrl is
   null until the Polar products exist; the UI then falls back to the pricing
   page, exactly like the buy buttons on the website. */
function licensingConfig() {
  const site = JSON.parse(readFileSync("site.config.json", "utf8"));
  const checkout = site.tiers.map((t) => t.checkoutUrl).find(Boolean) || null;
  return {
    trialDays: site.billing.trialDays,
    organizationId: site.billing.organizationId,
    checkoutUrl: checkout,
    pricingUrl: site.product.siteUrl + "pricing/",
    siteUrl: site.product.siteUrl,
  };
}

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });

let copied = 0;
let bytes = 0;

function copy(rel, to = rel) {
  const from = rel;
  const target = join(OUT, to);
  if (!existsSync(from)) {
    console.error(`FAILED: ${rel} is missing`);
    process.exit(1);
  }
  cpSync(from, target, { recursive: true });
  const stat = statSync(from);
  if (stat.isFile()) {
    copied++;
    bytes += stat.size;
  } else {
    const walk = (dir) => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const path = join(dir, entry.name);
        if (entry.isDirectory()) walk(path);
        else { copied++; bytes += statSync(path).size; }
      }
    };
    walk(from);
  }
}

for (const file of FILES) copy(file);
for (const dir of DIRS) copy(dir);
copy(WORKHORSE, "index.html");

/* The app ships no README, no CHANGELOG and no test fixtures; those live in the
   repository, not in the installer. */

function injectScript(page) {
  const target = join(OUT, page);
  const html = readFileSync(target, "utf8");

  if (html.includes("__baulensDesktopInjected")) {
    console.error(`FAILED: ${page} already carries the desktop injection`);
    process.exit(1);
  }

  /* Splice at the document end, not at the first `</body>` in the file:
     studio.html embeds a whole HTML document in a template literal for the
     dossier export, and injecting there would place the script inside that
     string — the `</script>` would truncate the page's own script element
     mid-template, killing both the page code and this injection. */
  const at = html.lastIndexOf("</body>");
  if (at === -1) {
    console.error(`FAILED: could not find </body> in ${page}`);
    process.exit(1);
  }
  const injected =
    html.slice(0, at) +
    `<!-- __baulensDesktopInjected -->\n<script>\n` +
    `window.BauLensLicensingConfig = ${JSON.stringify(licenseConfig)};\n` +
    `${licenseModule}\n${desktopScript}\n</script>\n` +
    html.slice(at);
  writeFileSync(target, injected, "utf8");
}

const desktopScript = assembleInjectScript();
const licenseModule = licensingScript();
const licenseConfig = licensingConfig();
for (const page of INJECTED) injectScript(page);

console.log(`staged the desktop frontend -> ${OUT}`);
console.log(`  files : ${copied}`);
console.log(`  size  : ${(bytes / 1024 / 1024).toFixed(1)} MiB`);
console.log(`  front : index.html from ${WORKHORSE}`);
console.log(`  script: ${INJECT_PARTS.join(" + ")} inlined into ${INJECTED.join(", ")}`);
console.log(`  license: ${LICENSING_MODULE} + site.config.json (trial ${licenseConfig.trialDays} days)`);

/* A page that lost its fonts would render in a fallback face, so fail loudly. */
for (const page of [...FILES, "index.html"]) {
  const html = readFileSync(join(OUT, page), "utf8");
  if (!html.includes("@font-face")) {
    console.error(`FAILED: ${page} carries no inlined @font-face`);
    process.exit(1);
  }
  if (/fonts\.(googleapis|gstatic)\.com/.test(html)) {
    console.error(`FAILED: ${page} still references a font CDN`);
    process.exit(1);
  }
}
console.log("  verified: every page carries its fonts and names no CDN");

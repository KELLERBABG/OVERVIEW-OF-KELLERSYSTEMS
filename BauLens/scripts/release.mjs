/**
 * Cuts a release.
 *
 *   node scripts/release.mjs 0.7.0          full run
 *   node scripts/release.mjs 0.7.0 --dry-run  show what would change
 *
 * Performs the whole local half of a release in one deterministic pass:
 *   1. bump package.json            (the single source of the version)
 *   2. promote CHANGELOG [Unveröffentlicht] to a dated section, leaving an
 *      empty one behind for the next cycle
 *   3. run the release gates: tests, docs payload, links, prices, bundle smoke
 *   4. commit, then create an annotated tag
 *
 * Pushing the tag is what starts the release build. That is left to you, so a
 * failed gate never publishes anything.
 */
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";

const DOCS = ["README.md", "CHANGELOG.md", "docs/RULES.md", "docs/WHITEPAPER.md", "docs/PAYMENTS_AND_LEGAL.md"];

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const skipGates = args.includes("--skip-gates");
const version = args.find((a) => !a.startsWith("--"));

function die(msg) {
  console.error(`FAILED: ${msg}`);
  process.exit(1);
}

if (!version) die("usage: node scripts/release.mjs <version> [--dry-run] [--skip-gates]");
if (!/^\d+\.\d+\.\d+$/.test(version)) die(`"${version}" is not a plain semver version (expected 1.2.3)`);

const tag = `v${version}`;
const today = new Date().toISOString().slice(0, 10);

/* --------------------------------------------------------------------------
 * 1. package.json
 * ------------------------------------------------------------------------ */

const pkgRaw = readFileSync("package.json", "utf8");
const pkg = JSON.parse(pkgRaw);
const previous = pkg.version;

if (previous === version) die(`package.json is already at ${version}`);
if (existsSync(".git")) {
  const tags = execFileSync("git", ["tag", "--list", tag], { encoding: "utf8" }).trim();
  if (tags) die(`tag ${tag} already exists`);
}

const bumped = pkgRaw.replace(
  new RegExp(`("version"\\s*:\\s*)"${previous.replace(/\./g, "\\.")}"`),
  `$1"${version}"`
);
if (bumped === pkgRaw) die("could not rewrite the version in package.json");

/* site.config.json carries the version that gets stamped into the pages and
   into studio.html's generated documents, so it moves with package.json. */
const siteRaw = readFileSync("site.config.json", "utf8");
const siteBumped = siteRaw.replace(
  new RegExp(`("version"\\s*:\\s*)"${previous.replace(/\./g, "\\.")}"`),
  `$1"${version}"`
);
if (siteBumped === siteRaw) {
  die(`site.config.json does not declare version ${previous}; refusing to leave it stale`);
}

/* --------------------------------------------------------------------------
 * 2. CHANGELOG
 * ------------------------------------------------------------------------ */

const changelog = readFileSync("CHANGELOG.md", "utf8");
const unreleased = /^##\s*\[?Unveröffentlicht\]?\s*$/m.exec(changelog);

if (!unreleased) {
  die(
    "CHANGELOG.md has no '## [Unveröffentlicht]' section.\n" +
    "       Nothing would be documented for this release, so it is not cut."
  );
}

/* Anything between the heading and the next '## ' heading is this release. */
const afterHeading = unreleased.index + unreleased[0].length;
const nextSection = changelog.slice(afterHeading).search(/^##\s/m);
const bodyEnd = nextSection === -1 ? changelog.length : afterHeading + nextSection;
const body = changelog.slice(afterHeading, bodyEnd);

if (!/###\s/.test(body) || body.replace(/[\s#]/g, "").length < 20) {
  die("the [Unveröffentlicht] section has no entries; refusing to cut an empty release");
}

const promoted = `## [Unveröffentlicht]

_Nichts ausstehend._

## [${version}] — ${today}

${body.trim()}`;

let newChangelog = changelog.slice(0, unreleased.index) + promoted + changelog.slice(bodyEnd);

/* Keep the compare links at the foot of the file in step. */
if (/^\[Unveröffentlicht\]:/m.test(newChangelog)) {
  newChangelog = newChangelog.replace(
    /^\[Unveröffentlicht\]:.*$/m,
    `[Unveröffentlicht]: https://github.com/KELLERBABG/BauLens/compare/${tag}...HEAD`
  );
}
const linkLine = `[${version}]: https://github.com/KELLERBABG/BauLens/releases/tag/${tag}`;
if (!newChangelog.includes(linkLine)) {
  newChangelog = newChangelog.trimEnd() + `\n${linkLine}\n`;
}

/* --------------------------------------------------------------------------
 * Report / apply
 * ------------------------------------------------------------------------ */

console.log(`release ${tag}`);
console.log(`  version    : ${previous} -> ${version}`);
console.log(`  date       : ${today}`);
console.log(`  changelog  : ${body.trim().split("\n").length} lines promoted`);

const touched = ["package.json", "site.config.json", "CHANGELOG.md"];
for (const extra of DOCS) {
  if (touched.includes(extra)) continue;
  if (readFileSync(extra, "utf8").includes(previous)) {
    console.log(`  note       : ${extra} still mentions ${previous}`);
  }
}

if (dryRun) {
  console.log("\n--dry-run: nothing written.");
  console.log("Would write: " + touched.join(", "));
  process.exit(0);
}

writeFileSync("package.json", bumped, "utf8");
writeFileSync("site.config.json", siteBumped, "utf8");
writeFileSync("CHANGELOG.md", newChangelog, "utf8");
console.log(`\n  wrote ${touched.join(", ")}`);

/* --------------------------------------------------------------------------
 * 3. Gates
 * ------------------------------------------------------------------------ */

if (skipGates) {
  console.log("\n--skip-gates: gates not run.");
} else {
  const gates = [
    ["tests", ["--test"]],
    /* build-docs first: promoting the changelog changes CHANGELOG.md, and
       docs.html embeds a copy of it, so verifying before regenerating would
       always report a stale payload. */
    ["docs build", ["scripts/build-docs.mjs"]],
    ["docs payload", ["scripts/verify-docs.mjs"]],
    ["site build", ["scripts/build-site.mjs"]],
    ["links", ["scripts/check-links.mjs"]],
    ["prices", ["scripts/check-prices.mjs"]],
    ["bundle", ["scripts/bundle.js"]],
    ["bundle equivalence", ["scripts/smoke-bundle.mjs"]],
  ];

  console.log("\ngates:");
  for (const [name, scriptArgs] of gates) {
    process.stdout.write(`  ${name.padEnd(20)} `);
    try {
      execFileSync(process.execPath, scriptArgs, { stdio: ["ignore", "pipe", "pipe"], encoding: "utf8" });
      console.log("ok");
    } catch (error) {
      console.log("FAILED");
      const out = `${error.stdout || ""}${error.stderr || ""}`.trim().split("\n").slice(-12).join("\n");
      console.error(out);
      die(`gate "${name}" failed; release not committed. Revert the version bump or fix and re-run.`);
    }
  }
}

/* --------------------------------------------------------------------------
 * 4. Commit and tag
 * ------------------------------------------------------------------------ */

/* The generated pages carry the version as well: the legal pages print it and
   studio.html stamps it into the XBau namensraum and the dossier footer. The
   `site build` gate above rewrites them, so they belong in the release commit.
   They are listed explicitly rather than staged with a blanket `git add -A`. */
const GENERATED = [
  "index.html",
  "studio.html",
  "docs.html",
  "_redirects",
  "pricing/index.html",
  "downloads/index.html",
  "checkout/success/index.html",
  "legal/impressum/index.html",
  "legal/datenschutz/index.html",
  "legal/agb/index.html",
  "legal/lizenz/index.html",
  "legal/widerruf/index.html",
];

if (!existsSync(".git")) {
  console.log("\nnot a git repository: skipping commit and tag.");
  process.exit(0);
}

const subject = `chore(release): ${tag}`;
const message = `${subject}

Promotes the CHANGELOG [Unveröffentlicht] section to ${version}, bumps
package.json and site.config.json, and regenerates the pages so every printed
version matches. Tagging starts the release build, which produces the
single-file bundle and the standalone binaries.
`;

const stageable = [...new Set([...GENERATED, "package.json", "site.config.json", "CHANGELOG.md"])]
  .filter((f) => existsSync(f));

try {
  execFileSync("git", ["add", "--", ...stageable], { stdio: "inherit" });

  /* Never tag a tree with unrelated edits picked up along the way. */
  const staged = execFileSync("git", ["diff", "--cached", "--name-only"], { encoding: "utf8" })
    .split("\n").map((l) => l.trim()).filter(Boolean);
  const unexpected = staged.filter((f) => !stageable.includes(f));
  if (unexpected.length) die(`unexpected files staged: ${unexpected.join(", ")}`);

  execFileSync("git", ["commit", "-m", message], { stdio: "inherit" });
  execFileSync("git", ["tag", "-a", tag, "-m", `BauLens ${version}`], { stdio: "inherit" });
} catch (error) {
  die(`git step failed: ${error.message}`);
}

console.log(`\n  committed and tagged ${tag}`);
console.log("\nnext:");
console.log(`  git push origin main && git push origin ${tag}`);
console.log("  the tag push runs the release workflow and publishes the artifacts");

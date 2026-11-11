/**
 * Extracts the release notes for the current package version from CHANGELOG.md.
 *
 *   node scripts/release-notes.mjs            write to stdout
 *   node scripts/release-notes.mjs out.md     write to a file
 *
 * The release workflow uses this so the GitHub Release body is the same text
 * that is already reviewed in the changelog, rather than a second copy that can
 * drift from it.
 */
import { readFileSync, writeFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const version = pkg.version;
const changelog = readFileSync("CHANGELOG.md", "utf8").replace(/\r\n/g, "\n");

/* Match "## [0.7.0] — 2026-10-02" or "## [0.7.0]" */
const heading = new RegExp(`^##\\s*\\[?${version.replace(/\./g, "\\.")}\\]?[^\\n]*$`, "m").exec(changelog);

if (!heading) {
  console.error(`FAILED: CHANGELOG.md has no section for ${version}`);
  process.exit(1);
}

const start = heading.index + heading[0].length;
const rest = changelog.slice(start);
const nextHeading = rest.search(/^##\s/m);
const section = (nextHeading === -1 ? rest : rest.slice(0, nextHeading)).trim();

if (!section) {
  console.error(`FAILED: the ${version} changelog section is empty`);
  process.exit(1);
}

const date = (/—\s*(\d{4}-\d{2}-\d{2})/.exec(heading[0]) || [])[1];
const header = `## BauLens v${version}${date ? ` — ${date}` : ""}

Deterministic DACH building-code pre-check engine. Pre-check only; not legal advice.

`;

/* Drop the trailing link-reference block, which is meaningless in a release body. */
const body = header + section.replace(/^\[[^\]]+\]:\s*\S+\s*$/gm, "").trim() + "\n";

const target = process.argv[2];
if (target) {
  writeFileSync(target, body, "utf8");
  console.error(`wrote ${target} (${body.length} chars) for v${version}`);
} else {
  process.stdout.write(body);
}

/**
 * Checks every internal link and asset reference in the generated pages.
 * Absolute-path links are flagged separately: they work on a web server but
 * break when a page is opened from disk, which is how this repo is previewed.
 */
import { readFileSync, existsSync } from "node:fs";
import { dirname, join, normalize } from "node:path";

const PAGES = [
  "index.html",
  "studio.html",
  "docs.html",
  "pricing/index.html",
  "downloads/index.html",
  "checkout/success/index.html",
  "legal/impressum/index.html",
  "legal/datenschutz/index.html",
  "legal/agb/index.html",
  "legal/lizenz/index.html",
  "legal/widerruf/index.html",
];

/* Targets that intentionally do not exist in the repo. */
const EXPECTED_EXTERNAL = [
  /^https?:/i,
  /^mailto:/i,
  /^tel:/i,
  /^#/,
];

let failures = 0;
let warnings = 0;

for (const page of PAGES) {
  if (!existsSync(page)) {
    console.error(`MISSING PAGE ${page}`);
    failures++;
    continue;
  }

  const html = readFileSync(page, "utf8");
  const dir = dirname(page);
  const refs = new Set();
  for (const m of html.matchAll(/(?:href|src)="([^"]+)"/g)) refs.add(m[1]);

  const problems = [];
  const absolute = [];

  for (const ref of refs) {
    if (!ref || EXPECTED_EXTERNAL.some((re) => re.test(ref))) continue;
    if (ref.startsWith("/")) { absolute.push(ref); continue; }

    const clean = ref.split("#")[0].split("?")[0];
    if (!clean) continue;

    /* A directory reference resolves to its index.html. */
    const target = normalize(join(dir, clean));
    const candidates = [target, join(target, "index.html")];
    if (!candidates.some((c) => existsSync(c))) problems.push(ref);
  }

  const label = page.padEnd(34);
  if (problems.length === 0 && absolute.length === 0) {
    console.log(`ok    ${label} ${refs.size} refs`);
  } else {
    console.log(`${problems.length ? "FAIL " : "warn "} ${label} ${refs.size} refs`);
    for (const p of problems) { console.log(`        broken: ${p}`); failures++; }
    for (const a of absolute) { console.log(`        server-only: ${a}`); warnings++; }
  }
}

console.log(`\n${failures === 0 ? "NO BROKEN LINKS" : failures + " BROKEN LINK(S)"}${warnings ? `, ${warnings} server-only reference(s)` : ""}`);
process.exit(failures === 0 ? 0 : 1);

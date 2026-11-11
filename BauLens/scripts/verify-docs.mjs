/**
 * Verifies that docs.html embeds a faithful copy of every source .md file.
 *
 * JSON.stringify is the only escaping applied, and JSON.parse reverses it, so
 * the embedded text must match the source exactly. Anything that differs is a
 * real corruption — this is the check that caught both the placeholder
 * substitution bug (CH_BS) and the double-escaped "<" in the embedded prose.
 */
import { readFileSync } from "node:fs";

const FILES = {
  readme: "README.md",
  rules: "docs/RULES.md",
  whitepaper: "docs/WHITEPAPER.md",
  changelog: "CHANGELOG.md",
};

const html = readFileSync("docs.html", "utf8");
const match = /<script type="application\/json" id="docSources">([\s\S]*?)<\/script>/.exec(html);
if (!match) { console.error("FAIL: docSources payload not found"); process.exit(1); }

const payload = JSON.parse(match[1]);
let failures = 0;

for (const [slug, file] of Object.entries(FILES)) {
  const source = readFileSync(file, "utf8").replace(/\r\n/g, "\n");
  const embedded = payload[slug];

  if (embedded === undefined) { console.error(`FAIL ${file}: not embedded`); failures++; continue; }

  if (embedded === source) {
    console.log(`ok   ${file.padEnd(20)} ${source.length} chars, identical`);
    continue;
  }

  failures++;
  console.error(`FAIL ${file}: embedded copy differs from source`);
  for (let i = 0; i < Math.max(embedded.length, source.length); i++) {
    if (embedded[i] !== source[i]) {
      const ctx = (s) => JSON.stringify(s.slice(Math.max(0, i - 45), i + 25));
      console.error(`  first diff at ${i}`);
      console.error(`  source:   ${ctx(source)}`);
      console.error(`  embedded: ${ctx(embedded)}`);
      break;
    }
  }
}

// Viewer script safety
const viewer = html.slice(html.indexOf("<script>") + 8, html.lastIndexOf("</script>"));
if (viewer.includes("__DOC_")) { console.error("FAIL: sentinel leaked into viewer"); failures++; }
if (!viewer.includes("var DOCS = [")) { console.error("FAIL: DOCS list not injected"); failures++; }

console.log(failures === 0 ? "\nEMBEDDED PAYLOAD VERIFIED" : `\n${failures} FAILURE(S)`);
process.exit(failures === 0 ? 0 : 1);

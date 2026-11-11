/**
 * Answers "what actually reaches the printed page?" by materialising the print
 * stylesheet.
 *
 *   node print-audit.mjs
 *
 * Rather than decoding a PDF (whose subset-font encodings make text extraction
 * unreliable), this rewrites studio.html so the @media print block applies
 * unconditionally, then asks the browser which elements are still visible. The
 * rewrite is written to the system temp directory and never touches the repo.
 */
import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { execFileSync } from "node:child_process";

const studio = readFileSync("studio.html", "utf8");

/* Pull out the print block and hoist it into an always-on style. */
const printBlock = /@media print \{([\s\S]*?)\n      \}/.exec(studio);
if (!printBlock) {
  console.error("FAILED: could not find the @media print block in studio.html");
  process.exit(1);
}

const forced = studio.replace(
  /@media print \{[\s\S]*?\n      \}/,
  `/* print block forced on for this audit */\n${printBlock[1]}`
);

/* The probe runs inside the page so it sees the forced rules. */
const probe = `
<script>
(function () {
  function vis(sel) {
    var el = document.querySelector(sel);
    if (!el) return "missing";
    var n = el;
    while (n && n !== document.body) {
      var cs = getComputedStyle(n);
      if (cs.display === "none" || cs.visibility === "hidden") return "hidden";
      n = n.parentElement;
    }
    return "VISIBLE";
  }
  var rows = [
    ["header bar", "header"],
    ["left input card", ".studio-layout > .card"],
    ["preset panel", ".preset-panel"],
    ["tab bar", ".tab-header"],
    ["KPI row", ".kpi-grid"],
    ["verdict banner", ".verdict-banner"],
    ["audit table", ".audit-table-wrap"],
    ["repair panel", "#viewRepair"],
    ["matrix panel", "#viewMatrix"],
    ["xbau panel", "#viewXbau"],
    ["legal drawer", "#legalDrawer"]
  ];
  var out = rows.map(function (r) { return r[0] + "=" + vis(r[1]); });

  /* The dossier is reported by its own computed display: it is concealed while
     another tab is active, so an ancestor walk proves nothing about whether the
     print rule restores it. */
  var cert = document.querySelector(".cert-container");
  out.push("dossier=" + (cert ? getComputedStyle(cert).display : "missing"));

  var el = document.createElement("div");
  el.id = "printAudit";
  el.textContent = out.join(" | ");
  document.body.appendChild(el);
})();
</script>`;

const dir = mkdtempSync(join(tmpdir(), "baulens-print-"));
const file = join(dir, "studio-print.html");
writeFileSync(file, forced + probe, "utf8");

const CHROME = process.env.CHROME_PATH ||
  "C:\\Users\\LNegenborn\\AppData\\Local\\Google\\Chrome\\Application\\chrome.exe";

let dom;
try {
  dom = execFileSync(CHROME, [
    "--headless=new", "--disable-gpu", "--virtual-time-budget=8000",
    `--user-data-dir=${join(dir, "profile")}`, "--dump-dom", `file:///${file.replace(/\\/g, "/")}`,
  ], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
} catch (error) {
  console.error("FAILED: could not run Chrome:", error.message);
  process.exit(1);
}

const m = /<div id="printAudit">([^<]*)<\/div>/.exec(dom);
if (!m) {
  console.error("FAILED: probe did not run");
  process.exit(1);
}

console.log("what reaches the printed page\n");
let surprising = 0;
let dossierOk = false;

for (const row of m[1].split(" | ")) {
  const [name, state] = row.split("=");

  /* Anything screen-only that stays VISIBLE is a print defect. */
  const screenOnly = ["header bar", "left input card", "preset panel", "tab bar", "KPI row", "verdict banner", "legal drawer"];

  if (name === "dossier") {
    /* Reported as its own computed display, not the ancestors': the dossier is
       concealed while another tab is active, so a visibility walk cannot tell a
       working print rule from a broken one. */
    dossierOk = state === "block";
    console.log(`  ${dossierOk ? "ok              " : "MUST PRINT      "}  ${name.padEnd(18)} display:${state}`);
    continue;
  }

  const isDefect = screenOnly.includes(name) && state === "VISIBLE";
  if (isDefect) surprising++;
  console.log(`  ${isDefect ? "SHOULD NOT PRINT" : "ok              "}  ${name.padEnd(18)} ${state}`);
}

if (!dossierOk) surprising++;
console.log(`\n${surprising === 0 ? "print output is clean: only the dossier prints" : surprising + " print defect(s)"}`);
process.exit(surprising === 0 ? 0 : 1);

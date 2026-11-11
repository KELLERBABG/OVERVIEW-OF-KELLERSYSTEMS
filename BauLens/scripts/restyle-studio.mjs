/**
 * One-shot codemod: move the BauLens Studio stylesheet onto the shared
 * design tokens used by index.html, without touching the class contract.
 *
 * Run from the repository root:  node scripts/restyle-studio.mjs
 */
import { readFileSync, writeFileSync } from "node:fs";

const PATH = "studio.html";
const raw = readFileSync(PATH, "utf8");

/* Normalise to LF for matching, restore CRLF on write — the file is CRLF. */
const eol = raw.includes("\r\n") ? "\r\n" : "\n";
let html = raw.split("\r\n").join("\n");
const before = html;

/* ---------------------------------------------------------------------------
 * 1. Fonts — one family in two roles, matching the landing page.
 * ------------------------------------------------------------------------- */
const FONT_LINK_OLD =
  '<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">';
const FONT_LINK_NEW =
  '<link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600&family=IBM+Plex+Sans:wght@300;400;500;600;700&display=swap" rel="stylesheet">';
assert(html.includes(FONT_LINK_OLD), "font link not found");
html = html.replace(FONT_LINK_OLD, FONT_LINK_NEW);

/* ---------------------------------------------------------------------------
 * 2. Token block — replaces the old :root entirely.
 *
 *    The legacy names (--bg, --surface, --text, --cyan, --amber, --rose,
 *    --accent-bright, --border-bright, --font-heading) stay defined because
 *    the markup, the inline styles and the generated report strings reference
 *    them. They now resolve to the shared palette.
 * ------------------------------------------------------------------------- */
const OLD_ROOT = `    :root {
      --bg: #070a12;
      --surface: #0c1220;
      --surface-raised: #121b2f;
      --surface-subtle: rgba(18, 27, 47, 0.6);
      --border: #1c2842;
      --border-bright: rgba(16, 185, 129, 0.35);
      
      --accent: #10b981;
      --accent-bright: #34d399;
      --cyan: #38bdf8;
      --amber: #f59e0b;
      --rose: #f43f5e;

      --text: #f1f5f9;
      --text-muted: #94a3b8;
      --text-dim: #64748b;

      --font-heading: 'Space Grotesk', -apple-system, sans-serif;
      --font-mono: 'JetBrains Mono', monospace;
    }`;

const NEW_ROOT = `    /* =====================================================================
       BauLens design tokens — shared with index.html and docs.html.
       One neutral surface ramp, one reserved accent. Legacy token names are
       aliased so existing markup and generated report strings keep working.
       ===================================================================== */
    :root {
      /* Surfaces */
      --ink-900: #05070c;
      --ink-800: #080b12;
      --ink-700: #0b0f18;
      --ink-600: #101623;
      --ink-500: #161e2e;
      --ink-400: #1e2839;
      --ink-300: #2a374b;

      --line: #1a2434;
      --line-strong: #26334a;
      --ink-line: #33415a;

      /* One reserved accent */
      --accent: #2fb6cf;
      --accent-hi: #57cde2;
      --accent-dim: #1c7c8e;
      --accent-wash: rgba(47, 182, 207, 0.08);
      --accent-edge: rgba(47, 182, 207, 0.32);
      --on-accent: #04141a;

      /* Status — never the only signal */
      --ok: #2fb6cf;
      --warn: #d9a441;
      --fail: #e0657a;
      --warn-wash: rgba(217, 164, 65, 0.1);
      --fail-wash: rgba(224, 101, 122, 0.1);

      /* Foreground ramp */
      --fg: #e9edf3;
      --fg-muted: #98a4b6;
      --fg-dim: #6b7788;
      --fg-faint: #4d5768;

      /* Type */
      --font-ui: 'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      --font-mono: 'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;

      /* Radii and motion */
      --r-sm: 3px;
      --r-md: 5px;
      --r-lg: 8px;
      --shadow-2: 0 12px 32px -12px rgba(0, 0, 0, 0.7);
      --shadow-3: 0 32px 72px -28px rgba(0, 0, 0, 0.85);
      --dur-fast: 120ms;
      --dur-base: 200ms;
      --ease-out: cubic-bezier(0.22, 0.61, 0.36, 1);
      --ease-swift: cubic-bezier(0.16, 1, 0.3, 1);

      /* Legacy aliases — do not remove, referenced by markup and report output */
      --bg: var(--ink-900);
      --surface: var(--ink-800);
      --surface-raised: var(--ink-700);
      --border: var(--line);
      --border-strong: var(--line-strong);
      --border-bright: var(--accent-edge);
      --accent-bright: var(--accent-hi);
      --cyan: var(--accent);
      --amber: var(--warn);
      --rose: var(--fail);
      --text: var(--fg);
      --text-muted: var(--fg-muted);
      --text-dim: var(--fg-dim);
      --text-faint: var(--fg-faint);
      --font-sans: var(--font-ui);
      --font-heading: var(--font-ui);
    }`;

assert(html.includes(OLD_ROOT), ":root block not found verbatim");
html = html.replace(OLD_ROOT, NEW_ROOT);

/* ---------------------------------------------------------------------------
 * 3. Literal colour replacements (dark-theme only, main stylesheet).
 * ------------------------------------------------------------------------- */
const MAP = [
  // accents
  ["#10b981", "var(--accent)"],
  ["#34d399", "var(--accent-hi)"],
  ["#38bdf8", "var(--accent)"],
  // status
  ["#f59e0b", "var(--warn)"],
  ["#f43f5e", "var(--fail)"],
  ["#fde68a", "var(--warn)"],
  // surfaces
  ["#070a12", "var(--ink-900)"],
  ["#04070e", "var(--ink-900)"],
  ["#060911", "var(--ink-900)"],
  ["#040711", "var(--ink-900)"],
  ["#080c16", "var(--ink-800)"],
  ["#090e1a", "var(--ink-800)"],
  ["#0c1220", "var(--ink-800)"],
  ["#0d1527", "var(--ink-700)"],
  ["#121b2f", "var(--ink-700)"],
  ["#1c2842", "var(--line)"],
  // foreground
  ["#f1f5f9", "var(--fg)"],
  ["#e2e8f0", "var(--fg)"],
  ["#cbd5e1", "var(--fg-muted)"],
  ["#94a3b8", "var(--fg-muted)"],
  ["#64748b", "var(--fg-dim)"],
  ["#04140b", "var(--on-accent)"],
  // neutrals
  ["#ffffff", "#ffffff"],
  ["#fff", "#fff"],
];

for (const [from, to] of MAP) {
  if (from === to) continue;
  const n = count(html, from);
  if (n > 0) {
    html = html.split(from).join(to);
    console.log(`  ${from} -> ${to}  (${n})`);
  }
}

/* ---------------------------------------------------------------------------
 * 4. Colour-valued rgba() -> tokens, so a palette change is one edit.
 * ------------------------------------------------------------------------- */
const RGBA_MAP = [
  ["rgba(16, 185, 129, 0.05)", "var(--accent-wash)"],
  ["rgba(16, 185, 129, 0.08)", "var(--accent-wash)"],
  ["rgba(16, 185, 129, 0.12)", "var(--accent-wash)"],
  ["rgba(16, 185, 129, 0.2)", "var(--accent-edge)"],
  ["rgba(16, 185, 129, 0.25)", "var(--accent-edge)"],
  ["rgba(16, 185, 129, 0.3)", "var(--accent-edge)"],
  ["rgba(16, 185, 129, 0.35)", "var(--accent-edge)"],
  ["rgba(16, 185, 129, 0.4)", "var(--accent-edge)"],
  ["rgba(56, 189, 248, 0.05)", "var(--accent-wash)"],
  ["rgba(56, 189, 248, 0.08)", "var(--accent-wash)"],
  ["rgba(56, 189, 248, 0.22)", "var(--accent-edge)"],
  ["rgba(56, 189, 248, 0.25)", "var(--accent-edge)"],
  ["rgba(245, 158, 11, 0.08)", "var(--warn-wash)"],
  ["rgba(245, 158, 11, 0.12)", "var(--warn-wash)"],
  ["rgba(245, 158, 11, 0.3)", "var(--accent-edge)"],
  ["rgba(245, 158, 11, 0.35)", "var(--accent-edge)"],
  ["rgba(244, 63, 94, 0.08)", "var(--fail-wash)"],
  ["rgba(244, 63, 94, 0.12)", "var(--fail-wash)"],
  ["rgba(244, 63, 94, 0.3)", "var(--accent-edge)"],
  ["rgba(244, 63, 94, 0.35)", "var(--accent-edge)"],
  ["rgba(18, 27, 47, 0.4)", "var(--ink-600)"],
  ["rgba(18, 27, 47, 0.6)", "var(--ink-600)"],
  ["rgba(7, 10, 18, 0.88)", "rgba(5, 7, 12, 0.88)"],
  ["rgba(2, 6, 23, 0.7)", "rgba(5, 7, 12, 0.72)"],
  ["rgba(255, 255, 255, 0.015)", "rgba(255, 255, 255, 0.02)"],
  ["rgba(255, 255, 255, 0.04)", "var(--line)"],
  ["rgba(255, 255, 255, 0.08)", "var(--line-strong)"],
];

for (const [from, to] of RGBA_MAP) {
  const n = count(html, from);
  if (n > 0) {
    html = html.split(from).join(to);
    console.log(`  ${from} -> ${to}  (${n})`);
  }
}

/* ---------------------------------------------------------------------------
 * 5. Body background — flat ink ground with the drafting grid, matching the
 *    landing page, instead of the green radial glow.
 * ------------------------------------------------------------------------- */
const OLD_BODY = `    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background: radial-gradient(ellipse at 50% 0%, #082820 0%, var(--bg) 70%);
      color: var(--text);
      font-family: var(--font-heading);
      line-height: 1.5;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }`;

const NEW_BODY = `    *, *::before, *::after { box-sizing: border-box; }

    body {
      margin: 0;
      padding: 0;
      background-color: var(--ink-900);
      background-image:
        linear-gradient(to right, rgba(255, 255, 255, 0.03) 1px, transparent 1px),
        linear-gradient(to bottom, rgba(255, 255, 255, 0.03) 1px, transparent 1px);
      background-size: 88px 88px;
      background-attachment: fixed;
      color: var(--fg);
      font-family: var(--font-ui);
      font-size: 0.9375rem;
      line-height: 1.55;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      overflow-x: clip;
      -webkit-font-smoothing: antialiased;
    }

    :focus-visible {
      outline: 2px solid var(--accent-hi);
      outline-offset: 2px;
      border-radius: var(--r-sm);
    }

    ::selection { background: var(--accent-edge); color: var(--fg); }

    /* Grid and flex children default to min-width:auto; a canvas or a long
       string would otherwise push a track past the viewport. */
    .studio-layout > *, .grid-2 > *, .grid-4 > *, .kpi-grid > *, .card > * { min-width: 0; }`;

assert(html.includes(OLD_BODY), "body block not found verbatim");
html = html.replace(OLD_BODY, NEW_BODY);

/* ---------------------------------------------------------------------------
 * 6. Remaining cosmetic alignment.
 * ------------------------------------------------------------------------- */
const TWEAKS = [
  // The dossier paper is a document, not UI chrome — it keeps its own type.
  ["font-family: 'Segoe UI', -apple-system, BlinkMacSystemFont, Roboto, Helvetica, Arial, sans-serif;",
   "font-family: var(--font-ui);"],
  // Corner radius language: 6px/10px/12px collapse onto the shared scale.
  ["border-radius: 6px;", "border-radius: var(--r-md);"],
  ["border-radius: 8px;", "border-radius: var(--r-lg);"],
  ["border-radius: 10px;", "border-radius: var(--r-lg);"],
  ["border-radius: 12px;", "border-radius: var(--r-lg);"],
  ["border-radius: 4px;", "border-radius: var(--r-sm);"],
  ["transition: all 0.15s;", "transition: background-color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);"],
  ["transition: all 0.2s;", "transition: background-color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);"],
  ["transition: all 0.15s ease;", "transition: background-color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);"],
];

for (const [from, to] of TWEAKS) {
  const n = count(html, from);
  if (n > 0) {
    html = html.split(from).join(to);
    console.log(`  ${from}  (${n})`);
  }
}

assert(html !== before, "no changes were made");

writeFileSync(PATH, eol === "\r\n" ? html.split("\n").join("\r\n") : html, "utf8");
console.log(`\nstudio.html rewritten (${before.length} -> ${html.length} chars, ${eol === "\r\n" ? "CRLF" : "LF"})`);

function count(haystack, needle) {
  return haystack.split(needle).length - 1;
}

function assert(condition, message) {
  if (!condition) {
    console.error(`FAILED: ${message}`);
    process.exit(1);
  }
}

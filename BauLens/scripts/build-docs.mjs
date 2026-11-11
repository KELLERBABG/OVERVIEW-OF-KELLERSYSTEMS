/**
 * Builds docs.html — the in-browser documentation viewer.
 *
 *   node scripts/build-docs.mjs
 *
 * The .md files stay the single source of truth. Their contents are embedded so
 * the page works when opened directly from disk (fetch() on file:// is
 * blocked), and at runtime the viewer re-fetches the live files when it is
 * served over http(s), so a deploy is never stale.
 *
 * Escaping note: the viewer script lives inside a JS template literal. Writing
 * regexes such as /\s+/ there needs quadruple backslashes, which is unreadable
 * and was the source of real bugs. Instead the template uses the placeholder
 * tokens BSL and BT, which are substituted for a backslash and a backtick after
 * the literal is assembled.
 */
import { readFileSync, writeFileSync } from "node:fs";

const DOCS = [
  { slug: "readme", title: "README", file: "README.md", blurb: "Zweck, Prüfumfang, CLI und Profilwahl" },
  { slug: "rules", title: "Regelkatalog", file: "docs/RULES.md", blurb: "Alle 15 Prüfmodule mit Formel und Fundstelle" },
  { slug: "whitepaper", title: "Whitepaper", file: "docs/WHITEPAPER.md", blurb: "Architektur, Betriebsmodell, Kommerzialisierung" },
  { slug: "changelog", title: "Änderungsverlauf", file: "CHANGELOG.md", blurb: "Versionshistorie und Releases" },
];

const read = (file) => readFileSync(file, "utf8").replace(/\r\n/g, "\n");

/* The @font-face rules are inlined rather than linked, so docs.html renders with
   no network at all and needs no stylesheet request under the desktop app's
   asset protocol. The comment header is dropped to keep the payload tight. */
const FONT_STYLE = (() => {
  const css = readFileSync("assets/fonts/fonts.css", "utf8")
    .split("\n")
    .filter((line) => !/^\s*(\/\*|\*)/.test(line))
    .join("\n")
    .trim();
  return `/* Self-hosted IBM Plex; see assets/fonts/OFL.txt */\n${css}\n`;
})();

function assert(condition, message) {
  if (!condition) {
    console.error(`FAILED: ${message}`);
    process.exit(1);
  }
}

/**
 * Markdown goes into a <script type="application/json"> block. The stringified
 * payload is emitted with every `<` written as \u003c, because HTML ends the
 * block at the first literal `</script>` — which any prose could contain
 * (the CHANGELOG names the tag). JSON.parse turns \u003c back into `<`, so the
 * reader sees the source text unmodified. Escape only here, after stringify:
 * pre-escaping the values would double-escape and surface a literal \u003c.
 */
const payload = Object.fromEntries(DOCS.map((d) => [d.slug, read(d.file)]));

const railItems = DOCS.map(
  (d) => `        <li>
          <a class="rail-link" data-doc="${d.slug}" href="#${d.slug}">
            <span class="rail-title">${d.title}</span>
            <span class="rail-blurb">${d.blurb}</span>
          </a>
        </li>`
).join("\n");

const mobileOptions = DOCS.map((d) => `        <option value="${d.slug}">${d.title}</option>`).join("\n");

/* --------------------------------------------------------------------------
 * The page. BSL = backslash, BT = backtick.
 * ------------------------------------------------------------------------ */
const template = String.raw`<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>BauLens &mdash; Dokumentation</title>
<meta name="description" content="BauLens-Dokumentation im Browser: Regelkatalog mit allen 15 Prüfmodulen, technisches Whitepaper, README und Änderungsverlauf.">
<meta name="color-scheme" content="dark">
<link rel="icon" type="image/svg+xml" href="assets/icon.svg">
<style>
${FONT_STYLE}
  /* ==========================================================================
     BauLens documentation viewer — same token system as index.html.
     ========================================================================== */
  :root {
    --ink-900: #05070c;
    --ink-800: #080b12;
    --ink-700: #0b0f18;
    --ink-600: #101623;
    --ink-500: #161e2e;
    --line: #1a2434;
    --line-strong: #26334a;
    --ink-line: #33415a;

    --accent: #2fb6cf;
    --accent-hi: #57cde2;
    --accent-wash: rgba(47, 182, 207, 0.08);
    --accent-edge: rgba(47, 182, 207, 0.32);
    --on-accent: #04141a;

    --ok: #2fb6cf;
    --warn: #d9a441;
    --warn-wash: rgba(217, 164, 65, 0.1);

    --fg: #e9edf3;
    --fg-muted: #98a4b6;
    --fg-dim: #6b7788;
    --fg-faint: #4d5768;

    --font-sans: 'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    --font-mono: 'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;

    --r-sm: 3px;
    --r-md: 5px;
    --r-lg: 8px;
    --dur-fast: 120ms;
    --ease-out: cubic-bezier(0.22, 0.61, 0.36, 1);

    --rail: 288px;
    --content: 78ch;
  }

  *, *::before, *::after { box-sizing: border-box; }

  html { scroll-behavior: smooth; scroll-padding-top: 88px; }

  body {
    margin: 0;
    min-height: 100vh;
    background-color: var(--ink-900);
    background-image:
      linear-gradient(to right, rgba(255, 255, 255, 0.028) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(255, 255, 255, 0.028) 1px, transparent 1px);
    background-size: 88px 88px;
    background-attachment: fixed;
    color: var(--fg);
    font-family: var(--font-sans);
    font-size: 1rem;
    line-height: 1.7;
    overflow-x: clip;
    -webkit-font-smoothing: antialiased;
  }

  :focus-visible { outline: 2px solid var(--accent-hi); outline-offset: 2px; border-radius: var(--r-sm); }
  ::selection { background: var(--accent-edge); color: var(--fg); }

  .skip-link {
    position: absolute; left: 1rem; top: -100px; z-index: 100;
    padding: 0.6rem 1rem; background: var(--ink-500);
    border: 1px solid var(--accent); border-radius: var(--r-md);
    font-family: var(--font-mono); font-size: 0.8125rem; text-decoration: none;
    transition: top var(--dur-fast) var(--ease-out);
  }
  .skip-link:focus { top: 1rem; }

  /* ---------- Header ---------- */
  .site-header {
    position: sticky; top: 0; z-index: 60;
    background: rgba(5, 7, 12, 0.88);
    border-bottom: 1px solid var(--line);
    backdrop-filter: blur(12px) saturate(140%);
  }
  .header-inner {
    display: flex; align-items: center; justify-content: space-between;
    gap: 1.5rem; height: 64px;
    max-width: 1420px; margin: 0 auto; padding: 0 1.5rem;
  }
  .brand { display: flex; align-items: center; gap: 0.7rem; text-decoration: none; color: inherit; }
  .brand-mark {
    display: grid; place-items: center; width: 32px; height: 32px;
    background: var(--ink-700); border: 1px solid var(--line-strong); border-radius: var(--r-md);
  }
  .brand-mark img { display: block; width: 22px; height: 22px; }
  .brand-text { display: flex; flex-direction: column; line-height: 1.1; }
  .brand-name { font-size: 1rem; font-weight: 600; letter-spacing: -0.02em; }
  .brand-owner {
    font-family: var(--font-mono); font-size: 0.62rem;
    letter-spacing: 0.12em; text-transform: uppercase; color: var(--fg-dim);
  }
  .header-actions { display: flex; align-items: center; gap: 0.5rem; }
  .header-link {
    font-size: 0.875rem; color: var(--fg-muted); text-decoration: none;
    padding: 0.4rem 0.6rem; border-radius: var(--r-md);
    transition: color var(--dur-fast) var(--ease-out), background-color var(--dur-fast) var(--ease-out);
  }
  .header-link:hover { color: var(--fg); background: var(--ink-600); }
  .header-cta {
    font-size: 0.8125rem; font-weight: 550; text-decoration: none;
    color: var(--on-accent); background: var(--accent);
    padding: 0.5rem 0.9rem; border-radius: var(--r-md);
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  .header-cta:hover { background: var(--accent-hi); }
  .mobile-doc-picker { display: none; min-width: 0; }
  .mobile-doc-picker select {
    background: var(--ink-700); color: var(--fg); border: 1px solid var(--line-strong);
    border-radius: var(--r-md); padding: 0.45rem 0.6rem;
    font-family: var(--font-mono); font-size: 0.8rem;
    max-width: 42vw;
  }
  .visually-hidden {
    position: absolute; width: 1px; height: 1px; margin: -1px; padding: 0;
    overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; border: 0;
  }

  /* ---------- Shell ---------- */
  .shell {
    display: grid;
    grid-template-columns: var(--rail) minmax(0, 1fr);
    gap: 3.5rem;
    max-width: 1420px;
    margin: 0 auto;
    padding: 0 1.5rem 6rem;
    align-items: start;
  }

  /* ---------- Rail ---------- */
  .rail { position: sticky; top: 88px; padding: 2.5rem 0 2rem; max-height: calc(100vh - 88px); overflow-y: auto; }
  .rail-label {
    font-family: var(--font-mono); font-size: 0.68rem; font-weight: 500;
    letter-spacing: 0.13em; text-transform: uppercase; color: var(--fg-dim);
    margin: 0 0 1rem;
  }
  .rail-list { list-style: none; margin: 0 0 2rem; padding: 0; }
  .rail-list li + li { margin-top: 1px; }
  .rail-link {
    display: block; padding: 0.7rem 0.85rem; text-decoration: none;
    border-left: 2px solid transparent; border-radius: 0 var(--r-md) var(--r-md) 0;
    transition: background-color var(--dur-fast) var(--ease-out),
                border-color var(--dur-fast) var(--ease-out);
  }
  .rail-link:hover { background: var(--ink-700); }
  .rail-link[aria-current="true"] { background: var(--accent-wash); border-left-color: var(--accent); }
  .rail-title { display: block; font-size: 0.9375rem; font-weight: 500; color: var(--fg); }
  .rail-link[aria-current="true"] .rail-title { color: var(--accent-hi); }
  .rail-blurb { display: block; font-size: 0.78rem; color: var(--fg-dim); line-height: 1.45; margin-top: 0.15rem; }

  .rail-toc { border-top: 1px solid var(--line); padding-top: 1.25rem; }
  .rail-toc ol { list-style: none; margin: 0; padding: 0; }
  .rail-toc li { margin: 0; }
  .rail-toc a {
    display: block; padding: 0.28rem 0.85rem; font-size: 0.8125rem;
    color: var(--fg-muted); text-decoration: none;
    border-left: 2px solid transparent;
    transition: color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out);
  }
  .rail-toc a:hover { color: var(--fg); border-left-color: var(--ink-line); }
  .rail-toc a.lvl-3 { padding-left: 1.6rem; font-size: 0.78rem; color: var(--fg-dim); }

  /* ---------- Article ---------- */
  .doc { padding: 2.5rem 0 0; min-width: 0; }
  .doc-status {
    display: inline-flex; align-items: center; gap: 0.45rem;
    font-family: var(--font-mono); font-size: 0.7rem; letter-spacing: 0.06em;
    text-transform: uppercase; color: var(--fg-dim); margin: 0 0 1.5rem;
  }
  .doc-status .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--ok); }

  .prose { max-width: var(--content); }
  .prose > *:first-child { margin-top: 0; }

  .prose h1 {
    font-size: clamp(1.85rem, 1.4rem + 1.7vw, 2.5rem);
    font-weight: 300; line-height: 1.12; letter-spacing: -0.028em;
    margin: 0 0 1.25rem; padding-bottom: 1.25rem;
    border-bottom: 1px solid var(--line-strong);
  }
  .prose h2 {
    font-size: clamp(1.3rem, 1.15rem + 0.7vw, 1.6rem);
    font-weight: 600; line-height: 1.2; letter-spacing: -0.02em;
    margin: 3.25rem 0 1rem; padding-top: 1.75rem;
    border-top: 1px solid var(--line);
  }
  .prose h3 { font-size: 1.1rem; font-weight: 600; letter-spacing: -0.012em; margin: 2.25rem 0 0.75rem; }
  .prose h4 {
    font-family: var(--font-mono); font-size: 0.82rem; font-weight: 500;
    letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-dim);
    margin: 1.75rem 0 0.6rem;
  }
  .prose h2, .prose h3, .prose h4 { scroll-margin-top: 92px; }

  .prose p { margin: 0 0 1.1rem; color: var(--fg-muted); }
  .prose strong { color: var(--fg); font-weight: 600; }
  .prose a { color: var(--accent-hi); text-decoration: none; border-bottom: 1px solid var(--accent-edge); }
  .prose a:hover { border-bottom-color: var(--accent-hi); }

  .prose ul, .prose ol { margin: 0 0 1.1rem; padding-left: 1.35rem; color: var(--fg-muted); }
  .prose li { margin-bottom: 0.4rem; }
  .prose li > ul, .prose li > ol { margin: 0.4rem 0 0.2rem; }
  .prose ul li::marker { color: var(--fg-faint); }
  .prose ol li::marker { color: var(--accent); font-family: var(--font-mono); font-size: 0.85em; }

  .prose blockquote {
    margin: 1.4rem 0; padding: 0.9rem 1.15rem;
    background: var(--ink-800); border-left: 2px solid var(--accent);
    border-radius: 0 var(--r-md) var(--r-md) 0;
    color: var(--fg-muted); font-size: 0.9375rem;
  }
  .prose blockquote p:last-child { margin-bottom: 0; }

  .prose hr { border: 0; border-top: 1px solid var(--line); margin: 2.75rem 0; }

  .prose code {
    font-family: var(--font-mono); font-size: 0.855em;
    background: var(--ink-600); border: 1px solid var(--line);
    border-radius: var(--r-sm); padding: 0.1em 0.35em; color: var(--fg);
  }
  .prose pre {
    margin: 1.4rem 0; padding: 1.1rem 1.25rem;
    background: var(--ink-800); border: 1px solid var(--line);
    border-radius: var(--r-lg); overflow-x: auto;
  }
  .prose pre code {
    background: none; border: 0; padding: 0;
    font-size: 0.83rem; line-height: 1.65; color: var(--fg-muted);
  }

  .prose .table-scroll { overflow-x: auto; margin: 1.4rem 0; border: 1px solid var(--line); border-radius: var(--r-lg); }
  .prose table { width: 100%; border-collapse: collapse; font-size: 0.875rem; min-width: 460px; }
  .prose thead th {
    background: var(--ink-700); text-align: left;
    padding: 0.65rem 0.9rem;
    font-family: var(--font-mono); font-size: 0.7rem; font-weight: 500;
    letter-spacing: 0.09em; text-transform: uppercase; color: var(--fg-dim);
    border-bottom: 1px solid var(--line);
  }
  .prose tbody td {
    padding: 0.7rem 0.9rem; color: var(--fg-muted);
    border-bottom: 1px solid var(--line); vertical-align: top;
  }
  .prose tbody tr:last-child td { border-bottom: 0; }
  .prose tbody td:first-child { color: var(--fg); }

  .doc-footer {
    max-width: var(--content); margin-top: 4rem; padding-top: 1.75rem;
    border-top: 1px solid var(--line);
    display: flex; flex-wrap: wrap; gap: 1rem;
    align-items: center; justify-content: space-between;
    font-size: 0.8125rem; color: var(--fg-dim);
  }
  .doc-footer a { color: var(--fg-muted); text-decoration: none; }
  .doc-footer a:hover { color: var(--accent-hi); }

  /* ---------- Responsive ---------- */
  @media (max-width: 1080px) {
    .shell { grid-template-columns: minmax(0, 1fr); gap: 0; }
    .rail { display: none; }
    .mobile-doc-picker { display: block; }
    .header-link { display: none; }
    .prose { max-width: 100%; }
  }
  @media (max-width: 620px) {
    .header-inner { padding: 0 1rem; gap: 0.6rem; }
    .shell { padding: 0 1rem 4rem; }
    .brand-owner { display: none; }
    .mobile-doc-picker select { max-width: 100%; }
  }

  @media (prefers-reduced-motion: reduce) {
    html { scroll-behavior: auto; }
    *, *::before, *::after {
      animation-duration: 0.001ms !important;
      transition-duration: 0.001ms !important;
    }
  }

  @media print {
    body { background: #fff; color: #000; }
    .site-header, .rail, .doc-footer, .doc-status, .skip-link { display: none !important; }
    .shell { display: block; padding: 0; max-width: 100%; }
    .prose { max-width: 100%; }
    .prose p, .prose li, .prose td { color: #000 !important; }
    .prose h1, .prose h2, .prose h3, .prose h4 { color: #000 !important; border-color: #ccc !important; }
    .prose pre, .prose code, .prose blockquote, .prose thead th { background: #f5f5f5 !important; border-color: #ccc !important; }
    .prose a { color: #000 !important; border: 0 !important; }
  }
</style>
</head>
<body>
<a class="skip-link" href="#doc">Zum Dokument springen</a>

<header class="site-header">
  <div class="header-inner">
    <a class="brand" href="index.html">
      <span class="brand-mark"><img src="assets/icon.svg" alt="" width="22" height="22"></span>
      <span class="brand-text">
        <span class="brand-name">BauLens</span>
        <span class="brand-owner">Dokumentation</span>
      </span>
    </a>

    <nav class="mobile-doc-picker" aria-label="Dokument wählen">
      <label for="docPicker" class="visually-hidden">Dokument</label>
      <select id="docPicker">
${mobileOptions}
      </select>
    </nav>

    <div class="header-actions">
      <a class="header-link" href="index.html">Produktseite</a>
      <a class="header-link" href="downloads/">Downloads</a>
      <a class="header-link" href="pricing/">Preise</a>
      <a class="header-link" href="legal/impressum/">Impressum</a>
      <a class="header-cta" href="downloads/">BauLens herunterladen</a>
    </div>
  </div>
</header>

<div class="shell">
  <aside class="rail">
    <p class="rail-label">Dokumente</p>
    <ul class="rail-list">
${railItems}
    </ul>

    <nav class="rail-toc" aria-label="Inhalt dieses Dokuments">
      <p class="rail-label">Inhalt</p>
      <ol id="tocList"></ol>
    </nav>
  </aside>

  <main class="doc" id="doc" tabindex="-1">
    <p class="doc-status"><span class="dot" aria-hidden="true"></span><span id="docSource">Eingebettete Fassung</span></p>
    <article class="prose" id="prose"></article>
    <div class="doc-footer">
      <p>Quelle: <span id="docPath">README.md</span> im Repository</p>
      <p><a href="index.html">Produktseite</a> &middot; <a href="downloads/">Downloads</a> &middot; <a href="https://github.com/KELLERBABG/BauLens" target="_blank" rel="noopener noreferrer">GitHub</a></p>
    </div>
  </main>
</div>

<script type="application/json" id="docSources">
__DOC_SOURCES__
</script>

<script>
(function () {
  "use strict";

  var DOCS = __DOC_LIST__;
  var embedded = JSON.parse(document.getElementById("docSources").textContent);

  var prose = document.getElementById("prose");
  var tocList = document.getElementById("tocList");
  var docSource = document.getElementById("docSource");
  var docPath = document.getElementById("docPath");
  var picker = document.getElementById("docPicker");

  /* ------------------------------------------------------------------ *
   * Minimal, dependency-free Markdown renderer.
   * It builds DOM nodes directly rather than assigning innerHTML, so a
   * malformed table or a stray angle bracket cannot inject markup.
   * ------------------------------------------------------------------ */

  function slugify(text, used) {
    var base = text.toLowerCase()
      .replace(/ä/g, "ae").replace(/ö/g, "oe").replace(/ü/g, "ue").replace(/ß/g, "ss")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "abschnitt";
    var id = base, n = 2;
    while (used[id]) { id = base + "-" + n; n += 1; }
    used[id] = true;
    return id;
  }

  function textNode(value) { return document.createTextNode(value); }

  /**
   * Inline spans: code, links, bold, italic.
   * Written as a scanner rather than one dense regex — the pattern needed to
   * express all four forms plus their delimiters was unreadable and brittle.
   */
  function inline(text, parent) {
    var s = String(text == null ? "" : text);
    var buf = "";
    var i = 0;

    function flush() {
      if (buf) { parent.appendChild(textNode(buf)); buf = ""; }
    }

    while (i < s.length) {
      var ch = s.charAt(i);

      /* BTcodeBT */
      if (ch === "BT") {
        var codeEnd = s.indexOf("BT", i + 1);
        if (codeEnd > i + 1) {
          flush();
          var code = document.createElement("code");
          code.textContent = s.slice(i + 1, codeEnd);
          parent.appendChild(code);
          i = codeEnd + 1;
          continue;
        }
      }

      /* [label](href) */
      if (ch === "[") {
        var labelEnd = s.indexOf("]", i + 1);
        if (labelEnd > i + 1 && s.charAt(labelEnd + 1) === "(") {
          var hrefEnd = s.indexOf(")", labelEnd + 2);
          var label = s.slice(i + 1, labelEnd);
          var href = hrefEnd > labelEnd ? s.slice(labelEnd + 2, hrefEnd) : "";
          if (href && !/BSs/.test(href)) {
            flush();
            var a = document.createElement("a");
            a.textContent = label;
            a.href = href;
            if (/^https?:/i.test(href)) { a.target = "_blank"; a.rel = "noopener noreferrer"; }
            parent.appendChild(a);
            i = hrefEnd + 1;
            continue;
          }
        }
      }

      /* **strong** and __strong__ */
      if ((ch === "*" || ch === "_") && s.charAt(i + 1) === ch) {
        var strongEnd = s.indexOf(ch + ch, i + 2);
        if (strongEnd > i + 2) {
          flush();
          var strong = document.createElement("strong");
          strong.textContent = s.slice(i + 2, strongEnd);
          parent.appendChild(strong);
          i = strongEnd + 2;
          continue;
        }
      }

      /* *em* and _em_ */
      if (ch === "*" || ch === "_") {
        var emEnd = s.indexOf(ch, i + 1);
        var emText = emEnd > i + 1 ? s.slice(i + 1, emEnd) : "";
        if (emText && emText.indexOf("BSn") === -1) {
          flush();
          var em = document.createElement("em");
          em.textContent = emText;
          parent.appendChild(em);
          i = emEnd + 1;
          continue;
        }
      }

      buf += ch;
      i += 1;
    }

    flush();
  }

  function isTableSeparator(line) {
    return /^BSs*BS|?[BSs:|-]+BS|BSs*$/.test(line) && line.indexOf("-") !== -1;
  }

  /** Split a table row on unescaped pipes. */
  function splitRow(line) {
    var s = line.trim().replace(/^BS|/, "").replace(/BS|$/, "");
    var cells = [], current = "", i = 0;
    while (i < s.length) {
      if (s.charAt(i) === "BSBS" && s.charAt(i + 1) === "|") { current += "|"; i += 2; continue; }
      if (s.charAt(i) === "|") { cells.push(current); current = ""; i += 1; continue; }
      current += s.charAt(i);
      i += 1;
    }
    cells.push(current);
    return cells.map(function (c) { return c.trim(); });
  }

  function alignments(separator) {
    return splitRow(separator).map(function (c) {
      if (/^:-+:$/.test(c)) return "center";
      if (/^-+:$/.test(c)) return "right";
      return "left";
    });
  }

  function render(markdown) {
    var lines = String(markdown || "").replace(/BSrBSn/g, "BSn").split("BSn");
    var frag = document.createDocumentFragment();
    var used = {};
    var headings = [];
    var i = 0;
    var para = [];

    function flushParagraph() {
      if (!para.length) return;
      var p = document.createElement("p");
      inline(para.join(" "), p);
      frag.appendChild(p);
      para.length = 0;
    }

    function makeHeading(level, text) {
      var el = document.createElement("h" + level);
      var clean = text.replace(/BS*|BT|_/g, "");
      el.id = slugify(clean, used);
      inline(text, el);
      frag.appendChild(el);
      if (level <= 3) headings.push({ id: el.id, text: clean, level: level });
    }

    while (i < lines.length) {
      var line = lines[i];

      /* Fenced code block */
      if (/^BSs*BTBTBT/.test(line)) {
        flushParagraph();
        var body = [];
        i += 1;
        while (i < lines.length && !/^BSs*BTBTBT/.test(lines[i])) { body.push(lines[i]); i += 1; }
        i += 1;
        var pre = document.createElement("pre");
        var code = document.createElement("code");
        code.textContent = body.join("BSn");
        pre.appendChild(code);
        frag.appendChild(pre);
        continue;
      }

      /* ATX heading */
      var h = /^(#{1,4})BSs+(.*)$/.exec(line);
      if (h) {
        flushParagraph();
        makeHeading(h[1].length, h[2].replace(/BSs+#+BSs*$/, ""));
        i += 1;
        continue;
      }

      /* Horizontal rule */
      if (/^BSs*([-*_])BSs*(BS1BSs*){2,}$/.test(line)) {
        flushParagraph();
        frag.appendChild(document.createElement("hr"));
        i += 1;
        continue;
      }

      /* Blockquote */
      if (/^BSs*>/.test(line)) {
        flushParagraph();
        var quote = [];
        while (i < lines.length && /^BSs*>/.test(lines[i])) {
          quote.push(lines[i].replace(/^BSs*>BSs?/, ""));
          i += 1;
        }
        var bq = document.createElement("blockquote");
        var bqPara = document.createElement("p");
        inline(quote.join(" "), bqPara);
        bq.appendChild(bqPara);
        frag.appendChild(bq);
        continue;
      }

      /* Table */
      if (line.indexOf("|") !== -1 && i + 1 < lines.length && isTableSeparator(lines[i + 1])) {
        flushParagraph();
        var head = splitRow(line);
        var align = alignments(lines[i + 1]);
        i += 2;
        var rows = [];
        while (i < lines.length && lines[i].indexOf("|") !== -1 && lines[i].trim() !== "") {
          rows.push(splitRow(lines[i]));
          i += 1;
        }
        var wrap = document.createElement("div");
        wrap.className = "table-scroll";
        var table = document.createElement("table");
        var thead = document.createElement("thead");
        var htr = document.createElement("tr");
        head.forEach(function (cell, ci) {
          var th = document.createElement("th");
          th.style.textAlign = align[ci] || "left";
          inline(cell, th);
          htr.appendChild(th);
        });
        thead.appendChild(htr);
        table.appendChild(thead);
        var tbody = document.createElement("tbody");
        rows.forEach(function (r) {
          var tr = document.createElement("tr");
          for (var ci = 0; ci < head.length; ci += 1) {
            var td = document.createElement("td");
            td.style.textAlign = align[ci] || "left";
            inline(r[ci] === undefined ? "" : r[ci], td);
            tr.appendChild(td);
          }
          tbody.appendChild(tr);
        });
        table.appendChild(tbody);
        wrap.appendChild(table);
        frag.appendChild(wrap);
        continue;
      }

      /* List */
      var li = /^(BSs*)([-*+]|BSd+BS.)BSs+(.*)$/.exec(line);
      if (li) {
        flushParagraph();
        var ordered = /BSd/.test(li[2]);
        var list = document.createElement(ordered ? "ol" : "ul");
        while (i < lines.length) {
          var item = /^(BSs*)([-*+]|BSd+BS.)BSs+(.*)$/.exec(lines[i]);
          if (!item) {
            var continuation = lines[i];
            if (continuation.trim() !== "" && list.lastChild &&
                !/^BSs*(#{1,4}BSs|BS||>|BTBTBT)/.test(continuation)) {
              list.lastChild.appendChild(document.createTextNode(" " + continuation.trim()));
              i += 1;
              continue;
            }
            break;
          }
          var entry = document.createElement("li");
          inline(item[3], entry);
          list.appendChild(entry);
          i += 1;
        }
        frag.appendChild(list);
        continue;
      }

      /* Blank line closes the paragraph */
      if (line.trim() === "") { flushParagraph(); i += 1; continue; }

      para.push(line.trim());
      i += 1;
    }

    flushParagraph();
    return { fragment: frag, headings: headings };
  }

  /* ------------------------------------------------------------------ *
   * Rendering and routing
   * ------------------------------------------------------------------ */

  function findDoc(slug) {
    for (var i = 0; i < DOCS.length; i += 1) { if (DOCS[i].slug === slug) return DOCS[i]; }
    return null;
  }

  function paint(slug, markdown) {
    var doc = findDoc(slug);
    var result = render(markdown);

    prose.textContent = "";
    prose.appendChild(result.fragment);

    tocList.textContent = "";
    result.headings.forEach(function (heading) {
      var li = document.createElement("li");
      var a = document.createElement("a");
      a.href = "#" + heading.id;
      a.textContent = heading.text;
      if (heading.level === 3) a.className = "lvl-3";
      a.addEventListener("click", function (e) {
        e.preventDefault();
        var target = document.getElementById(heading.id);
        if (target) {
          target.scrollIntoView({ behavior: "smooth" });
          history.replaceState(null, "", location.pathname + location.search + "#" + heading.id);
        }
      });
      li.appendChild(a);
      tocList.appendChild(li);
    });

    Array.prototype.forEach.call(document.querySelectorAll(".rail-link"), function (link) {
      if (link.getAttribute("data-doc") === slug) link.setAttribute("aria-current", "true");
      else link.removeAttribute("aria-current");
    });

    if (doc) {
      docPath.textContent = doc.file;
      document.title = doc.title + " — BauLens Dokumentation";
      if (picker) picker.value = slug;
    }
  }

  var activeDocSlug = "";

  /**
   * Prefer the live .md file when served over http(s) so a deploy is never
   * stale. Opening from disk blocks fetch(), so the embedded copy is the base
   * case rather than a failure mode.
   */
  function load(slug, push, targetAnchor) {
    var doc = findDoc(slug) || DOCS[0];
    activeDocSlug = doc.slug;
    paint(doc.slug, embedded[doc.slug]);
    docSource.textContent = "Eingebettete Fassung";

    if (push !== false) {
      var suffix = location.pathname + location.search + "#" + doc.slug;
      history.replaceState(null, "", suffix);
    }

    if (targetAnchor) {
      setTimeout(function () {
        var el = document.getElementById(targetAnchor);
        if (el) el.scrollIntoView({ behavior: "smooth" });
      }, 0);
    }

    if (location.protocol !== "http:" && location.protocol !== "https:") return;

    fetch(doc.file, { cache: "no-cache" })
      .then(function (response) {
        if (!response.ok) throw new Error("HTTP " + response.status);
        var type = (response.headers.get("content-type") || "").toLowerCase();
        if (type.indexOf("html") !== -1) throw new Error("vom Server als HTML ausgeliefert");
        return response.text();
      })
      .then(function (text) {
        /* A host that runs a static-site generator converts .md to HTML. Keep
           the embedded copy then, rather than rendering a page of tags as prose. */
        if (text.indexOf("<!DOCTYPE") === 0 || text.indexOf("<!doctype") === 0 ||
            /^BSs*</.test(text)) {
          throw new Error("keine Markdown-Antwort");
        }
        if (activeDocSlug !== doc.slug) return;   /* the user moved on */
        paint(doc.slug, text);
        docSource.textContent = "Aktuelle Fassung aus dem Repository";
        if (targetAnchor) {
          setTimeout(function () {
            var el2 = document.getElementById(targetAnchor);
            if (el2) el2.scrollIntoView({ behavior: "smooth" });
          }, 0);
        }
      })
      .catch(function () {
        /* The embedded copy is already on screen; just say so instead of
           raising a warning for what is a supported state. */
        docSource.textContent = "Eingebettete Fassung";
      });
  }

  function handleHashChange() {
    var hash = location.hash.replace(/^#/, "");
    if (!hash) {
      if (activeDocSlug !== DOCS[0].slug) load(DOCS[0].slug, false);
      return;
    }
    var doc = findDoc(hash);
    if (doc) {
      if (activeDocSlug !== doc.slug) load(doc.slug, false);
    } else {
      var target = document.getElementById(hash);
      if (target) {
        target.scrollIntoView({ behavior: "smooth" });
      }
    }
  }

  if (picker) picker.addEventListener("change", function () { load(picker.value, true); });

  Array.prototype.forEach.call(document.querySelectorAll(".rail-link"), function (link) {
    link.addEventListener("click", function (event) {
      event.preventDefault();
      var targetDoc = link.getAttribute("data-doc");
      if (activeDocSlug !== targetDoc) {
        load(targetDoc, true);
        window.scrollTo({ top: 0, behavior: "instant" });
      }
    });
  });

  window.addEventListener("hashchange", handleHashChange);

  var initialHash = location.hash.replace(/^#/, "");
  var initialDoc = findDoc(initialHash);
  if (initialDoc) {
    load(initialDoc.slug, false);
  } else if (initialHash) {
    load(DOCS[0].slug, false, initialHash);
  } else {
    load(DOCS[0].slug, false);
  }
})();
</script>
</body>
</html>
`;

/* Substitution order is load-bearing in two ways.
 *
 * 1. The payload must not be present while the placeholder passes run: a
 *    literal "BS" in the embedded markdown (the Swiss canton code CH_BS) would
 *    be rewritten into a stray backslash. So the payload is injected from a
 *    sentinel afterwards.
 * 2. Within the template, the backslash pass runs first — it can never emit a
 *    "BT", so the backtick pass stays correct.
 */
const DOC_SOURCES_SENTINEL = "__DOC_SOURCES__";
const DOC_LIST_SENTINEL = "__DOC_LIST__";

let html = template.split("BS").join("\\").split("BT").join("`");

assert(html.includes(DOC_SOURCES_SENTINEL), "docSources sentinel lost");
assert(html.includes(DOC_LIST_SENTINEL), "docList sentinel lost");

html = html
  .split(DOC_SOURCES_SENTINEL).join(JSON.stringify(payload).replace(/</g, "\\u003c"))
  .split(DOC_LIST_SENTINEL).join(JSON.stringify(DOCS));

assert(!html.includes("\\u0000"), "sentinel leaked into the output");

writeFileSync("docs.html", html, "utf8");
console.log(`docs.html written (${html.length} chars, ${DOCS.length} documents embedded)`);
for (const d of DOCS) console.log(`  ${d.file.padEnd(20)} ${payload[d.slug].length} chars`);

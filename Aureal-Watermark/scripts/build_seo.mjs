#!/usr/bin/env node
"use strict";

import { writeFileSync, mkdirSync, existsSync, readFileSync, readdirSync } from "node:fs";
import { resolve, join } from "node:path";
import { CONFIG, PAGES } from "./lexicon-data.mjs";

// ─────────────────────────────────────────────────────────────────────────────
// 1. Validation — fail before writing a page that breaks the claim guardrails
// ─────────────────────────────────────────────────────────────────────────────
const REQUIRED = [
  "slug", "h1", "shortTitle", "category", "metaDesc", "shortAnswer",
  "technicalBasis", "detail", "trapExample", "whyItFails", "redlineExample", "limits", "faq"
];

const slugs = new Set();
for (const item of PAGES) {
  for (const field of REQUIRED) {
    const value = item[field];
    if (!value || (Array.isArray(value) && value.length === 0)) {
      throw new Error(`${item.slug || "?"}: missing required field "${field}".`);
    }
  }
  if (!/^[a-z0-9-]+$/.test(item.slug)) {
    throw new Error(`Invalid slug: ${item.slug}`);
  }
  if (slugs.has(item.slug)) throw new Error(`Duplicate slug: ${item.slug}`);
  slugs.add(item.slug);

  if (item.metaDesc.length > 155) {
    throw new Error(`${item.slug}: meta description is ${item.metaDesc.length} chars (max 155).`);
  }
  if (!Array.isArray(item.faq) || item.faq.length < 2) {
    throw new Error(`${item.slug}: at least two FAQ entries are required.`);
  }
  if (!Array.isArray(item.limits) || item.limits.length < 1) {
    throw new Error(`${item.slug}: at least one scope/limit entry is required.`);
  }
  const bodyLength = [
    item.shortAnswer, item.technicalBasis, item.detail, item.trapExample, item.whyItFails,
    item.redlineExample, ...item.limits, ...item.faq.flatMap((entry) => [entry.q, entry.a])
  ].join(" ").length;
  if (bodyLength < 1200) {
    console.warn(`⚠ ${item.slug}: only ${bodyLength} chars of unique content.`);
  }
}

for (const item of PAGES) {
  for (const related of item.related || []) {
    if (!PAGES.some((entry) => entry.slug === related)) {
      throw new Error(`${item.slug}: related page "${related}" does not exist.`);
    }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Helpers
// ─────────────────────────────────────────────────────────────────────────────
const ROOT = resolve(".");
const OUT_DIR = join(ROOT, CONFIG.outDir);
if (!existsSync(OUT_DIR)) mkdirSync(OUT_DIR, { recursive: true });

const CHECK = process.argv.includes("--check");
let drift = 0;
const FENCE = "```";
const PAGE_COUNT = PAGES.length;
const PLANNED_TOPICS = CONFIG.plannedTopics || PAGE_COUNT;
const lastmod = (item) => item.updated || CONFIG.lastReviewed;
// Cloudflare Pages normalises "<name>.html" to the extensionless URL with a 308,
// so the canonical, sitemap, links and JSON-LD must use the clean form the host
// actually serves. The files stay as lexicon/<slug>.html on disk.
const pageUrl = (item) => `${CONFIG.baseUrl}/${CONFIG.outDir}/${item.slug}`;
const hubUrl = `${CONFIG.baseUrl}/${CONFIG.outDir}/`;

const esc = (value = "") =>
  String(value).replace(/[&<>"']/g, (char) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
  }[char]));

const jsonLd = (value) =>
  JSON.stringify(value)
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e")
    .replace(/&/g, "\\u0026");

function emit(file, content) {
  if (!CHECK) {
    writeFileSync(file, content, "utf8");
    return;
  }
  const existing = existsSync(file) ? readFileSync(file, "utf8") : null;
  if (existing !== content) {
    console.error(`DRIFT: ${file}`);
    drift += 1;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Shared chrome
// ─────────────────────────────────────────────────────────────────────────────
const CSS = `
:root {
  --bg: #08040d;
  --surface: #12091f;
  --surface-raised: #1a0f2b;
  --border: #301a52;
  --border-bright: #4b2a78;
  --text: #f0f6fc;
  --soft: #d8d2e6;
  --muted: #a8a2b8;
  --dim: #7c7490;
  --purple: #8b5cf6;
  --cyan: #38bdf8;
  --emerald: #10b981;
  --rose: #f43f5e;
  --amber: #f59e0b;
  --font: "Space Grotesk", ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0;
  min-height: 100vh;
  background:
    radial-gradient(ellipse at 50% -10%, rgba(139, 92, 246, 0.22), transparent 55%),
    var(--bg);
  color: var(--text);
  font-family: var(--font);
  line-height: 1.7;
  -webkit-font-smoothing: antialiased;
}
a { color: var(--cyan); text-underline-offset: 3px; }
a:hover { color: #fff; }
.wrap { width: min(980px, calc(100% - 40px)); margin: 0 auto; }
.site-header {
  position: sticky;
  top: 0;
  z-index: 20;
  border-bottom: 1px solid var(--border);
  background: rgba(8, 4, 13, 0.9);
  backdrop-filter: blur(14px);
}
.nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  min-height: 68px;
  padding: 10px 0;
}
.brand { display: flex; flex-direction: column; line-height: 1.15; text-decoration: none; color: var(--text); flex-shrink: 0; }
.brand strong { font-size: 1rem; letter-spacing: -0.02em; }
.brand span { margin-top: 3px; color: var(--muted); font-size: 0.64rem; letter-spacing: 0.14em; text-transform: uppercase; }
.nav-links { display: flex; align-items: center; flex-wrap: wrap; gap: 8px 16px; }
.nav-links a { color: var(--muted); font-size: 0.83rem; text-decoration: none; }
.nav-links a:hover, .nav-links a[aria-current="page"] { color: var(--text); }
.nav-links .nav-cta {
  padding: 8px 14px;
  border-radius: 6px;
  background: linear-gradient(135deg, var(--purple), #6366f1);
  color: #fff;
  font-weight: 600;
  box-shadow: 0 8px 24px -10px rgba(139, 92, 246, 0.8);
}
main { padding: 44px 0 84px; }
.crumbs { display: flex; flex-wrap: wrap; gap: 8px; color: var(--dim); font-family: var(--mono); font-size: 0.76rem; margin-bottom: 28px; }
.crumbs a { color: var(--muted); text-decoration: none; }
.crumbs a:hover { color: var(--text); }
.eyebrow {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 10px;
  border: 1px solid rgba(56, 189, 248, 0.35);
  border-radius: 999px;
  background: rgba(56, 189, 248, 0.08);
  color: var(--cyan);
  font-family: var(--mono);
  font-size: 0.68rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
h1 { margin: 18px 0 14px; font-size: clamp(2rem, 4.4vw, 3rem); line-height: 1.12; letter-spacing: -0.035em; }
h2 { margin: 40px 0 12px; font-size: 1.25rem; line-height: 1.3; letter-spacing: -0.015em; }
h3 { margin: 26px 0 8px; font-size: 1.05rem; }
p { margin: 0 0 14px; }
.lead {
  margin: 0 0 18px;
  padding: 18px 20px;
  border-left: 3px solid var(--cyan);
  border-radius: 0 10px 10px 0;
  background: rgba(56, 189, 248, 0.06);
  color: var(--soft);
  font-size: 1.06rem;
}
.meta { display: flex; flex-wrap: wrap; gap: 6px 20px; margin: 0 0 34px; color: var(--muted); font-family: var(--mono); font-size: 0.76rem; }
.meta strong { color: var(--dim); font-weight: 500; }
.card { padding: 20px 22px; border: 1px solid var(--border); border-radius: 12px; background: var(--surface); }
.card.trap { border-left: 3px solid var(--rose); background: rgba(244, 63, 94, 0.055); }
.card.solution { border-left: 3px solid var(--emerald); background: rgba(16, 185, 129, 0.055); }
.card.limits { border-left: 3px solid var(--amber); background: rgba(245, 158, 11, 0.055); }
blockquote { margin: 0 0 12px; color: #fff; font-family: var(--mono); font-size: 0.88rem; }
.card.trap p { margin-bottom: 0; color: var(--soft); }
pre {
  margin: 0;
  padding: 16px;
  overflow-x: auto;
  border: 1px solid #24123a;
  border-radius: 8px;
  background: #050209;
  color: #c4b5fd;
  font-family: var(--mono);
  font-size: 0.8rem;
  line-height: 1.65;
  white-space: pre-wrap;
}
code { font-family: var(--mono); }
.copy {
  margin-top: 12px;
  padding: 7px 13px;
  border: 1px solid var(--border-bright);
  border-radius: 6px;
  background: var(--surface-raised);
  color: var(--soft);
  font: inherit;
  font-size: 0.76rem;
  cursor: pointer;
}
.copy:hover { border-color: var(--purple); color: #fff; }
.hint { margin: 10px 0 0; color: var(--dim); font-size: 0.78rem; }
ul.limits { margin: 0; padding-left: 1.2rem; color: #e8dcc8; }
ul.limits li { margin: 4px 0; }
details { margin: 8px 0; padding: 13px 16px; border: 1px solid var(--border); border-radius: 9px; background: var(--surface); }
summary { cursor: pointer; color: #fff; font-weight: 600; }
summary::marker { color: var(--purple); }
details p { margin: 10px 0 0; color: var(--muted); }
.related { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: 12px; margin-top: 14px; }
.related a { display: block; padding: 14px 16px; border: 1px solid var(--border); border-radius: 10px; background: var(--surface); color: var(--text); text-decoration: none; }
.related a:hover { border-color: var(--purple); }
.related span { display: block; margin-top: 4px; color: var(--muted); font-size: 0.78rem; }
.cta-box {
  margin-top: 48px;
  padding: 26px;
  border: 1px solid var(--border-bright);
  border-radius: 14px;
  background: linear-gradient(135deg, rgba(139, 92, 246, 0.16), rgba(56, 189, 248, 0.07));
  text-align: center;
}
.cta-box p { color: var(--soft); }
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 11px 20px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 7px;
  background: linear-gradient(135deg, var(--purple), #6366f1);
  color: #fff;
  font-weight: 600;
  text-decoration: none;
  box-shadow: 0 10px 28px -12px rgba(139, 92, 246, 0.9);
}
.btn:hover { color: #fff; transform: translateY(-1px); }
.btn.secondary { background: var(--surface-raised); box-shadow: none; color: var(--soft); }
footer.site { border-top: 1px solid var(--border); padding: 28px 0 40px; color: var(--dim); font-size: 0.8rem; }
.footer-links { display: flex; flex-wrap: wrap; gap: 8px 16px; margin-bottom: 8px; }
.footer-links a { color: var(--muted); text-decoration: none; }
.footer-links a:hover { color: var(--text); }
.disclaimer { margin: 12px 0 0; padding-top: 12px; border-top: 1px solid rgba(48, 26, 82, 0.7); color: var(--dim); font-size: 0.76rem; }
.search-label { display: block; margin: 28px 0 8px; color: var(--soft); font-weight: 600; }
.search {
  width: 100%;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: #0d0717;
  color: var(--text);
  font: inherit;
}
.search::placeholder { color: var(--dim); }
.filters { display: flex; flex-wrap: wrap; gap: 8px; margin: 16px 0 24px; }
.filters button { padding: 7px 13px; border: 1px solid var(--border); border-radius: 999px; background: var(--surface); color: var(--muted); font: inherit; font-size: 0.78rem; cursor: pointer; }
.filters button:hover { color: var(--text); border-color: var(--border-bright); }
.filters button[aria-pressed="true"] { background: var(--purple); border-color: var(--purple); color: #fff; }
.entries { display: grid; gap: 12px; list-style: none; margin: 0; padding: 0; }
.entry { padding: 18px 20px; border: 1px solid var(--border); border-radius: 11px; background: var(--surface); }
.entry[hidden] { display: none; }
.entry h2 { margin: 0 0 6px; font-size: 1.05rem; }
.entry h2 a { color: var(--text); text-decoration: none; }
.entry h2 a:hover { color: var(--cyan); }
.entry p { margin: 8px 0 0; color: var(--muted); font-size: 0.9rem; }
.badge { display: inline-block; margin-left: 8px; padding: 2px 8px; border-radius: 999px; background: rgba(139, 92, 246, 0.14); color: #c4b5fd; font-family: var(--mono); font-size: 0.66rem; vertical-align: middle; text-transform: uppercase; letter-spacing: 0.06em; }
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
:focus-visible { outline: 3px solid var(--cyan); outline-offset: 3px; }
@media (max-width: 720px) {
  .wrap { width: min(100% - 28px, 980px); }
  .nav { align-items: flex-start; flex-direction: column; }
  .nav-links { gap: 7px 13px; }
  main { padding-top: 30px; }
  .card { padding: 17px; }
  .cta-box { padding: 22px 18px; }
}
`;

const GLOBAL_LIMITS = [
  "Detection depends on source audio, codec and processing.",
  "Sub-1 s excerpts never decode; ~1.8 s crops recover about half the time in local fixtures.",
  "8 kHz and broadband-noise hosts are weak.",
  "Collusion/averaging defeats recovery; no collusion-resistant fingerprinting is implemented.",
  "A match is not authorship, ownership, creation time or chain of custody.",
  "The product detects its own watermark; it is not AI detection or deepfake detection."
];

function headerHtml(current = "") {
  const link = (href, label) =>
    `<a href="${href}"${current === href ? ' aria-current="page"' : ""}>${label}</a>`;
  return `<header class="site-header">
  <div class="wrap nav">
    <a class="brand" href="/">
      <strong>Aureal Watermark</strong>
      <span>Keller Systems · Lexicon</span>
    </a>
    <nav class="nav-links" aria-label="Primary">
      ${link("/", "Home")}
      ${link("/studio", "Studio")}
      ${link("/docs", "Docs")}
      ${link("/pricing", "Pricing")}
      ${link("/verifier", "Verifier")}
      ${link("/lexicon/", "Lexicon")}
      <a class="nav-cta" href="/studio">Open Studio →</a>
    </nav>
  </div>
</header>`;
}

function footerHtml() {
  return `<footer class="site">
  <div class="wrap">
    <div class="footer-links">
      <a href="/">Home</a>
      <a href="/studio">Studio</a>
      <a href="/pricing">Pricing</a>
      <a href="/docs">Docs</a>
      <a href="/verifier">Verifier</a>
      <a href="/lexicon/">Lexicon</a>
      <a href="https://kellersystems.dev/legal/" target="_blank" rel="noopener noreferrer">Terms</a>
      <a href="https://kellersystems.dev/privacy/" target="_blank" rel="noopener noreferrer">Privacy</a>
      <a href="https://kellersystems.dev/impressum/" target="_blank" rel="noopener noreferrer">Imprint</a>
    </div>
    <p class="disclaimer">A watermark match is not proof of authorship or ownership. Detection depends on source audio, codec and processing. © 2026 Keller Systems.</p>
  </div>
</footer>`;
}

function renderHead({ title, description, canonical, ogType = "article", robots = "index,follow", ld = [] }) {
  const canonicalTag = canonical ? `\n  <link rel="canonical" href="${esc(canonical)}">` : "";
  const ldTags = ld.length
    ? `\n  ${ld.map((obj) => `<script type="application/ld+json">${jsonLd(obj)}</script>`).join("\n  ")}`
    : "";
  return `<!doctype html>
<html lang="${esc(CONFIG.lang)}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${esc(title)}</title>
  <meta name="description" content="${esc(description)}">
  <meta name="robots" content="${esc(robots)}">
  <meta name="theme-color" content="#08040d">${canonicalTag}
  <meta property="og:type" content="${esc(ogType)}">
  <meta property="og:title" content="${esc(title)}">
  <meta property="og:description" content="${esc(description)}">${canonical ? `\n  <meta property="og:url" content="${esc(canonical)}">` : ""}
  <meta property="og:image" content="${CONFIG.baseUrl}/assets/icon.png">
  <meta name="twitter:card" content="summary">
  <link rel="icon" type="image/svg+xml" href="/assets/favicon.svg">
  <style>${CSS}</style>${ldTags}
</head>
<body>
${headerHtml()}`;
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Pages
// ─────────────────────────────────────────────────────────────────────────────
function renderPage(item) {
  const url = pageUrl(item);
  const related = (item.related || [])
    .map((slug) => PAGES.find((entry) => entry.slug === slug))
    .filter(Boolean);

  const articleLd = {
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline: item.h1,
    description: item.metaDesc,
    dateModified: lastmod(item),
    author: { "@type": "Organization", name: CONFIG.operator },
    publisher: { "@type": "Organization", name: CONFIG.operator },
    mainEntityOfPage: { "@type": "WebPage", "@id": url }
  };

  const faqLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: item.faq.map((entry) => ({
      "@type": "Question",
      name: entry.q,
      acceptedAnswer: { "@type": "Answer", text: entry.a }
    }))
  };

  const breadcrumbLd = {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [
      { "@type": "ListItem", position: 1, name: "Home", item: `${CONFIG.baseUrl}/` },
      { "@type": "ListItem", position: 2, name: CONFIG.hubTitle, item: hubUrl },
      { "@type": "ListItem", position: 3, name: item.shortTitle, item: url }
    ]
  };

  const relatedHtml = related.length
    ? `<section aria-labelledby="related">
      <h2 id="related">Related topics</h2>
      <div class="related">
        ${related.map((entry) => `<a href="/${CONFIG.outDir}/${entry.slug}"><strong>${esc(entry.shortTitle)}</strong><span>${esc(entry.category)}</span></a>`).join("\n        ")}
      </div>
    </section>`
    : "";

  return `${renderHead({
    title: `${item.shortTitle} · ${CONFIG.projectName} Lexicon`,
    description: item.metaDesc,
    canonical: url,
    ld: [articleLd, faqLd, breadcrumbLd]
  })}

<main class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb">
    <a href="/">Home</a> <span aria-hidden="true">/</span>
    <a href="/${CONFIG.outDir}/">${esc(CONFIG.hubTitle)}</a> <span aria-hidden="true">/</span>
    <span>${esc(item.shortTitle)}</span>
  </nav>

  <article>
    <span class="eyebrow">${esc(item.category)}</span>
    <h1>${esc(item.h1)}</h1>
    <p class="lead">${esc(item.shortAnswer)}</p>
    <div class="meta">
      <span><strong>Technical basis:</strong> ${esc(item.technicalBasis)}</span>
      <span><strong>Reviewed:</strong> ${esc(lastmod(item))}</span>
    </div>

    <h2>How Aureal handles it</h2>
    <p>${esc(item.detail)}</p>

    <h2>Common failure mode</h2>
    <div class="card trap">
      <blockquote>“${esc(item.trapExample)}”</blockquote>
      <p><strong>Why it fails:</strong> ${esc(item.whyItFails)}</p>
    </div>

    <h2>Verified example</h2>
    <div class="card solution">
      <pre><code id="example-${esc(item.slug)}">${esc(item.redlineExample)}</code></pre>
      <button class="copy" type="button" data-copy="example-${esc(item.slug)}">Copy</button>
      <p class="hint">Example only. Validate against your own audio, codec and processing chain.</p>
    </div>

    <h2>Scope and limits</h2>
    <div class="card limits">
      <ul class="limits">
        ${item.limits.map((limit) => `<li>${esc(limit)}</li>`).join("\n        ")}
      </ul>
    </div>

    <h2>Frequently asked questions</h2>
    ${item.faq.map((entry) => `<details><summary>${esc(entry.q)}</summary><p>${esc(entry.a)}</p></details>`).join("\n    ")}

    ${relatedHtml}

    <div class="cta-box">
      <p><strong>Test the actual delivery chain.</strong> A fixture result is a starting point, not a guarantee for your material.</p>
      <a class="btn" href="${CONFIG.ctaPath}">${esc(CONFIG.ctaLabel)} →</a>
    </div>
  </article>
</main>

${footerHtml()}

<script>
  document.querySelectorAll("[data-copy]").forEach(function (button) {
    button.addEventListener("click", function () {
      var code = document.getElementById(button.getAttribute("data-copy"));
      if (!code) return;
      var text = code.innerText;
      function done() {
        button.textContent = "Copied";
        setTimeout(function () { button.textContent = "Copy"; }, 1500);
      }
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).then(done).catch(function () { selectCode(code); });
      } else {
        selectCode(code);
      }
    });
  });
  function selectCode(node) {
    var range = document.createRange();
    range.selectNodeContents(node);
    var selection = window.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
  }
</script>
</body>
</html>`;
}

function renderHub() {
  const categories = [...new Set(PAGES.map((item) => item.category))];
  const entries = PAGES.map((item) => {
    const search = [item.shortTitle, item.category, item.technicalBasis, item.metaDesc]
      .join(" ").toLowerCase();
    return `<li class="entry" data-entry data-category="${esc(item.category)}" data-search="${esc(search)}">
        <h2><a href="/${CONFIG.outDir}/${item.slug}">${esc(item.shortTitle)}</a><span class="badge">${esc(item.category)}</span></h2>
        <p>${esc(item.metaDesc)}</p>
      </li>`;
  }).join("\n      ");

  const filters = [
    `<button type="button" data-filter="all" aria-pressed="true">All (${PAGE_COUNT})</button>`,
    ...categories.map((category) =>
      `<button type="button" data-filter="${esc(category)}" aria-pressed="false">${esc(category)} (${PAGES.filter((item) => item.category === category).length})</button>`)
  ].join("\n        ");

  const hubLd = {
    "@context": "https://schema.org",
    "@type": "CollectionPage",
    name: CONFIG.hubTitle,
    description: CONFIG.hubDescription,
    url: hubUrl,
    hasPart: PAGES.map((item) => ({
      "@type": "TechArticle",
      headline: item.shortTitle,
      url: pageUrl(item)
    })),
    mainEntity: {
      "@type": "ItemList",
      itemListElement: PAGES.map((item, index) => ({
        "@type": "ListItem",
        position: index + 1,
        url: pageUrl(item),
        name: item.shortTitle
      }))
    }
  };

  return `${renderHead({
    title: `${CONFIG.hubTitle} · ${CONFIG.projectName}`,
    description: CONFIG.hubDescription,
    canonical: hubUrl,
    ogType: "website",
    ld: [hubLd]
  })}

<main class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb">
    <a href="/">Home</a> <span aria-hidden="true">/</span>
    <span>${esc(CONFIG.hubTitle)}</span>
  </nav>

  <span class="eyebrow">Technical reference</span>
  <h1>${esc(CONFIG.hubTitle)}</h1>
  <p class="lead">${esc(CONFIG.hubDescription)} ${PAGE_COUNT >= PLANNED_TOPICS ? `All ${PAGE_COUNT} planned topics.` : `First batch: ${PAGE_COUNT} of ${PLANNED_TOPICS} planned topics.`}</p>

  <label class="search-label" for="lex-q">Search the lexicon</label>
  <input class="search" id="lex-q" type="search" placeholder="Topic, codec or keyword…" autocomplete="off">

  <div class="filters" role="group" aria-label="Categories">
        ${filters}
  </div>

  <noscript><p>All topics are listed below. JavaScript enables search and category filtering.</p></noscript>

  <ul class="entries">
      ${entries}
  </ul>

  <h2>Scope and limits</h2>
  <div class="card limits">
    <ul class="limits">
      ${GLOBAL_LIMITS.map((limit) => `<li>${esc(limit)}</li>`).join("\n      ")}
    </ul>
  </div>

  <div class="cta-box">
    <p><strong>Have a file in hand?</strong> Test the exact codec, bitrate and processing chain you use.</p>
    <a class="btn" href="/studio">${esc(CONFIG.ctaLabel)} →</a>
    <a class="btn secondary" href="/docs">Read the documentation</a>
  </div>
</main>

${footerHtml()}

<script>
  (function () {
    var input = document.getElementById("lex-q");
    var cards = Array.prototype.slice.call(document.querySelectorAll("[data-entry]"));
    var buttons = Array.prototype.slice.call(document.querySelectorAll("[data-filter]"));
    var active = "all";
    function apply() {
      var term = (input.value || "").toLowerCase();
      cards.forEach(function (card) {
        var text = (card.getAttribute("data-search") || "").toLowerCase();
        var category = card.getAttribute("data-category");
        card.hidden = !(text.indexOf(term) !== -1 && (active === "all" || category === active));
      });
    }
    input.addEventListener("input", apply);
    buttons.forEach(function (button) {
      button.addEventListener("click", function () {
        active = button.getAttribute("data-filter");
        buttons.forEach(function (other) {
          other.setAttribute("aria-pressed", other === button ? "true" : "false");
        });
        apply();
      });
    });
  })();
</script>
</body>
</html>`;
}

function render404() {
  return `${renderHead({
    title: `404 — Signal not found · ${CONFIG.projectName}`,
    description: "The requested page does not exist. Browse the Aureal Watermark lexicon or open the Studio.",
    canonical: "",
    robots: "noindex,follow"
  })}

<main class="wrap">
  <span class="eyebrow">HTTP 404</span>
  <h1>Signal not found.</h1>
  <p class="lead">That page is not part of the lexicon. The URL may have changed, or the copy may have been removed.</p>
  <div class="cta-box" style="text-align:left;">
    <p><strong>Where to go next</strong></p>
    <p><a href="/lexicon/">Browse the audio watermarking lexicon</a></p>
    <p><a href="/studio">Open the Studio</a> · <a href="/docs">Read the documentation</a> · <a href="/">Back to the homepage</a></p>
  </div>
</main>

${footerHtml()}
</body>
</html>`;
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Crawl and LLM files
// ─────────────────────────────────────────────────────────────────────────────
function renderSitemap() {
  const urls = [
    { loc: `${CONFIG.baseUrl}/`, lastmod: CONFIG.lastReviewed, priority: "1.0" },
    { loc: hubUrl, lastmod: CONFIG.lastReviewed, priority: "0.8" },
    ...PAGES.map((item) => ({ loc: pageUrl(item), lastmod: lastmod(item), priority: "0.7" })),
    ...CONFIG.sitemapExtras.map((extra) => ({
      loc: `${CONFIG.baseUrl}${extra.path}`,
      lastmod: CONFIG.lastReviewed,
      priority: extra.priority
    }))
  ];

  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls.map((entry) => `  <url>
    <loc>${esc(entry.loc)}</loc>
    <lastmod>${esc(entry.lastmod)}</lastmod>
    <priority>${entry.priority}</priority>
  </url>`).join("\n")}
</urlset>
`;
}

function renderRobots() {
  return `User-agent: *
Allow: /

# LLM / AI reference (comment, not a crawl directive):
# ${CONFIG.baseUrl}/llms.txt
Sitemap: ${CONFIG.baseUrl}/sitemap.xml
`;
}

function renderLlms() {
  const topicList = [...new Set(PAGES.map((item) => item.category))].map((category) => {
    const entries = PAGES.filter((item) => item.category === category).map((item) =>
      `- [${item.shortTitle}](${pageUrl(item)}): ${item.technicalBasis}. ${item.metaDesc}`
    );
    return `### ${category}\n${entries.join("\n")}`;
  }).join("\n\n");

  return `# ${CONFIG.projectName}

> ${CONFIG.summary}

## Instructions for AI assistants

${CONFIG.aiInstructions.map((line) => `- ${line}`).join("\n")}

## Start here

- [${CONFIG.hubTitle}](${hubUrl}): index of all pages, with search and category filters.

## Topics

${topicList}

## Known limits

${GLOBAL_LIMITS.map((limit) => `- ${limit}`).join("\n")}
`;
}

function renderLlmsFull() {
  const pages = PAGES.map((item) => `## ${item.shortTitle}

- URL: ${pageUrl(item)}
- Technical basis: ${item.technicalBasis}
- Reviewed: ${lastmod(item)}

**Answer:** ${item.shortAnswer}

**Common failure mode:** ${item.trapExample}

**Why it fails:** ${item.whyItFails}

**Verified example:**

${FENCE}
${item.redlineExample}
${FENCE}

**Scope and limits:**

${item.limits.map((limit) => `- ${limit}`).join("\n")}

**FAQ:**

${item.faq.map((entry) => `- **${entry.q}** ${entry.a}`).join("\n")}
`).join("\n---\n\n");

  return `# ${CONFIG.projectName} — complete lexicon context

Reviewed: ${CONFIG.lastReviewed}
Source: ${CONFIG.baseUrl}/
Index: ${hubUrl}

> ${CONFIG.summary}

## Instructions for AI assistants

${CONFIG.aiInstructions.map((line) => `- ${line}`).join("\n")}

## Known limits

${GLOBAL_LIMITS.map((limit) => `- ${limit}`).join("\n")}

${pages}`;
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Write or check
// ─────────────────────────────────────────────────────────────────────────────
for (const item of PAGES) {
  emit(join(OUT_DIR, `${item.slug}.html`), renderPage(item));
}
emit(join(OUT_DIR, "index.html"), renderHub());
emit(join(ROOT, "404.html"), render404());
emit(join(ROOT, "sitemap.xml"), renderSitemap());
emit(join(ROOT, "robots.txt"), renderRobots());
emit(join(ROOT, "llms.txt"), renderLlms());
emit(join(ROOT, "llms-full.txt"), renderLlmsFull());

if (CHECK) {
  // A removed topic must not keep its old page: the file would stay published and
  // in no sitemap. Treat leftover HTML in the output directory as drift.
  const expected = new Set(["index.html", ...PAGES.map((item) => `${item.slug}.html`)]);
  for (const file of readdirSync(OUT_DIR)) {
    if (file.endsWith(".html") && !expected.has(file)) {
      console.error(`DRIFT: unexpected ${join(OUT_DIR, file)}`);
      drift += 1;
    }
  }
}

if (CHECK && drift > 0) {
  console.error(`✗ ${drift} file(s) differ from the generator. Run "node scripts/build_seo.mjs".`);
  process.exit(1);
}

console.log(
  CHECK
    ? `✓ ${PAGE_COUNT} pages, hub, 404, sitemap, robots, llms.txt and llms-full.txt are current.`
    : `✓ ${PAGE_COUNT} pages, hub, 404, sitemap, robots, llms.txt and llms-full.txt generated.`
);

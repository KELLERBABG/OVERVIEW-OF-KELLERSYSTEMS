/**
 * Generates the commercial pages for BauLens from site.config.json.
 *
 *   node scripts/build-site.mjs
 *
 * Outputs
 *   legal/impressum/index.html     § 5 DDG provider information
 *   legal/datenschutz/index.html   privacy policy (DSGVO)
 *   legal/agb/index.html           terms and conditions
 *   legal/lizenz/index.html        licence terms (EULA)
 *   legal/widerruf/index.html      cancellation policy
 *   pricing/index.html             plan overview with Polar checkout
 *   checkout/success/index.html    post-purchase landing page
 *   downloads/index.html           download hub with OS detection
 *   pricing.block.html             fragment injected into index.html
 *
 * The JSON config is the single source of truth for prices and checkout links,
 * so a price change is one edit and every surface stays consistent.
 */
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { formatEuro } from "./pricing-format.mjs";

const CONFIG = JSON.parse(readFileSync("site.config.json", "utf8"));
const { product, billing, legal, tiers, enterprise, downloads } = CONFIG;

/* The @font-face rules are inlined into every generated page rather than
   linked, and the Google Fonts tags are gone. Two reasons, both required by the
   desktop build: a page must render identically with no network, and a
   stylesheet request would be a second thing to make work under the app's asset
   protocol. Reading the file here keeps these pages in step with index.html,
   studio.html and docs.html, which are patched by scripts/setup-fonts.mjs. */
const FONT_CSS = readFileSync("assets/fonts/fonts.css", "utf8")
  .split("\n")
  .filter((line) => !/^\s*(\/\*|\*)/.test(line))
  .join("\n")
  .trim();
const FONT_STYLE = `/* Self-hosted IBM Plex; see assets/fonts/OFL.txt */\n${FONT_CSS}\n`;

/* --------------------------------------------------------------------------
 * Helpers
 * ------------------------------------------------------------------------ */

/* Prices are formatted by one shared module so the generated pages and
   scripts/check-prices.mjs cannot disagree about what "1.490 €" looks like. */
const money = (n) => formatEuro(n);
const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

function assert(condition, message) {
  if (!condition) {
    console.error(`FAILED: ${message}`);
    process.exit(1);
  }
}

function write(path, html) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, html, "utf8");
  console.log(`  ${path.padEnd(34)} ${html.length} chars`);
}

/** Shared head + CSS. `depth` is the number of directory levels to the repo root. */
function page({ title, description, depth, body, extraCss = "" }) {
  const up = "../".repeat(depth);
  return `<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>${esc(title)} &mdash; BauLens</title>
<meta name="description" content="${esc(description)}">
<meta name="color-scheme" content="dark">
<meta name="robots" content="index, follow">
<link rel="icon" type="image/svg+xml" href="${up}assets/icon.svg">
<style>
${FONT_STYLE}
  :root {
    --ink-900: #05070c; --ink-800: #080b12; --ink-700: #0b0f18; --ink-600: #101623; --ink-500: #161e2e;
    --line: #1a2434; --line-strong: #26334a; --ink-line: #33415a;
    --accent: #2fb6cf; --accent-hi: #57cde2;
    --accent-wash: rgba(47, 182, 207, 0.08); --accent-edge: rgba(47, 182, 207, 0.32);
    --on-accent: #04141a;
    --warn: #d9a441; --warn-wash: rgba(217, 164, 65, 0.1);
    --fg: #e9edf3; --fg-muted: #98a4b6; --fg-dim: #6b7788; --fg-faint: #4d5768;
    --font-sans: 'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    --font-mono: 'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    --r-sm: 3px; --r-md: 5px; --r-lg: 8px;
    --dur-fast: 120ms; --ease-out: cubic-bezier(0.22, 0.61, 0.36, 1);
    --content: 74ch;
  }
  *, *::before, *::after { box-sizing: border-box; }
  html { scroll-behavior: smooth; scroll-padding-top: 88px; }
  body {
    margin: 0; min-height: 100vh;
    background-color: var(--ink-900);
    background-image:
      linear-gradient(to right, rgba(255,255,255,0.028) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(255,255,255,0.028) 1px, transparent 1px);
    background-size: 88px 88px; background-attachment: fixed;
    color: var(--fg); font-family: var(--font-sans); font-size: 1rem; line-height: 1.7;
    overflow-x: clip; -webkit-font-smoothing: antialiased;
  }
  :focus-visible { outline: 2px solid var(--accent-hi); outline-offset: 2px; border-radius: var(--r-sm); }
  ::selection { background: var(--accent-edge); color: var(--fg); }
  .skip-link {
    position: absolute; left: 1rem; top: -100px; z-index: 100;
    padding: 0.6rem 1rem; background: var(--ink-500); border: 1px solid var(--accent);
    border-radius: var(--r-md); font-family: var(--font-mono); font-size: 0.8125rem; text-decoration: none;
    transition: top var(--dur-fast) var(--ease-out);
  }
  .skip-link:focus { top: 1rem; }

  .site-header {
    position: sticky; top: 0; z-index: 60;
    background: rgba(5,7,12,0.88); border-bottom: 1px solid var(--line);
    backdrop-filter: blur(12px) saturate(140%);
  }
  .header-inner {
    display: flex; align-items: center; justify-content: space-between; gap: 1.5rem;
    height: 64px; max-width: 1200px; margin: 0 auto; padding: 0 1.5rem;
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
    font-family: var(--font-mono); font-size: 0.62rem; letter-spacing: 0.12em;
    text-transform: uppercase; color: var(--fg-dim);
  }
  .header-actions { display: flex; align-items: center; gap: 0.5rem; }
  .header-link {
    font-size: 0.875rem; color: var(--fg-muted); text-decoration: none;
    padding: 0.4rem 0.6rem; border-radius: var(--r-md);
    transition: color var(--dur-fast) var(--ease-out), background-color var(--dur-fast) var(--ease-out);
  }
  .header-link:hover { color: var(--fg); background: var(--ink-600); }
  .header-cta {
    font-size: 0.8125rem; font-weight: 550; text-decoration: none; color: var(--on-accent);
    background: var(--accent); padding: 0.5rem 0.9rem; border-radius: var(--r-md);
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  .header-cta:hover { background: var(--accent-hi); }

  .shell { max-width: 1200px; margin: 0 auto; padding: 3rem 1.5rem 6rem; }
  .prose { max-width: var(--content); }
  .prose h1 {
    font-size: clamp(1.75rem, 1.35rem + 1.6vw, 2.4rem); font-weight: 300;
    line-height: 1.14; letter-spacing: -0.026em; margin: 0 0 1.25rem;
  }
  .prose h2 {
    font-size: clamp(1.2rem, 1.08rem + 0.6vw, 1.45rem); font-weight: 600;
    letter-spacing: -0.018em; margin: 2.75rem 0 0.85rem; padding-top: 1.5rem;
    border-top: 1px solid var(--line);
  }
  .prose h3 { font-size: 1.05rem; font-weight: 600; margin: 1.9rem 0 0.6rem; }
  .prose p { margin: 0 0 1.05rem; color: var(--fg-muted); }
  .prose strong { color: var(--fg); font-weight: 600; }
  .prose a { color: var(--accent-hi); text-decoration: none; border-bottom: 1px solid var(--accent-edge); }
  .prose a:hover { border-bottom-color: var(--accent-hi); }
  .prose ul, .prose ol { margin: 0 0 1.05rem; padding-left: 1.35rem; color: var(--fg-muted); }
  .prose li { margin-bottom: 0.4rem; }
  .prose ul li::marker { color: var(--fg-faint); }
  .prose ol li::marker { color: var(--accent); font-family: var(--font-mono); font-size: 0.85em; }
  .prose code {
    font-family: var(--font-mono); font-size: 0.86em; background: var(--ink-600);
    border: 1px solid var(--line); border-radius: var(--r-sm); padding: 0.1em 0.35em; color: var(--fg);
  }
  .prose hr { border: 0; border-top: 1px solid var(--line); margin: 2.5rem 0; }
  .prose .lead { font-size: 1.05rem; color: var(--fg-muted); }
  .prose .meta { font-family: var(--font-mono); font-size: 0.75rem; color: var(--fg-dim); letter-spacing: 0.04em; }

  .callout {
    max-width: var(--content); margin: 1.25rem 0; padding: 0.95rem 1.15rem;
    background: var(--warn-wash); border: 1px solid rgba(217,164,65,0.32);
    border-radius: var(--r-md); color: var(--fg-muted); font-size: 0.9375rem;
  }
  .callout strong { color: var(--fg); }
  .callout--accent { background: var(--accent-wash); border-color: var(--accent-edge); }

  .data-block {
    max-width: var(--content);
    background: var(--ink-800); border: 1px solid var(--line); border-radius: var(--r-lg);
    padding: 1.15rem 1.35rem; margin: 1.25rem 0;
    font-size: 0.9375rem; color: var(--fg-muted);
  }
  .data-block p { margin: 0 0 0.35rem; }
  .data-block p:last-child { margin-bottom: 0; }
  .data-block strong { color: var(--fg); }

  .site-footer { border-top: 1px solid var(--line); background: var(--ink-800); }
  .footer-inner {
    max-width: 1200px; margin: 0 auto; padding: 2.5rem 1.5rem;
    display: flex; flex-wrap: wrap; gap: 1.5rem; justify-content: space-between;
  }
  .footer-links { display: flex; flex-wrap: wrap; gap: 1.25rem; list-style: none; margin: 0; padding: 0; }
  .footer-links a { font-size: 0.875rem; color: var(--fg-muted); text-decoration: none; }
  .footer-links a:hover { color: var(--accent-hi); }
  .footer-note { font-size: 0.8125rem; color: var(--fg-dim); max-width: 62ch; margin: 0; }
  .footer-meta { font-family: var(--font-mono); font-size: 0.72rem; color: var(--fg-faint); }

  @media (max-width: 720px) {
    .header-link { display: none; }
    .shell { padding: 2rem 1.1rem 4rem; }
    .header-inner { padding: 0 1.1rem; }
  }
  @media (prefers-reduced-motion: reduce) {
    html { scroll-behavior: auto; }
    *, *::before, *::after { animation-duration: 0.001ms !important; transition-duration: 0.001ms !important; }
  }
  @media print {
    body { background: #fff; color: #000; }
    .site-header, .site-footer, .skip-link { display: none !important; }
    .prose p, .prose li, .data-block, .callout { color: #000 !important; }
    .prose h1, .prose h2, .prose h3 { color: #000 !important; border-color: #ccc !important; }
    .prose a { color: #000 !important; border: 0 !important; }
  }
${extraCss}
</style>
</head>
<body>
<a class="skip-link" href="#main">Zum Inhalt springen</a>

<header class="site-header">
  <div class="header-inner">
    <a class="brand" href="${up}index.html">
      <span class="brand-mark"><img src="${up}assets/icon.svg" alt="" width="22" height="22"></span>
      <span class="brand-text">
        <span class="brand-name">BauLens</span>
        <span class="brand-owner">${esc(legal.providerTradingAs)}</span>
      </span>
    </a>
    <div class="header-actions">
      <a class="header-link" href="${up}pricing/">Preise</a>
      <a class="header-link" href="${up}downloads/">Downloads</a>
      <a class="header-link" href="${up}docs.html">Dokumentation</a>
      <a class="header-cta" href="${up}downloads/">BauLens herunterladen</a>
    </div>
  </div>
</header>

<main class="shell" id="main">
${body}
</main>

<footer class="site-footer">
  <div class="footer-inner">
    <div>
      <p class="footer-note">${esc(product.name)} &mdash; deterministische Bauordnungs-Vorprüfung. Vorprüfung, keine Rechtsberatung.</p>
      <p class="footer-meta">&copy; 2026 ${esc(legal.providerName)} &bull; ${esc(product.vendor)}</p>
    </div>
    <ul class="footer-links">
      <li><a href="${up}pricing/">Preise</a></li>
      <li><a href="${up}legal/impressum/">Impressum</a></li>
      <li><a href="${up}legal/datenschutz/">Datenschutz</a></li>
      <li><a href="${up}legal/agb/">AGB</a></li>
      <li><a href="${up}legal/lizenz/">Lizenz</a></li>
      <li><a href="${up}legal/widerruf/">Widerruf</a></li>
      <li><a href="${up}docs.html">Dokumentation</a></li>
    </ul>
  </div>
</footer>
</body>
</html>
`;
}

/* --------------------------------------------------------------------------
 * Legal content
 * ------------------------------------------------------------------------ */

const providerBlock = `<div class="data-block">
  <p><strong>${esc(legal.providerName)}</strong></p>
  <p>${esc(legal.providerTradingAs)}</p>
  <p>${esc(legal.providerCareOf)}</p>
  <p>${esc(legal.providerStreet)}</p>
  <p>${esc(legal.providerPostalCode)} ${esc(legal.providerCity)}</p>
  <p>${esc(legal.providerCountry)}</p>
</div>`;

const legalPages = [
  {
    slug: "impressum",
    title: "Impressum",
    description: "Anbieterkennzeichnung nach § 5 DDG für BauLens.",
    body: `
<article class="prose">
  <h1>Impressum</h1>
  <p class="lead">Angaben gemäß § 5 Digitale-Dienste-Gesetz (DDG).</p>

  <h2>Diensteanbieter</h2>
  ${providerBlock}

  <h2>Kontakt</h2>
  <div class="data-block">
    <p><strong>E-Mail:</strong> <a href="mailto:${esc(legal.email)}">${esc(legal.email)}</a></p>
    <p><strong>Website:</strong> <a href="${esc(product.siteUrl)}">${esc(product.siteUrl)}</a></p>
  </div>

  <h2>Umsatzsteuer</h2>
  <p>${esc(legal.vatNote)}</p>

  <h2>Verantwortlich für den Inhalt nach § 18 Abs. 2 MStV</h2>
  <p>${esc(legal.providerName)}, Anschrift wie oben.</p>

  <h2>EU-Streitbeilegung</h2>
  <p>Die Europäische Kommission stellt eine Plattform zur Online-Streitbeilegung (OS) bereit:
  <a href="https://ec.europa.eu/consumers/odr" target="_blank" rel="noopener noreferrer">https://ec.europa.eu/consumers/odr</a>.
  Unsere E-Mail-Adresse findest du oben im Impressum.</p>

  <h2>Verbraucherstreitbeilegung</h2>
  <p>Wir sind nicht bereit oder verpflichtet, an Streitbeilegungsverfahren vor einer
  Verbraucherschlichtungsstelle teilzunehmen.</p>

  <h2>Haftung für Inhalte</h2>
  <p>Die Inhalte dieser Seiten wurden mit Sorgfalt erstellt. Für die Richtigkeit, Vollständigkeit und
  Aktualität der Inhalte können wir jedoch keine Gewähr übernehmen. Als Diensteanbieter sind wir für
  eigene Inhalte auf diesen Seiten nach den allgemeinen Gesetzen verantwortlich.</p>

  <div class="callout">
    <strong>Fachlicher Hinweis:</strong> BauLens ist ein Werkzeug zur technischen und planerischen
    Vorprüfung. Es erbringt keine Rechtsberatung im Sinne des Rechtsdienstleistungsgesetzes (RDG) und
    ersetzt weder die Prüfung durch eine bauvorlageberechtigte Person noch den Genehmigungsbescheid
    der zuständigen Bauaufsichtsbehörde.
  </div>
</article>`,
  },

  {
    slug: "datenschutz",
    title: "Datenschutzerklärung",
    description: "Informationen zur Verarbeitung personenbezogener Daten bei BauLens nach Art. 13 DSGVO.",
    body: `
<article class="prose">
  <h1>Datenschutzerklärung</h1>
  <p class="lead">Informationen über die Verarbeitung personenbezogener Daten nach Art. 13 und 14 DSGVO.</p>

  <h2>1. Verantwortlicher</h2>
  ${providerBlock}
  <p>E-Mail: <a href="mailto:${esc(legal.email)}">${esc(legal.email)}</a></p>

  <h2>2. Das Wichtigste zuerst: Deine Plandaten bleiben bei dir</h2>
  <p>Die Prüf-Engine von BauLens rechnet ausschließlich lokal &mdash; im Browser oder auf deinem
  eigenen Server. Das gilt für die gesamte Prüfung:</p>
  <ul>
    <li>Grundrisse, CAD- und BIM-Dateien, Baubeschreibungen und Raumdaten werden <strong>nicht</strong>
    an uns oder an Dritte übertragen.</li>
    <li>Der Prüfbefund, das Dossier und das SHA-256-Prüfsiegel entstehen vollständig auf deinem Gerät.</li>
    <li>Diese Website lädt <strong>keine</strong> Ressourcen von fremden Servern. Schriften, Stile und
    Skripte liegen im selben Verzeichnis wie die Seite und werden von dort geladen.</li>
    <li>Es gibt keine Telemetrie, keine Nutzungsstatistik, keine Analyse- und keine Marketing-Cookies.</li>
  </ul>
  <div class="callout callout--accent">
    <strong>Ausnahme, die du kennen solltest:</strong> Wenn du im Studio freiwillig eine Lizenz
    aktivierst, sendet das Studio deinen Lizenzschlüssel an die API von Polar, um ihn zu prüfen. Ohne
    diesen Schritt findet keine Verbindung statt. Details in Abschnitt 5.
  </div>

  <h2>3. Hosting</h2>
  <p>Diese Website wird über <strong>${esc(legal.hosting.name)}</strong> bereitgestellt, betrieben von
  ${esc(legal.hosting.operator)}.</p>
  <p>Beim Aufruf der Seiten erhebt Cloudflare automatisch technische Zugriffsdaten in Server-Logfiles
  (unter anderem IP-Adresse, Datum und Uhrzeit des Zugriffs, aufgerufene Ressource, übertragene
  Datenmenge, Browsertyp). Diese Verarbeitung ist erforderlich, um die Betriebssicherheit zu
  gewährleisten und Angriffe abzuwehren.</p>
  <div class="data-block">
    <p><strong>Rechtsgrundlage:</strong> Art. 6 Abs. 1 lit. f DSGVO (berechtigtes Interesse an
    sicherem und stabilem Betrieb).</p>
    <p><strong>Drittlandübermittlung:</strong> Cloudflare, Inc. hat seinen Sitz in den USA. Die
    Übermittlung ist durch ${esc(legal.hosting.transferSafeguard)} abgesichert.</p>
    <p><strong>Speicherdauer:</strong> Die Aufbewahrung richtet sich nach den Vorgaben von Cloudflare;
    wir haben auf die konkreten Fristen keinen Einfluss.</p>
  </div>
  <p>Die Auslieferung erfolgt über ${esc(legal.hosting.region)}.</p>

  <h2>4. Schriften und externe Ressourcen</h2>
  <p>Die verwendeten Schriften (IBM Plex Sans und IBM Plex Mono) sind Teil dieses Angebots und werden
  zusammen mit den Seiten ausgeliefert. Es besteht <strong>keine Verbindung zu einem Schriftarten-Dienst
  wie Google Fonts</strong>, und es wird keine IP-Adresse an einen solchen Dienst übertragen.</p>
  <p>Die Desktop-Anwendung von BauLens arbeitet vollständig offline. Sie ruft beim Start und während
  der Nutzung keine Ressourcen aus dem Netz ab.</p>
  <p>Soweit du einen Link zu einer externen Seite anklickst (etwa zum Quellcode-Repository oder zum
  Bezahlvorgang), gelten die Datenschutzbestimmungen des jeweiligen Anbieters.</p>

  <h2>5. Lizenzkauf und Lizenzaktivierung (Polar als Merchant of Record)</h2>
  <p>Der Verkauf kostenpflichtiger Lizenzen läuft über <strong>${esc(billing.provider)}</strong>
  (${esc(billing.providerLegalName)}, <a href="${esc(billing.providerUrl)}" target="_blank" rel="noopener noreferrer">${esc(billing.providerUrl)}</a>).
  Polar tritt dabei als <strong>Merchant of Record</strong> auf: Der Kaufvertrag kommt mit Polar
  zustande, nicht mit uns. Polar verarbeitet Zahlungsdaten, stellt Rechnungen aus und führt die
  Umsatzsteuer ab.</p>
  <div class="data-block">
    <p><strong>Was Polar verarbeitet:</strong> Name, E-Mail-Adresse, Rechnungsanschrift, ggf.
    Umsatzsteuer-Identifikationsnummer, Zahlungsdaten sowie Angaben zur abgeschlossenen Bestellung.</p>
    <p><strong>Was wir erhalten:</strong> Polar übermittelt uns den Namen und die E-Mail-Adresse der
    lizenzierten Person sowie Angaben zur Lizenz (Produkt, Umfang, Laufzeit). Kartendaten oder
    Bankverbindungen erhalten wir <strong>zu keinem Zeitpunkt</strong>.</p>
    <p><strong>Rechtsgrundlage:</strong> Art. 6 Abs. 1 lit. b DSGVO (Vertragserfüllung) sowie
    Art. 6 Abs. 1 lit. c DSGVO (steuer- und handelsrechtliche Aufbewahrungspflichten).</p>
  </div>
  <p><strong>Freiwillige Lizenzaktivierung im Studio.</strong> Trägst du im Studio einen
  Lizenzschlüssel ein, übermittelt dein Browser den Schlüssel und die Organisationskennung unseres
  Polar-Kontos an die öffentliche Prüfschnittstelle von Polar
  (<code>api.polar.sh/v1/customer-portal/license-keys/validate</code>). Übertragen werden dabei
  ausschließlich der Lizenzschlüssel, die Organisationskennung und technisch notwendige
  Verbindungsdaten. Plandaten sind nicht enthalten. Diesen Schritt löst du selbst aus; er ist für die
  Nutzung der Prüf-Engine nicht erforderlich.
  <br><strong>Rechtsgrundlage:</strong> Art. 6 Abs. 1 lit. b DSGVO (Erfüllung des Lizenzvertrags).</p>

  <h2>6. Kontaktaufnahme per E-Mail</h2>
  <p>Schreibst du uns eine E-Mail, verarbeiten wir deine Adresse und den Inhalt der Nachricht, um sie
  zu beantworten.</p>
  <div class="data-block">
    <p><strong>Rechtsgrundlage:</strong> Art. 6 Abs. 1 lit. b DSGVO bei vertragsbezogenen Anfragen,
    sonst Art. 6 Abs. 1 lit. f DSGVO (Interesse an der Beantwortung).</p>
    <p><strong>Speicherdauer:</strong> Wir löschen die Nachricht, sobald sie erledigt ist und keine
    gesetzlichen Aufbewahrungspflichten entgegenstehen.</p>
  </div>

  <h2>7. Cookies</h2>
  <p>Diese Website setzt <strong>keine</strong> Cookies zu Analyse-, Werbe- oder Trackingzwecken. Es
  werden ausschließlich Informationen im lokalen Speicher deines Browsers abgelegt, wenn du im Studio
  einen Lizenzschlüssel hinterlegst oder Eingaben zwischenspeicherst. Diese Daten verbleiben auf
  deinem Gerät und lassen sich jederzeit über die Einstellungen deines Browsers löschen.</p>

  <h2>8. Deine Rechte</h2>
  <p>Du hast gegenüber dem Verantwortlichen folgende Rechte:</p>
  <ul>
    <li><strong>Auskunft</strong> über die verarbeiteten Daten (Art. 15 DSGVO)</li>
    <li><strong>Berichtigung</strong> unrichtiger Daten (Art. 16 DSGVO)</li>
    <li><strong>Löschung</strong> (Art. 17 DSGVO)</li>
    <li><strong>Einschränkung der Verarbeitung</strong> (Art. 18 DSGVO)</li>
    <li><strong>Datenübertragbarkeit</strong> (Art. 20 DSGVO)</li>
    <li><strong>Widerspruch</strong> gegen Verarbeitungen auf Grundlage von Art. 6 Abs. 1 lit. f DSGVO
    (Art. 21 DSGVO)</li>
  </ul>
  <p>Für Daten, die Polar im Rahmen des Lizenzkaufs verarbeitet, ist Polar der richtige
  Ansprechpartner. Du kannst dich aber jederzeit an uns wenden; wir leiten dein Anliegen weiter.</p>

  <h2>9. Beschwerderecht bei der Aufsichtsbehörde</h2>
  <p>Du hast das Recht, dich bei einer Datenschutz-Aufsichtsbehörde zu beschweren (Art. 77 DSGVO).
  Zuständig ist die Behörde deines gewöhnlichen Aufenthaltsorts, deines Arbeitsplatzes oder des Orts
  des vermuteten Verstoßes.</p>

  <h2>10. Keine automatisierte Entscheidungsfindung</h2>
  <p>Eine automatisierte Entscheidungsfindung einschließlich Profiling im Sinne des Art. 22 DSGVO
  findet nicht statt. Die Prüfergebnisse von BauLens sind technische Berechnungen auf Grundlage
  deiner Eingaben und treffen keine rechtliche Entscheidung über dich.</p>

  <p class="meta">Stand: ${new Date().toISOString().slice(0, 10)}</p>
</article>`,
  },

  {
    slug: "agb",
    title: "Allgemeine Geschäftsbedingungen",
    description: "AGB für den Erwerb und die Nutzung von BauLens-Lizenzen.",
    body: `
<article class="prose">
  <h1>Allgemeine Geschäftsbedingungen</h1>
  <p class="lead">für den Erwerb und die Nutzung von Lizenzen an der Software BauLens.</p>

  <h2>1. Geltungsbereich und Vertragspartner</h2>
  <p>Diese Bedingungen gelten für alle Verträge über die Nutzung der Software BauLens zwischen</p>
  ${providerBlock}
  <p>(nachfolgend &bdquo;wir&ldquo; oder &bdquo;Lizenzgeber&ldquo;) und der nutzenden Person
  (nachfolgend &bdquo;du&ldquo; oder &bdquo;Lizenznehmer&ldquo;).</p>
  <div class="callout">
    <strong>Wichtig:</strong> Der Verkauf kostenpflichtiger Lizenzen erfolgt über
    ${esc(billing.provider)} (${esc(billing.providerLegalName)}) als <strong>Merchant of Record</strong>.
    Der Kaufvertrag über eine kostenpflichtige Lizenz kommt daher zwischen dir und Polar zustande.
    Polar ist dein Vertragspartner für den Erwerb, die Abrechnung und die Rechnungsstellung; für
    Reklamationen zum Kaufvorgang, zur Zahlung oder zur Rechnung ist Polar zuständig. Diese AGB regeln
    das Nutzungsverhältnis an der Software zwischen dir und uns.
  </div>
  <p>Abweichende Bedingungen erkennen wir nicht an, es sei denn, wir stimmen ihrer Geltung
  ausdrücklich schriftlich zu.</p>

  <h2>2. Leistungsgegenstand</h2>
  <p>BauLens ist eine Software zur <strong>deterministischen Vorprüfung</strong> von Bauvorhaben gegen
  hinterlegte Regelprofile (unter anderem Musterbauordnung, Landesbauordnungen, OIB-Richtlinien,
  IVHB-Kantonsrecht, BauNVO und kommunale Satzungen).</p>
  <p>Der konkrete Leistungsumfang ergibt sich aus der von dir gewählten Lizenzstufe. Die
  Funktionsbeschreibung im Repository und in der <a href="../../docs.html">Dokumentation</a> ist
  maßgeblich; der Regelkatalog beschreibt ausdrücklich auch die Grenzen und Vereinfachungen der
  einzelnen Prüfmodule.</p>

  <h2>3. Keine Rechtsberatung, keine Genehmigung</h2>
  <div class="callout">
    <strong>BauLens erbringt keine Rechtsberatung</strong> im Sinne des Rechtsdienstleistungsgesetzes
    (RDG). Die Ergebnisse sind technische und planerische Vorprüfungen auf Grundlage der von dir
    eingegebenen Daten. Sie ersetzen weder die Prüfung durch eine bauvorlageberechtigte Person noch
    den Genehmigungsbescheid der zuständigen Bauaufsichtsbehörde. Rechtlich verbindlich ist
    ausschließlich der aktuelle Gesetzestext in der jeweils geltenden Fassung.
  </div>
  <p>Insbesondere wird keine Genehmigungsfähigkeit zugesichert. Werte, die im Regelkatalog mit
  <code>[prüfen]</code> gekennzeichnet sind, beruhen auf vereinfachten Annahmen oder
  Sekundärliteratur und sind vor einer Verwendung im Verfahren eigenständig zu verifizieren.</p>

  <h2>4. Preise, Laufzeit und Zahlung</h2>
  <p>Die aktuellen Preise ergeben sich aus der <a href="../../pricing/">Preisübersicht</a>. Alle
  Preise werden in ${esc(billing.currency)} ausgewiesen und verstehen sich
  ${billing.taxBehavior === "inclusive" ? "einschließlich" : "zuzüglich"} Umsatzsteuer. Die konkrete
  Steuerberechnung nimmt Polar im Checkout anhand deiner Rechnungsadresse vor.</p>
  <ul>
    <li>Kostenpflichtige Lizenzen werden als <strong>Jahresabonnement</strong> verkauft und verlängern
    sich entsprechend den im Checkout genannten Bedingungen.</li>
    <li>Die Zahlung erfolgt über die von Polar angebotenen Zahlungswege an Polar.</li>
    <li>Preise für Folgezeiträume können wir mit angemessener Ankündigung anpassen. Polar informiert
    dich vor einer Verlängerung über den dann gültigen Preis.</li>
  </ul>
  <p>Für die Testlizenz fallen keine Kosten an; sie ist auf die angegebene Anzahl von Prüfungen
  begrenzt.</p>

  <h2>5. Einräumung von Nutzungsrechten</h2>
  <p>Wir räumen dir für die Dauer der Lizenz ein <strong>einfaches, nicht ausschließliches,
  nicht übertragbares Nutzungsrecht</strong> an der Software im Umfang der gebuchten Lizenzstufe ein.</p>
  <ul>
    <li>Bei Lizenzen mit einem Arbeitsplatz darf die Software von einer natürlichen Person genutzt
    werden. Die Anzahl zulässiger Arbeitsplätze ergibt sich aus der gebuchten Lizenz.</li>
    <li>Die Prüfergebnisse, Berichte und Dossiers, die du mit BauLens erzeugst, darfst du ohne
    Einschränkung für deine eigenen Projekte und gegenüber deinen Auftraggebern verwenden.</li>
    <li>Nicht gestattet ist der Weiterverkauf, die Untervermietung, die Weitergabe der Lizenz oder die
    Nutzung als Grundlage eines eigenen, am Markt angebotenen Prüfdienstes.</li>
  </ul>
  <p>Die weitergehenden Bestimmungen enthält die <a href="../lizenz/">Lizenzvereinbarung</a>.</p>

  <h2>6. Pflichten des Lizenznehmers</h2>
  <p>Du bist dafür verantwortlich, dass die von dir eingegebenen Daten zutreffend sind. BauLens
  rechnet ausschließlich mit dem, was du bereitstellst: Sind Angaben unvollständig oder falsch,
  können Ergebnisse unrichtig sein. Die Software weist auf fehlende Angaben durch Warnungen hin,
  ersetzt deren eigenständige Prüfung aber nicht.</p>
  <p>Du wirst die Software nicht dazu verwenden, um entgegen geltendem Recht Dritten die Prüfung
  durch eine berechtigte Person vorzutäuschen.</p>

  <h2>7. Verfügbarkeit</h2>
  <p>Die Prüf-Engine läuft lokal; ihre Verfügbarkeit hängt damit nicht von unseren Servern ab. Für die
  Abrufbarkeit dieser Website und für die Kaufabwicklung gelten die Zusagen der jeweiligen
  Betreiber (${esc(legal.hosting.name)} beziehungsweise Polar). Eine bestimmte Verfügbarkeit von
  Website oder Lizenzprüfung sagen wir nicht zu. Die Lizenzprüfung ist ein zusätzliches Angebot; die
  Prüf-Engine bleibt auch ohne sie nutzbar.</p>

  <h2>8. Aktualisierungen und Änderungen</h2>
  <p>Wir können die Software weiterentwickeln und Fehler beheben. Wir können den Funktionsumfang
  ändern, wenn dies für dich zumutbar ist und der Vertragszweck erhalten bleibt. Eine Pflicht, neue
  Regelprofile, Gesetzesfassungen oder neue Module kostenfrei bereitzustellen, besteht nicht.</p>

  <h2>9. Haftung</h2>
  <p>Wir haften unbeschränkt bei Vorsatz und grober Fahrlässigkeit sowie bei der Verletzung von Leben,
  Körper oder Gesundheit. Bei einfacher Fahrlässigkeit haften wir nur bei Verletzung einer
  wesentlichen Vertragspflicht und begrenzt auf den vertragstypischen, vorhersehbaren Schaden. Im
  Übrigen ist die Haftung ausgeschlossen.</p>
  <p>Ein Schaden, der daraus entsteht, dass du Ergebnisse von BauLens ungeprüft für ein
  Genehmigungsverfahren verwendest, ist kein vertragstypischer Schaden. Die Verantwortung für die
  Verwendung der Ergebnisse im Einzelfall liegt bei der bauvorlageberechtigten Person.</p>
  <p>Die Haftung nach dem Produkthaftungsgesetz sowie für das Fehlen zugesicherter Eigenschaften
  bleibt unberührt.</p>

  <h2>10. Gewährleistung</h2>
  <p>Es gelten die gesetzlichen Gewährleistungsregelungen. Eine Beschaffenheit gilt nur dann als
  zugesichert, wenn sie ausdrücklich als zugesichert bezeichnet wurde. Die im Regelkatalog
  dokumentierten Vereinfachungen und Einschränkungen sind keine Mängel, sondern beschreiben den
  vereinbarten Leistungsumfang.</p>

  <h2>11. Laufzeit und Kündigung</h2>
  <ul>
    <li>Die Testlizenz endet mit dem Verbrauch der enthaltenen Prüfungen.</li>
    <li>Jahreslizenzen laufen für die gebuchte Laufzeit und verlängern sich nach den im Checkout
    genannten Bedingungen. Die Kündigung erfolgt gegenüber Polar über das dortige Kundenportal.</li>
    <li>Das Recht zur außerordentlichen Kündigung aus wichtigem Grund bleibt für beide Seiten
    unberührt. Ein wichtiger Grund liegt für uns insbesondere vor, wenn du die Lizenz erheblich
    missbräuchlich nutzt oder Rechte Dritter verletzt.</li>
  </ul>

  <h2>12. Widerrufsrecht</h2>
  <p>Für Verbraucherinnen und Verbraucher gilt das gesetzliche Widerrufsrecht. Einzelheiten,
  einschließlich der Besonderheiten bei digitalen Inhalten, findest du in der
  <a href="../widerruf/">Widerrufsbelehrung</a>.</p>

  <h2>13. Streitbeilegung</h2>
  <p>Wir sind nicht bereit oder verpflichtet, an Streitbeilegungsverfahren vor einer
  Verbraucherschlichtungsstelle teilzunehmen. Die Plattform der Europäischen Kommission zur
  Online-Streitbeilegung findest du unter
  <a href="https://ec.europa.eu/consumers/odr" target="_blank" rel="noopener noreferrer">ec.europa.eu/consumers/odr</a>.</p>

  <h2>14. Anwendbares Recht</h2>
  <p>Es gilt das Recht der Bundesrepublik Deutschland unter Ausschluss des UN-Kaufrechts. Zwingende
  Verbraucherschutzvorschriften des Staates, in dem du deinen gewöhnlichen Aufenthalt hast, bleiben
  unberührt.</p>

  <h2>15. Schlussbestimmungen</h2>
  <p>Sollte eine Bestimmung dieser Bedingungen unwirksam sein, bleibt die Wirksamkeit der übrigen
  Bestimmungen unberührt. An die Stelle der unwirksamen Bestimmung tritt die gesetzliche Regelung.</p>
  <p>Wir können diese Bedingungen mit Wirkung für die Zukunft anpassen, soweit dies erforderlich ist
  und du hierdurch nicht unangemessen benachteiligt wirst. Über Änderungen informieren wir rechtzeitig.</p>

  <p class="meta">Stand: ${new Date().toISOString().slice(0, 10)}</p>
</article>`,
  },

  {
    slug: "lizenz",
    title: "Lizenzvereinbarung",
    description: "Endnutzer-Lizenzvereinbarung für die Software BauLens.",
    body: `
<article class="prose">
  <h1>Lizenzvereinbarung</h1>
  <p class="lead">Endnutzer-Lizenzvereinbarung (EULA) für die Software ${esc(product.name)}, Version ${esc(product.version)}.</p>

  <h2>1. Vertragsgegenstand</h2>
  <p>Gegenstand dieser Vereinbarung ist die Überlassung der Software ${esc(product.name)} zur Nutzung
  durch den Lizenznehmer. Lizenzgeber ist ${esc(legal.providerName)}, ${esc(legal.providerTradingAs)},
  ${esc(legal.providerStreet)}, ${esc(legal.providerPostalCode)} ${esc(legal.providerCity)}.</p>
  <p>Ergänzend gelten die <a href="../agb/">Allgemeinen Geschäftsbedingungen</a>.</p>

  <h2>2. Umfang der Lizenz</h2>
  <p>Der Lizenzgeber räumt dem Lizenznehmer ein einfaches, nicht ausschließliches, nicht übertragbares
  und auf die Laufzeit der gebuchten Lizenzstufe befristetes Recht ein, die Software zu nutzen.</p>
  <div class="data-block">
    <p><strong>Testlizenz:</strong> Nutzung zur Bewertung, begrenzt auf die angegebene Anzahl von
    Projektprüfungen.</p>
    <p><strong>Pro Architekt:</strong> Nutzung durch einen Arbeitsplatz, bis zu 180 Projektprüfungen
    pro Lizenzjahr.</p>
    <p><strong>Büro / Kanzlei:</strong> Nutzung durch einen Arbeitsplatz je erworbener Lizenz; weitere
    Arbeitsplätze sind im Checkout zubuchbar.</p>
    <p><strong>Enterprise / Serien:</strong> Nutzung nach gesonderter Vereinbarung, insbesondere zur
    Katalogprüfung von Haustypenlisten und für den API-Zugang.</p>
  </div>

  <h2>3. Erlaubte Nutzung</h2>
  <ul>
    <li>Die Software darf im vereinbarten Umfang beruflich und gewerblich eingesetzt werden.</li>
    <li>Erzeugte Prüfbefunde, Berichte, Dossiers und Exporte dürfen uneingeschränkt für eigene
    Projekte und gegenüber Auftraggebern verwendet und weitergegeben werden.</li>
    <li>Die Software darf für eigene Vorhaben angepasst werden, soweit dies nicht zu einer Weitergabe
    an Dritte führt.</li>
  </ul>

  <h2>4. Nicht erlaubte Nutzung</h2>
  <ul>
    <li>Weiterverkauf, Vermietung, Untervermietung, Verleih oder sonstige Überlassung an Dritte.</li>
    <li>Bereitstellung der Software oder ihrer Prüfergebnisse als eigener, am Markt angebotener
    Prüfdienst, auch nicht in Teilen.</li>
    <li>Umgehung technischer Schutz- oder Lizenzmaßnahmen.</li>
    <li>Entfernung oder Veränderung von Urheberrechtsvermerken, Prüfsiegeln oder
    Herstellerangaben.</li>
    <li>Nutzung, um gegenüber Dritten eine behördliche Prüfung oder Genehmigung vorzutäuschen.</li>
  </ul>

  <h2>5. Herkunft und Open-Source-Bestandteile</h2>
  <p>Der Quellcode ist im Repository
  <a href="${esc(product.repoUrl)}" target="_blank" rel="noopener noreferrer">${esc(product.repoUrl)}</a>
  veröffentlicht und unter der MIT-Lizenz nutzbar. Diese Lizenzvereinbarung regelt die Nutzung der
  aufbereiteten Prüfprofile, der amtlichen Zitatdaten und der Dossier- und Exportfunktionen sowie die
  kommerzielle Lizenzeinräumung. Soweit Bestandteile Dritter enthalten sind, gelten deren jeweilige
  Lizenzbedingungen; diese gehen diesen Bestimmungen im Konfliktfall vor.</p>

  <h2>6. Amtliche Regelwerke und Zitate</h2>
  <p>Amtliche Gesetzestexte, Verordnungen und Satzungen sind gemeinfrei. Die Zusammenstellung,
  Kommentierung, Zuordnung zu Prüfmodulen und Auswertung innerhalb der Software ist eine eigene
  Leistung des Lizenzgebers. Zitate dienen ausschließlich der Information; verbindlich ist
  ausschließlich die amtlich verkündete Fassung.</p>

  <h2>7. Keine Rechtsberatung</h2>
  <div class="callout">
    Die Software stellt eine technische Vorprüfung bereit. Sie erbringt
    <strong>keine Rechtsberatung</strong> und ersetzt weder die Prüfung durch eine
    bauvorlageberechtigte Person noch den Genehmigungsbescheid der zuständigen Bauaufsichtsbehörde.
  </div>

  <h2>8. Gewährleistung und Haftung</h2>
  <p>Die Gewährleistung und die Haftung richten sich nach den Abschnitten 9 und 10 der
  <a href="../agb/">Allgemeinen Geschäftsbedingungen</a>. Der Regelkatalog dokumentiert die
  Vereinfachungen der einzelnen Prüfmodule ausdrücklich; diese dokumentierten Grenzen sind Teil der
  vereinbarten Beschaffenheit.</p>

  <h2>9. Laufzeit und Beendigung</h2>
  <p>Die Lizenz endet mit Ablauf der gebuchten Laufzeit. Nach Beendigung sind die Nutzung
  einzustellen und bereitgestellte Lizenzschlüssel nicht mehr zu verwenden. Bereits erzeugte
  Prüfergebnisse und Dossiers darf der Lizenznehmer weiterhin verwenden.</p>

  <h2>10. Schlussbestimmungen</h2>
  <p>Es gilt das Recht der Bundesrepublik Deutschland. Zwingende Verbraucherschutzvorschriften des
  Aufenthaltsstaats bleiben unberührt. Änderungen dieser Vereinbarung bedürfen der Textform;
  Änderungen mit Wirkung für die Zukunft sind unter den Voraussetzungen des Abschnitts 15 der
  Allgemeinen Geschäftsbedingungen möglich.</p>

  <p class="meta">Stand: ${new Date().toISOString().slice(0, 10)} &bull; Fassung ${esc(product.version)}</p>
</article>`,
  },

  {
    slug: "widerruf",
    title: "Widerrufsbelehrung",
    description: "Widerrufsrecht und Widerrufsbelehrung für Verbraucher bei BauLens.",
    body: `
<article class="prose">
  <h1>Widerrufsbelehrung</h1>
  <p class="lead">Für Verbraucherinnen und Verbraucher.</p>

  <h2>Widerrufsrecht</h2>
  <p>Du hast das Recht, binnen vierzehn Tagen ohne Angabe von Gründen diesen Vertrag zu widerrufen.
  Die Widerrufsfrist beträgt vierzehn Tage ab dem Tag des Vertragsabschlusses.</p>
  <p>Um dein Widerrufsrecht auszuüben, musst du uns mittels einer eindeutigen Erklärung (zum Beispiel
  ein mit der Post versandter Brief oder eine E-Mail) über deinen Entschluss informieren, diesen
  Vertrag zu widerrufen:</p>
  ${providerBlock}
  <p>E-Mail: <a href="mailto:${esc(legal.email)}">${esc(legal.email)}</a></p>
  <p>Zur Wahrung der Widerrufsfrist reicht es aus, dass du die Mitteilung über die Ausübung des
  Widerrufsrechts vor Ablauf der Widerrufsfrist absendest.</p>
  <p>Da der Kauf kostenpflichtiger Lizenzen über ${esc(billing.provider)} als Merchant of Record
  abgewickelt wird, ist Polar dein Vertragspartner für den Erwerb. Du kannst den Widerruf deshalb
  sowohl uns als auch Polar gegenüber erklären; wir leiten Erklärungen, die uns erreichen, an Polar
  weiter.</p>

  <h2>Folgen des Widerrufs</h2>
  <p>Wenn du diesen Vertrag widerrufst, haben wir dir alle Zahlungen, die wir von dir erhalten haben,
  unverzüglich und spätestens binnen vierzehn Tagen ab dem Tag zurückzuzahlen, an dem die Mitteilung
  über deinen Widerruf bei uns eingegangen ist. Für die Rückzahlung wird dasselbe Zahlungsmittel
  verwendet, das du bei der ursprünglichen Transaktion eingesetzt hast.</p>

  <h2>Vorzeitiges Erlöschen bei digitalen Inhalten</h2>
  <div class="callout">
    <strong>Wichtig:</strong> Bei einem Vertrag über die Bereitstellung digitaler Inhalte erlischt das
    Widerrufsrecht, wenn wir mit der Ausführung des Vertrags begonnen haben, nachdem du
    <strong>ausdrücklich zugestimmt</strong> hast, dass wir vor Ablauf der Widerrufsfrist mit der
    Ausführung beginnen, und du deine Kenntnis davon bestätigt hast, dass du durch deine Zustimmung
    dein Widerrufsrecht verlierst (§ 356 Abs. 5 BGB).
  </div>
  <p>Diese Zustimmung holst du im Checkout von Polar ein, weil die Lizenz dort unmittelbar nach dem
  Kauf bereitgestellt wird. Solange du diese Zustimmung nicht erteilst, wird die Lizenz nicht
  vorzeitig freigeschaltet und dein Widerrufsrecht bleibt bestehen.</p>

  <h2>Freiwillige Geld-zurück-Zusage</h2>
  <p>Unabhängig von der gesetzlichen Regelung gewähren wir für kostenpflichtige Lizenzen eine
  <strong>${legal.withdrawalGuaranteeDays}-tägige Geld-zurück-Zusage</strong>: Erklärt du innerhalb
  von ${legal.withdrawalGuaranteeDays} Tagen nach dem Kauf, dass BauLens für dich nicht passt,
  erstatten wir den Kaufpreis vollständig. Diese Zusage ist ein freiwilliges Entgegenkommen und
  erweitert deine gesetzlichen Rechte, ohne sie einzuschränken.</p>

  <h2>Muster-Widerrufsformular</h2>
  <p>Wenn du den Vertrag widerrufen willst, kannst du dieses Formular ausfüllen und an uns
  zurücksenden. Es ist nicht vorgeschrieben.</p>
  <div class="data-block">
    <p>An ${esc(legal.providerName)}, ${esc(legal.providerTradingAs)}, ${esc(legal.providerStreet)}, ${esc(legal.providerPostalCode)} ${esc(legal.providerCity)}, E-Mail: ${esc(legal.email)}</p>
    <p>Hiermit widerrufe ich den von mir abgeschlossenen Vertrag über den Kauf der folgenden Lizenz:</p>
    <p>&mdash; Bestellt am / erhalten am:</p>
    <p>&mdash; Name des Verbrauchers:</p>
    <p>&mdash; Anschrift des Verbrauchers:</p>
    <p>&mdash; E-Mail-Adresse der Bestellung:</p>
    <p>&mdash; Datum:</p>
    <p>&mdash; Unterschrift (nur bei Mitteilung auf Papier):</p>
  </div>

  <p class="meta">Stand: ${new Date().toISOString().slice(0, 10)}</p>
</article>`,
  },
];

/* --------------------------------------------------------------------------
 * Pricing components
 * ------------------------------------------------------------------------ */

function priceLine(tier) {
  if (tier.price === 0) {
    return `<p class="plan-price"><span class="plan-amount">${esc(billing.currencySymbol)}0</span> <small>&middot; Testlizenz</small></p>`;
  }
  const per = tier.perSeat
    ? `<small>/ ${esc(tier.priceUnit)} &middot; Arbeitsplatz</small>`
    : `<small>&middot; ${esc(tier.priceUnit)}</small>`;
  const annual = tier.annualPrice
    ? `<small class="plan-annual">oder ${money(tier.annualPrice)} / Jahr</small>`
    : "";
  return `<p class="plan-price"><span class="plan-amount">${money(tier.price)}</span> ${per}${annual}</p>`;
}

/**
 * Renders one plan card. `depth` is how many directories the consuming page is
 * below the repository root, so a single config value (a repo-root-relative
 * path) resolves correctly on the landing page and on /pricing/ alike.
 */
function planCard(tier, featuredLabel, depth = 0) {
  const featured = tier.id === "pro";
  const up = "../".repeat(depth);
  /* Every tier names its audience in the header, not only the featured one: with
     three plans of very different shape, "Projekt-Pass" or "Für Planungsbüros"
     is what tells the reader which one they are looking at. */
  const label = tier.flag ?? (featured ? featuredLabel : null);
  const flag = label ? `<span class="plan-flag">${esc(label)}</span>` : "";
  const isCheckout = Boolean(tier.checkoutUrl);
  const attrs = isCheckout
    ? `href="${esc(tier.checkoutUrl)}" target="_blank" rel="noopener noreferrer"`
    : `href="${up}${esc(tier.ctaHref)}"`;
  const btnClass = tier.ctaStyle === "primary" ? "btn btn--primary" : "btn btn--ghost";

  return `        <article class="plan${featured ? " plan--featured" : ""}">
          <div class="plan-tier">
            <h3>${esc(tier.name)}</h3>
            ${flag}
          </div>
          ${priceLine(tier)}
          <p class="plan-for">${esc(tier.audience)}</p>
          <ul>
${tier.features.map((f) => `            <li><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg><span>${f}</span></li>`).join("\n")}
          </ul>
          <a class="${btnClass} btn--block" ${attrs}>${esc(tier.cta)}${isCheckout ? '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>' : ""}</a>
          <p class="plan-note">${esc(tier.note)}</p>
        </article>`;
}

/* Landing page: paths are relative to the repository root. */
const pricingBlock = `      <div class="plans">
${tiers.map((t) => planCard(t, t.flag, 0)).join("\n")}
      </div>

      <div class="enterprise">
        <p>
          <strong>${esc(enterprise.name)}:</strong> ${esc(enterprise.audience)}
          <span class="num">${money(enterprise.price)} / ${esc(enterprise.priceUnit)}</span>
        </p>
        <a class="btn btn--ghost btn--sm" href="${esc(enterprise.ctaHref)}">${esc(enterprise.cta)}</a>
      </div>

      <p class="billing-note">${esc(billing.pricesNote)}</p>
`;

/* --------------------------------------------------------------------------
 * Pricing page
 * ------------------------------------------------------------------------ */

const pricingCss = `
  .plan-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 1rem; margin-top: 2rem; }
  @media (max-width: 980px) { .plan-grid { grid-template-columns: minmax(0, 1fr); } }
  .plan {
    display: flex; flex-direction: column; min-width: 0;
    background: var(--ink-800); border: 1px solid var(--line);
    border-radius: var(--r-lg); padding: 1.6rem;
  }
  .plan--featured { border-color: var(--accent-edge); background: linear-gradient(180deg, var(--accent-wash), var(--ink-800) 42%); }
  .plan-tier { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; margin-bottom: 0.9rem; }
  .plan-tier h3 { font-size: 1rem; font-weight: 600; margin: 0; }
  .plan-flag {
    font-family: var(--font-mono); font-size: 0.62rem; letter-spacing: 0.1em; text-transform: uppercase;
    color: var(--accent-hi); border: 1px solid var(--accent-edge); border-radius: var(--r-sm); padding: 0.15rem 0.45rem;
    white-space: nowrap;
  }
  .plan-price {
    display: flex; align-items: baseline; gap: 0.45rem; margin: 0 0 0.35rem;
    font-family: var(--font-mono); font-variant-numeric: tabular-nums;
  }
  .plan-amount { font-size: 1.85rem; font-weight: 500; letter-spacing: -0.02em; color: var(--fg); }
  .plan-price small { font-size: 0.78rem; color: var(--fg-dim); }
  .plan-for { font-size: 0.875rem; color: var(--fg-muted); margin: 0 0 1.25rem; min-height: 3.4em; }
  .plan ul { list-style: none; margin: 0 0 1.4rem; padding: 0; display: flex; flex-direction: column; gap: 0.55rem; flex: 1; }
  .plan li { display: grid; grid-template-columns: 1rem minmax(0,1fr); gap: 0.5rem; font-size: 0.85rem; color: var(--fg-muted); }
  .plan li svg { color: var(--accent); margin-top: 0.3rem; }
  .plan li strong { color: var(--fg); font-weight: 550; }
  .plan-note { font-size: 0.75rem; color: var(--fg-faint); margin: 0.75rem 0 0; text-align: center; }
  .plan-annual { font-size: 0.72rem; color: var(--fg-faint); display: block; margin-top: 0.15rem; }
  .plan-price { flex-wrap: wrap; }
  .btn {
    display: inline-flex; align-items: center; justify-content: center; gap: 0.45rem;
    font-size: 0.9rem; font-weight: 550; line-height: 1; padding: 0.72rem 1.2rem;
    border: 1px solid transparent; border-radius: var(--r-md); text-decoration: none;
    white-space: nowrap; transition: background-color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);
  }
  .btn svg { flex: none; opacity: 0.75; }
  .btn--primary { background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .btn--primary:hover { background: var(--accent-hi); }
  .btn--primary svg { opacity: 1; }
  .btn--ghost { background: transparent; border-color: var(--line-strong); color: var(--fg); }
  .btn--ghost:hover { background: var(--ink-600); border-color: var(--ink-line); }
  .btn--block { width: 100%; }
  .btn--sm { padding: 0.5rem 0.85rem; font-size: 0.8rem; }
  .enterprise {
    display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 1.25rem;
    margin-top: 1rem; padding: 1.25rem 1.6rem; border: 1px solid var(--line);
    border-radius: var(--r-lg); background: var(--ink-800);
  }
  .enterprise p { margin: 0; font-size: 0.9rem; color: var(--fg-muted); max-width: 62ch; }
  .enterprise strong { color: var(--fg); }
  .enterprise .num { font-family: var(--font-mono); color: var(--accent-hi); white-space: nowrap; }
  .billing-note { font-size: 0.8rem; color: var(--fg-dim); max-width: 74ch; margin-top: 1.5rem; }
  .price-table { width: 100%; border-collapse: collapse; margin-top: 1.5rem; font-size: 0.875rem; }
  .price-table th, .price-table td { text-align: left; padding: 0.7rem 0.9rem; border-bottom: 1px solid var(--line); }
  .price-table thead th {
    font-family: var(--font-mono); font-size: 0.68rem; font-weight: 500; letter-spacing: 0.09em;
    text-transform: uppercase; color: var(--fg-dim); background: var(--ink-700);
  }
  .price-table td { color: var(--fg-muted); }
  .price-table td:first-child { color: var(--fg); }
`;

const pricingPage = page({
  title: "Preise",
  description: `Lizenzen für BauLens: Starter ${money(tiers[0].price)} einmalig, Pro Architekt ${money(tiers[1].price)} pro Monat (${money(tiers[1].annualPrice)} / Jahr), Büro/Kanzlei ${money(tiers[2].price)} pro Monat für drei Arbeitsplätze. Kauf über Polar.`,
  depth: 1,
  extraCss: pricingCss,
  body: `<article class="prose" style="max-width:100%;">
  <h1>Preise</h1>
  <p class="lead" style="max-width:62ch;">Alle Lizenzen arbeiten mit demselben Regelkern. Unterschiedlich sind Umfang,
  Ausgabeformate und Anzahl der Arbeitsplätze. Der Kauf läuft über Polar als Merchant of Record.</p>

  <div class="plan-grid">
${tiers.map((t) => planCard(t, t.flag, 1)).join("\n")}
  </div>

  <div class="enterprise">
    <p><strong>${esc(enterprise.name)}:</strong> ${esc(enterprise.audience)}
    <span class="num">${money(enterprise.price)} / ${esc(enterprise.priceUnit)}</span></p>
    <a class="btn btn--ghost btn--sm" href="${esc(enterprise.ctaHref)}">${esc(enterprise.cta)}</a>
  </div>

  <p class="billing-note">${esc(billing.pricesNote)}</p>

  <h2>Leistungsumfang im Vergleich</h2>
  <div style="overflow-x:auto;">
    <table class="price-table">
      <caption class="meta" style="caption-side:top;text-align:left;padding:0 0 0.75rem;">Starter, Pro Architekt und Büro/Kanzlei im direkten Vergleich.</caption>
      <thead>
        <tr>
          <th scope="col">Leistung</th>
          <th scope="col">Starter</th>
          <th scope="col">Pro Architekt</th>
          <th scope="col">Büro / Kanzlei</th>
        </tr>
      </thead>
      <tbody>
        <tr><th scope="row">Preis</th><td>${money(tiers[0].price)} ${esc(tiers[0].priceUnit)}</td><td>${money(tiers[1].price)} / Monat<br><span class="meta">${money(tiers[1].annualPrice)} / Jahr</span></td><td>${money(tiers[2].price)} / Monat<br><span class="meta">${money(tiers[2].annualPrice)} / Jahr</span></td></tr>
        <tr><th scope="row">Arbeitsplätze</th><td>1</td><td>1</td><td>3 inklusive, weitere +49 € / Monat</td></tr>
        <tr><th scope="row">Umfang</th><td>1 Vorhaben, 30 Tage</td><td>unbegrenzt</td><td>unbegrenzt</td></tr>
        <tr><th scope="row">Re-Runs bei Entwurfsänderung</th><td>bis zu 10</td><td>unbegrenzt</td><td>unbegrenzt</td></tr>
        <tr><th scope="row">DACH-Regelprofile</th><td>alle 34</td><td>alle 34</td><td>alle 34</td></tr>
        <tr><th scope="row">Multi-Projekt-Workspace</th><td>ja</td><td>ja</td><td>ja</td></tr>
        <tr><th scope="row">CAD-/BIM-Ingestion</th><td>DXF, IFC, XPlanung</td><td>DXF, IFC, XPlanung</td><td>DXF, IFC, XPlanung</td></tr>
        <tr><th scope="row">DIN-A4-Dossier mit Prüfsiegel</th><td>ja</td><td>ja</td><td>ja</td></tr>
        <tr><th scope="row">XBau-Antrag, BCF-Export</th><td>ja</td><td>ja</td><td>ja</td></tr>
        <tr><th scope="row">Compliance-Repair-Engine</th><td>nein</td><td>ja</td><td>ja</td></tr>
        <tr><th scope="row">Firmenlogo im Dossier</th><td>nein</td><td>nein</td><td>ja</td></tr>
        <tr><th scope="row">DACH-Quervergleich</th><td>nein</td><td>nein</td><td>ja</td></tr>
        <tr><th scope="row">Support</th><td>E-Mail</td><td>E-Mail</td><td>vorrangig</td></tr>
      </tbody>
    </table>
  </div>

  <h2>Abwicklung und Rechnungsstellung</h2>
  <p>Der Kauf kostenpflichtiger Lizenzen wird von <strong>${esc(billing.provider)}</strong>
  (${esc(billing.providerLegalName)}) als Merchant of Record abgewickelt. Das bedeutet: Polar ist dein
  Vertragspartner für den Erwerb, wickelt die Zahlung ab, stellt die Rechnung aus und führt die
  Umsatzsteuer ab. Kartendaten erreichen uns zu keinem Zeitpunkt.</p>
  <p>Nach dem Kauf sendet Polar den Lizenzschlüssel per E-Mail. Du kannst ihn in der
  <a href="../downloads/">Desktop-App</a> hinterlegen; die Prüfung funktioniert auch ohne Aktivierung.</p>

  <h2>Fragen vor dem Kauf</h2>
  <p>Unsicher, welche Lizenz passt? Die <a href="../docs.html">Dokumentation</a> beschreibt jedes der
  15 Prüfmodule mit Formel, Fundstelle und den dokumentierten Vereinfachungen. Für eine
  Enterprise-Anfrage genügt eine E-Mail an <a href="mailto:${esc(legal.email)}">${esc(legal.email)}</a>.</p>

  <p class="meta">Rechtliches: <a href="../legal/agb/">AGB</a> &middot;
  <a href="../legal/widerruf/">Widerrufsbelehrung</a> &middot;
  <a href="../legal/lizenz/">Lizenzvereinbarung</a> &middot;
  <a href="../legal/impressum/">Impressum</a> &middot;
  <a href="../legal/datenschutz/">Datenschutz</a></p>
</article>`,
});

/* --------------------------------------------------------------------------
 * Checkout success page
 * ------------------------------------------------------------------------ */

const successPage = page({
  title: "Vielen Dank für deinen Kauf",
  description: "Bestätigung des Lizenzkaufs bei BauLens und Hinweise zur Aktivierung in der Desktop-App.",
  depth: 2,
  extraCss: `
  .steps { list-style: none; margin: 1.5rem 0; padding: 0; display: flex; flex-direction: column; gap: 1rem; }
  .steps li { display: grid; grid-template-columns: 1.75rem minmax(0,1fr); gap: 0.85rem; }
  .step-num {
    font-family: var(--font-mono); font-size: 0.75rem; color: var(--accent);
    border: 1px solid var(--accent-edge); border-radius: var(--r-sm);
    display: grid; place-items: center; height: 1.5rem;
  }
  .steps strong { color: var(--fg); display: block; margin-bottom: 0.15rem; }
  .steps p { margin: 0; font-size: 0.9rem; }
`,
  body: `<article class="prose">
  <h1>Vielen Dank für deinen Kauf</h1>
  <p class="lead">Deine Lizenz ist bestellt. Die Rechnung und den Lizenzschlüssel erhältst du per
  E-Mail von Polar &mdash; dort, wo du auch die Zahlung abgeschlossen hast.</p>

  <ol class="steps">
    <li>
      <span class="step-num">1</span>
      <div>
        <strong>E-Mail von Polar prüfen</strong>
        <p>Polar sendet dir die Rechnung und den Lizenzschlüssel. Schau zur Sicherheit im Spam-Ordner
        nach, falls die Nachricht nicht innerhalb weniger Minuten ankommt.</p>
      </div>
    </li>
    <li>
      <span class="step-num">2</span>
      <div>
        <strong>Lizenzschlüssel in der Desktop-App hinterlegen</strong>
        <p>Lade die <a href="../../downloads/">Desktop-App</a> und trage den Schlüssel im Bereich
        <em>Lizenz</em> ein. Die Aktivierung ist freiwillig &mdash; die Prüf-Engine rechnet auch ohne
        Schlüssel, ausschließlich lokal.</p>
      </div>
    </li>
    <li>
      <span class="step-num">3</span>
      <div>
        <strong>Erstes Projekt prüfen</strong>
        <p>Parameter erfassen oder eine Datei einlesen, prüfen lassen und das DIN-A4-Dossier als PDF
        exportieren.</p>
      </div>
    </li>
  </ol>

  <div class="callout">
    <strong>Keine E-Mail erhalten?</strong> Dann prüfe zuerst den Spam-Ordner und die bei Polar
    hinterlegte Adresse. Bleibt es dabei, schreib uns an
    <a href="mailto:${esc(legal.email)}?subject=BauLens%20Lizenz">${esc(legal.email)}</a> mit dem
    ungefähren Kaufzeitpunkt &mdash; wir finden die Bestellung.
  </div>

  <h2>Rechnung und Vertragspartner</h2>
  <p>Dein Vertragspartner für den Erwerb ist ${esc(billing.provider)} (${esc(billing.providerLegalName)}),
  der als Merchant of Record die Rechnung ausstellt und die Umsatzsteuer abführt. Rechnungsdaten
  kannst du im Kundenportal von Polar einsehen und ändern.</p>

  <h2>Widerruf und Geld-zurück-Zusage</h2>
  <p>Du hast ein 14-tägiges Widerrufsrecht; zusätzlich gewähren wir eine freiwillige 14-tägige
  Geld-zurück-Zusage. Einzelheiten stehen in der
  <a href="../../legal/widerruf/">Widerrufsbelehrung</a>.</p>

  <h2>Nächste Schritte</h2>
  <p><a href="../../downloads/">Desktop-App herunterladen</a> &middot;
  <a href="../../docs.html">Dokumentation lesen</a> &middot;
  <a href="../../pricing/">Preisübersicht</a></p>
</article>`,
});

/* --------------------------------------------------------------------------
 * Download hub
 *
 * The website no longer ships the studio. This page is the funnel instead: it
 * detects the operating system, links the signed release assets directly (no
 * GitHub navigation), and states the checksum and signature facts so the
 * "download the wrong thing" failure cannot repeat.
 * ------------------------------------------------------------------------ */

const PLATFORM_LABEL = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
  any: "Alle Systeme",
};

const releaseBaseUrl = downloads.releaseBaseUrl.replace("{version}", product.version);
const artifactUrl = (artifact) => `${releaseBaseUrl}/${artifact.file.replace("{version}", product.version)}`;
const artifactName = (artifact) => artifact.file.replace("{version}", product.version);

/* What the client-side OS detection reads. Kept as JSON in the page rather than
   duplicated in the inline script, so the list has one source: site.config.json. */
const downloadIndex = downloads.artifacts.map((artifact) => ({
  id: artifact.id,
  platform: artifact.platform,
  kind: artifact.kind,
  label: artifact.label,
  file: artifactName(artifact),
  url: artifactUrl(artifact),
  primary: Boolean(artifact.primary),
}));

const downloadRows = downloads.artifacts
  .map(
    (artifact) => `        <tr>
          <th scope="row"><span class="os-tag" data-platform="${esc(artifact.platform)}">${esc(PLATFORM_LABEL[artifact.platform] ?? artifact.platform)}</span> ${esc(artifact.label)}</th>
          <td><code>${esc(artifactName(artifact))}</code></td>
          <td>${esc(artifact.detail)}</td>
          <td><a href="${esc(artifactUrl(artifact))}">Direkt laden</a></td>
        </tr>`
  )
  .join("\n");

const downloadsCss = `
  .dl-hero {
    display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr); gap: 1.5rem; align-items: center;
    margin: 1.75rem 0 0.5rem; padding: 1.5rem 1.6rem;
    background: linear-gradient(180deg, var(--accent-wash), var(--ink-800) 60%);
    border: 1px solid var(--accent-edge); border-radius: var(--r-lg);
  }
  @media (max-width: 820px) { .dl-hero { grid-template-columns: minmax(0, 1fr); } }
  .dl-eyebrow {
    font-family: var(--font-mono); font-size: 0.68rem; letter-spacing: 0.12em;
    text-transform: uppercase; color: var(--fg-dim); margin: 0 0 0.35rem;
  }
  .dl-os { font-size: 1.35rem; font-weight: 600; letter-spacing: -0.02em; color: var(--fg); margin: 0 0 0.35rem; }
  .dl-os-note { font-size: 0.875rem; color: var(--fg-muted); margin: 0; max-width: 52ch; }
  .dl-hero-actions { display: flex; flex-direction: column; gap: 0.6rem; }
  .dl-hero-actions .btn { width: 100%; }
  .dl-alt { font-size: 0.8rem; color: var(--fg-dim); margin: 0; text-align: center; }
  .dl-alt a { color: var(--accent-hi); }
  .os-tag {
    display: inline-block; margin-right: 0.5rem; padding: 0.1rem 0.4rem;
    font-family: var(--font-mono); font-size: 0.62rem; letter-spacing: 0.08em; text-transform: uppercase;
    color: var(--accent-hi); border: 1px solid var(--accent-edge); border-radius: var(--r-sm); white-space: nowrap;
  }
  .dl-table td, .dl-table th { vertical-align: top; }
  .dl-table code { font-size: 0.78rem; white-space: nowrap; }
  .trust-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: 1rem; margin: 1.5rem 0; }
  .trust-card { background: var(--ink-800); border: 1px solid var(--line); border-radius: var(--r-lg); padding: 1.15rem 1.3rem; }
  .trust-card h3 { margin: 0 0 0.5rem; font-size: 0.9375rem; }
  .trust-card p { margin: 0; font-size: 0.875rem; color: var(--fg-muted); }
  .trust-card code { font-size: 0.78rem; }
  .downloads-actions { display: flex; flex-wrap: wrap; gap: 0.75rem; margin-top: 1.5rem; }
`;

const downloadsPage = page({
  title: "Downloads",
  description: `BauLens für Windows, macOS und Linux herunterladen: Installer, AppImage und Debian-Paket, CLI-Binaries sowie SHA-256-Prüfsummen. Version ${product.version}, 100 Prozent lokal.`,
  depth: 1,
  extraCss: downloadsCss,
  body: `<article class="prose" style="max-width:100%;">
  <h1>BauLens herunterladen</h1>
  <p class="lead" style="max-width:66ch;">${esc(downloads.intro)} Version ${esc(product.version)}.</p>

  <div class="dl-hero">
    <div>
      <p class="dl-eyebrow">Erkanntes System</p>
      <p class="dl-os" id="dlOsLabel">wird erkannt …</p>
      <p class="dl-os-note" id="dlOsNote">Die passende Datei wird ausgewählt, sobald der Browser das System meldet. Die Liste unten gilt immer.</p>
    </div>
    <div class="dl-hero-actions">
      <a class="btn btn--primary" id="dlOsPrimary" href="${esc(downloads.releasesUrl)}">Zur Veröffentlichungsübersicht</a>
      <p class="dl-alt">Alle Downloads offen auf <a href="${esc(downloads.releasesUrl)}" target="_blank" rel="noopener noreferrer">lens.kellersystems.dev</a> &mdash; kein Konto nötig.</p>
    </div>
  </div>

  <h2>Alle Pakete dieser Version</h2>
  <p>Die Verweise zeigen direkt auf die signierten Dateien der Veröffentlichung v${esc(product.version)}.
  Die Prüfsummen liegen als <code>${esc(downloads.checksumFile)}</code> daneben.</p>
  <div style="overflow-x:auto;">
    <table class="price-table dl-table">
      <caption class="meta" style="caption-side:top;text-align:left;padding:0 0 0.75rem;">Installation und Kommandozeile für Windows, macOS und Linux.</caption>
      <thead>
        <tr>
          <th scope="col">Paket</th>
          <th scope="col">Datei</th>
          <th scope="col">Hinweis</th>
          <th scope="col">Link</th>
        </tr>
      </thead>
      <tbody>
${downloadRows}
      </tbody>
    </table>
  </div>

  <h2>Vertrauen und Prüfbarkeit</h2>
  <p>${esc(downloads.privateNote)}</p>
  <div class="trust-grid">
    <div class="trust-card">
      <h3>Prüfsummen</h3>
      <p>${esc(downloads.checksumNote)}</p>
    </div>
    <div class="trust-card">
      <h3>Signatur</h3>
      <p>${esc(downloads.signatureNote)}</p>
    </div>
    <div class="trust-card">
      <h3>Kein Konto, keine Telemetrie</h3>
      <p>Die Anwendung startet ohne Anmeldung, ruft beim Start keine Ressourcen aus dem Netz ab und
      sendet keine Nutzungsdaten. Die Lizenzaktivierung ist optional; die Prüf-Engine rechnet auch ohne sie.</p>
    </div>
    <div class="trust-card">
      <h3>Systemvoraussetzungen</h3>
      <p>Windows 10/11 (64-Bit), macOS 11 oder neuer (Apple Silicon und Intel), Linux mit WebKitGTK 4.1.
      Die CLI-Binaries brauchen nichts weiter; die Engine als Skript läuft mit Node.js ab 18.</p>
    </div>
  </div>

  <h2>14 Tage testen</h2>
  <p>Nach dem ersten Start läuft der volle Funktionsumfang der Desktop-Workstation
  <strong>${billing.trialDays} Tage</strong> lang ohne Schlüssel: Multi-Projekt-Workspace, interaktives
  3D/2D-Modell, alle 34 DACH-Profile, DIN-A4-Dossier und XBau 2.3. Danach brauchst du eine Lizenz aus der
  <a href="../pricing/">Preisübersicht</a>. Ein Lizenzschlüssel wird lokal geprüft; Plandaten bleiben auf
  deinem Gerät.</p>

  <h2>Kommandozeile und CI/CD</h2>
  <p>Dieselbe geprüfte Engine gibt es als eigenständige Binärdatei für Server und Pipelines:
  <code>${esc(artifactName(downloads.artifacts.find((a) => a.id === "cli-windows")))}</code>,
  <code>${esc(artifactName(downloads.artifacts.find((a) => a.id === "cli-linux")))}</code> und
  <code>${esc(artifactName(downloads.artifacts.find((a) => a.id === "cli-macos")))}</code>.</p>
  <div class="data-block">
    <p><code>baulens check lageplan.json --land BY --report bericht.md</code></p>
    <p><code>baulens --lands</code> &middot; <code>baulens check entwurf.ifc --repair</code> &middot; <code>baulens --help</code></p>
  </div>

  <h2>Nach dem Download</h2>
  <ol>
    <li>Installer ausführen beziehungsweise das AppImage ausführbar machen und starten.</li>
    <li>Projektordner öffnen &mdash; BauLens indiziert IFC-, DXF-, GML- und BCF-Dateien rekursiv und legt
    jedes Unterverzeichnis als eigenes Vorhaben an.</li>
    <li>Vorprüfung starten und das DIN-A4-Dossier als PDF sichern.</li>
  </ol>

  <div class="callout">
    <strong>Einordnung:</strong> BauLens erbringt eine technische Vorprüfung und keine Rechtsberatung.
    Der Befund ersetzt weder die Prüfung durch eine bauvorlageberechtigte Person noch den
    Genehmigungsbescheid der Bauaufsichtsbehörde.
  </div>

  <div class="downloads-actions">
    <a class="btn btn--ghost btn--sm" href="../pricing/">Preise und Lizenzen</a>
    <a class="btn btn--ghost btn--sm" href="../docs.html">Dokumentation</a>
    <a class="btn btn--ghost btn--sm" href="../legal/lizenz/">Lizenzvereinbarung</a>
  </div>

  <script type="application/json" id="dlIndex">${JSON.stringify(downloadIndex).replace(/</g, "\\u003c")}</script>
  <script>
  (function () {
    var node = document.getElementById("dlIndex");
    if (!node) return;
    var items;
    try { items = JSON.parse(node.textContent); } catch (e) { return; }

    var ua = navigator.userAgent || "";
    var platform = /Windows/i.test(ua) ? "windows"
      : /Mac OS X|Macintosh/i.test(ua) ? "macos"
      : /Linux|X11|CrOS|Android/i.test(ua) ? "linux"
      : null;

    var label = document.getElementById("dlOsLabel");
    var note = document.getElementById("dlOsNote");
    var button = document.getElementById("dlOsPrimary");

    if (!platform) {
      if (label) label.textContent = "System nicht erkannt";
      if (note) note.textContent = "Wähle unten das passende Paket aus der Liste.";
      return;
    }

    var pretty = { windows: "Windows 10/11", macos: "macOS", linux: "Linux" }[platform];
    if (label) label.textContent = pretty;

    var recommended = null;
    for (var i = 0; i < items.length; i++) {
      if (items[i].primary && items[i].platform === platform) { recommended = items[i]; break; }
    }
    if (!recommended) {
      if (note) note.textContent = "Für dieses System liegt kein Installer bereit; die CLI unten läuft trotzdem.";
      return;
    }

    if (button) { button.href = recommended.url; button.textContent = recommended.label + " herunterladen"; }
    if (note) {
      note.textContent = "Empfohlen: " + recommended.file + ". Prüfe die Datei gegen " +
        "${esc(downloads.checksumFile)} und starte dann den Installer.";
    }
  })();
  </script>
</article>`,
});

/* --------------------------------------------------------------------------
 * Write everything
 * ------------------------------------------------------------------------ */

console.log("Legal pages:");
for (const p of legalPages) {
  write(`legal/${p.slug}/index.html`, page({ title: p.title, description: p.description, depth: 2, body: p.body }));
}

console.log("\nCommercial pages:");
write("pricing/index.html", pricingPage);
write("checkout/success/index.html", successPage);
write("downloads/index.html", downloadsPage);

/* Fragment consumed by index.html between the pricing markers. It is written to
   the system temp directory, not into the repository: it is a build
   intermediate, and leaving it in the tree would mean adding another ignore
   rule for something that has no business being tracked. */
const generatedDir = join(tmpdir(), "baulens-build");
mkdirSync(generatedDir, { recursive: true });
const fragmentPath = join(generatedDir, "pricing.block.html");
writeFileSync(fragmentPath, pricingBlock, "utf8");
console.log(`  ${fragmentPath}  ${pricingBlock.length} chars`);

/* --------------------------------------------------------------------------
 * Inject the pricing block into index.html between its markers, so the
 * landing page, the pricing page and the checkout cannot drift apart.
 * ------------------------------------------------------------------------ */
const START = "<!-- PRICING:START";
const END = "<!-- PRICING:END -->";

const indexHtml = readFileSync("index.html", "utf8").replace(/\r\n/g, "\n");
const startIdx = indexHtml.indexOf(START);
const endIdx = indexHtml.indexOf(END);

assert(startIdx !== -1, "index.html is missing the PRICING:START marker");
assert(endIdx !== -1, "index.html is missing the PRICING:END marker");
assert(startIdx < endIdx, "PRICING markers are out of order in index.html");

const markerLineEnd = indexHtml.indexOf("\n", startIdx);
const head = indexHtml.slice(0, markerLineEnd + 1);

/* The generated fragment is authored at 6-space indent; normalise to 8 to sit
   inside <div class="shell">. */
const indented = pricingBlock
  .split("\n")
  .map((line) => (line.trim() === "" ? "" : "  " + line))
  .join("\n")
  .replace(/\s+$/, "");

const tail = indexHtml.slice(endIdx);
const injected = `${head}\n${indented}\n        ${tail}`;

/* Checkout links are optional until the Polar products exist, so the guard is
   not "there is a buy.polar.sh URL". What must hold is that every tier reached
   the landing page with its printed price and its call to action. */
for (const tier of tiers) {
  assert(injected.includes(money(tier.price)), `pricing injection is missing the price of ${tier.name}`);
  assert(injected.includes(esc(tier.cta)), `pricing injection is missing the CTA of ${tier.name}`);
}
assert(injected.includes("class=\"plan\""), "pricing injection produced no plan cards");

const eol = readFileSync("index.html", "utf8").includes("\r\n") ? "\r\n" : "\n";
writeFileSync("index.html", eol === "\r\n" ? injected.split("\n").join("\r\n") : injected, "utf8");
console.log(`  index.html                            pricing block injected (${injected.length} chars)`);

/* --------------------------------------------------------------------------
 * Cloudflare Pages routing.
 *
 * GitHub Pages honours directory index.html on its own, so the clean routes
 * below are only needed on Cloudflare. Both are written because the site is
 * migrating; neither breaks the other.
 * ------------------------------------------------------------------------ */
const redirects = `# Cloudflare Pages redirects.
#
# The Pages build output directory is the repository ROOT, so every path below
# is a real path in this repo. GitHub Pages resolves the same clean URLs on its
# own via <dir>/index.html; these rules exist so the extensionless legal and
# pricing routes stay stable on Cloudflare.
#
# The web studio is retired: studio.html still exists because the desktop build
# stages it as its engine view, but the website points at /downloads/ instead.
#
# Order matters: static rules first, specific before splats.

# Canonical legal routes.
/impressum    /legal/impressum/    200
/datenschutz  /legal/datenschutz/  200
/agb          /legal/agb/          200
/lizenz       /legal/lizenz/       200
/widerruf     /legal/widerruf/     200
/legal/*      /legal/:splat        200

# Historical and short forms.
/privacy      /legal/datenschutz/  301
/terms        /legal/agb/          301
/legal        /legal/impressum/    301
/success      /checkout/success/   301

# Retired web studio and its aliases -> download hub.
/studio.html   /downloads/         301
/studio/       /downloads/         301
/studio        /downloads/         301
/downloads.html /downloads/        301

# Canonical .html forms -> clean routes.
/pricing.html  /pricing/           301
/index.html    /                   301
`;

writeFileSync("_redirects", redirects, "utf8");
console.log(`  _redirects                            ${redirects.length} chars`);

/* --------------------------------------------------------------------------
 * Keep the version baked into studio.html's generated documents in step.
 *
 * The XBau namensraum and the dossier footer are built from template literals
 * inside studio.html, so they carry a literal version. Rewriting those two
 * occurrences from the config is what stops them drifting from package.json.
 * ------------------------------------------------------------------------ */
const studioBefore = readFileSync("studio.html", "utf8");
const versionPattern = /v(\d+\.\d+\.\d+)/g;
const currentVersion = product.version;

const studioAfter = studioBefore.replace(
  /(<softwareProdukt>BauLens Compliance Engine v)\d+\.\d+\.\d+(<\/softwareProdukt>)/g,
  `$1${currentVersion}$2`
).replace(
  /(Generiert von BauLens Studio v)\d+\.\d+\.\d+/g,
  `$1${currentVersion}`
);

if (studioAfter !== studioBefore) {
  writeFileSync("studio.html", studioAfter, "utf8");
  console.log(`  studio.html                           version stamps -> v${currentVersion}`);
} else {
  console.log(`  studio.html                           version stamps already v${currentVersion}`);
}

const stale = [...studioAfter.matchAll(versionPattern)]
  .map((m) => m[1])
  .filter((v, i, all) => all.indexOf(v) === i);
if (stale.length !== 1 || stale[0] !== currentVersion) {
  console.error(`FAILED: studio.html mentions versions ${stale.join(", ")}, expected only ${currentVersion}`);
  process.exit(1);
}

console.log(`\n${legalPages.length + 3} pages written from site.config.json`);

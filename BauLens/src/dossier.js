import { createHash } from "node:crypto";
import { ENGINE_LABEL } from "./version.js";

/**
 * Generates an SVG vector CAD site plan for embedding into printable dossiers.
 */
export function generateCadSvgPlan(data) {
  const { nord = 4.0, sued = 4.0, ost = 4.0, west = 4.0, reqSetback = 3.0, isPass = true } = data;
  const w = 480, h = 260;
  const padX = 35, padY = 25;
  const plotW = w - padX * 2;
  const plotH = h - padY * 2;

  const scale = 6.8;
  const offN = Math.max(12, Math.min(plotH / 2 - 10, nord * scale));
  const offS = Math.max(12, Math.min(plotH / 2 - 10, sued * scale));
  const offW = Math.max(25, Math.min(plotW / 2 - 20, west * scale));
  const offO = Math.max(25, Math.min(plotW / 2 - 20, ost * scale));

  const bldgX = padX + offW;
  const bldgY = padY + offN;
  const bldgW = Math.max(80, plotW - offW - offO);
  const bldgH = Math.max(30, plotH - offN - offS);

  const strokeColor = isPass ? "#059669" : "#dc2626";
  const fillColor = isPass ? "rgba(5, 150, 105, 0.08)" : "rgba(220, 38, 38, 0.08)";

  return `
  <svg width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" xmlns="http://www.w3.org/2000/svg" style="background:#f8fafc; border:1px solid #cbd5e1; border-radius:6px; font-family:monospace;">
    <!-- Grid -->
    <defs>
      <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
        <path d="M 20 0 L 0 0 0 20" fill="none" stroke="#e2e8f0" stroke-width="0.8"/>
      </pattern>
    </defs>
    <rect width="${w}" height="${h}" fill="url(#grid)" />

    <!-- Boundary -->
    <rect x="${padX}" y="${padY}" width="${plotW}" height="${plotH}" fill="none" stroke="#0284c7" stroke-width="1.8" stroke-dasharray="6,4"/>
    <text x="${padX + 8}" y="${padY + 14}" font-size="10" fill="#0369a1" font-weight="bold">FLURSTÜCKSGRENZE (CAD LAYER: GRENZE)</text>

    <!-- Building -->
    <rect x="${bldgX}" y="${bldgY}" width="${bldgW}" height="${bldgH}" fill="${fillColor}" stroke="${strokeColor}" stroke-width="2"/>
    <text x="${bldgX + bldgW / 2}" y="${bldgY + bldgH / 2 + 4}" font-size="11" fill="#1e293b" font-weight="bold" text-anchor="middle">HAUSKÖRPER</text>

    <!-- Dimension arrows -->
    <!-- North -->
    <line x1="${bldgX + bldgW / 2}" y1="${padY}" x2="${bldgX + bldgW / 2}" y2="${bldgY}" stroke="${strokeColor}" stroke-width="1.5"/>
    <text x="${bldgX + bldgW / 2 + 6}" y="${padY + offN / 2 + 4}" font-size="10" fill="${strokeColor}" font-weight="bold">N: ${nord.toFixed(2)}m (Soll: ${reqSetback.toFixed(2)}m)</text>

    <!-- South -->
    <line x1="${bldgX + bldgW / 2}" y1="${bldgY + bldgH}" x2="${bldgX + bldgW / 2}" y2="${padY + plotH}" stroke="${strokeColor}" stroke-width="1.5"/>
    <text x="${bldgX + bldgW / 2 + 6}" y="${bldgY + bldgH + offS / 2 + 4}" font-size="10" fill="${strokeColor}" font-weight="bold">S: ${sued.toFixed(2)}m</text>

    <!-- Orientation -->
    <text x="${w - 30}" y="${padY + 16}" font-size="12" fill="#0369a1" font-weight="bold" text-anchor="middle">N ↑</text>
  </svg>
  `.trim();
}

/**
 * Reduces a value to the form the seal is computed over: object keys in sorted
 * order, and fields that were not filled in removed.
 *
 * Both rules exist because the seal must not depend on how the caller happens to
 * pass the same document. `JSON.stringify` drops `undefined` but keeps `null`,
 * and it preserves key insertion order, so the CLI (which passes null for a
 * missing Aktenzeichen) and the desktop IPC layer (which passes undefined) used
 * to stamp two different Prüfziffern on the same plan. An auditor comparing a
 * file from the app with one from the command line would have found a mismatch
 * that means nothing. Absent, null and undefined all mean "not filled in" here.
 */
function canonicalise(value) {
  if (Array.isArray(value)) return value.map((entry) => canonicalise(entry));
  if (value === null || typeof value !== "object") return value;

  const out = {};
  for (const key of Object.keys(value).sort()) {
    const entry = value[key];
    if (entry === undefined || entry === null) continue;
    out[key] = canonicalise(entry);
  }
  return out;
}

/**
 * Computes a deterministic SHA-256 verification hash for the report.
 */
export function computeReportHash(spec, verdict) {
  const content = JSON.stringify({
    spec: canonicalise(spec) ?? {},
    verdict: verdict ?? null,
    generator: "BauLens Core v0.3",
  });
  return createHash("sha256").update(content).digest("hex");
}

/**
 * Generates an official, standalone DIN A4 Bauaufsichtlicher Vorprüfungsbefund HTML dossier.
 */
export function generateDossierHtml(projectData, runResult, options = {}) {
  const {
    projekt = "Bauvorhaben Neubau",
    aktenzeichen = "BV-2026-" + Math.floor(100000 + Math.random() * 900000),
    flurstueck = "Flur 4, Flurstück 108/2",
    gemarkung = "Musterhausen",
    entwurfsverfasser = "Architekturbüro Dipl.-Ing. Muster",
    bauherr = "Projektentwicklung GmbH & Co. KG",
  } = projectData;

  const { verdict, counts, results, profile, municipality } = runResult;
  const dateStr = options.datum || new Date().toLocaleDateString("de-DE");
  const hash = computeReportHash(projectData, verdict);

  const isPass = counts.fail === 0;
  const statusColor = isPass ? "#059669" : "#dc2626";
  const statusBg = isPass ? "#ecfdf5" : "#fef2f2";
  const statusTitle = isPass
    ? (counts.warn === 0 ? "BESTANDEN (VOLLSTÄNDIG GENEHMIGUNGSFÄHIG)" : "MIT AUFLAGEN (BEFREIUNGSERFORDERNISSE)")
    : "NICHT BESTANDEN (MATERIELLE MÄNGEL)";

  // SVG Plan
  const svgPlan = generateCadSvgPlan({
    nord: projectData.grundstueck?.grenzabstaende?.norden ?? 4.0,
    sued: projectData.grundstueck?.grenzabstaende?.sueden ?? 4.0,
    ost: projectData.grundstueck?.grenzabstaende?.osten ?? 4.0,
    west: projectData.grundstueck?.grenzabstaende?.westen ?? 4.0,
    reqSetback: 3.5,
    isPass,
  });

  const rows = results.map((r) => {
    const hasFail = r.findings.some((f) => f.status === "fail");
    const hasWarn = r.findings.some((f) => f.status === "warn");
    const statusText = hasFail ? "MANGEL" : (hasWarn ? "AUFLAGE" : "KONFORM");
    const statusPill = hasFail ? "background:#fef2f2; color:#b91c1c; border:1px solid #f87171;"
      : (hasWarn ? "background:#fffbeb; color:#b45309; border:1px solid #fcd34d;"
      : "background:#ecfdf5; color:#047857; border:1px solid #6ee7b7;");

    const messages = r.findings.map((f) => `<div>${f.message}</div>`).join("") || "Keine Beanstandungen.";

    return `
      <tr style="border-bottom: 1px solid #e2e8f0;">
        <td style="padding: 8px 12px; font-weight:bold; font-family: monospace;">${r.para}</td>
        <td style="padding: 8px 12px; font-weight:600;">${r.title}</td>
        <td style="padding: 8px 12px; font-size: 13px; color: #334155;">${messages}</td>
        <td style="padding: 8px 12px; text-align: center;">
          <span style="display:inline-block; font-size:11px; font-weight:bold; padding:2px 8px; border-radius:4px; ${statusPill}">
            ${statusText}
          </span>
        </td>
      </tr>
    `;
  }).join("");

  const countryCode = profile?.country ?? "DE";
  const authorityName = profile?.authorityTitle ?? (countryCode === "AT" ? "Zuständige Baubehörde (Magistrat / Gemeinde)" : (countryCode === "CH" ? "Zuständiges Bauinspektorat" : "Untere Bauaufsichtsbehörde / Bauamt"));
  const flag = countryCode === "AT" ? "🇦🇹" : (countryCode === "CH" ? "🇨🇭" : "🇩🇪");

  return `<!DOCTYPE html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <title>Bauaufsichtlicher Vorprüfungsbefund – ${projekt}</title>
  <style>
    @page { size: A4 portrait; margin: 15mm 20mm; }
    body { font-family: 'Segoe UI', Helvetica, Arial, sans-serif; color: #0f172a; margin: 0; padding: 20px; font-size: 13px; line-height: 1.5; background: #fff; }
    .header-box { border-bottom: 2px solid #0f172a; padding-bottom: 12px; margin-bottom: 20px; display: flex; justify-content: space-between; align-items: flex-end; }
    .title-main { font-size: 20px; font-weight: bold; margin: 0 0 4px 0; color: #0f172a; }
    .title-sub { font-size: 12px; color: #475569; margin: 0; }
    .meta-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 15px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; padding: 12px 16px; margin-bottom: 20px; }
    .meta-item { margin-bottom: 4px; }
    .meta-label { font-weight: 600; color: #64748b; font-size: 11px; text-transform: uppercase; }
    .meta-val { font-size: 13px; font-weight: 500; }
    .verdict-box { border-radius: 6px; padding: 14px 18px; margin-bottom: 24px; border: 1.5px solid ${statusColor}; background: ${statusBg}; display: flex; justify-content: space-between; align-items: center; }
    .verdict-text { font-size: 16px; font-weight: bold; color: ${statusColor}; }
    .verdict-stats { font-family: monospace; font-size: 12px; color: #334155; }
    table { width: 100%; border-collapse: collapse; margin-bottom: 24px; }
    th { background: #f1f5f9; padding: 8px 12px; text-align: left; font-size: 11px; text-transform: uppercase; color: #475569; border-bottom: 1px solid #cbd5e1; }
    .plan-section { margin-bottom: 24px; page-break-inside: avoid; }
    .signature-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 40px; margin-top: 35px; page-break-inside: avoid; }
    .sig-line { border-bottom: 1px solid #0f172a; height: 40px; margin-bottom: 6px; }
    .seal-box { background: #f8fafc; border: 1px dashed #cbd5e1; border-radius: 6px; padding: 10px 14px; font-family: monospace; font-size: 10px; color: #64748b; margin-top: 25px; word-break: break-all; }
    @media print { body { padding: 0; } }
  </style>
</head>
<body>
  <div class="header-box">
    <div>
      <div style="font-size: 11px; font-weight: 700; color: #0369a1; text-transform: uppercase; margin-bottom: 2px;">
        ${flag} ${authorityName}
      </div>
      <h1 class="title-main">Bauaufsichtlicher Vorprüfungsbefund</h1>
      <p class="title-sub">Planungs- und bauordnungsrechtlicher Kriterienabgleich gem. ${profile?.citationBase ?? 'Bauordnung'}</p>
    </div>
    <div style="text-align: right; font-family: monospace; font-size: 11px;">
      <div>AZ: ${aktenzeichen}</div>
      <div>Datum: ${dateStr}</div>
    </div>
  </div>

  <div class="meta-grid">
    <div>
      <div class="meta-item"><span class="meta-label">Bauvorhaben / Projekt:</span> <span class="meta-val">${projekt}</span></div>
      <div class="meta-item"><span class="meta-label">Lage / Parzelle:</span> <span class="meta-val">${flurstueck}, Gem. ${gemarkung}</span></div>
      <div class="meta-item"><span class="meta-label">Rechtsbereich:</span> <span class="meta-val">${flag} ${profile?.name ?? 'DACH'} (${profile?.citationBase ?? 'Referenz'})</span></div>
    </div>
    <div>
      <div class="meta-item"><span class="meta-label">Entwurfsverfasser:</span> <span class="meta-val">${entwurfsverfasser}</span></div>
      <div class="meta-item"><span class="meta-label">Bauherrschaft:</span> <span class="meta-val">${bauherr}</span></div>
      <div class="meta-item"><span class="meta-label">Kommune / Satzungen:</span> <span class="meta-val">${municipality ? municipality.name + ' (Satzungen aktiv)' : (countryCode === 'CH' ? 'Gemeinde / BZO' : (countryCode === 'AT' ? 'Gemeinde / Bebauungsplan' : 'Standard'))}</span></div>
    </div>
  </div>

  <div class="verdict-box">
    <div>
      <div style="font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: ${statusColor}; font-weight: bold;">Gesamtergebnis der automatisierten Vorprüfung</div>
      <div class="verdict-text">${statusTitle}</div>
    </div>
    <div class="verdict-stats">
      ${counts.ok} Erfüllt · ${counts.warn} Auflagen · ${counts.fail} Mängel
    </div>
  </div>

  <h3 style="font-size: 14px; text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 8px;">1. Tabellarischer Prüfbefund</h3>
  <table>
    <thead>
      <tr>
        <th style="width: 15%;">Norm (§)</th>
        <th style="width: 25%;">Prüfkriterium</th>
        <th>Befund &amp; Auswertung</th>
        <th style="width: 15%; text-align: center;">Status</th>
      </tr>
    </thead>
    <tbody>
      ${rows}
    </tbody>
  </table>

  <div class="plan-section">
    <h3 style="font-size: 14px; text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 8px;">2. Lageplan &amp; Abstandsflächengeometrie (CAD Vektor)</h3>
    ${svgPlan}
  </div>

  <div class="signature-grid">
    <div>
      <div class="sig-line"></div>
      <div style="font-size: 11px; color: #475569;">Ort, Datum / Unterschrift Bauherr(in)</div>
    </div>
    <div>
      <div class="sig-line"></div>
      <div style="font-size: 11px; color: #475569;">Unterschrift bauvorlageberechtigte(r) Entwurfsverfasser(in)</div>
    </div>
  </div>

  <div class="seal-box">
    <strong>DIGITALES PRÜFSIEGEL (SHA-256):</strong> ${hash}<br>
    Prüfungsstand: DACH ${ENGINE_LABEL} · Berechnet deterministisch ohne externe Cloud-Dienste · Keine Rechtsberatung gem. RDG.
  </div>
</body>
</html>`;
}

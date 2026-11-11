#!/usr/bin/env node
"use strict";

/**
 * tools/generate_outreach_letters.mjs
 * 
 * Generates personalized, print-ready DIN-A4 HTML letters for German postal outreach
 * compliant with § 7 UWG & DSGVO Art. 6 Abs. 1 lit. f.
 * 
 * Includes:
 * - Proper DIN 5008 address field formatting (compatible with C6/5 window envelopes)
 * - Unique lead tracking QR code (pointing to https://clause.kellersystems.dev?ref=<ID>)
 * - Direct, tailored BGB value proposition
 * - Integrated print stylesheet (@media print) for direct PDF export or 1-click printer output.
 */

import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { resolve, join } from "node:path";

const ROOT_DIR = resolve(".");
const LEADS_FILE = join(ROOT_DIR, "outreach", "TARGET_LEADS.json");
const OUT_DIR = join(ROOT_DIR, "outreach", "letters");

if (!existsSync(OUT_DIR)) {
  mkdirSync(OUT_DIR, { recursive: true });
}

const leads = JSON.parse(readFileSync(LEADS_FILE, "utf8"));

function generateLetterHtml(lead) {
  const refUrl = `https://clause.kellersystems.dev/?ref=${encodeURIComponent(lead.id)}&target=${encodeURIComponent(lead.company)}`;
  const qrApiUrl = `https://api.qrserver.com/v1/create-qr-code/?size=140x140&data=${encodeURIComponent(refUrl)}&margin=0`;

  return `<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <title>Brief: ${lead.company} (${lead.id})</title>
  <style>
    @page {
      size: A4 portrait;
      margin: 20mm 20mm 20mm 25mm;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      font-size: 11pt;
      line-height: 1.5;
      color: #1a1a1a;
      background: #fff;
    }
    .letter-page {
      width: 100%;
      max-width: 170mm;
      margin: 0 auto;
      padding: 10mm 0;
    }
    /* DIN 5008 Absenderzeile */
    .sender-return-line {
      font-size: 7pt;
      color: #666;
      border-bottom: 0.5pt solid #999;
      padding-bottom: 2mm;
      margin-bottom: 4mm;
      letter-spacing: 0.05em;
    }
    /* Empfängerfeld (DIN 5008 Fensterzone) */
    .recipient-address {
      height: 35mm;
      margin-bottom: 12mm;
      font-size: 10pt;
      line-height: 1.4;
    }
    .recipient-address .confidential {
      font-size: 8pt;
      font-weight: bold;
      color: #b8442c;
      text-transform: uppercase;
      margin-bottom: 2mm;
    }
    /* Metadaten Datum & Betreff */
    .meta-row {
      display: flex;
      justify-content: flex-end;
      margin-bottom: 8mm;
      font-size: 9.5pt;
      color: #555;
    }
    .subject {
      font-size: 13pt;
      font-weight: 700;
      color: #0f172a;
      margin-bottom: 6mm;
      line-height: 1.3;
    }
    .body-p {
      margin-bottom: 4.5mm;
      text-align: justify;
    }
    .highlight-box {
      background: #f8fafc;
      border-left: 3pt solid #2c5282;
      padding: 3.5mm 4.5mm;
      margin: 4.5mm 0;
      font-size: 10pt;
    }
    .highlight-box ul {
      margin-left: 5mm;
      margin-top: 2mm;
    }
    .highlight-box li {
      margin-bottom: 1.5mm;
    }
    /* Footer mit QR-Code */
    .qr-footer {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-top: 6mm;
      padding-top: 4mm;
      border-top: 1pt solid #e2e8f0;
    }
    .qr-text {
      font-size: 9pt;
      color: #475569;
      max-width: 110mm;
    }
    .qr-text strong {
      color: #0f172a;
    }
    .qr-code img {
      width: 28mm;
      height: 28mm;
      display: block;
      border: 1pt solid #cbd5e1;
      padding: 1mm;
      border-radius: 2mm;
    }
    .signature {
      margin-top: 6mm;
      margin-bottom: 4mm;
    }
    .signature-name {
      font-weight: 600;
      color: #0f172a;
    }
    .signature-title {
      font-size: 8.5pt;
      color: #64748b;
    }
    @media print {
      body { background: transparent; }
      .letter-page { padding: 0; }
      .no-print { display: none; }
    }
  </style>
</head>
<body>
  <div class="letter-page">
    <div class="sender-return-line">
      Keller Systems &bull; ClauseLens B2B Legal Technology &bull; https://clause.kellersystems.dev
    </div>

    <div class="recipient-address">
      <div class="confidential">Vertraulich &bull; Persönlich an die Geschäftsleitung</div>
      <strong>${lead.company}</strong><br>
      ${lead.contactPerson ? `z. Hd. ${lead.contactPerson}<br>` : "z. Hd. der Geschäftsführung<br>"}
      ${lead.street}<br>
      <strong>${lead.zip} ${lead.city}</strong>
    </div>

    <div class="meta-row">
      ${lead.city}, den ${new Date().toLocaleDateString("de-DE", { day: "2-digit", month: "long", year: "numeric" })}
    </div>

    <div class="subject">
      BGB-Haftungsrisiken in Dienstleisterverträgen: Vorprüfung Ihrer Vertragsklauseln
    </div>

    <p class="body-p">
      Sehr geehrte Geschäftsleitung, sehr geehrte Damen und Herren,
    </p>

    <p class="body-p">
      als IT-Dienstleister und Systemhaus kennen Sie die Herausforderung bei B2B-Aufträgen: Kunden und Auftraggeber fordern immer weitreichendere Haftungsübernahmen, verschleierte SLA-Strafen oder einseitige Kündigungsrechte. Häufig werden solche Klauselwerke unter hohem Zeitdruck gegengezeichnet.
    </p>

    <p class="body-p">
      Die juristische Praxis nach §§ 305–310 BGB zeigt jedoch: <strong>Über 60 % der in der Praxis verwendeten AGB- und Haftungsklauseln sind vor deutschen Gerichten angreifbar oder unwirksam.</strong> Das gravierende Risiko: Kippt eine Klausel, gilt automatisch das unbegrenzte gesetzliche BGB-Haftungsrecht zu Ihren Lasten.
    </p>

    <div class="highlight-box">
      <strong>Typische Klauselfallen, die ClauseLens in Millisekunden identifiziert:</strong>
      <ul>
        <li><strong>Verschleierte Haftungsausschlüsse:</strong> Klauseln, die wesentliche Vertragspflichten (Kardinalpflichten) unzulässig aushöhlen (§ 307 BGB).</li>
        <li><strong>Unwirksame Pauschalstrafen:</strong> Strafgebühren und pönalisierte Mehraufwände ohne Schadensnachweismöglichkeit (§ 309 Nr. 5 BGB).</li>
        <li><strong>Ehehafte Kündigungs- und Verlängerungsklauseln:</strong> Versteckte Verlängerungsautomatiken in Dauerschuldverhältnissen.</li>
      </ul>
    </div>

    <p class="body-p">
      Um diesen Prüfprozess für Sie vollständig zu automatisieren, haben wir <strong>ClauseLens</strong> entwickelt: Eine deterministische Vorprüf-Engine, die PDF- und DOCX-Verträge in Sekunden lokal auf 47 BGB-Fallgruppen scannt – <strong>zu 100 % lokal auf Ihrem PC, ohne Cloud-Übertragung und vollkommen DSGVO-konform.</strong>
    </p>

    <div class="signature">
      Mit freundlichen Grüßen,<br><br>
      <div class="signature-name">Lukas Keller</div>
      <div class="signature-title">Entwicklungsleitung ClauseLens &bull; Keller Systems</div>
    </div>

    <div class="qr-footer">
      <div class="qr-text">
        <strong>Ihr persönlicher Vorprüf-Zugang:</strong><br>
        Scannen Sie den QR-Code mit Ihrem Smartphone oder rufen Sie auf:<br>
        <span style="font-family: monospace; font-size: 8pt; color: #2563eb;">clause.kellersystems.dev/?ref=${lead.id}</span><br>
        <em>Testen Sie die Vorprüfung unverbindlich an einem Ihrer aktuellen Kundenverträge.</em>
      </div>
      <div class="qr-code">
        <img src="${qrApiUrl}" alt="QR-Code für ${lead.id}" />
      </div>
    </div>
  </div>
</body>
</html>`;
}

console.log(`Generating personalized letters for ${leads.length} leads...`);

let count = 0;
for (const lead of leads) {
  const fileName = `Brief_${lead.id}_${lead.company.replace(/[^a-zA-Z0-9]/g, "_").substring(0, 30)}.html`;
  writeFileSync(join(OUT_DIR, fileName), generateLetterHtml(lead), "utf8");
  count++;
}

// Generate print index page linking to all letters
const indexHtml = `<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <title>ClauseLens Post-Outreach: Druckübersicht (24 Briefe)</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 900px; margin: 30px auto; padding: 0 20px; line-height: 1.5; color: #1e293b; }
    h1 { margin-bottom: 8px; font-size: 24px; }
    p { color: #64748b; margin-bottom: 24px; }
    table { width: 100%; border-collapse: collapse; margin-top: 10px; font-size: 14px; }
    th, td { padding: 10px 14px; border: 1px solid #cbd5e1; text-align: left; }
    th { background: #f1f5f9; font-weight: 600; }
    tr:hover { background: #f8fafc; }
    .btn { display: inline-block; padding: 4px 10px; background: #2563eb; color: #fff; text-decoration: none; border-radius: 4px; font-size: 12px; font-weight: 500; }
    .btn:hover { background: #1d4ed8; }
  </style>
</head>
<body>
  <h1>ClauseLens Post-Outreach Druck-Zentrale</h1>
  <p>Hier sind alle ${leads.length} personalisierten DIN-A4 Anschreiben mit individuellen QR-Codes bereitgestellt. Klicken Sie auf einen Brief, um ihn im Browser zu öffnen und per <strong>Strg + P</strong> als PDF zu drucken oder direkt an den Drucker zu senden.</p>
  
  <table>
    <thead>
      <tr>
        <th>ID</th>
        <th>Unternehmen</th>
        <th>Ort</th>
        <th>Ansprechpartner</th>
        <th>Aktion</th>
      </tr>
    </thead>
    <tbody>
      ${leads.map(l => {
        const f = `Brief_${l.id}_${l.company.replace(/[^a-zA-Z0-9]/g, "_").substring(0, 30)}.html`;
        return `<tr>
          <td><strong>${l.id}</strong></td>
          <td>${l.company}</td>
          <td>${l.zip} ${l.city}</td>
          <td>${l.contactPerson || "Geschäftsführung"}</td>
          <td><a href="${f}" target="_blank" class="btn">Brief drucken &rarr;</a></td>
        </tr>`;
      }).join("\n      ")}
    </tbody>
  </table>
</body>
</html>`;

writeFileSync(join(OUT_DIR, "index.html"), indexHtml, "utf8");
console.log(`Successfully generated ${count} personalized letters + print index in outreach/letters/`);

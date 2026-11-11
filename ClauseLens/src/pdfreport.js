"use strict";

import { createHash } from "node:crypto";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

function escapePdfText(str) {
  if (typeof str !== "string") return "";
  // Map German umlauts and special typography to standard ASCII / Type 1 transliterations
  const clean = str
    .replace(/ä/g, "ae")
    .replace(/Ä/g, "Ae")
    .replace(/ö/g, "oe")
    .replace(/Ö/g, "Oe")
    .replace(/ü/g, "ue")
    .replace(/Ü/g, "Ue")
    .replace(/ß/g, "ss")
    .replace(/§/g, "Par.")
    .replace(/–/g, "-")
    .replace(/—/g, "-")
    .replace(/[^\x20-\x7E]/g, " ");

  return clean.replace(/\\/g, "\\\\").replace(/\(/g, "\\(").replace(/\)/g, "\\)");
}

function wrapText(text, maxChars = 85) {
  if (!text) return [];
  const words = text.split(/\s+/);
  const lines = [];
  let cur = "";

  for (const w of words) {
    if (!cur) {
      cur = w;
    } else if (cur.length + 1 + w.length <= maxChars) {
      cur += " " + w;
    } else {
      lines.push(cur);
      cur = w;
    }
  }
  if (cur) lines.push(cur);
  return lines;
}

function renderFindingCard(ops, f, idx, yTop) {
  const isHoch = f.severity === "hoch";
  const isMittel = f.severity === "mittel";
  const cardH = 92;
  const cardY = yTop - cardH;

  // Background Box
  ops.push("0.98 0.98 0.97 rg");
  ops.push(`40 ${cardY} 515 ${cardH} re f`);
  ops.push("0.85 0.82 0.76 RG");
  ops.push("0.75 w");
  ops.push(`40 ${cardY} 515 ${cardH} re S`);

  // Left Severity Indicator Bar (4pt)
  if (isHoch) ops.push("0.72 0.27 0.17 rg"); // red #b8442c
  else if (isMittel) ops.push("0.75 0.52 0.15 rg"); // amber #b88526
  else ops.push("0.25 0.50 0.30 rg"); // green
  ops.push(`40 ${cardY} 4 ${cardH} re f`);

  // 1. Title line
  const loc = Array.isArray(f.lineNumbers) && f.lineNumbers.length
    ? `(Zeilen ${f.lineNumbers.join("-")})`
    : "";
  const cite = f.citation ? `[${f.citation}] ` : "";
  const titleText = `${idx + 1}. [${(f.severity || "INFO").toUpperCase()}] ${cite}${f.titlePlainGerman || f.type} ${loc}`;
  ops.push("BT");
  ops.push("/F2 9.5 Tf"); // Bold
  ops.push("0.1 0.1 0.1 rg");
  ops.push(`52 ${yTop - 18} Td`);
  ops.push(`(${escapePdfText(titleText.slice(0, 102))}) Tj`);
  ops.push("ET");

  // 2. Excerpt line
  const excerptText = f.excerpt ? `Fundstelle: "${f.excerpt}"` : "Fundstelle: (Klauselkontext)";
  const excerptLines = wrapText(excerptText, 95);
  ops.push("BT");
  ops.push("/F1 8.5 Tf"); // Regular
  ops.push("0.35 0.35 0.35 rg");
  ops.push(`52 ${yTop - 34} Td`);
  ops.push(`(${escapePdfText(excerptLines[0] || "")}) Tj`);
  ops.push("ET");

  // 3. Why line
  const whyText = f.why ? `Risiko: ${f.why}` : "";
  const whyLines = wrapText(whyText, 95);
  ops.push("BT");
  ops.push("/F1 8.5 Tf");
  ops.push("0.2 0.2 0.2 rg");
  ops.push(`52 ${yTop - 50} Td`);
  ops.push(`(${escapePdfText(whyLines[0] || "")}) Tj`);
  ops.push("ET");

  // 4. Suggestion line
  const suggText = f.suggestion ? `Gegenvorschlag: ${f.suggestion}` : "";
  const suggLines = wrapText(suggText, 92);
  ops.push("BT");
  ops.push("/F2 8.5 Tf"); // Bold
  ops.push("0.15 0.45 0.2 rg"); // Green tint
  ops.push(`52 ${yTop - 74} Td`);
  ops.push(`(${escapePdfText(suggLines[0] || "")}) Tj`);
  ops.push("ET");
}

function renderFooter(ops, pageNum, totalPages, fingerprint) {
  ops.push("0.85 0.82 0.76 RG");
  ops.push("0.5 w");
  ops.push("40 60 515 0.5 re S");

  ops.push("BT");
  ops.push("/F1 8 Tf");
  ops.push("0.45 0.45 0.45 rg");
  ops.push("40 45 Td");
  ops.push(`(${escapePdfText("Hinweis: Automatisierte Vorpruefung nach BGB Par. 305-310. Keine Rechtsberatung im Sinne des RDG.")}) Tj`);
  ops.push("40 33 Td");
  ops.push(`(${escapePdfText("ClauseLens v1.0.0 Enterprise Engine  -  https://clauselens.kellersystems.dev")}) Tj`);
  ops.push("ET");

  ops.push("BT");
  ops.push("/F2 8 Tf");
  ops.push("0.35 0.35 0.35 rg");
  ops.push("475 45 Td");
  ops.push(`(${escapePdfText(`Seite ${pageNum} von ${totalPages}`)}) Tj`);
  ops.push("ET");

  ops.push("BT");
  ops.push("/F3 7 Tf");
  ops.push("0.5 0.5 0.5 rg");
  ops.push("475 33 Td");
  ops.push(`(${escapePdfText(`ID:${fingerprint.slice(0, 10)}`)}) Tj`);
  ops.push("ET");
}

/**
 * Generates an executive, multi-page vector PDF audit report in DIN A4.
 * Zero external dependencies, pure native Node.js PDF 1.4 vector drawing.
 */
export function generatePdfReport(result, meta = {}) {
  const sourceName = meta.sourceName || meta.source || "Unbekannter Vertrag";
  const analyzedAt = meta.analyzedAt || new Date().toISOString().slice(0, 19).replace("T", " ");
  const score = Math.max(0, Math.min(100, Number(result.riskScore) || 0));
  const level = (result.riskLevel || "niedrig").toUpperCase();
  const counts = result.counts || { hoch: 0, mittel: 0, niedrig: 0 };
  const findings = Array.isArray(result.findings) ? result.findings : [];

  const fingerprint = createHash("sha256")
    .update(JSON.stringify(result) + sourceName)
    .digest("hex")
    .slice(0, 24)
    .toUpperCase();

  // Multi-page layout calculations:
  // Page 1 contains executive summary + top 3 findings (or fewer if <= 3 total).
  // Continuation pages hold up to 5 findings per page.
  const FINDINGS_PAGE_1 = 3;
  const FINDINGS_PER_CONT_PAGE = 5;

  let totalPages = 1;
  if (findings.length > FINDINGS_PAGE_1) {
    totalPages = 1 + Math.ceil((findings.length - FINDINGS_PAGE_1) / FINDINGS_PER_CONT_PAGE);
  }

  const pagesOps = [];

  // ==========================================
  // PAGE 1: EXECUTIVE AUDIT SUMMARY & TOP CARDS
  // ==========================================
  const page1 = [];

  // Background Header Bar (Dark slate)
  page1.push("0.07 0.07 0.06 rg"); // #121110
  page1.push("0 740 595.28 102 re f");

  // Red Accent Line
  page1.push("0.72 0.27 0.17 rg"); // #b8442c
  page1.push("0 736 595.28 4 re f");

  // Header Title
  page1.push("BT");
  page1.push("/F2 20 Tf"); // Helvetica-Bold 20pt
  page1.push("1 1 1 rg");
  page1.push("40 790 Td");
  page1.push(`(${escapePdfText("CLAUSELENS - VERTRAGSRISIKO-AUDIT")}) Tj`);
  page1.push("ET");

  // Header Subtitle
  page1.push("BT");
  page1.push("/F1 10 Tf"); // Helvetica 10pt
  page1.push("0.8 0.78 0.75 rg");
  page1.push("40 762 Td");
  page1.push(`(${escapePdfText(`Dokument: ${sourceName}  |  Datum: ${analyzedAt}  |  BGB Par. 305-310`)}) Tj`);
  page1.push("ET");

  // Executive Score Box (Left)
  page1.push("0.97 0.96 0.94 rg");
  page1.push("40 635 240 85 re f");
  page1.push("0.85 0.82 0.76 RG");
  page1.push("1 w");
  page1.push("40 635 240 85 re S");

  // Score Number
  page1.push("BT");
  page1.push("/F2 28 Tf");
  if (score >= 70) page1.push("0.72 0.27 0.17 rg");
  else if (score >= 35) page1.push("0.72 0.53 0.17 rg");
  else page1.push("0.25 0.48 0.29 rg");
  page1.push("56 678 Td");
  page1.push(`(${score}) Tj`);
  page1.push("ET");

  page1.push("BT");
  page1.push("/F1 11 Tf");
  page1.push("0.3 0.3 0.3 rg");
  page1.push("105 688 Td");
  page1.push("(/ 100 RISIKO-PUNKTE) Tj");
  page1.push("ET");

  page1.push("BT");
  page1.push("/F2 11 Tf");
  page1.push("56 652 Td");
  page1.push(`(${escapePdfText(`Einstufung: ${level}  (${counts.hoch}x Hoch, ${counts.mittel}x Mittel)`)}) Tj`);
  page1.push("ET");

  // Inspection Fingerprint Box (Right)
  page1.push("0.97 0.96 0.94 rg");
  page1.push("300 635 255 85 re f");
  page1.push("0.85 0.82 0.76 RG");
  page1.push("300 635 255 85 re S");

  page1.push("BT");
  page1.push("/F2 9.5 Tf");
  page1.push("0.2 0.2 0.2 rg");
  page1.push("314 695 Td");
  page1.push("(OFFIZIELLER PRUEFSTEMPEL & INTEGRITAET) Tj");
  page1.push("ET");

  page1.push("BT");
  page1.push("/F3 8.5 Tf"); // Courier
  page1.push("0.4 0.4 0.4 rg");
  page1.push("314 675 Td");
  page1.push(`(${escapePdfText(`SHA256: ${fingerprint}`)}) Tj`);
  page1.push("314 658 Td");
  page1.push("(AIR-GAPPED AUDIT - ZERO EXTERNAL EGRESS) Tj");
  page1.push("314 645 Td");
  page1.push("(DSGVO-KONFORM - OHNE CLOUD-LLM) Tj");
  page1.push("ET");

  // Findings Section Header
  page1.push("BT");
  page1.push("/F2 13 Tf");
  page1.push("0.1 0.1 0.1 rg");
  page1.push("40 602 Td");
  page1.push(`(${escapePdfText(`RISIKOBEFUNDE (${findings.length} Gesamtbefunde)`)}) Tj`);
  page1.push("ET");

  if (findings.length > FINDINGS_PAGE_1) {
    page1.push("BT");
    page1.push("/F1 8.5 Tf");
    page1.push("0.45 0.45 0.45 rg");
    page1.push("40 588 Td");
    page1.push(`(${escapePdfText(`Zeige die ersten 3 Kernbefunde; Fortsetzung der Details ab Seite 2.`)}) Tj`);
    page1.push("ET");
  }

  if (findings.length === 0) {
    // Clean document badge
    page1.push("0.95 0.98 0.95 rg");
    page1.push("40 480 515 90 re f");
    page1.push("0.3 0.6 0.35 RG");
    page1.push("1 w");
    page1.push("40 480 515 90 re S");
    page1.push("0.25 0.50 0.30 rg");
    page1.push("40 480 4 90 re f");

    page1.push("BT");
    page1.push("/F2 12 Tf");
    page1.push("0.15 0.45 0.2 rg");
    page1.push("56 540 Td");
    page1.push(`(${escapePdfText("KEINE RECHTLICHEN RISIKEN IDENTIFIZIERT")}) Tj`);
    page1.push("ET");

    page1.push("BT");
    page1.push("/F1 9.5 Tf");
    page1.push("0.25 0.25 0.25 rg");
    page1.push("56 518 Td");
    page1.push(`(${escapePdfText("Die Pruefung nach BGB Par. 305-310 ergab keine unwirksamen oder unausgewogenen Klauseln.")}) Tj`);
    page1.push("56 500 Td");
    page1.push(`(${escapePdfText("Der Vertrag kann unter Beruecksichtigung der Pruefbaseline freigegeben werden.")}) Tj`);
    page1.push("ET");
  } else {
    // Render first batch on Page 1
    const p1Findings = findings.slice(0, FINDINGS_PAGE_1);
    let y = findings.length > FINDINGS_PAGE_1 ? 575 : 580;
    for (let i = 0; i < p1Findings.length; i++) {
      renderFindingCard(page1, p1Findings[i], i, y);
      y -= 104;
    }
  }

  // Footer on Page 1
  renderFooter(page1, 1, totalPages, fingerprint);
  pagesOps.push(page1);

  // ==========================================
  // CONTINUATION PAGES (Page 2 .. totalPages)
  // ==========================================
  for (let p = 2; p <= totalPages; p++) {
    const pageN = [];

    // Compact Continuation Running Header Bar
    pageN.push("0.07 0.07 0.06 rg");
    pageN.push("0 802 595.28 40 re f");

    // Red Accent Line
    pageN.push("0.72 0.27 0.17 rg");
    pageN.push("0 798 595.28 4 re f");

    // Running Header text
    pageN.push("BT");
    pageN.push("/F2 11 Tf");
    pageN.push("1 1 1 rg");
    pageN.push("40 820 Td");
    pageN.push(`(${escapePdfText("CLAUSELENS - RISIKOBERICHT (BEFUND-DETAILS)")}) Tj`);
    pageN.push("ET");

    pageN.push("BT");
    pageN.push("/F1 8.5 Tf");
    pageN.push("0.8 0.78 0.75 rg");
    pageN.push("40 808 Td");
    pageN.push(`(${escapePdfText(`Dokument: ${sourceName}  |  Audit-ID: ${fingerprint.slice(0, 16)}`)}) Tj`);
    pageN.push("ET");

    // Render continuation cards
    const startIdx = FINDINGS_PAGE_1 + (p - 2) * FINDINGS_PER_CONT_PAGE;
    const endIdx = Math.min(findings.length, startIdx + FINDINGS_PER_CONT_PAGE);
    const contBatch = findings.slice(startIdx, endIdx);

    let y = 780;
    for (let i = 0; i < contBatch.length; i++) {
      const globalIdx = startIdx + i;
      renderFindingCard(pageN, contBatch[i], globalIdx, y);
      y -= 106;
    }

    renderFooter(pageN, p, totalPages, fingerprint);
    pagesOps.push(pageN);
  }

  // ==========================================
  // CONSTRUCT PDF 1.4 OBJECT GRAPH
  // ==========================================
  // Obj 1: Catalog
  // Obj 2: Pages (Kids array)
  // Obj 3: Font /Helvetica
  // Obj 4: Font /Helvetica-Bold
  // Obj 5: Font /Courier
  // For each page p (0..totalPages-1):
  //   Page Obj Num: 6 + p * 2
  //   Stream Obj Num: 6 + p * 2 + 1
  const objects = [];

  objects.push("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

  const pageRefs = [];
  for (let p = 0; p < totalPages; p++) {
    const pageObjNum = 6 + p * 2;
    pageRefs.push(`${pageObjNum} 0 R`);
  }
  objects.push(`2 0 obj\n<< /Type /Pages /Kids [${pageRefs.join(" ")}] /Count ${totalPages} >>\nendobj\n`);

  objects.push("3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
  objects.push("4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n");
  objects.push("5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n");

  for (let p = 0; p < totalPages; p++) {
    const pageObjNum = 6 + p * 2;
    const streamObjNum = pageObjNum + 1;
    const streamContent = pagesOps[p].join("\n");
    const streamLength = Buffer.byteLength(streamContent, "utf-8");

    objects.push(
      `${pageObjNum} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595.28 841.89] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents ${streamObjNum} 0 R >>\nendobj\n`
    );
    objects.push(
      `${streamObjNum} 0 obj\n<< /Length ${streamLength} >>\nstream\n${streamContent}\nendstream\nendobj\n`
    );
  }

  // Assemble PDF with exact xref table
  let pdf = "%PDF-1.4\n%\xe2\xe3\xcf\xd3\n";
  const xrefOffsets = [0];

  for (const obj of objects) {
    xrefOffsets.push(Buffer.byteLength(pdf, "utf-8"));
    pdf += obj;
  }

  const xrefStart = Buffer.byteLength(pdf, "utf-8");
  pdf += `xref\n0 ${xrefOffsets.length}\n`;
  pdf += "0000000000 65535 f \n";
  for (let i = 1; i < xrefOffsets.length; i++) {
    pdf += String(xrefOffsets[i]).padStart(10, "0") + " 00000 n \n";
  }
  pdf += `trailer\n<< /Size ${xrefOffsets.length} /Root 1 0 R >>\nstartxref\n${xrefStart}\n%%EOF\n`;

  return Buffer.from(pdf, "utf-8");
}

export function writePdfReport(pdfBuffer, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, pdfBuffer);
}

"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { generatePdfReport, writePdfReport } from "../src/pdfreport.js";
import { analyzeContract } from "../src/analyze.js";
import { pdfToText } from "../src/pdfextract.js";
import { mkdtempSync, readFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

test("pdfreport: generates valid DIN A4 PDF 1.4 buffer with inspection seal", () => {
  const text = `
    Der Vertrag verlängert sich automatisch um weitere 24 Monate.
    Die Haftung des Anbieters ist für sämtliche Schäden ausgeschlossen.
  `;
  const result = analyzeContract(text);
  const pdfBuffer = generatePdfReport(result, { sourceName: "muster_vertrag.docx" });

  assert.ok(Buffer.isBuffer(pdfBuffer));
  assert.ok(pdfBuffer.length > 500);

  const pdfStr = pdfBuffer.toString("utf-8");
  assert.ok(pdfStr.startsWith("%PDF-1.4"));
  assert.ok(pdfStr.includes("CLAUSELENS - VERTRAGSRISIKO-AUDIT"));
  assert.ok(pdfStr.includes("SHA256:"));
  assert.ok(pdfStr.includes("/Count 1"));
  assert.ok(pdfStr.includes("startxref"));
  assert.ok(pdfStr.endsWith("%%EOF\n"));
});

test("pdfreport: generates clean report with affirmative badge when 0 findings", () => {
  const result = analyzeContract("Die Parteien vereinbaren eine transparente Zusammenarbeit.");
  assert.equal(result.findings.length, 0);

  const pdfBuffer = generatePdfReport(result, { sourceName: "clean_contract.txt" });
  const pdfStr = pdfBuffer.toString("utf-8");

  assert.ok(pdfStr.includes("/Count 1"));
  assert.ok(pdfStr.includes("KEINE RECHTLICHEN RISIKEN IDENTIFIZIERT"));
  assert.ok(pdfStr.includes("Seite 1 von 1"));
});

test("pdfreport: generates multi-page PDF when findings exceed page capacity", () => {
  // Construct finding set with 9 findings (Page 1 has 3, Page 2 has 5, Page 3 has 1 -> 3 pages)
  const mockFindings = [];
  for (let i = 1; i <= 9; i++) {
    mockFindings.push({
      type: `test_trap_${i}`,
      titlePlainGerman: `Test-Risikoklausel Nr. ${i}`,
      severity: i % 2 === 0 ? "hoch" : "mittel",
      lineNumbers: [i * 10, i * 10 + 2],
      excerpt: `Dies ist der Auszug fuer verdaechtige Klausel ${i}`,
      why: `Erklaerung weshalb Klausel ${i} das AGB-Recht verletzt.`,
      suggestion: `Empfohlene Ersatzformulierung fuer Klausel ${i}.`,
    });
  }

  const mockResult = {
    riskScore: 85,
    riskLevel: "hoch",
    counts: { hoch: 4, mittel: 5, niedrig: 0 },
    findings: mockFindings,
    summary: "Gesamtbewertung ueber 9 identifizierte Risiken.",
  };

  const pdfBuffer = generatePdfReport(mockResult, { sourceName: "grosser_enterprise_vertrag.pdf" });
  const pdfStr = pdfBuffer.toString("utf-8");

  // Check multi-page catalog and structure
  assert.ok(pdfStr.includes("/Count 3"));
  assert.ok(pdfStr.includes("/Kids [6 0 R 8 0 R 10 0 R]"));
  assert.ok(pdfStr.includes("CLAUSELENS - RISIKOBERICHT"));
  assert.ok(pdfStr.includes("BEFUND-DETAILS"));
  assert.ok(pdfStr.includes("Seite 1 von 3"));
  assert.ok(pdfStr.includes("Seite 2 von 3"));
  assert.ok(pdfStr.includes("Seite 3 von 3"));

  // Check that all 9 findings are contained in the text stream
  for (let i = 1; i <= 9; i++) {
    assert.ok(
      pdfStr.includes(`Test-Risikoklausel Nr. ${i}`),
      `Finding ${i} must be present in the multi-page PDF`
    );
  }

  // Verify round-trip extraction using ClauseLens native pdfToText
  const extractedText = pdfToText(pdfBuffer);
  assert.ok(extractedText.includes("CLAUSELENS - VERTRAGSRISIKO-AUDIT"));
  assert.ok(extractedText.includes("Test-Risikoklausel Nr. 1"));
  assert.ok(extractedText.includes("Test-Risikoklausel Nr. 9"));
});

test("pdfreport: writePdfReport writes binary PDF file to disk", () => {
  const tmp = mkdtempSync(path.join(tmpdir(), "cl-pdf-"));
  try {
    const outPath = path.join(tmp, "report.pdf");
    const result = analyzeContract("Die Haftung ist ausgeschlossen.");
    const buf = generatePdfReport(result);
    writePdfReport(buf, outPath);

    assert.ok(existsSync(outPath));
    const diskBuf = readFileSync(outPath);
    assert.equal(diskBuf.length, buf.length);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

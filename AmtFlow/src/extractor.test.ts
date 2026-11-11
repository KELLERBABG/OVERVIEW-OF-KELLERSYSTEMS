import { test, describe } from 'node:test';
import assert from 'node:assert';
import { LetterExtractor } from './extractor.js';

describe('BriefHelfer / AmtFlow Document Intelligence', () => {
  test('correctly understands Versicherungsnummernachweis (DRV) with zero deadline', () => {
    const rawRVNR = `
      Deutsche Rentenversicherung Bund
      10709 Berlin
      
      Versicherungsnummernachweis
      
      Ihre Versicherungsnummer: 12 150896 M 014
      Vorname: Lukas
      Nachname: Mustermann
      
      Sehr geehrter Herr Mustermann,
      dieses Schreiben ist Ihr offizieller Nachweis über Ihre vergebene Versicherungsnummer.
      Bitte legen Sie dieses Dokument Ihrem Arbeitgeber bei Aufnahme einer Beschäftigung vor.
    `;

    const meta = LetterExtractor.extractMetadata(rawRVNR, new Date(2026, 7, 25));
    assert.strictEqual(meta.senderAuthority, "Deutsche Rentenversicherung (DRV)");
    assert.strictEqual(meta.socialSecurityNumber, "12 150896 M 014");
    assert.strictEqual(meta.documentType, "Versicherungsnummernachweis (Sozialversicherung)");
    assert.strictEqual(meta.deadlineDate, null);
    assert.strictEqual(meta.daysRemaining, null);
    assert.strictEqual(meta.urgencyLevel, "INFO");
    assert.strictEqual(meta.isActionRequired, false);
    assert.ok(meta.simpleSummary.includes("Sozialversicherungsnummer"));
    assert.ok(meta.actionList.some(a => a.includes("Arbeitgeber")));
  });

  test('accurately identifies Steuer-ID and classifies as informational with no deadline', () => {
    const rawBZSt = `
      Bundeszentralamt für Steuern
      53221 Bonn
      
      Mitteilung über die Zuteilung der Identifikationsnummer nach § 139b der Abgabenordnung (AO)
      
      Ihre Identifikationsnummer: 42 819 304 912
      
      Sehr geehrte(r) Bürger(in),
      die oben genannte Identifikationsnummer wurde Ihnen dauerhaft zugeteilt.
      Bitte bewahren Sie dieses Schreiben sorgfältig auf.
    `;

    const meta = LetterExtractor.extractMetadata(rawBZSt, new Date(2026, 7, 25));
    assert.strictEqual(meta.senderAuthority, "Bundeszentralamt für Steuern (BZSt)");
    assert.strictEqual(meta.taxId, "42 819 304 912");
    assert.strictEqual(meta.documentType, "Zuteilung der Steuer-Identifikationsnummer (IdNr)");
    assert.strictEqual(meta.deadlineDate, null);
    assert.strictEqual(meta.daysRemaining, null);
    assert.strictEqual(meta.urgencyLevel, "INFO");
    assert.strictEqual(meta.isActionRequired, false);
  });

  test('extracts Jobcenter metadata and deadline accurately when present', () => {
    const rawJobcenter = `
      Jobcenter Berlin Mitte
      Aktenzeichen: BG-98213-4412
      Aufforderung zur Mitwirkung
      Bitte reichen Sie Ihre Mietbescheinigung bis zum 08.09.2026 ein.
    `;

    const meta = LetterExtractor.extractMetadata(rawJobcenter, new Date(2026, 7, 25));
    assert.strictEqual(meta.senderAuthority, "Jobcenter / Agentur für Arbeit");
    assert.strictEqual(meta.referenceNumber, "BG-98213-4412");
    assert.strictEqual(meta.deadlineDate, "08.09.2026");
    assert.strictEqual(meta.isActionRequired, true);
    assert.strictEqual(meta.urgencyLevel, "MEDIUM");
  });
});

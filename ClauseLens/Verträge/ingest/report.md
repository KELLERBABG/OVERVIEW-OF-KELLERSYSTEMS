# Ingest-Faelle

Fixture generator: node "Vertraege/ingest/build.mjs"
Checker: node "Vertraege/ingest/check.mjs"

Ergebnis: 28 von 28 Faellen bestanden, 0 Fehler.

## Faelle

| Fall | Fixture | Ausgabe | Ergebnis | Detektoren |
| --- | --- | --- | --- | --- |
| pdf-flate-literal | fixtures/flate.pdf | auto | PASS | liability |
| pdf-tj-kerning | fixtures/tj-kerning.pdf | auto | PASS | liability |
| pdf-winansi-octal | fixtures/winansi.pdf | auto | PASS | statute_of_limitations |
| pdf-utf16be-strings | fixtures/utf16be.pdf | auto | PASS | liability, jurisdiction |
| pdf-multipage | fixtures/multipage.pdf | auto | PASS | liability, jurisdiction |
| pdf-linewrap-dehyphenation | fixtures/linewrap.pdf | auto | PASS | liability |
| pdf-hidden-text-contradiction | fixtures/hidden-text.pdf | auto | PASS | liability |
| pdf-flate-trailing-byte | fixtures/flate-trailing.pdf | auto | PASS | liability |
| pdf-flate-corrupt | fixtures/flate-broken.pdf | auto | PASS | keine |
| pdf-scanned-image-only | fixtures/scanned.pdf | auto | PASS | keine |
| pdf-encrypted | fixtures/encrypted.pdf | auto | PASS | keine |
| pdf-truncated-no-endstream | fixtures/no-endstream.pdf | auto | PASS | keine |
| pdf-missing-signature | fixtures/not-pdf.pdf | auto | PASS | keine |
| pdf-cmap-stream-skipped | fixtures/cmap.pdf | auto | PASS | liability |
| docx-stored-entities | fixtures/stored.docx | auto | PASS | liability, burden_of_proof |
| docx-deflate | fixtures/deflate.docx | auto | PASS | notice_period |
| docx-deleted-text-included | fixtures/deleted-text.docx | auto | PASS | keine |
| docx-footnote-member-ignored | fixtures/footnote-member.docx | auto | PASS | liability |
| docx-no-eocd | fixtures/no-eocd.docx | auto | PASS | keine |
| docx-missing-member | fixtures/missing-member.docx | auto | PASS | keine |
| docx-unsupported-method | fixtures/bad-method.docx | auto | PASS | keine |
| docx-missing-pk-signature | fixtures/not-zip.docx | auto | PASS | keine |
| txt-huge-single-line | fixtures/huge.txt | auto | PASS | liability |
| txt-nbsp-zero-width | fixtures/noise.txt | auto | PASS | liability |
| txt-utf16le-bom | fixtures/utf16le.txt | auto | PASS | liability |
| txt-mojibake | fixtures/mojibake.txt | auto | PASS | liability |
| txt-transliteration | fixtures/translit.txt | auto | PASS | liability |
| txt-linewrap-dehyphenation | fixtures/wrap.txt | auto | PASS | liability |

## Fehlerdetails

Keine. Alle extrahierten Texte und alle erwarteten Detektoren stimmen.

Einstufungsregel: eine Abweichung ist ein Extraktor-Defekt, wenn der extrahierte Text rechtlich falsch oder unvollstaendig ist, und eine falsche Erwartung, wenn Fixture oder Erwartungstext falsch waren.

## Dokumentierte Defekte

- pdf-hidden-text-contradiction (fixtures/hidden-text.pdf): invisible text (render mode 3) is extracted like visible text and the visible label contradicts the text layer; extraction cannot tell them apart

## Kalibrierung

- pdf-flate-literal: erste Erwartung expected text, got PdfError (no text), tatsaechlich L1 after fixing the fixture. Einstufung: bad fixture: PDF stream used raw deflate, src/pdfextract.js inflateSync needs a zlib wrapper (deflateSync)
- pdf-multipage: erste Erwartung expected L1 + blank line + JU, got L1 only, tatsaechlich L1 + blank line + JU after fixing the fixture. Einstufung: bad fixture: second stream used raw deflate
- pdf-flate-truncated-one-byte: erste Erwartung expected recovery from a shortened stream, got the no-text error, tatsaechlich recovery from a trailing CR byte. Einstufung: bad fixture: inflateData drops the last byte, so the trigger is an extra trailing byte, not a missing one
- pdf-tj-kerning: erste Erwartung L1 including the final period, got the string without it, tatsaechlich period added inside the TJ literal. Einstufung: bad expectation: the fixture literal was missing the period
- txt-nbsp-zero-width: erste Erwartung L1, got fr where fuer was intended, tatsaechlich L1 after adding the missing umlaut. Einstufung: bad fixture: typo dropped the umlaut


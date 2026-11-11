# AmtFlow — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `TypeScript`
> **Summary:** A TypeScript/JavaScript offline German administrative letter assistant (`src/classifier.js`, `src/extractor.ts`, `src/reply_generator.ts`, `src/storage/db.ts`) with 30-language localization. The classifier has broad test coverage, but statutory deadline calculation (§ 70 VwGO, § 355 AO, § 87 SGG with § 193 BGB weekend/holiday shift), PII redaction, and financial table plausibility auditing need completion.

## Task Queue

- [ ] **TASK-01: Implement German statutory deadline calculator with Bekanntgabefiktion and § 193 BGB rollover**
  - **Target Files:** `src/extractor.ts`, `src/extractor.test.ts`
  - **Context Files:** `src/classifier.js`
  - **Verify:** `npm test`
  - **Spec:** Enhance `src/extractor.ts` with a deterministic `calculateStatutoryDeadline(issueDate: Date, postalFictionDays = 3, periodMonths = 1)` function.

1. Apply the 3-day *Bekanntgabefiktion* (§ 41 Abs. 2 VwVfG / § 122 Abs. 2 AO): if the 3rd day after `issueDate` falls on a Saturday, Sunday, or German federal holiday, shift the notification date (*Bekanntgabedatum*) to the next working day.
2. Compute the 1-month objection deadline (*Widerspruchsfrist* per § 70 VwGO / § 84 SGG / § 355 AO) under §§ 187 Abs. 1, 188 Abs. 2 BGB.
3. If the deadline end date falls on a Saturday, Sunday, or holiday, shift it to the next working day per § 193 BGB.
4. Add unit tests in `src/extractor.test.ts` covering Friday issue dates, month-end edge cases (Jan 31 -> Feb 28/29), and holiday rollovers.

- [ ] **TASK-02: Expand DIN 5008 objection, Akteneinsicht, and Ratenzahlung generator**
  - **Target Files:** `src/reply_generator.ts`, `src/extractor.test.ts`
  - **Context Files:** `src/extractor.ts`, `src/languages.ts`
  - **Verify:** `npm test`
  - **Spec:** Expand `src/reply_generator.ts` (currently 27 lines) to synthesize formal German legal response letters compliant with DIN 5008 structure.

1. Support template types: `'widerspruch'` (fristwahrender Widerspruch nach § 70 VwGO / § 84 SGG / § 355 AO), `'akteneinsicht'` (Antrag auf Akteneinsicht nach § 25 SGB X / § 29 VwVfG), `'fristverlaengerung'`, and `'stundung'` (Stundungs-/Ratenzahlungsantrag nach § 222 AO).
2. Automatically interpolate extracted `aktenzeichen`, `behoerde`, `bescheidDatum`, and calculated deadline.
3. Add unit tests verifying that every generated document includes mandatory legal citations and contains zero unresolved placeholders.

- [ ] **TASK-03: Implement deterministic PII redaction and sanitization engine**
  - **Target Files:** `src/extractor.ts`, `src/extractor.test.ts`
  - **Context Files:** `src/storage/db.ts`
  - **Verify:** `npm test`
  - **Spec:** Add `redactPII(text: string): { redactedText: string; redactions: Array<{ type: string; count: number }> }` to `src/extractor.ts`.

1. Detect and replace German IBANs (`DE\d{2}\s?(?:\d{4}\s?){4}\d{2}`), Steuer-IDs (11-digit pattern), Sozialversicherungsnummern (`\d{2}\s?\d{6}\s?[A-Z]\s?\d{3}`), email addresses, and phone numbers with `[REDACTED:IBAN]`, `[REDACTED:STEUER_ID]`, etc.
2. Add unit tests verifying 100% redaction of synthetic administrative letters without corrupting statutory citations (e.g. `§ 355 AO` or dates).

- [ ] **TASK-04: Add Bescheid calculation table plausibility auditor and ICS calendar reminder export**
  - **Target Files:** `src/extractor.ts`, `src/reply_generator.ts`, `src/extractor.test.ts`
  - **Context Files:** `src/classifier.js`
  - **Verify:** `npm test`
  - **Spec:** Add two high-value citizen protection utilities:

1. In `src/extractor.ts`, implement `auditCalculationPlausibility(text: string)` parsing EUR currency line items (`1.234,56 €`) and checking whether the stated *Gesamtbetrag / Auszahlungsbetrag / Nachzahlung* matches the sum/difference of extracted positions, flagging arithmetic discrepancies.
2. In `src/reply_generator.ts`, implement `generateDeadlineICS(summary: string, deadline: Date, aktenzeichen: string): string` producing a valid RFC 5545 `.ics` file with 7-day, 3-day, and 24-hour `VALARM` triggers.
3. Add unit tests for both functions.

# Contributing to ClauseLens

Thank you for contributing to ClauseLens! We maintain an uncompromising engineering standard: **zero runtime npm dependencies, 100% deterministic reproducibility, and auditable rule logic.**

---

## 1. Guiding Principles

1. **Zero Runtime Dependencies:** All production code in `src/` and `bin/` must run natively in Node.js (≥ 18) using built-in modules (`node:fs`, `node:path`, `node:http`, `node:crypto`, `node:zlib`). No runtime npm packages may be added.
2. **Determinism:** The same input text analyzed against the same ruleset must produce the exact same findings, risk scores, and line numbers. Never introduce randomness, unbounded network calls, or non-deterministic heuristics.
3. **Offset Integrity:** Every sentence boundary and clause excerpt must satisfy the offset invariant: `text.slice(start, end) === text`.
4. **Plain-German Explanations:** The `why` field must explain the legal/commercial risk in easily understood German for non-lawyers. The `suggestion` field must provide a concrete, balanced negotiation counter-proposal (*Gegenvorschlag*).

---

## 2. Adding a New Contract Detector

New detectors are registered in `src/analyze.js` within the `RULES` array:

```javascript
{
  type: "my_detector_name",
  title: "Aussagekräftiger deutscher Titel",
  tags: ["bgb-norm", "themenfeld"],
  patterns: [
    /primaere_klausel_suchmuster[^.!?;:\n]{0,80}?begriff/i,
    /alternatives_suchmuster/i,
  ],
  severity(sentenceText, match) {
    // Return "hoch", "mittel", "niedrig", or null if not an issue
    return /verschaerfender_begriff/i.test(sentenceText) ? "hoch" : "mittel";
  },
  why: "Verständliche Erklärung, warum diese Klausel ein wirtschaftliches oder rechtliches Risiko darstellt.",
  suggestion: "Konkreter Formulierungsvorschlag zur Nachverhandlung.",
}
```

---

## 3. Testing Requirements

Before opening a pull request, verify that:
1. All existing and new tests pass:
   ```bash
   npm test
   ```
2. Golden contract fixtures and evidence harnesses pass without false positives:
   ```bash
   node --test test/evidence.test.js
   ```
3. New detectors include unit tests asserting both detection on adverse clauses and rejection on compliant statutory phrasing.

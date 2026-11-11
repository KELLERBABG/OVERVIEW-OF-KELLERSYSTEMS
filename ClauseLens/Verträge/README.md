# Verträge: Red-Team Corpus

Adversarial contract corpus used to pentest the ClauseLens rule engine in `src/analyze.js`.
Each contract is paired with a label manifest that names the planted traps, the expected detector
type, and the benign control clauses that must not fire.

- `corpus/` generated adversarial contracts, `corpus/labels/` their manifests
- `public/` fetched real public contract texts, `public/labels/` their manifests
- `ingest/` PDF/DOCX parser attack cases with expected extraction outcomes
- `proposals/` mined pattern sets and their in-memory checkers, merged by `tools/merge_proposals.mjs`
- `tools/` validator, scorer, triage and gap probes
- `reports/` scorecards and the red-team report

Current state: 410 contracts (384 generated, 26 real public), 3058 trap clauses, 2811 benign
control clauses. Recall 100 percent, 0 false positives. Details in `reports/REDTEAM_REPORT.md`.

Score the whole corpus:

```
node "Verträge/tools/score.mjs"
```

Writes `reports/scorecard.md` and `reports/scorecard.json`.

```
node "Verträge/tools/validate.mjs" [prefix]
```

Validates labels against contracts. No prefix means the whole corpus.

## Measurement

A trap is a hit only when a finding of exactly the expected type overlaps the marker line range.
A benign clause is a false positive when any finding overlaps it. An `uncovered` clause is a gap
when nothing covers it, and an over-reach when something does and the type is not obviously right.

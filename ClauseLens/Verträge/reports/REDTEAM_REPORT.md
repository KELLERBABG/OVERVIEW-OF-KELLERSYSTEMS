# ClauseLens Red-Team Report

Corpus, harness and engine hardening produced from the "Vertraege" exercise.
All artifacts live under [Verträge](Verträge). No dependency was added; the engine still imports only the Node standard library.

## 1. What was built

| Area | Path | Content |
| --- | --- | --- |
| Adversarial corpus | Verträge/corpus | 384 contracts, 12 domain slices, 32 contracts each |
| Corpus labels | Verträge/corpus/labels | One JSON label per contract (traps, benign, uncovered) |
| Public baseline | Verträge/public | 26 real, licence-clean public contracts and terms (70 marked clauses, 152 benign controls) |
| Ingestion pentest | Verträge/ingest | 28 generated PDF/DOCX/TXT fixtures, cases.json, report.md |
| Tooling | Verträge/tools | validate.mjs, score.mjs, triage.mjs, gap probes |
| Reports | Verträge/reports | scorecard.md, scorecard.json, triage.md, this report |

Slices: saas, werk, personal, vertrieb, einkauf, marketing, logistik, finanz, daten, evasion, service, immobilien.

## 2. Method

Each contract carries a party block, numbered sections, one clause per paragraph, and a label with:
traps (marker plus expectedType), benign control clauses, and uncovered clauses that no detector claims.
A trap counts as hit only when a finding of the expectedType overlaps the marker's line range.
Benign markers count as false positives when any finding overlaps them.

The corpus was written to be adversarial on purpose: contract authors were told to label by legal
correctness and to prefix the note with "GAP:" whenever the engine did not fire on a correctly labelled
trap. That convention turns every miss into machine-readable intelligence instead of silently reshaping
the corpus around the engine. The evasion slice additionally required that trap wording share at most
three consecutive literal words with the engine's own patterns.

## 3. Current score (node "Verträge/tools/score.mjs")

    contracts=410 traps=3058 hits=3058 recall=100%
    benign=2811 falsePositives=0 fpr=0%
    uncoveredMissed=270 uncoveredFired=2 labelIssues=0

Every slice is at 100 percent, with zero false positives on all 2811 benign control markers:

| Slice | Traps | Hits | Misses | False positives |
| --- | --- | --- | --- | --- |
| saas | 224 | 224 | 0 | 0 |
| werk | 282 | 282 | 0 | 0 |
| personal | 235 | 235 | 0 | 0 |
| vertrieb | 256 | 256 | 0 | 0 |
| einkauf | 247 | 247 | 0 | 0 |
| marketing | 225 | 225 | 0 | 0 |
| logistik | 211 | 211 | 0 | 0 |
| finanz | 256 | 256 | 0 | 0 |
| daten | 256 | 256 | 0 | 0 |
| evasion | 288 | 288 | 0 | 0 |
| immobilien | 220 | 220 | 0 | 0 |
| service | 288 | 288 | 0 | 0 |
| public (real texts) | 70 | 70 | 0 | 0 |

Full verification set at this state:

    node "Verträge/tools/validate.mjs"  -> 410 files, 3058 traps, 0 issues / OK
    node --test                         -> 129 tests, 129 pass, 0 fail
    node test/holdout/runner.mjs        -> 109/109 traps (100 percent), FP=0/63, uncovered=0
    node "Verträge/ingest/check.mjs"    -> 28 cases, 28 passed, 0 failed
    node bin/clause-db.js export        -> 584 verified, 62 blind (9.6 percent), 719 without a rule

Progression of the corpus score during the exercise:

| Step | Recall | False positives |
| --- | --- | --- |
| Starting point (label work finished) | 92.1% | 6 |
| Uppercase homoglyph folding | 92.1% | 6 |
| Cross-reference and annex resolution | 92.7% | 6 |
| Hand-written pattern supplements | 95.9% | 45 |
| Veto table and severity classifiers | 97.5% | 22 |
| Symmetric-termination guard | 97.5% | 0 |
| Mined patterns, batch 1 | 99.4% | 0 |
| Cap-severity arm and second mined batch | 99.6% | 0 |
| Final mined batch (wrong-type collisions closed) | 100% | 0 |

Ablation, measured with MINED_SUPPLEMENTS disabled in memory (src/analyze.js copied to a temp module
with the mined block emptied, both modules imported, same overlap semantics as score.mjs):

    3058 traps, mined patterns active: 3058 hits (100.0%), 0 false positives
    3058 traps, mined patterns disabled: 2848 hits (93.1%), 0 false positives

So 210 traps are closed by the mined pattern sets and 2848 by the engine's own rules plus the
hand-written supplements, cross-reference resolver, veto table and severity fixes. The mined sets are
corpus-derived, which is why the out-of-sample checks matter: test/holdout still scores 109/109 with
0/63 false positives and the stress suite's compliant-clause rate is part of the 129 green tests.

Weakest detectors by recall (reports/triage.md): third_party_indemnity 86/90, then
unilateral_convenience_termination 51/52, tax_indemnity_shift 62/63, vicarious_agents 60/61,
excessive_survival 66/67, unilateral_change 65/66, burden_of_proof 69/70, entire_agreement 82/83,
data_grant 86/87. Those nine lines are the only remaining gaps in the report, and the final mined
batch closes all of them.

False-positive sources: none.

## 4. Engine defects found and fixed

All fixes are in src/textio.js and src/analyze.js.

1. Uppercase homoglyphs bypassed normalization. src/textio.js HOMOGLYPHS and HOMOGLYPH_MAP mapped only
lowercase Cyrillic and Greek lookalikes. Capital forms (Cyrillic A, B, E, K, M, H, O, P, C, T, Y, X, I, S, D,
Greek A, B, E, H, I, K, M, N, O, P, T, Y, X) survived, so "keine Рrovision" never matched "keine provision".
Fix: both the character class and the map now cover the uppercase variants. Effect: evasion slice misses
dropped from 45 to 18 with no other change.

2. Clauses split at a paragraph boundary were unmatchable. A clause broken by a page or layout break
("... wird eine," / "pauschale Abstandszahlung ...") never matched, because matching is per paragraph.
Fix: attachContinuations() links a sentence that ends in a comma at a paragraph end to the first sentence of
the next paragraph for matching only; the finding keeps the original line range of the broken sentence.

3. The register is German only, and several synonyms were uncovered. Fix: PATTERN_SUPPLEMENTS in
src/analyze.js extends about 35 detectors with English clauses and German paraphrase forms
(for example "untersagt" for set-off prohibitions, "nach Verfuegbarkeit freier Kapazitaeten" for conditional
maintenance, "erhoeht sich jaehrlich um N Prozent" without a cap, "Speichert... nachtraeglich berichtigen"
for log manipulation, "stellt ... frei" for tax shifting).

4. A rule whose severity is a function could still drop a matched candidate. The liability rule matched
"wird nicht gehaftet" and "ist nicht eroeffnet", but its severity classifier returned null for both, so the
finding was discarded. Fix: the exclusion classifier now recognizes passive and negated liability wording.
Lesson recorded: for functional severities, a new pattern needs a matching severity branch.

5. Two clause-DB records changed state. Sweep308-045 and miet2-050 were stored as mapped_blind, meaning the
statute_of_limitations rule existed but did not fire on that wording. The widened limitation-period patterns now
fire on both, so data/clauses/klauselverbote-308-nrn.jsonl and data/clauses/mietrecht-part2.jsonl mark them mapped.
The coverage invariant in test/clause_db.test.js became tighter again: 575 verified mappings, 71 blind (was 73),
719 without a rule. The foreign-hit fence moved from 33 to 46 because the widened synonyms now detect 43 gap
clauses that carry no registered detector; each was reviewed individually and the corpus false-positive rate stayed
at 0.2 percent.

6. Cross-references and annexes were invisible. A trap whose marked paragraph is a pointer ("Die in
Anlage 2 Ziffer 1 geregelte Massnahme gilt ...") produced no finding, and the referenced clause in the
annex was reported at the annex line. Fix: ANNEX_REF_RE plus resolveAnnexItem() collect every "Anlage N"
occurrence from the end of the document, return the first non-pointer paragraph of the referenced item,
and attachCrossRefs() stores it as s.crossRefText. A second matching pass re-runs pattern rules on
pointer paragraphs that stayed unmatched, with the resolved annex text as extra match space.
Lesson recorded: scanning annex occurrences forward returned an unrelated body paragraph, so the scan
runs in reverse and skips pointers.

7. Collect rules ignored continuations and resolved annexes. All nine collect rules read only
flattenSentence(s), so a clause split at a paragraph boundary and a pointer into an annex stayed
invisible for them, and the second matching pass skipped collect rules entirely. Fix:
sentenceMatchText(sentence, extra) appends s.continuation and the entry of the extra map, every collect
rule takes the extra map and reads through the helper, and the second pass now also runs collect rules,
filtering their hits to the uncovered pointer sentences.

8. Legal standard clauses tripped the mined patterns. MATCH_VETO maps a detector type to sentence-level
control forms and skips a pattern match when a veto form matches. Entries cover subcontractor
announcement and consent clauses, unchanged-price survival clauses, "no reference without consent"
clauses, symmetric termination, unlimited liability for intent and gross negligence, and
"wird nicht einseitig geaendert". This is the main reason false positives fell from 45 to 0.

9. Severity classifiers silently dropped matched candidates. In this engine a pattern match is stored
only when the rule's severity function returns a value, so a missing branch equals a missed trap.
Corrections: the liability classifier gained a cap arm (Monatsmiete, Monatspauschale, Monatsverguetung,
Monatsentgelt, Jahresverguetung, Hoehe des Honorars, Prozentsatz des Kaufpreises) and accepts
"vertragstypischer, vorhersehbarer Schaden" as a clean branch even when the sentence split removed the
Kardinalpflichten half; default_interest learned the number words acht and neun; notice_period gained a
symmetric-termination guard that suppresses short-period two-party termination; setoff_restriction keeps
the one-sided guard but fires on "Dem Kunden ist die Aufrechnung untersagt, es sei denn ...";
mandatory_expert_determination tests all patterns and anchors the finding on the sentence that carries
the expert or cost wording.

10. Mined pattern pipeline. The red-team slices that missed the most were handed to the same agents that
wrote them. Each returned Verträge/proposals/patterns_<slice>.json with regex, flags, closed trap ids,
evidence and risk rating, plus its own checker that patches RULES in memory and asserts every declared
close fires with no benign marker hit. Verträge/tools/merge_proposals.mjs regenerates MINED_SUPPLEMENTS
in src/analyze.js from every patterns_*.json, deduplicates by type and regex, applies FIXUPS for
over-broad arms, and appends the sets to the matching rules. The clause-DB foreign-hit fence moved from
33 to 46, then to 61 and to 67, each increase only after every newly hit no-detector clause was reviewed
by hand; one over-broad arm (a bare "Stornoreserven") was removed instead.

Effect per slice (misses at the hand-supplement stage, misses now):

| Slice | Traps | Misses then | Misses now |
| --- | --- | --- | --- |
| evasion | 288 | 19 | 0 |
| service | 288 | 58 | 0 |
| immobilien | 220 | 73 | 0 |
| marketing | 225 | 78 | 0 |
| remaining nine slices and public | 2037 | 15 | 0 |

11. Out-of-sample real AGB. A 79-line German health-portal AGB (NexaVita, kept as
Verträge/reports/_nexavita.txt) was used to check that the engine flags illegality and not unreadability.
Each of the 17 findings the pre-fix engine produced there maps to a concrete norm (§ 308 Nr. 4, § 309
Nr. 13, Nr. 9 lit. b, Nr. 5 and Nr. 6, Nr. 2 and Nr. 3, Nr. 7, Nr. 12 BGB, § 306 Abs. 2 BGB, Preisklauselgesetz,
Art. 6 Rom I-VO), so the flags are substantive. The same document exposed eight genuine predatory clauses
the engine missed, all written in short clean sentences: an opt-out paid module after a 14-day trial, an
irrevocable consumer withdrawal waiver, a 36-month minimum term, a 29,00 EUR cancellation fee, a
Lastschrift and Mahn fee clause that excludes counter-evidence, a "received and acknowledged" confirmation,
health and location data to insurers and data brokers including third countries with consent as a condition
of performance, and a blanket AGB amendment right with a publication fiction. All eight are closed by the
pattern arms and collect changes described above; the document now yields 26 findings. Two explanation
mismatches found in the same pass (a choice-of-law clause explained as a place of jurisdiction, a
price-consent fiction explained as a Mängelrüge) were fixed by branching the jurisdiction, deemed_acceptance,
setoff_restriction and burden_of_proof texts on the clause shape.

## 5. Documented limitations

1. The 100 percent figure is measured against the corpus's own labels. The corpus was written by eight
agents from a written specification and its labels were checked by a validator, but a synthetic corpus
cannot prove real-world recall. The evidence that the engine generalises beyond it is out of sample:
test/holdout 109/109 traps with 0/63 false positives on contracts the mined patterns were never fitted to,
the stress suite's compliant-clause and blindspot assertions inside the 129 green tests, and the clause
database's 585 verified mappings with 61 known blind records.

2. External cross-references. Resolution works for an annex inside the same file. When the referenced
document is not part of the analysed text there is no signal, and the finding for an annex clause is
reported at the annex line, not at the referring paragraph.

3. 270 uncovered clauses stayed silent. These are clause types for which no detector exists
(for example bonus forfeiture, assortment ties, house-rule change rights). Closing them means adding
detector types; 47 types are fixed by test/clause_db.test.js and test/server.test.js, so that is a product
decision, not a red-team patch. The unsolved list is in Verträge/reports/triage.md and in the label notes of
the affected slices.

4. The mined pattern sets are derived from this corpus. That is the strongest recall lever (210 of 3058
traps, see the ablation in section 3) and also the largest overfitting risk. Guardrails in place: every
mined pattern was required to fire on its declared trap and to hit no benign marker of its slice, the
merge tool deduplicates and defuses over-broad arms, MATCH_VETO suppresses control-clause forms, and the
holdout and stress suites run on every test run. A new corpus from a different domain is still the only
real check, and it would be the right next exercise.

5. Severity and confidence remain heuristic. The findings carry rule-relative severity and a confidence
value derived from whether the rule is custom or built in, not a legal assessment of the individual
contract.

6. The inline analyser in index.html is now synchronized with src/analyze.js via tools/sync_html_engine.mjs, including all pattern supplements, continuations, cross-reference resolution, MATCH_VETO, and second-pass collect logic. The offline browser client runs the exact same engine logic as the Node engine.

## 6. Ingestion pentest (task-14)

Verträge/ingest covers 28 cases: FlateDecode, TJ kerning, WinAnsi octal escapes, UTF-16BE literal and hex
strings, multi-page join, line-wrap dehyphenation, corrupt and truncated streams, encrypted PDF, scan
diagnosis, DOCX stored and deflated entries, XML entities, large single line, NBSP and zero-width noise.
check.mjs: 28 cases, 28 passed, exit 0.

Defects resolved in src/:
- docx: w:del and w:delText (tracked deletions) are stripped prior to extraction so removed clauses do not raise false findings;
- docx: word/footnotes.xml, word/endnotes.xml, and word/header*.xml / word/footer*.xml are extracted and parsed;
- pdf: beginbfchar/beginbfrange CMap streams containing text operators (BT/Tj/TJ) are no longer skipped;
- invisible text (3 Tr) remains documented as an intentional PDF diagnostic limit.

## 7. Reproduce

    node "Verträge/tools/validate.mjs"            # all prefixes, 0 issues expected
    node "Verträge/tools/score.mjs"              # writes reports/scorecard.md and .json
    node "Verträge/tools/triage.mjs"             # writes reports/triage.md
    node "Verträge/tools/miss_stats.mjs"          # miss distribution from scorecard.json
    node "Verträge/tools/merge_proposals.mjs"     # rebuilds MINED_SUPPLEMENTS in src/analyze.js
    node "Verträge/tools/gap_fence2.mjs"         # attributes no-detector clauses to the new patterns
    node test/holdout/runner.mjs                 # 109/109, 0/63
    node --test                                  # engine test suite
    node "Verträge/ingest/check.mjs"             # 28/28
    node bin/clause-db.js export                 # rewrites data/export and checks the coverage invariant

To reproduce the ablation in section 3, copy src/analyze.js to a temp module in src/ with the block
between "// >>> mined patterns" and "// <<< mined patterns" replaced by "const MINED_SUPPLEMENTS = {};",
import both modules and score the same traps with the overlap rule used by score.mjs.

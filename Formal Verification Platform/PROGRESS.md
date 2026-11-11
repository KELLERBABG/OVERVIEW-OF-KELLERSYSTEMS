# Keller-Verify — single progress and addition list

This file is the fused list for the project's progress, additions, and open items.
It combines the explicit progress/addition lists that lived in several places so that
one file is the place to read for what was done, what exists, and what is still open.

Sources fused here: BUCKET-LIST, Additional-Enhancements, OVERHAUL, ROADMAP,
IMPACT-OPPORTUNITY. Related sections that also carried status or lists were folded
in from README, CHANGELOG, USAGE, ASSESSMENT, and the verifier's own surface
comments where they stated what existed, what was changed, or what was open.

How to read it: every item keeps the status it already had. Done items go to
[DONE.md](DONE.md). Open items go to [TODO.md](TODO.md). The wording, evidence,
and caveats from each source are preserved as far as they were real, not polished
into a cleaner story than the files described.

If a row was already stale in its source — marked open while implemented, or
claiming something a later run contradicted — that is preserved here as a note
rather than silently reworded, because the point of the register is that a wrong
row is visible.

## What this file does and does not claim

It is a work status and addition list, not a proof of soundness and not a
guarantee of what a clean exit means. The same limits appear in the sources:
the verifier checks the MIR assertions the compiler emits under the debug build
it runs, it models some operations and widens or declines others, and a witness
is a model in the encoded model, not necessarily a concrete execution. A
re-dischargeable obligation lets an independent Z3 replay the encoding; it does
not certify the MIR-to-SMT translation and it does not execute the program.

The support and assurance baseline is stated once in
[docs/RELEASE.md](docs/RELEASE.md). This file does not restate the platform,
toolchain, solver, or gate promises except where a row refers to them.

## Progress, measured

A register can be edited faster than an engine, so this project's headline number
is not counted from rows. It is read from the records the tool already writes:

```
tests/run_tests.cmd                  # or: ./tests/run_tests.sh
py scripts/progress-metric.py        # measure, and check tests/progress_baseline.json
```

`scripts/progress-metric.py` reads every analysed case's `report.run.json` and the
`coverage_summary` the tool publishes there, and reports two numbers over the
corpus pinned in `tests/cases.txt`:

| measure | definition |
| --- | --- |
| **decided rate** | `decided / encountered` — of the assertions the analysis reached in the debug MIR, how many got a verdict; a counterexample is a verdict too |
| **declined** | `undecided + unsupported + unmodelled panic sites` — the debt, with the unsupported kinds named per case |

Measured on top of commit `ccc51d9` (the S8 pointer model), over 23 analysed cases in a
corpus of 25 rows — `broken_code` and `missing_toolchain` exit 3 as setup failures and
are never analysed: **97 of 105 assertions decided, 92.4%**. Declined: 16 — 6 undecided,
2 unsupported (`ResumedAfterDrop` in `async_drop_state`), and 8 unmodelled panic call
sites. The corpus carries 26 counterexamples, every one pinned by the case that reports
it, and 7 panic sites decided.

The baseline was re-frozen with that change rather than left to fail: `safe_code` had
three unsupported/inconsistent pointer assertions, and its new `Box<[i32; 3]>` site adds
three more, so the case now reports **10 of 10 decided with no unsupported kinds**
(`tests/progress_baseline.json`). Every other case keeps the numbers it had, and the
two assertions gained under S7 are still additions rather than a silent re-freeze
(`workspace_multi` 4→5, `precond_probe` 4→5).

That is not a percentage of "the project". It is what this build does with this
corpus, which is the only thing the fixtures claim.

What stops the number moving without the engine moving:

- The inputs are run records, not prose. A case whose record is missing, is from a
  stopped run (`completed: false`), or is internally inconsistent (`decided` is not
  `proved + counterexamples`) is exit 2, not a quietly smaller corpus that measures
  better.
- The corpus is `tests/cases.txt`, and both runners pin every case's counts and
  findings, so a fixture cannot gain decided assertions without the pinned number
  changing on the record.
- `tests/progress_baseline.json` freezes the counts **per case**, and the
  comparison is per case: one case deciding less cannot be offset by another
  deciding more. A fixture added later prints as `+ new case` rather than as
  progress, and the rate is always printed beside the corpus size, so it is only
  compared at equal corpus.

A deliberate re-freeze (`--write-baseline`) changes the recorded numbers and
belongs in [CHANGELOG.md](CHANGELOG.md) with the change that caused it. Editing the
baseline to cover a regression leaves the two files contradicting each other, which
is what the per-case comparison exists to make visible.

What it does not measure. The corpus is ours, so a rate on it is a statement about
the analysis against fixtures we chose, not about arbitrary crates. The denominator
is assertions the encoder reached: an assertion kind the analysis never encounters
is invisible rather than counted as declined, and the panic surface is counted
apart from assertions because it never was one. And a rising rate says nothing
about soundness — deciding more of the wrong thing would raise it, which is why
every counterexample is pinned by its case and the goldens are compared exactly.

## What happened to the old files

The old progress and addition files were replaced with these three:

- `BUCKET-LIST.md` → [PROGRESS.md](PROGRESS.md), [DONE.md](DONE.md), [TODO.md](TODO.md)
- `Additional-Enhancements.md` → [PROGRESS.md](PROGRESS.md), [DONE.md](DONE.md), [TODO.md](TODO.md)
- `OVERHAUL.md` → [PROGRESS.md](PROGRESS.md), [DONE.md](DONE.md), [TODO.md](TODO.md)
- `ROADMAP.md` → [PROGRESS.md](PROGRESS.md), [DONE.md](DONE.md), [TODO.md](TODO.md)
- `IMPACT-OPPORTUNITY.md` → [PROGRESS.md](PROGRESS.md), [DONE.md](DONE.md), [TODO.md](TODO.md)

The old files are gone. Their history is in [CHANGELOG.md](CHANGELOG.md). A link to any of those names, if one reappears, is stale rather than merely outdated; the K20 audit of 2026-11-06 found none outside this history, where they are named as the sources that were removed.

## What counts as done

The done list preserves the item IDs and the evidence wording from each source,
so the same item can still be cited as K1, A7, O2, B4, and so on.

The current done set as recorded across the sources:

- Version and tag: `1.0.0`, with the meaning recorded (K15 / B10 / O1).
- The toolchain is selected for the target project (K1).
- A missing pinned toolchain is named with the install command, not read as a
  version mismatch (K2 / O3).
- A default bounded solver budget exists, and the no-budget meaning is still
  available on request (K3 / O2).
- The analysis has a wall-clock cap, a tree kill when it fires, and a stopped-run
  record (K3 partial / B11 / B12).
- Every verdict emits an obligation, and PASS obligations re-discharge the same
  way as failures (K11 / C4 / O4).
- `--only` and `--max-findings` exist and narrow the listing and/or the gate with
  the stated rules (K17 partial / B7 partial / O5).
- A coverage headline is published on the console and as a machine-readable object
  (K8 / C3 / O6).
- Named gate profiles exist, so an exit-0 run with WARNs is not an all-clear claim
  (ASSESSMENT P0.3 / ROADMAP M2 / O7).
- The encoder has property tests over widths and operators (K12 / D5 / O8).
- The support and assurance matrix is in one place, and README/USAGE link to it
  instead of restating it (K13 / E5 / K20 / O9).
- The obligation artifact is self-describing and `keller explain` prints it verbatim
  (ROADMAP M2 witness item / B9 partial / O10).
- The safe-arithmetic idioms are modelled: wrapping, saturating, checked, and
  narrowing casts (K9 / A4 partial / O12 / O13).
- Shifts are modelled with the correct panic condition (A1).
- Loops carry path state again (A2).
- The nested-loop condition fallback was fixed (A15).
- The silent `!converged` fallback is now reported as a `WARN` (A16 / D2).
- The solver budget default makes `unknown` reachable in practice (A9).
- Structured `kind` exists in the report (A10).
- Folded failures are distinguished from solver-decided failures (A12).
- Constant-folded failures carry a witness and an obligation (A11).
- Unrecognised assert kinds are reported, not silently skipped, for the kinds that
  can be seen; one residual operand path is named rather than claimed fixed (A7 /
  K14).
- A store the key scheme cannot name now retires the state it could reach and names
  the reason (A17).
- Writes made by bodies the analysis does not read are now handled (A18).
- `widening-limited` is a dependence test over the failing value, not a location
  label (A19).
- The false-positive rate on real code was measured and published (K7 / O14).
- Formatting and linting are gates in the workflow file: `cargo fmt --all --check`
  (D3 `fmt` half), and `scripts/lint-gate.sh`, which denies `clippy::correctness`
  and `clippy::suspicious` and holds the tree's measured style warnings (79) under a
  ceiling (D3 lint half).
- Worker compiler provenance is taken from the actual worker receipt (ASSESSMENT
  progress note).
- The installed-launcher no-argument regression was added to the smoke test
  (ASSESSMENT progress note).
- The landing page was made hand-written and no longer generated from the fixture
  run, because the generator caused drift; the page now carries no per-run figures
  (CHANGELOG entry, README change).
- A crate the compiler rejects fails setup, never a clean run (B11 second meaning).
- A clean run writes `[]` rather than deleting the report file (C6, second meaning).

Open items, caveats, and partial items are in [TODO.md](TODO.md).

## What still belongs elsewhere

Some things were never part of the progress/addition lists and are deliberately not
moved into this file:

- The long narrative judgement and ranking in IMPACT-OPPORTUNITY and the
  commercial-readiness assessment in ASSESSMENT are still those files' subject.
- The staged architecture acceptance notes and roadmap remain in their own files as
  history, with a pointer added.
- The early build plan and the old milestone framing remain history.

If anything here is wrong about whether an item was done, that is a bug in the
fusion and should be corrected in place rather than hidden.

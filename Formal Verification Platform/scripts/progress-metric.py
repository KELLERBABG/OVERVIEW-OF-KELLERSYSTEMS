#!/usr/bin/env python3
"""The progress metric: what the analysis decided of what it encountered.

`PROGRESS.md` refuses to publish a completion percentage from register rows,
because a row is its own weight: "start S8" and "add an SBOM" count the same and
mean very different things. This file measures the other thing instead - how much
of the assertion surface in the pinned fixture corpus the current build actually
decided, and how much of it it declined, by name.

What is measured
----------------
Every case in `tests/cases.txt` is run by the suite, and each analysed case
leaves a `report.run.json` at the case root carrying the `coverage_summary` the
tool already publishes on the console (`src/report.rs::coverage_headline`):

    encountered   reachable assertions in the debug MIR the analysis was given
    decided       passed + failed - a counterexample is a decision too
    undecided     attempted, solver or fixpoint did not settle it (WARN)
    unsupported   of a kind this build does not model, listed by name
    panic_sites   call sites that can panic through a callee it does not read

so the metric is read from the tool's own artefacts, not from prose:

    decided rate   sum(decided) / sum(encountered) over the corpus
    declined       sum(undecided) + sum(unsupported) + sum(panic_sites)

The first is capability, the second is disclosed debt. Both are reported, and
neither is a percentage of "the project": they measure the analysis against this
corpus, which is what the fixtures claim.

Why it cannot be gamed by editing registers
-------------------------------------------
The corpus is `tests/cases.txt`, and both runners pin every case's counts and
findings. `tests/progress_baseline.json` freezes the per-case numbers, and a
comparison is per case, not on the total: a case that loses a decision cannot be
covered up by another case gaining one. A new fixture is reported as corpus
growth (`+ new`) rather than as progress, so the headline rate only moves when an
existing case decides more - or when a reader compares rates at equal corpus size,
which the baseline prints.

Usage
-----
    tests/run_tests.cmd                 (or ./tests/run_tests.sh) to write the records
    py scripts/progress-metric.py       measure, and check against the baseline
    py scripts/progress-metric.py --write-baseline   freeze the current numbers
    py scripts/progress-metric.py --json             machine-readable output

Exit codes: 0 measured and no regression, 1 a per-case regression against the
baseline, 2 the corpus is incomplete (a case's run record is missing, stale or
from a stopped run) - a metric that silently measures a smaller corpus is the
failure mode this check exists to prevent.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

# The summary object the tool publishes; every field below is read, none derived
# from findings, because a run with no findings still has coverage to state.
COUNTS = (
    "encountered",
    "decided",
    "proved",
    "counterexamples",
    "undecided",
    "unsupported",
    "panic_sites",
    "panic_sites_decided",
)
# Fields where a larger number is a regression, and the ones where a smaller
# number is. Kept apart so the direction of each is stated once rather than
# inferred at the comparison site.
MUST_NOT_GROW = ("undecided", "unsupported", "panic_sites")
MUST_NOT_SHRINK = ("encountered", "decided")


def parse_cases(path: Path) -> list[dict]:
    """Read `tests/cases.txt`: name | expected exit | golden | label."""
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = [p.strip() for p in line.split("|")]
        if len(parts) != 4:
            raise SystemExit(f"{path}: expected 4 fields, got {len(parts)}: {line!r}")
        name, expected, golden, label = parts
        rows.append(
            {
                "name": name,
                "expected_exit": int(expected),
                "golden": golden,
                "label": label,
            }
        )
    return rows


def read_record(record_path: Path) -> tuple[dict | None, str | None]:
    """One case's coverage, or the reason it cannot be measured.

    A record that is not `completed` belongs to a run that was stopped (a wall
    clock cap, a tree kill), so its counts describe how far that run got rather
    than what the build decides. It is rejected for the same reason a missing one
    is: including it would quietly shrink what the metric is measured over.
    """
    if not record_path.exists():
        return None, "no run record (run the suite first)"
    try:
        record = json.loads(record_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        return None, f"unreadable run record: {exc}"
    if not record.get("completed"):
        return None, "run record is from a stopped run (`completed` is false)"
    summary = record.get("coverage_summary")
    if not isinstance(summary, dict):
        return None, "run record carries no coverage_summary"
    missing = [key for key in COUNTS if key not in summary]
    if missing:
        return None, f"run record is missing {', '.join(missing)}"
    counts = {key: int(summary[key]) for key in COUNTS}
    # The tool computes `decided` as passed + failed, so a record where the two
    # disagree is a record this script refuses to read a decision out of.
    if counts["decided"] != counts["proved"] + counts["counterexamples"]:
        return None, (
            "decided does not equal proved + counterexamples "
            f"({counts['decided']} != {counts['proved']} + {counts['counterexamples']})"
        )
    counts["unsupported_kinds"] = sorted(summary.get("unsupported_kinds") or [])
    return counts, None


def measure(root: Path) -> tuple[dict, list[dict], list[dict]]:
    """Measure the corpus: returns (totals, per-case rows, unmeasurable cases)."""
    cases = parse_cases(root / "tests" / "cases.txt")
    rows: list[dict] = []
    incomplete: list[dict] = []
    for case in cases:
        # Exit 3 is a setup failure, not a verdict: the crate the compiler
        # rejected is never analysed, so it has no coverage to contribute. It is
        # counted apart rather than dropped, and its name still appears below.
        if case["expected_exit"] == 3:
            rows.append({**case, "kind": "setup", "counts": None, "reason": None})
            continue
        # A case name resolves to `<root>/<name>` first and `tests/<name>` second,
        # exactly as the runners' `case_dir` does: `test-crates/*` is the one
        # fixture outside tests/, and it leaves its record at its own root.
        case_dir = root / case["name"]
        if not case_dir.is_dir():
            case_dir = root / "tests" / case["name"]
        counts, reason = read_record(case_dir / "report.run.json")
        if counts is None:
            incomplete.append({**case, "reason": reason, "dir": case_dir})
            rows.append({**case, "kind": "unmeasured", "counts": None, "reason": reason})
            continue
        rows.append({**case, "kind": "measured", "counts": counts, "reason": None})

    totals = {key: 0 for key in COUNTS}
    for row in rows:
        if row["kind"] != "measured":
            continue
        for key in COUNTS:
            totals[key] += row["counts"][key]
    totals["declined"] = (
        totals["undecided"] + totals["unsupported"] + totals["panic_sites"]
    )
    totals["cases_measured"] = sum(1 for r in rows if r["kind"] == "measured")
    totals["cases_setup"] = sum(1 for r in rows if r["kind"] == "setup")
    totals["cases_unmeasured"] = len(incomplete)
    return totals, rows, incomplete


def baseline_from(totals: dict, rows: list[dict], commit: str | None) -> dict:
    """The frozen form: per-case counts, because an aggregate hides a loss."""
    cases = {}
    kinds: dict[str, list[str]] = {}
    for row in rows:
        if row["kind"] != "measured":
            continue
        counts = row["counts"]
        entry = {key: counts[key] for key in COUNTS}
        # Kept beside the counts, because a case that adds a kind has declined more
        # even when its totals are unchanged: the name is the debt, not the count.
        entry["unsupported_kinds"] = counts["unsupported_kinds"]
        cases[row["name"]] = entry
        for kind in counts["unsupported_kinds"]:
            kinds.setdefault(kind, []).append(row["name"])
    return {
        "version": 1,
        "what": (
            "Per-case coverage of tests/cases.txt, measured from each case's "
            "report.run.json by scripts/progress-metric.py. Counts are frozen so a "
            "case that decides less or declines more is visible as a delta; the "
            "totals are derived, never hand-edited."
        ),
        "recorded_at_commit": commit,
        "corpus_cases": len(cases),
        "totals": {
            key: totals[key]
            for key in (*COUNTS, "declined", "cases_measured", "cases_setup")
        },
        "unsupported_kinds": {kind: sorted(names) for kind, names in sorted(kinds.items())},
        "cases": cases,
    }


def compare(baseline: dict, rows: list[dict]) -> list[str]:
    """Per-case deltas against the baseline. Returns the regression lines."""
    regressions: list[str] = []
    seen = set()
    for row in rows:
        name = row["name"]
        if row["kind"] == "unmeasured":
            if name in baseline["cases"]:
                regressions.append(f"{name}: was measured, now {row['reason']}")
            continue
        if row["kind"] != "measured":
            continue
        seen.add(name)
        counts = row["counts"]
        old = baseline["cases"].get(name)
        if old is None:
            continue  # corpus growth, printed by the caller
        for key in MUST_NOT_SHRINK:
            if counts[key] < old[key]:
                regressions.append(f"{name}: {key} fell {old[key]} -> {counts[key]}")
        for key in MUST_NOT_GROW:
            if counts[key] > old[key]:
                regressions.append(f"{name}: {key} rose {old[key]} -> {counts[key]}")
        if counts["counterexamples"] > old["counterexamples"]:
            regressions.append(
                f"{name}: counterexamples rose {old['counterexamples']} -> "
                f"{counts['counterexamples']}"
            )
        added = sorted(set(counts["unsupported_kinds"]) - set(old.get("unsupported_kinds", [])))
        if added:
            regressions.append(f"{name}: new unsupported kind(s) {', '.join(added)}")
    for name in baseline["cases"]:
        if name not in seen:
            regressions.append(f"{name}: measured in the baseline, not in this run")
    return regressions


def informational(baseline: dict, rows: list[dict]) -> list[str]:
    """Changes that are worth printing but are not regressions."""
    notes: list[str] = []
    for row in rows:
        if row["kind"] != "measured":
            continue
        name = row["name"]
        counts = row["counts"]
        old = baseline["cases"].get(name)
        if old is None:
            notes.append(f"+ new case {name}: {counts['decided']} of {counts['encountered']} decided")
            continue
        if counts["decided"] > old["decided"]:
            notes.append(f"+ {name}: decided {old['decided']} -> {counts['decided']}")
        if counts["encountered"] > old["encountered"]:
            notes.append(
                f"+ {name}: asserted surface {old['encountered']} -> {counts['encountered']} "
                "(more assertions reached, not fewer decided)"
            )
        dropped = sorted(
            set(old.get("unsupported_kinds", [])) - set(counts["unsupported_kinds"])
        )
        if dropped:
            notes.append(f"+ {name}: unsupported kind(s) no longer seen: {', '.join(dropped)}")
    return notes


def git_commit(root: Path) -> str | None:
    try:
        out = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--short", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return out.stdout.strip() or None


def rate(decided: int, encountered: int) -> float:
    return 100.0 * decided / encountered if encountered else 100.0


def print_report(totals: dict, rows: list[dict], incomplete: list[dict], baseline_path: Path,
                 baseline: dict | None, regressions: list[str], notes: list[str]) -> None:
    width = max(len(r["name"]) for r in rows) + 2
    print(f"Corpus: tests/cases.txt - {totals['cases_measured']} measured, "
          f"{totals['cases_setup']} setup-only, {totals['cases_unmeasured']} unmeasured")
    print()
    print(f"{'case':<{width}}{'decided':>7}{'of':>5}{'rate':>8}{'undec':>7}"
          f"{'unsupp':>7}{'panic':>7}  kinds")
    for row in rows:
        if row["kind"] == "setup":
            print(f"{row['name']:<{width}}{'-':>7}{'-':>5}{'setup':>8}{'-':>7}{'-':>7}{'-':>7}")
            continue
        if row["kind"] == "unmeasured":
            print(f"{row['name']:<{width}}{'-':>7}{'-':>5}{'UNMEASURED':>8}"
                  f"{'-':>7}{'-':>7}{'-':>7}  {row['reason']}")
            continue
        c = row["counts"]
        kinds = ",".join(c["unsupported_kinds"])
        print(f"{row['name']:<{width}}{c['decided']:>7}{c['encountered']:>5}"
              f"{rate(c['decided'], c['encountered']):>7.1f}%{c['undecided']:>7}"
              f"{c['unsupported']:>7}{c['panic_sites']:>7}  {kinds}")
    print()
    print(f"Decided assertions : {totals['decided']} of {totals['encountered']} "
          f"({rate(totals['decided'], totals['encountered']):.1f}%)")
    print(f"Declined, by name  : {totals['declined']} "
          f"(undecided {totals['undecided']}, unsupported {totals['unsupported']}, "
          f"unmodelled panic sites {totals['panic_sites']})")
    print(f"Counterexamples    : {totals['counterexamples']} (every one pinned by its case)")
    if totals["panic_sites_decided"]:
        print(f"Panic sites decided: {totals['panic_sites_decided']}")

    if baseline is not None:
        print()
        commit = baseline.get("recorded_at_commit") or "unknown commit"
        print(f"Baseline           : {baseline_path} ({commit})")
        for note in notes:
            print(f"  {note}")
        for regression in regressions:
            print(f"  [FAIL] {regression}")
        if not regressions and not notes:
            print("  no change")

    for case in incomplete:
        print()
        print(f"[FAIL] {case['name']}: {case['reason']} ({case['dir']})")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--root", default=str(Path(__file__).resolve().parent.parent),
                        help="repository root (default: the parent of scripts/)")
    parser.add_argument("--baseline", default=None,
                        help="baseline file (default: tests/progress_baseline.json)")
    parser.add_argument("--write-baseline", action="store_true",
                        help="freeze the current measurement as the baseline")
    parser.add_argument("--json", action="store_true", help="print the measurement as JSON")
    args = parser.parse_args(argv)

    root = Path(args.root).resolve()
    baseline_path = Path(args.baseline) if args.baseline else root / "tests" / "progress_baseline.json"
    totals, rows, incomplete = measure(root)
    commit = git_commit(root)

    if args.write_baseline:
        frozen = baseline_from(totals, rows, commit)
        baseline_path.write_text(json.dumps(frozen, indent=2) + "\n", encoding="utf-8")
        print(f"Wrote {baseline_path} at {commit or 'unknown commit'}: "
              f"{len(frozen['cases'])} cases, {totals['decided']} of "
              f"{totals['encountered']} decided")
        if incomplete:
            for case in incomplete:
                print(f"[FAIL] {case['name']}: not measured, so not frozen ({case['reason']})")
            return 2
        return 0

    baseline = None
    regressions: list[str] = []
    notes: list[str] = []
    if baseline_path.exists():
        baseline = json.loads(baseline_path.read_text(encoding="utf-8"))
        if baseline.get("version") != 1:
            raise SystemExit(f"{baseline_path}: unsupported version {baseline.get('version')!r}")
        regressions = compare(baseline, rows)
        notes = informational(baseline, rows)

    if args.json:
        payload = {
            "totals": totals,
            "decided_rate": round(rate(totals["decided"], totals["encountered"]), 1),
            "commit": commit,
            "cases": [
                {
                    "name": row["name"],
                    "kind": row["kind"],
                    "counts": row["counts"],
                    "reason": row["reason"],
                }
                for row in rows
            ],
            "regressions": regressions,
            "changes": notes,
        }
        print(json.dumps(payload, indent=2))
    else:
        print_report(totals, rows, incomplete, baseline_path, baseline, regressions, notes)

    if incomplete:
        print()
        print(f"[FAIL] corpus incomplete: {len(incomplete)} case(s) could not be measured; "
              "run the suite to write their records")
        return 2
    if regressions:
        print()
        print(f"[FAIL] {len(regressions)} regression(s) against {baseline_path}")
        return 1
    if baseline is not None:
        print()
        print("[PASS] no per-case regression against the frozen baseline")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

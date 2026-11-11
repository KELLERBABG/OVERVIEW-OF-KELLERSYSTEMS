#!/usr/bin/env python3
"""The false-positive rate of a run, from the reports it wrote (K7).

`report.json` classifies every FAIL by the state it was checked in (K6), which is
what makes a rate measurable at all: the FAILs that may be false positives are the
ones the analysis itself says it leaned on a generalised value to produce. This
script turns a set of reports into that number and fails when it is over a budget,
so a regression fails a step `tests/run_tests.sh` runs rather than being a
paragraph.

The method, stated once:

* A **FAIL** is a finding with `"status": "FAIL"`. Findings are grouped by the
  `crate` they name, so a tree with several reports for one crate is one row.
* **The same finding in two reports is counted once.** A run analyses the target
  crate *and* the crates it depends on, so a shared crate appears in more than one
  report: the measurement that prompted this script had `sss` in two. A finding is
  identified by crate, function, block, status, kind, details and witness, which is
  what makes the count a count of findings rather than of files.
* **widening-limited** — the value the *finding is about* leans on the loop
  abstraction. This is a dependence test over that value, not a location label
  (it was a location label until A19): the input is the failing assertion's own
  operands together with its witness symbols, and the answer is yes only when one
  of those names is a previous-trip symbol the widening introduced, a place the
  widening *emptied* because the back edge carries it (`PathState::generalised`),
  or is defined from one through the state's own definitions. A widened symbol
  declared for a place the finding never reads is not evidence, and neither is
  sitting inside a widened block. **This is the headline rate:
  `widening-limited / (FAIL - decided panics)`.** A decided panic
  (`defect_class: known-panic`) is a ground fact about a known enum variant, not
  a model the abstraction produced, so it is counted in the FAIL total and
  excluded from the denominator. On the committed corpus the rate is 2 of 18
  (0.111) — `tests/loop_fail` and `tests/nested_loop`, whose failing operands are
  loop-carried values — and the budget this project gates with is 0.15, so
  `--budget 0.10` against that same corpus exits 1.
* **guard-limited** — the guards that lead to the finding are all in the formula
  and the model satisfies them. Those are the ones to look at first.
* **unguarded** — the state carried no path condition at all. It is reported and
  deliberately *not* counted in the rate: a straight-line obligation legitimately
  has no guard, so the class mixes genuine findings with lost guards, and its size
  is the best available statement of how high the true rate could be.
* **constant-folded** — decided from literals, under no symbolic state.
* **known-panic** — a `FAIL` from a decided panic site: the receiver's enum
  variant is known (`None`, `Err`), so the call always panics. It carries no
  witness by design — there is no input to model — and is excluded from the
  rate's denominator for the same reason.
* **extreme** — FAILs whose witness sits at a type's boundary
  (`18446744073709551608`-shaped, `i64::MAX`, `2^128-1`). A witness the program's
  own guards exclude is the same suspicion seen from the witness's side, so it is
  reported as a cross-check on the breakdown rather than folded into it.

`WARN` findings are obligations the analysis *declined* to decide. Nothing was
claimed there, so they are counted as `undecided` and kept out of the rate.

The kinds that are not a verdict on an assertion are excluded from that count:
`unsupported-assertion` (an assertion whose kind this build cannot encode),
`path-insensitive` (a lost guard, reported so a FAIL beside it can be read with
suspicion) and `unmodelled-panic` (a call into another crate whose body is not
read). None of the three is an obligation this analysis declined, and counting
them as one inflated the column with things the column is not about - measured:
a fixture with two assertions and six `unwrap` call sites reported eight.

Usage:
  scripts/fp-rate.py tests/goldens                       # a directory or files
  scripts/fp-rate.py report.json --budget 0.20           # fail over budget
  scripts/fp-rate.py --dir . --json fp-rate.json         # write the table

Exit codes: 0 when within budget (or no budget given), 1 when over it, 2 on a
report that cannot be read as one.
"""

import argparse
import json
import os
import sys

# The witness values a modelled-but-unreachable state tends to produce: the top
# of an unsigned range and the boundary of a signed one. The same needles as
# scripts/sweep-local-crates.ps1, so the two measurements agree.
EXTREME = ("184467440737095516", "9223372036854775807", "340282366920938463463374607431768211")

# The classes a FAIL can be attributed to, in the order they are printed.
CLASSES = ("widening-limited", "guard-limited", "unguarded", "constant-folded", "known-panic")

# A FAIL in this class is a ground decision, not a model the abstraction produced:
# it is counted in the FAIL total but excluded from the rate's denominator.
DECIDED = ("known-panic",)

# The `WARN` kinds that are not a verdict on an assertion, so not an obligation
# either: one predicate, matching `report::is_non_assertion_kind`. An
# `unsupported-assertion` was never encoded, a `path-insensitive` WARN is a lost
# guard rather than a declined obligation, and an `unmodelled-panic` is a call
# into a crate whose body is not read.
NON_ASSERTION_KINDS = ("unsupported-assertion", "path-insensitive", "unmodelled-panic")

# Directories that hold copies of a run rather than the run: the verifier compiles
# each crate in an isolated copy under `target/`, so walking into one would count
# the same finding once per build directory.
SKIP_DIRS = {"target", ".git", "node_modules", ".measured"}


def is_report(path):
    """A JSON file that parses as a list of findings.

    Anything else in the tree — the `report.environment.json` sidecar beside a
    run, a golden for something that is not a report — is not one.
    """
    try:
        with open(path, encoding="utf-8") as handle:
            return isinstance(json.load(handle), list)
    except (OSError, json.JSONDecodeError):
        return False


def collect(paths, directories):
    """Every report under the given paths.

    A directory is taken to mean "the reports in here": the `report.json` of
    every run below it, plus the `*.json` files directly inside it — which is how
    a directory of goldens, named after their cases, is read. Build-copy
    directories are skipped: the verifier compiles each crate in an isolated copy
    under `target/`, so walking into one would count a finding once per copy.
    """
    files = []
    for path in paths:
        files.append(path)
    for directory in directories:
        if not os.path.isdir(directory):
            files.append(directory)
            continue
        for name in sorted(os.listdir(directory)):
            candidate = os.path.join(directory, name)
            if name.endswith(".json") and os.path.isfile(candidate):
                files.append(candidate)
        for root, dirs, names in os.walk(directory):
            dirs[:] = [d for d in dirs if d not in SKIP_DIRS and not d.startswith(".")]
            if "report.json" in names:
                files.append(os.path.join(root, "report.json"))
    return sorted({path for path in files if is_report(path)})


def blank(name):
    return {
        "crate": name,
        "pass": 0,
        "fail": 0,
        "undecided": 0,
        "extreme": 0,
        "unlabelled": 0,
        "unknown_class": 0,
        "duplicate": 0,
        **{name: 0 for name in CLASSES},
    }


def identity(finding):
    """What makes two appearances the same finding.

    Deliberately includes the witness: two runs of the same functions that agree on
    everything but the model are two findings to look at, not one.
    """
    witness = tuple(
        (w.get("symbol"), w.get("input"), w.get("value")) for w in (finding.get("witness") or [])
    )
    return (
        finding.get("crate"),
        finding.get("function"),
        finding.get("block"),
        finding.get("status"),
        finding.get("kind"),
        finding.get("details"),
        witness,
    )


def measure(path, crates, seen):
    try:
        with open(path, encoding="utf-8") as handle:
            findings = json.load(handle)
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"cannot read {path} as a report.json: {error}") from error
    if not isinstance(findings, list):
        raise SystemExit(f"{path} is not a report.json (expected a list of findings)")
    fallback = os.path.basename(os.path.dirname(os.path.abspath(path)))
    for finding in findings:
        crate = finding.get("crate") or fallback
        row = crates.setdefault(crate, blank(crate))
        marker = identity(finding)
        if marker in seen:
            row["duplicate"] += 1
            continue
        seen.add(marker)
        status = finding.get("status")
        if status == "PASS":
            row["pass"] += 1
            continue
        # A WARN is an obligation the analysis declined to decide: nothing was
        # claimed, so it is not a FAIL and must not reach the block below. The
        # three kinds that are not an obligation at all are counted nowhere: see
        # the note in this file's docstring.
        if status == "WARN":
            if finding.get("kind") not in NON_ASSERTION_KINDS:
                row["undecided"] += 1
            continue
        if status != "FAIL":
            continue
        row["fail"] += 1
        defect_class = finding.get("defect_class")
        if defect_class in CLASSES:
            row[defect_class] += 1
        else:
            row["unknown_class"] += 1
        # A decided panic carries no witness by design: there is no input to model.
        if defect_class in DECIDED:
            continue
        witnesses = finding.get("witness") or []
        if not witnesses:
            row["unlabelled"] += 1
        rendered = " ".join(str(w.get("value")) for w in witnesses)
        if any(needle in rendered for needle in EXTREME):
            row["extreme"] += 1


def rate_of(row):
    decided = sum(row[name] for name in DECIDED)
    denominator = row["fail"] - decided
    return (row["widening-limited"] / denominator) if denominator else 0.0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("reports", nargs="*", help="report.json files, or directories to search")
    parser.add_argument(
        "--dir",
        action="append",
        default=[],
        help="search this directory for report.json files (repeatable)",
    )
    parser.add_argument(
        "--budget",
        type=float,
        default=None,
        help="fail when the total suspect rate exceeds this fraction (e.g. 0.20)",
    )
    parser.add_argument("--json", dest="json_out", default=None, help="write the table as JSON here")
    args = parser.parse_args(argv)

    paths = collect(args.reports, args.dir)
    if not paths:
        raise SystemExit("no report.json given; pass files, directories, or --dir")

    crates = {}
    seen = set()
    for path in paths:
        measure(path, crates, seen)
    rows = sorted(crates.values(), key=lambda row: row["crate"])
    total = blank("TOTAL")
    for row in rows:
        for key, value in row.items():
            if key != "crate":
                total[key] += value
    for row in rows + [total]:
        row["rate"] = rate_of(row)

    header = (
        f"{'crate':<28}{'FAIL':>6}{'widen':>7}{'guard':>7}{'plain':>7}{'fold':>6}"
        f"{'known':>6}{'extreme':>9}{'undecided':>11}{'rate':>8}"
    )
    print("K7: FAILs by the state they were checked in (method: scripts/fp-rate.py)")
    print(header)
    print("-" * len(header))
    for row in rows:
        print(
            f"{row['crate']:<28}{row['fail']:>6}{row['widening-limited']:>7}"
            f"{row['guard-limited']:>7}{row['unguarded']:>7}{row['constant-folded']:>6}"
            f"{row['known-panic']:>6}{row['extreme']:>9}{row['undecided']:>11}{row['rate']:>8.3f}"
        )
    print("-" * len(header))
    print(
        f"{total['crate']:<28}{total['fail']:>6}{total['widening-limited']:>7}"
        f"{total['guard-limited']:>7}{total['unguarded']:>7}{total['constant-folded']:>6}"
        f"{total['known-panic']:>6}{total['extreme']:>9}{total['undecided']:>11}{total['rate']:>8.3f}"
    )
    print()
    decided = sum(total[name] for name in DECIDED)
    print(
        f"rate = widening-limited / (FAIL - decided panics) = {total['widening-limited']}/"
        f"{total['fail'] - decided} = {total['rate']:.3f} over {len(paths)} report(s), "
        f"{len(rows)} crate(s); {total['fail']} FAIL(s), {decided} decided panic(s) excluded"
    )
    if total["unknown_class"]:
        print(f"WARNING: {total['unknown_class']} FAIL(s) carry no recognised defect_class")
    if total["unlabelled"]:
        print(f"WARNING: {total['unlabelled']} FAIL(s) carry no witness at all")
    if total["duplicate"]:
        print(
            f"note: {total['duplicate']} finding(s) appeared in more than one report "
            "(a crate that two runs both analysed) and were counted once"
        )

    if args.json_out:
        with open(args.json_out, "w", encoding="utf-8") as handle:
            json.dump({"crates": rows, "total": total}, handle, indent=2, sort_keys=True)
            handle.write("\n")

    if args.budget is not None:
        if total["rate"] > args.budget:
            print(
                f"[FAIL] the suspect rate {total['rate']:.3f} is over the budget "
                f"{args.budget:.3f}: {total['widening-limited']} of "
                f"{total['fail'] - decided} abstraction-sensitive FAIL(s) "
                "lean on the loop abstraction"
            )
            return 1
        print(f"[PASS] the suspect rate {total['rate']:.3f} is within the budget {args.budget:.3f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

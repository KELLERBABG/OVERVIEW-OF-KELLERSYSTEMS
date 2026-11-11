#!/usr/bin/env bash
# ===========================================================================
#  keller-verify integration test runner (Linux / macOS)
#
#  Builds the verifier, then runs `keller verify` against each test crate and
#  asserts the expected exit code and report.json contents:
#
#    safe_code      -> exit 0 (no FAIL events)
#    overflow_fail  -> exit 1 (report.json contains a FAIL)
#    divzero_fail   -> exit 1 (report.json contains a FAIL)
#    branch_merge   -> exit 1 (a constant merged from two branches must not
#                              be folded to "safe" and silently skipped)
#    guarded        -> exit 1, and the four functions whose arithmetic a branch
#                              guard already rules out must NOT appear in the
#                              report, while the one with too weak a guard must
#    workspace_multi -> exit 1 and report.json must contain BOTH members'
#                              findings (per-crate reports are merged)
#    loop_fail      -> exit 1: a loop body is analysed under its own guards, so
#                              the counter increment is *proved* while the
#                              accumulator's reachable overflow is still
#                              reported, with the trip it can fire on
#    nested_loop    -> exit 1: the shape next door to loop_fail â€” an inner loop
#                              inside an outer one, with break and continue. The
#                              cycle widening bounds the *condition* as well as
#                              the values, so its guarded increments are proved
#                              too and only the accumulator's unbounded overflow
#                              is reported; the single-cycle control in the same
#                              crate is proved as well, which is the boundary the
#                              case pins. Run once more with the round budget cut
#                              to one round, which also pins the *report* of a
#                              lost precision: both increments come back as FAILs
#                              and the report says why, instead of leaving the
#                              false positives unattributable
#    shift_amount   -> exit 1, and the shift whose amount can leave the type's
#                              width must be reported while the two shifts that
#                              cannot panic must be proved
#    fold_fail      -> exit 1, and the failures the constant-folding pass found
#                              without asking Z3 must carry a witness and a
#                              re-dischargeable obligation like any other
#    broken_code    -> exit 3: a crate the compiler rejected is a setup failure,
#                              not a clean run
#    precond_probe  -> exit 1: a precondition written as an in-body assertion is
#                              believed as a guard; a private leaf is now analysed
#                              under the precondition its constant call sites prove
#                              (S7), while an exported helper and one called with an
#                              unbounded value are still judged without their call
#                              sites (A8's boundary, narrowed and still pinned)
#    helper_summary -> exit 1: the S7 summary is used only where its call sites
#                              prove it, so inc loses its unguarded finding and
#                              boom/open/exported keep theirs
#    test-crates/buggy-project -> exit 1: one finding of each named class
#                              (bounds, overflow, division by zero)
#    test-crates/safe-project  -> exit 0: the negative control beside it, and the
#                              only fixture outside tests/ that is run at all
#
#  A FAIL must also name the input that triggers it â€” whether the obligation went
#  to Z3 or was decided by constant folding: every FAIL entry carries a `witness`
#  in report.json, and the pinned `*.witness.smt2` obligation â€” the formula with
#  those values asserted â€” must re-discharge to `sat`.
#
#  Finally the exit-code contract's own flags: `--deny-warn` must tighten the
#  gate without changing any verdict above, and an unknown flag must exit 2
#  rather than quietly run with the gate switched off.
#
#  Last, the published page. index.html is a hand-written page and this suite no
#  longer generates or diffs it: it carries no per-run figures, so there is
#  nothing here for it to disagree with.
#
#  Exit code 0 = all cases passed, 1 = at least one case failed.
#
#  Requirements: rustup with the pinned nightly (see tests/rust-toolchain.toml),
#  Z3 installed (Z3_HOME pointing at its root, or a system install).
# ===========================================================================
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERIFIER="$ROOT/target/debug/keller-verify"

# Windows builds the binary with an .exe suffix.
if [ ! -x "$VERIFIER" ] && [ -x "$VERIFIER.exe" ]; then
  VERIFIER="$VERIFIER.exe"
fi

# Always, not only when it is missing: the suite decides every case with this
# binary, so a stale one means the run reports on code nobody is looking at â€” and
# `cargo check` does not produce a binary at all. That is not hypothetical: a
# source edit that only `cargo check` had seen was tested by this suite as though
# it had been applied. Incremental, so it costs a second when nothing changed.
if [ ! -x "$VERIFIER" ]; then
  VERIFIER="$ROOT/target/debug/keller-verify"
  [ -x "$VERIFIER.exe" ] && VERIFIER="$VERIFIER.exe"
fi
echo "[tests] Building the verifier from this tree..."
(cd "$ROOT" && cargo build) || {
  echo "[FAIL] build: cargo build failed, so no case can be run against this source"
  exit 1
}

# Make the toolchain's runtime libraries and Z3 findable by the wrapper.
#
# The lookup is not written here. `src/host_env.rs` holds it - one module shared
# with `build.rs`, which links against the same Z3 - and `keller-env`, a second
# binary built from this crate, prints the answer; this is that printout, asked
# for in shell form (`/c/...` paths, quoted assignments) so it can simply be
# `eval`ed. The helper is `std`-only on purpose: the verifier is linked against
# Z3 and loads the toolchain's driver dylibs at run time, so it cannot start
# until these directories are already on PATH - which is exactly what it would
# be being asked to work out.
#
# This script used to carry its own copy of the rule, and so did run_tests.cmd,
# and the two had drifted. The build took the *last* of several sorted releases
# under `.z3/`; this script took the *first* that ran; the Windows one did not
# glob at all and fell back to a machine-specific path, never checking that what
# it found would start. A checkout with two releases unpacked could therefore be
# linked against one and re-discharged by the other. There is one copy now, and
# a unit test in that module fails the build if either script grows another
# (`the_runners_do_not_look_for_the_toolchain_or_z3_themselves`).
ENV_HELPER="$ROOT/target/debug/keller-env"
if [ ! -x "$ENV_HELPER" ] && [ -x "$ENV_HELPER.exe" ]; then
  ENV_HELPER="$ENV_HELPER.exe"
fi

# Every value the helper prints, empty when it could not be found, so nothing
# below has to guess whether a line was missing or blank.
RUSTC=""
HOST=""
RUSTLIB=""
Z3_BIN=""
Z3_SOURCE=""
PATH_ADD=""
ENV_DUMP="$("$ENV_HELPER" --shell sh --manifest-dir "$ROOT")"
env_exit=$?
if [ "$env_exit" != "0" ]; then
  echo "[FAIL] env: keller-env could not report the toolchain and Z3 (exit $env_exit)"
  exit 1
fi
eval "$ENV_DUMP"
if [ -n "$PATH_ADD" ]; then
  PATH="$PATH_ADD:$PATH"
  export PATH
fi
if [ -n "$RUSTLIB" ] || [ -n "$PATH_ADD" ]; then
  export LD_LIBRARY_PATH="${RUSTLIB:+${RUSTLIB}:}${PATH_ADD:+${PATH_ADD}:}${LD_LIBRARY_PATH:-}"
fi

# Nothing below can run without the toolchain's driver dylibs: every case would
# die at load time, which Windows reports as STATUS_DLL_NOT_FOUND and a POSIX
# shell reports as exit 127 - the code an unfound command gives - so a broken
# setup reads as a broken tool. Stop here, with the reason, instead of letting
# every case report it one at a time.
if [ ! -d "$RUSTLIB" ]; then
  echo "[FAIL] env: the pinned toolchain's rustc driver dylibs were not found, so no case can start the wrapper"
  echo "[env] rustc: ${RUSTC:-<rustup named none>} (host ${HOST:-<unknown>})"
  echo "[env] install the pinned nightly with its rustc-dev component (see tests/rust-toolchain.toml)"
  exit 1
fi

echo "[env] rustc: $RUSTC ($HOST)"

# Whether the fixture crates can spell an x86-64 inline asm block, decided from
# the toolchain's host rather than from a guess: `tests/statement_writes` carries
# one shape only that architecture can assemble, and both the case's warn count
# and the assertions about that shape follow this.
SW_ASM=0
case "$HOST" in
  x86_64-* | i686-*) SW_ASM=1 ;;
esac
echo "[env] driver dylibs: $RUSTLIB"
echo "[env] solver: ${Z3_BIN:-<none>} (from $Z3_SOURCE)"
# The toolchain half is fatal above, because nothing runs without it. The solver
# half is not: a machine without Z3 still runs every case, and reports what it
# could not re-discharge rather than treating it as checked.
if [ -n "$Z3_BIN" ]; then
  echo "[PASS] env: keller-env resolved the toolchain and a solver"
else
  echo "[SKIP] env: no runnable z3, so obligations cannot be re-discharged"
fi


# Where each case's console stream and the list of cases land. Later cases grep
# these streams for the lines they assert on, and a FAIL has to quote the run it
# came from, so they are captured rather than streamed.
MEASURED="$ROOT/tests/.measured"
# The cases this run actually ran, from tests/cases.txt. Written by
# `run_table_case` and by the cases that need their own invocation; read by
# `check_case_table`.
CONSUMED="$MEASURED/cases.consumed"
rm -rf "$MEASURED"
mkdir -p "$MEASURED"

FAILED=0

# Where a case's crate lives.
#
# Almost every fixture is `tests/<name>`, and the checks below are written in
# terms of which case they are looking at. Two are not: `test-crates/*` has been
# at the repository root since it was written. Resolving the directory here is
# what lets those two go through the same runner and the same checks instead of
# carrying a second, thinner copy of them.
case_dir() {
  local case="$1"
  if [ -d "$ROOT/$case" ]; then
    printf '%s' "$ROOT/$case"
  else
    printf '%s' "$ROOT/tests/$case"
  fi
}

# A case name that can be a file name. `test-crates/safe-project` is one case
# with a path separator in its name, and `>"$MEASURED/test-crates/safe-project.log"`
# fails outright — the redirect needs a directory that does not exist, the command
# never runs, and the case reports "exit 1 but report.json contains no FAIL" for a
# crate that was never analysed. That is exactly how a fixture looks wired into a
# suite while proving nothing, so the log names are slugged and the case names are
# not.
case_slug() {
  printf '%s' "${1//\//__}"
}

# The fixture cases, their exit codes and their goldens live in one file that
# this runner and the Windows runner both read, so a case cannot exist in one and
# be missing from the other. The row is the authority: `run_table_case` looks the
# exit code and the label up rather than taking them as arguments, and compares
# the golden exactly when the row names one. `check_case_table` at the end checks
# both ends - every row consumed, and the golden column equal to `tests/goldens`.
CASES_FILE="$ROOT/tests/cases.txt"
case_field() { grep -m1 "^$1|" "$CASES_FILE" | cut -d'|' -f"$2"; }
case_known() { grep -q "^$1|" "$CASES_FILE"; }
case_exit() { case_field "$1" 2; }
case_golden() { case_field "$1" 3; }
case_label() { case_field "$1" 4; }
case_table_names() { grep -v '^#' "$CASES_FILE" | cut -d'|' -f1; }
case_table_goldens() {
  grep -v '^#' "$CASES_FILE" | awk -F'|' '$3 != "-" { print $3 }' | sort
}
# Recorded, not inferred: a case that needed its own invocation (a solver budget,
# an undecided verdict, the precision probe) is marked where it runs, so the
# completeness check below can tell "handled deliberately" from "forgotten".
mark_consumed() { printf '%s\n' "$1" >> "$CONSUMED"; }

# One case, driven by the table.
run_table_case() {
  local case="$1"
  if ! case_known "$case"; then
    echo "[FAIL] $case: no row in tests/cases.txt, so nothing says what it must do"
    FAILED=1
    return
  fi
  run_case "$case" "$(case_exit "$case")" "$(case_label "$case")"
  mark_consumed "$case"
  # A case the table records a golden for is compared; one it does not is not.
  # Which cases those are is no longer a decision each runner makes for itself.
  [ "$(case_golden "$case")" = "-" ] || check_goldens "$case"
}

# The two ends of that single owner, checked once the cases have run.
#
# 1. Every row was consumed. A case that exists in `tests/cases.txt` and is never
#    run here is exactly the gap the file exists to close, so it is a failure and
#    not a note. A case that needs its own invocation - a solver budget, an
#    undecided verdict, the precision probe - marks itself where it runs, so
#    "handled deliberately" and "forgotten" are different states.
# 2. The rows that name a golden are exactly the files in `tests/goldens`. This is
#    the drift that was measured before the file existed: the Windows runner
#    compared ten goldens while this one compared thirteen, and nothing in the
#    tree said which number was right.
#
# Neither check runs while recording: `UPDATE_GOLDENS=1` is the one mode in which
# a case with no golden is supposed to be a case with no golden.
check_case_table() {
  local name unrun=0
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    if ! grep -qxF "$name" "$CONSUMED" 2>/dev/null; then
      echo "[FAIL] $name: tests/cases.txt lists it and this runner never ran it"
      FAILED=1
      unrun=$((unrun + 1))
    fi
  done < <(case_table_names)
  if [ "$unrun" = "0" ]; then
    echo "[PASS] every case in tests/cases.txt was run"
  fi

  if [ "${UPDATE_GOLDENS:-0}" = "1" ]; then
    echo "[SKIP] goldens: recording mode, so the table and tests/goldens may differ"
    return 0
  fi
  case_table_goldens > "$MEASURED/goldens.expected"
  (cd "$ROOT/tests/goldens" && ls *.json 2>/dev/null | sed 's/\.json$//' | sort) \
    > "$MEASURED/goldens.actual"
  local goldens_n cases_n
  goldens_n=$(wc -l < "$MEASURED/goldens.actual" | tr -d ' ')
  cases_n=$(case_table_names | wc -l | tr -d ' ')
  if diff -q "$MEASURED/goldens.expected" "$MEASURED/goldens.actual" >/dev/null 2>&1; then
    echo "[PASS] tests/cases.txt and tests/goldens agree: $goldens_n goldens over $cases_n cases"
  else
    echo "[FAIL] tests/cases.txt and tests/goldens disagree about which cases have a golden"
    diff -u "$MEASURED/goldens.expected" "$MEASURED/goldens.actual" | sed 's/^/       /' | head -20
    FAILED=1
  fi
}

run_case() {
  local case="$1" expected="$2" label="$3"
  local dir slug
  dir="$(case_dir "$case")"
  slug="$(case_slug "$case")"
  local exitcode
  # Guarantee a fresh compilation through the verifier: cargo's incremental
  # fingerprints do not notice RUSTC_WRAPPER changes.
  rm -rf "$dir/target"
  (cd "$dir" && rm -f report.json)
  # Captured, not discarded: the landing page's obligation rows are derived from
  # exactly the run asserted below, so the page and the check cannot drift apart.
  (cd "$dir" && "$VERIFIER" keller verify >"$MEASURED/$slug.log" 2>&1)
  exitcode=$?
  if [ "$case" = "safe_code" ]; then
    # vec! emits null/alignment assertions which are explicitly unsupported.
    printf '%s\t%s\t2\n' "$case" "$expected" >> "$MEASURED/manifest.tsv"
  else
    printf '%s\t%s\n' "$case" "$expected" >> "$MEASURED/manifest.tsv"
  fi

  if [ "$exitcode" != "$expected" ]; then
    echo "[FAIL] $case: expected exit $expected, got $exitcode ($label)"
    FAILED=1
    return
  fi

  if [ "$expected" = "1" ]; then
    if ! grep -q '"status": "FAIL"' "$dir/report.json" 2>/dev/null; then
      echo "[FAIL] $case: exit 1 but report.json contains no FAIL ($label)"
      FAILED=1
      return
    fi
  fi

  echo "[PASS] $case: exit $exitcode as expected ($label)"
}

# `run_case` plus extra command-line arguments, for the cases that probe the
# exit-code contract's own flags rather than the analysis.
run_gate_case() {
  local case="$1" expected="$2" label="$3"
  shift 3
  local dir
  dir="$(case_dir "$case")"
  local exitcode
  rm -rf "$dir/target"
  # Gate probes must not replace the report paired with the measured log.
  local saved="$MEASURED/$(case_slug "$case").gate-report.json"
  if [ -f "$dir/report.json" ]; then
    cp "$dir/report.json" "$saved"
  fi
  (cd "$dir" && "$VERIFIER" keller verify "$@" >/dev/null 2>&1)
  exitcode=$?
  if [ -f "$saved" ]; then
    cp "$saved" "$dir/report.json"
    rm -f "$saved"
  fi

  if [ "$exitcode" != "$expected" ]; then
    echo "[FAIL] $case: expected exit $expected, got $exitcode ($label)"
    FAILED=1
    return
  fi

  echo "[PASS] $case: exit $exitcode as expected ($label)"
}

# A verdict is only worth something if the file it came from is a valid SMT-LIB
# problem. `Solver`'s Display wraps long commands, so the emitter is easy to
# break in a way that still "looks" right; re-discharge every obligation with
# the standalone solver instead of trusting it.
check_obligations() {
  local case="$1"
  local dir="$(case_dir "$case")/obligations"
  [ -d "$dir" ] || return 0

  # The solver the helper resolved, which is the one the verifier was linked
  # against. This used to be a narrower lookup of its own (`PATH`, then
  # `$Z3_HOME/bin`) and the page generator carried a third, so three answers to
  # which Z3 decides were possible in one run.
  local z3bin="$Z3_BIN"
  if [ -z "$z3bin" ]; then
    echo "[SKIP] $case: no runnable z3 was found to re-discharge obligations"
    return 0
  fi

  # Obligations are filed under a per-crate subdirectory, so two crates cannot
  # overwrite each other's files. Every verdict now leaves one, and the file's
  # recorded verdict is what the standalone solver has to reproduce:
  #
  #   FAIL (sat)|FAIL (folded)  -> the negation is satisfiable: `sat`
  #   PASS (unsat)              -> the negation is unsatisfiable: `unsat`
  #   WARN (unknown)            -> whatever the solver says under its own budget
  #
  # A `*.witness.smt2` file pins the extracted counterexample to the reported
  # values, so it must be `sat` â€” an `unsat` there would mean report.json named an
  # input that does not fail. Only a FAIL gets one, which is the rule asserted at
  # the end.
  local f out base pinned verdict want got
  local fail_n=0 pass_n=0 pinned_n=0
  while IFS= read -r f; do
    base="${f#"$dir"/}"
    out="$("$z3bin" -smt2 "$f" 2>&1)"
    # The verdict line the emitter wrote, without the header's padding.
    verdict="$(grep -m1 '^; verdict:' "$f" | sed 's/^; verdict: *//')"
    case "$base" in
      *.witness.smt2)
        pinned=1
        pinned_n=$((pinned_n + 1))
        ;;
      *)
        pinned=0
        case "$verdict" in
          FAIL*) fail_n=$((fail_n + 1)) ;;
          PASS*) pass_n=$((pass_n + 1)) ;;
        esac
        ;;
    esac
    if printf '%s' "$out" | grep -qiE "error|unsupported"; then
      echo "[FAIL] $case: $base is not valid SMT-LIB: $out"
      FAILED=1
      continue
    fi
    got="$(printf '%s' "$out" | tr -d '\r\n ')"
    if [ "$pinned" = "1" ]; then
      if [ "$got" != "sat" ]; then
        echo "[FAIL] $case: $base pins a witness but is not sat: $out"
        FAILED=1
      else
        echo "[PASS] $case: $base pinned witness re-discharged -> sat"
      fi
      continue
    fi
    case "$verdict" in
      "FAIL (sat)"|"FAIL (folded)")
        want=sat
        ;;
      "PASS (unsat)")
        want=unsat
        ;;
      *)
        want=""
        ;;
    esac
    if [ -n "$want" ]; then
      if [ "$got" != "$want" ]; then
        echo "[FAIL] $case: $base records '$verdict' but re-discharges to '$got'"
        FAILED=1
      else
        echo "[PASS] $case: $base re-discharged -> $want (records '$verdict')"
      fi
    else
      # An undecided verdict is a budget answer, not a formula property: the run's
      # budget decided it, and a standalone solver without one is free to answer
      # `sat`, `unsat` or `unknown`. It is only required to be a valid problem.
      echo "[PASS] $case: $base is valid SMT-LIB and answers '$got' (records '$verdict')"
    fi
  done < <(find "$dir" -type f -name '*.smt2' | sort)

  # Every FAIL has its pinned counterpart, and only a FAIL has one: the witness is
  # what turns a counterexample into a bug report, while a proof has no model to
  # pin. This used to count every obligation against every pinned file, which was
  # the same rule only while a PASS left no file at all. It also used to sit after
  # `check_locations`, which left `check_locations` defined *inside* this function â€”
  # it worked, but only because the outer body happened to run before the call.
  if [ "$fail_n" != "$pinned_n" ]; then
    echo "[FAIL] $case: $fail_n FAIL obligation(s) but $pinned_n pinned witness file(s)"
    FAILED=1
  fi
  if [ "$pass_n" -gt 0 ]; then
    echo "[PASS] $case: $pass_n prove(s) left a re-dischargeable obligation, not just a verdict"
  fi
}

# Each event also has to say where it came from: a finding names its file and
# line, and the class it belongs to, so a reader (or a platform) can act on it
# without string-matching the prose.
check_locations() {
  local case="$1"
  local report="$(case_dir "$case")/report.json"
  [ -f "$report" ] || return 0
  local findings
  findings=$(grep -c '"status": ' "$report" 2>/dev/null || true)
  local located
  located=$(grep -c '"file": ' "$report" 2>/dev/null || true)
  local kinds
  kinds=$(grep -c '"kind": ' "$report" 2>/dev/null || true)
  if [ "$findings" -gt 0 ] && [ "$located" -lt "$findings" ]; then
    echo "[FAIL] $case: $findings finding(s) but only $located carry a source file"
    FAILED=1
    return
  fi
  if [ "$findings" -gt 0 ] && [ "$kinds" -lt "$findings" ]; then
    echo "[FAIL] $case: $findings finding(s) but only $kinds carry a machine-readable kind"
    FAILED=1
    return
  fi
  echo "[PASS] $case: every finding names its file, line and class"
}

# A committed golden for each case's whole `report.json`.
#
# The case checks assert the fields someone thought to assert, which leaves the
# rest of the file â€” field order, the class and the source location, the witness
# values, the set of events â€” unguarded. A golden catches any of it changing
# without anyone deciding that it should. Re-record after an intended change with
# `UPDATE_GOLDENS=1 tests/run_tests.sh`, and read the diff as the review.
check_goldens() {
  local case="$1"
  local report="$(case_dir "$case")/report.json"
  local golden="$ROOT/tests/goldens/$case.json"

  if [ "${UPDATE_GOLDENS:-0}" = "1" ]; then
    if [ -f "$report" ]; then
      mkdir -p "$ROOT/tests/goldens"
      cp "$report" "$golden"
      echo "[RECORDED] $case: tests/goldens/$case.json"
    fi
    return 0
  fi

  # No report and no golden: a case with nothing to report, nothing to compare.
  if [ ! -f "$report" ] && [ ! -f "$golden" ]; then
    return 0
  fi
  if [ ! -f "$golden" ]; then
    echo "[FAIL] $case: no golden for this case â€” record one with UPDATE_GOLDENS=1"
    FAILED=1
    return
  fi
  if [ ! -f "$report" ]; then
    echo "[FAIL] $case: a golden exists but this run produced no report.json"
    FAILED=1
    return
  fi
  if diff -q "$golden" "$report" >/dev/null 2>&1; then
    echo "[PASS] $case: report.json matches its golden"
  else
    echo "[FAIL] $case: report.json differs from tests/goldens/$case.json"
    diff -u "$golden" "$report" | head -30
    FAILED=1
  fi
}

# Every run records the toolchain and the solver it was decided in, beside the
# report.
#
# Not inside `report.json`: that file is byte-compared against the goldens and
# these facts are absolute paths, so a machine-specific path in it would fail
# every golden on any other machine. It is written to `report.environment.json`
# instead, which is why this reads a second file.
#
# Asserted in both directions. The sidecar must carry every key `keller-env`
# prints â€” a fact that went missing would leave the verdict unattributable â€” and
# `report.json` must not have grown an `env` block, because that is the shape
# change that would silently break every golden diff.
check_environment() {
  local case="$1"
  local report="$(case_dir "$case")/report.json"
  local sidecar="$(case_dir "$case")/report.environment.json"
  if [ ! -f "$sidecar" ]; then
    echo "[FAIL] $case: no report.environment.json, so nothing says what decided it"
    FAILED=1
    return
  fi
  local missing="" key
  for key in env_source rustc host rustlib z3_bin z3_source path_add; do
    if ! grep -q "\"$key\":" "$sidecar" 2>/dev/null; then
      missing="$missing $key"
    fi
  done
  if [ -n "$missing" ]; then
    echo "[FAIL] $case: report.environment.json is missing:$missing"
    FAILED=1
    return
  fi
  if [ -f "$report" ] && grep -q '"env":' "$report" 2>/dev/null; then
    echo "[FAIL] $case: report.json carries an env block, which would break its golden"
    FAILED=1
    return
  fi
  echo "[PASS] $case: report.environment.json records the toolchain and Z3"
}

# A FAIL has to name the input that triggers it, both machine-readably
# ("witness") and in the human-readable "details" line.
check_witnesses() {
  local case="$1"
  local report="$(case_dir "$case")/report.json"
  [ -f "$report" ] || return 0

  local fails inputs labelled
  # `grep -c` prints 0 and exits 1 on no match; `|| true` keeps that 0 rather
  # than appending a second one.
  fails=$(grep -c '"status": "FAIL"' "$report" 2>/dev/null || true)
  inputs=$(grep -c '"input": ' "$report" 2>/dev/null || true)
  labelled=$(grep -c '"details": ".* = ' "$report" 2>/dev/null || true)

  if [ "$fails" -gt 0 ] && [ "$inputs" -lt "$fails" ]; then
    echo "[FAIL] $case: $fails FAIL(s) but only $inputs witness input(s) in report.json"
    FAILED=1
    return
  fi
  if [ "$fails" -gt 0 ] && [ "$labelled" -lt "$fails" ]; then
    echo "[FAIL] $case: $fails FAIL(s) but only $labelled details line(s) name a value"
    FAILED=1
    return
  fi
  echo "[PASS] $case: every FAIL carries a concrete witness in report.json"
}

run_table_case safe_code
check_obligations safe_code
# Completed runs always have a findings array and an environment sidecar.
# This fixture now exposes the pointer assertions it previously skipped.
check_environment safe_code
check_witnesses safe_code
run_table_case overflow_fail
check_obligations overflow_fail
check_witnesses overflow_fail
check_locations overflow_fail
check_environment overflow_fail
run_table_case divzero_fail
check_obligations divzero_fail
check_witnesses divzero_fail
check_locations divzero_fail
run_table_case branch_merge
check_obligations branch_merge
check_witnesses branch_merge
check_locations branch_merge
run_table_case guarded
check_obligations guarded
check_witnesses guarded
check_locations guarded

# Path sensitivity, asserted in both directions. A regression that simply stopped
# flagging overflows would pass the "proved" half and fail the control; one that
# went back to ignoring the guards fails the "proved" half. The grep patterns
# carry the quote so `sum_under_guarded` cannot satisfy the `sum_guarded` check.
for proved in sum_guarded slice_guarded div_guarded dead_branch; do
  if grep -q "\"$proved\"" "$ROOT/tests/guarded/report.json" 2>/dev/null; then
    echo "[FAIL] guarded: $proved is guarded and must be proved, but it is reported"
    FAILED=1
  else
    echo "[PASS] guarded: $proved is proved, so it is absent from the report"
  fi
done
if grep -q '"sum_under_guarded"' "$ROOT/tests/guarded/report.json" 2>/dev/null; then
  echo "[PASS] guarded: sum_under_guarded is still flagged (its guard is too weak)"
else
  echo "[FAIL] guarded: sum_under_guarded must be flagged, but the report is silent"
  FAILED=1
fi

run_table_case workspace_multi
check_obligations workspace_multi
check_witnesses workspace_multi
check_locations workspace_multi

# The point of the merge: both members must be present in one report. Asserting
# the function names catches a regression where the last worker wins again â€” the
# exit code alone would still be 1 with only one of them.
for needle in alpha_overflow beta_divide; do
  if grep -q "\"$needle\"" "$ROOT/tests/workspace_multi/report.json" 2>/dev/null; then
    echo "[PASS] workspace_multi: merged report contains $needle"
  else
    echo "[FAIL] workspace_multi: merged report is missing $needle"
    FAILED=1
  fi
done

# Five members, not two (D9). The merge is a *parallel* write and the bug this
# case replaced was a parallel-ordering clobber, so what is asserted is the
# invariant a race can break, not an implementation detail: every member is
# accounted for by its own worker (one `--crate-name` receipt each, five of them),
# and the findings are exactly the two this workspace has always reported - which is
# why three clean members leave its golden unchanged. `gamma`, `delta` and
# `epsilon` are the empty reports that would erase alpha's and beta's findings if the
# merge ever went back to "the last worker writes report.json".
ws_workers=$(grep -c '"workers":' "$ROOT/tests/workspace_multi/report.run.json" 2>/dev/null || true)
ws_named=$(grep -A1 '"--crate-name"' "$ROOT/tests/workspace_multi/report.run.json" 2>/dev/null \
  | grep -cE '^ *"(alpha|beta|gamma|delta|epsilon)",' || true)
if [ "${ws_workers:-0}" = "1" ] && [ "${ws_named:-0}" = "5" ]; then
  echo "[PASS] workspace_multi: five members, five worker receipts, one merged coverage"
else
  echo "[FAIL] workspace_multi: expected 5 worker receipts and one coverage block, got ${ws_named:-0} and ${ws_workers:-0}"
  FAILED=1
fi
# And the property a race cannot be trusted to keep: the same five members, run
# again, produce the same bytes. A clobber that depends on which member finished
# last is exactly what a second run with a different order shows up, and the golden
# above is one of the two files this compares.
cp "$ROOT/tests/workspace_multi/report.json" "$MEASURED/workspace_multi.first.json"
rm -rf "$ROOT/tests/workspace_multi/target"
(cd "$ROOT/tests/workspace_multi" && "$VERIFIER" keller verify >"$MEASURED/workspace_multi.repeat.log" 2>&1)
ws_repeat=$?
if [ "$ws_repeat" = "1" ] && diff -q "$MEASURED/workspace_multi.first.json" "$ROOT/tests/workspace_multi/report.json" >/dev/null 2>&1; then
  echo "[PASS] workspace_multi: a second run of the same five members is byte-identical, so no member's report depends on the order"
else
  echo "[FAIL] workspace_multi: the merge is not stable across runs (exit $ws_repeat)"
  FAILED=1
fi

run_table_case loop_fail
check_obligations loop_fail
check_witnesses loop_fail
check_locations loop_fail

# A lost precision has to be *reported*, and the run above cannot show that: its
# dataflow converges, so there is nothing to report. What can be shown is that the
# record exists and is reachable. The round budget is the boundary of this finding
# as well as a safety net, so cutting it to one round drives every function in this
# crate to that boundary â€” and the run then has to say so, instead of quietly
# reporting the guards' work as failures. The two increments the default budget
# proves come back as FAILs, and that pair is exactly the false positive this
# record exists to attribute. Run before the case below, which resets report.json
# and the console log the checks and the page are built from.
rm -rf "$ROOT/tests/nested_loop/target"
(cd "$ROOT/tests/nested_loop" && rm -f report.json && export KELLER_MAX_ROUNDS=1 && "$VERIFIER" keller verify >"$MEASURED/nested_loop.imprecise.log" 2>&1)
nl_imprecise=$?
if [ "$nl_imprecise" = "1" ]; then
  echo "[PASS] nested_loop: a cut round budget still exits 1 on the flags it leaves behind"
else
  echo "[FAIL] nested_loop: expected exit 1 with the round budget cut to one round, got $nl_imprecise"
  FAILED=1
fi
if grep -qF '[WARN] Path-insensitive fallback in nested_walk' "$MEASURED/nested_loop.imprecise.log" 2>/dev/null; then
  echo "[PASS] nested_loop: the lost precision is announced on the console"
else
  echo "[FAIL] nested_loop: a fallback nobody announces leaves the false positives unattributable"
  FAILED=1
fi
# Recorded, not merely printed: the console is for whoever is watching, and
# report.json is what a pipeline and the next reader see.
for needle in '"status": "WARN"' '"kind": "path-insensitive"' '"block": "body"' '"function": "nested_walk"'; do
  if grep -qF "$needle" "$ROOT/tests/nested_loop/report.json" 2>/dev/null; then
    echo "[PASS] nested_loop: the fallback is recorded with $needle"
  else
    echo "[FAIL] nested_loop: report.json is missing $needle, so the loss is silent again"
    FAILED=1
  fi
done
# The note has to state the direction as well as the number: forgetting a guard
# only weakens the precondition, so the FAILs are the suspect verdicts and the
# PASSes are still proofs. Which budget was hit is the number to act on.
for needle in 'round budget (1 round)' 'false positive' 'A PASS still holds'; do
  if grep -qF "$needle" "$ROOT/tests/nested_loop/report.json" 2>/dev/null; then
    echo "[PASS] nested_loop: the note says $needle"
  else
    echo "[FAIL] nested_loop: the note must say $needle"
    FAILED=1
  fi
done
# And the false positives it explains are there: the two increments the default
# budget proves are flagged as soon as the guards are gone.
for resurrected in bb4 bb8; do
  if grep -qF "\"$resurrected\"" "$ROOT/tests/nested_loop/report.json" 2>/dev/null; then
    echo "[PASS] nested_loop: $resurrected is flagged once the guards are gone, as the note warns"
  else
    echo "[FAIL] nested_loop: $resurrected should be flagged with the budget cut to one round"
    FAILED=1
  fi
done
nl_precision=$(grep -c '"kind": "path-insensitive"' "$ROOT/tests/nested_loop/report.json" 2>/dev/null || true)
if [ "$nl_precision" = "3" ]; then
  echo "[PASS] nested_loop: every function that lost its guards is reported"
else
  echo "[FAIL] nested_loop: expected 3 path-insensitive findings (nested_walk, single_counter, main), got $nl_precision"
  FAILED=1
fi

run_table_case nested_loop
check_obligations nested_loop
check_witnesses nested_loop
check_locations nested_loop

run_table_case shift_amount
check_obligations shift_amount
check_witnesses shift_amount
check_locations shift_amount

# A target project pins its own toolchain, which is what every real project does.
# `cargo` is a rustup shim and follows that pin, and stable cargo rejects the `-Z`
# flag the verify command passes before any MIR is lowered: the run ends as a setup
# failure (exit 3) with no verdict, on code that compiles cleanly. Eight of the
# fifteen crates this was tried on failed that way. The fixture is therefore its own
# toolchain pin, and what it asserts is that the analysis ran at all:
# `unguarded_increment` is the counterexample that proves it, and the guarded one
# beside it must be proved. The closing quote in the check below matters, because
# `guarded_increment` is a substring of `unguarded_increment`.
run_table_case stable_pin
check_obligations stable_pin
check_witnesses stable_pin
check_locations stable_pin

if grep -qF '"function": "guarded_increment"' "$ROOT/tests/stable_pin/report.json" 2>/dev/null; then
  echo "[FAIL] stable_pin: guarded_increment cannot overflow and must be proved, but it is reported"
  FAILED=1
else
  echo "[PASS] stable_pin: the guarded increment is proved under the project's own toolchain pin"
fi

# Shifts, asserted in both directions, the same shape `guarded` uses, because the
# bug this fixture was written for was a *false positive*: the constant path called
# `200u8 << 1` an overflow, although Rust truncates there and never panics, while a
# symbolic shift amount was not checked at all. `shift_by` must therefore still be
# reported (with the amount named, not the shifted value), and the two functions
# whose shifts cannot panic must be proved and stay out of the report.
# The witness has to name the *amount*, not the shifted value: "by = 32" is the
# failing input, and `x` is irrelevant to why it panics.
if grep -qF '"input": "by (_2)"' "$ROOT/tests/shift_amount/report.json" 2>/dev/null &&
   grep -qF '"value": "32"' "$ROOT/tests/shift_amount/report.json" 2>/dev/null; then
  echo "[PASS] shift_amount: shift_by is reported with the out-of-range amount"
else
  echo "[FAIL] shift_amount: shift_by must be reported with its amount (by = 32)"
  FAILED=1
fi
for proved in truncate discard_high_bits; do
  if grep -q "\"$proved\"" "$ROOT/tests/shift_amount/report.json" 2>/dev/null; then
    echo "[FAIL] shift_amount: $proved cannot panic and must be proved, but it is reported"
    FAILED=1
  else
    echo "[PASS] shift_amount: $proved cannot panic, so it is absent from the report"
  fi
done

# The loop probe. A block inside a cycle used to get no path state, so the body's
# two increments were checked path-insensitively: the accumulator and the counter
# were free u8 symbols rather than the values the loop reaches, and `i + 1` was
# flagged at 255 â€” a value the loop's own guard, `i < steps`, makes unreachable.
# Cycles are now unrolled to their back edge and their loop-carried values
# widened, so a loop body is checked under its own guards while the result stays
# an over-approximation. Both directions are asserted, the way `guarded` and
# `shift_amount` assert their rule.
#
# The proofs first, because they are the new capability and they are absences:
# `accumulate`'s counter increment and `count_to`, the same loop shape with
# nothing else in it, must come back proved. The proof lives on the console, not
# in report.json â€” a proved obligation writes no report entry â€” so this reads the
# log the run above captured. A change that widened the guard away turns both
# back into the false `i = 255` FAILs this fixture was written for.
for proof in 'accumulate (bb3)' 'count_to (bb2)'; do
  if grep -qF "[PASS] Verified formal integrity: $proof" "$MEASURED/loop_fail.log" 2>/dev/null; then
    echo "[PASS] loop_fail: the loop guard proves $proof"
  else
    echo "[FAIL] loop_fail: $proof must be proved under the loop's guard, but the run does not prove it"
    FAILED=1
  fi
done
# Then the counterexample, which must survive the tightening: `acc = 250` over six
# trips walks 250..=255, so the sixth trip overflows a u8, and losing it would call
# a reachable overflow safe. What is asserted is what the report is allowed to
# claim: the operands of the failing addition and the accumulator's value, which
# the obligation *pins* to 255.
lf_fails=$(grep -c '"status": "FAIL"' "$ROOT/tests/loop_fail/report.json" 2>/dev/null || true)
if [ "$lf_fails" != "1" ]; then
  echo "[FAIL] loop_fail: expected exactly the accumulator's increment to be flagged, got $lf_fails FAIL(s)"
  FAILED=1
fi
for needle in '"function": "accumulate"' 'acc (_1) = 255'; do
  if grep -qF "$needle" "$ROOT/tests/loop_fail/report.json" 2>/dev/null; then
    echo "[PASS] loop_fail: report carries $needle"
  else
    echo "[FAIL] loop_fail: report is missing $needle â€” the loop body lost the reachable overflow"
    FAILED=1
  fi
done
# The guard's own symbols must *not* be in the witness. `i < steps` admits a
# range of values for both, so neither is pinned by the obligation: printing
# whichever one the model happened to pick would report a choice as a fact, and
# two runs of the same check could print two different numbers. The guard itself
# is what the obligation file carries (asserted below).
for absent in '"input": "steps (_2)"' '"input": "i (_3)"'; do
  if grep -qF "$absent" "$ROOT/tests/loop_fail/report.json" 2>/dev/null; then
    echo "[FAIL] loop_fail: report names $absent, a value the obligation does not pin â€” the model chose it"
    FAILED=1
  else
    echo "[PASS] loop_fail: the unforced guard symbol $absent is not reported"
  fi
done
# The obligation itself has to carry the guard: a FAIL whose precondition never
# mentions `i < steps` is the old path-insensitive one wearing a witness.
if grep -qF '(< mir_3 mir_2)' "$ROOT/tests/loop_fail/obligations/loop_fail/accumulate_bb2.smt2" 2>/dev/null; then
  echo "[PASS] loop_fail: the obligation is conditioned on the loop's guard (i < steps)"
else
  echo "[FAIL] loop_fail: the obligation must be conditioned on the loop's guard, but it is not"
  FAILED=1
fi
# And the value that guard forbids must be gone: `i = 255` was the witness the
# path-insensitive fallback reported for a state the loop cannot be in.
if grep -qF 'i (_3) = 255' "$ROOT/tests/loop_fail/report.json" 2>/dev/null; then
  echo "[FAIL] loop_fail: the report still names i = 255, a value the loop's guard forbids"
  FAILED=1
else
  echo "[PASS] loop_fail: the witness the guard forbids (i = 255) is gone"
fi

# The nested-loop probe. `loop_fail` above is a single cycle; this crate is the
# shape next door â€” an inner loop inside an outer one, with `continue` in the
# inner body and `break` in the outer â€” and it is where the loop abstraction used
# to stop working. Its body fell back to the path-insensitive state, because the
# *condition* dimension was never widened: `PathState::widen` forgot a cycle's
# loop-carried values but kept its condition, and at a join that condition is a
# disjunction over the paths that arrive, one of them the back edge carrying the
# previous round's condition â€” so on a nested header the condition re-wrapped
# itself once per round and grew (~8 KB, 33 KB, 123 KB, 383 KB over four rounds)
# instead of settling. The round cap fired and every increment was reported.
#
# `PathAnalysis::compute` now widens the condition too: past `MAX_LOOP_TRIPS` a
# cycle block's in-state is rebuilt from its *forward* contributions alone, so
# the back edge no longer re-feeds the block's condition into itself. A dropped
# back-edge disjunct is a subset of a forward one â€” the same guards on the same
# widened symbols â€” so the union is unchanged and the result is the fixpoint. The
# two guarded increments must therefore be proved now, the one real counterexample
# must stay, and the single-cycle control must stay proved.
for proof in 'nested_walk (bb4)' 'nested_walk (bb8)' 'single_counter (bb2)'; do
  if grep -qF "[PASS] Verified formal integrity: $proof" "$MEASURED/nested_loop.log" 2>/dev/null; then
    echo "[PASS] nested_loop: the guards prove $proof"
  else
    echo "[FAIL] nested_loop: $proof must be proved under its guards, but the run does not prove it"
    FAILED=1
  fi
done
# Then the one real counterexample. `nested_walk` has three arithmetic checks; a
# converged run proves the two the guards bound (`inner + 1` under `inner < cap`,
# `outer + 1` under `outer < limit`) and reports only `total + 1`, which
# accumulates for as long as both loops run and no guard bounds.
nl_fails=$(grep -c '"status": "FAIL"' "$ROOT/tests/nested_loop/report.json" 2>/dev/null || true)
if [ "$nl_fails" != "1" ]; then
  echo "[FAIL] nested_loop: expected only the accumulator's overflow to be flagged, got $nl_fails FAIL(s)"
  FAILED=1
fi
for needle in '"function": "nested_walk"' 'total (_3) = 255'; do
  if grep -qF "$needle" "$ROOT/tests/nested_loop/report.json" 2>/dev/null; then
    echo "[PASS] nested_loop: report carries $needle"
  else
    echo "[FAIL] nested_loop: report is missing $needle â€” the nested body lost the reachable overflow"
    FAILED=1
  fi
done
# The two proved increments must be *absent* â€” their reappearing is the
# regression this fixture now catches, and it is exactly what it reported before
# the condition was widened.
for absent in bb4 bb8; do
  if grep -qF "\"block\": \"$absent\"" "$ROOT/tests/nested_loop/report.json" 2>/dev/null; then
    echo "[FAIL] nested_loop: $absent is reported although its guard proves it â€” the condition is not widening"
    FAILED=1
  else
    echo "[PASS] nested_loop: $absent is proved, so it is absent from the report"
  fi
done
# And the counterexample's obligation must carry the guards from *both* headers:
# `outer < limit` from the outer header (`mir_4 < mir_1`) and `inner < cap` from
# the inner one (`mir_7 < mir_2`). A FAIL whose precondition never mentions them
# is the path-insensitive one this fixture started as.
if grep -qF '(< mir_4 mir_1)' "$ROOT/tests/nested_loop/obligations/nested_loop/nested_walk_bb6.smt2" 2>/dev/null &&
   grep -qF '(< mir_7 mir_2)' "$ROOT/tests/nested_loop/obligations/nested_loop/nested_walk_bb6.smt2" 2>/dev/null; then
  echo "[PASS] nested_loop: the counterexample is conditioned on both loop headers' guards"
else
  echo "[FAIL] nested_loop: the obligation must carry the outer and the inner guard, but it does not"
  FAILED=1
fi
# A proof is evidence too, so a proved site leaves an obligation that records
# `PASS (unsat)` â€” and re-discharging that file has to answer `unsat`, which is the
# proof itself rather than a promise that one exists. Before this, `obligations/`
# only held failures, so the artifacts let a reader check that a FAIL is real and
# not that a PASS is.
for proved in bb4 bb8; do
  proof="$ROOT/tests/nested_loop/obligations/nested_loop/nested_walk_$proved.smt2"
  if [ ! -f "$proof" ]; then
    echo "[FAIL] nested_loop: the proved site $proved emitted no obligation, so its proof cannot be re-checked"
    FAILED=1
    continue
  fi
  if ! grep -qF '; verdict:  PASS (unsat)' "$proof"; then
    echo "[FAIL] nested_loop: $proved is a proof but its obligation does not record PASS (unsat)"
    FAILED=1
    continue
  fi
  if [ -n "$Z3_BIN" ]; then
    answer="$("$Z3_BIN" -smt2 "$proof" 2>&1 | tr -d '\r\n ')"
    if [ "$answer" != "unsat" ]; then
      echo "[FAIL] nested_loop: $proved records PASS (unsat) but re-discharges to '$answer'"
      FAILED=1
      continue
    fi
    echo "[PASS] nested_loop: $proved re-discharged -> unsat (a checkable proof)"
  else
    echo "[SKIP] nested_loop: no z3 to re-discharge $proved"
  fi
done

run_table_case fold_fail
check_obligations fold_fail
check_witnesses fold_fail
check_locations fold_fail

# The folded path used to be a second-class FAIL: the operands were prose in
# `details`, no witness was recorded, and nothing was written under
# `obligations/` â€” so "every FAIL names an input" held only for the obligations
# Z3 decided, and a folded FAIL could not be re-checked at all. Both halves are
# asserted here: the structured witness, and the ground obligation that an
# independent solver re-discharges (check_obligations above proves the file is
# `sat`).
ff_fails=$(grep -c '"status": "FAIL"' "$ROOT/tests/fold_fail/report.json" 2>/dev/null || true)
if [ "$ff_fails" != "6" ]; then
  echo "[FAIL] fold_fail: expected all 6 folded failures to be reported, got $ff_fails"
  FAILED=1
else
  echo "[PASS] fold_fail: all 6 folded sites are reported"
fi
if grep -qF 'Counterexample found by constant folding' "$ROOT/tests/fold_fail/report.json" 2>/dev/null; then
  echo "[PASS] fold_fail: each folded FAIL says how it was decided"
else
  echo "[FAIL] fold_fail: the folded FAILs do not identify the path that found them"
  FAILED=1
fi

# A crate the compiler rejected is a *setup* failure. The wrapper used to throw
# rustc's exit status away, so this came out as exit 0 with an empty report.json:
# the analysis found no assertions in a body rustc never built, and "no findings"
# is not "nothing to check". This is the case that pins the distinction, and it
# has no report to compare against a golden â€” there is no verdict to record.
run_table_case broken_code

# The exit-code contract, both halves. `--deny-warn` may only ever *tighten* it:
# a run that is clean without the flag stays clean with it, and a counterexample
# is still fatal. The WARN half used to be covered only by the unit tests in
# `src/report.rs`, because no fixture produced an `unknown` verdict â€” so what is
# checked below is that the flag reaches the top-level gate at all, and that a
# misspelled flag is refused rather than silently leaving the stricter gate
# switched off. The undecided verdict itself is exercised end to end right after
# these three, against a real witness of it: `tests/undecided`.
# `safe_code`'s null and alignment checks are proved from pointer provenance now, so
# the strict gate has nothing left to reject in it; the WARN half is exercised on a
# real undecided obligation by `tests/undecided` below and on a counterexample by
# `overflow_fail`, so `--deny-warn` still has a case where it fails.
run_gate_case safe_code 0 "pointer checks are proved, so --deny-warn passes" --deny-warn
run_gate_case overflow_fail 1 "a counterexample is still fatal under --deny-warn" --deny-warn
run_gate_case safe_code 2 "a misspelled flag is refused, not ignored" --deny-warns
# The other half of that contract, end to end and with a real undecided
# obligation. `tests/undecided` is the crate that produces one: reaching the body
# of its `guarded_sum` needs `a * b == 2^61 - 1` with both factors above 1, and
# 2^61 - 1 is prime, so the body is unreachable â€” but only factorisation shows
# that, which is exactly what Z3's nonlinear *integer* arithmetic cannot do.
#
# That obligation needs a budget to be *undecided* rather than open-ended: with no
# budget Z3 does not answer `unknown`, it does not answer at all (measured, it is
# still running after 90 seconds). The budget is what turns that into the verdict
# this contract is written about. The crate's other obligation is a bound Z3 does
# decide, in about 0.4 s, so 5 s separates the two by more than an order of
# magnitude on either side.
#
# Both runs clear `target` first: cargo's incremental fingerprints do not notice
# that only the verifier's budget changed, so a warm target would mean no worker
# ran at all â€” and a gate with no findings passes, which would make this probe
# assert nothing.
und_z3="$Z3_BIN"
rm -rf "$ROOT/tests/undecided/target"
(cd "$ROOT/tests/undecided" && rm -f report.json && "$VERIFIER" keller verify --solver-timeout 5000 >"$MEASURED/undecided.log" 2>&1)
und_tolerated=$?
# Recorded in the same manifest the other cases go into, so the page is generated
# from a run that includes this one: the third field is how many `WARN`s the case
# is expected to carry while still exiting 0, and it is the number the generator
# holds the captured log to. Left out, the run's one undecided verdict would be
# invisible on a page whose every figure claims to come from this run.
printf '%s\t%s\t%s\n' undecided 0 1 >> "$MEASURED/manifest.tsv"
mark_consumed undecided
if [ "$und_tolerated" = "0" ]; then
  echo "[PASS] undecided: an undecided obligation does not fail the build by default"
else
  echo "[FAIL] undecided: expected exit 0 with a WARN and no FAIL, got $und_tolerated"
  FAILED=1
fi
# The verdict has to *be* the solver's, and it has to be recorded: classed,
# attributed, and never laundered into a counterexample the tool does not have.
for needle in '"status": "WARN"' '"kind": "overflow"' '"function": "guarded_sum"'; do
  if grep -qF "$needle" "$ROOT/tests/undecided/report.json" 2>/dev/null; then
    echo "[PASS] undecided: the obligation is recorded as $needle"
  else
    echo "[FAIL] undecided: report.json is missing $needle"
    FAILED=1
  fi
done
if grep -q '"status": "FAIL"' "$ROOT/tests/undecided/report.json" 2>/dev/null; then
  echo "[FAIL] undecided: an undecided obligation was reported as a counterexample"
  FAILED=1
else
  echo "[PASS] undecided: nothing undecided was laundered into a counterexample"
fi
# A verdict nothing announces is the failure mode this probe exists for: a WARN
# is the one verdict that must never be read as a pass, and it is only a finding
# if a reader is told about it.
if grep -qF '[WARN] Complex state in guarded_sum' "$MEASURED/undecided.log" 2>/dev/null; then
  echo "[PASS] undecided: the undecided verdict is announced on the console"
else
  echo "[FAIL] undecided: an undecided verdict that nothing announces reads like a pass"
  FAILED=1
fi
# The whole report, byte for byte: one WARN and nothing else. This is the
# determinism claim â€” the same obligation, run again, comes out the same rather
# than drifting with the machine's speed.
if diff -q "$ROOT/tests/goldens/undecided.json" "$ROOT/tests/undecided/report.json" >/dev/null 2>&1; then
  echo "[PASS] undecided: report.json matches its golden"
else
  echo "[FAIL] undecided: report.json differs from tests/goldens/undecided.json"
  diff -u "$ROOT/tests/goldens/undecided.json" "$ROOT/tests/undecided/report.json" | head -20
  FAILED=1
fi
# Re-discharge the obligation with the standalone solver under the same budget.
# The undecided verdict has to be the solver's, not the tool's story about the
# solver â€” and no pinned file may exist, because `unknown` produces no model, so
# there is no witness to pin.
if [ -n "$und_z3" ]; then
  und_out="$("$und_z3" -smt2 -T:5 "$ROOT/tests/undecided/obligations/undecided/guarded_sum_bb6.smt2" 2>&1)"
  if printf '%s' "$und_out" | grep -qiE 'timeout|unknown'; then
    echo "[PASS] undecided: the standalone solver cannot decide it either ($(printf '%s' "$und_out" | tr -d '\r\n'))"
  else
    echo "[FAIL] undecided: the obligation re-discharged to '$und_out', so the undecided verdict is not the solver's"
    FAILED=1
  fi
else
  echo "[SKIP] undecided: no z3 binary to re-discharge the undecided obligation"
fi
if [ -f "$ROOT/tests/undecided/obligations/undecided/guarded_sum_bb6.witness.smt2" ]; then
  echo "[FAIL] undecided: an undecided verdict has no model, so it must pin no witness"
  FAILED=1
else
  echo "[PASS] undecided: no witness is pinned for a verdict with no model"
fi
rm -rf "$ROOT/tests/undecided/target"
(cd "$ROOT/tests/undecided" && "$VERIFIER" keller verify --solver-timeout 5000 --deny-warn >"$MEASURED/undecided.denywarn.log" 2>&1)
und_denied=$?
if [ "$und_denied" = "1" ]; then
  echo "[PASS] undecided: the same obligation is fatal under --deny-warn"
else
  echo "[FAIL] undecided: expected exit 1 under --deny-warn, got $und_denied"
  FAILED=1
fi
if grep -qF 'are undecided (WARN) and --deny-warn was given' "$MEASURED/undecided.denywarn.log" 2>/dev/null; then
  echo "[PASS] undecided: the gate says which verdict it failed on"
else
  echo "[FAIL] undecided: the gate failed without naming the undecided finding"
  FAILED=1
fi

# ===========================================================================
#  A run that stops making progress stops the run.
#
#  K3: a crate can wedge rather than finish. `tests/undecided` with no solver
#  budget at all is an unbounded Z3 call: the `workspace-wrapper` that hosts Z3
#  is CPU-bound, not blocked. `op-gate` also wedges on the default budget (run
#  2026-10-13, stopped at the --run-timeout cap), so a budget does not bound
#  every wedge; its report is not reproducible from this repository. What this case pins is the
#  contract a pipeline can rely on whatever the solver does: the run stops instead of waiting, it says which crate was still in
#  flight, it records itself as a stopped run and not as a verdict, it publishes
#  nothing over the last completed report, and the process tree it started is gone.
#
#  That last one is a failure this repository has already shipped: `Stop-Process`
#  on the top-level verifier leaves the workspace wrappers and their cargo alive,
#  and the sweep script that measured these crates leaked one such tree per
#  attempt. A kill of the direct child would pass every other check here.
#
#  `--solver-timeout 0` is how this case reaches the wedge: with a budget the same
#  crate reports `unknown` and exits, which is asserted just above. That caller has
#  asked for no solver limit, and is exactly the caller a wall-clock cap has to
#  protect.
# ===========================================================================
rm -rf "$ROOT/tests/undecided/target"
(cd "$ROOT/tests/undecided" && rm -f report.json)
# `ps -W` is what Git Bash needs to see native processes and a POSIX `ps` refuses
# it, so which spelling was used also decides whether the stricter check below can
# name a survivor from *this* case, or whether the count is the whole assertion.
if ps -W >/dev/null 2>&1; then
  PS_ALL="ps -W"
  PS_PATHS=1
else
  PS_ALL="ps -A"
  PS_PATHS=0
fi
# `|| true`: `grep -c` prints 0 and exits 1 when nothing matched, which is the case
# this expects, and an exit status that failed the suite would be the wrong report.
survivors() { $PS_ALL 2>/dev/null | grep -cE 'workspace-wrapper|cargo(\.exe)?$' || true; }
survivors_before="$(survivors)"
ls "$ROOT/tests/undecided/.keller/runs" > "$MEASURED/undecided.runs.before" 2>/dev/null
cap_seconds=20
cap_started="$(date +%s)"
(cd "$ROOT/tests/undecided" && "$VERIFIER" keller verify --solver-timeout 0 --run-timeout "$cap_seconds" >"$MEASURED/run_timeout.log" 2>&1)
cap_exit=$?
cap_elapsed=$(( $(date +%s) - cap_started ))
if [ "$cap_exit" = "3" ]; then
  echo "[PASS] run_timeout: a run stopped at the cap reaches no verdict (exit 3)"
else
  echo "[FAIL] run_timeout: expected exit 3 from a run stopped at the cap, got $cap_exit"
  FAILED=1
fi
# Slack over the cap: the deadline is polled, and a cold build may still be running
# when it fires. What this rules out is the failure that matters -- a run that
# ignores the cap and keeps going.
if [ "$cap_elapsed" -le $((cap_seconds + 15)) ]; then
  echo "[PASS] run_timeout: it stopped after ${cap_elapsed}s for a ${cap_seconds}s cap"
else
  echo "[FAIL] run_timeout: ran for ${cap_elapsed}s under a ${cap_seconds}s cap"
  FAILED=1
fi
for needle in "Stopped at the --run-timeout cap (${cap_seconds} s)" "in flight: undecided"; do
  if grep -qF "$needle" "$MEASURED/run_timeout.log" 2>/dev/null; then
    echo "[PASS] run_timeout: the stopped run reports $needle"
  else
    echo "[FAIL] run_timeout: the stopped run does not report $needle"
    FAILED=1
  fi
done
# Evidence from this run and no other: a record that predates the case would pass
# the two checks below while reporting on a different run entirely.
cap_run="$(ls -t "$ROOT/tests/undecided/.keller/runs" | head -1)"
if grep -qxF "$cap_run" "$MEASURED/undecided.runs.before" 2>/dev/null; then
  echo "[FAIL] run_timeout: the newest run directory predates this case, so it is not its evidence"
  FAILED=1
fi
cap_record="$ROOT/tests/undecided/.keller/runs/$cap_run/report.run.json"
if grep -qF '"status": "run-timeout"' "$cap_record" 2>/dev/null; then
  echo "[PASS] run_timeout: the stopped run records itself as run-timeout"
else
  echo "[FAIL] run_timeout: no run record at '$cap_record' says run-timeout"
  FAILED=1
fi
if grep -qE '"status": "(gate-passed|gate-failed|PASS|FAIL|WARN)"' "$cap_record" 2>/dev/null; then
  echo "[FAIL] run_timeout: a stopped run recorded a verdict it never reached"
  FAILED=1
else
  echo "[PASS] run_timeout: no verdict is recorded for a run that reached none"
fi
# What this rules out is a truncated report replacing the last completed one: a
# reader of `report.json` would be shown "no findings" for a crate the analysis
# never finished, which is indistinguishable from a clean run.
if [ -f "$ROOT/tests/undecided/report.json" ]; then
  echo "[FAIL] run_timeout: the stopped run published a report over the last completed one"
  FAILED=1
else
  echo "[PASS] run_timeout: nothing was published in place of the last completed report"
fi
survivors_after="$(survivors)"
if [ "$survivors_after" -le "$survivors_before" ]; then
  echo "[PASS] run_timeout: the stopped run left no process behind ($survivors_before before, $survivors_after after)"
else
  echo "[FAIL] run_timeout: the cap left $((survivors_after - survivors_before)) process(es) running:"
  $PS_ALL 2>/dev/null | grep -E 'workspace-wrapper|cargo(\.exe)?$' | head -5
  FAILED=1
fi
# Stricter, and only meaningful where `ps` prints paths: a survivor from *this*
# case. On a POSIX `ps` the count above is the whole assertion, which is why this
# half says which of the two ran.
if [ "$PS_PATHS" = "1" ]; then
  if $PS_ALL 2>/dev/null | grep -q 'undecided'; then
    echo "[FAIL] run_timeout: a process from this case is still running:"
    $PS_ALL 2>/dev/null | grep 'undecided' | head -3
    FAILED=1
  else
    echo "[PASS] run_timeout: no process from this case survived the cap"
  fi
fi
rm -rf "$ROOT/tests/undecided/target"

# ===========================================================================
#  A store the key scheme cannot name must retire what it recorded.
#
#  `place_key` understands field projections only, so `*r = 250` names no place,
#  and the store used to be ignored entirely â€” leaving `v` holding the value the
#  copy out of `s + x` had put there, which the guard bounds by 8. `v + 240` was
#  then *proved*: the tool printed `[PASS] ref_write (bb3)` for a function that
#  panics. Retiring the storage the store may have reached turns that into the
#  honest verdict â€” undecided, with the store named â€” and both halves are asserted
#  below: the obligation is never proved again, and the control whose store the
#  analysis *can* name stays proved, because a fix that gave up on every place a
#  reference touches would report that one, which is a false positive bought for
#  nothing.
#
#  The emitted obligation re-discharges to `sat`. The counterexample really is
#  there (`v = 250` reaches `v + 240`); what the tool declines to do is *claim* it,
#  because the value it would have to name came out of storage the analysis cannot
#  follow. That is the policy `tests/undecided` pins for a solver that cannot
#  decide, applied to the case where the plaintext analysis cannot attribute, and
#  it is why this case exits 0 with a WARN rather than 1 with a counterexample.
rm -rf "$ROOT/tests/unnamed_write/target"
(cd "$ROOT/tests/unnamed_write" && rm -f report.json && "$VERIFIER" keller verify >"$MEASURED/unnamed_write.log" 2>&1)
uw_exit=$?
# Third field: this case prints exactly one `[WARN] â€¦ (bb)` line while exiting 0,
# and the page generator holds the run to that number.
printf '%s\t%s\t%s\n' unnamed_write 0 1 >> "$MEASURED/manifest.tsv"
mark_consumed unnamed_write
if [ "$uw_exit" = "0" ]; then
  echo "[PASS] unnamed_write: a check answered from storage this analysis cannot follow does not fail the build by default"
else
  echo "[FAIL] unnamed_write: expected exit 0 with a WARN and no FAIL, got $uw_exit"
  FAILED=1
fi
# The verdict has to be the honest one *and* it has to name the store: the point
# of retiring the storage is that a reader can see which write cost the value.
for needle in '"status": "WARN"' '"function": "ref_write"' 'is written through a projection this analysis cannot name'; do
  if grep -qF "$needle" "$ROOT/tests/unnamed_write/report.json" 2>/dev/null; then
    echo "[PASS] unnamed_write: the obligation is recorded as $needle"
  else
    echo "[FAIL] unnamed_write: report.json is missing $needle"
    FAILED=1
  fi
done
if grep -q '"status": "FAIL"' "$ROOT/tests/unnamed_write/report.json" 2>/dev/null; then
  echo "[FAIL] unnamed_write: a store the analysis cannot attribute was laundered into a counterexample"
  FAILED=1
else
  echo "[PASS] unnamed_write: the decline is not laundered into a counterexample"
fi
# The regression itself, in the direction it failed: `ref_write (bb3)` is the
# obligation that used to be printed as `[PASS]` from the overwritten value. This
# is the assertion that reddens if the stale fact comes back.
if grep -qF '[PASS] Verified formal integrity: ref_write (bb3)' "$MEASURED/unnamed_write.log" 2>/dev/null; then
  echo "[FAIL] unnamed_write: ref_write (bb3) is proved again â€” the stale value is back"
  FAILED=1
else
  echo "[PASS] unnamed_write: ref_write (bb3) is not proved from the value the store destroyed"
fi
if grep -qF '[WARN] Undecided in ref_write (bb3)' "$MEASURED/unnamed_write.log" 2>/dev/null; then
  echo "[PASS] unnamed_write: the undecided verdict is announced on the console"
else
  echo "[FAIL] unnamed_write: a verdict nothing announces reads like a pass"
  FAILED=1
fi
# The precision half. The named store retires and re-derives its value, and the
# guard still bounds it, so both of the control's obligations stay proved.
for expected in 'named_store_is_kept (bb2)' 'named_store_is_kept (bb3)'; do
  if grep -qF "[PASS] Verified formal integrity: $expected" "$MEASURED/unnamed_write.log" 2>/dev/null; then
    echo "[PASS] unnamed_write: the named store keeps its proof ($expected)"
  else
    echo "[FAIL] unnamed_write: $expected is no longer proved â€” the retirement gave up a value it could name"
    FAILED=1
  fi
done
check_goldens unnamed_write
# The formula is satisfiable, and that is the whole point of the case: the
# counterexample is real and the tool declines to name it, rather than inventing
# an input or quietly keeping the one the store destroyed.
if [ -n "$Z3_BIN" ]; then
  uw_out="$("$Z3_BIN" -smt2 "$ROOT/tests/unnamed_write/obligations/unnamed_write/ref_write_bb3.smt2" 2>&1)"
  uw_answer="$(printf '%s' "$uw_out" | tr -d '\r\n')"
  if [ "$uw_answer" = "sat" ]; then
    echo "[PASS] unnamed_write: the undecided obligation re-discharges to sat â€” the counterexample is there, the attribution is what is missing"
  else
    echo "[FAIL] unnamed_write: expected the obligation to re-discharge to sat, got '$uw_answer'"
    FAILED=1
  fi
else
  echo "[SKIP] unnamed_write: no z3 binary to re-discharge the obligation"
fi
# A verdict that names no input pins no witness, for the same reason `undecided`
# does not: there is no model the tool is willing to stand behind.
if [ -f "$ROOT/tests/unnamed_write/obligations/unnamed_write/ref_write_bb3.witness.smt2" ]; then
  echo "[FAIL] unnamed_write: an undecided verdict names no input, so it must pin no witness"
  FAILED=1
else
  echo "[PASS] unnamed_write: no witness is pinned for a verdict that names no input"
fi
# And the gate, so the finding is not merely cosmetic: a project that asks for it
# gets a failed build out of the same obligation.
rm -rf "$ROOT/tests/unnamed_write/target"
(cd "$ROOT/tests/unnamed_write" && "$VERIFIER" keller verify --deny-warn >"$MEASURED/unnamed_write.denywarn.log" 2>&1)
uw_denied=$?
if [ "$uw_denied" = "1" ]; then
  echo "[PASS] unnamed_write: the undecided obligation is fatal under --deny-warn"
else
  echo "[FAIL] unnamed_write: expected exit 1 under --deny-warn, got $uw_denied"
  FAILED=1
fi
if grep -qF 'are undecided (WARN) and --deny-warn was given' "$MEASURED/unnamed_write.denywarn.log" 2>/dev/null; then
  echo "[PASS] unnamed_write: the gate says which verdict it failed on"
else
  echo "[FAIL] unnamed_write: the gate failed without naming the undecided finding"
  FAILED=1
fi

# ===========================================================================
#  Where a write is *spelled*, not what it writes.
#
#  `apply_assign` was the only thing that retired anything, and the call
#  terminator retired only its own destination, so every other way a body this
#  analysis does not read can write â€” a callee's `&mut` argument, drop glue, an
#  intrinsic's store, an asm block's output â€” was invisible. Four functions in
#  `tests/statement_writes` panic on every input their guard admits and all four
#  were *proved* before the rule was generalised (A18). Each is asserted
#  in the direction it failed: not proved, not laundered into a counterexample,
#  but undecided *with the write named*.
#
#  `asm_writes` pins the walk as well as the write. The reachability walk did not
#  know the `InlineAsm` terminator, so the assertion after the asm block was never
#  reached â€” and a walk that stops cannot report what it skipped, because the
#  coverage line counts the assertions it *encountered*. The obligation file for
#  that block is the evidence it is inside the walk now.
#
#  The two controls are the precision half. A callee's pointer names one local when
#  the body's own MIR says so (`unrelated_local_is_kept`), and a call handed no
#  pointer into this body's storage cannot have written the value at all
#  (`no_pointer_argument_is_kept`). Both must stay proved: retiring everything a
#  call touches would buy the four WARNs above with two false positives.
rm -rf "$ROOT/tests/statement_writes/target"
(cd "$ROOT/tests/statement_writes" && rm -f report.json && "$VERIFIER" keller verify >"$MEASURED/statement_writes.log" 2>&1)
sw_exit=$?
# Third field: four `[WARN] â€¦ (bb)` lines while exiting 0, which the page generator
# holds the run to. The asm shape does not exist off x86-64, where it is three.
if [ "$SW_ASM" = "1" ]; then
  printf '%s\t%s\t%s\n' statement_writes 0 4 >> "$MEASURED/manifest.tsv"
else
  printf '%s\t%s\t%s\n' statement_writes 0 3 >> "$MEASURED/manifest.tsv"
fi
mark_consumed statement_writes
if [ "$sw_exit" = "0" ]; then
  echo "[PASS] statement_writes: a check answered from storage an unread body wrote does not fail the build by default"
else
  echo "[FAIL] statement_writes: expected exit 0 with WARNs and no FAIL, got $sw_exit"
  FAILED=1
fi
# The finding has to name the write that cost the value, one per shape.
for needle in 'the call to `bump` is given a pointer into this body' 'a drop here runs glue this analysis does not read' 'is given a pointer into this body' 'is written by an inline asm block'; do
  if grep -qF "$needle" "$ROOT/tests/statement_writes/report.json" 2>/dev/null; then
    echo "[PASS] statement_writes: the retirement names the write ($needle)"
  else
    echo "[FAIL] statement_writes: report.json does not name the write ($needle)"
    FAILED=1
  fi
done
if grep -q '"status": "FAIL"' "$ROOT/tests/statement_writes/report.json" 2>/dev/null; then
  echo "[FAIL] statement_writes: a write the analysis cannot attribute was laundered into a counterexample"
  FAILED=1
else
  echo "[PASS] statement_writes: the declines are not laundered into counterexamples"
fi
# The regression itself, in the direction it failed: each of these obligations was
# printed as `[PASS]` from the value the unread body had already overwritten.
for proved in 'call_writes (bb4)' 'drop_writes (bb4)' 'intrinsic_writes (bb4)'; do
  if grep -qF "[PASS] Verified formal integrity: $proved" "$MEASURED/statement_writes.log" 2>/dev/null; then
    echo "[FAIL] statement_writes: $proved is proved again â€” the value the write destroyed is back"
    FAILED=1
  else
    echo "[PASS] statement_writes: $proved is not proved from the value the write destroyed"
  fi
  if grep -qF "[WARN] Undecided in $proved" "$MEASURED/statement_writes.log" 2>/dev/null; then
    echo "[PASS] statement_writes: the undecided verdict for $proved is announced on the console"
  else
    echo "[FAIL] statement_writes: $proved is undecided but nothing announces it, which reads like a pass"
    FAILED=1
  fi
done
# The assertion after the asm block is inside the walk now. Before the walk knew
# the `InlineAsm` terminator this file did not exist: nothing had reached the
# block, so the summary neither checked its assertion nor counted it as missed.
if [ "$SW_ASM" = "1" ]; then
  if [ -f "$ROOT/tests/statement_writes/obligations/statement_writes/asm_writes_bb4.smt2" ]; then
    echo "[PASS] statement_writes: the assertion after the asm block is reached, so it is checked"
  else
    echo "[FAIL] statement_writes: the walk stopped at the asm block â€” nothing follows it in the analysis"
    FAILED=1
  fi
else
  echo "[SKIP] statement_writes: no x86-64 asm in this build, so the asm shape is not in the crate"
fi
for kept in 'unrelated_local_is_kept (bb4)' 'no_pointer_argument_is_kept (bb4)'; do
  if grep -qF "[PASS] Verified formal integrity: $kept" "$MEASURED/statement_writes.log" 2>/dev/null; then
    echo "[PASS] statement_writes: the control keeps its proof ($kept)"
  else
    echo "[FAIL] statement_writes: $kept is no longer proved â€” a value no unread body could have written was given up"
    FAILED=1
  fi
done
check_obligations statement_writes
check_goldens statement_writes
check_environment statement_writes
# The formula is satisfiable, and that is the point of the case: the counterexample
# is real (`bump` really does write 250) and the tool declines to *name* it, rather
# than inventing an input or quietly keeping the one the callee destroyed.
if [ -n "$Z3_BIN" ]; then
  sw_out="$("$Z3_BIN" -smt2 "$ROOT/tests/statement_writes/obligations/statement_writes/call_writes_bb4.smt2" 2>&1)"
  sw_answer="$(printf '%s' "$sw_out" | tr -d '\r\n')"
  if [ "$sw_answer" = "sat" ]; then
    echo "[PASS] statement_writes: the undecided obligation re-discharges to sat â€” the counterexample is there, the attribution is what is missing"
  else
    echo "[FAIL] statement_writes: expected the obligation to re-discharge to sat, got '$sw_answer'"
    FAILED=1
  fi
else
  echo "[SKIP] statement_writes: no z3 binary to re-discharge the obligation"
fi
if [ -f "$ROOT/tests/statement_writes/obligations/statement_writes/call_writes_bb4.witness.smt2" ]; then
  echo "[FAIL] statement_writes: an undecided verdict names no input, so it must pin no witness"
  FAILED=1
else
  echo "[PASS] statement_writes: no witness is pinned for a verdict that names no input"
fi
# And the gate, so the finding is not merely cosmetic.
rm -rf "$ROOT/tests/statement_writes/target"
(cd "$ROOT/tests/statement_writes" && "$VERIFIER" keller verify --deny-warn >"$MEASURED/statement_writes.denywarn.log" 2>&1)
sw_denied=$?
if [ "$sw_denied" = "1" ]; then
  echo "[PASS] statement_writes: the undecided obligations are fatal under --deny-warn"
else
  echo "[FAIL] statement_writes: expected exit 1 under --deny-warn, got $sw_denied"
  FAILED=1
fi
if grep -qF 'are undecided (WARN) and --deny-warn was given' "$MEASURED/statement_writes.denywarn.log" 2>/dev/null; then
  echo "[PASS] statement_writes: the gate says which verdict it failed on"
else
  echo "[FAIL] statement_writes: the gate failed without naming the undecided findings"
  FAILED=1
fi

# ===========================================================================
#  The panics a report does not mention (A6).
#
#  Every case above is a proof about an *assertion* the compiler inserted.
#  `unwrap`, `expect`, `panic!`, `unreachable!` and the failure path of a user
#  `assert!` are not assertions: they are calls into another crate whose body this
#  analysis does not read, and before this pass they were walked past in silence.
#  `tests/potency_probe` below asserts that a crate that cannot panic reports
#  nothing; this is the other half of that sentence, and it is what makes the
#  probe's clean report mean anything: a crate full of `unwrap`s produced `[]` as
#  well, and only these counts tell the two apart.
#
#  Asserted in the direction each half could fail:
#
#    * six shapes, one per function, each named on the console with the callee,
#      and exactly six - so a call to a *local* function is named where the panic
#      is rather than again at every caller, and the two controls that cannot
#      panic add nothing.
#    * `assert_eq!` reaches `core::panicking::assert_failed`, which is not the
#      `panic_fmt` a bare `panic!` lowers to; the two are told apart because they
#      say different things about what was assumed.
#    * no FAIL, and no change to the assertion counts: a panic site is its own
#      line in the summary and its own field in coverage, never summed with the
#      assertions it is not.
#    * the guard an `assert!` provides still proves the arithmetic after it, and
#      the bounds check in the control is still decided rather than reported.
#    * `--deny-warn` is what makes one fatal, which is why every exit code that
#      existed before this pass is unchanged.
# ---------------------------------------------------------------------------
rm -rf "$ROOT/tests/panic_sites/target"
(cd "$ROOT/tests/panic_sites" && rm -f report.json && "$VERIFIER" keller verify >"$MEASURED/panic_sites.log" 2>&1)
ps_exit=$?
# Third field: this case prints six named panic sites while exiting 0.
printf '%s\t%s\t%s\n' panic_sites 0 6 >> "$MEASURED/manifest.tsv"
mark_consumed panic_sites
PS_LOG="$MEASURED/panic_sites.log"
PS_REPORT="$ROOT/tests/panic_sites/report.json"
if [ "$ps_exit" = "0" ]; then
  echo "[PASS] panic_sites: a crate whose functions are full of unwraps does not fail the build by default"
else
  echo "[FAIL] panic_sites: expected exit 0 with WARNs and no FAIL, got $ps_exit"
  FAILED=1
fi
for shape in unwrapped expected explicit_program_panic never_reached guarded_by_assert asserted_equal; do
  if grep -qF "[WARN] Unmodelled panic site in $shape (" "$PS_LOG" 2>/dev/null; then
    echo "[PASS] panic_sites: $shape's panic site is named on the console"
  else
    echo "[FAIL] panic_sites: $shape's panic site is not reported - a path nothing says anything about reads like a proved one"
    FAILED=1
  fi
done
ps_named=$(grep -c '^\[WARN\] Unmodelled panic site in ' "$PS_LOG" 2>/dev/null || true)
ps_named=${ps_named:-0}
if [ "$ps_named" = "6" ]; then
  echo "[PASS] panic_sites: exactly six sites are named, so a local callee is not counted again at its caller"
else
  echo "[FAIL] panic_sites: expected 6 named panic sites, got $ps_named"
  FAILED=1
fi
# The report carries the same six, with the callee in the details: the console
# line says a site exists, the artifact has to say through what.
for needle in '"kind": "unmodelled-panic"' 'std::option::Option' 'std::result::Result' 'core::panicking::assert_failed' '"status": "WARN"'; do
  if grep -qF "$needle" "$PS_REPORT" 2>/dev/null; then
    echo "[PASS] panic_sites: the finding is recorded as $needle"
  else
    echo "[FAIL] panic_sites: report.json is missing $needle"
    FAILED=1
  fi
done
ps_recorded=$(grep -c '"kind": "unmodelled-panic"' "$PS_REPORT" 2>/dev/null || true)
ps_recorded=${ps_recorded:-0}
if [ "$ps_recorded" = "6" ]; then
  echo "[PASS] panic_sites: the report records all six, so the console and the artifact cannot disagree"
else
  echo "[FAIL] panic_sites: report.json records $ps_recorded panic sites, not 6"
  FAILED=1
fi
# A panic site is not a counterexample: nothing was proved false, so the byte the
# goldens pin must not contain a FAIL.
if grep -q '"status": "FAIL"' "$PS_REPORT" 2>/dev/null; then
  echo "[FAIL] panic_sites: a call whose body is not read was reported as a counterexample"
  FAILED=1
else
  echo "[PASS] panic_sites: an unread callee is not laundered into a counterexample"
fi
# Counted apart from the assertions, in the two places a reader meets the number.
if grep -q '"panic_sites": 6' "$ROOT/tests/panic_sites/report.run.json" 2>/dev/null; then
  echo "[PASS] panic_sites: coverage records the six beside the assertions rather than inside them"
else
  echo "[FAIL] panic_sites: report.run.json does not carry panic_sites: 6"
  FAILED=1
fi
for needle in '[Keller]   6 call site(s) that can panic were not modelled (WARN)' '  Panic sites not modelled: 6'; do
  if grep -qF "$needle" "$PS_LOG" 2>/dev/null; then
    echo "[PASS] panic_sites: the run announces $needle"
  else
    echo "[FAIL] panic_sites: nothing announces '$needle'"
    FAILED=1
  fi
done
# The half that keeps the count honest: the fixture has two assertions, and the
# summary has to say two. Six panic sites added to them would be the same
# overstatement as reporting nothing, from the other side.
if grep -qF '  Total assertions checked: 2' "$PS_LOG" 2>/dev/null; then
  echo "[PASS] panic_sites: the summary counts the two assertions, not the six call sites"
else
  echo "[FAIL] panic_sites: the assertion count has absorbed the panic sites"
  FAILED=1
fi
# The precision half: an `assert!` in the body is still believed as a guard, and
# the one control with an obligation still has it decided. A rule that reported
# every call it cannot read would show up here instead.
for expected in 'guarded_by_assert (bb1)' 'control_bounds_check (bb1)'; do
  if grep -qF "[PASS] Verified formal integrity: $expected" "$PS_LOG" 2>/dev/null; then
    echo "[PASS] panic_sites: $expected is still decided"
  else
    echo "[FAIL] panic_sites: $expected is no longer proved - the panic-site pass cost a verdict"
    FAILED=1
  fi
done
check_goldens panic_sites
# Advisory by default, fatal on request: the same contract `undecided` pins for a
# solver that cannot answer, applied to a path nothing here reads.
rm -rf "$ROOT/tests/panic_sites/target"
(cd "$ROOT/tests/panic_sites" && "$VERIFIER" keller verify --deny-warn >"$MEASURED/panic_sites.denywarn.log" 2>&1)
ps_denied=$?
if [ "$ps_denied" = "1" ]; then
  echo "[PASS] panic_sites: an unmodelled panic site is fatal under --deny-warn"
else
  echo "[FAIL] panic_sites: expected exit 1 under --deny-warn, got $ps_denied"
  FAILED=1
fi
if grep -qF 'are undecided (WARN) and --deny-warn was given' "$MEASURED/panic_sites.denywarn.log" 2>/dev/null; then
  echo "[PASS] panic_sites: the gate says which verdict it failed on"
else
  echo "[FAIL] panic_sites: the gate failed without naming the WARN it failed on"
  FAILED=1
fi

# ===========================================================================
#  The panics this analysis *can* decide (A6, the other half).
#
#  `tests/panic_sites` pins the receiver-is-a-parameter case, which has to stay
#  a disclosure. This fixture pins the one where the receiver's variant is
#  known: a safe variant must leave no finding at all (the decided count is its
#  only trace), and the variant the callee cannot return must be a FAIL rather
#  than a WARN, with no solver call and no invented witness.
# ---------------------------------------------------------------------------
run_table_case coroutine_state
# S6: the resume-state guards of an async fn are decided, not counted as
# unsupported. They are named on the console only - a PASS carries an event, but
# report.json records FAIL and WARN - so the log is what has to be read.
CS_LOG="$MEASURED/coroutine_state.log"
for cs_line in '[PASS] coroutine resume state in one_yield::{closure#0} (bb12): not `2` (poisoned)' \
               '[PASS] coroutine resume state in one_yield::{closure#0} (bb13): not `1` (returned)'; do
  if grep -qF "$cs_line" "$CS_LOG" 2>/dev/null; then
    echo "[PASS] coroutine_state: $cs_line"
  else
    echo "[FAIL] coroutine_state: a resume-state assertion is no longer decided, so S6 has regressed"
    FAILED=1
  fi
done
if grep -qE 'ResumedAfter(Return|Panic|Drop)' "$CS_LOG" 2>/dev/null; then
  echo "[FAIL] coroutine_state: a resume-state assertion is back among the unsupported kinds"
  FAILED=1
else
  echo "[PASS] coroutine_state: no resume-state assertion is counted as unsupported"
fi
if grep -qF 'Coverage: 2 of 2 encountered assertion(s) decided' "$CS_LOG" 2>/dev/null; then
  echo "[PASS] coroutine_state: the coverage line counts both assertions as supported"
else
  echo "[FAIL] coroutine_state: the coverage line no longer counts the resume-state assertions as supported"
  FAILED=1
fi
if grep -q "Path-insensitive fallback in one_yield" "$CS_LOG" 2>/dev/null; then
  echo "[FAIL] coroutine_state: the async body fell back to path-insensitivity again"
  FAILED=1
else
  echo "[PASS] coroutine_state: the async body reaches its fixpoint"
fi
run_table_case async_drop_state
# S6's third kind, checked against real MIR rather than by symmetry. The
# async-drop guard is reached through suspension states, which a resume may
# arrive in, so the invariant does not decide it: it must be named as an
# unsupported assertion and never reported as a counterexample.
AD_LOG="$MEASURED/async_drop_state.log"
for ad_line in '[PASS] coroutine resume state in holds_guard::{closure#0} (bb23): not `1` (returned)' \
               '[PASS] coroutine resume state in holds_guard::{closure#0} (bb22): not `2` (poisoned)'; do
  if grep -qF "$ad_line" "$AD_LOG" 2>/dev/null; then
    echo "[PASS] async_drop_state: $ad_line"
  else
    echo "[FAIL] async_drop_state: the kinds the resume invariant covers are no longer decided here"
    FAILED=1
  fi
done
if grep -qF 'ResumedAfterDrop' "$AD_LOG" 2>/dev/null; then
  echo "[PASS] async_drop_state: the async-drop guard is named as an unsupported assertion"
else
  echo "[FAIL] async_drop_state: ResumedAfterDrop is not reported at all"
  FAILED=1
fi
if grep -qF '"status": "FAIL"' "$ROOT/tests/async_drop_state/report.json" 2>/dev/null; then
  echo "[FAIL] async_drop_state: the async-drop guard is reported as a counterexample, which real MIR says it is not"
  FAILED=1
else
  echo "[PASS] async_drop_state: no counterexample is reported for the async-drop guard"
fi
run_table_case panic_decided
PD_LOG="$MEASURED/panic_decided.log"
PD_REPORT="$ROOT/tests/panic_decided/report.json"
PD_RUN="$ROOT/tests/panic_decided/report.run.json"
for settled in known_none known_err known_ok_unwrap_err; do
  if grep -qF "[FAIL] Decided panic site in $settled (" "$PD_LOG" 2>/dev/null; then
    echo "[PASS] panic_decided: $settled's known failing variant is a counterexample"
  else
    echo "[FAIL] panic_decided: $settled was not decided even though its receiver variant is known"
    FAILED=1
  fi
done
for safe in known_some known_ok known_err_unwrap_err dead_site; do
  if grep -qF "\"function\": \"$safe\"" "$PD_REPORT" 2>/dev/null; then
    echo "[FAIL] panic_decided: $safe is reported even though its site was decided safe"
    FAILED=1
  else
    echo "[PASS] panic_decided: $safe's site is decided safe and leaves no finding"
  fi
done
pd_decided=$(grep -c '"kind": "decided-panic"' "$PD_REPORT" 2>/dev/null || true)
pd_decided=${pd_decided:-0}
if [ "$pd_decided" = "3" ]; then
  echo "[PASS] panic_decided: exactly the three failing variants are counterexamples"
else
  echo "[FAIL] panic_decided: expected 3 decided panics, got $pd_decided"
  FAILED=1
fi
if grep -q '"panic_sites": 0' "$PD_RUN" 2>/dev/null && \
   grep -q '"panic_sites_decided": 7' "$PD_RUN" 2>/dev/null; then
  echo "[PASS] panic_decided: coverage separates the seven decided sites from the undecided ones"
else
  echo "[FAIL] panic_decided: report.run.json does not carry panic_sites: 0 and panic_sites_decided: 7"
  FAILED=1
fi
for needle in \
  '[Keller]   7 call site(s) that can panic were decided (proof or counterexample)' \
  '  ✅ Panic sites decided: 7 (3 panicking)'; do
  if grep -qF "$needle" "$PD_LOG" 2>/dev/null; then
    echo "[PASS] panic_decided: the run announces $needle"
  else
    echo "[FAIL] panic_decided: nothing announces '$needle'"
    FAILED=1
  fi
done

# ===========================================================================
#  The precision probe: eighteen functions of safe Rust, asserted clean.
#
#  `tests/potency_probe` is eighteen functions of safe Rust â€” nothing in it can
#  panic. Three of its obligations used to be counterexamples (K5), in
#  the same shape: a loop's own condition did not reach the assertion it guards,
#  so the solver was free to choose the loop's *exit* value. The loop relation in
#  `widened_state` closed that, so this block asserts the end state instead of
#  pinning the debt: each line below fails the suite if a verdict moves back.
#
#  The one obligation that is still not decided is asserted as such rather than
#  ignored. `fixed_array`'s index comes out of `Range::next`, a call this analysis
#  does not model, so the obligation is reported undecided *with that reason
#  named* â€” not silently passed, and not reported as a failure either.
#
#  This deliberately does NOT go through `run_case`/the manifest: the published
#  page counts the obligations of the fixture run, and publishing
#  counterexamples from a crate that cannot panic would be a claim the tool has
#  not earned.
# ---------------------------------------------------------------------------
rm -rf "$ROOT/tests/potency_probe/target"
(cd "$ROOT/tests/potency_probe" && rm -f report.json && "$VERIFIER" keller verify >"$MEASURED/potency_probe.log" 2>&1)
probe_exit=$?
mark_consumed potency_probe
PROBE_LOG="$MEASURED/potency_probe.log"
probe_fails=$(grep -c '^\[FAIL\]' "$PROBE_LOG" 2>/dev/null || true)
probe_fails=${probe_fails:-0}
probe_warns=$(grep -c '^\[WARN\]' "$PROBE_LOG" 2>/dev/null || true)
probe_warns=${probe_warns:-0}
if [ "$probe_exit" = "0" ] && [ "$probe_fails" = "0" ]; then
  echo "[PASS] potency_probe: 18 functions that cannot panic report no counterexample (K5)"
else
  echo "[FAIL] potency_probe: exit $probe_exit with $probe_fails false positive(s) â€” K5 has regressed"
  grep '^\[FAIL\]' "$PROBE_LOG" 2>/dev/null | sed 's/^/       /'
  FAILED=1
fi

# The loop the relation was written for: its three obligations are *decided*, not
# merely absent. An undecided loop is a regression too â€” a `[WARN]` here is not
# the clean verdict this case is about, and neither is a missing line, which is
# why the line itself is grepped for rather than the absence of a FAIL.
for expected in 'chunk_loop (bb1)' 'chunk_loop (bb3)' 'chunk_loop (bb5)'; do
  if grep -qF "[PASS] Verified formal integrity: $expected" "$PROBE_LOG"; then
    echo "[PASS] potency_probe: $expected is decided (its guard reaches it)"
  else
    echo "[FAIL] potency_probe: $expected is not decided PASS â€” K5 has regressed"
    FAILED=1
  fi
done

# S5: the pointer borrows one local and is never pointed anywhere else, so a read
# through it is a read of that local, and the guard and the bounds check are one
# quantity. Without the resolution the read has no term and the index is undecided.
for expected in 'through_local_ref (bb1)'; do
  if grep -qF "[PASS] Verified formal integrity: $expected" "$PROBE_LOG"; then
    echo "[PASS] potency_probe: $expected is decided (the reference reads the local it borrowed)"
  else
    echo "[FAIL] potency_probe: $expected is not decided PASS, so dereference resolution regressed"
    FAILED=1
  fi
done

# And the artefact, not just the console: the guard and the value it guards are
# the same quantity, which is the whole content of the fix. The body's index is
# related to the *previous trip's* counter under that trip's own guard, and the
# length it is checked against is bounded â€” which is what closes the overflow
# check in the header. A proof that stopped naming the previous trip, or stopped
# bounding the length, would still re-discharge: it would just no longer be the
# argument this asserts.
probe_bb3="$ROOT/tests/potency_probe/obligations/potency_probe/chunk_loop_bb3.smt2"
if [ ! -f "$probe_bb3" ]; then
  echo "[FAIL] potency_probe: no obligation was emitted for chunk_loop bb3"
  FAILED=1
else
  for needle in '(<= (+ mir_3 8) len_1)' '(= mir_3 (+ prev_bb6_' '(<= len_1 9223372036854775807)'; do
    if grep -qF "$needle" "$probe_bb3"; then
      echo "[PASS] potency_probe: the loop obligation carries $needle"
    else
      echo "[FAIL] potency_probe: the loop obligation no longer carries $needle"
      FAILED=1
    fi
  done
fi

# The safe-arithmetic forms (K9). Each function below asks the
# analysis to use a *fact* about an idiom's result, so it is decided only when the
# idiom has an encoding: a saturating sum is never below its right operand, a
# checked sum in its `Some` arm is the exact sum, a wrapping sum cannot wrap when
# both inputs are under 100, and a narrowing cast is a modulus. All four are
# decided, `checked_*` since its discriminant was modelled with its payload.
for expected in 'saturating_never_below (bb1)' 'checked_sum_gives_the_sum (bb4)' 'wrapping_sum_under_input_bound (bb4)' 'narrowed_under_input_bound (bb2)'; do
  if grep -qF "[PASS] Verified formal integrity: $expected" "$PROBE_LOG"; then
    echo "[PASS] potency_probe: $expected is decided (its idiom has an encoding)"
  else
    echo "[FAIL] potency_probe: $expected is not decided PASS â€” the encoding for its idiom regressed"
    FAILED=1
  fi
done

# And in the artefact, not just the verdict: a `checked_*` result is an `Option`,
# so the state has to carry both the payload (the sum, reduced into the operand
# type) and the test that chose the variant (`Some` exactly when that sum is in
# range), tied to the *same* sum. A payload without the test is what reported
# this function as an underflow for `s = 0, x = 1` â€” `y = 255`, the `None` arm â€”
# so the guard's presence is the difference between a proof and a false positive.
# The needles are the two the artifact actually carries, and re-discharging it is
# asserted to still answer `unsat` rather than `sat`.
probe_checked="$ROOT/tests/potency_probe/obligations/potency_probe/checked_sum_gives_the_sum_bb4.smt2"
if [ ! -f "$probe_checked" ]; then
  echo "[FAIL] potency_probe: no obligation was emitted for checked_sum_gives_the_sum bb4"
  FAILED=1
else
  for needle in '(ite (and (>= (+ mir_1 mir_2) 0) (<= (+ mir_1 mir_2) 255)) 1 0) 1' '(= overflow_l (mod (+ mir_1 mir_2) 256))'; do
    if grep -qF "$needle" "$probe_checked"; then
      echo "[PASS] potency_probe: the checked obligation carries $needle"
    else
      echo "[FAIL] potency_probe: the checked obligation no longer carries $needle"
      FAILED=1
    fi
  done
  if [ -n "$Z3_BIN" ]; then
    if [ "$("$Z3_BIN" -smt2 "$probe_checked" 2>/dev/null | head -1 | tr -d '\r')" = "unsat" ]; then
      echo "[PASS] potency_probe: the checked obligation re-discharges to unsat"
    else
      echo "[FAIL] potency_probe: the checked obligation no longer re-discharges to unsat"
      FAILED=1
    fi
  fi
fi

# The one obligation it cannot decide is reported undecided, with the reason
# named: the unmodelled `Range::next` behind `fixed_array`'s index. More than that
# one is a precision regression.
if grep -qF '[WARN] Undecided in fixed_array (bb5)' "$PROBE_LOG" &&
  grep -qF 'Range<usize>' "$PROBE_LOG"; then
  echo "[PASS] potency_probe: the loop-body obligation it cannot decide is reported undecided, with the unmodelled call named"
else
  echo "[FAIL] potency_probe: fixed_array's undecided obligation is gone, or its reason no longer names the unmodelled call"
  FAILED=1
fi
if [ "$probe_warns" -gt 1 ]; then
  echo "[FAIL] potency_probe: $probe_warns undecided obligation(s), more than the one this case accounts for â€” the analysis got weaker"
  FAILED=1
fi
rm -rf "$ROOT/tests/potency_probe/target"


# ===========================================================================
#  The fixtures nothing was running (D8).
#
#  `tests/precond_probe` and `test-crates/*` sat in the tree and were run by
#  nothing: a fixture whose verdict nobody asserts is a fixture where a
#  regression is invisible, which is why they are wired in here rather than
#  left described but unrun. They deliberately do NOT go through `check_goldens`
#  or the manifest: `tests/goldens` is the corpus the K7 rate below is computed
#  over, and adding straight-line `unguarded` FAILs to it would move a published
#  number for a reason that has nothing to do with the analysis.
#
#  Each is asserted in both directions, so neither can pass by reporting nothing.
# ===========================================================================

# `precond_probe`: what the tool makes of a precondition. The crate writes one as
# an in-body `assert!` and calls the two callees below it only with values that
# cannot overflow, so one run separates two questions that are easy to conflate —
# is an in-body assertion believed, and is a callee judged on its call sites?
run_table_case precond_probe
check_obligations precond_probe
check_witnesses precond_probe
check_locations precond_probe
check_environment precond_probe

# Believed, and shown in the artefact rather than inferred from the verdict: the
# proof of `x + 1` in `entry_assert` is emitted with the assertion's own condition
# as the reachability guard. A run that stopped assuming it would have to report
# that overflow as a FAIL, so this is the line that distinguishes "the assertion
# is trusted" from "the assertion is checked".
pp_entry="$ROOT/tests/precond_probe/obligations/precond_probe/entry_assert_bb1.smt2"
if [ ! -f "$pp_entry" ]; then
  echo "[FAIL] precond_probe: no obligation was emitted for entry_assert bb1"
  FAILED=1
elif grep -qF '; verdict:  PASS (unsat)' "$pp_entry" && grep -qF '(< mir_1 100)' "$pp_entry"; then
  echo "[PASS] precond_probe: the in-body assertion is believed — its condition is the guard the overflow beneath it was proved under"
else
  echo "[FAIL] precond_probe: entry_assert's precondition is no longer the guard of the proof beneath it"
  FAILED=1
fi

# A8's boundary, narrowed by S7: a private leaf whose every call passes a constant
# it can take is analysed under that precondition, so `private_callee` leaves no
# finding and the check that earns the assumption is at its call site. The two no
# call site decides — an exported helper (callers outside this crate are not in the
# scan) and one called with a value this analysis does not bound — are still
# reported on their own body, which is what "judged without its call sites" means
# here now. Pinned as a boundary either way: the day either side moves, the case has
# to be re-read and go red rather than quietly assert something else.
if grep -q '"private_callee"' "$ROOT/tests/precond_probe/report.json" 2>/dev/null; then
  echo "[FAIL] precond_probe: private_callee is reported although every call site proves its derived precondition"
  FAILED=1
else
  echo "[PASS] precond_probe: private_callee is analysed under the precondition its one call site proves, and leaves no finding"
fi
for callee in public_callee private_unknown; do
  if grep -q "\"$callee\"" "$ROOT/tests/precond_probe/report.json" 2>/dev/null; then
    echo "[PASS] precond_probe: $callee is still reported on its own body (A8's boundary)"
  else
    echo "[FAIL] precond_probe: $callee is no longer reported — no call site decides it, so this case's boundary has moved and must be rewritten"
    FAILED=1
  fi
done

# The inventory the case is written against: the two arithmetic checks no call site
# decides, the proof beside them, and the S7 summary line that says the one helper
# whose call sites do decide it was analysed under that precondition. The user's own
# `assert!` is *not* among the encountered assertions — the guard above is its only
# effect (A6's recorded gap) — so a change to that inventory has to be seen here and
# not inferred from an exit code.
pp_fails=$(grep -c '^\[FAIL\]' "$MEASURED/precond_probe.log" 2>/dev/null || true)
pp_passes=$(grep -c '^\[PASS\]' "$MEASURED/precond_probe.log" 2>/dev/null || true)
if [ "${pp_fails:-0}" = "2" ] && [ "${pp_passes:-0}" = "3" ]; then
  echo "[PASS] precond_probe: the assertion inventory is the 2 counterexamples, the 1 proof and the 1 summary this case pins"
else
  echo "[FAIL] precond_probe: expected 2 FAIL and 3 PASS lines, got ${pp_fails:-0} and ${pp_passes:-0}"
  FAILED=1
fi
# And the fourth line, added by A6: the same `assert!` whose condition is believed
# is also a call into the panic machinery, so its failure path is named as a site
# nothing was decided about. Counted apart from the counterexamples above, because
# the two say different things.
pp_sites=$(grep -c '"kind": "unmodelled-panic"' "$ROOT/tests/precond_probe/report.json" 2>/dev/null || true)
if [ "${pp_sites:-0}" = "1" ]; then
  echo "[PASS] precond_probe: the in-body assertion's failure path is named as a panic site"
else
  echo "[FAIL] precond_probe: expected the body's own assert! to be reported once as a panic site, got ${pp_sites:-0}"
  FAILED=1
fi

# `helper_summary`: S7. A small, private, leaf helper is analysed under the
# precondition its own body implies and its call sites prove, so `inc`'s unguarded
# overflow is gone; the three helpers no call site decides keep exactly the finding
# they had — `boom` (one caller passes 255), `open` (the caller passes a runtime
# value) and `exported` (`pub`, so its callers are not all visible here). The check
# is pinned three ways: the console names the summary and the two call sites, the
# report still carries the three bodies, and the proof of `x + 1` carries the derived
# bound as its reachability guard — the same shape precond_probe pins for a written
# precondition.
run_table_case helper_summary
check_obligations helper_summary
check_witnesses helper_summary
check_locations helper_summary
check_environment helper_summary

for hs_line in \
  '[PASS] Helper summary in inc: `x <= 254` is checked at all 2 call site(s), so this body is analysed under it' \
  '[PASS] Verified formal integrity: inc (bb0)' \
  '[PASS] Helper precondition in main (bb5): `x <= 254` holds for `inc` (argument 7)' \
  '[PASS] Helper precondition in main (bb6): `x <= 254` holds for `inc` (argument 200)'; do
  if grep -qF "$hs_line" "$MEASURED/helper_summary.log" 2>/dev/null; then
    echo "[PASS] helper_summary: $hs_line"
  else
    echo "[FAIL] helper_summary: expected \"$hs_line\""
    FAILED=1
  fi
done
for hs_callee in boom open exported; do
  if grep -q "\"function\": \"$hs_callee\"" "$ROOT/tests/helper_summary/report.json" 2>/dev/null; then
    echo "[PASS] helper_summary: $hs_callee keeps its unguarded finding, so no precondition was assumed there"
  else
    echo "[FAIL] helper_summary: $hs_callee is no longer reported — no call site proves a precondition for it"
    FAILED=1
  fi
done
hs_proof="$ROOT/tests/helper_summary/obligations/helper_summary/inc_bb0.smt2"
if [ -f "$hs_proof" ] && grep -qF '; verdict:  PASS (unsat)' "$hs_proof" && grep -qF '; reachable: (<= mir_1 254)' "$hs_proof"; then
  echo "[PASS] helper_summary: inc's overflow is proved under the bound its call sites were checked against"
else
  echo "[FAIL] helper_summary: the proof of inc no longer carries the derived precondition as its guard"
  FAILED=1
fi

# `test-crates/buggy-project`: one finding of each class the report can state, on
# a crate written to be caught. The classes are asserted by name because the exit
# code alone would still be 1 with only one of the three surviving.
run_table_case test-crates/buggy-project
check_obligations test-crates/buggy-project
check_witnesses test-crates/buggy-project
check_locations test-crates/buggy-project
check_environment test-crates/buggy-project

for needle in '"kind": "bounds"' '"kind": "overflow"' '"kind": "divzero"'; do
  if grep -qF "$needle" "$ROOT/test-crates/buggy-project/report.json" 2>/dev/null; then
    echo "[PASS] buggy-project: the report carries $needle"
  else
    echo "[FAIL] buggy-project: the report is missing $needle"
    FAILED=1
  fi
done
# K10's rule on a real run rather than in a unit test: `check_index`'s index is a
# value the obligation only bounds, so its witness is one instance among many and
# has to say so — while the pinned witness in the same file is a value the
# obligation forces. A `pinned` on the bounds finding would report the solver's
# choice as a fact about the program, which is what the field exists to stop.
for needle in '"function": "check_index"' '"witness_kind": "one-of-many"'; do
  if grep -qF "$needle" "$ROOT/test-crates/buggy-project/report.json" 2>/dev/null; then
    echo "[PASS] buggy-project: the report carries $needle"
  else
    echo "[FAIL] buggy-project: the report is missing $needle"
    FAILED=1
  fi
done

# The control inside the same crate: the divisor that cannot be zero *is* proved.
# Without this, a change that reported every division would pass the three checks
# above while getting the answer wrong.
if grep -qF '[PASS] Verified formal integrity: check_div (bb1)' "$MEASURED/test-crates__buggy-project.log"; then
  echo "[PASS] buggy-project: the in-range divisor is proved, so the findings above are not a blanket complaint"
else
  echo "[FAIL] buggy-project: check_div's safe path is no longer decided"
  FAILED=1
fi

# `test-crates/safe-project`: the negative control. It exits 0 *and* reports
# nothing, which is the pair that makes the case above mean something, and it is
# the only fixture outside `tests/` that any runner had ever executed.
run_table_case test-crates/safe-project
check_environment test-crates/safe-project
if [ "$(grep -c '"status"' "$ROOT/test-crates/safe-project/report.json" 2>/dev/null || true)" = "0" ]; then
  echo "[PASS] safe-project: the report is empty, not merely passing"
else
  echo "[FAIL] safe-project: a fixture that cannot panic produced a finding"
  FAILED=1
fi

# ===========================================================================
#  index.html is a hand-written page, not an artefact of this run.
#
#  It used to be regenerated from this suite by tests/gen_index.py and diffed
#  here. That machinery is gone: the page carries no per-run figures, so there
#  is nothing for this suite to compare it against, and the evidence it used to
#  reproduce lives in tests/ where a reader can run it. Numbers that would drift
#  (obligation counts, verdict totals) are deliberately absent from it.
# ===========================================================================
PYTHON=""
for candidate in python3 py python; do
  if command -v "$candidate" >/dev/null 2>&1 && "$candidate" -c 'import sys' >/dev/null 2>&1; then
    PYTHON="$candidate"
    break
  fi
done
if [ -z "$PYTHON" ] || ! "$PYTHON" "$ROOT/tests/trust_checks.py"; then
  echo "[FAIL] trust: CLI regression gate failed or Python is unavailable"
  FAILED=1
fi

# K7: what the FAILs are attributed to, and a budget on the class a false
# positive can come from.
#
# The published rate on code nobody here wrote lives in
# docs/FALSE-POSITIVE-RATE.md, with the command that measured it. This is the
# half that can run in CI: the curated corpus is checked in, so the same
# attribution is recomputed from the committed goldens and compared against the
# budget those measurements set. It is a weak gate in one direction â€” a change
# that makes the *analysis* worse needs the real crates to show up â€” and a real
# one in the other: a widening change that turns a genuine finding into an
# artifact of the loop abstraction moves these FAILs into `widening-limited` and
# trips it here. The budget is the measured 0.111 plus a margin of one finding.
FPRATE_BUDGET="0.15"
if [ -z "$PYTHON" ]; then
  echo "[SKIP] k7: no Python, so the false-positive budget could not be recomputed"
else
  "$PYTHON" "$ROOT/scripts/fp-rate.py" --dir "$ROOT/tests/goldens" --budget "$FPRATE_BUDGET" \
    >"$MEASURED/fp-rate.txt" 2>&1
  fp_exit=$?
  sed 's/^/       /' "$MEASURED/fp-rate.txt"
  if [ "$fp_exit" = "0" ]; then
    echo "[PASS] k7: the curated corpus stays within the false-positive budget ($FPRATE_BUDGET)"
  else
    echo "[FAIL] k7: more FAILs are attributed to the loop abstraction than the budget allows"
    FAILED=1
  fi
fi

# ===========================================================================
#  K2 / O3: a missing pinned toolchain is named, not read as a version
#  mismatch.
#
#  The verifier is linked against one exact nightly, so before `cargo` starts it
#  asks `rustup toolchain list` whether that toolchain is installed and exits 3
#  with the install line if it is not. CONTRIBUTING.md forbids uninstalling the
#  pinned toolchain to test this, and that rule is right: a suite that removes
#  the compiler it runs on can destroy the machine it audits. The branch is
#  reached instead with a real `rustup` pointed at an empty `RUSTUP_HOME`, where
#  `rustup toolchain list` genuinely answers `no installed toolchains` (exit 0,
#  observed) while the installed toolchain is untouched. The check reads exactly
#  that listing, so this is the branch a machine without the toolchain takes.
#
#  What this does not prove is a real absence on disk: the compiler is still
#  installed, and DONE.md says so rather than implying a clean machine was
#  tested. A compiled stub `rustup` was not used: it would have to be built by
#  the toolchain whose absence the case is about.
# ---------------------------------------------------------------------------
EMPTY_RUSTUP="$MEASURED/empty-rustup-home"
rm -rf "$EMPTY_RUSTUP"
mkdir -p "$EMPTY_RUSTUP"
MT_LOG="$MEASURED/missing_toolchain.log"
(cd "$ROOT/tests/safe_code" && RUSTUP_HOME="$EMPTY_RUSTUP" "$VERIFIER" keller verify >"$MT_LOG" 2>&1)
mt_exit=$?
# Third field: this case publishes no report, so there is no WARN count to hold.
printf '%s\t%s\t%s\n' missing_toolchain 3 0 >> "$MEASURED/manifest.tsv"
mark_consumed missing_toolchain
if [ "$mt_exit" = "3" ]; then
  echo "[PASS] missing_toolchain: a missing pinned toolchain is a setup failure, not a verdict"
else
  echo "[FAIL] missing_toolchain: expected exit 3 (setup failure), got $mt_exit"
  FAILED=1
fi
if grep -qF "is not installed, and it cannot analyse a crate without it" "$MT_LOG"; then
  echo "[PASS] missing_toolchain: the message names the toolchain it cannot run without"
else
  echo "[FAIL] missing_toolchain: the message does not say which toolchain is missing"
  FAILED=1
fi
DRIVER_TOOLCHAIN="$(cd "$ROOT" && rustup show active-toolchain 2>/dev/null | awk '{print $1}')"
if [ -z "$DRIVER_TOOLCHAIN" ]; then
  echo "[SKIP] missing_toolchain: rustup would not name the active toolchain, so the install line was not matched"
elif grep -qF "rustup toolchain install $DRIVER_TOOLCHAIN" "$MT_LOG"; then
  echo "[PASS] missing_toolchain: the message carries the install line for $DRIVER_TOOLCHAIN"
else
  echo "[FAIL] missing_toolchain: the install line does not name $DRIVER_TOOLCHAIN"
  FAILED=1
fi
# The other branch this must not be: an installed-but-different compiler is
# reported with both identities (`built with:` / `installed:`), and a run that
# reached *that* one has not tested the missing toolchain at all.
if grep -qF "built with:" "$MT_LOG"; then
  echo "[FAIL] missing_toolchain: the run reported a compiler mismatch, not a missing toolchain"
  FAILED=1
else
  echo "[PASS] missing_toolchain: the run stopped at the missing toolchain, not at a compiler mismatch"
fi
if [ -f "$ROOT/tests/safe_code/report.json" ]; then
  echo "[FAIL] missing_toolchain: a verdict was published although the toolchain was missing"
  FAILED=1
else
  echo "[PASS] missing_toolchain: no verdict is published when the toolchain is missing"
fi

check_case_table

if [ "$FAILED" = "1" ]; then
  echo "[tests] *** FAILURES DETECTED ***"
  exit 1
fi
echo "[tests] All integration tests passed."
exit 0

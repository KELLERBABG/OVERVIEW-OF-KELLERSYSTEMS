@echo off
setlocal
rem ===========================================================================
rem  keller-verify integration test runner (Windows)
rem
rem  Builds the verifier, then runs `keller verify` against each test crate and
rem  asserts the expected exit code and report.json contents:
rem
rem    safe_code      -> exit 0 (no FAIL events)
rem    overflow_fail  -> exit 1 (report.json contains a FAIL)
rem    divzero_fail   -> exit 1 (report.json contains a FAIL)
rem    branch_merge   -> exit 1 (a constant merged from two branches must not
rem                              be folded to "safe" and silently skipped)
rem    guarded        -> exit 1, and the four functions whose arithmetic a branch
rem                              guard already rules out must NOT appear in the
rem                              report, while the one with too weak a guard must
rem    workspace_multi -> exit 1 and report.json must contain BOTH members'
rem                              findings (per-crate reports are merged)
rem    loop_fail      -> exit 1: a loop body is analysed under its own guards, so
rem                              the counter increment is *proved* while the
rem                              accumulator's reachable overflow is still
rem                              reported, with the trip it can fire on
rem    nested_loop    -> exit 1: the shape next door to loop_fail - an inner loop
rem                              inside an outer one, with break and continue. The
rem                              cycle widening bounds the *condition* as well as
rem                              the values, so its guarded increments are proved
rem                              too and only the accumulator's unbounded overflow
rem                              is reported; the single-cycle control in the same
rem                              crate is proved as well, which is the boundary the
rem                              case pins. Run once more with the round budget cut
rem                              to one round, which also pins the *report* of a
rem                              lost precision: both increments come back as FAILs
rem                              and the report says why, instead of leaving the
rem                              false positives unattributable
rem    shift_amount   -> exit 1, and the shift whose amount can leave the type's
rem                              width must be reported while the two shifts that
rem                              cannot panic must be proved
rem    fold_fail      -> exit 1, and the failures the constant-folding pass found
rem                              without asking Z3 must carry a witness and an
rem                              obligation like any other
rem    broken_code    -> exit 3: a crate the compiler rejected is a setup
rem                              failure, not a clean run
rem    precond_probe  -> exit 1: a precondition written as an in-body assertion is
rem                              believed as a guard; a private leaf is now analysed
rem                              under the precondition its constant call sites prove
rem                              (S7), while an exported helper and one called with an
rem                              unbounded value are still judged without their call
rem                              sites (A8's boundary, narrowed and still pinned)
rem    helper_summary -> exit 1: the S7 summary is used only where its call sites
rem                              prove it, so inc loses its unguarded finding and
rem                              boom/open/exported keep theirs
rem    test-crates/buggy-project -> exit 1: one finding of each named class
rem                              (bounds, overflow, division by zero)
rem    test-crates/safe-project  -> exit 0: the negative control beside it, and the
rem                              only fixture outside tests/ that is run at all
rem
rem  A FAIL must also name the input that triggers it - whether the obligation
rem  went to Z3 or was decided by constant folding: report.json carries a
rem  "witness" for every FAIL, and a pinned *.witness.smt2 obligation (the
rem  formula with those values asserted) is written so the counterexample can be
rem  re-discharged with `z3 -smt2`.
rem
rem  Finally the exit-code contract's own flags: --deny-warn must tighten the
rem  gate without changing any verdict above, and an unknown flag must exit 2
rem  rather than quietly run with the gate switched off.
rem
rem  Exit code 0 = all cases passed, 1 = at least one case failed.
rem ===========================================================================

set "ROOT=%~dp0.."
set "VERIFIER=%ROOT%\target\debug\keller-verify.exe"
set "ENV_HELPER=%ROOT%\target\debug\keller-env.exe"

rem The fixture cases, the exit code each must produce and which cases have a
rem golden live in one file that this runner and tests/run_tests.sh both read:
rem tests\cases.txt. The row is the authority - :run_table_case looks the exit
rem code up from it and compares the golden exactly when the row names one - and
rem :check_case_table at the end checks both directions, so a case cannot be run
rem by one runner and not the other, and the goldens compared here cannot be a
rem shorter list than the goldens that exist. Before the file they were exactly
rem that: 85 checks over ten goldens here against 281 over thirteen there, with
rem nothing in the tree saying which number was right.
set "CASES_FILE=%ROOT%\tests\cases.txt"
set "CASES_CONSUMED=%TEMP%\keller-cases-%RANDOM%.txt"

rem Both binaries come out of one `cargo build`: the verifier and the helper that
rem reports the environment the verifier needs to start (see below).
set "NEED_BUILD="
if not exist "%VERIFIER%" set "NEED_BUILD=1"
if not exist "%ENV_HELPER%" set "NEED_BUILD=1"
if defined NEED_BUILD (
  echo [tests] Binaries not found, building...
  pushd "%ROOT%"
  call cargo build || (popd & exit /b 1)
  popd
)

rem Set before anything can test it: an undefined variable does not expand to an
rem empty string on both sides of a comparison here, so `if not "%FAILED%"=="0"`
rem would be true of a runner that has not failed yet - which it was, and the run
rem stopped after the environment block without running a single case.
set "FAILED=0"

rem Make the toolchain's runtime libraries and Z3 findable by the wrapper.
rem
rem The lookup is not written here. src\host_env.rs holds it - one module shared
rem with build.rs, which links against the same Z3 - and keller-env.exe, a second
rem binary built from this crate, prints the answer; run_tests.sh consumes the
rem same printout. The helper is std-only on purpose: the verifier is linked
rem against Z3 and loads the toolchain's driver dylibs at run time, so it cannot
rem start until these directories are already on PATH - which is exactly what it
rem would be being asked to work out.
rem
rem This file used to ask `rustc -vV` for the host triple, build the rustlib path
rem by hand, and take Z3 from the environment, then .z3\current, then a hardcoded
rem machine-specific install - never globbing several releases and never checking
rem that the solver would start. Hardcoding msvc paths meant the wrapper could not
rem even start on a -gnu host: Windows reported STATUS_DLL_NOT_FOUND
rem (-1073741515) and every case looked like a verdict failure rather than a
rem setup one. There is one copy of the rule now, and a unit test fails the build
rem if either runner grows another.
rem
rem The helper's own warnings are echoed first, so a missing solver is visible
rem before the cases that report what they could not re-discharge. Its output is
rem read from a file rather than a pipe so stderr can be shown and the key=value
rem lines set without either being parsed out of the other.
set "ENV_OUT=%TEMP%\keller-env-%RANDOM%.txt"
"%ENV_HELPER%" --shell plain --manifest-dir "%ROOT%" >"%ENV_OUT%" 2>&1
set "ENV_EXIT=%ERRORLEVEL%"
for /f "delims=" %%L in ('findstr /b /C:"[env]" "%ENV_OUT%"') do echo %%L
for /f "tokens=1,* delims==" %%A in ('findstr /b /v /C:"[env]" "%ENV_OUT%"') do call :env_set "%%A" "%%B"
del "%ENV_OUT%" >nul 2>&1

rem One top-level line, outside any parenthesised block: a `set` inside a block
rem expands the PATH it was parsed with, not the one the helper reported.
if defined PATH_ADD set "PATH=%PATH_ADD%;%PATH%"

if defined RUSTC (
  for %%I in ("%RUSTC%") do set "TOOLCHAIN_BIN=%%~dpI"
)
if defined TOOLCHAIN_BIN (
  if exist "%VERIFIER%" copy /Y "%VERIFIER%" "%TOOLCHAIN_BIN%keller-verify.exe" >nul
  if exist "%ROOT%\.z3\current\bin\libz3.dll" copy /Y "%ROOT%\.z3\current\bin\libz3.dll" "%TOOLCHAIN_BIN%libz3.dll" >nul
  set "VERIFIER=%TOOLCHAIN_BIN%keller-verify.exe"
)

if not "%ENV_EXIT%"=="0" (
  echo [FAIL] env: keller-env could not report the toolchain and Z3, exit %ENV_EXIT%
  set "FAILED=1"
)

rem Nothing below can run without the toolchain's driver dylibs: every case would
rem die at load with STATUS_DLL_NOT_FOUND, which reads as a broken tool rather
rem than a broken setup. Say so once instead of letting every case say it.
if not defined RUSTLIB (
  echo [FAIL] env: the pinned toolchain's rustc driver dylibs were not found, so no case can start the wrapper
  rem No parentheses in any echo below: inside a parenthesised block the parser
  rem counts them, so a stray one ends the block early and the lines after it are
  rem executed as commands - which is how a comment line once ran and reported a
  rem command named "without" as missing. The quiet version of that is worse than
  rem a stray command: `set "FAILED=1"` below an `echo` carrying a `(s)` is no
  rem longer inside its condition, so the check never reports and the run exits 1
  rem having printed no [FAIL] at all - which reads as a broken fixture. The rule
  rem is asserted on this file by
  rem `host_env::tests::no_echo_inside_a_cmd_block_carries_a_bare_parenthesis`,
  rem so a new one fails `cargo test` instead of the suite.
  echo [env] install the pinned nightly with its rustc-dev component - see tests\rust-toolchain.toml
  set "FAILED=1"
) else (
  rem `set X` prints the value verbatim, so a path containing & is not split.
  set RUSTLIB
)
if defined Z3_BIN (
  echo [PASS] env: keller-env resolved the toolchain and a solver
) else (
  echo [SKIP] env: no runnable z3 was found
)

rem Whether the fixture crates can spell an x86-64 asm block, decided from the
rem toolchain's host rather than from a guess: tests\statement_writes carries one
rem shape only that architecture can assemble, and the assertions about it
rem follow this. tests/run_tests.sh decides it the same way from the same value.
set "SW_ASM=0"
echo %HOST%| findstr /b /C:"x86_64-" >nul 2>&1
if not errorlevel 1 set "SW_ASM=1"
echo %HOST%| findstr /b /C:"i686-" >nul 2>&1
if not errorlevel 1 set "SW_ASM=1"

if not "%FAILED%"=="0" (
  echo [tests] *** FAILURES DETECTED ***
  exit /b 1
)

call :run_table_case safe_code
rem Every completed run has a findings array and environment: nothing to report
rem is not nothing to attribute.
call :check_env safe_code
call :run_table_case overflow_fail
call :check_env overflow_fail
call :run_table_case divzero_fail
call :run_table_case branch_merge
call :run_table_case guarded
call :run_table_case workspace_multi

rem  Five members, not two (D9), asserted the same way tests/run_tests.sh asserts
rem  it: one `--crate-name` receipt per member and exactly the two findings, so a
rem  clean member cannot erase a member that has one. The repeated run is the half a
rem  parallel-ordering bug shows up in, and it is compared byte for byte.
call :count_lines "%ROOT%\tests\workspace_multi\report.run.json" "--crate-name" WS_NAMED
if not "%WS_NAMED%"=="5" (
  echo [FAIL] workspace_multi: expected 5 worker receipts, one per member, got %WS_NAMED%
  set "FAILED=1"
) else (
  echo [PASS] workspace_multi: five members, five worker receipts
)
for %%N in (alpha_overflow beta_divide) do (
  findstr /C:"\"%%N\"" "%ROOT%\tests\workspace_multi\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] workspace_multi: merged report is missing %%N, so a clean member erased one that has a finding
    set "FAILED=1"
  ) else (
    echo [PASS] workspace_multi: merged report contains %%N
  )
)
set "WS_FIRST=%TEMP%\keller-workspace-first.json"
copy /y "%ROOT%\tests\workspace_multi\report.json" "%WS_FIRST%" >nul 2>&1
pushd "%ROOT%\tests\workspace_multi"
if exist target rmdir /s /q target
"%VERIFIER%" keller verify >"%TEMP%\keller-workspace-repeat.log" 2>&1
set "WS_REPEAT=%ERRORLEVEL%"
popd
fc /b "%WS_FIRST%" "%ROOT%\tests\workspace_multi\report.json" >nul 2>&1
if not "%WS_REPEAT%"=="1" (
  echo [FAIL] workspace_multi: the repeated run exited %WS_REPEAT%, not 1
  set "FAILED=1"
) else (
  if errorlevel 1 (
    echo [FAIL] workspace_multi: the merge is not stable across runs, so a member's report depends on the order
    set "FAILED=1"
  ) else (
    echo [PASS] workspace_multi: a second run of the same five members is byte-identical
  )
)

rem Path sensitivity, asserted in both directions. A regression that simply
rem stopped flagging overflows would pass the "proved" half and fail the
rem control; one that went back to ignoring the guards fails the "proved" half.
rem The search patterns carry the quote so `sum_under_guarded` cannot satisfy the
rem `sum_guarded` check.
for %%N in (sum_guarded slice_guarded div_guarded dead_branch) do (
  findstr /C:"\"%%N\"" "%ROOT%\tests\guarded\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [PASS] guarded: %%N is proved, so it is absent from the report
  ) else (
    echo [FAIL] guarded: %%N is guarded and must be proved, but it is reported
    set "FAILED=1"
  )
)
findstr /C:"\"sum_under_guarded\"" "%ROOT%\tests\guarded\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] guarded: sum_under_guarded must be flagged, but the report is silent
  set "FAILED=1"
) else (
  echo [PASS] guarded: sum_under_guarded is still flagged, as its guard is too weak
)

rem The point of the merge: both members must be present in one report.
rem Asserting the function names catches a regression where the last worker wins
rem again - the exit code alone would still be 1 with only one of them.
for %%N in (alpha_overflow beta_divide) do (
  findstr /C:"%%N" "%ROOT%\tests\workspace_multi\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] workspace_multi: merged report is missing %%N
    set "FAILED=1"
  ) else (
    echo [PASS] workspace_multi: merged report contains %%N
  )
)

call :run_table_case loop_fail

rem The loop probe. A block inside a cycle used to get no path state, so the body's
rem two increments were checked path-insensitively and `i + 1` was flagged at 255 -
rem a value the loop's own guard, `i < steps`, makes unreachable. Cycles are now
rem unrolled to their back edge and their loop-carried values widened, so the body
rem is checked under its own guards. The counter increment must therefore be
rem proved (it is absent from the report) and the accumulator's overflow - which
rem is real: acc = 250 over six trips walks 250..=255 - must stay reported, naming
rem the operands of the failing addition and the accumulator value the obligation
rem pins. The guard's own symbols must NOT appear: `i < steps` leaves both of them
rem free, so their values are the model's choice rather than a fact. The
rem obligation is re-discharged by :run_table_case, as tests/run_tests.sh does.
for %%N in (bb2 "acc (_1) = 255") do (
  findstr /C:%%N "%ROOT%\tests\loop_fail\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] loop_fail: report is missing %%N - the loop body lost the guard that makes its overflow a concrete trip
    set "FAILED=1"
  ) else (
    echo [PASS] loop_fail: report carries %%N
  )
)
rem Two more absences. bb3 is the counter increment and count_to is the same loop
rem shape on its own: neither may appear, or the guard was widened away. steps and
rem i are values the obligation does not pin, so naming them would report the
rem model's choice as a fact.
for %%N in (bb3 count_to "steps (_2)" "i (_3)") do (
  findstr /C:%%N "%ROOT%\tests\loop_fail\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [PASS] loop_fail: %%N is absent from the report
  ) else (
    echo [FAIL] loop_fail: %%N must not be reported
    set "FAILED=1"
  )
)

rem A lost precision has to be *reported*, and the case below cannot show that:
rem its dataflow converges, so there is nothing to report. What this shows is that
rem the record exists and is reachable. The round budget is the boundary of the
rem finding as well as a safety net, so cutting it to one round drives every
rem function in this crate to that boundary - and the run then has to say so,
rem instead of quietly reporting the guards' work as failures. The two increments
rem the default budget proves come back as FAILs, which is exactly the false
rem positive this record exists to attribute. Kept before the case below, which
rem resets report.json and writes the console log its checks read.
rem The override is cleared straight afterwards: left set, every later case would
rem run with the budget this one deliberately sabotages.
set "IMPRECISE_LOG=%TEMP%\keller-nested-loop-imprecise.log"
pushd "%ROOT%\tests\nested_loop"
if exist target rmdir /s /q target
if exist report.json del report.json
set "KELLER_MAX_ROUNDS=1"
"%VERIFIER%" keller verify >"%IMPRECISE_LOG%" 2>&1
set "IMPRECISE_EXIT=%ERRORLEVEL%"
set "KELLER_MAX_ROUNDS="
popd
if not "%IMPRECISE_EXIT%"=="1" (
  echo [FAIL] nested_loop: expected exit 1 with the round budget cut to one round, got %IMPRECISE_EXIT%
  set "FAILED=1"
) else (
  echo [PASS] nested_loop: a cut round budget still exits 1 on the flags it leaves behind
)
findstr /C:"Path-insensitive fallback in nested_walk" "%IMPRECISE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] nested_loop: the lost precision is not announced on the console
  set "FAILED=1"
) else (
  echo [PASS] nested_loop: the lost precision is announced on the console
)
rem Recorded, not merely printed: the console is for whoever is watching, and
rem report.json is what a pipeline and the next reader see. The note has to state
rem the direction as well as the number - forgetting a guard only weakens the
rem precondition, so the FAILs are the suspect verdicts and the PASSes are not.
for %%N in (path-insensitive "\"body\"" "\"function\": \"nested_walk\"" "round budget (1 round)" "false positive") do (
  findstr /C:%%N "%ROOT%\tests\nested_loop\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] nested_loop: report.json is missing %%N, so the loss is unattributable
    set "FAILED=1"
  ) else (
    echo [PASS] nested_loop: the fallback is recorded with %%N
  )
)
rem And the false positives it explains are there: the two increments the default
rem budget proves are flagged as soon as the guards are gone.
for %%N in ("\"bb4\"" "\"bb8\"") do (
  findstr /C:%%N "%ROOT%\tests\nested_loop\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] nested_loop: %%N should be flagged with the budget cut to one round
    set "FAILED=1"
  ) else (
    echo [PASS] nested_loop: %%N is flagged once the guards are gone, as the note warns
  )
)

call :run_table_case nested_loop

rem The nested-loop probe. loop_fail above is a single cycle; this crate is an
rem inner loop inside an outer one, with continue in the inner body and break in
rem the outer, and it is where the loop abstraction used to stop working. Its body
rem fell back to the path-insensitive state because the *condition* dimension was
rem never widened, so every increment was reported - including the two the guards
rem prove. PathAnalysis::compute now widens the condition too: past MAX_LOOP_TRIPS
rem a cycle block's in-state is rebuilt from its forward contributions alone, so
rem the back edge no longer re-feeds the block's condition into itself. The two
rem guarded increments must therefore be proved now (absent from the report - a
rem proof writes no entry), the one real counterexample must stay, and the
rem single-cycle control must stay proved. run_tests.sh asserts the same thing and
rem also checks that the counterexample's obligation carries both headers' guards.
for %%N in ("total (_3) = 255" nested_walk) do (
  findstr /C:%%N "%ROOT%\tests\nested_loop\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] nested_loop: report is missing %%N - the nested body lost the reachable overflow
    set "FAILED=1"
  ) else (
    echo [PASS] nested_loop: report carries %%N
  )
)
rem The two proved increments and the control must not appear: bb4 and bb8 are
rem bounded by their guards, and single_counter is a single cycle.
for %%N in (bb4 bb8 single_counter) do (
  findstr /C:%%N "%ROOT%\tests\nested_loop\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [PASS] nested_loop: %%N is proved, so it is absent from the report
  ) else (
    echo [FAIL] nested_loop: %%N must be proved, but it is reported
    set "FAILED=1"
  )
)

call :run_table_case shift_amount

rem Shifts, asserted in the direction that was broken. The constant path used to
rem call `200u8 << 1` an overflow although Rust truncates there and never panics,
rem while a symbolic shift amount was not checked at all. So the shift whose
rem amount can exceed the width must still be reported, and the two functions
rem whose shifts cannot panic must stay out of the report. The failing input is
rem the *amount*, not the shifted value, so the check names the exact value
rem (`by = 32`) the obligation pins rather than only the finding's presence.
findstr /C:"Exploit path found" "%ROOT%\tests\shift_amount\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] shift_amount: the out-of-range shift amount must be reported
  set "FAILED=1"
) else (
  echo [PASS] shift_amount: the out-of-range shift amount is reported
)
findstr /C:"by (_2) = 32" "%ROOT%\tests\shift_amount\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] shift_amount: the finding must name the out-of-range amount ^(by = 32^), not the shifted value
  set "FAILED=1"
) else (
  echo [PASS] shift_amount: the finding names the out-of-range amount ^(by = 32^)
)
for %%N in (truncate discard_high_bits) do (
  findstr /C:%%N "%ROOT%\tests\shift_amount\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [PASS] shift_amount: %%N cannot panic, so it is absent from the report
  ) else (
    echo [FAIL] shift_amount: %%N cannot panic and must be proved, but it is reported
    set "FAILED=1"
  )
)

rem A target project that pins its own toolchain. `cargo` is a rustup shim and
rem follows that pin, and stable cargo rejects the -Z flag the verify command
rem passes, so the run used to end as a setup failure (exit 3) with no verdict on
rem code that compiles cleanly. This case is the pin itself; the counterexample
rem below is what proves the analysis ran, and the guarded increment beside it has
rem to be proved. The exact witness is pinned by the golden and by
rem tests\run_tests.sh.
call :run_table_case stable_pin

findstr /C:"unguarded_increment" "%ROOT%\tests\stable_pin\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] stable_pin: the unguarded increment must still be reported
  set "FAILED=1"
) else (
  echo [PASS] stable_pin: a crate with its own toolchain pin is still analysed
)
rem `findstr /C:"guarded_increment"` would also match `unguarded_increment`, and
rem findstr cannot spell a quote inside /C:"..." (see :check_env below), so the
rem proof that the guarded increment was *proved* is that the report names exactly
rem one function. Only the counterexample may be there.
call :count_lines "%ROOT%\tests\stable_pin\report.json" "function" SP_FUNCS
if "%SP_FUNCS%"=="1" (
  echo [PASS] stable_pin: the guarded increment is proved under the project's toolchain pin
) else (
  echo [FAIL] stable_pin: the report names %SP_FUNCS% functions; the guarded increment must be proved, not reported
  set "FAILED=1"
)

call :run_table_case fold_fail
rem The folded path used to be a second-class FAIL: the operands were prose in
rem `details`, no witness was recorded, and nothing was written under
rem obligations\. run_case above already checked the witness and the pinned
rem file, the golden compares the whole report byte for byte, and
rem :run_table_case re-discharges the ground obligation with z3 - the same
rem three checks tests/run_tests.sh makes.
findstr /C:"Counterexample found by constant folding" "%ROOT%\tests\fold_fail\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] fold_fail: the folded FAILs do not say how they were decided
  set "FAILED=1"
) else (
  echo [PASS] fold_fail: the folded FAILs identify the path that found them
)
rem The obligation header records the verdict as "FAIL (folded)", so the file on
rem disk says which path produced it and a reader is not left to guess whether
rem the formula came from the solver or from folding.
findstr /S /C:"FAIL (folded)" "%ROOT%\tests\fold_fail\obligations\*.smt2" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] fold_fail: no obligation records that it came from constant folding
  set "FAILED=1"
) else (
  echo [PASS] fold_fail: the obligations are marked as constant-folded
)

rem A crate the compiler rejected is a *setup* failure, not a verdict. The
rem wrapper used to discard rustc's exit status, so this came out as exit 0 with
rem an empty report.json - the analysis found no assertions in a body rustc never
rem built, and "no findings" is not "nothing to check".
call :run_table_case broken_code

rem The exit-code contract, both halves. --deny-warn may only ever *tighten* it:
rem a run that is clean without the flag stays clean with it, and a counterexample
rem is still fatal. The WARN half used to be covered only by the unit tests in
rem src/report.rs, because no fixture produced an `unknown` verdict - so what is
rem checked below is that the flag reaches the top-level gate at all, and that a
rem misspelled flag is refused rather than silently leaving the stricter gate
rem switched off. The undecided verdict itself is exercised end to end right after
rem these three, against a real witness of it: tests\undecided.
rem safe_code's null and alignment checks are proved from pointer provenance now, so
rem the strict gate has nothing left to reject in it. The WARN half is exercised on a
rem real undecided obligation by tests\undecided below, and on a counterexample by
rem overflow_fail: --deny-warn must still be able to fail, it just no longer does here.
call :run_gate safe_code 0 --deny-warn
call :run_gate overflow_fail 1 --deny-warn
call :run_gate safe_code 2 --deny-warns
rem The other half of that contract, end to end and with a real undecided
rem obligation. tests\undecided is the crate that produces one: reaching the
rem body of guarded_sum needs `a * b == 2^61 - 1` with both factors above 1, and
rem 2^61 - 1 is prime, so the body is unreachable - but only factorisation shows
rem that, which is exactly what Z3's nonlinear *integer* arithmetic cannot do.
rem
rem The obligation needs a budget to be undecided rather than open-ended: with no
rem budget Z3 does not answer `unknown`, it does not answer at all (measured, it is
rem still running after 90 seconds). The crate's other obligation is a bound Z3
rem does decide, in about 0.4s, so 5s separates the two by more than an order of
rem magnitude on either side. Both runs clear target first: cargo's fingerprints
rem do not notice that only the verifier's budget changed, so a warm target would
rem mean no worker ran at all - and a gate with no findings passes, which would
rem make this probe assert nothing.
rem
rem The whole report is compared against its golden byte for byte, which is what
rem pins the status, the class and the function - and pins that nothing was
rem laundered into a counterexample. The obligation itself is re-discharged
rem below; its verdict is a budget answer, so only its validity is required.
set "UND_LOG=%TEMP%\keller-undecided.log"
pushd "%ROOT%\tests\undecided"
if exist target rmdir /s /q target
if exist report.json del report.json
"%VERIFIER%" keller verify --solver-timeout 5000 >"%UND_LOG%" 2>&1
set "UND_EXIT=%ERRORLEVEL%"
popd
rem This case needs a solver budget, so it is not driven by :run_table_case; it
rem marks itself consumed so :check_case_table can tell that from an oversight.
call :mark_case undecided
if not "%UND_EXIT%"=="0" (
  echo [FAIL] undecided: expected exit 0 with a WARN and no FAIL, got %UND_EXIT%
  set "FAILED=1"
) else (
  echo [PASS] undecided: an undecided obligation does not fail the build by default
)
findstr /C:"guarded_sum" "%ROOT%\tests\undecided\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] undecided: the undecided obligation is not attributed to guarded_sum
  set "FAILED=1"
) else (
  echo [PASS] undecided: the undecided verdict is recorded against guarded_sum
)
rem One WARN and nothing else: the same obligation, run again, comes out the same.
rem Through :check_golden, so which cases have a golden and where it is compared
rem is one decision, read from tests\cases.txt, rather than one made here.
call :check_golden undecided
rem A verdict nothing announces is the failure mode this probe exists for.
findstr /C:"[WARN] Complex state in guarded_sum" "%UND_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] undecided: an undecided verdict that nothing announces reads like a pass
  set "FAILED=1"
) else (
  echo [PASS] undecided: the undecided verdict is announced on the console
)
if exist "%ROOT%\tests\undecided\obligations\undecided\guarded_sum_bb6.witness.smt2" (
  echo [FAIL] undecided: an undecided verdict has no model, so it must pin no witness
  set "FAILED=1"
) else (
  echo [PASS] undecided: no witness is pinned for a verdict with no model
)
pushd "%ROOT%\tests\undecided"
if exist target rmdir /s /q target
"%VERIFIER%" keller verify --solver-timeout 5000 --deny-warn >"%UND_LOG%" 2>&1
set "UND_DENIED=%ERRORLEVEL%"
popd
if not "%UND_DENIED%"=="1" (
  echo [FAIL] undecided: expected exit 1 under --deny-warn, got %UND_DENIED%
  set "FAILED=1"
) else (
  echo [PASS] undecided: the same obligation is fatal under --deny-warn
)
findstr /C:"are undecided (WARN) and --deny-warn was given" "%UND_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] undecided: the gate failed without naming the undecided finding
  set "FAILED=1"
) else (
  echo [PASS] undecided: the gate says which verdict it failed on
)
call :check_obligations undecided

rem ===========================================================================
rem  A run that stops making progress stops the run (K3).
rem
rem  `tests/undecided` with no solver budget at all is an unbounded Z3 call, so
rem  the wall-clock cap is what has to stop it. What this case pins is the
rem  contract a pipeline can rely on whatever the solver does: the run stops
rem  instead of waiting, it says which crate was still in flight, it records
rem  itself as a stopped run and not as a verdict, it publishes nothing over the
rem  last completed report, and the process tree it started is gone.
rem
rem  That last one has already been a shipped failure: killing the top-level
rem  verifier leaves the workspace wrappers and their cargo alive. The counts
rem  below are taken before and after, so a cap that kills only its direct child
rem  fails here even though every other assertion passes.
rem ===========================================================================
pushd "%ROOT%\tests\undecided"
if exist target rmdir /s /q target
if exist report.json del report.json
popd
set "CAP_RUNS_BEFORE=%TEMP%\keller-undecided-runs-before-%RANDOM%.txt"
dir /b /ad "%ROOT%\tests\undecided\.keller\runs" >"%CAP_RUNS_BEFORE%" 2>nul
set "SURV_BEFORE=0"
for /f %%C in ('powershell -NoProfile -Command "(Get-Process -Name workspace-wrapper -ErrorAction SilentlyContinue | Measure-Object).Count"') do set "SURV_BEFORE=%%C"
for /f %%C in ('powershell -NoProfile -Command "(Get-Process -Name cargo -ErrorAction SilentlyContinue | Measure-Object).Count"') do set /a SURV_BEFORE+=%%C
set "CAP_SECONDS=20"
for /f %%T in ('powershell -NoProfile -Command "[DateTimeOffset]::UtcNow.ToUnixTimeSeconds()"') do set "CAP_STARTED=%%T"
pushd "%ROOT%\tests\undecided"
"%VERIFIER%" keller verify --solver-timeout 0 --run-timeout %CAP_SECONDS% >"%TEMP%\keller-run-timeout.log" 2>&1
set "CAP_EXIT=%ERRORLEVEL%"
popd
for /f %%T in ('powershell -NoProfile -Command "[DateTimeOffset]::UtcNow.ToUnixTimeSeconds()"') do set "CAP_ENDED=%%T"
set /a CAP_ELAPSED=CAP_ENDED-CAP_STARTED
if not "%CAP_EXIT%"=="3" (
  echo [FAIL] run_timeout: expected exit 3 from a run stopped at the cap, got %CAP_EXIT%
  set "FAILED=1"
) else (
  echo [PASS] run_timeout: a run stopped at the cap reaches no verdict ^(exit 3^)
)
rem Slack over the cap: the deadline is polled, and a cold build may still be
rem running when it fires. What this rules out is the failure that matters - a
rem run that ignores the cap and keeps going.
set /a CAP_LIMIT=CAP_SECONDS+25
if %CAP_ELAPSED% LEQ %CAP_LIMIT% (
  echo [PASS] run_timeout: it stopped after %CAP_ELAPSED%s for a %CAP_SECONDS%s cap
) else (
  echo [FAIL] run_timeout: ran for %CAP_ELAPSED%s under a %CAP_SECONDS%s cap
  set "FAILED=1"
)
for %%N in ("Stopped at the --run-timeout cap (%CAP_SECONDS% s)" "in flight: undecided") do (
  findstr /C:%%N "%TEMP%\keller-run-timeout.log" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] run_timeout: the stopped run does not report %%N
    set "FAILED=1"
  ) else (
    echo [PASS] run_timeout: the stopped run reports %%N
  )
)
rem Evidence from this run and no other: a record that predates the case would
rem pass the checks below while reporting on a different run entirely.
set "CAP_RUN="
for /f "delims=" %%D in ('dir /b /ad /o-d "%ROOT%\tests\undecided\.keller\runs" 2^>nul') do if not defined CAP_RUN set "CAP_RUN=%%D"
if not defined CAP_RUN (
  echo [FAIL] run_timeout: the stopped run left no run directory, so nothing records it
  set "FAILED=1"
  goto :run_timeout_record_done
)
findstr /x /C:"%CAP_RUN%" "%CAP_RUNS_BEFORE%" >nul 2>&1
if not errorlevel 1 (
  echo [FAIL] run_timeout: the newest run directory predates this case, so it is not its evidence
  set "FAILED=1"
)
set "CAP_RECORD=%ROOT%\tests\undecided\.keller\runs\%CAP_RUN%\report.run.json"
findstr /C:"run-timeout" "%CAP_RECORD%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] run_timeout: no run record at '%CAP_RECORD%' says run-timeout
  set "FAILED=1"
) else (
  echo [PASS] run_timeout: the stopped run records itself as run-timeout
)
findstr /C:"gate-passed" /C:"gate-failed" "%CAP_RECORD%" >nul 2>&1
if not errorlevel 1 (
  echo [FAIL] run_timeout: a stopped run recorded a verdict it never reached
  set "FAILED=1"
) else (
  echo [PASS] run_timeout: no verdict is recorded for a run that reached none
)
:run_timeout_record_done
if exist "%ROOT%\tests\undecided\report.json" (
  echo [FAIL] run_timeout: the stopped run published a report over the last completed one
  set "FAILED=1"
) else (
  echo [PASS] run_timeout: nothing was published in place of the last completed report
)
set "SURV_AFTER=0"
for /f %%C in ('powershell -NoProfile -Command "(Get-Process -Name workspace-wrapper -ErrorAction SilentlyContinue | Measure-Object).Count"') do set "SURV_AFTER=%%C"
for /f %%C in ('powershell -NoProfile -Command "(Get-Process -Name cargo -ErrorAction SilentlyContinue | Measure-Object).Count"') do set /a SURV_AFTER+=%%C
if %SURV_AFTER% GTR %SURV_BEFORE% (
  echo [FAIL] run_timeout: the cap left %SURV_AFTER% process^(es^) running, up from %SURV_BEFORE%
  set "FAILED=1"
) else (
  echo [PASS] run_timeout: the stopped run left no process behind ^(%SURV_BEFORE% before, %SURV_AFTER% after^)
)
del "%CAP_RUNS_BEFORE%" >nul 2>&1
pushd "%ROOT%\tests\undecided"
if exist target rmdir /s /q target
popd

rem ===========================================================================
rem  Where a write is spelled, not what it writes (A18).
rem
rem  apply_assign was the only thing that retired anything, and the call
rem  terminator retired only its own destination, so every other way a body this
rem  analysis does not read can write - a callee's `&mut` argument, drop glue, an
rem  intrinsic's store, an asm block's output - was invisible: four functions in
rem  tests\statement_writes panic on every input their guard admits, and all four
rem  were *proved* before the rule was generalised. tests\unnamed_write is the
rem  single-store shape beside them.
rem
rem  Both are asserted in the direction they failed: not proved, not laundered
rem  into a counterexample, but undecided with the write named - and the control
rem  whose store the analysis *can* name stays proved, because giving up on every
rem  reference would buy those WARNs with a false positive. tests/run_tests.sh
rem  asserts the same things and also re-discharges every obligation in the crate.
rem ===========================================================================

rem  tests\unnamed_write: a value answered from storage this analysis cannot
rem  follow is not a counterexample. The obligation that used to be printed as a
rem  proof from the overwritten value is announced undecided instead.
set "UW_LOG=%TEMP%\keller-unnamed-write.log"
pushd "%ROOT%\tests\unnamed_write"
if exist target rmdir /s /q target
if exist report.json del report.json
"%VERIFIER%" keller verify >"%UW_LOG%" 2>&1
set "UW_EXIT=%ERRORLEVEL%"
popd
call :mark_case unnamed_write
if not "%UW_EXIT%"=="0" (
  echo [FAIL] unnamed_write: expected exit 0 with a WARN and no FAIL, got %UW_EXIT%
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: a check answered from storage this analysis cannot follow does not fail the build by default
)
rem The verdict has to be the honest one and it has to name the store: the point
rem of retiring the storage is that a reader can see which write cost the value.
findstr /C:"\"status\": \"WARN\"" "%ROOT%\tests\unnamed_write\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] unnamed_write: the undecided verdict is not recorded as a WARN
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: the obligation is recorded as a WARN
)
findstr /C:"\"function\": \"ref_write\"" "%ROOT%\tests\unnamed_write\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] unnamed_write: the undecided verdict is not attributed to ref_write
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: the undecided verdict is recorded against ref_write
)
findstr /C:"is written through a projection this analysis cannot name" "%ROOT%\tests\unnamed_write\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] unnamed_write: the finding does not name the write that cost the value
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: the finding names the write that cost the value
)
findstr /C:"\"status\": \"FAIL\"" "%ROOT%\tests\unnamed_write\report.json" >nul 2>&1
if errorlevel 1 (
  echo [PASS] unnamed_write: the decline is not laundered into a counterexample
) else (
  echo [FAIL] unnamed_write: a store the analysis cannot attribute was laundered into a counterexample
  set "FAILED=1"
)
rem The regression itself, in the direction it failed: this obligation used to be
rem printed as [PASS] and taken from the value the store had already overwritten.
rem It is the assertion that reddens if the stale fact comes back.
findstr /C:"[PASS] Verified formal integrity: ref_write (bb3)" "%UW_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [PASS] unnamed_write: ref_write bb3 is not proved from the value the store destroyed
) else (
  echo [FAIL] unnamed_write: ref_write bb3 is proved again, so the stale value is back
  set "FAILED=1"
)
findstr /C:"[WARN] Undecided in ref_write (bb3)" "%UW_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] unnamed_write: a verdict nothing announces reads like a pass
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: the undecided verdict is announced on the console
)
rem The precision half: the named store retires and re-derives its value, and the
rem guard still bounds it, so both of the control's obligations stay proved.
findstr /C:"[PASS] Verified formal integrity: named_store_is_kept (bb2)" "%UW_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] unnamed_write: the named store lost its bb2 proof, so the retirement gave up a value it could name
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: the named store keeps its proof in bb2
)
findstr /C:"[PASS] Verified formal integrity: named_store_is_kept (bb3)" "%UW_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] unnamed_write: the named store lost its bb3 proof, so the retirement gave up a value it could name
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: the named store keeps its proof in bb3
)
call :check_obligations unnamed_write
call :check_golden unnamed_write
rem The counterexample is real and the tool declines to *name* it, rather than
rem inventing an input or quietly keeping the one the store destroyed, so the
rem emitted obligation re-discharges to sat. The same check tests/run_tests.sh
rem makes with the same solver the verifier was linked against.
call :z3_answer "%ROOT%\tests\unnamed_write\obligations\unnamed_write\ref_write_bb3.smt2"
if not defined Z3_BIN (
  echo [SKIP] unnamed_write: no z3 binary to re-discharge the obligation
) else if "%ANSWER%"=="sat" (
  echo [PASS] unnamed_write: the undecided obligation re-discharges to sat, so the counterexample is there and the attribution is what is missing
) else (
  echo [FAIL] unnamed_write: expected the obligation to re-discharge to sat, got %ANSWER%
  set "FAILED=1"
)
rem A verdict that names no input pins no witness, for the same reason tests\undecided
rem does not: there is no model the tool is willing to stand behind.
if exist "%ROOT%\tests\unnamed_write\obligations\unnamed_write\ref_write_bb3.witness.smt2" (
  echo [FAIL] unnamed_write: an undecided verdict names no input, so it must pin no witness
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: no witness is pinned for a verdict that names no input
)
rem And the gate, so the finding is not merely cosmetic: the same obligation
rem fails a project that asks for it.
pushd "%ROOT%\tests\unnamed_write"
if exist target rmdir /s /q target
"%VERIFIER%" keller verify --deny-warn >"%UW_LOG%" 2>&1
set "UW_DENIED=%ERRORLEVEL%"
popd
if not "%UW_DENIED%"=="1" (
  echo [FAIL] unnamed_write: expected exit 1 under --deny-warn, got %UW_DENIED%
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: the undecided obligation is fatal under --deny-warn
)
findstr /C:"are undecided (WARN) and --deny-warn was given" "%UW_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] unnamed_write: the gate failed without naming the undecided finding
  set "FAILED=1"
) else (
  echo [PASS] unnamed_write: the gate says which verdict it failed on
)

rem  tests\statement_writes: the same rule where a *write* is spelled rather
rem  than a value reassigned - a callee handed a pointer into this body, drop
rem  glue, an intrinsic's store, an asm block's output. Four functions panic on
rem  every input their guard admits and all four were proved before the rule was
rem  generalised. The two controls are the precision half: a value the body's own
rem  MIR says the callee names stays proved, and a call handed no pointer into
rem  this body could not have written anything.
set "SW_LOG=%TEMP%\keller-statement-writes.log"
pushd "%ROOT%\tests\statement_writes"
if exist target rmdir /s /q target
if exist report.json del report.json
"%VERIFIER%" keller verify >"%SW_LOG%" 2>&1
set "SW_EXIT=%ERRORLEVEL%"
popd
call :mark_case statement_writes
if not "%SW_EXIT%"=="0" (
  echo [FAIL] statement_writes: expected exit 0 with WARNs and no FAIL, got %SW_EXIT%
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: a check answered from storage an unread body wrote does not fail the build by default
)
rem The finding has to name the write that cost the value, one per shape.
findstr /C:"the call to `bump` is given a pointer into this body" "%ROOT%\tests\statement_writes\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] statement_writes: the finding does not name the callee's pointer into this body
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: the retirement names the callee's write
)
findstr /C:"a drop here runs glue this analysis does not read" "%ROOT%\tests\statement_writes\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] statement_writes: the finding does not name the drop glue as the write
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: the retirement names the drop glue
)
findstr /C:"is given a pointer into this body" "%ROOT%\tests\statement_writes\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] statement_writes: the finding does not name the intrinsic's store
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: the retirement names the intrinsic's store
)
findstr /C:"is written by an inline asm block" "%ROOT%\tests\statement_writes\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] statement_writes: the finding does not name the asm block's output
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: the retirement names the asm block's write
)
findstr /C:"\"status\": \"FAIL\"" "%ROOT%\tests\statement_writes\report.json" >nul 2>&1
if errorlevel 1 (
  echo [PASS] statement_writes: the declines are not laundered into counterexamples
) else (
  echo [FAIL] statement_writes: a write the analysis cannot attribute was laundered into a counterexample
  set "FAILED=1"
)
rem The regression itself, in the direction it failed: each of these obligations
rem was printed as [PASS] from the value the unread body had already overwritten.
for %%N in (call_writes drop_writes intrinsic_writes) do (
  findstr /C:"[PASS] Verified formal integrity: %%N (bb4)" "%SW_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [PASS] statement_writes: %%N bb4 is not proved from the value the write destroyed
  ) else (
    echo [FAIL] statement_writes: %%N bb4 is proved again, so the value the write destroyed is back
    set "FAILED=1"
  )
  findstr /C:"[WARN] Undecided in %%N (bb4)" "%SW_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] statement_writes: %%N bb4 is undecided but nothing announces it, which reads like a pass
    set "FAILED=1"
  ) else (
    echo [PASS] statement_writes: the undecided verdict for %%N bb4 is announced on the console
  )
)
rem The assertion after the asm block is inside the walk now. Before the walk knew
rem the InlineAsm terminator this file did not exist: nothing had reached the
rem block, so the summary neither checked its assertion nor counted it as missed.
if "%SW_ASM%"=="1" (
  if exist "%ROOT%\tests\statement_writes\obligations\statement_writes\asm_writes_bb4.smt2" (
    echo [PASS] statement_writes: the assertion after the asm block is reached, so it is checked
  ) else (
    echo [FAIL] statement_writes: the walk stopped at the asm block, so nothing follows it in the analysis
    set "FAILED=1"
  )
) else (
  echo [SKIP] statement_writes: no x86-64 asm in this build, so the asm shape is not in the crate
)
findstr /C:"[PASS] Verified formal integrity: unrelated_local_is_kept (bb4)" "%SW_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] statement_writes: the unrelated local lost its proof, so a value no unread body could have written was given up
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: the control keeps its proof for the unrelated local
)
findstr /C:"[PASS] Verified formal integrity: no_pointer_argument_is_kept (bb4)" "%SW_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] statement_writes: the no-pointer control lost its proof, so a call that cannot write here was treated as if it had
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: the control handed no pointer keeps its proof
)
call :check_obligations statement_writes
call :check_golden statement_writes
rem The formula is satisfiable, and that is the point of the case: the
rem counterexample is real (`bump` really does write 250) and the tool declines to
rem *name* it, rather than inventing an input or quietly keeping the one the
rem callee destroyed.
call :z3_answer "%ROOT%\tests\statement_writes\obligations\statement_writes\call_writes_bb4.smt2"
if not defined Z3_BIN (
  echo [SKIP] statement_writes: no z3 binary to re-discharge the obligation
) else if "%ANSWER%"=="sat" (
  echo [PASS] statement_writes: the undecided obligation re-discharges to sat, so the counterexample is there and the attribution is what is missing
) else (
  echo [FAIL] statement_writes: expected the obligation to re-discharge to sat, got %ANSWER%
  set "FAILED=1"
)
if exist "%ROOT%\tests\statement_writes\obligations\statement_writes\call_writes_bb4.witness.smt2" (
  echo [FAIL] statement_writes: an undecided verdict names no input, so it must pin no witness
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: no witness is pinned for a verdict that names no input
)
pushd "%ROOT%\tests\statement_writes"
if exist target rmdir /s /q target
"%VERIFIER%" keller verify --deny-warn >"%SW_LOG%" 2>&1
set "SW_DENIED=%ERRORLEVEL%"
popd
if not "%SW_DENIED%"=="1" (
  echo [FAIL] statement_writes: expected exit 1 under --deny-warn, got %SW_DENIED%
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: the undecided obligations are fatal under --deny-warn
)
findstr /C:"are undecided (WARN) and --deny-warn was given" "%SW_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] statement_writes: the gate failed without naming the undecided findings
  set "FAILED=1"
) else (
  echo [PASS] statement_writes: the gate says which verdict it failed on
)

rem ===========================================================================
rem  The precision probe: safe Rust that cannot panic, asserted clean (K5).
rem
rem  tests\potency_probe is eighteen functions of safe Rust. Three of its
rem  obligations used to be counterexamples, in the same shape: a loop's own
rem  condition did not reach the assertion it guards, so the solver was free to
rem  choose the loop's exit value. The loop relation in widened_state closed that,
rem  so this block asserts the end state instead of pinning the debt - each line
rem  below fails the suite if a verdict moves back.
rem
rem  The last one, fixed_array, went further: its index came out of Range::next,
rem  a call this analysis does not model, so the obligation was undecided. S3/S4
rem  give the iterator's bound and the slice's length one term, so it decides too.
rem  Every function here cannot panic, so an undecided verdict is now a
rem  regression of its own.
rem
rem  Deliberately not in tests\goldens and not in the measured manifest: that
rem  corpus is what the false-positive rate in docs/FALSE-POSITIVE-RATE.md is
rem  computed over, and publishing counterexamples from a crate that cannot panic
rem  would be a claim the tool has not earned.
rem ===========================================================================
set "PROBE_LOG=%TEMP%\keller-potency-probe.log"
pushd "%ROOT%\tests\potency_probe"
if exist target rmdir /s /q target
if exist report.json del report.json
"%VERIFIER%" keller verify >"%PROBE_LOG%" 2>&1
set "PROBE_EXIT=%ERRORLEVEL%"
popd
call :mark_case potency_probe
call :count_lines "%PROBE_LOG%" "[FAIL]" PROBE_FAILS
set "PROBE_OK=1"
if not "%PROBE_EXIT%"=="0" set "PROBE_OK=0"
if not "%PROBE_FAILS%"=="0" set "PROBE_OK=0"
if "%PROBE_OK%"=="1" (
  echo [PASS] potency_probe: eighteen functions that cannot panic report no counterexample
) else (
  echo [FAIL] potency_probe: exit %PROBE_EXIT% with %PROBE_FAILS% false positives, so K5 has regressed
  set "FAILED=1"
)
rem The loop the relation was written for: its three obligations are *decided*, not
rem merely absent. An undecided loop is a regression too, and so is a missing
rem line, which is why the line itself is grepped for.
findstr /C:"[PASS] Verified formal integrity: chunk_loop (bb1)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: chunk_loop bb1 is not decided PASS, so K5 has regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: chunk_loop bb1 is decided because its guard reaches it
)
findstr /C:"[PASS] Verified formal integrity: chunk_loop (bb3)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: chunk_loop bb3 is not decided PASS, so K5 has regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: chunk_loop bb3 is decided because its guard reaches it
)
findstr /C:"[PASS] Verified formal integrity: chunk_loop (bb5)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: chunk_loop bb5 is not decided PASS, so K5 has regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: chunk_loop bb5 is decided because its guard reaches it
)
rem The safe-arithmetic idioms (K9): each is decided only while the idiom has an
rem encoding - a saturating sum is never below its right operand, a checked sum in
rem its Some arm is the exact sum, a wrapping sum cannot wrap when both inputs are
rem under 100, and a narrowing cast is a modulus.
findstr /C:"[PASS] Verified formal integrity: saturating_never_below (bb1)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: saturating_never_below bb1 is not decided, so its idiom's encoding regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: the saturating sum's bound is decided
)
findstr /C:"[PASS] Verified formal integrity: checked_sum_gives_the_sum (bb4)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: checked_sum_gives_the_sum bb4 is not decided, so the checked idiom's encoding regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: the checked sum's payload is decided
)
findstr /C:"[PASS] Verified formal integrity: wrapping_sum_under_input_bound (bb4)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: wrapping_sum_under_input_bound bb4 is not decided, so the wrapping idiom's encoding regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: the wrapping sum's bound is decided
)
findstr /C:"[PASS] Verified formal integrity: narrowed_under_input_bound (bb2)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: narrowed_under_input_bound bb2 is not decided, so the narrowing cast's encoding regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: the narrowing cast's bound is decided
)
rem And the artefact, not just the console: the body's index is related to the
rem previous trip's counter under that trip's own guard, and the length it is
rem checked against is bounded. A proof that stopped naming the previous trip, or
rem stopped bounding the length, would still re-discharge - it would just no
rem longer be the argument this asserts.
set "PROBE_BB3=%ROOT%\tests\potency_probe\obligations\potency_probe\chunk_loop_bb3.smt2"
if not exist "%PROBE_BB3%" (
  echo [FAIL] potency_probe: no obligation was emitted for chunk_loop bb3
  set "FAILED=1"
) else (
  findstr /C:"(<= (+ mir_3 8) len_1)" "%PROBE_BB3%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] potency_probe: the loop obligation no longer bounds the index by the length
    set "FAILED=1"
  ) else (
    echo [PASS] potency_probe: the loop obligation bounds the index by the length
  )
  findstr /C:"(= mir_3 (+ prev_bb6_" "%PROBE_BB3%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] potency_probe: the loop obligation no longer names the previous trip's counter
    set "FAILED=1"
  ) else (
    echo [PASS] potency_probe: the loop obligation names the previous trip's counter
  )
  findstr /C:"(<= len_1 9223372036854775807)" "%PROBE_BB3%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] potency_probe: the loop obligation no longer bounds the length
    set "FAILED=1"
  ) else (
    echo [PASS] potency_probe: the loop obligation bounds the length
  )
)
rem fixed_array's index comes out of `Range::next`, an unmodelled call, and used
rem to make its obligation undecided. The iterator's bound and the slice length
rem are now the same term, so it decides. Any undecided obligation here is a
rem precision regression: every function in this crate cannot panic.
findstr /C:"[PASS] Verified formal integrity: fixed_array (bb5)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: fixed_array bb5 is no longer decided, so the iterator bound regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: fixed_array bb5 is decided through the iterator's own bound
)
rem through_local_ref is S5: the pointer borrows one local and is never pointed
rem anywhere else, so `*r` reads the value recorded for that local - the guard and
rem the bounds check are then one quantity. Without the resolution the read has no
rem term at all and the index is undecided rather than proved.
findstr /C:"[PASS] Verified formal integrity: through_local_ref (bb1)" "%PROBE_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] potency_probe: through_local_ref bb1 is not decided, so dereference resolution regressed
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: a read through a local's reference is decided
)

call :count_lines "%PROBE_LOG%" "Undecided in" PROBE_WARNS
if %PROBE_WARNS% NEQ 0 (
  echo [FAIL] potency_probe: %PROBE_WARNS% obligations left undecided in a crate that cannot panic
  set "FAILED=1"
) else (
  echo [PASS] potency_probe: nothing in a crate that cannot panic is left undecided
)

rem ===========================================================================
rem  The panics a report does not mention (A6).
rem
rem  Every case above is a proof about an assertion the compiler inserted.
rem  `unwrap`, `expect`, `panic!`, `unreachable!` and the failure path of a user
rem  `assert!` are calls into another crate whose body this analysis does not read,
rem  and before this pass they were walked past in silence. tests\potency_probe
rem  below asserts that a crate which cannot panic reports nothing - and a crate
rem  full of `unwrap`s reported nothing as well, which is what this case exists to
rem  tell apart.
rem
rem  The same claims tests/run_tests.sh asserts, from the same fixture: six shapes,
rem  one per function, named on the console; exactly six, so a call to a local
rem  function is not counted again at its caller; no FAIL; the assertion counts
rem  unchanged; the two controls still decided; and --deny-warn as the thing that
rem  makes one fatal. What is asserted here and not there is only the reading of
rem  report.run.json, which findstr cannot do with a quoted key - the console line
rem  carrying the same number is checked instead.
rem ===========================================================================
set "PS_LOG=%TEMP%\keller-panic-sites.log"
pushd "%ROOT%\tests\panic_sites"
if exist target rmdir /s /q target
if exist report.json del report.json
if exist .keller rmdir /s /q .keller
"%VERIFIER%" keller verify >"%PS_LOG%" 2>&1
set "PS_EXIT=%ERRORLEVEL%"
popd
call :mark_case panic_sites
if not "%PS_EXIT%"=="0" (
  echo [FAIL] panic_sites: expected exit 0 with WARNs and no FAIL, got %PS_EXIT%
  set "FAILED=1"
) else (
  echo [PASS] panic_sites: a crate whose functions are full of `unwrap`s does not fail the build by default
)
for %%S in (unwrapped expected explicit_program_panic never_reached guarded_by_assert asserted_equal) do (
  findstr /C:"[WARN] Unmodelled panic site in %%S (" "%PS_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] panic_sites: %%S's panic site is not reported, so a path nothing says anything about reads like a proved one
    set "FAILED=1"
  ) else (
    echo [PASS] panic_sites: %%S's panic site is named on the console
  )
)
call :count_lines "%PS_LOG%" "[WARN] Unmodelled panic site in" PS_NAMED
if not "%PS_NAMED%"=="6" (
  echo [FAIL] panic_sites: expected 6 named panic sites, got %PS_NAMED%
  set "FAILED=1"
) else (
  echo [PASS] panic_sites: exactly six sites are named, so a local callee is not counted again at its caller
)
call :count_lines "%ROOT%\tests\panic_sites\report.json" "unmodelled-panic" PS_RECORDED
if not "%PS_RECORDED%"=="6" (
  echo [FAIL] panic_sites: report.json records %PS_RECORDED% panic sites, not 6
  set "FAILED=1"
) else (
  echo [PASS] panic_sites: the report records all six, so the console and the artifact cannot disagree
)
rem A panic site is not a counterexample: nothing was proved false.
call :count_lines "%ROOT%\tests\panic_sites\report.json" "FAIL" PS_FAILS
if not "%PS_FAILS%"=="0" (
  echo [FAIL] panic_sites: a call whose body is not read was reported as a counterexample
  set "FAILED=1"
) else (
  echo [PASS] panic_sites: an unread callee is not laundered into a counterexample
)
rem The callee is named, and the assertion macro's own entry point is told apart
rem from the bare panic it is not.
for %%N in (std::option::Option std::result::Result core::panicking::assert_failed) do (
  findstr /C:"%%N" "%ROOT%\tests\panic_sites\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] panic_sites: report.json does not name the callee %%N
    set "FAILED=1"
  ) else (
    echo [PASS] panic_sites: report.json names the callee %%N
  )
)
rem Counted apart from the assertions, in the summary a reader actually sees: two
rem assertions and six call sites, never eight assertions.
for %%N in ("[Keller]   6 call site(s) that can panic were not modelled (WARN)" "  Panic sites not modelled: 6" "  Total assertions checked: 2") do (
  findstr /C:%%N "%PS_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] panic_sites: the run does not say %%N
    set "FAILED=1"
  ) else (
    echo [PASS] panic_sites: the run says %%N
  )
)
rem The precision half: an `assert!` in the body is still believed as a guard, and
rem the control with an obligation still has it decided.
for %%N in ("guarded_by_assert (bb1)" "control_bounds_check (bb1)") do (
  findstr /C:"[PASS] Verified formal integrity: %%N" "%PS_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] panic_sites: %%N is no longer proved, so the panic-site pass cost a verdict
    set "FAILED=1"
  ) else (
    echo [PASS] panic_sites: %%N is still decided
  )
)
call :check_golden panic_sites
rem Advisory by default, fatal on request - the contract `undecided` pins for a
rem solver that cannot answer, applied to a path nothing here reads.
pushd "%ROOT%\tests\panic_sites"
if exist target rmdir /s /q target
"%VERIFIER%" keller verify --deny-warn >"%PS_LOG%.denywarn" 2>&1
set "PS_DENIED=%ERRORLEVEL%"
popd
if not "%PS_DENIED%"=="1" (
  echo [FAIL] panic_sites: expected exit 1 under --deny-warn, got %PS_DENIED%
  set "FAILED=1"
) else (
  echo [PASS] panic_sites: an unmodelled panic site is fatal under --deny-warn
)
findstr /C:"are undecided (WARN) and --deny-warn was given" "%PS_LOG%.denywarn" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] panic_sites: the gate failed without naming the WARN it failed on
  set "FAILED=1"
) else (
  echo [PASS] panic_sites: the gate says which verdict it failed on
)

rem ===========================================================================
rem  The panics this analysis can decide (A6, the other half).
rem
rem  panic_sites pins the receiver-is-a-parameter case, which must stay a
rem  disclosure. This fixture pins the known-variant case: the safe variants
rem  leave no finding at all, and the variant the callee cannot return is a FAIL
rem  with no solver call and no invented witness.
rem ===========================================================================
set "PD_LOG=%TEMP%\keller-panic-decided.log"
pushd "%ROOT%\tests\panic_decided"
if exist target rmdir /s /q target
if exist report.json del report.json
if exist .keller rmdir /s /q .keller
"%VERIFIER%" keller verify >"%PD_LOG%" 2>&1
set "PD_EXIT=%ERRORLEVEL%"
popd
call :mark_case panic_decided
if not "%PD_EXIT%"=="1" (
  echo [FAIL] panic_decided: expected exit 1 with the three decided panics, got %PD_EXIT%
  set "FAILED=1"
) else (
  echo [PASS] panic_decided: a known failing variant is a counterexample and fails the gate
)
for %%S in (known_none known_err known_ok_unwrap_err) do (
  findstr /C:"[FAIL] Decided panic site in %%S (" "%PD_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] panic_decided: %%S was not decided even though its receiver variant is known
    set "FAILED=1"
  ) else (
    echo [PASS] panic_decided: %%S's known failing variant is a counterexample
  )
)
for %%S in (known_some known_ok known_err_unwrap_err dead_site) do (
  findstr /C:"Decided panic site in %%S (" "%PD_LOG%" >nul 2>&1
  if not errorlevel 1 (
    echo [FAIL] panic_decided: %%S is reported even though its site was decided safe
    set "FAILED=1"
  ) else (
    echo [PASS] panic_decided: %%S's site is decided safe and leaves no finding
  )
)
call :count_lines "%ROOT%\tests\panic_decided\report.json" "decided-panic" PD_DECIDED
if not "%PD_DECIDED%"=="3" (
  echo [FAIL] panic_decided: expected 3 decided panics, got %PD_DECIDED%
  set "FAILED=1"
) else (
  echo [PASS] panic_decided: exactly the three failing variants are counterexamples
)
for %%N in ("7 call site(s) that can panic were decided" "Panic sites decided: 7") do (
  findstr /C:%%N "%PD_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] panic_decided: the run does not say %%N
    set "FAILED=1"
  ) else (
    echo [PASS] panic_decided: the run says %%N
  )
)
findstr /C:"call site(s) that can panic were not modelled" "%PD_LOG%" >nul 2>&1
if not errorlevel 1 (
  echo [FAIL] panic_decided: an undecided panic site is still announced although all seven were decided
  set "FAILED=1"
) else (
  echo [PASS] panic_decided: no undecided panic site remains in this fixture
)
call :check_golden panic_decided

rem ===========================================================================
rem  The fixtures nothing was running (D8).
rem
rem  `tests/precond_probe` and `test-crates/*` sat in the tree and were run by
rem  nothing: a fixture whose verdict nobody asserts is a fixture where a
rem  regression is invisible. They deliberately do NOT go through `check_golden`:
rem  `tests/goldens` is the corpus the K7 rate is computed over, and adding
rem  straight-line `unguarded` FAILs to it would move a published number for a
rem  reason that has nothing to do with the analysis.
rem
rem  Each is asserted in both directions, so neither can pass by reporting nothing.
rem ===========================================================================

rem  `precond_probe`: what the tool makes of a precondition. The crate writes one
rem  as an in-body `assert!` and calls the callees below it only with values that
rem  cannot overflow, so one run separates two questions that are easy to conflate -
rem  is an in-body assertion believed, and is a callee judged on its call sites?
rem  Since S7 the second is answered per helper: a private leaf whose callers all
rem  pass constants it can take is analysed under the bound they prove, while an
rem  exported one and one called with an unbounded value are not.
call :run_table_case coroutine_state
rem  S6: the resume-state guards of an async fn are decided, not counted as
rem  unsupported. They are named on the console only - a PASS carries an event, but
rem  report.json records FAIL and WARN - so the case is run once more with its output
rem  kept, and these lines are the assertion.
set "CS_LOG=%TEMP%\keller-coroutine-state.log"
pushd "%ROOT%\tests\coroutine_state"
"%VERIFIER%" keller verify >"%CS_LOG%" 2>&1
set "CS_EXIT=%ERRORLEVEL%"
popd
if not "%CS_EXIT%"=="0" (
  echo [FAIL] coroutine_state: expected exit 0, got %CS_EXIT%
  set "FAILED=1"
)
for %%N in ("[PASS] coroutine resume state in one_yield::{closure#0} (bb12): not `2` (poisoned)" "[PASS] coroutine resume state in one_yield::{closure#0} (bb13): not `1` (returned)") do (
  findstr /C:%%N "%CS_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] coroutine_state: a resume-state assertion is no longer decided, so S6 has regressed
    set "FAILED=1"
  ) else (
    echo [PASS] coroutine_state: %%N
  )
)
findstr /C:"ResumedAfterReturn" /C:"ResumedAfterPanic" /C:"ResumedAfterDrop" "%CS_LOG%" >nul 2>&1
if not errorlevel 1 (
  echo [FAIL] coroutine_state: a resume-state assertion is back among the unsupported kinds
  set "FAILED=1"
) else (
  echo [PASS] coroutine_state: no resume-state assertion is counted as unsupported
)
findstr /C:"Coverage: 2 of 2 encountered assertion(s) decided" "%CS_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] coroutine_state: the coverage line no longer counts the resume-state assertions as supported
  set "FAILED=1"
) else (
  echo [PASS] coroutine_state: the coverage line counts both assertions as supported
)
rem  The async body must reach its fixpoint: the resume-after-completion block is a
rem  terminal, not a self-edge, because its assertion can never fall through (S6).
findstr /C:"Path-insensitive fallback in one_yield" "%CS_LOG%"
if not errorlevel 1 (
  echo [FAIL] coroutine_state: the async body fell back to path-insensitivity again
  set "FAILED=1"
) else (
  echo [PASS] coroutine_state: the async body reaches its fixpoint
)
call :run_table_case async_drop_state
rem  S6's third kind, checked against real MIR rather than by symmetry: the
rem  async-drop guard is reached through suspension states, so the invariant does
rem  not decide it - it must be named unsupported, never reported as a FAIL.
set "AD_LOG=%TEMP%\keller-async-drop-state.log"
pushd "%ROOT%\tests\async_drop_state"
"%VERIFIER%" keller verify >"%AD_LOG%" 2>&1
set "AD_EXIT=%ERRORLEVEL%"
popd
if not "%AD_EXIT%"=="0" (
  echo [FAIL] async_drop_state: expected exit 0, got %AD_EXIT%
  set "FAILED=1"
)
for %%N in ("[PASS] coroutine resume state in holds_guard::{closure#0} (bb23): not `1` (returned)" "[PASS] coroutine resume state in holds_guard::{closure#0} (bb22): not `2` (poisoned)") do (
  findstr /C:%%N "%AD_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] async_drop_state: the kinds the resume invariant covers are no longer decided here
    set "FAILED=1"
  ) else (
    echo [PASS] async_drop_state: %%N
  )
)
findstr /C:"ResumedAfterDrop" "%AD_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] async_drop_state: ResumedAfterDrop is not reported at all
  set "FAILED=1"
) else (
  echo [PASS] async_drop_state: the async-drop guard is named as an unsupported assertion
)
findstr /C:"\"status\": \"FAIL\"" "%ROOT%\tests\async_drop_state\report.json" >nul 2>&1
if not errorlevel 1 (
  echo [FAIL] async_drop_state: the async-drop guard is reported as a counterexample, which real MIR says it is not
  set "FAILED=1"
) else (
  echo [PASS] async_drop_state: no counterexample is reported for the async-drop guard
)
call :run_table_case precond_probe
call :check_env precond_probe

rem  Believed, and shown in the artefact rather than inferred from the verdict: the
rem  proof of `x + 1` in `entry_assert` is emitted with the assertion's own
rem  condition as the reachability guard. A run that stopped assuming it would have
rem  to report that overflow as a FAIL, so this is the line that distinguishes "the
rem  assertion is trusted" from "the assertion is checked".
set "PP_ENTRY=%ROOT%\tests\precond_probe\obligations\precond_probe\entry_assert_bb1.smt2"
if not exist "%PP_ENTRY%" (
  echo [FAIL] precond_probe: no obligation was emitted for entry_assert bb1
  set "FAILED=1"
) else (
  findstr /C:"; verdict:  PASS (unsat)" "%PP_ENTRY%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] precond_probe: entry_assert's overflow must be proved under its precondition
    set "FAILED=1"
  ) else (
    findstr /C:"(< mir_1 100)" "%PP_ENTRY%" >nul 2>&1
    if errorlevel 1 (
      echo [FAIL] precond_probe: the proof no longer carries the assertion's own condition as its guard
      set "FAILED=1"
    ) else (
      echo [PASS] precond_probe: the in-body assertion is believed, and it is the guard of the proof beneath it
    )
  )
)

rem  A8's boundary, narrowed by S7: a private leaf whose every call passes a constant
rem  it can take is analysed under that precondition, so `private_callee` leaves no
rem  finding and the check that earns the assumption is at its call site. The two no
rem  call site decides - an exported helper (callers outside this crate are not in
rem  the scan) and one called with a value this analysis does not bound - are still
rem  reported on their own body. Pinned as a boundary either way.
findstr /C:"private_callee" "%ROOT%\tests\precond_probe\report.json" >nul 2>&1
if not errorlevel 1 (
  echo [FAIL] precond_probe: private_callee is reported although every call site proves its derived precondition
  set "FAILED=1"
) else (
  echo [PASS] precond_probe: private_callee is analysed under the precondition its one call site proves, and leaves no finding
)
for %%N in (public_callee private_unknown) do (
  findstr /C:"\"function\": \"%%N\"" "%ROOT%\tests\precond_probe\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] precond_probe: %%N is no longer reported - no call site decides it, so this case's boundary has moved
    set "FAILED=1"
  ) else (
    echo [PASS] precond_probe: %%N is still reported on its own body
  )
)

rem  The inventory the case is written against: the two counterexamples no call site
rem  decides, plus the proof and the S7 summary beside them. The user's own `assert!`
rem  is *not* among the encountered assertions - the guard is its only effect (A6's
rem  recorded gap) - so a change to that inventory has to be seen here.
call :count_lines "%ROOT%\tests\precond_probe\report.json" "FAIL" PP_STATUS
if not "%PP_STATUS%"=="2" (
  echo [FAIL] precond_probe: expected 2 counterexamples, got %PP_STATUS%
  set "FAILED=1"
) else (
  echo [PASS] precond_probe: the report is the 2 counterexamples this case pins
)
rem  And the fourth line, added by A6: the same `assert!` whose condition is
rem  believed is also a call into the panic machinery, so its failure path is named
rem  as a site nothing was decided about. Counted apart from the counterexamples,
rem  because the two say different things.
call :count_lines "%ROOT%\tests\precond_probe\report.json" "unmodelled-panic" PP_SITES
if not "%PP_SITES%"=="1" (
  echo [FAIL] precond_probe: the body's own `assert!` is not reported as a panic site, got %PP_SITES%
  set "FAILED=1"
) else (
  echo [PASS] precond_probe: the in-body assertion's failure path is named as a panic site
)

rem  `helper_summary`: S7. A small, private, leaf helper is analysed under the
rem  precondition its own body implies and its call sites prove, so `inc`'s
rem  unguarded overflow is gone; the three helpers no call site decides keep exactly
rem  the finding they had - `boom` (one caller passes 255), `open` (the caller passes
rem  a runtime value) and `exported` (`pub`, so its callers are not all visible here).
rem  The console half is captured the way async_drop_state captures it, because
rem  `run_case` sends the run to nul.
call :run_table_case helper_summary
call :check_env helper_summary
call :check_obligations helper_summary
set "HS_LOG=%TEMP%\keller-helper-summary.log"
pushd "%ROOT%\tests\helper_summary"
"%VERIFIER%" keller verify >"%HS_LOG%" 2>&1
popd
for %%N in ("[PASS] Helper summary in inc: `x <= 254` is checked at all 2 call site(s), so this body is analysed under it" "[PASS] Verified formal integrity: inc (bb0)" "[PASS] Helper precondition in main (bb5): `x <= 254` holds for `inc` (argument 7)" "[PASS] Helper precondition in main (bb6): `x <= 254` holds for `inc` (argument 200)") do (
  findstr /C:%%N "%HS_LOG%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] helper_summary: the console no longer carries a line this case pins
    set "FAILED=1"
  ) else (
    echo [PASS] helper_summary: %%N
  )
)
for %%N in (boom open exported) do (
  findstr /C:"\"function\": \"%%N\"" "%ROOT%\tests\helper_summary\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] helper_summary: %%N is no longer reported - no call site proves a precondition for it
    set "FAILED=1"
  ) else (
    echo [PASS] helper_summary: %%N keeps its unguarded finding
  )
)
set "HS_PROOF=%ROOT%\tests\helper_summary\obligations\helper_summary\inc_bb0.smt2"
if not exist "%HS_PROOF%" (
  echo [FAIL] helper_summary: no obligation was emitted for inc bb0
  set "FAILED=1"
) else (
  findstr /C:"; verdict:  PASS (unsat)" "%HS_PROOF%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] helper_summary: inc's overflow must be proved under the derived bound
    set "FAILED=1"
  ) else (
    findstr /C:"; reachable: (<= mir_1 254)" "%HS_PROOF%" >nul 2>&1
    if errorlevel 1 (
      echo [FAIL] helper_summary: the proof of inc no longer carries the derived precondition as its guard
      set "FAILED=1"
    ) else (
      echo [PASS] helper_summary: inc's overflow is proved under the bound its call sites were checked against
    )
  )
)

rem  `test-crates/buggy-project`: one finding of each class the report can state, on
rem  a crate written to be caught. The classes are asserted by name because the exit
rem  code alone would still be 1 with only one of the three surviving.
call :run_table_case test-crates/buggy-project
call :check_env test-crates/buggy-project
for %%N in (bounds overflow divzero) do (
  findstr /C:"\"kind\": \"%%N\"" "%ROOT%\test-crates\buggy-project\report.json" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] buggy-project: the report is missing the %%N class
    set "FAILED=1"
  ) else (
    echo [PASS] buggy-project: the report carries a %%N finding
  )
)

rem  K10's rule on a real run: `check_index`'s index is a value the obligation only
rem  bounds, so its witness is one instance among many and has to say so. A `pinned`
rem  there would report the solver's choice as a fact about the program.
findstr /C:"\"witness_kind\": \"one-of-many\"" "%ROOT%\test-crates\buggy-project\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] buggy-project: the bounds witness is no longer marked one-of-many
  set "FAILED=1"
) else (
  echo [PASS] buggy-project: the bounds witness is marked one-of-many, not reported as a fact
)

rem  `test-crates/safe-project`: the negative control. It exits 0 *and* reports
rem  nothing, which is the pair that makes the case above mean something, and it is
rem  the only fixture outside `tests/` that any runner had ever executed.
call :run_table_case test-crates/safe-project
call :check_env test-crates/safe-project
call :count_lines "%ROOT%\test-crates\safe-project\report.json" "status" SP_STATUS
if not "%SP_STATUS%"=="0" (
  echo [FAIL] safe-project: a fixture that cannot panic produced %SP_STATUS% findings
  set "FAILED=1"
) else (
  echo [PASS] safe-project: the report is empty, not merely passing
)

rem ===========================================================================
rem  K2 / O3: a missing pinned toolchain is named, not read as a version
rem  mismatch.
rem
rem  The verifier is linked against one exact nightly, so before cargo starts it
rem  asks `rustup toolchain list` whether that toolchain is installed and exits 3
rem  with the install line if it is not. CONTRIBUTING.md forbids uninstalling the
rem  pinned toolchain to test this, and that rule is right: a suite that removes
rem  the compiler it runs on can destroy the machine it audits. The branch is
rem  reached instead with a real rustup pointed at an empty RUSTUP_HOME, where
rem  `rustup toolchain list` genuinely answers `no installed toolchains` while the
rem  installed toolchain is untouched, and the check reads exactly that listing.
rem
rem  What this does not prove is a real absence on disk: the compiler is still
rem  installed, and DONE.md says so rather than implying a clean machine was
rem  tested. The same claims tests/run_tests.sh asserts, from the same invocation.
rem ===========================================================================
set "EMPTY_RUSTUP=%TEMP%\keller-empty-rustup-home"
if exist "%EMPTY_RUSTUP%" rmdir /s /q "%EMPTY_RUSTUP%"
mkdir "%EMPTY_RUSTUP%"
set "MT_LOG=%TEMP%\keller-missing-toolchain.log"
pushd "%ROOT%\tests\safe_code"
if exist report.json del report.json
set "RUSTUP_HOME=%EMPTY_RUSTUP%"
"%VERIFIER%" keller verify >"%MT_LOG%" 2>&1
set "MT_EXIT=%ERRORLEVEL%"
set "RUSTUP_HOME="
popd
call :mark_case missing_toolchain
if "%MT_EXIT%"=="3" (
  echo [PASS] missing_toolchain: a missing pinned toolchain is a setup failure, not a verdict
) else (
  echo [FAIL] missing_toolchain: expected exit 3 ^(setup failure^), got %MT_EXIT%
  set "FAILED=1"
)
findstr /C:"is not installed, and it cannot analyse a crate without it" "%MT_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] missing_toolchain: the message does not say which toolchain is missing
  set "FAILED=1"
) else (
  echo [PASS] missing_toolchain: the message names the toolchain it cannot run without
)
set "DRIVER_TOOLCHAIN="
for /f "tokens=1" %%T in ('rustup show active-toolchain 2^>nul') do set "DRIVER_TOOLCHAIN=%%T"
if not defined DRIVER_TOOLCHAIN goto :mt_install_skipped
findstr /C:"rustup toolchain install %DRIVER_TOOLCHAIN%" "%MT_LOG%" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] missing_toolchain: the install line does not name %DRIVER_TOOLCHAIN%
  set "FAILED=1"
) else (
  echo [PASS] missing_toolchain: the message carries the install line for %DRIVER_TOOLCHAIN%
)
:mt_install_skipped
rem The other branch this must not be: an installed-but-different compiler is
rem reported with both identities (`built with:` / `installed:`), and a run that
rem reached that one has not tested the missing toolchain at all.
findstr /C:"built with:" "%MT_LOG%" >nul 2>&1
if not errorlevel 1 (
  echo [FAIL] missing_toolchain: the run reported a compiler mismatch, not a missing toolchain
  set "FAILED=1"
) else (
  echo [PASS] missing_toolchain: the run stopped at the missing toolchain, not at a compiler mismatch
)
if exist "%ROOT%\tests\safe_code\report.json" (
  echo [FAIL] missing_toolchain: a verdict was published although the toolchain was missing
  set "FAILED=1"
) else (
  echo [PASS] missing_toolchain: no verdict is published when the toolchain is missing
)

call :check_case_table
del "%CASES_CONSUMED%" >nul 2>&1

if "%FAILED%"=="1" (
  echo [tests] *** FAILURES DETECTED ***
  exit /b 1
)

echo [tests] All integration tests passed.
exit /b 0

rem ---------------------------------------------------------------------------
rem  The row for a case in tests\cases.txt, into CASE_EXIT, CASE_GOLDEN and
rem  CASE_LABEL. Nothing parses the table by hand: the table and the runners are
rem  the only things that know a case's exit code, and this is where both ends
rem  meet.
:case_row
set "CASE_EXIT="
set "CASE_GOLDEN="
set "CASE_LABEL="
for /f "tokens=1,2,3,4 delims=|" %%A in ('findstr /b /C:"%~1|" "%CASES_FILE%"') do (
  set "CASE_EXIT=%%B"
  set "CASE_GOLDEN=%%C"
  set "CASE_LABEL=%%D"
)
exit /b 0

rem ---------------------------------------------------------------------------
rem  One case, driven by the table: the exit code is looked up rather than
rem  written twice, and the golden is compared exactly when the row names one.
rem  Which cases those are is no longer a decision this runner makes for itself -
rem  it was, and that is how it came to compare ten goldens while
rem  tests/run_tests.sh compared thirteen.
:run_table_case
set "CASE=%~1"
call :case_row "%~1"
if not defined CASE_EXIT (
  echo [FAIL] %CASE%: no row in tests\cases.txt, so nothing says what it must do
  set "FAILED=1"
  exit /b 0
)
call :run_case "%CASE%" "%CASE_EXIT%"
rem Every obligation the run left on disk is re-discharged by the standalone
rem solver, not merely counted: a verdict is a claim about a formula, so the file
rem it came from has to be a problem the solver still answers the same way. A case
rem with no obligations skips this.
call :check_obligations "%CASE%"
if not "%CASE_GOLDEN%"=="-" call :check_golden "%CASE%"
call :mark_case "%CASE%"
exit /b 0

rem ---------------------------------------------------------------------------
rem  A case that needed its own invocation - a solver budget, an undecided verdict,
rem  the precision probe - marks itself here instead of going through
rem  :run_table_case. :check_case_table can then tell "handled deliberately" from
rem  "forgotten", which is what a case list drifting between two runners has to be
rem  loud about.
:mark_case
>>"%CASES_CONSUMED%" echo %~1
exit /b 0

rem ---------------------------------------------------------------------------
rem  The answer the standalone solver gives for one obligation, in ANSWER. Used
rem  where the point of a case is that a verdict is the solver's own answer rather
rem  than the tool's story about it. Z3_BIN is the solver the verifier was linked
rem  against, which is what makes it the right one to ask; with none, ANSWER stays
rem  empty and the caller reports a SKIP rather than inventing a verdict.
:z3_answer
set "ANSWER="
if not defined Z3_BIN exit /b 0
rem The solver's output is read back from a file rather than through `for /f`: a
rem quoted program path with a space in it is split by the command string that
rem `for /f` builds, and this checkout's path has one. `set /p` takes the first
rem line, which is the verdict - the same answer tests/run_tests.sh compares.
set "Z3_ANSWER_FILE=%TEMP%\keller-z3-answer-%RANDOM%.txt"
"%Z3_BIN%" -smt2 "%~1" >"%Z3_ANSWER_FILE%" 2>&1
set /p ANSWER=<"%Z3_ANSWER_FILE%"
del "%Z3_ANSWER_FILE%" >nul 2>&1
exit /b 0

rem ---------------------------------------------------------------------------
rem  Every obligation a case left on disk is re-discharged by the standalone
rem  solver with the answer its own `; verdict:` header records. This is the
rem  Windows half of tests/run_tests.sh's check_obligations: counting verdicts
rem  says a case produced findings, not that the formulas it produced are
rem  satisfiable in the way the report claims. A `*.witness.smt2` is the pinned
rem  counterexample and must answer `sat`; a FAIL obligation must answer `sat`;
rem  a PASS must answer `unsat`; a WARN is a budget answer, so only its validity
rem  is asserted. The counts at the end are the rule that every FAIL has exactly
rem  one pinned witness and a proof has none.
rem
rem  The files are two levels deep (`obligations\<crate>\<function>_bbN.smt2`),
rem  so the relative name is formed by cutting at `\obligations\` rather than by
rem  assuming everything below it is one file name.
:check_obligations
set "OBL_CASE=%~1"
call :case_dir %~1
set "OBL_ROOT=%ROOT%\%DIR%\obligations"
if not exist "%OBL_ROOT%" exit /b 0
if not defined Z3_BIN (
  echo [SKIP] %OBL_CASE%: no runnable z3 was found to re-discharge obligations
  exit /b 0
)
set "OBL_FAIL_N=0"
set "OBL_PASS_N=0"
set "OBL_PINNED_N=0"
for /r "%OBL_ROOT%" %%F in (*.smt2) do call :check_one_obligation "%OBL_CASE%" "%%F"
if not "%OBL_FAIL_N%"=="%OBL_PINNED_N%" (
  echo [FAIL] %OBL_CASE%: %OBL_FAIL_N% FAIL obligation^(s^) but %OBL_PINNED_N% pinned witness file^(s^)
  set "FAILED=1"
)
if %OBL_PASS_N% GTR 0 (
  echo [PASS] %OBL_CASE%: %OBL_PASS_N% prove^(s^) left a re-dischargeable obligation, not just a verdict
)
exit /b 0

:check_one_obligation
set "OBL_ABS=%~2"
set "OBL_REL=%OBL_ABS:*\obligations\=%"
set "OBL_NAME=%~nx2"
set "OBL_ANSWER=%TEMP%\keller-z3-obligation-%RANDOM%.txt"
"%Z3_BIN%" -smt2 "%OBL_ABS%" >"%OBL_ANSWER%" 2>&1
rem The first line is the verdict; read it through a file because a quoted
rem program path with a space cannot be spliced into a `for /f` command string.
rem `set /p` keeps the first line, and the token loop drops the trailing spaces
rem the solver may print.
set "OBL_GOT="
set /p OBL_GOT=<"%OBL_ANSWER%"
for /f "tokens=1" %%G in ("%OBL_GOT%") do set "OBL_GOT=%%G"
%SystemRoot%\System32\findstr.exe /i /C:"error" /C:"unsupported" "%OBL_ANSWER%" >nul 2>&1
if not errorlevel 1 (
  echo [FAIL] %OBL_CASE%: %OBL_REL% is not valid SMT-LIB
  type "%OBL_ANSWER%"
  set "FAILED=1"
  del "%OBL_ANSWER%" >nul 2>&1
  exit /b 0
)
rem The verdict line the emitter wrote, without the header's padding. Splitting
rem at the colon leaves `  PASS (unsat)`, and the second loop trims the spaces.
rem Read through a file rather than `for /f ('findstr ... "%OBL_ABS%"')`: a
rem command nested in `for /f` loses the quotes around a path with a space in it,
rem so findstr would be handed `C:\Users\INTAL` and `Admin\...` as two arguments.
rem The file also keeps the `&` in this checkout's path out of the command string.
rem `eol=|` is not cosmetic: the header starts with `;`, and `for /f`'s default
rem `eol=;` treats that line as a comment and reads nothing, which left every
rem verdict empty. `|` is not a character the header can contain.
set "OBL_HEADER=%TEMP%\keller-obligation-header-%RANDOM%.txt"
%SystemRoot%\System32\findstr.exe /b /C:"; verdict:" "%OBL_ABS%" >"%OBL_HEADER%" 2>nul
set "OBL_VERDICT="
for /f "usebackq eol=| tokens=1,* delims=:" %%A in ("%OBL_HEADER%") do set "OBL_VERDICT=%%B"
del "%OBL_HEADER%" >nul 2>&1
for /f "tokens=* delims= " %%V in ("%OBL_VERDICT%") do set "OBL_VERDICT=%%V"
rem A witness is checked before the verdict: its header says FAIL (sat) too, but
rem it is not a FAIL obligation and must not be counted as one.
if /i "%OBL_NAME:~-13%"==".witness.smt2" (
  set /a OBL_PINNED_N+=1
  if /i not "%OBL_GOT%"=="sat" (
    echo [FAIL] %OBL_CASE%: %OBL_REL% pins a witness but is not sat: %OBL_GOT%
    set "FAILED=1"
  ) else (
    echo [PASS] %OBL_CASE%: %OBL_REL% pinned witness re-discharged to sat
  )
  del "%OBL_ANSWER%" >nul 2>&1
  exit /b 0
)
set "OBL_WANT="
if /i "%OBL_VERDICT%"=="FAIL (sat)" set "OBL_WANT=sat"
if /i "%OBL_VERDICT%"=="FAIL (folded)" set "OBL_WANT=sat"
if /i "%OBL_VERDICT%"=="PASS (unsat)" set "OBL_WANT=unsat"
if not defined OBL_WANT goto :obligation_unknown
if /i "%OBL_VERDICT:~0,4%"=="FAIL" set /a OBL_FAIL_N+=1
if /i "%OBL_VERDICT:~0,4%"=="PASS" set /a OBL_PASS_N+=1
if /i not "%OBL_GOT%"=="%OBL_WANT%" goto :obligation_mismatch
echo [PASS] %OBL_CASE%: %OBL_REL% re-discharged to %OBL_WANT% ^(records '%OBL_VERDICT%'^)
goto :obligation_done
:obligation_mismatch
rem Not inside a parenthesised block on purpose: `%OBL_VERDICT%` expands to a
rem value containing parentheses (`FAIL (sat)`), and cmd closes the block at that
rem `)` before the rest of the line is parsed. Echoing from top level keeps them
rem literal.
echo [FAIL] %OBL_CASE%: %OBL_REL% records '%OBL_VERDICT%' but re-discharges to '%OBL_GOT%'
set "FAILED=1"
goto :obligation_done
:obligation_unknown
echo [PASS] %OBL_CASE%: %OBL_REL% is valid SMT-LIB and answers '%OBL_GOT%' ^(records '%OBL_VERDICT%'^)
:obligation_done
del "%OBL_ANSWER%" >nul 2>&1
exit /b 0

rem ---------------------------------------------------------------------------
rem  The two ends of tests\cases.txt, checked once the cases have run.
rem
rem  1. Every row was consumed. A case that exists in the table and is never run
rem     here is exactly the gap the file exists to close, so it is a failure and
rem     not a note.
rem  2. The rows that name a golden are exactly the files in tests\goldens. That is
rem     the drift that was measured before the file existed: 85 checks over ten
rem     goldens here against 281 over thirteen in tests/run_tests.sh, with nothing
rem     in the tree saying which number was right.
:check_case_table
set "TABLE_UNRUN=0"
set "GOLDEN_MISMATCH=0"
for /f "tokens=1 delims=|" %%A in ('findstr /v /b /C:"#" "%CASES_FILE%"') do (
  findstr /x /C:"%%A" "%CASES_CONSUMED%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] %%A: tests\cases.txt lists it and this runner never ran it
    set "FAILED=1"
    set "TABLE_UNRUN=1"
  )
)
if "%TABLE_UNRUN%"=="0" echo [PASS] every case in tests\cases.txt was run
rem A golden is looked up as the case name, so a row whose golden column names
rem something else would be a row lying about which file gets compared.
for /f "tokens=1,2,3,4 delims=|" %%A in ('findstr /v /b /C:"#" "%CASES_FILE%"') do (
  if not "%%C"=="-" (
    if not "%%C"=="%%A" (
      echo [FAIL] %%A: the table names golden %%C, and a golden is looked up as the case name
      set "FAILED=1"
      set "GOLDEN_MISMATCH=1"
    )
    if not exist "%ROOT%\tests\goldens\%%C.json" (
      echo [FAIL] %%A: the table says it has a golden and tests\goldens\%%C.json is not there
      set "FAILED=1"
      set "GOLDEN_MISMATCH=1"
    )
  )
)
rem And the other direction: a golden no row names is a file the table has never
rem heard of, which is what a recorded golden with no case looks like.
for /f %%F in ('dir /b /a-d "%ROOT%\tests\goldens\*.json"') do (
  findstr /b /C:"%%~nF|" "%CASES_FILE%" >nul 2>&1
  if errorlevel 1 (
    echo [FAIL] tests\goldens\%%F has no row in tests\cases.txt
    set "FAILED=1"
    set "GOLDEN_MISMATCH=1"
  )
)
if "%GOLDEN_MISMATCH%"=="0" echo [PASS] tests\cases.txt and tests\goldens name the same goldens
exit /b 0

rem ---------------------------------------------------------------------------
:run_case
set "CASE=%~1"
set "EXPECTED=%~2"
rem  The crate's directory, resolved the same way the shell runner resolves it:
rem  third argument first, then `tests\<case>`, and a fixture that lives somewhere
rem  else is named by its own path relative to the repository root. Without this a
rem  case called `test-crates/safe-project` is looked for at `tests\test-crates/...`,
rem  and the whole case fails with a path error for a crate that was never analysed.
call :case_dir %~3
pushd "%ROOT%\%DIR%"
rem Guarantee a fresh compilation through the verifier: cargo's incremental
rem fingerprints do not notice RUSTC_WRAPPER changes, and Windows mtime
rem granularity can defeat timestamp-only touches.
if exist target rmdir /s /q target
if exist report.json del report.json
"%VERIFIER%" keller verify >nul 2>nul
set "EXITCODE=%ERRORLEVEL%"
popd

if not "%EXITCODE%"=="%EXPECTED%" (
  echo [FAIL] %CASE%: expected exit %EXPECTED%, got %EXITCODE%
  set "FAILED=1"
  exit /b 0
)

if not "%EXPECTED%"=="1" goto :case_pass

rem FAIL case: report.json must exist, contain a FAIL event, and carry the
rem witness that names the failing input - a verdict alone is not a bug report.
pushd "%ROOT%\%DIR%"
findstr /C:"FAIL" report.json >nul 2>&1
set "HASFAIL=%ERRORLEVEL%"
findstr /C:"input" report.json >nul 2>&1
set "HASWITNESS=%ERRORLEVEL%"
set "HASPINNED=1"
dir /b /s obligations\*.witness.smt2 >nul 2>&1
if not errorlevel 1 set "HASPINNED=0"
popd
if not "%HASFAIL%"=="0" (
  echo [FAIL] %CASE%: exit 1 but report.json contains no FAIL
  set "FAILED=1"
  exit /b 0
)
if not "%HASWITNESS%"=="0" (
  echo [FAIL] %CASE%: report.json FAILs carry no concrete witness
  set "FAILED=1"
  exit /b 0
)
if not "%HASPINNED%"=="0" (
  echo [FAIL] %CASE%: no pinned witness obligation was written
  set "FAILED=1"
  exit /b 0
)

:case_pass
echo [PASS] %CASE%: exit %EXITCODE% as expected
exit /b 0

rem ---------------------------------------------------------------------------
rem  run_case, plus one extra command-line argument, for the cases that probe the
rem  Compare a crate's report.json against the committed golden for it. The case
rem  checks assert the fields someone thought to assert; the golden catches any
rem  other change to the file. Re-record with:
rem    copy /y tests\<case>\report.json tests\goldens\<case>.json
:check_golden
set "CASE=%~1"
if not exist "%ROOT%\tests\goldens\%CASE%.json" (
  echo [SKIP] %CASE%: no golden recorded
  exit /b 0
)
if not exist "%ROOT%\tests\%CASE%\report.json" (
  echo [FAIL] %CASE%: a golden exists but this run produced no report
  set "FAILED=1"
  exit /b 0
)
fc /b "%ROOT%\tests\goldens\%CASE%.json" "%ROOT%\tests\%CASE%\report.json" >nul 2>&1
if errorlevel 1 (
  echo [FAIL] %CASE%: report.json differs from tests\goldens\%CASE%.json
  rem A bare "differs" leaves the reader to find the change. The byte compare
  rem above decides pass/fail; a line-numbered text compare below shows what
  rem changed, which is the part a reviewer needs to accept or reject.
  echo        --- tests\goldens\%CASE%.json / tests\%CASE%\report.json
  fc /n "%ROOT%\tests\goldens\%CASE%.json" "%ROOT%\tests\%CASE%\report.json" 2>&1
  set "FAILED=1"
) else (
  echo [PASS] %CASE%: report.json matches its golden
)
exit /b 0

rem ---------------------------------------------------------------------------
rem  Every run records the toolchain and Z3 it was decided in, beside the report.
rem  Not inside report.json: that file is byte-compared against the goldens (`fc
rem  /b` below) and these facts are absolute paths, so a machine-specific path in
rem  it would fail every golden on any other machine. The facts live in
rem  report.environment.json instead, and both directions are asserted — the
rem  sidecar carries every key keller-env prints, and report.json has not grown
rem  the env block that would break the golden diff.
rem
rem  The search strings are the bare key names on purpose: findstr has no escape
rem  for a literal quote inside /C:"...", and serde prints the key as `"name":`,
rem  so neither the quote nor the colon that follows it can be spelled here. A key
rem  name is already specific. The leak check looks for z3_bin, which appears in
rem  report.json only if the environment is back inside it.
:check_env
set "CASE=%~1"
call :case_dir %~2
set "ENVFILE=%ROOT%\%DIR%\report.environment.json"
if not exist "%ENVFILE%" (
  echo [FAIL] %CASE%: no report.environment.json, so nothing says what decided it
  set "FAILED=1"
  exit /b 0
)
set "ENVOK=1"
findstr /C:"env_source" "%ENVFILE%" >nul 2>&1
if errorlevel 1 set "ENVOK=0"
findstr /C:"rustc" "%ENVFILE%" >nul 2>&1
if errorlevel 1 set "ENVOK=0"
findstr /C:"host" "%ENVFILE%" >nul 2>&1
if errorlevel 1 set "ENVOK=0"
findstr /C:"rustlib" "%ENVFILE%" >nul 2>&1
if errorlevel 1 set "ENVOK=0"
findstr /C:"z3_bin" "%ENVFILE%" >nul 2>&1
if errorlevel 1 set "ENVOK=0"
findstr /C:"z3_source" "%ENVFILE%" >nul 2>&1
if errorlevel 1 set "ENVOK=0"
findstr /C:"path_add" "%ENVFILE%" >nul 2>&1
if errorlevel 1 set "ENVOK=0"
if "%ENVOK%"=="0" (
  echo [FAIL] %CASE%: report.environment.json is missing a key keller-env prints
  set "FAILED=1"
  exit /b 0
)
if exist "%ROOT%\%DIR%\report.json" (
  findstr /C:"z3_bin" "%ROOT%\%DIR%\report.json" >nul 2>&1
  if not errorlevel 1 (
    echo [FAIL] %CASE%: report.json carries an env block, which would break its golden
    set "FAILED=1"
    exit /b 0
  )
)
echo [PASS] %CASE%: report.environment.json records the toolchain and Z3
exit /b 0

rem ---------------------------------------------------------------------------
rem  run_case, plus one extra command-line argument, for the cases that probe the
rem  exit-code contract's own flags rather than the analysis.
:run_gate
set "CASE=%~1"
set "EXPECTED=%~2"
set "EXTRA=%~3"
pushd "%ROOT%\tests\%CASE%"
if exist target rmdir /s /q target
if exist report.json del report.json
"%VERIFIER%" keller verify %EXTRA% >nul 2>nul
set "EXITCODE=%ERRORLEVEL%"
popd

if not "%EXITCODE%"=="%EXPECTED%" (
  echo [FAIL] %CASE%: expected exit %EXPECTED%, got %EXITCODE% %EXTRA%
  set "FAILED=1"
  exit /b 0
)
echo [PASS] %CASE%: exit %EXITCODE% as expected %EXTRA%
exit /b 0

rem ---------------------------------------------------------------------------
rem  Where the case's crate is, into DIR. The argument, when one is given, is the
rem  directory relative to the repository root; otherwise the case name is tried as
rem  a path and `tests\<case>` is the fallback. Each `set`/`if` is on its own line
rem  rather than in a parenthesised block: cmd expands `%DIR%` when it parses a
rem  block, so a test inside one would see the value from before the `set` beside it.
:case_dir
set "DIR=%~1"
if not "%DIR%"=="" goto :case_dir_done
set "DIR=%CASE:/=\%"
if not exist "%ROOT%\%DIR%\Cargo.toml" set "DIR=tests\%CASE%"
:case_dir_done
exit /b 0

rem ---------------------------------------------------------------------------
rem  Count the lines of a file that contain a token, into the variable named by
rem  the third argument. `find /c` cannot be used from here: cmd treats the /C:
rem  argument as an option, and `findstr` has no escape for a quote inside /C:"...".
rem  A bare token is specific enough, and the variables are cleared first so a
rem  stale value from an earlier call cannot answer for this one.
rem
rem  Both tools are named by full path on purpose. `findstr` has no GNU homonym,
rem  but `find` does, and this script is run from whatever shell a developer has
rem  open: under Git Bash's PATH, `find /c /v ""` is GNU `find` walking `/c` — the
rem  whole of C: — and the runner hangs there instead of counting anything. The
rem  count is a diagnostic; a suite that stops on a drive scan is not.
:count_lines
set "COUNT_FILE=%~1"
set "COUNT_TOKEN=%~2"
set "COUNT_RESULT=0"
for /f %%C in ('%SystemRoot%\System32\findstr.exe /C:"%COUNT_TOKEN%" "%COUNT_FILE%" ^| %SystemRoot%\System32\find.exe /c /v ""') do set "COUNT_RESULT=%%C"
set "%~3=%COUNT_RESULT%"
exit /b 0

rem ---------------------------------------------------------------------------
rem  One `KEY=VALUE` line of keller-env.exe's output as a variable. The keys are
rem  that program's contract (documented in src/bin/keller-env.rs), so nothing is
rem  enumerated here: a key it stops printing sets a variable nothing reads, and
rem  a key it adds is picked up without this file changing.
:env_set
set "%~1=%~2"
exit /b 0

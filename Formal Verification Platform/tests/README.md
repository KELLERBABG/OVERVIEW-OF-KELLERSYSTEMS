# Integration Tests

End-to-end tests that run the real `keller-verify` binary (via `RUSTC_WRAPPER`)
against small throwaway crates and assert the pipeline's exit-code contract.

| Crate | Expectation |
|---|---|
| `safe_code` | `keller verify` exits **0** â€” no FAIL events |
| `overflow_fail` | `keller verify` exits **1** â€” Z3 finds `a + b` overflow (witness `a = -2147483648, b = -1`) |
| `divzero_fail` | `keller verify` exits **1** â€” Z3 proves the *divisor* can be 0 (witness `b = 0`) |
| `branch_merge` | `keller verify` exits **1** â€” a constant merged from two branches must not be folded to "safe" and skipped |
| `guarded` | `keller verify` exits **1**, and the four functions whose arithmetic a branch guard rules out must be **absent** from `report.json` while `sum_under_guarded` (guard too weak) must be there |
| `workspace_multi` | `keller verify` exits **1** and the merged `report.json` contains **both** members' findings (`alpha_overflow` *and* `beta_divide`) |
| `loop_fail` | `keller verify` exits **1** â€” a loop body is checked under the loop's own guards, so its counter increment is **proved** while the accumulator's reachable overflow is still reported (witness `acc (_1) = 255`, the value the obligation pins) |
| `nested_loop` | `keller verify` exits **1** â€” the shape next door to `loop_fail`: an inner loop inside an outer one (with `break`/`continue`). Its guarded increments are **proved** too â€” the cycle widening bounds the condition as well as the values, which is what a loop inside a loop used to need â€” so only the accumulator's unbounded overflow is reported (witness `total (_3) = 255`). The runner then invokes it once more with the round budget cut to one round, which pins the *report* of a lost precision â€” see below |
| `shift_amount` | `keller verify` exits **1** â€” a shift panics only when its amount is negative or at least the operand's width, so `shift_by` is reported with the amount (`by = 32`) while the guarded shift and the value-truncating constant shift are **proved** and stay out of the report |
| `stable_pin` | `keller verify` exits **1** â€” this crate carries its own `rust-toolchain.toml` pinning `stable`, which is what a real project does. `cargo` is a rustup shim and follows that pin, so the run used to end as a setup failure (exit **3**, `error: the -Z flag is only accepted on the nightly channel`) with no verdict on code that compiles cleanly. The verifier selects the compiler it was built against, so the analysis runs: `unguarded_increment` is the counterexample and `guarded_increment` beside it is proved |
| `fold_fail` | `keller verify` exits **1** â€” six failures the constant-folding pass reaches *without* asking Z3 still carry a witness and a re-dischargeable obligation, exactly like a solved one |
| `broken_code` | `keller verify` exits **3** â€” the crate does not compile, so no verdict exists: a setup failure, not a clean run |
| `precond_probe` | `keller verify` exits **1** â€” what the tool makes of a precondition: the `assert!` at the top of `entry_assert` is **believed** (its condition is the guard the overflow beneath it is proved under), while `private_callee` / `public_callee` / `private_unknown` are judged on their own bodies and reported even though every call in the crate passes an in-range constant (A8's boundary) |
| `test-crates/buggy-project` | `keller verify` exits **1** â€” one finding of each modelled class (`bounds`, `overflow`, `divzero`) and the control beside them: the divisor that cannot be zero is **proved** |
| `test-crates/safe-project` | `keller verify` exits **0** and reports nothing â€” the negative control for the crate above |
| `undecided` | `keller verify` exits **0** â€” its second obligation is one Z3 does not decide, so it is reported as a `WARN` (`bb6`) rather than proved or refuted. The same run under `--deny-warn` exits **1**, which is the other half of the exit-code contract. The runner also invokes it a third time with **no** solver budget and a 20 s `--run-timeout`, which is the one configuration that reaches the analysis wedge (K3): nine checks there pin that the run is stopped by the cap (exit **3**, not a verdict), that it names the crate still in flight, that its run record says `run-timeout` and publishes nothing over the last completed `report.json`, and that no wrapper or cargo process it started is left running |
| `unnamed_write` | `keller verify` exits **0** â€” a store through a reference (`*r = 250`) names no place, so the value it overwrote is retired rather than kept: the check that depended on it is a `WARN` naming the store instead of a `PASS` answered from a value the program had already destroyed. The control beside it, whose store the key scheme *can* name, stays proved. Under `--deny-warn` the same run exits **1** |
| `statement_writes` | `keller verify` exits **0** â€” a write made by a body the analysis does not read retires what it wrote, wherever the write is spelled: a callee's `&mut` argument, a `Drop` impl writing through a stored reference, a `copy_nonoverlapping` store and an inline asm block's output were **all four proved** before the rule was generalised, and each is now undecided with the write named. The asm shape pins the walk too: the assertion *after* the asm block was outside the analysis rather than undecided in it. The two controls â€” a call whose pointer names one local, and a call handed no pointer at all â€” stay proved. Under `--deny-warn` the same run exits **1** |
| `panic_sites` | `keller verify` exits **0** â€” six unmodelled panic call sites (`unwrap`, `expect`, `panic!`, `unreachable!`, the user's `assert!` and `assert_eq!`) are each a `WARN` with `kind: "unmodelled-panic"`, counted in `coverage.panic_sites` and named on their own console line. The two controls add none, and a local callee's `main` call sites are not reported again |
| `panic_decided` | `keller verify` exits **1** â€” a known enum variant decides the site: `Some`/`Ok` unwraps and the `Err` `unwrap_err` are proved, while `None`, `Err` and the `Ok` `unwrap_err` are ground `FAIL`s with `kind: "decided-panic"`. The contradictory-guard site is decided safe, and a proved decided site leaves only `coverage.panic_sites_decided` as its trace |

| `missing_toolchain` | no crate of its own: `safe_code` is verified with a real `rustup` pointed at an empty `RUSTUP_HOME`, which makes `rustup toolchain list` genuinely answer `no installed toolchains`. The run must exit **3**, name the toolchain it cannot run without and carry the `rustup toolchain install …` line, publish no `report.json`, and never reach the compiler-mismatch branch. This exercises the missing-toolchain branch against an emptied rustup home, **not** a real uninstall: the installed toolchain is untouched, which is what `CONTRIBUTING.md` requires |

The FAIL crates use *runtime* operands (parsed from `std::env::args`), so the
overflow/div-zero checks are genuine symbolic Z3 queries, not compile-time
constant folding. `fold_fail` is the deliberate exception â€” every operand there is
a constant, on purpose, so the solver is never asked â€” and `broken_code` is not
about the analysis at all: it pins the exit code for a crate the compiler rejects.
`undecided` is the exception of the other kind: its finding is not a counterexample
but a gap, and it is the only case whose run exits **0** *without* being clean.

## What `guarded` pins down

| Function | Guard | Expected |
|---|---|---|
| `sum_guarded` | `x < 100` | proved â€” `x + 1` cannot leave `u8` |
| `slice_guarded` | `i < 4` | proved â€” the guard *is* the bounds check |
| `div_guarded` | `b > 0` | proved â€” rules out both `/ 0` and `MIN / -1` |
| `dead_branch` | `x > 200 && x < 10` | proved â€” no input reaches the body |
| `sum_under_guarded` | `x < 200` | **still a FAIL** â€” `156..=199` overflows, as the witness `x = 192` shows |

With the precondition disabled (a temporary one-line change to
`PathState::assert_into`, measured and then reverted) all six obligations are SAT,
so the table above is a measurement of the guards rather than of an encoding that
stopped reporting.

## What `loop_fail` pins down

`loop_fail` is the loop probe: it is where a block inside a cycle is exercised. A cycle is
now unrolled to its back edge, so the body is asked its question under the loop's own
guards, and past `MAX_LOOP_TRIPS` (3) trips the **values** a cycle carries are widened â€”
`PathState::widen` forgets a loop-carried value, which is then re-derived on a read as a
fresh symbol bounded by the local's type. Guards accumulate; values only widen. That is
the sound direction (a forgotten value can turn a PASS into a FAIL, never the reverse) and
it is also what makes the dataflow converge, because a widened state no longer depends on
the trip before it.

The fixture holds two functions. `accumulate(250, 6)` is a `while i < steps` loop whose
body does `acc = acc + 1` and `i = i + 1`: the accumulator walks 250, 251, 252, 253, 254,
255 and the sixth (last, and reachable) trip is where `acc + 1` leaves `u8`. `count_to` is
the same loop with only the counter in it.

| Function / block | Increment | Expected |
|---|---|---|
| `accumulate` `bb2` | `acc + 1` | **FAIL** â€” witness `acc (_1) = 255`, the value the obligation pins; the guard's own symbols are left out, see below |
| `accumulate` `bb3` | `i + 1` | **proved** â€” the guard `i < steps` and `steps: u8` leave `i <= 254` |
| `count_to` `bb2` | `i + 1` | **proved** â€” the loop-counter pattern on its own, which used to be a false FAIL |

Both directions are asserted, the way `guarded` and `shift_amount` assert their rule. The
old behaviour â€” a block inside a cycle getting no state at all â€” reported `i + 1` with the
witness `i (_3) = 255`, a value the guard forbids, so the regression that matters is that
false FAIL coming back: the runner asserts the two proofs *and* that `i = 255` is gone
from the report. Losing the accumulator's overflow is the other regression â€” `acc = 250`
over six trips really does leave `u8` â€” so that FAIL has to stay, and the runner asserts it
is still there, that `obligations/loop_fail/accumulate_bb2.smt2` is conditioned on the
guard, and that the guard's own symbols are *not* in the witness. Every emitted obligation
is re-discharged like the other cases'.

That last assertion is the witness rule, and this fixture is where it shows. A FAIL's
witness is a list of *forced* values: the operands the failure is stated over, which are
what the report is about, plus each other symbol whose value the obligation pins â€” checked
by asking the solver whether the value could have been anything else. `i < steps` leaves `i`
and `steps` free, so the model's values for them are a choice; the earlier version of this
fixture printed `steps (_2) = 1, i (_3) = 0` alongside `acc (_1) = 255`, and two runs of the
same check were free to print two different pairs. The guard that bounds them is in the
obligation file instead, where it is a formula rather than a number.

The unrolling bound is a work bound, not a soundness switch: the verdicts here are
identical at `MAX_LOOP_TRIPS` 0, 3 and 8, and only the witness detail differs. Carrying
the state round the cycle is what gives a loop body its guards; the bound is what stops
the values growing once per trip.

## What `nested_loop` pins down

`nested_loop` is the second loop probe, and it is the shape that made the first one stop working.
`loop_fail` is a *single* cycle; this crate is an inner loop inside an outer one, with `continue` in
the inner body and `break` in the outer â€” and a nested loop is where a body used to fall back to the
path-insensitive state and report every increment, its guarded ones included.

The reason was the *condition* dimension, not the values. `PathState::widen` forgot a cycle's
loop-carried **values** but kept its path condition, because a guard like `inner < cap` is a fact
about control flow rather than about one trip. At a join that condition is a disjunction over the
paths that arrive, and one of them is the back edge, carrying the condition the block had on the
previous round â€” so on a header reached by a *conditional* edge it re-wrapped itself once per round
and grew instead of settling. Measured on this body, the printed condition was about 8 KB after three
rounds, 33 KB after four, 123 KB after five and 383 KB after six, roughly doubling every round; the
round cap (`MAX_ROUNDS`) was reached long before a fixpoint, and `PathAnalysis::compute` took its
documented last resort â€” `PathState::unknown()` for every block. A single loop never had the problem
because its header is entered by an *unconditional* edge, so its condition resets to "no condition"
and stabilises, which is why `loop_fail` works.

`PathAnalysis::compute` now widens the condition as well: past `MAX_LOOP_TRIPS` a cycle block's
in-state is rebuilt from its **forward** contributions alone (predecessors earlier in reverse
postorder), so the back edge no longer re-feeds the block's condition into itself. That is not a loss
of precision â€” a back edge's disjunct is a subset of a forward one, because its guards are the same
guards imposed on the same widened symbols, so the union is unchanged and what comes out is the
fixpoint rather than a step towards it.

| Function / block | Increment | Expected |
|---|---|---|
| `nested_walk` `bb4` | `inner + 1` | **proved** â€” the inner guard `inner < cap` now reaches the body (`inner <= 254`) |
| `nested_walk` `bb8` | `outer + 1` | **proved** â€” the outer guard `outer < limit` survives the inner loop |
| `nested_walk` `bb6` | `total + 1` | **FAIL** â€” a real overflow: `total` accumulates for as long as both loops run and no guard bounds it |
| `single_counter` `bb2` | `i + 1` | **proved** â€” the control, one cycle instead of two |

Both directions are asserted. The two proofs are the capability this fixture exists to protect; the
one counterexample is the *real* overflow, and its obligation must carry the guards from both headers
(`outer < limit` as `(< mir_4 mir_1)`, `inner < cap` as `(< mir_7 mir_2)`) â€” a precondition that never
mentions them is the path-insensitive obligation the fixture started as. The regression the case
catches is exactly that: `bb4` and `bb8` reappearing in the report, with `bb4`'s obligation emitted
again, is what this crate reported before the condition was widened. So the two proved increments must
also leave no `obligations/` file at all, and that absence is asserted too.

The bound is still a work bound, not a soundness switch: the verdicts here are the same at
`MAX_LOOP_TRIPS` 0, 3 and 8. And the fallback that used to fire was sound in the direction that
matters â€” a weaker precondition can only turn a PASS into a FAIL â€” so the false positives this fixture
used to report were never missed bugs, only lost precision. What removed them is bounding the
condition, not raising the round cap; that change is closed in this pass.

### The report of a lost precision

The fallback is still there, and the case above cannot show it: this crate's dataflow converges, so
there is nothing to report. What can be shown is that the *record* exists and is reachable, because the
round budget is the boundary of the finding as well as a safety net. The runner therefore invokes this
crate once more with `KELLER_MAX_ROUNDS=1`, before the default run:

* `nested_walk` and `single_counter` each get a `WARN` with `"kind": "path-insensitive"` and
  `"block": "body"`, and the console announces each one â€” a fallback nobody announces leaves the false
  positives it causes unattributable;
* the note names the budget that was hit (`round budget (1 round)`) and the direction that matters:
  `false positive` for the FAILs, `A PASS still holds` for the proofs;
* and the false positives are there â€” `bb4` and `bb8` come back as FAILs, which is exactly what the
  default budget proves and the warning explains.

That pair is the whole point of the record: one report now carries both the failure and the reason to
distrust it. It also keeps the safety net honest in the other direction, since a future change that
lets the dataflow stop converging would have to show up here rather than as a quiet increase in
reported failures.

## What `stable_pin` pins down

The crate has a `rust-toolchain.toml` of its own:

```toml
[toolchain]
channel = "stable"
```

That is what every real project has, and it is why this case exists. `cargo` is a rustup shim, so it
resolved the toolchain from the *project under verification* rather than from the compiler the verifier
links against, and `stable` cargo refuses the `-Z` flag the verify command passes. The run ended as a
setup failure (exit **3**) with no verdict, on a crate that compiles cleanly, and nothing in the
message said which toolchain was wanted. Eight of the fifteen crates this was tried on failed that way.

`keller verify` now selects the toolchain it was built against when it spawns cargo, so the analysis
runs. What the case asserts, therefore, is that a verdict was reached at all:

| Function | Expected |
|---|---|
| `unguarded_increment` | **FAIL** â€” `x + 1` on a `u8` can overflow; this is the proof that the analysis ran rather than failing setup |
| `guarded_increment` | proved â€” `if x < 100 { x + 1 }` cannot leave `u8`, and its absence from `report.json` is what rules out "the run exited 1 for some other reason" |

The project's own pin is not touched: no `rust-toolchain.toml` is written and nothing is exported into
the caller's shell, so `cargo build` and the editor in that project still use stable.

## What `shift_amount` pins down

Rust's `<<` and `>>` panic in exactly one situation: the shift amount is negative or at least the
bit width of the left operand. A value that leaves the type's range is silently truncated â€”
`200u8 << 1` is `144`, not a panic â€” which is the opposite of `+`, `-` and `*`.

The verifier used to be wrong in both directions: a symbolic shift amount was not checked at all,
and the constant path called a value-truncating shift an overflow. Both halves are pinned here.

| Function | Shift | Expected |
|---|---|---|
| `shift_by` | `x << by`, `by` from the caller | **FAIL** â€” `by >= 32` is reachable, witness `by (_2) = 32` |
| `truncate` | `if by < 32 { x << by } else { x }` | proved â€” the guard rules the panic out, and unlike before this is a proof about the shift rather than a skip |
| `discard_high_bits` | `x << 3` on a `u8` | proved â€” the value is truncated, which Rust never checks, so reporting it would be an invented counterexample |

Both directions matter: a regression that stopped checking shifts would lose the first row, and one
that went back to checking the *value* would show up as `discard_high_bits` reappearing in
`report.json`. The emitted obligations are re-discharged like every other case's.

## What `fold_fail` pins down

The other FAIL fixtures all have at least one symbolic operand, so Z3 reaches the verdict and Z3's
model names the input. In `fold_fail` both operands of every operation are constants, so the
constant-folding pass decides on its own and the solver is never consulted. That path used to be a
second-class FAIL: the operands were prose inside `details`, no `witness` was recorded, and nothing
was written under `obligations/`. So the rule the runner asserts â€” *every* FAIL names a concrete
input and can be re-discharged independently â€” held only for the obligations Z3 happened to decide,
and a folded FAIL could not be re-checked at all.

| Function | Folded check | Expected |
|---|---|---|
| `folded_add` | `200 + 100` in a `u8` | **FAIL** â€” witness `_1 = 200, _2 = 100` |
| `folded_bounds` | index `5` into a length-`3` array | **FAIL** â€” witness `_2 = 5, 3_usize = 3` |
| `folded_neg` | `-(-128i8)` | **FAIL** â€” witness `_1 = -128` |
| `folded_div_overflow` | `i32::MIN / -1` | **FAIL** â€” the one division that overflows a signed type |
| `folded_div` | `7u8 / 0` | **FAIL** â€” witness `_2 = 0` |
| `folded_rem` | `7u8 % 0` | **FAIL** â€” its own assert kind, checked separately from `/` |

Each one writes a *ground* obligation rather than the formula Z3 was handed (there was none): the
folded operands pinned to their constants, and the violation predicate in the shape the symbolic
encoder uses. It is small enough to read, its header records the verdict as `FAIL (folded)` so a
reader knows which path produced it, and an independent solver re-discharges it to `sat` like any
other pinned witness. A `details` line that ends with `-> Counterexample found by constant folding!`
names those values too.

The fixture starts with `#![allow(arithmetic_overflow, unconditional_panic)]`, and that is the
point rather than an inconvenience: rustc agrees with the analysis here, and it is `cargo build`
that rejects this code, not `cargo check`. The fixture is about the analysis, not the lint.

## What `undecided` pins down

Every other crate here pins something the tool *decides*. This one pins what it does not, and it is
the only case whose `WARN` comes from the solver rather than from the encoder declining to model
something: the obligation is expressed as an arithmetic-overflow check like any other
(`"kind": "overflow"`), and Z3 does not decide it.

`guarded_sum(step: u8, a: u64, b: u64)` has two obligations.

| Obligation | Block | Expected |
|---|---|---|
| `a * b` under `a < 2^32 && b < 2^32` | `bb2` | proved â€” the guard bounds the product below `u64::MAX` |
| `step + 1` under `product == 2^61 - 1 && a > 1 && b > 1` | `bb6` | **undecided** â€” a `WARN`: not proved, and not refuted either |

That equality has no solution: 2^61 - 1 is prime, so no pair with both factors above 1 exists, the
body is unreachable, and `step + 1` cannot overflow. The trouble is that this is a fact about
factorisation rather than about that arithmetic, and nonlinear *integer* reasoning over primes is
what Z3 cannot do. With no budget at all it does not answer `unknown`, it does not answer: measured,
it is still running after 90 seconds. So the runner gives it one â€” `--solver-timeout 5000`, more than
ten times what the crate's *other* obligation costs (about 0.4 s) and less than the time this one
does not finish in â€” and the verdict that comes back is `unknown`.

The multiplication is a checked `a * b` rather than `wrapping_mul` on purpose. `wrapping_mul` lowers
to a call to a core method, whose result the analyser cannot name, so the product would never reach
the solver as a term: the equality would be about a free symbol, and this case would collapse into an
ordinary `FAIL` naming `step = 255` â€” a counterexample the tool does not have.

What it pins is the other half of the exit-code contract. By default the run exits `0` and prints
`[WARN] Complex state in guarded_sum (bb6)`: a gap is a finding, but an *undecided* one must not fail
a build on its own, or a solver that gave up would be reported as a bug in the code. Under
`--deny-warn` the same obligation is fatal â€” exit `1`, with the gate naming which half of the
contract failed (`... are undecided (WARN) and --deny-warn was given`) instead of reusing the message
for a counterexample. The verdict also has to be the *solver's*, so the runner re-discharges the
emitted obligation with the standalone solver under the same budget and requires it to come back
unanswered too, and it asserts that no `*.witness.smt2` was pinned: `unknown` produces no model, so a
pinned file would be a value the tool invented.

## What `unnamed_write` pins down

Every other crate here is about a *value* the analysis models. This one is about a store it cannot even name, and the failure it prevents is the worst kind this tool can produce: a proof of a program that is not.

`place_key` names a place by its local and its field path, so a store through a reference â€” `*r = 250` â€” has a `Deref` in its projection and names nothing at all, and `apply_assign` used to return on that answer without touching the state. But the store had happened: `v` kept the value the copy out of `s + x` had put there, the surrounding guard bounds that by 8, and `v + 240` was answered from it and *proved*. On this crate, before the fix, the tool printed

```text
[PASS] Verified formal integrity: ref_write (bb2)
[PASS] Verified formal integrity: ref_write (bb3)
```

for a function that panics on every input its guard admits. A stale value is the only way this analysis could report a proof it does not have, which is why the fix takes the conservative direction.

A store the key scheme cannot name now retires what it may have touched and records why: its own base local â€” an index write `a[i] = â€¦` destroys the whole `a` the state may have recorded â€” and, when the projection goes *through* a pointer, every local the body has taken the address of, because which of them the reference points at is the aliasing question and this analysis does not answer it.

| Function / block | Store | Expected |
|---|---|---|
| `ref_write` `bb3` | `*r = 250`, with `r = &mut v` | **WARN, undecided**, with the store named â€” never a `PASS`, and not a counterexample either |
| `ref_write` `bb2` | `s + x` under `s < 5 && x < 5` | proved â€” nothing to do with the store, and still proved |
| `named_store_is_kept` `bb2`, `bb3` | `v = s`, a store the key scheme *can* name | proved â€” the control: the value is retired, re-derived, and the guard still bounds it |

Both halves are asserted, and the first is asserted in the direction it failed: the runner requires that `[PASS] â€¦ ref_write (bb3)` is *absent* from the console stream. When the store was ignored that line was there, so a regression that stopped retiring the storage brings it back.

The verdict is a `WARN` rather than a `FAIL` on purpose, and that is the honest answer rather than a weaker one: without aliasing, the analysis does not know the reference pointed at `v`, and what it must not do is keep claiming the value that storage held before. So the obligation is *undecided*, with the store named, and the exit-code contract is the same as `undecided`'s â€” exit `0` by default, exit `1` under `--deny-warn`, with the gate naming the verdict it failed on. Its emitted obligation re-discharges to **`sat`** under the standalone solver, which is worth stating plainly: the counterexample (`v = 250` reaches `v + 240`) really is there, and what the tool declines to do is *claim* it, because the value it would have to name came out of storage the analysis cannot follow. As with `undecided`, no `*.witness.smt2` is pinned, because a verdict that names no input has no model to pin.

What it costs is precision: the storage loses its value, so anything downstream of the store is answered over the local's type again. That direction can only add counterexamples, where keeping the stale value hid a real one â€” and the control in this crate is what keeps the cost honest, since it is the same overflow with a store the analysis can name.

## What `statement_writes` pins down

`unnamed_write` is about a store the key scheme cannot *name*. This one is about where a write is
*spelled*: `apply_assign` was the only thing that retired anything, and the call terminator retired only
its own destination, so every other way a body this analysis does not read can write â€” a callee's `&mut`
argument, drop glue, an intrinsic's store, an asm block's output â€” was invisible. Four functions here
panic on every input their guard admits, and all four were **proved** before this rule existed. The
shape is the same one `unnamed_write` reproduced: `v` holds `s + x`, the guard bounds that by
8, and `v + 240` is answered from it although the program has already replaced the value.

| Function / block | The write | Expected |
|---|---|---|
| `call_writes` `bb4` | `bump(&mut v)` â€” safe Rust, a `&mut` argument | **WARN, undecided**, naming the callee and the pointer â€” never a `PASS` |
| `drop_writes` `bb4` | `impl Drop` writing `*self.0`, reached through the scope exit | **WARN, undecided**, naming the drop |
| `intrinsic_writes` `bb4` | `core::ptr::copy_nonoverlapping(&source, &mut v, 1)` | **WARN, undecided**, naming the pointer it is given |
| `asm_writes` `bb4` | `asm!("mov {0}, 250", out(reg_byte) v)` | **WARN, undecided** â€” and, the second thing this row is for, *reached at all* |
| `unrelated_local_is_kept` `bb4` | the same call, but its pointer is `&mut w` | proved â€” the control: nothing in the body says `v` was written |
| `no_pointer_argument_is_kept` `bb4` | a call handed `v` **by value** | proved â€” a call that receives no pointer into this body's storage cannot have written it |

The retirement is one rule, in one place: a statement or terminator that writes a place the analysis
cannot model retires that place and every local the write could reach, and records *why*, so the
obligation becomes undecided-with-a-reason instead of a proof. A callee's pointer is followed one hop â€”
`f(&mut w)` says `w`, not "everything borrowed" â€” and a pointer this body never wrote is a *parameter*,
whose pointee is the caller's memory rather than one of these locals. That is what keeps the last two
rows proved: retiring every local any call might touch would buy the four WARNs above with two false
positives.

The `asm_writes` row asserts the walk as well as the write. The reachability walk did not know the
`InlineAsm` terminator at all, so the block after the asm was never entered â€” and a walk that stops cannot
report what it skipped, because the coverage line counts the assertions it *encountered*, so the summary
claimed 100% of what it had looked at. The runner's evidence for that half is the obligation file for
`asm_writes_bb4`: before the fix it did not exist, because nothing had reached the block it belongs to.

The verdicts are `WARN` rather than `FAIL` for the same reason `unnamed_write`'s is: without aliasing the
analysis cannot say the callee's pointer points at `v`, and what it must not do is *keep* the value the
program overwrote. So the same exit-code contract applies â€” exit `0` by default, exit `1` under
`--deny-warn`, with no witness pinned for a verdict that names no input â€” and each emitted obligation
still re-discharges to `sat` under the standalone solver, because the counterexample is real and the
attribution is what is missing.

## What `precond_probe` pins down

Two questions that are easy to conflate, separated by one run. The crate writes a precondition as
`assert!(x < 100)` at the top of `entry_assert` and calls its callees only with constants that cannot
overflow:

- **A precondition asserted in the body is believed, not checked.** The obligation for `x + 1` in
  `entry_assert` is `PASS (unsat)` and its `; reachable:` comment is the assertion's own condition —
  the proof *uses* `x < 100` as a guard. A run that stopped assuming it would have to report that
  overflow as a FAIL, which is why the case asserts the artifact and not just the verdict. The
  assertion itself is not one of the encountered obligations at all (the user's `assert!` does not
  produce a checkable MIR assert — see A6), so the inventory this case pins
  is **3 counterexamples and 1 proof**.
- **A callee is judged on its own body, not on its call sites.** `private_callee(7)` and
  `public_callee(9)` are the only calls, both safe, and both bodies are still reported with
  `x = 255`. There is no cross-function reasoning yet (A8), and the day there is, this case has to be
  re-read — it is asserted as a *boundary*, and its failure message says so.

## What `test-crates/buggy-project` and `safe-project` pin down

They are the missing-finding and false-finding pair, and they live at the repository root because
that is where they were written. `buggy-project` asks for one counterexample of each class the report
can state — `bounds` (with a one-of-many witness: the index the solver picks is one of many),
`overflow` and `divzero` — plus a control that must be **proved** (`check_div bb1`, the branch where
the divisor cannot be zero), so a change that reported every division would still fail here.
`safe-project` must exit `0` **and** report nothing; an empty report is written as `[]`, so the check
is that it carries no findings rather than that it is missing.

Both runners reach them by path: a case is `tests/<name>` unless a directory with that name exists at
the root, in which case it is that. The measured console log is named after the case
(`test-crates/buggy-project` becomes `test-crates__buggy-project.log`) — a redirect into a directory
that does not exist would silently skip the run, which is exactly how a fixture ends up in a suite
while proving nothing.

## Why `broken_code` exists

`broken_code` is not a program with a bug in it. It is a program the *compiler* refuses, and the
only thing asserted about it is the exit code.

The wrapper runs rustc itself, and it used to discard the status rustc returned â€” so a crate the
compiler rejected came out as **exit 0 with an empty `report.json`**. The analysis had found no
assertions in a body that was never built, and "no findings" was indistinguishable from "nothing to
check". For a tool whose whole claim is that it never says *safe* without checking, that was the one
wrong answer worth a fixture of its own: the crate must exit `3` ("the analysis could not run"), and
`0` is the specific outcome this case exists to catch.

## The landing page is hand-written

`index.html` is a plain page with no generated regions and no per-run figures. It used to quote this
suite's numbers, which meant a generator (`tests/gen_index.py`) rebuilt parts of it from the captured
logs and the suite failed when the committed page stopped matching the run. That is gone: an edit to
the page is now one file, and the evidence it used to reproduce lives in this directory where a
reader can run it.

The captured console streams under `tests/.measured/` (gitignored) stay, because the later cases in
`tests/run_tests.sh` grep them for the lines they assert on.
## How to run

Windows:

```powershell
tests\run_tests.cmd
```

Linux / macOS:

```bash
tests/run_tests.sh
```

The runner builds the verifier and the `keller-env` helper if needed, resolves the toolchain and Z3
through that helper (see Prerequisites), and then, for each crate:

1. `cd` into the crate and run `keller verify`
2. asserts the expected exit code
3. for FAIL cases, asserts `report.json` contains a `"status": "FAIL"` entry
4. asserts every FAIL names its triggering input â€” a `witness` entry *and* a
   `details` line carrying the values
5. re-discharges every emitted `obligations/**/*.smt2` with the standalone solver;
   a pinned `*.witness.smt2` must answer **`sat`**, since it asserts the reported
   values â€” `unsat` there would mean `report.json` named an input that does not
   fail (the check was verified to bite by corrupting a pinned value)
6. asserts every obligation has a pinned witness counterpart
7. for `guarded`, asserts path sensitivity in both directions: nothing for
   `sum_guarded` / `slice_guarded` / `div_guarded` / `dead_branch` (they are
   proved), and a FAIL for `sum_under_guarded` â€” a regression that stopped
   reporting overflows would pass the first half, one that ignored the guards
   fails it
8. for `workspace_multi`, asserts *both* members survive the merge
9. for `loop_fail`, asserts the loop rule in both directions: the counter increments
   under `i < steps` (in `accumulate` and in `count_to`) are proved, the value the old
   fallback named (`i = 255`) is gone, and the accumulator's reachable overflow is
   still a FAIL whose obligation carries the loop's guard
10. for `nested_loop`, asserts the cycle widening one level deeper: the nested body's guarded
    increments (`bb4`, `bb8`) and the single-cycle control (`single_counter`) are all **proved**, only
    the unbounded `total + 1` (`bb6`) is reported, and its obligation carries the guards from *both*
    loop headers â€” the assertion that flips if the condition stops being widened
11. for `shift_amount`, asserts shifts in both directions: the out-of-range amount is reported with
    `by` named as the failing input, while the guarded shift and the truncating constant shift are
    proved and stay out of the report
12. for `fold_fail`, asserts the folded path is a first-class one: all six constant-folded failures
    are reported, each carries a witness, each says how it was decided, and each has a ground
    obligation re-discharged to `sat`
13. for `broken_code`, asserts a crate the compiler rejects exits `3` â€” the regression it catches is
    that same crate exiting `0`
14. asserts every finding carries its source `file`/`line` and its machine-readable `kind`, so a
    consumer does not have to match on the prose in `details`
15. compares each crate's whole `report.json` against the committed golden under `tests/goldens/`.
    The case checks above assert the fields someone thought to assert, which leaves the rest of the
    file unguarded; re-record an intended change with `UPDATE_GOLDENS=1 tests/run_tests.sh` and read
    the diff as the review
16. runs the exit-code contract's own flags: `--deny-warn` must leave a clean run
   clean and keep a counterexample fatal, and an unknown flag must exit `2`
   rather than silently leaving the stricter gate switched off
17. for `undecided`, asserts the other half of that contract with a verdict rather than a flag: the
    `WARN` is recorded and classed (`status`, `kind`, `function`), announced on the console, is *not*
    laundered into a `FAIL`, pins no witness (there is no model to pin), matches its golden byte for
    byte, and is fatal under `--deny-warn` â€” exit `1`, with the gate naming the undecided verdict.
    The emitted obligation is re-discharged with the standalone solver under the same budget and has
    to come back unanswered as well, so the verdict is the solver's rather than the tool's story
    about it. Both runs clear `target` first, because cargo's fingerprints do not notice that only
    the solver budget changed
18. for `unnamed_write`, asserts the store the key scheme cannot name in both directions: the
    obligation that used to be proved from the value it destroyed is not proved again (the runner
    requires the old `[PASS] â€¦ ref_write (bb3)` line to be *absent*), the verdict is recorded as a
    `WARN` naming the store and announced on the console, nothing is laundered into a `FAIL`, the
    emitted obligation re-discharges to `sat`, no witness is pinned, the control whose store *is*
    nameable stays proved, and the same obligation is fatal under `--deny-warn`
19. for `statement_writes`, asserts every *spelling* of a write by a body the analysis does not read:
    the four shapes that used to be proved from the value they overwrote are not proved again (the
    runner requires each old `[PASS] â€¦ (bb4)` line to be *absent*), each is recorded as a `WARN`
    naming the write and announced on the console, nothing is laundered into a `FAIL`, the assertion
    after the asm block is reached at all (its obligation file exists), the two controls that no
    unread body could have written stay proved, and the run is fatal under `--deny-warn`
20. leaves `index.html` alone: it is a hand-written page with no per-run figures, so there is nothing
    in it for this run to disagree with
21. recomputes the K7 false-positive budget over the committed goldens
    (`scripts/fp-rate.py --dir tests/goldens --budget 0.15`), which fails the run when too many of the
    corpus's `FAIL`s are attributed to the loop abstraction. The attribution it counts is a dependence
    test over the value in each finding, not a label on the block it was found in,
    and the gate is shown to be fallible rather than assumed: `0.111` passes at `0.15` and the same
    command exits `1` at `--budget 0.10`. A missing Python is a `[SKIP]` with the run still failing
    loudly, not a silent pass

Exit code 0 = all cases passed.

**Two runs in this pass did not reproduce, and are not written up as resolved.**
The suite exited `3` once on the `workspace_multi` case, and one intermediate run reported six `FAIL`s
â€” the `loop_fail`/`nested_loop` proofs plus a stale page â€” none of which recurred on the same bytes,
while that run's own captured logs and obligation files contain the proofs its runner said were
missing. The cause is unestablished. One observation in the same window, with its numbers rather than
as an explanation: a `keller-verify.exe` (**WINPID 12404**, started `03:08`) left behind by the earlier
sweep was still alive about six hours later, blocked with its `workspace-wrapper.exe` child in an
unrelated crate's `.keller/runs/` directory and holding `0.06 s` of CPU â€” an orphan, not a load, and
it is recorded because the suite reported failures it cannot explain, not because it is known to have
caused them.

## Prerequisites

- `rustup` with the pinned nightly (`tests/rust-toolchain.toml` inherits from
  the repo root's pin, so the test crates compile with the same toolchain the
  verifier was built against â€” a mismatch would crash the wrapper).
- Z3: the runners ask `target/debug/keller-env`, the same program `cargo build` produces beside the
  verifier, and never look for themselves. The order is `KELLER_Z3` (an explicit solver or install
  root), a `z3` already on `PATH`, `Z3_HOME`, the project-local install (`.z3/current`, else the last
  of the sorted releases â€” the pick `build.rs` links against), then the platform default. `Z3_HOME` is
  therefore optional: unpack a release under `.z3/`, which is where CI puts it, or install Z3
  system-wide. A solver that exists but does not start is skipped rather than half-used, and a run that
  finds none reports `[SKIP]` for each obligation it cannot re-discharge rather than treating them as
  checked.
- Both runners read `tests/cases.txt` for the case list, the exit code each case must produce and
  which cases have a golden. That file is the single owner of those three facts, and each runner checks
  both ends of it: every row must be consumed (a case needing its own invocation - a solver budget, an
  undecided verdict, the precision probe - marks itself), and the rows naming a golden must be exactly
  the files in `tests/goldens`. It exists because the two runners had drifted: the Windows runner
  compared ten goldens against the shell runner's thirteen and ran two fewer cases, and nothing in the
  tree said which number was right. A case added to the table and missing from a runner now fails that
  runner by name rather than being silently absent from it.
- The Windows runner (`run_tests.cmd`) now asserts the same cases and the same properties as the shell
  one, and the few differences left are in how a fact is read, not in what is claimed. Both runners
  re-discharge every emitted obligation with the standalone solver and compare a pinned `*.witness.smt2`
  to `sat`, check the shift witness's exact value (`by (_2) = 32`) rather than only the finding's
  presence, run the `--run-timeout` cap probe with its process-tree check, and fail by name when a
  golden differs. The mechanical difference is the diff itself: the shell runner prints `diff -u`, while
  the Windows runner decides with `fc /b` and then prints a line-numbered `fc /n` so the change is
  readable, which is one command fewer for the same evidence.
  What it does now run is every case in the table, including `unnamed_write`, `statement_writes` and
  `potency_probe`, the three it had no block for, with the assertions those fixtures exist for (the
  verdict, the class and the function, the named write or the announced undecided verdict, the controls
  that must stay proved, the `sat` re-discharge of the obligations whose counterexample is the point,
  and the same obligation fatal under `--deny-warn`). One difference this file has to name, because it
  is new and it is the kind that should not be discovered by reading a diff: `panic_sites` is asserted
  on Windows from the console line that carries the count (`6 call site(s) that can panic were not
  modelled`, `Panic sites not modelled: 6`) where the shell runner reads the number out of
  `report.run.json`, because `findstr` has no escape for a quoted key. The claims are the same; the
  probe differs, and the block says so where it is written.
  `run_tests.sh` is the gate the workflow runs (*Fixture suite*); the workflow is in the tree, and no
  run of it can be shown from this checkout.
- Both runners get the toolchain and the solver from **one** implementation: `src/host_env.rs`,
  which `build.rs` includes (so the verifier links against the release the suite will run) and
  `src/bin/keller-env.rs` prints as `KEY=VALUE` lines. The shell runner asks for `--shell sh` and
  `eval`s the quoted assignments; the Windows runner reads `--shell plain` from a file, so a warning on
  stderr is still shown, and neither script carries a lookup of its own any more. The helper is a
  second binary rather than a `keller verify` subcommand because the verifier is linked against Z3 and
  loads the toolchain's driver dylibs at run time: it cannot start until the directories the helper
  reports are already on `PATH`. The dylib directory is derived from the rustc `rustup` names for this
  checkout, so whichever host triple is active (`-msvc` or `-gnu`, including a `rust-toolchain.toml`
  override) is used, and a missing one is now a fatal setup error before the first case instead of
  every case dying at load one at a time.
- That sharing is enforced rather than asked for: the unit test
  `the_runners_do_not_look_for_the_toolchain_or_z3_themselves` scans both scripts for the primitives a
  lookup would need (comments excluded) and fails the build if either grows one back. It was verified to
  bite by planting a lookup in the Windows runner and watching it fail with the file, line and primitive
  named.
- The Windows runner's own syntax is checked the same way. `cmd` ends a parenthesised block at the first
  `)` its parser meets, so an `echo` inside one that carries a bare parenthesis closes the block early
  and the lines under it become top-level commands: `echo [FAIL] … produced %N% finding(s)` in the
  safe-project check put `set "FAILED=1"` outside its condition, and the runner then printed every check
  as passing, printed no `[FAIL]` at all and exited 1 — a gate that fails without saying what failed. The
  rule is in the script's own comment; `no_echo_inside_a_cmd_block_carries_a_bare_parenthesis` asserts it
  on the file (escaped `^(…^)` is accepted, quoted parentheses are ignored) and fails `cargo test`
  naming the line.
- Why the discovery exists at all, because it is the history these scripts are written against:
  `run_tests.cmd` used to hardcode the msvc toolchain and a `C:\z3-4.16.0-x64-win` install; when neither
  matched, the wrapper could not start and every case was reported as a verdict failure
  (`STATUS_DLL_NOT_FOUND`, exit `-1073741515`) instead of a setup one. The shell runner had the same
  failure in a quieter form: it built that dylib path with `xargs dirname`, which splits it at the space
  in a Windows user path, so nothing reached `PATH` and every case died at load with exit `127` â€” the
  code an unfound command gives. The build took the *last* of several sorted releases unpacked under
  `.z3/` while the shell runner took the *first* with a runnable `z3`, and the Windows runner globbed
  none of them, so a checkout with two releases unpacked could be linked against one and re-discharged
  by the other.

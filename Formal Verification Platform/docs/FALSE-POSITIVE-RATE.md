# The false-positive rate on code nobody here wrote

**K7.** K5 closed the one false positive that could be reproduced on
demand (a loop's exit value reported as the violation). This is the other half of
the question a buyer asks: on code this project did not write, how often is a
`FAIL` not a bug? The measurement, its method, and its limits are below, and the
machine-readable table is
[measurements/fp-rate-2026-10-13.json](measurements/fp-rate-2026-10-13.json).

**The number: 1 of 9 `FAIL`s (0.111) is attributed to the loop abstraction, and 5
of the 9 carry a witness at a type's boundary (`u64::MAX`-shaped) — so read the
rate as a floor, not as the answer.** The section below says why those two numbers
are stated together rather than one of them.

The class that number counts **changed after the first measurement**, because the
test behind it could not fail: `widening-limited` used to ask whether the state
mentioned a previous-trip symbol, which the loop widening declares for every symbol
on the back edge, so any finding near a widened block was that class by
construction. It is now a dependence test over the value the
finding is about. The four crates that carried the sample's `FAIL`s were
**re-measured** with it on 2026-10-13 and the result is in the same section: the
finding sets are identical finding for finding and **no class moved**, so the figure
below is 1 of 9 both before and after — the difference is that it is now a
measurement rather than a label.

## The method

Every `FAIL` carries a `defect_class` (K6), decided from the state the obligation
was checked in. That is what makes a rate measurable instead of a hand audit:

| class | what it means | in the rate? |
|---|---|---|
| `widening-limited` | the value the finding is **about** leans on the loop abstraction: one of the failing assertion's own operand places, or one of its witnesses, is a previous-trip symbol the widening introduced, or a place the widening *emptied* because the back edge carries it, or is defined from one through the state's own definitions. A widened symbol declared for a place this finding never reads is **not** evidence, and neither is sitting inside a widened block | **yes, the headline rate** |
| `guard-limited` | the guards that lead to the finding are all in the formula and the model satisfies them | no — these are the ones to look at first |
| `unguarded` | the state carried no path condition at all | no — a straight-line obligation legitimately has none, so the class mixes genuine findings with lost guards |
| `constant-folded` | decided from literals, under no symbolic state | no |
| `known-panic` | a decided panic site: the receiver's enum variant is known (`None`, `Err`), so the call always panics and there is no witness to model | no — excluded from the denominator |
| `extreme` (cross-check) | a `FAIL` whose witness sits at the top of an unsigned range, at `i64::MAX`, or at `2^128-1` — a value a program's own preconditions usually exclude | no, reported beside the rate |

`rate = widening-limited / (FAIL - decided panics)`. `WARN`s are obligations the analysis *declined*
to decide: nothing was claimed there, so they are counted as `undecided` and kept
out of the rate. A finding that appears in two reports — a crate analysed in two
runs because it is a dependency of both targets — is counted once, identified by
crate, function, block, status, kind, details and witness; the sample below had
two such findings, and counting them twice would have inflated both the numerator
and the denominator. The classifier is
[scripts/fp-rate.py](../scripts/fp-rate.py); the sweep is
[scripts/sweep-local-crates.ps1](../scripts/sweep-local-crates.ps1), and the
per-crate cap is what keeps one slow crate from swallowing the time box
(`-TimeoutSeconds N`; a killed run reports `TIMEDOUT` and leaves no `report.json`,
which is why it counts as *not measured* rather than as a crate with no findings).

Reproduce it with:

```text
py scripts/fp-rate.py <crate>/report.json ... [--budget 0.15] [--json out.json]
powershell -NoProfile -ExecutionPolicy Bypass -Command \
  "& '.\scripts\sweep-local-crates.ps1' -Only @('zk-core','sca-security','smpc-commitments', ...) -TimeoutSeconds 150"
```

## The sample (2026-10-13, Windows host, rustc 1.99.0-nightly / `nightly-2026-07-24`, Z3 4.16.0)

Sixteen crates were attempted. Seven runs completed and wrote a report: five with
findings, two with none at all. Nine were not measured — eight ran past their cap
and were killed, one failed to set up — and none of them is counted as clean.

| crate | from | `FAIL` | widening | guard | unguarded | folded | extreme | undecided |
|---|---|---|---|---|---|---|---|---|
| `sca_security` | `Algorithmic Supply Chain Arbitrage/crates/sca-security` | 3 | 1 | 0 | 2 | 0 | 3 | 10 |
| `smpc_engine` | `SMPC Dark Pool/crates/homomorphic-compare` | 4 | 0 | 0 | 4 | 0 | 0 | 0 |
| `sss` | `SMPC Dark Pool/crates/{homomorphic-compare,threshold-crypto}` | 1 | 0 | 0 | 1 | 0 | 1 | 1 |
| `zk_genome_zk_core` | `Zero-Knowledge Genome/core/zk-core` | 1 | 0 | 0 | 1 | 0 | 1 | 0 |
| `op_math` | `OP-Roboter/crates/op-math` | 0 | 0 | 0 | 0 | 0 | 0 | 12 |
| `city_types` | `City OS/crates/city-types` | 0 | — | — | — | — | — | — (20 functions, no assertions) |
| `smpc_commitments` | `SMPC Dark Pool/crates/commitments` | 0 | — | — | — | — | — | — (no findings) |
| **total** | 7 reports | **9** | **1** | **0** | **8** | **0** | **5** | **23** |

The four rows with findings were **re-measured on 2026-10-13** (the crates are on
this host; each run is `target/debug/keller-verify.exe keller verify --quiet` with the
crate as the working directory, capped at 420 s by
[scripts/sweep-local-crates.ps1](../scripts/sweep-local-crates.ps1)) under the
classifier of A19. What the re-measurement shows, finding for finding:

| crate (report) | findings, old → new | classes moved | `FAIL` / `WARN` in the report | seconds |
|---|---|---|---|---|
| `sca-security` | 13 → 13 | 0 | 3 / 10 | 56.8 |
| `homomorphic-compare` | 6 → 6 | 0 | 5 / 1 | 14.0 |
| `threshold-crypto` | 2 → 2 | 0 | 1 / 1 | 22.8 |
| `zk-core` | 1 → 1 | 0 | 1 / 0 | 1.9 |

A report counts more `FAIL`s than its crate's row because one run analyses the crate
and the crates it depends on: `homomorphic-compare`'s five are four of its own plus
the shared `sss`-named finding, which `threshold-crypto` reports as well and which the
classifier counts once.

Nothing was added, nothing disappeared and no finding changed class, so the table's
figure is the same under both tests — and the one attribution to the loop
abstraction is the finding the old test got *right*. The machine-readable tables are
[measurements/fp-rate-2026-10-13.json](measurements/fp-rate-2026-10-13.json) (first
measurement) and
[measurements/fp-rate-2026-10-13-recheck.json](measurements/fp-rate-2026-10-13-recheck.json)
(the re-measurement). The three zero-`FAIL` rows are as first measured; a crate with
no findings has no class to re-measure.

The crate is the name the findings carry, which is why `sss` lists two source
reports: one run analysed the crate as a dependency of both targets, and its two
duplicate findings are counted once (the total would otherwise read 10, 6 and 24).

Attempted and **not measured** — each exceeded its cap and was killed, so it
contributes nothing rather than being read as clean:

| crate (directory) | what happened |
|---|---|
| `Transit OS/crates/state-machine` | killed after ~14 minutes with no cap (serde-heavy; tens of functions) |
| `SCA/crates/sca-matching`, `OP-Roboter/crates/op-gate`, `City OS/crates/city-dag` | killed after ~12 minutes under a 420 s cap that the MSYS `timeout` did not enforce |
| `SCA/crates/sca-types`, `Transit OS/crates/{smt-solver,path-constraints}`, `Autnomous Proof-of-Damage Layer/crates/physics-logic` | killed at 162 s under a 150 s cap |
| `City OS/crates/city-brp` | setup failure (0 s, exit 1, log held by another process): not a verification run |

## What the numbers say

- **The finding shape that matters is `unguarded` with a boundary witness.** Eight
  of the nine `FAIL`s are `unguarded` — the analysis is saying "here is a
  satisfying model", not "the violation is reachable under these guards". Five of
  the nine have a witness at the type's boundary: `ipc::IpcChannel::create_message`
  reports `((*_1).1: u64) = 18446744073709551615`, `temporal::Schedule::add_slot`
  reports `_5 = 1, _6 = 18446744073709551615`. Those are values a real caller's
  own invariants would exclude, and they are exactly the shape flagged in an earlier
  City OS crate (51 of 118 findings).
- **One finding is attributed to the loop abstraction, and that attribution is
  the honest one — including under the stricter test:**
  `memory_encryption::MemoryEncryptor::xor_with_keystream` (`bb25`), witness
  `counter = 18446744073709551615, 1_u64 = 1`. The counter is a loop-carried value
  the widening generalised and no guard relates it to the bound that would rule the
  failure out — which is precisely the case A13 (loop invariants) exists for, and
  why K7 had to land before A13 can be argued about. Under A19's dependence test it
  is still the only `widening-limited` finding in the whole sample, and the two other
  `FAIL`s in that crate — straight-line functions with no widened block — are
  `unguarded` and stay `unguarded`: the class now separates them, which is the thing
  the location test could not do.
- **No `guard-limited` findings at all.** None of the nine is a violation the
  analysis can defend as reachable under guards it captured. On this sample the
  tool has not produced the thing it is best at; it has produced models.
- **The rate's denominator is small (9), so treat 0.111 as an order of magnitude,
  not as a decimal.** The class-based rate and the boundary-witness cross-check
  disagree by 5×, and the cross-check is the more conservative view of the same
  evidence. Publishing the pair is the point: a single number here would be a
  claim the sample cannot support.

## What this does not show

- **It is not ground truth.** The classes are the analysis' *own* attribution. A
  `guard-limited` `FAIL` could still be wrong if the guard itself is wrong, and an
  `unguarded` one could be a real bug. Deciding which needs the next two rows:
  a replayable witness (K10) and an executable reproduction (the witness a
  developer can paste into `#[test]`).
- **It is a snapshot, not a monitor.** The crates are local projects on one host,
  not a corpus anyone else can fetch. The budget therefore runs over the *committed*
  corpus, where the same attribution is recomputed from the checked-in goldens — see
  below — and the real-crate figure is re-measurable with the command above, not
  continuously measured.
- **Crates where the analysis declines contribute nothing.** `op_math`'s 12
  obligations are all `WARN`, and a crate with no `FAIL`s has no rate. That is
  reported rather than smoothed over: a rate is silent about the code the tool
  could not decide, and on this sample that is 23 obligations against 9 findings.

## The budget

`--budget 0.15`. The curated corpus sits at **0.111** (2 of 18: the loop fixtures
`loop_fail` and `nested_loop`, whose failing operands are loop-carried values the
widening gave up on — `widening-limited` on that evidence, not because a block near
them was widened) and the real-crate sample at **0.111** (1 of 9), so 0.15 leaves room
for one more finding to be attributed to widening before the gate complains — which is
the point: it is a prompt to look, not a proof of regression. The corpus now contains
21 `FAIL`s; the three decided panics in `tests/panic_decided` are ground facts about a
known enum variant, so they are counted in that total and excluded from the rate's
denominator. **The gate's failure path is demonstrated rather than assumed**: on the
committed corpus, `--budget 0.15` exits `0` and the same command with `--budget 0.10`
exits `1` (`[FAIL] the suspect rate 0.111 is over the budget 0.100`). At the CI budget
the margin is one finding: a third widening-attributed `FAIL` in that denominator of
18 would read `0.167`.

The computation runs in `tests/run_tests.sh` (step 21 of the fixture suite), over the
committed goldens, so a local run fails the way the gate is meant to. The workflow's
**False-positive budget** step carries the same command, but no run of that workflow
has been observed from this checkout: the command is shown, not a CI result. The local
run of the same command against the committed goldens is the evidence above.

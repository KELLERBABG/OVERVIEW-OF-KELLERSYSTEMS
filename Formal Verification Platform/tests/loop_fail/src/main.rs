// A loop body's obligations, analysed with path state.
//
// A cycle used to be cut out of the analysis: every block inside one got no
// state at all (`cycle_blocks`), so a loop body was checked path-insensitively.
// Its iteration variable was a free `u8`, and the body's `i + 1` was asked
// whether *any* `u8` overflows — which one does, at 255, even though the loop's
// own guard makes that value unreachable.
//
// The walk now unrolls a cycle to its back edge for a bounded number of trips
// and then widens the loop-carried *values*. The merged state therefore stops
// growing once per trip, while the guards that lead into the body stay. Both
// halves of that are visible in this crate.
//
//   * `i = i + 1` at bb3 is a *proof* now. The state reaching it carries the
//     loop's guard, `i < steps`, and `steps` is a `u8`, so `i <= 254` and
//     `i + 1` cannot overflow. The `i (_3) = 255` witness the fallback used to
//     report was a value the guard forbids. `count_to` below is the same proof
//     with nothing else in the loop body.
//
//   * `acc = acc + 1` at bb2 is still a FAIL, and correctly so: `acc` can be
//     255 and the loop does run. Its witness is `acc = 255` — the value the
//     obligation *pins* — and not the guard's own symbols, whose values
//     `i < steps` leaves the model free to choose. Starting from `acc = 250`,
//     the sixth trip (250, 251, ... 255) is the one that overflows; the widened
//     value cannot name that trip count, and the obligation file is where the
//     guard that bounds it is recorded.
//
// Both directions are asserted by tests/run_tests.sh. A change that widened the
// guard away turns bb3 back into a false FAIL; one that forgot to widen lets a
// term grow once per trip, and one that cut the back edge instead of widening it
// would lose the states a trip count beyond the bound can reach.
fn accumulate(mut acc: u8, steps: u8) -> u8 {
    let mut i = 0u8;
    while i < steps {
        acc = acc + 1;
        i = i + 1;
    }
    acc
}

/// The loop-counter pattern, on its own: `i + 1` under the guard `i < limit`.
///
/// Nothing here can overflow, so the function is proved and stays out of
/// report.json. Path-insensitively it was a false FAIL — the same `i = 255`
/// witness `accumulate`'s bb3 used to produce — and that false positive applied
/// to every loop in every program the verifier looked at.
fn count_to(limit: u8) -> u8 {
    let mut i = 0u8;
    while i < limit {
        i = i + 1;
    }
    i
}

fn main() {
    // 250 + 6 = 256 -> overflows u8 on the last iteration.
    let result = accumulate(250, 6);
    println!("{}", count_to(result));
}

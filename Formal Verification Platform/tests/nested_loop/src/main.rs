// A *nested* loop, and what the cycle abstraction does with it.
//
// `loop_fail` is the single-cycle probe; this crate is the shape next door — an
// inner loop inside an outer one, with `continue` in the inner body and `break`
// in the outer — and it is the one that used to fall back to the path-insensitive
// state and report every increment, its guarded ones included.
//
// The reason was the *condition*, not the values. `PathState::widen` forgets a
// cycle's loop-carried values but keeps its path condition, because a guard such
// as `inner < cap` is a fact about control flow rather than about one trip. At a
// join that condition is a disjunction over the paths that arrive, and one of
// them is the back edge — which carries the condition the block had on the
// previous round. So on a shape whose header is reached by a *conditional* edge
// the condition re-wrapped itself once per round and grew instead of settling:
// measured on this body, about 8 KB after three rounds, 33 KB after four, 123 KB
// after five and 383 KB after six, roughly doubling. The round cap was reached
// long before a fixpoint, and `PathAnalysis::compute` then took its documented
// last resort — `PathState::unknown()` for every block.
//
// A single loop never had that problem: its header is entered by an
// *unconditional* edge, so the "no condition" path makes the header's condition
// `None` and it stabilises immediately, which is why `loop_fail` worked.
//
// `PathAnalysis::compute` now widens the condition as well. Past `MAX_LOOP_TRIPS`
// a cycle block's in-state is rebuilt from its *forward* contributions alone
// (predecessors earlier in reverse postorder), so the back edge no longer
// re-feeds the block's condition into itself. That is not a loss of precision: a
// back edge's disjunct is a subset of a forward one, because its guards are the
// same guards imposed on the same widened symbols, so the union is unchanged and
// what comes out is the fixpoint rather than a step towards it.
//
// So all three of this body's increments are now checked under the guards that
// really lead into them:
//
//   * `inner + 1` at bb4 is *proved* — `inner < cap` with a `u8` `cap` leaves
//     `inner <= 254`.
//   * `outer + 1` at bb8 is *proved* — the outer guard `outer < limit` survives
//     the inner loop and bounds `outer` the same way.
//   * `total + 1` at bb6 is still a FAIL, and correctly so: `total` accumulates
//     for as long as both loops run and no guard bounds it.
//
// Both directions are asserted by tests/run_tests.sh: the two proofs, the one
// counterexample with its obligation conditioned on *both* headers' guards, and
// the single-cycle control below. A regression that stopped widening the
// condition brings bb4 and bb8 back into the report — which is exactly what this
// crate reported before the fix.
fn nested_walk(limit: u8, cap: u8) -> u8 {
    let mut total = 0u8;
    let mut outer = 0u8;
    while outer < limit {
        let mut inner = 0u8;
        while inner < cap {
            // bb4: proved — the inner guard `inner < cap` forces `inner <= 254`.
            inner = inner + 1;
            if inner == 3 {
                // The `continue` back edge to the inner header, bb5 -> bb3.
                continue;
            }
            // bb6: a real overflow — `total` accumulates for as long as both
            // loops run and is bounded by no guard.
            total = total + 1;
        }
        // bb8: proved — the outer guard `outer < limit` survives the inner loop.
        outer = outer + 1;
        if outer == 7 {
            // The `break` out of the outer loop, bb9 -> bb10.
            break;
        }
    }
    total
}

/// The control: one cycle around the counter instead of two. It converges too,
/// so `single_counter (bb2)` is proved and stays out of report.json.
fn single_counter(limit: u8) -> u8 {
    let mut i = 0u8;
    while i < limit {
        i = i + 1;
    }
    i
}

fn main() {
    // Many trips, so the nested `total + 1` really does leave u8.
    let result = nested_walk(250, 4);
    println!("{}", single_counter(result));
}

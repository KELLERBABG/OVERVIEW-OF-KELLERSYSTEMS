// The undecided-verdict probe.
//
// Every other crate here pins something the tool decides. This one pins what it
// cannot, and it is the only crate built to produce a `WARN` from the *solver*
// rather than from the encoder declining to model something: the verdict is
// `unknown` because Z3 genuinely cannot answer, not because a budget was set
// too small or an encoder gave up.
//
// Reaching the body of `guarded_sum` needs a and b with
//
//     a * b == 2^61 - 1     and     a > 1     and     b > 1
//
// and 2^61 - 1 is prime, so no such pair exists: the body is unreachable and
// `step + 1` can never overflow. That is a fact about factorisation, and Z3's
// *nonlinear integer* arithmetic cannot establish it. With no budget Z3 does not
// answer at all - measured, it is still running after 90 seconds, and it does not
// answer within 30 seconds of its own either - and with a budget it answers
// `unknown`. So the tool reports `undecided` rather than a proof it does not
// have. A `WARN` is the honest verdict here, and it is the reason the exit-code
// contract has a second half: a `WARN` is tolerated by default and fatal under
// `--deny-warn`, which is what the runners assert against this crate.
//
// The multiplication is a checked `a * b` under a guard that makes *it* provably
// safe, rather than `wrapping_mul`. That is deliberate. `wrapping_mul` lowers to
// a call to a core method, whose result the analyser cannot name, so the product
// would never reach the solver as a term: the equality would be about a free
// symbol and this would be an ordinary `FAIL` naming `step = 255` instead of an
// `unknown`. A checked multiply gives the state the product term `(* a b)` as
// the tuple's value field, while its own overflow assert is discharged by the
// guard - so the crate has exactly two obligations, one proved and one
// undecided, and nothing else can decide the run.

fn main() {}

/// One proved obligation, and one the solver cannot decide.
pub fn guarded_sum(step: u8, a: u64, b: u64) -> u8 {
    if a < 4_294_967_296 && b < 4_294_967_296 {
        // Proved: (2^32 - 1)^2 = 2^64 - 2^33 + 1, which is below u64::MAX, so
        // this multiply cannot overflow and the obligation is discharged.
        let product = a * b;
        // Undecided: this equality has no solution with both factors above 1,
        // but only factorisation shows that.
        if product == 2_305_843_009_213_693_951 && a > 1 && b > 1 {
            step + 1
        } else {
            step
        }
    } else {
        step
    }
}

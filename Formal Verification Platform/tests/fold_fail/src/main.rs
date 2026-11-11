// Failures the verifier finds *without asking Z3*.
//
// Every check in the other fixtures has at least one symbolic operand, so it is
// the solver that reaches a verdict and the solver's model that names the input.
// Here nothing is symbolic: both operands of each operation are constants, so the
// constant-folding pass decides on its own and the solver is never asked.
//
// That path used to be a second-class citizen. It reported a FAIL with the
// operands spelled out in prose, but with no structured witness and no file under
// `obligations/` — so the fixture suite's rule that *every* FAIL names a concrete
// input and can be re-discharged independently held only for solver-found
// failures. This fixture makes that hole visible and permanent: all six folded
// sites must produce a FAIL that carries a witness and a ground obligation.
//
// Two allowances are needed, and they are the point rather than an inconvenience.
// rustc happens to agree with the analysis here: `arithmetic_overflow` and
// `unconditional_panic` are deny-by-default, and because `cargo check` does not
// force the MIR passes that raise them, a plain `cargo check` accepts this file
// while `cargo build` rejects it. This fixture is about the *analysis*, not the
// lint, so the lints are allowed and the folded path is left to be checked.
#![allow(arithmetic_overflow, unconditional_panic)]

/// Both operands are constants: `200 + 100` overflows a `u8` at 255.
pub fn folded_add() -> u8 {
    let a: u8 = 200;
    let b: u8 = 100;
    a + b
}

/// A constant index into a fixed-size array. The index and the length are both
/// known, so the bounds check is decided by folding: `5 >= 3`.
pub fn folded_bounds() -> u8 {
    let a = [1u8, 2, 3];
    let i: usize = 5;
    a[i]
}

/// Negating the minimum of a signed type is the one negation that overflows.
pub fn folded_neg() -> i8 {
    let v: i8 = -128;
    -v
}

/// `MIN / -1` is the one division that overflows a signed type. rustc emits an
/// `Overflow` assert for it, the same shape as `+`, not the divide-by-zero assert.
pub fn folded_div_overflow() -> i32 {
    let a: i32 = i32::MIN;
    let b: i32 = -1;
    a / b
}

/// A constant zero divisor. rustc folds the check, not the division: `a / z` with
/// a `let`-bound zero is accepted by the compiler and only panics at run time.
pub fn folded_div() -> u8 {
    let a: u8 = 7;
    let z: u8 = 0;
    a / z
}

/// The same for the remainder operator, which has its own assert kind.
pub fn folded_rem() -> u8 {
    let a: u8 = 7;
    let z: u8 = 0;
    a % z
}

fn main() {
    println!("{}", folded_add());
    println!("{}", folded_bounds());
    println!("{}", folded_neg());
    println!("{}", folded_div_overflow());
    println!("{}", folded_div());
    println!("{}", folded_rem());
}

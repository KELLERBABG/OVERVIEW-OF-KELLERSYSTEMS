// Path-sensitivity fixture.
//
// Every assertion used to be handed to Z3 on its own, as if it were unguarded:
// arithmetic a comparison already rules out was still reported as a failure, with
// a witness the guard makes impossible (`x = 255` for `if x < 100 { x + 1 }`).
// Threading the branch guards through the control-flow graph is what makes the
// first four functions provably safe.
//
// `sum_under_guarded` is the control. If the analysis had simply stopped
// reporting overflows, it would have gone quiet too — so `keller verify` must
// still exit 1 because of it, and the other four must not appear in report.json.

/// `x < 100` keeps `x + 1` inside `u8`.
pub fn sum_guarded(x: u8) -> u8 {
    if x < 100 { x + 1 } else { 0 }
}

/// The same guard, but as an index check: `i < 4` is exactly the bound the slice
/// access asserts, so the bounds check is implied rather than additional.
pub fn slice_guarded(v: &[u8; 4], i: usize) -> u8 {
    if i < 4 { v[i] } else { 0 }
}

/// `b > 0` rules out both division by zero and the `MIN / -1` overflow.
pub fn div_guarded(a: i32, b: i32) -> i32 {
    if b > 0 { a / b } else { 0 }
}

/// Two guards that cannot both hold make the body unreachable, so the overflow
/// inside it is not a bug anyone can trigger.
pub fn dead_branch(x: u8) -> u8 {
    if x > 200 && x < 10 { x + 100 } else { 0 }
}

/// The guard is real but far too weak: `x` can still be 156..=199, where
/// `x + 100` overflows. This must stay a FAIL.
pub fn sum_under_guarded(x: u8) -> u8 {
    if x < 200 { x + 100 } else { 0 }
}

fn main() {
    let arg: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let x = arg as u8;
    let v: [u8; 4] = [1, 2, 3, 4];

    println!("{}", sum_guarded(x));
    println!("{}", slice_guarded(&v, arg));
    println!("{}", div_guarded(arg as i32, arg as i32));
    println!("{}", dead_branch(x));
    println!("{}", sum_under_guarded(x));
}

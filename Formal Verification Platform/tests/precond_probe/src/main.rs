// What does the tool report today (a) when the precondition is an entry
// assertion, and (b) when a parameterised body is only ever called with safe
// values?

/// A precondition written as an assertion at the top of the body.
pub fn entry_assert(x: u8) -> u8 {
    assert!(x < 100);
    x + 1
}

/// Private callee whose only call passes a safe constant.
fn private_callee(x: u8) -> u8 {
    x + 1
}

/// Public callee whose only call passes a safe constant.
pub fn public_callee(x: u8) -> u8 {
    x + 1
}

/// Private callee called with a value the tool cannot bound.
fn private_unknown(x: u8) -> u8 {
    x + 1
}

pub fn caller() -> u8 {
    let safe_a = private_callee(7);
    let safe_b = public_callee(9);
    let arg: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let unknown = private_unknown(arg);
    let checked = entry_assert(arg);
    safe_a ^ safe_b ^ unknown ^ checked
}

fn main() {
    println!("{}", caller());
}

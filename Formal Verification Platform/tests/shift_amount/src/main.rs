// A shift is checked on its *amount*, and on nothing else.
//
// Rust's `<<` and `>>` panic in exactly one situation: the shift amount is
// negative or at least the bit width of the left operand. A value that leaves the
// type's range is silently truncated — `200u8 << 1` is `144`, not a panic — which
// is the opposite of `+`, `-` and `*`.
//
// The verifier used to get this wrong in both directions, and this fixture pins
// the fix:
//
//   * `shift_by` shifts by a runtime amount. `by >= 32` is a real panic on a
//     `u32`, so this must be a FAIL whose witness names the *amount*.
//   * `truncate` shifts by a runtime amount that a guard has already shown to be
//     in range, so the same panic condition is unreachable and the obligation must
//     be *proved*, not reported. Before shifts were modelled this was neither:
//     the check was silently skipped.
//   * `discard_high_bits` shifts a value out of its range by a constant amount.
//     Rust truncates that, so a verifier that reported it would be inventing a
//     counterexample out of a program that cannot panic.
//
// The last one is the false-positive control: the constant-fold path used to call
// `200u8 << 1` an overflow.

/// `by >= 32` shifts by at least the width of a `u32`: a reachable panic.
pub fn shift_by(x: u32, by: u32) -> u32 {
    x << by
}

/// The guard makes the panic condition `by >= 32` impossible, so this must be
/// proved — and, unlike before, proved *about the shift* rather than skipped.
pub fn truncate(x: u32, by: u32) -> u32 {
    if by < 32 {
        x << by
    } else {
        x
    }
}

/// A constant shift amount that pushes the value out of `u8`. Rust truncates:
/// there is no panic here, so there must be no finding here.
pub fn discard_high_bits(x: u8) -> u8 {
    x << 3
}

fn main() {
    let mut args = std::env::args().skip(1);
    let x: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let by: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    println!("{}", shift_by(x, by));
    println!("{}", truncate(x, by));
    println!("{}", discard_high_bits(x as u8));
}

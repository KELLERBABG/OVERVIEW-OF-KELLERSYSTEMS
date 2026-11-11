// A target project that pins its own toolchain, which is what every real project
// does and what this checkout does not do.
//
// `cargo` is a rustup shim: with this file present it selects `stable` unless
// something selects otherwise, and stable cargo rejects the `-Z` flag the verify
// command passes before any MIR is lowered. Eight of the fifteen crates this was
// tried on failed exactly that way, with the setup-failure exit 3 and no verdict,
// on code that compiles cleanly.
//
// The verifier lowers the MIR one exact compiler produces, so it drives that
// compiler rather than the project's. This crate is the regression: it must exit
// 1 (one counterexample below), not 3, and the counterexample is what proves the
// analysis ran at all.

#[inline(never)]
fn guarded_increment(x: u8) -> u8 {
    if x < 100 { x + 1 } else { x }
}

#[inline(never)]
fn unguarded_increment(x: u8) -> u8 {
    x + 1
}

fn main() {
    println!("{}", guarded_increment(std::hint::black_box(1)));
    println!("{}", unguarded_increment(std::hint::black_box(2)));
}

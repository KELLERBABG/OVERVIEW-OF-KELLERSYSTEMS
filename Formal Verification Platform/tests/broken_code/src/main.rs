// A crate that does not compile.
//
// This fixture exists for the exit-code contract, not for the analysis. The
// wrapper runs rustc itself, and it used to discard the status rustc returned, so
// a crate the compiler rejected came out as **exit 0 with an empty
// `report.json`**: the analysis had found no assertions in a body that was never
// built, and "no findings" was indistinguishable from "nothing to check". A tool
// whose whole claim is that it never says "safe" without checking has exactly one
// wrong answer, and that was it.
//
// So this is deliberately not a program with a bug in it. It is a program the
// compiler refuses, and the only thing asserted about it is the exit code: `3`,
// "the analysis could not run", which the top-level command reaches because
// `cargo check` now fails the way it does for a normal build.

/// `u8` is not a `str`, and no amount of analysis changes that.
pub fn type_error() -> u8 {
    "not a number"
}

fn main() {
    println!("{}", type_error());
}

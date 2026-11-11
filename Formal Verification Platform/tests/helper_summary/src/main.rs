//! S7: the precondition of a small, private, cycle-free helper is derived from
//! its own body and checked at its call sites, so a body that is only ever
//! called with values it can take stops being reported `unguarded`.
//!
//! Four helpers of the same shape separate the four answers, and the fixture
//! pins all of them in one run:
//!
//! * `inc` — every caller passes a constant it can take, so its `x + 1` is
//!   checked against those callers and the body's unguarded finding goes away;
//! * `boom` — one caller passes the one value it cannot take, so there is no
//!   summary and the body keeps its `unguarded` finding: the bound is checked,
//!   never assumed;
//! * `open` — the only caller passes a value this analysis does not bound, so
//!   again nothing is assumed;
//! * `exported` — `pub`, so its callers are not all visible here and "every call
//!   site satisfies it" is not a fact this run can state.

/// Called only with `7` and `200`: `x + 1` cannot overflow either.
fn inc(x: u8) -> u8 {
    x + 1
}

/// Called with `255`, which is exactly the value `y + 1` cannot take.
fn boom(y: u8) -> u8 {
    y + 1
}

/// Called with a runtime value, so its argument is not a constant to check.
fn open(z: u8) -> u8 {
    z + 1
}

/// Exported: a caller in another crate is not part of this scan.
pub fn exported(w: u8) -> u8 {
    w + 1
}

/// Calls a local function, so it is not a leaf and not cycle-free: no summary.
fn rec(n: u8) -> u8 {
    if n == 0 {
        0
    } else {
        rec(n - 1)
    }
}

fn main() {
    let runtime: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let a = inc(7);
    let b = inc(200);
    let c = boom(255);
    let d = open(runtime);
    let e = exported(9);
    let f = rec(runtime % 3);
    println!("{}", a ^ b ^ c ^ d ^ e ^ f);
}

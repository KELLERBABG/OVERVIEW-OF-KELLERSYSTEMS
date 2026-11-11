// Regression fixture for the branch-merged-constant soundness bug.
//
// `m` is 255 on one path and 0 on the other, so `m + 100` overflows for
// `flag == true`. The old single-pass propagation visited the `0` arm last,
// recorded `m = 0`, folded the check to `100 <= 255`, and skipped the
// obligation entirely — a false PASS on a reachable overflow.
//
// `keller verify` must report FAIL (exit 1) here.

fn bump(flag: bool) -> u8 {
    let m: u8 = if flag { 255 } else { 0 };
    m + 100
}

fn main() {
    let flag = std::env::args().nth(1).is_some();
    println!("{}", bump(flag));
}

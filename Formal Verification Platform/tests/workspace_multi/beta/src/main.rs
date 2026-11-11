// Workspace member beta: a distinct counterexample, which must also survive the
// merge. Asserting both crates appear is what makes the merge a tested
// property rather than an implementation detail.

fn beta_divide(a: i32, b: i32) -> i32 {
    a / b
}

fn main() {
    let mut args = std::env::args().skip(1);
    let a: i32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    let b: i32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    println!("{}", beta_divide(a, b));
}

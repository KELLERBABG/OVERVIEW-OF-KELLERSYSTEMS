// Workspace member alpha: its own counterexample must survive the merge into
// report.json. Before the per-crate reports, a second member compiling later
// overwrote this crate's findings.

fn alpha_overflow(x: u32, y: u32) -> u32 {
    x + y
}

fn main() {
    let mut args = std::env::args().skip(1);
    let a: u32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    let b: u32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    println!("{}", alpha_overflow(a, b));
}

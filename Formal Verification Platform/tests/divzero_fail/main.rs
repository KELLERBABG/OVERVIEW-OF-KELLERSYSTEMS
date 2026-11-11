// Deliberately divisible-by-zero code: `keller verify` must FAIL and exit 1.
//
// `b` is a runtime value, so Z3 has to prove the divisor can be 0.
fn divide(a: i32, b: i32) -> i32 {
    a / b
}

fn main() {
    let mut args = std::env::args().skip(1);
    let a: i32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    let b: i32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    let q = divide(a, b);
    println!("{}", q);
}

// Deliberately overflowable code: `keller verify` must FAIL and exit 1.
//
// The operands are runtime values (not compile-time constants), so Z3 has to
// find the witness itself: `a = i32::MAX, b = 1` makes `a + b` overflow.
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let mut args = std::env::args().skip(1);
    let a: i32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    let b: i32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    let x = add(a, b);
    println!("{}", x);
}

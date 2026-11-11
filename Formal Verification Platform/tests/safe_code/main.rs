// Intentionally safe code: `keller verify` must exit 0 with zero FAIL events.
fn main() {
    // Bounds-safe slice access (constant index within a constant length).
    let v: Vec<i32> = vec![10, 20, 30];
    let i = 1usize;
    println!("v[1] = {}", v[i]);

    // No overflow: 100u8 + 50u8 = 150 <= 255.
    let a: u8 = 100;
    let b: u8 = 50;
    let sum = a + b;
    println!("a + b = {}", sum);

    // No division by zero (constant divisors).
    let q = 10 / 2;
    let r = 7 % 3;
    println!("q = {}, r = {}", q, r);
}

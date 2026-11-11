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

    // A boxed array: the null and alignment checks `rustc` emits before the
    // dereference are decided from the allocation's provenance, not left
    // unchecked, because a `Box` address is non-null and aligned by construction.
    let b: Box<[i32; 3]> = Box::new([4, 5, 6]);
    println!("b[2] = {}", b[2]);
}

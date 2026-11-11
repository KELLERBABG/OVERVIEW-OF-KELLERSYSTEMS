fn main() {
    // All safe operations - should all PASS
    let arr = [1, 2, 3];
    let idx: usize = 1;
    let _ = arr[idx];

    let x: u8 = 100;
    let y: u8 = 50;
    let _ = x + y;

    let divisor: i32 = 2;
    let _ = 42 / divisor;
}
fn check_index(idx: usize) {
    let arr = [1, 2, 3];
    let _ = arr[idx];
}

fn check_overflow(x: u8, y: u8) {
    let _ = x + y;
}

fn check_div(divisor: i32) {
    let _ = 42 / divisor;
}

fn main() {
    // These will be caught by Rust's compile-time checks, 
    // but the verifier should also detect them:
    check_index(5);
    check_overflow(250, 10);
    check_div(0);
}
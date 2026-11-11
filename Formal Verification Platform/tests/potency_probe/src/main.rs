// False-positive probe: safe Rust, written the way library code is written.
//
// Nothing here can panic. The crate exists to measure how much of that the
// analysis recognises, and `tests/run_tests.sh` asserts the answer: none of the
// ten is reported as a counterexample — the eight control functions below produce
// no finding at all, and the two loop bodies that used to are decided here or, in
// one case, reported undecided with its reason named.
//
//   * `while i + 8 <= v.len()` used to be reported as an overflow whose witness
//     was `i + 8 = 18446744073709551608`, a value the loop condition excludes.
//   * `v[i]` in the body used to be a bounds failure with `i = 8, len = 8` — the
//     exit value, not a reachable index.
//
// Both closed with one change (see `widened_state` in `src/path_state.rs`): a
// widened loop head no longer simply forgets the places its back edge carries, it
// keeps a *relation* to their previous-trip versions,
//
// ```text
// mir_3 = 0  ∨  (prev guard ∧ mir_3 = prev_bb6_i + 8)
// ```
//
// so the guard on the edge out of the head and the value its body reads are the
// same quantity — `chunk_loop_bb3.smt2` now carries `(<= (+ mir_3 8) len_1)` body
// *and* the previous-trip symbols, and re-discharges to `unsat`.
//
// The third shape lives on as an *undecided* obligation rather than a
// counterexample, which is the honest verdict: `for i in 0..8 { v[i] }` on
// `&[u8; 8]` indexes through `Range::next`, a call this analysis does not model, so
// `fixed_array` reports `Undecided ... the result of the call to
// <std::ops::Range<usize> as std::iter::Iterator>::next is not modelled` — the
// bounds check is not claimed to fail, and it is not claimed to hold either. The
// suite asserts that report, so a fix that silently decided it would show up as a
// diff too.
//
// The functions below `chunk_loop` are the control: bit masks, a shift with a
// masked amount, a narrowing cast, and the saturating/checked/wrapping arithmetic
// forms.

fn xor_mask(x: u32, mask: u32) -> u32 {
    x ^ mask
}

fn and_mask(x: u32) -> u32 {
    x & 0x0fff
}

fn or_flag(x: u32) -> u32 {
    x | 1
}

fn shift_by_amount(x: u32, s: u32) -> u32 {
    x << (s & 31)
}

fn narrowing_cast(x: u64) -> u32 {
    x as u32
}

fn saturating(x: u8, y: u8) -> u8 {
    x.saturating_add(y)
}

fn checked(x: u8, y: u8) -> u8 {
    x.checked_add(y).unwrap_or(0)
}

fn wrapping(x: u8, y: u8) -> u8 {
    x.wrapping_add(y)
}

// K9: the safe arithmetic forms, each *leaned on*. Every function below asks the
// analysis to use a fact about the idiom's result — that a saturating sum is
// never below its right operand, that a checked sum in its `Some` arm is the
// exact sum, that a wrapping sum and a narrowing cast are both defined by a
// modulus — so a modelled encoding decides the obligation and a free symbol
// cannot. Each is guarded so that it cannot actually fail: they are proofs.
//
// `checked_*` needs both halves of its `Option`, and takes both: the payload is
// the mathematical sum reduced into the operand type, and the discriminant is
// "`Some` exactly when that sum is in range". With only the payload the `None`
// arm would stay reachable carrying `(x + y) mod 2^8` — a value no path can read
// — and this very function was reported as an underflow for `s = 0, x = 1`
// (that is `y = 255`, which is the `None` arm) until the discriminant was
// modelled with it. The emitted obligation now reads
//
// ```text
// ; reachable: (= (ite (and (>= (+ mir_1 mir_2) 0) (<= (+ mir_1 mir_2) 255)) 1 0) 1)
// (assert (= overflow_l (mod (+ mir_1 mir_2) 256)))
// ```
//
// which is `s = x + y` under the guard that the sum fit, and re-discharges to
// `unsat`.

/// A saturating sum is never below its right operand, so this cannot underflow.
fn saturating_never_below(x: u8, y: u8) -> u8 {
    let s = x.saturating_add(y);
    s - y
}

/// In its `Some` arm a checked sum is exactly `x + y`, so it is at least `x`.
fn checked_sum_gives_the_sum(x: u8, y: u8) -> u8 {
    match x.checked_add(y) {
        Some(s) => s - x,
        None => 0,
    }
}

/// `x.wrapping_add(y)` is bounded by its inputs: with both under 100 the sum
/// cannot wrap, so it is at most 200 and the increment cannot overflow.
fn wrapping_sum_under_input_bound(x: u8, y: u8) -> u8 {
    if x > 100 || y > 100 {
        return 0;
    }
    let s = x.wrapping_add(y);
    s + 1
}

/// `v as u8` is `v mod 256`, so `v < 200` bounds the narrowed value below 200.
fn narrowed_under_input_bound(v: u16) -> u8 {
    if v >= 200 {
        return 0;
    }
    let b = v as u8;
    b + 55
}

fn chunk_loop(v: &[u8]) -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i + 8 <= v.len() {
        n = n.wrapping_add(v[i] as usize);
        i += 8;
    }
    n
}

fn fixed_array(v: &[u8; 8]) -> u8 {
    let mut acc = 0u8;
    for i in 0..8 {
        acc = acc.wrapping_add(v[i]);
    }
    acc
}

// S3/S4: the container and iterator models lean on *named* facts. A `0..v.len()`
// loop is decided only while the range's `end` and the bounds check's length are
// the same quantity; a `Vec`/`String` length is decided only while the model's
// fact carries the real `len` bound (no `Vec` or `String`/`str` can hold more
// than `isize::MAX` bytes, so two lengths cannot overflow a `usize` sum).
//
//   * `range_len_of_slice` indexes through the same dynamic length the loop
//     ends on — an unmodelled `Range::next` made this one undecided (its value
//     was free and the length it read a different symbol).
//   * the two length sums were *false positives*: an unmodelled `Vec::<u8>::len`
//     was a free `usize` symbol that could be exactly `usize::MAX`, and the
//     solver chose a pair that overflowed.

/// A loop whose end is the slice's own length: the yielded index is bounded by it.
fn range_len_of_slice(v: &[u8], i: usize) -> u8 {
    for j in 0..v.len() {
        if j == i {
            return v[j];
        }
    }
    0
}

/// Two `Vec` lengths cannot overflow a `usize` sum: each is at most `isize::MAX`.
fn vec_len_sum_cannot_overflow(v: &Vec<u8>, w: &Vec<u8>) -> usize {
    v.len() + w.len()
}

/// A `String` and a `str` length likewise: both are byte counts, each `<= isize::MAX`.
fn str_and_string_len_cannot_overflow(s: &String, t: &str) -> usize {
    s.len() + t.len()
}

// S5: a pointer this body made from one local reads that local. The guard and the
// bounds check are then the same quantity, so the index is proved. Without it the
// read of `*r` has no term at all, the branch guard is unknown, and the bounds
// check below is undecided rather than proved.

/// `r` borrows `len`, so `i < *r` bounds `v[i]` by the slice's own length.
fn through_local_ref(v: &[u8], i: usize) -> u8 {
    let len = v.len();
    let r = &len;
    if i < *r {
        v[i]
    } else {
        0
    }
}

fn main() {
    println!("{}", xor_mask(1, 2));
    println!("{}", and_mask(3));
    println!("{}", or_flag(4));
    println!("{}", shift_by_amount(5, 40));
    println!("{}", narrowing_cast(6));
    println!("{}", saturating(250, 10));
    println!("{}", checked(250, 10));
    println!("{}", wrapping(250, 10));
    println!("{}", chunk_loop(&[0u8; 16]));
    println!("{}", fixed_array(&[1u8; 8]));
    println!("{}", saturating_never_below(1, 2));
    println!("{}", checked_sum_gives_the_sum(3, 4));
    println!("{}", wrapping_sum_under_input_bound(5, 6));
    println!("{}", narrowed_under_input_bound(7));
    println!("{}", range_len_of_slice(&[], 0));
    println!("{}", vec_len_sum_cannot_overflow(&Vec::new(), &Vec::new()));
    println!("{}", str_and_string_len_cannot_overflow(&String::new(), ""));
    println!("{}", through_local_ref(&[1u8], 0));
}

// What a report does not say is as important as what it says.
//
// Every check this tool makes is a proof about an *assertion* the compiler
// inserted: an overflow, a division by zero, a bounds check. `unwrap`, `expect`,
// `panic!`, `unreachable!` and the failure path of a user `assert!` are not
// assertions. They are calls into another crate whose body this analysis does not
// read, and until the pass this fixture was written for, they were walked past in
// silence: they contributed nothing to `report.json`, nothing to the console and
// nothing to the coverage counts. A crate of `unwrap`s therefore produced a report
// with no findings, and "no findings" is what a reader takes to mean "no panics".
//
// What it pins, and why each half is here:
//
//   * `unwrapped`, `expected`, `explicit_program_panic`, `never_reached`,
//     `guarded_by_assert` and `asserted_equal` are the six shapes. Each must be
//     counted in
//     `coverage.panic_sites` and reported as a `WARN` whose `kind` is
//     `unmodelled-panic` and whose details name the callee, so the negative space
//     is stated rather than left to be inferred.
//   * `control_plain` and `control_bounds_check` are the other half: a function
//     with nothing that can panic must add no panic site at all, or the count is
//     noise rather than a disclosure. `control_bounds_check` is also why these are
//     counted *apart* from the assertions it does model: the bounds check on its
//     index is an obligation with a verdict, and it must stay proved.
//   * `main` is the third half, and the reason a local callee is excluded: it
//     calls every one of the five, and none of those call sites may be reported
//     again. A local function's own body is analysed, so its panic is named where
//     it happens; reporting it at the caller too would multiply the count by the
//     number of call sites, which is a number about this file rather than about
//     the program.
//
// None of it is a FAIL. Nothing was proved false, so the exit code is unchanged
// and a crate full of `unwrap`s does not fail a build; `--deny-warn` is what
// makes an undecided path fatal, which is the same contract the `undecided`
// fixture pins for a solver that cannot answer.

/// `Option::unwrap`: a call into `core` that panics when it has no value.
pub fn unwrapped(x: Option<u8>) -> u8 {
    x.unwrap()
}

/// `Result::expect`, the same shape with a message attached.
pub fn expected(x: Result<u8, u8>) -> u8 {
    x.expect("a value the caller promised")
}

/// `panic!` written by the program itself, so the site is a call rather than a
/// condition the compiler inserted.
pub fn explicit_program_panic(flag: bool) -> u8 {
    if flag {
        panic!("the caller asked for this");
    }
    7
}

/// `unreachable!`: still a call, and still reachable on the path that does not
/// take the early return.
pub fn never_reached(x: u8) -> u8 {
    if x < 3 {
        return x;
    }
    unreachable!("x is never 3 or more")
}

/// The user's own `assert!`, in its plain form: the condition is a guard for
/// everything after it, so `x + 1` must stay proved, while the panic the assertion
/// raises when the condition is false goes unmodelled.
pub fn guarded_by_assert(x: u8) -> u8 {
    assert!(x < 200);
    x + 1
}

/// `assert_eq!`, which is the one assertion macro whose failure path reaches
/// `core::panicking::assert_failed` rather than the `panic_fmt` a plain `assert!`
/// or `panic!` lowers to. It is here so the *assert* class of call site is
/// exercised by a run rather than described: the analysis assumed the two values
/// agree, and the panic when they do not is what it does not read.
pub fn asserted_equal(a: u8, b: u8) -> u8 {
    assert_eq!(a, b, "a and b must agree, got {a} and {b}");
    a
}

/// The control: no assertion, no call that can panic, nothing to disclose.
pub fn control_plain(x: u8) -> u8 {
    if x < 250 {
        x
    } else {
        0
    }
}

/// The control with an obligation: the index is bounds-checked, which is an
/// assertion this analysis does model, and the guard on it proves the check. Its
/// `PASS` and the absent panic site are two different statements, and a count that
/// conflated them would either lose the proof or invent a panic.
pub fn control_bounds_check(v: &[u8; 4], i: usize) -> u8 {
    if i < 4 {
        v[i]
    } else {
        0
    }
}

fn main() {
    println!("{}", unwrapped(Some(1)));
    println!("{}", expected(Err(2)).wrapping_add(1));
    println!("{}", explicit_program_panic(false));
    println!("{}", never_reached(1));
    println!("{}", guarded_by_assert(3));
    println!("{}", asserted_equal(5, 5));
    println!("{}", control_plain(4));
    println!("{}", control_bounds_check(&[1, 2, 3, 4], 0));
}

// A store the analysis cannot name must still retire what it recorded.
//
// `place_key` understands field projections only, so `*r = 250` — a dereference —
// names no place, and `apply_assign` used to return on that answer and leave every
// recorded value standing. `v` therefore kept the value the copy out of `s + x`
// had put there, which the guard bounds by 8, and `v + 240` was *proved* from it:
// the tool reported `ref_write` (bb2) and (bb3) as `[PASS]` for a function that
// panics. A stale fact is the one failure shape that produces a wrong proof, so
// it is what this fixture exists to catch.
//
// What it pins, and why each half is here:
//
//   * `ref_write` is that program. Its store cannot be read back, so the check
//     that depends on it may not be claimed either way: the answer must be
//     undecided *with the reason named*, never PASS. (The reason, and not a
//     counterexample, is the honest verdict — without aliasing the analysis does
//     not know the reference pointed at `v`; what it must not do is keep claiming
//     the old value.)
//   * `named_store_is_kept` is the control, and the precision half of the
//     contract: the same overflow with a store the key scheme *can* name. The
//     value is retired and re-derived there too, and the guard still bounds it, so
//     the obligation must stay proved. A fix that gave up on every place a
//     reference touches would report this one instead, which is a false positive
//     bought for nothing.

/// A store the key scheme cannot name, in front of a check that used to be
/// answered from the value it destroyed: `*r = 250` is a dereference.
pub fn ref_write(s: u8, x: u8) -> u8 {
    if s < 5 && x < 5 {
        let mut v = s + x;
        let r = &mut v;
        *r = 250;
        v + 240
    } else {
        0
    }
}

/// The control: the same overflow with a store the analysis can name, so the
/// retired value is replaced by a known one and the proof has to survive.
#[allow(unused_assignments)] // the first value is the one the store replaces
pub fn named_store_is_kept(s: u8, x: u8) -> u8 {
    if s < 5 && x < 5 {
        let mut v = s + x;
        v = s;
        v + 240
    } else {
        0
    }
}

fn main() {
    let _ = ref_write(1, 2);
    let _ = named_store_is_kept(1, 2);
}

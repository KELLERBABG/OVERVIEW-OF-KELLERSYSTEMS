// A write made by a body this analysis does not read has to retire what it wrote,
// wherever the write is *spelled*.
//
// `apply_assign` was the only thing that could retire anything, and the call
// terminator retired only its own destination — the value the callee wrote
// through a pointer argument, the value a `Drop` impl overwrote, an asm block's
// output and an intrinsic's store were all invisible. Four functions below are
// programs that panic on every input their guard admits, and every one of them was
// *proved* before this rule existed (A18). Each is here as evidence
// rather than as a hypothesis: `v + 240` was answered from the `s + x` the guard
// bounds by 8, in a function that has already replaced that value.
//
// What this case pins, and why each half is here:
//
//   * `call_writes`, `drop_writes`, `intrinsic_writes`, `asm_writes` are the four
//     shapes. The check downstream of each must be *undecided with the reason
//     named* — a `WARN` saying which write cost the value — and never `PASS`.
//   * `asm_writes` carries a second assertion: the reachability walk did not know
//     the `InlineAsm` terminator either, so the assertion *after* the asm block
//     was not checked at all. A walk that stops at a terminator it does not know
//     cannot report that it skipped anything: the summary said 100% of the
//     assertions it had "encountered" were decided, about a walk that had already
//     given up.
//   * `unrelated_local_is_kept` and `no_pointer_argument_is_kept` are the
//     controls, and the precision half of the contract. A callee's pointer names
//     exactly one local when the body's own MIR says so, and a call that is handed
//     no pointer into this body's storage cannot have written `v` at all: both
//     obligations must stay proved. A fix that retired every local any call could
//     touch would report those two instead, which is a false positive bought for
//     nothing.
//
// The verdict is `WARN`, not `FAIL`, for the same reason `tests/unnamed_write`
// and `tests/undecided` are: what the analysis declines to do is *claim* the
// counterexample. Without aliasing it cannot say the callee's pointer points at
// `v`; what it must not do is keep the value the program already replaced.

/// A callee that writes through the pointer it is given.
fn bump(v: &mut u8) {
    *v = 250;
}

/// A call given a pointer into this body's own storage.
pub fn call_writes(s: u8, x: u8) -> u8 {
    if s < 5 && x < 5 {
        let mut v = s + x;
        bump(&mut v);
        v + 240
    } else {
        0
    }
}

/// A `Drop` impl that writes through a reference stored in the value being
/// dropped — safe Rust, no `unsafe` anywhere: the drop glue is a body this
/// analysis does not read, and it holds `v`'s address by the time it runs.
pub struct Holder<'a>(&'a mut u8);

impl Drop for Holder<'_> {
    fn drop(&mut self) {
        *self.0 = 250;
    }
}

pub fn drop_writes(s: u8, x: u8) -> u8 {
    if s < 5 && x < 5 {
        let mut v = s + x;
        {
            let _holder = Holder(&mut v);
        }
        v + 240
    } else {
        0
    }
}

/// An intrinsic that writes through a pointer.
pub fn intrinsic_writes(s: u8, x: u8) -> u8 {
    if s < 5 && x < 5 {
        let mut v = s + x;
        let source = 250u8;
        unsafe {
            core::ptr::copy_nonoverlapping(&source, &mut v, 1);
        }
        v + 240
    } else {
        0
    }
}

/// An inline asm block's output — and the assertion after it, which used to be
/// outside the walk rather than undecided in it.
#[cfg(target_arch = "x86_64")]
pub fn asm_writes(s: u8, x: u8) -> u8 {
    if s < 5 && x < 5 {
        let mut v = s + x;
        unsafe {
            core::arch::asm!("mov {0}, 250", out(reg_byte) v);
        }
        v + 240
    } else {
        0
    }
}

/// The control: the callee's pointer is `&mut w`, and nothing in this body says
/// `v` was written, so the proof about `v` has to survive the call.
pub fn unrelated_local_is_kept(s: u8, x: u8) -> u8 {
    if s < 5 && x < 5 {
        let mut w = 0u8;
        let v = s + x;
        bump(&mut w);
        v + 240
    } else {
        0
    }
}

/// The second control: a call handed no pointer into this body's storage at all.
fn double(value: u8) -> u8 {
    value.wrapping_mul(2)
}

pub fn no_pointer_argument_is_kept(s: u8, x: u8) -> u8 {
    if s < 5 && x < 5 {
        let v = s + x;
        let _ = double(v);
        v + 240
    } else {
        0
    }
}

fn main() {
    let _ = call_writes(1, 2);
    let _ = drop_writes(1, 2);
    let _ = intrinsic_writes(1, 2);
    #[cfg(target_arch = "x86_64")]
    let _ = asm_writes(1, 2);
    let _ = unrelated_local_is_kept(1, 2);
    let _ = no_pointer_argument_is_kept(1, 2);
}

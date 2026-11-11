// S6: the third coroutine kind, `ResumedAfterDrop`.
//
// `ResumedAfterReturn` and `ResumedAfterPanic` are decided by the resume-entry
// invariant: the transform dispatches on the coroutine's discriminant and sends
// `1` (returned) and `2` (poisoned) to blocks that abort, which a resume cannot
// arrive in. This case exists to ask whether the third kind follows the same
// argument, and the answer from real MIR is that it does not.
//
// A coroutine with an **async drop** in scope gets a extra state variant for the
// drop path, and the transform emits `resumed after async drop` asserts into
// blocks that its state dispatch reaches through *suspension* states
// (`Suspend1`, `Suspend2`, ...). Those states are ones a resume may legitimately
// arrive in, so the resume-entry invariant says nothing about those blocks: they
// are entered when the coroutine is dropped while suspended, not when it is
// resumed. Treating this kind like its two siblings therefore reports a
// counterexample on code that cannot produce one — the reason this fixture
// exists, and the reason the kind stays an unsupported assertion rather than a
// decided one.
//
// What this case pins:
//
//   * real MIR does contain `ResumedAfterDrop`, in the async fn's own body (not
//     only in a drop shim the analysis never walks);
//   * the two kinds the invariant does cover are still decided `PASS` here;
//   * the async-drop ones are reported by name as unsupported, never as a FAIL.
//
// `async_drop` is unstable, which is why this fixture is the only one carrying a
// feature attribute; the pinned toolchain is a nightly, so it compiles.

#![feature(async_drop)]

use std::future::{AsyncDrop, Future};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Pending once, then ready: one suspension point for the state machine.
struct Wake(bool);

impl Future for Wake {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}

/// A value with an async destructor, which is what puts a drop path into the
/// coroutine's state machine.
struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {}
}

impl AsyncDrop for Guard {
    async fn drop(self: Pin<&mut Self>) {}
}

async fn holds_guard() {
    let _g = Guard;
    Wake(false).await;
}

fn main() {
    let _ = holds_guard();
}

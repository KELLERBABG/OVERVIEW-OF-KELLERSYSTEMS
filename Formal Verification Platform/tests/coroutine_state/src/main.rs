// S6: the state a coroutine is resumed in.
//
// An `async fn` lowers to a coroutine whose discriminant is the state the
// compiler's transform dispatches on. Resuming one that has already returned or
// panicked is undefined behaviour, so the transform guards the resume entry with
// `AssertKind::ResumedAfterReturn` / `ResumedAfterPanic` (and
// `ResumedAfterDrop` on the drop path). Those are `AssertKind`s like any other,
// and until this fixture existed this analysis counted them as
// `unsupported-assertion`: an assertion it had reached and refused to read.
//
// What this case pins:
//
//   * the coroutine body is analysed at all — it arrives as
//     `one_yield::{closure#0}`, and a body that is never walked cannot report
//     anything;
//   * the resume-state assertions are decided rather than counted as
//     unsupported: they appear as supported assertions and no
//     `unsupported-assertion` WARN names any of the three kinds;
//   * nothing here is a counterexample — resuming a completed coroutine takes
//     `unsafe`, so a FAIL would be a false positive about safe code.
//
// The verdict is decided under the coroutine ABI: a resume arrives with state 0
// (not started) or a suspension state, never the returned or poisoned one. That
// is the boundary the transform itself encodes, and it is named in the finding's
// own details rather than left implicit.
//
// The arithmetic is `wrapping_add` on purpose: this case is about the resume
// state, and an overflowing `x + 1` would put a genuine counterexample in the
// report and bury the thing being pinned.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// Pending once, then ready: awaiting this gives the enclosing `async fn` a real
/// suspension point, so its state machine has a resumed state as well as the
/// not-yet-started one.
struct YieldOnce(bool);

impl Future for YieldOnce {
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

/// The coroutine under test: one suspension point between two additions.
async fn one_yield(x: u8) -> u8 {
    let y = x.wrapping_add(1);
    YieldOnce(false).await;
    y.wrapping_add(1)
}

fn main() {
    // Polled exactly as many times as the state machine needs: pending at the
    // suspension point, ready after it. A poll after completion would be the one
    // thing the resume-state assertions exist to catch, and it is not something
    // safe code can reach through this future.
    let mut fut = Box::pin(one_yield(1));
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let first = fut.as_mut().poll(&mut cx);
    let second = fut.as_mut().poll(&mut cx);
    match (first, second) {
        (Poll::Pending, Poll::Ready(v)) => println!("{v}"),
        _ => println!("unexpected poll order"),
    }
}

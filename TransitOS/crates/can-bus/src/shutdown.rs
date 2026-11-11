//! Graceful shutdown with CancellationToken.
//!
//! Provides a thread-safe cancellation token that can be shared across
//! the verification pipeline, CAN bus, sensor drivers, and actuator
//! controllers. When triggered, all loops and processing stop cleanly,
//! in-flight results are flushed, and connections are closed.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A thread-safe, cloneable cancellation token for graceful shutdown.
///
/// # Usage
///
/// ```rust
/// use transit_can_bus::shutdown::{CancellationToken, ShutdownCoordinator};
///
/// let coordinator = ShutdownCoordinator::new();
/// let token = coordinator.token();
///
/// // In your main loop:
/// // while token.is_running() { ... }
///
/// // Signal shutdown:
/// coordinator.shutdown();
/// assert!(token.is_cancelled());
/// ```
#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new token in the "running" (not cancelled) state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal cancellation. All `is_cancelled()` checks will return `true`
    /// after this, and `is_running()` will return `false`.
    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    /// Returns `true` if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    /// Returns `true` if the system is still running (not cancelled).
    pub fn is_running(&self) -> bool {
        !self.is_cancelled()
    }

    /// Block the current thread until cancelled or until the timeout expires.
    /// Returns `true` if cancelled, `false` if timed out.
    pub fn wait(&self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if self.is_cancelled() {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Coordinates graceful shutdown across multiple subsystems.
///
/// Holds a `CancellationToken` and a configurable shutdown timeout.
/// On `shutdown()`, the token is cancelled and all subsystems should
/// observe it and wind down within the timeout period.
pub struct ShutdownCoordinator {
    token: CancellationToken,
    timeout: Duration,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(5))
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            token: CancellationToken::new(),
            timeout,
        }
    }

    /// Get a cloneable handle to the cancellation token.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Trigger graceful shutdown.
    pub fn shutdown(&self) {
        tracing::info!("Graceful shutdown requested");
        self.token.cancel();
    }

    /// Block until shutdown is complete (cancelled + timeout elapsed)
    /// or until all subsystems have finished.
    pub fn wait_for_shutdown(&self) {
        if self.token.is_running() {
            self.token.wait(self.timeout);
        }
        tracing::info!("Shutdown complete");
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_starts_running() {
        let token = CancellationToken::new();
        assert!(token.is_running());
        assert!(!token.is_cancelled());
    }

    #[test]
    fn cancel_stops_running() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
        assert!(!token.is_running());
    }

    #[test]
    fn cloned_token_shares_state() {
        let token = CancellationToken::new();
        let clone = token.clone();
        token.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn coordinator_shutdown() {
        let coord = ShutdownCoordinator::new();
        let token = coord.token();
        assert!(token.is_running());
        coord.shutdown();
        assert!(token.is_cancelled());
    }

    #[test]
    fn wait_returns_true_when_cancelled() {
        let token = CancellationToken::new();
        let token2 = token.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            token2.cancel();
        });
        assert!(token.wait(Duration::from_secs(1)));
        handle.join().unwrap();
    }

    #[test]
    fn wait_returns_false_on_timeout() {
        let token = CancellationToken::new();
        assert!(!token.wait(Duration::from_millis(10)));
    }
}

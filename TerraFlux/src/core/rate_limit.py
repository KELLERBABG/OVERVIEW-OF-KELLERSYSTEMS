"""Dependency-free rate limiting and account lockout.

The limiter is in-process. That is sufficient for a single-worker deployment and for
the Docker Compose setup (one API container). For a multi-worker or multi-replica
deployment the same interface should be backed by Redis so the counters are shared -
the call sites in ``src/api/main.py`` only depend on the public methods below.

All methods are thread-safe.
"""
import threading
import time
from collections import deque
from dataclasses import dataclass, field
from typing import Deque, Dict, Tuple


@dataclass
class _LockoutState:
    failures: int = 0
    locked_until: float = 0.0
    last_failure_at: float = 0.0


@dataclass
class _Window:
    hits: Deque[float] = field(default_factory=deque)


class RateLimiter:
    """Fixed-window rate limiter with per-key failure lockout."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._windows: Dict[str, _Window] = {}
        self._lockouts: Dict[str, _LockoutState] = {}

    def check(self, key: str, limit: int, window_seconds: int) -> Tuple[bool, int]:
        """Consume one slot for `key`.

        Returns ``(allowed, retry_after_seconds)``. ``retry_after_seconds`` is 0 when
        the request is allowed.
        """
        if limit <= 0:
            return True, 0
        now = time.time()
        with self._lock:
            window = self._windows.setdefault(key, _Window())
            while window.hits and (now - window.hits[0]) >= window_seconds:
                window.hits.popleft()
            if len(window.hits) >= limit:
                retry_after = int(window_seconds - (now - window.hits[0])) + 1
                return False, max(1, retry_after)
            window.hits.append(now)
            return True, 0

    def register_failure(self, key: str, max_failures: int, lock_seconds: int) -> bool:
        """Record a failed attempt. Returns True when the key just became locked."""
        now = time.time()
        with self._lock:
            state = self._lockouts.setdefault(key, _LockoutState())
            if state.locked_until > now:
                return True
            if state.last_failure_at and (now - state.last_failure_at) > lock_seconds:
                # The previous failure streak is stale - start counting again.
                state.failures = 0
            state.failures += 1
            state.last_failure_at = now
            if state.failures >= max_failures:
                state.locked_until = now + lock_seconds
                state.failures = 0
                return True
            return False

    def is_locked(self, key: str) -> Tuple[bool, int]:
        """Return ``(locked, seconds_remaining)`` for `key`."""
        now = time.time()
        with self._lock:
            state = self._lockouts.get(key)
            if not state:
                return False, 0
            if state.locked_until > now:
                return True, int(state.locked_until - now) + 1
            if state.locked_until:
                state.locked_until = 0.0
                state.failures = 0
            return False, 0

    def reset(self, key: str) -> None:
        """Clear the failure counter for `key` (successful login)."""
        with self._lock:
            self._lockouts.pop(key, None)

    def clear(self) -> None:
        """Drop all counters (used by tests)."""
        with self._lock:
            self._windows.clear()
            self._lockouts.clear()


# Dedicated limiters so that a burst of webhook traffic cannot starve the login limiter.
auth_limiter = RateLimiter()
api_limiter = RateLimiter()
webhook_limiter = RateLimiter()

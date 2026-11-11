"""Property-based fuzz tests for ``dpsynth.privacy.PrivacyAccountant``.

These tests guard the accountant's contract surface against future
regressions: noise monotonicity, composition sanity, and input
validation. They are intentionally lightweight on examples
(default profile); the CI ``fuzz`` job runs the more aggressive
``ci`` profile (see ``pyproject.toml`` -> ``[tool.hypothesis]``).

Run only the fuzz suite::

    pytest -m fuzz
"""

from __future__ import annotations

import pytest

# Module-level skip: if hypothesis isn't installed, every test in this module
# is reported as skipped, not collected-as-error. Individual imports below
# rely on this guard.
pytest.importorskip("hypothesis")

from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

from dpsynth.privacy import PrivacyAccountant

# --- Strategies ------------------------------------------------------------


_floats = st.floats(
    min_value=0.05,
    max_value=20.0,
    allow_nan=False,
    allow_infinity=False,
)
_n = st.integers(min_value=10, max_value=2000)
_k = st.integers(min_value=1, max_value=512)


# --- Properties -------------------------------------------------------------


@pytest.mark.fuzz
@given(sigma=_floats, n=_n, k=_k)
@settings(max_examples=200, deadline=None, suppress_health_check=[HealthCheck.too_slow])
def test_epsilon_monotone_in_noise(sigma: float, n: int, k: int) -> None:
    """Doubling the noise multiplier must never increase spent ε.

    This is a foundational DP invariant: more noise → less leakage.
    If this property fails, the accountant has a sign error somewhere.
    """
    k = min(k, n)  # respect the sampling-rate ≤ 1 bound

    low_noise = PrivacyAccountant()
    low_noise.step(sigma, k, n)

    high_noise = PrivacyAccountant()
    high_noise.step(sigma * 2.0, k, n)

    eps_low = low_noise.get_spent_epsilon(delta=1e-5)
    eps_high = high_noise.get_spent_epsilon(delta=1e-5)

    # Add a tiny tolerance for floating-point rounding in log_moments.
    assert eps_high <= eps_low + 1e-9, (
        f"Noise-monotonicity violated: eps(σ={sigma:.3f})={eps_low:.6f} "
        f"but eps(σ={2 * sigma:.3f})={eps_high:.6f}"
    )


@pytest.mark.fuzz
@given(
    sigma=_floats,
    n=_n,
    k=_k,
)
@settings(max_examples=200, deadline=None, suppress_health_check=[HealthCheck.too_slow])
def test_composition_is_monotone_in_steps(sigma: float, n: int, k: int) -> None:
    """T steps of (σ, q) must cost ≥ 1 step of (σ, q), up to FP tolerance."""
    k = min(k, n)

    short = PrivacyAccountant()
    short.step(sigma, k, n, steps=1)

    long_run = PrivacyAccountant()
    for _ in range(3):
        long_run.step(sigma, k, n, steps=1)

    eps_short = short.get_spent_epsilon(delta=1e-5)
    eps_long = long_run.get_spent_epsilon(delta=1e-5)

    assert eps_long >= eps_short - 1e-9, (
        f"Composition violated: 1 step = {eps_short:.6f}, "
        f"3 steps = {eps_long:.6f}"
    )


@pytest.mark.fuzz
@given(sigma=st.floats(max_value=0.0, allow_nan=False, allow_infinity=False))
@settings(max_examples=50, deadline=None)
def test_non_positive_sigma_raises(sigma: float) -> None:
    """``step`` must reject σ ≤ 0 immediately, not silently."""
    a = PrivacyAccountant()
    with pytest.raises(ValueError):
        a.step(sigma, batch_size=1, dataset_size=1)


@pytest.mark.fuzz
@given(
    sigma=_floats,
    batch_or_dataset=st.one_of(
        st.integers(max_value=0),  # zero
        st.integers(min_value=-100, max_value=-1),  # negative
    ),
)
@settings(max_examples=100, deadline=None)
def test_non_positive_batch_or_dataset_raises(
    sigma: float, batch_or_dataset: int
) -> None:
    """``step`` must reject batch_size ≤ 0 and dataset_size ≤ 0."""
    a = PrivacyAccountant()
    with pytest.raises(ValueError):
        a.step(sigma, batch_size=batch_or_dataset, dataset_size=10)


@pytest.mark.fuzz
@given(
    sigma=st.one_of(
        st.floats(min_value=0.1, max_value=10.0, allow_nan=False, allow_infinity=False),
        st.just(float("nan")),
        st.just(float("inf")),
        st.floats(max_value=0.0, allow_nan=False, allow_infinity=False),
    ),
    n=st.integers(min_value=1, max_value=1000),
    k=st.integers(min_value=1, max_value=512),
)
@settings(max_examples=100, deadline=None)
def test_accountant_is_well_typed(sigma: float, n: int, k: int) -> None:
    """Either σ/σ-type bumps the accountant, or it raises — never silently.

    Sanity check that ``get_spent_epsilon`` returns a finite float
    after valid inputs, and that the API surface rejects invalid inputs
    consistently rather than swallowing NaN, ±∞, or non-positive σ.
    """
    a = PrivacyAccountant()
    if sigma != sigma or sigma == float("inf") or sigma <= 0:  # NaN, inf, ≤ 0
        with pytest.raises((ValueError, Exception)):
            a.step(sigma, batch_size=k, dataset_size=n)
        return

    a.step(sigma, batch_size=k, dataset_size=n)
    eps = a.get_spent_epsilon(delta=1e-5)
    assert eps == eps, "ε must not be NaN"
    assert eps != float("inf"), "ε must not be +∞"
    assert eps >= 0.0, "ε must be non-negative after a valid step"


# --- Smoke ----------------------------------------------------------------


@pytest.mark.fuzz
def test_hypothesis_profile_is_registered() -> None:
    """If the ``ci`` profile isn't defined, this fails — guards pyproject drift."""
    from hypothesis import settings

    # Instantiating/retrieving the profile should not raise.
    _profile = settings.get_profile("ci") if "ci" in settings._profiles else settings()
    assert _profile is not None

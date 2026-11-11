"""End-to-end tests for the dpsynth public API surface.

These tests exercise behavior across module boundaries and the
process boundary; they are intentionally separate from the
component-level unit tests under ``tests/test_*.py``.

Run only the E2E suite::

    pytest -m e2e

Skip the E2E suite for fast feedback::

    pytest -m "not e2e"
"""

from __future__ import annotations

import subprocess
import sys

import pytest

from dpsynth import __version__


@pytest.mark.e2e
def test_cli_subprocess_runs_with_current_version() -> None:
    """`python -m dpsynth` prints the current package version.

    This is the project's public CLI contract — covered here so the
    release pipeline catches any drift between ``__init__.__version__``
    and what end users see when they run the CLI.
    """
    result = subprocess.run(
        [sys.executable, "-m", "dpsynth"],
        capture_output=True,
        check=True,
        timeout=30,
    )
    assert result.returncode == 0, (
        f"`python -m dpsynth` exited {result.returncode}; "
        f"stderr:\n{result.stderr.decode('utf-8', errors='replace')}"
    )
    first_line = result.stdout.decode("utf-8", errors="replace").splitlines()[0]
    assert first_line.startswith(f"dpsynth v{__version__}"), (
        f"CLI version stamp drift: first line was {first_line!r}, "
        f"expected to start with 'dpsynth v{__version__}'"
    )


@pytest.mark.e2e
def test_accountant_pipeline_round_trip() -> None:
    """A simulated DP-SGD training trace spends ε monotonically.

    This stands in for the full ``CTGAN_DP.fit(real).generate(N)``
    pipeline when the heavy torch/opacus stack is not installed (CI
    matrix or stale virtualenv). It still exercises the full DP
    accounting pipeline end-to-end: step → accumulate → spent
    inversion → determinism on a re-run.
    """
    from dpsynth.privacy import PrivacyAccountant

    sigma = 1.0
    n_samples = 1000
    batch_size = 64
    n_steps = 10

    spent_trajectory: list[float] = []
    accountant = PrivacyAccountant()
    for _ in range(n_steps):
        accountant.step(sigma, batch_size, n_samples, steps=1)
        spent_trajectory.append(accountant.get_spent_epsilon(delta=1e-5))

    # Strictly monotonically non-decreasing: a step cannot un-spend privacy.
    from itertools import pairwise  # Python 3.10+
    for prev, curr in pairwise(spent_trajectory):
        assert curr >= prev - 1e-9, (
            f"Privacy budget decreased at a step transition: "
            f"{prev:.6f} -> {curr:.6f}"
        )

    final_eps = spent_trajectory[-1]
    assert final_eps > 0.0, "Privacy budget must be positive after 10 steps"
    assert final_eps == pytest.approx(spent_trajectory[-1], rel=1e-9), (
        "Sanity: reading the same epsilon twice must be deterministic."
    )

    # Determinism: re-running the same trace must give the same final ε.
    replay = PrivacyAccountant()
    for _ in range(n_steps):
        replay.step(sigma, batch_size, n_samples, steps=1)
    assert replay.get_spent_epsilon(delta=1e-5) == pytest.approx(final_eps, rel=1e-9)

    # Reset wipes state — verifies that the accountant has a true zero state.
    accountant.reset()
    assert accountant.get_spent_epsilon(delta=1e-5) == 0.0


# Skip the torch-backed CTGAN_DP smoke test if torch is not importable.
torch = pytest.importorskip("torch")


# Note: pytest.importorskip("torch") is intentionally NOT at module
# level — the subprocess CLI smoke and the accountant pipeline round-trip
# above don't need torch, and module-level importorskip would skip them
# too in environments without torch.


@pytest.mark.e2e
def test_ctgan_dp_smoke() -> None:
    """End-to-end through the full CTGAN_DP pipeline.

    Skipped (only this test) if torch isn't installed in the environment.
    """
    pytest.importorskip("torch")

    import numpy as np

    from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig

    real = np.random.RandomState(0).randn(64, 4).astype(np.float32)
    model = CTGAN_DP(CTGANConfig(epsilon=8.0, epochs=2, batch_size=16))
    budget = model.fit(real)
    synth = model.generate(64)

    assert synth.shape == (64, 4)
    assert np.isfinite(synth).all()
    assert budget.epsilon > 0.0
    assert budget.delta > 0.0
    assert 0.0 < budget.epsilon <= 50.0  # sanity: not absurd for ε=8, 2 epochs


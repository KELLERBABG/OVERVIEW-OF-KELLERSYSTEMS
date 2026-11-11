"""Shared fixtures for the Herzschrittmacher test suite."""

from __future__ import annotations

import pytest

from herzschrittmacher.modes import DEFAULT_TIMING, CardiacState, PacingMode


@pytest.fixture
def cycles():
    return DEFAULT_TIMING


@pytest.fixture
def fresh_state():
    return CardiacState()


@pytest.fixture
def ddd():
    return PacingMode.DDD
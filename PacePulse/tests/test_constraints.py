"""Tests for kinetic path constraint generation."""

from __future__ import annotations

from herzschrittmacher.constraints import ConstraintGenerator, PathConstraint
from herzschrittmacher.modes import DEFAULT_TIMING, CardiacState, Chamber, PacingMode


def _gen(mode: PacingMode, state: CardiacState, t: int) -> ConstraintGenerator:
    return ConstraintGenerator(mode, DEFAULT_TIMING, state, t)


def test_trivial_path():
    p = PathConstraint(())
    assert p.is_trivial
    assert str(p) == "⊤"


def test_path_string_joins_with_conjunction():
    p = PathConstraint(("a=x", "b=y"))
    assert str(p) == "a=x ∧ b=y"


def test_pulse_path_ventricle_ddd():
    state = CardiacState()
    gen = _gen(PacingMode.DDD, state, 1000)
    path = gen.pulse_path(Chamber.VENTRICLE)

    text = str(path)
    assert "mode=DDD" in text
    assert "can_pace(ventricle)=true" in text
    assert "refractory(ventricle)=false" in text
    assert "elapsed_ms=∞" in text


def test_pulse_path_records_elapsed_after_sense():
    state = CardiacState()
    state.last_ventricular_sense_ms = 100
    gen = _gen(PacingMode.VVI, state, 1000)
    path = gen.pulse_path(Chamber.VENTRICLE)

    text = str(path)
    assert "elapsed_ms=900" in text
    assert "elapsed>=escape_interval=false" in text  # 900 < 1000


def test_mode_cannot_pace_path():
    gen = _gen(PacingMode.AAI, CardiacState(), 0)
    path = gen.pulse_path(Chamber.VENTRICLE)
    text = str(path)
    assert "can_pace(ventricle)=false" in text


def test_sense_path():
    gen = _gen(PacingMode.DDD, CardiacState(), 1234)
    path = gen.sense_path(Chamber.ATRIUM)
    text = str(path)
    assert "sense(atrium)=true" in text
    assert "time_ms=1234" in text


def test_mode_switch_path():
    gen = _gen(PacingMode.DDD, CardiacState(), 0)
    path = gen.mode_switch_path(PacingMode.VVI, "electromagnetic_interference")
    text = str(path)
    assert "mode_switch=DDD->VVI" in text
    assert "reason=electromagnetic_interference" in text


def test_coalesce_dedupes():
    a = PathConstraint(("x=1", "y=2"))
    b = PathConstraint(("y=2", "z=3"))
    gen = _gen(PacingMode.DDD, CardiacState(), 0)
    merged = gen.coalesce(a, b)
    assert merged.components == ("x=1", "y=2", "z=3")
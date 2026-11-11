"""Tests for the Z3 verification engine and the trustless execution gate."""

from __future__ import annotations

import pytest

from herzschrittmacher.constraints import ConstraintGenerator, PathConstraint
from herzschrittmacher.execution_gate import ExecutionGate, GateResult
from herzschrittmacher.modes import DEFAULT_TIMING, CardiacState, Chamber, PacingMode
from herzschrittmacher.pacing import PacingDecision, PacingSystem
from herzschrittmacher.verification import (
    PathEncoder,
    VerificationEngine,
    VerificationResult,
)


def _safe_pulse_path():
    """A pulse path for a clean escape-interval ventricular pace."""
    gen = ConstraintGenerator(
        PacingMode.DDD, DEFAULT_TIMING, CardiacState(), time_ms=1500
    )
    return gen.pulse_path(Chamber.VENTRICLE)


class TestPathEncoder:
    def test_no_path_components_encodes_empty(self):
        encoder = PathEncoder()
        assert encoder.encode(PathConstraint(())) == []

    def test_refractory_component(self):
        encoder = PathEncoder()
        assertions = encoder.encode(PathConstraint(("refractory(ventricle)=false",)))
        assert len(assertions) == 1
        assert assertions[0].num_args() == 2

    def test_elapsed_infinity(self):
        encoder = PathEncoder()
        assertions = encoder.encode(PathConstraint(("elapsed_ms=∞",)))
        assert len(assertions) == 1


class TestVerificationEngine:
    def test_safe_pulse_releases(self):
        engine = VerificationEngine()
        result = engine.check_decision(
            _safe_pulse_path(), Chamber.VENTRICLE, 3.5, 0.5
        )
        assert isinstance(result, VerificationResult)
        assert result.safe
        assert result.all_invariants_hold
        assert result.blocked_invariants == ()
        assert all(v == "unsat" for v in result.checks.values())

    def test_refractory_pulse_blocked(self):
        engine = VerificationEngine()
        state = CardiacState()
        state.last_ventricular_pace_ms = 500
        gen = ConstraintGenerator(PacingMode.DDD, DEFAULT_TIMING, state, time_ms=600)
        path = gen.pulse_path(Chamber.VENTRICLE)
        assert "refractory(ventricle)=true" in str(path)

        result = engine.check_decision(path, Chamber.VENTRICLE, 3.5, 0.5)
        assert not result.safe
        assert "no_pulse_in_refractory" in result.blocked_invariants
        assert result.checks["no_pulse_in_refractory"] == "sat"

    def test_rate_violation_blocked(self):
        engine = VerificationEngine()
        state = CardiacState()
        state.last_ventricular_pace_ms = 4500
        gen = ConstraintGenerator(PacingMode.VVI, DEFAULT_TIMING, state, time_ms=4900)
        path = gen.pulse_path(Chamber.VENTRICLE)

        result = engine.check_decision(path, Chamber.VENTRICLE, 3.5, 0.5)
        assert not result.safe
        assert "rate_within_envelope" in result.blocked_invariants

    def test_high_energy_blocked(self):
        engine = VerificationEngine()
        result = engine.check_decision(
            _safe_pulse_path(), Chamber.VENTRICLE, 25.0, 3.0
        )
        assert not result.safe
        assert "pulse_energy_within_envelope" in result.blocked_invariants

    def test_symbolic_energy_envelope_violation_found(self):
        engine = VerificationEngine(max_energy_uj=0.05)
        result = engine.verify_symbolic_energy_envelope(
            Chamber.VENTRICLE,
            amplitude_range=(0.5, 8.0),
            width_range=(0.1, 2.0),
        )
        assert result["envelope_violation"] is True

    def test_symbolic_energy_envelope_clean(self):
        engine = VerificationEngine(max_energy_uj=1000.0)
        result = engine.verify_symbolic_energy_envelope(
            Chamber.VENTRICLE,
            amplitude_range=(0.5, 8.0),
            width_range=(0.1, 2.0),
        )
        assert result["envelope_violation"] is False

    def test_runtime_measured(self):
        engine = VerificationEngine()
        result = engine.check_decision(
            _safe_pulse_path(), Chamber.VENTRICLE, 3.5, 0.5
        )
        assert result.runtime_ms >= 0.0


class TestExecutionGate:
    def test_gate_releases_safe_pace(self):
        pace = PacingSystem(mode=PacingMode.DDD)
        pace.start()
        decision = pace.check_escape(Chamber.VENTRICLE, 1500)
        assert decision.action == "pace"

        gate = ExecutionGate(VerificationEngine())
        result = gate.release(decision, Chamber.VENTRICLE, 3.5, 0.5)
        assert result.released
        assert isinstance(result, GateResult)
        assert result.verification is not None
        assert result.verification.safe

    def test_gate_denies_blocked_decision(self):
        pace = PacingSystem(mode=PacingMode.DDD)
        pace.start()
        pace.sense(Chamber.VENTRICLE, 500)
        decision = pace.request_pace(Chamber.VENTRICLE, 600, 3.5, 0.5)
        assert decision.action == "blocked"

        gate = ExecutionGate(VerificationEngine())
        result = gate.release(decision, Chamber.VENTRICLE, 3.5, 0.5)
        assert not result.released
        assert "did not authorize" in result.reason

    def test_gate_denies_without_path(self):
        decision = PacingDecision(
            action="pace", chamber=Chamber.VENTRICLE, time_ms=1000, reason="test"
        )
        gate = ExecutionGate(VerificationEngine())
        result = gate.release(decision, Chamber.VENTRICLE, 3.5, 0.5)
        assert not result.released
        assert "missing kinetic path" in result.reason

    def test_gate_history_recorded(self):
        pace = PacingSystem(mode=PacingMode.DDD)
        pace.start()
        decision = pace.check_escape(Chamber.VENTRICLE, 1500)

        gate = ExecutionGate(VerificationEngine())
        gate.release(decision, Chamber.VENTRICLE, 3.5, 0.5)
        gate.release(decision, Chamber.VENTRICLE, 25.0, 3.0)
        assert len(gate.history) == 2
        assert gate.history[0].released
        assert not gate.history[1].released

    def test_gate_result_as_dict(self):
        pace = PacingSystem(mode=PacingMode.DDD)
        pace.start()
        decision = pace.check_escape(Chamber.VENTRICLE, 1500)
        gate = ExecutionGate(VerificationEngine())
        result = gate.release(decision, Chamber.VENTRICLE, 3.5, 0.5)
        d = result.as_dict()
        assert d["released"] is True
        assert d["verification"] is not None
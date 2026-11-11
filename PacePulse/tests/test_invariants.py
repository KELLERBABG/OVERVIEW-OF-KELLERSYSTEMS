"""Tests for the lethal invariant enforcement layer."""

from __future__ import annotations

from herzschrittmacher.invariants import (
    DEFAULT_INVARIANTS,
    EnergyWithinEnvelope,
    InvariantVerdict,
    NoPulseInBlanking,
    NoPulseInRefractory,
    NoSensedPulseInBlanking,
    RateWithinEnvelope,
    check_all_invariants,
)
from herzschrittmacher.modes import CardiacState, Chamber
from herzschrittmacher.pulse_analysis import SymbolicPulse


class TestNoPulseInRefractory:
    def test_holds_when_not_refractory(self, cycles):
        inv = NoPulseInRefractory()
        state = CardiacState()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        verdict = inv.check(state, pulse, cycles)
        assert verdict.holds

    def test_blocked_in_refractory(self, cycles):
        inv = NoPulseInRefractory()
        state = CardiacState()
        state.apply_ventricular_pace(500, cycles)
        pulse = SymbolicPulse(Chamber.VENTRICLE, 600, 3.5, 0.5)
        verdict = inv.check(state, pulse, cycles)
        assert not verdict.holds
        assert "refractory" in verdict.detail


class TestNoPulseInBlanking:
    def test_holds_outside_blanking(self, cycles):
        inv = NoPulseInBlanking()
        state = CardiacState()
        state.apply_ventricular_pace(500, cycles)
        pulse = SymbolicPulse(Chamber.VENTRICLE, 600, 3.5, 0.5)
        verdict = inv.check(state, pulse, cycles)
        assert verdict.holds

    def test_blocked_inside_blanking(self, cycles):
        inv = NoPulseInBlanking()
        state = CardiacState()
        state.apply_ventricular_pace(500, cycles)
        pulse = SymbolicPulse(
            Chamber.VENTRICLE, 500 + 10, 3.5, 0.5
        )  # inside 40ms blanking
        verdict = inv.check(state, pulse, cycles)
        assert not verdict.holds
        assert "blanking" in verdict.detail


class TestRateWithinEnvelope:
    def test_holds_when_no_prior_event(self, cycles):
        inv = RateWithinEnvelope()
        state = CardiacState()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        assert inv.check(state, pulse, cycles).holds

    def test_blocked_when_too_fast(self, cycles):
        inv = RateWithinEnvelope()
        state = CardiacState()
        state.apply_ventricular_pace(9000, cycles)
        pulse = SymbolicPulse(Chamber.VENTRICLE, 9500, 3.5, 0.5)  # 500ms interval
        verdict = inv.check(state, pulse, cycles)
        assert not verdict.holds
        assert "faster than" in verdict.detail

    def test_holds_when_at_floor(self, cycles):
        inv = RateWithinEnvelope()
        state = CardiacState()
        state.apply_ventricular_pace(1000, cycles)
        pulse = SymbolicPulse(Chamber.VENTRICLE, 2000, 3.5, 0.5)
        assert inv.check(state, pulse, cycles).holds


class TestEnergyWithinEnvelope:
    def test_nominal_pulse_holds(self, cycles):
        inv = EnergyWithinEnvelope()
        state = CardiacState()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        verdict = inv.check(state, pulse, cycles)
        assert verdict.holds
        assert "energy" in verdict.detail

    def test_high_amplitude_blocked(self, cycles):
        inv = EnergyWithinEnvelope()
        state = CardiacState()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 9.0, 0.5)
        verdict = inv.check(state, pulse, cycles)
        assert not verdict.holds
        assert "amplitude" in verdict.detail

    def test_wide_pulse_blocked(self, cycles):
        inv = EnergyWithinEnvelope()
        state = CardiacState()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 2.5)
        verdict = inv.check(state, pulse, cycles)
        assert not verdict.holds

    def test_high_energy_blocked(self, cycles):
        inv = EnergyWithinEnvelope()
        state = CardiacState()
        # 22mA^2 * 1ms / 15µF = 32.3µJ — exceeds the 30µJ envelope so the
        # energy reason is surface too (alongside the amplitude bound).
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 22.0, 1.0)
        verdict = inv.check(state, pulse, cycles)
        assert not verdict.holds
        assert "energy" in verdict.detail


class TestNoSensedPulseInBlanking:
    def test_holds_when_sense_outside_blanking(self, cycles):
        inv = NoSensedPulseInBlanking()
        state = CardiacState()
        state.apply_atrial_event(800, cycles)  # sense at 800
        pulse = SymbolicPulse(Chamber.ATRIUM, 1200, 3.5, 0.5)
        assert inv.check(state, pulse, cycles).holds

    def test_holds_for_own_sense_blanking(self, cycles):
        """A sense is always inside its own post-sense blanking window;
        the engine is *expected* to blank that amplifier — benign."""
        inv = NoSensedPulseInBlanking()
        state = CardiacState()
        state.apply_ventricular_event(1000, cycles)  # intrinsic V-sense
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1500, 3.5, 0.5)
        assert inv.check(state, pulse, cycles).holds

    def test_blocked_when_sense_in_others_blanking(self, cycles):
        """A second sense processed inside a blanking window started by a
        *different* intrinsic event is the oversensing pre-condition."""
        inv = NoSensedPulseInBlanking()
        state = CardiacState()
        state.apply_ventricular_event(1000, cycles)  # intrinsic V-sense
        # Buggy engine processed a second sense inside that window.
        state.last_ventricular_sense_ms = 1010
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1500, 3.5, 0.5)
        verdict = inv.check(state, pulse, cycles)
        assert not verdict.holds
        assert "blanking" in verdict.detail


class TestCheckAllInvariants:
    def test_default_invariants_all_hold_for_nominal(self, cycles):
        state = CardiacState()
        state.apply_atrial_event(200, cycles)
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        verdicts = check_all_invariants(state, pulse, cycles)
        assert len(verdicts) == len(DEFAULT_INVARIANTS)
        assert all(v.holds for v in verdicts)

    def test_returns_names(self, cycles):
        state = CardiacState()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        verdicts = check_all_invariants(state, pulse, cycles)
        names = [v.name for v in verdicts]
        assert "no_pulse_in_refractory" in names
        assert "rate_within_envelope" in names
        assert "pulse_energy_within_envelope" in names
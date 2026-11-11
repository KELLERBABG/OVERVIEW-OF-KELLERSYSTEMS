"""Tests for the formal cardiac model and NBG pacing modes."""

from __future__ import annotations

import pytest

from herzschrittmacher.modes import (
    CHAMBER_FROM_STRING,
    CardiacState,
    Chamber,
    PacingMode,
    Polarity,
    Response,
    TimingCycles,
)


class TestChamberLookup:
    def test_atrium_synonyms(self):
        assert CHAMBER_FROM_STRING["atrium"] is Chamber.ATRIUM
        assert CHAMBER_FROM_STRING["atrial"] is Chamber.ATRIUM
        assert CHAMBER_FROM_STRING["a"] is Chamber.ATRIUM

    def test_ventricle_synonyms(self):
        assert CHAMBER_FROM_STRING["ventricle"] is Chamber.VENTRICLE
        assert CHAMBER_FROM_STRING["ventricular"] is Chamber.VENTRICLE
        assert CHAMBER_FROM_STRING["v"] is Chamber.VENTRICLE


class TestPacingMode:
    def test_ddd_paces_and_senses_dual(self):
        assert PacingMode.DDD.paced is Polarity.DUAL
        assert PacingMode.DDD.sensed is Polarity.DUAL
        assert PacingMode.DDD.response is Response.DUAL

    def test_vvi_ventricular_only(self):
        assert PacingMode.VVI.paced is Polarity.VENTRICLE
        assert PacingMode.VVI.can_pace(Chamber.VENTRICLE)
        assert not PacingMode.VVI.can_pace(Chamber.ATRIUM)
        assert PacingMode.VVI.response is Response.INHIBITED

    def test_dddr_is_rate_adaptive(self):
        assert PacingMode.DDDR.rate_adaptive
        assert not PacingMode.DDD.rate_adaptive

    def test_off_paces_nothing(self):
        assert not PacingMode.OFF.can_pace(Chamber.VENTRICLE)
        assert not PacingMode.OFF.can_pace(Chamber.ATRIUM)
        assert not PacingMode.OFF.can_sense(Chamber.VENTRICLE)

    def test_vdd_senses_atrium_paces_ventricle(self):
        assert PacingMode.VDD.can_sense(Chamber.ATRIUM)
        assert PacingMode.VDD.can_sense(Chamber.VENTRICLE)
        assert PacingMode.VDD.can_pace(Chamber.VENTRICLE)
        assert not PacingMode.VDD.can_pace(Chamber.ATRIUM)


class TestTimingCycles:
    def test_default_cycles(self):
        c = TimingCycles()
        assert c.av_delay == 180
        assert c.pvarp == 350
        assert c.lower_rate_interval == 1000

    def test_av_delay_floor(self):
        with pytest.raises(ValueError):
            TimingCycles(av_delay=40)

    def test_pvarp_floor(self):
        with pytest.raises(ValueError):
            TimingCycles(pvarp=100)

    def test_upper_must_be_shorter_than_lower(self):
        with pytest.raises(ValueError):
            TimingCycles(lower_rate_interval=300, upper_rate_interval=500)

    def test_escape_floor(self):
        with pytest.raises(ValueError):
            TimingCycles(lower_rate_interval=100)


class TestCardiacState:
    def test_fresh_state_not_refractory(self, cycles):
        state = CardiacState()
        assert not state.is_refractory(Chamber.VENTRICLE, 100, cycles)
        assert not state.is_refractory(Chamber.ATRIUM, 100, cycles)

    def test_ventricular_event_starts_pvarp(self, cycles):
        state = CardiacState()
        state.apply_ventricular_event(200, cycles)
        assert state.pvarp_until_ms == 200 + cycles.pvarp
        # Atrial sensing within PVARP window is refractory.
        assert state.is_refractory(Chamber.ATRIUM, 300, cycles)
        # After PVARP ends, atrium is not refractory from PVARP.
        assert not state.is_refractory(Chamber.ATRIUM, 200 + cycles.pvarp + 1, cycles)

    def test_ventricular_refractory(self, cycles):
        state = CardiacState()
        state.apply_ventricular_pace(100, cycles)
        assert state.is_refractory(Chamber.VENTRICLE, 100 + 100, cycles)
        assert not state.is_refractory(
            Chamber.VENTRICLE, 100 + cycles.ventricular_refractory + 1, cycles
        )

    def test_blanking(self, cycles):
        state = CardiacState()
        state.apply_ventricular_pace(100, cycles)
        assert state.in_blanking(Chamber.VENTRICLE, 100, cycles)
        assert not state.can_sense(Chamber.VENTRICLE, 100, cycles)
        assert state.can_sense(
            Chamber.VENTRICLE, 100 + cycles.post_ventricular_blanking + 1, cycles
        )

    def test_atrial_blanking_independent(self, cycles):
        state = CardiacState()
        state.apply_atrial_pace(100, cycles)
        assert state.in_blanking(Chamber.ATRIUM, 100, cycles)
        assert not state.in_blanking(Chamber.VENTRICLE, 100, cycles)
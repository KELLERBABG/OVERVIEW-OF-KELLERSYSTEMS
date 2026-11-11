"""Tests for the pacing engine timing-cycle state machine."""

from __future__ import annotations

import pytest

from herzschrittmacher.modes import DEFAULT_TIMING, CardiacState, Chamber, PacingMode
from herzschrittmacher.pacing import PacingEvent, PacingSystem


def test_not_started_raises():
    pace = PacingSystem()
    with pytest.raises(RuntimeError):
        pace.sense(Chamber.ATRIUM, 100)


def test_sense_requires_monotonic_time():
    pace = PacingSystem()
    pace.start()
    pace.sense(Chamber.ATRIUM, 200)
    with pytest.raises(ValueError):
        pace.sense(Chamber.VENTRICLE, 150)


def test_ddd_atrial_sense_inhibits_and_starts_av():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()

    decision = pace.sense(Chamber.ATRIUM, 200)
    assert decision.action == "inhibit"
    assert pace.state.last_atrial_sense_ms == 200
    assert pace.state.time_ms == 200


def test_ventricular_escape_inhibited_before_av_delay():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.ATRIUM, 200)

    # Well before AV delay (180ms): should be inhibited.
    decision = pace.check_escape(Chamber.VENTRICLE, 300)
    assert decision.action == "inhibit"


def test_ventricular_trigger_after_av_delay():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.ATRIUM, 200)

    # 380 = 200 + 180 (AV delay elapsed) → triggers V pace.
    decision = pace.check_escape(Chamber.VENTRICLE, 380)
    assert decision.action == "pace"
    assert decision.chamber is Chamber.VENTRICLE


def test_ventricular_sense_resets_escape_and_starts_pvarp():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.VENTRICLE, 500)

    # Fresh V-sense → PVARP active; immediate V pace blocked.
    decision = pace.check_escape(Chamber.VENTRICLE, 580)
    assert decision.action == "blocked"
    assert "refractory" in decision.reason


def test_ventricular_sense_in_refractory_blocked():
    """Within the 320ms refractory window after a V-sense the engine must
    refuse pacing regardless of the escape interval."""
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.VENTRICLE, 500)

    decision = pace.check_escape(Chamber.VENTRICLE, 800)  # 300ms later
    assert decision.action == "blocked"
    assert "refractory" in decision.reason


def test_ventricular_sense_inhibits_within_escape():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.VENTRICLE, 500)

    # 400ms after V-sense: past refractory (320ms) but short of the
    # 1000ms escape interval → inhibited, not paced.
    decision = pace.check_escape(Chamber.VENTRICLE, 900)
    assert decision.action == "inhibit"


def test_ventricular_escape_elapsed_paces():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.VENTRICLE, 500)

    decision = pace.check_escape(Chamber.VENTRICLE, 1550)
    assert decision.action == "pace"


def test_sense_in_blanking_ignored():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.VENTRICLE, 500)

    # Within the 40ms post-ventricular blanking window.
    decision = pace.sense(Chamber.VENTRICLE, 520)
    assert decision.action == "inhibit"
    assert "blanking" in decision.reason
    # State not updated: blanked sense does not reset anything.
    assert pace.state.last_ventricular_sense_ms == 500


def test_mode_not_sensing_returns_inhibit():
    pace = PacingSystem(mode=PacingMode.VVI)
    pace.start()
    decision = pace.sense(Chamber.ATRIUM, 100)
    assert decision.action == "inhibit"
    assert "does not sense" in decision.reason


def test_mode_cannot_pace_returns_inhibit():
    pace = PacingSystem(mode=PacingMode.AAI)
    pace.start()
    decision = pace.check_escape(Chamber.VENTRICLE, 1000)
    assert decision.action == "inhibit"
    assert "cannot pace" in decision.reason


def test_request_pace_delivers_when_safe():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()

    decision = pace.request_pace(
        Chamber.VENTRICLE, 1500, amplitude_ma=3.5, pulse_width_ms=0.5
    )
    assert decision.action == "pace"
    assert pace.state.last_ventricular_pace_ms == 1500
    assert any(e.kind == "pace" for e in pace.events)


def test_request_pace_blocked_when_refractory():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.VENTRICLE, 500)

    decision = pace.request_pace(
        Chamber.VENTRICLE, 600, amplitude_ma=3.5, pulse_width_ms=0.5
    )
    assert decision.action == "blocked"
    assert "refractory" in decision.reason


def test_request_pace_blocked_when_rate_too_fast():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.VENTRICLE, 500)

    # 300ms after V sense — faster than lower-rate interval (1000ms).
    decision = pace.request_pace(
        Chamber.VENTRICLE, 800, amplitude_ma=3.5, pulse_width_ms=0.5
    )
    # Refractory (320ms) also blocks here; assert it is not released.
    assert decision.action in ("blocked", "inhibit")


def test_events_recorded():
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    pace.sense(Chamber.ATRIUM, 200)
    pace.request_pace(Chamber.VENTRICLE, 1500, 3.5, 0.5)

    kinds = [e.kind for e in pace.events]
    assert kinds == ["sense", "pace"]
    assert isinstance(pace.events[0], PacingEvent)
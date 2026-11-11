"""Tests for symbolic pulse energy/charge analysis."""

from __future__ import annotations

import pytest

from herzschrittmacher.modes import Chamber
from herzschrittmacher.pulse_analysis import PulseAnalyzer, SymbolicPulse


class TestSymbolicPulse:
    def test_charge(self):
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        assert pulse.charge_uc == pytest.approx(1.75)

    def test_conversions(self):
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        assert pulse.current_a == pytest.approx(0.0035)
        assert pulse.width_s == pytest.approx(0.0005)


class TestPulseAnalyzer:
    def test_nominal_pulse_clean(self):
        analyzer = PulseAnalyzer()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        result = analyzer.analyze(pulse)
        assert not result.over_limit
        assert result.charge_uc == pytest.approx(1.75)
        # E = 3.5^2 * 0.5 / 15 = 0.4083 µJ
        assert result.energy_uj == pytest.approx(3.5**2 * 0.5 / 15.0)

    def test_negative_amplitude_rejected(self):
        analyzer = PulseAnalyzer()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, -1.0, 0.5)
        result = analyzer.analyze(pulse)
        assert result.over_limit
        assert "amplitude" in result.reasons[0]

    def test_excessive_width_rejected(self):
        analyzer = PulseAnalyzer()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 3.0)
        result = analyzer.analyze(pulse)
        assert result.over_limit
        assert any("pulse_width" in r for r in result.reasons)

    def test_excessive_current_rejected(self):
        analyzer = PulseAnalyzer()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 9.0, 0.5)
        result = analyzer.analyze(pulse)
        assert result.over_limit
        assert any("amplitude" in r for r in result.reasons)

    def test_excessive_charge_rejected(self):
        analyzer = PulseAnalyzer(max_charge_uc=1.0)
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        result = analyzer.analyze(pulse)
        assert result.over_limit
        assert any("charge" in r for r in result.reasons)

    def test_excessive_energy_rejected(self):
        analyzer = PulseAnalyzer(max_energy_uj=0.2)
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        result = analyzer.analyze(pulse)
        assert result.over_limit
        assert any("energy" in r for r in result.reasons)

    def test_as_dict(self):
        analyzer = PulseAnalyzer()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        d = analyzer.analyze(pulse).as_dict()
        assert d["chamber"] == "ventricle"
        assert d["time_ms"] == 1000
        assert d["over_limit"] is False

    def test_ms_by_ms_ledger(self):
        analyzer = PulseAnalyzer()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
        rows = analyzer.analyze_ms_by_ms(pulse)
        assert len(rows) == 1
        assert rows[0]["ms"] == 0
        assert rows[0]["over_limit"] is False

    def test_ms_by_ms_multiple_rows(self):
        analyzer = PulseAnalyzer()
        pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 1.8)
        rows = analyzer.analyze_ms_by_ms(pulse)
        assert len(rows) == 2
        assert rows[0]["ms"] == 0
        assert rows[1]["ms"] == 1

    def test_zero_capacitance_rejected(self):
        with pytest.raises(ValueError):
            PulseAnalyzer(output_capacitance_uf=0)
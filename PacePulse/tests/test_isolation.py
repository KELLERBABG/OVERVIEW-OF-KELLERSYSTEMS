"""Tests for the temporal isolation asymmetry scheduling model."""

from __future__ import annotations

import pytest

from herzschrittmacher.isolation import IsolationModel, TaskClass


class TestIsolationModel:
    def test_default_model_feasible(self):
        model = IsolationModel()
        result = model.schedulability_check()
        assert result.feasible
        assert result.total_utilization < 1.0

    def test_critical_utilization(self):
        model = IsolationModel()
        u = model.critical_utilization()
        expected = (model.pulse_window_us + model.switching_cost_us) / model.critical_period_us
        expected += model.verification_budget_us / model.critical_period_us
        assert u == pytest.approx(expected)

    def test_monitoring_utilization(self):
        model = IsolationModel()
        assert model.monitoring_utilization() == pytest.approx(0.05)

    def test_can_delay_pulse_asymmetry(self):
        model = IsolationModel()
        assert not model.can_delay_pulse(TaskClass.CRITICAL)
        assert model.can_delay_pulse(TaskClass.VERIFICATION)
        assert not model.can_delay_pulse(TaskClass.MONITORING)

    def test_verification_over_budget_infeasible(self):
        model = IsolationModel(pulse_window_us=250, verification_budget_us=300)
        result = model.schedulability_check()
        assert not result.feasible
        assert "exceeds pulse window" in result.reason

    def test_critical_window_cannot_exceed_period(self):
        with pytest.raises(ValueError):
            IsolationModel(pulse_window_us=1200, critical_period_us=1000)

    def test_worst_case_critical_plus_verification_infeasible(self):
        model = IsolationModel(
            pulse_window_us=600, critical_period_us=1000, verification_budget_us=450
        )
        result = model.schedulability_check()
        assert not result.feasible
        assert "utilization" in result.reason

    def test_per_class_utilization_recorded(self):
        model = IsolationModel()
        result = model.schedulability_check()
        assert TaskClass.CRITICAL in result.per_class
        assert TaskClass.VERIFICATION in result.per_class
        assert TaskClass.MONITORING in result.per_class
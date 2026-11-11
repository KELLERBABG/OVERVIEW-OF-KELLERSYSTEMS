"""Tests for the taint-coded sensor stream audit."""

from __future__ import annotations

from herzschrittmacher.taint import (
    SensorSource,
    SensorStreamAuditor,
    TaintLabel,
    TaintedValue,
    taint_summary,
)


class TestTaintedValue:
    def test_default_label_is_tainted(self):
        v = TaintedValue(value=100.0, source=SensorSource.RATE_SENSOR)
        assert v.label is TaintLabel.TAINTED

    def test_propagate_extends_trail(self):
        v = TaintedValue(value=120.0, source=SensorSource.EGM_ATRIAL)
        v2 = v.propagate(121.0)
        assert v2.value == 121.0
        assert v2.source is SensorSource.EGM_ATRIAL
        assert v2.origin_trail == (SensorSource.EGM_ATRIAL,)


class TestSensorStreamAuditor:
    def test_clean_rate_sensor_allowed(self):
        auditor = SensorStreamAuditor()
        value = TaintedValue(value=75.0, source=SensorSource.RATE_SENSOR)
        record = auditor.audit_for_critical_use(value, "rate_adaptation")
        assert record.allowed
        assert record.value.label is TaintLabel.UNTAINTED
        assert len(auditor.records) == 1

    def test_telemetry_blocked_from_critical(self):
        auditor = SensorStreamAuditor()
        value = TaintedValue(value=5.0, source=SensorSource.TELEMETRY)
        record = auditor.audit_for_critical_use(value, "pulse_amplitude")
        assert not record.allowed
        assert "telemetry" in record.reason

    def test_implausible_rate_blocked(self):
        auditor = SensorStreamAuditor()
        value = TaintedValue(value=9999.0, source=SensorSource.RATE_SENSOR)
        record = auditor.audit_for_critical_use(value, "rate_adaptation")
        assert not record.allowed
        assert "plausible" in record.reason

    def test_deep_trail_suspect(self):
        auditor = SensorStreamAuditor(max_propagation_depth=2)
        value = TaintedValue(value=80.0, source=SensorSource.EGM_VENTRICULAR)
        for _ in range(5):
            value = value.propagate()
        record = auditor.audit_for_critical_use(value, "pulse_discrimination")
        assert not record.allowed
        assert "propagation depth" in record.reason

    def test_history_recorded(self):
        auditor = SensorStreamAuditor()
        v1 = TaintedValue(value=70.0, source=SensorSource.RATE_SENSOR)
        v2 = TaintedValue(value=1.0, source=SensorSource.TELEMETRY)
        auditor.audit_for_critical_use(v1, "rate_adaptation")
        auditor.audit_for_critical_use(v2, "mode_switch")
        assert len(auditor.records) == 2


class TestTaintSummary:
    def test_summary_allowed(self):
        record = SensorStreamAuditor().audit_for_critical_use(
            TaintedValue(value=75.0, source=SensorSource.RATE_SENSOR),
            "rate_adaptation",
        )
        text = taint_summary(record)
        assert "[ALLOWED]" in text
        assert "rate_sensor" in text

    def test_summary_blocked(self):
        record = SensorStreamAuditor().audit_for_critical_use(
            TaintedValue(value=5.0, source=SensorSource.TELEMETRY),
            "pulse_amplitude",
        )
        text = taint_summary(record)
        assert "[BLOCKED]" in text
        assert "telemetry" in text
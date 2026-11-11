"""Taint-coded sensor stream audit.

Continuously tracks data flow from untrusted external sources (ECG/EGM
electrodes, rate sensors, implant telemetry) to critical life-sustaining
system changes, so sensor faults or data manipulation are intercepted
immediately.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class SensorSource(Enum):
    """Untrusted external data sources feeding the pacing engine."""

    EGM_ATRIAL = "egm_atrial"
    EGM_VENTRICULAR = "egm_ventricular"
    RATE_SENSOR = "rate_sensor"
    TELEMETRY = "telemetry"
    IMPLANTED_HISTORY = "implanted_history"


class TaintLabel(Enum):
    """Propagation labels for tainted data."""

    UNTAINTED = "untainted"
    TAINTED = "tainted"
    SUSPECT = "suspect"
    BLOCKED = "blocked"


@dataclass(frozen=True)
class TaintedValue:
    """A value plus its taint origin trail."""

    value: float
    source: SensorSource
    label: TaintLabel = TaintLabel.TAINTED
    origin_trail: tuple[SensorSource, ...] = ()

    def propagate(self, new_value: float | None = None) -> "TaintedValue":
        """Propagate taint through computation."""
        return TaintedValue(
            value=self.value if new_value is None else new_value,
            source=self.source,
            label=self.label,
            origin_trail=(self.source,) + self.origin_trail,
        )


@dataclass
class TaintAuditRecord:
    """One audited data-flow step."""

    value: TaintedValue
    used_by: str
    allowed: bool
    reason: str = ""


@dataclass
class SensorStreamAuditor:
    """Tracks and audits tainted sensor data into critical decisions.

    A value is *critical* if it crosses into the pulse-decision path.
    The auditor blocks critical uses of tainted data that fail
    plausibility or that come from a telemetry source (implant
    telemetry is the classic attack surface for pacemaker exploits).
    """

    max_propagation_depth: int = 8
    telemetry_never_critical: bool = True
    plausibility_bounds: dict[SensorSource, tuple[float, float]] = field(
        default_factory=lambda: {
            SensorSource.EGM_ATRIAL: (0.0, 10_000.0),
            SensorSource.EGM_VENTRICULAR: (0.0, 10_000.0),
            SensorSource.RATE_SENSOR: (0.0, 400.0),  # bpm
            SensorSource.TELEMETRY: (-1e9, 1e9),
            SensorSource.IMPLANTED_HISTORY: (-1e9, 1e9),
        }
    )
    _records: list[TaintAuditRecord] = field(
        default_factory=list, init=False, repr=False
    )

    @property
    def records(self) -> tuple[TaintAuditRecord, ...]:
        return tuple(self._records)

    def audit_for_critical_use(
        self,
        value: TaintedValue,
        used_by: str,
    ) -> TaintAuditRecord:
        """Audit a tainted value crossing into a critical decision path."""
        reasons: list[str] = []

        # 1. Telemetry sources are never allowed into critical paths.
        if self.telemetry_never_critical and value.source is SensorSource.TELEMETRY:
            reasons.append("telemetry source cannot feed a critical path")

        # 2. Propagation depth bound — a value too deep in the audit trail
        #    has lost provenance and is suspect.
        if len(value.origin_trail) > self.max_propagation_depth:
            reasons.append(
                f"propagation depth {len(value.origin_trail)} exceeds "
                f"bound {self.max_propagation_depth}"
            )

        # 3. Plausibility bounds.
        lo, hi = self.plausibility_bounds[value.source]
        if not (lo <= value.value <= hi):
            reasons.append(
                f"value {value.value} outside plausible "
                f"range [{lo}, {hi}] for {value.source.value}"
            )

        allowed = not reasons
        label = TaintLabel.UNTAINTED if allowed else TaintLabel.BLOCKED
        # Re-label the recorded value with the audit outcome.
        audited_value = TaintedValue(
            value=value.value,
            source=value.source,
            label=label,
            origin_trail=value.origin_trail,
        )
        record = TaintAuditRecord(
            value=audited_value,
            used_by=used_by,
            allowed=allowed,
            reason="; ".join(reasons) if reasons else "clean",
        )
        self._records.append(record)
        return record


def taint_summary(record: TaintAuditRecord) -> str:
    """Human-readable summary of an audit record."""
    status = "ALLOWED" if record.allowed else "BLOCKED"
    return (
        f"[{status}] {record.value.source.value}"
        f"(label={record.value.label.value}) -> {record.used_by}"
        + (f" :: {record.reason}" if record.reason and record.reason != "clean" else "")
    )
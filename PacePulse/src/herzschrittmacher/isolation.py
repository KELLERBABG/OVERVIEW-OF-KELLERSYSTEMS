"""Temporal isolation asymmetry — scheduling for pulse-delivery isolation.

Prevents secondary monitoring processes (telemetry transmission, diagnostics)
from deriving information via CPU-cycle measurement or from delaying critical
real-time pulse delivery. Scheduling is *asymmetric*: critical pacing tasks
get a hard reservation and cannot be preempted or delayed; background work
is strictly budgeted.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class TaskClass(Enum):
    """Task classification for asymmetric scheduling."""

    CRITICAL = "critical"          # pulse delivery, pacing engine
    VERIFICATION = "verification"  # SMT proof — bounded, high priority
    MONITORING = "monitoring"      # telemetry, diagnostics, episode log


@dataclass
class TaskBudget:
    """Worst-case execution budget for a task class."""

    task_class: TaskClass
    budget_us: int          # worst-case budget in microseconds
    deadline_us: int        # relative deadline in microseconds
    period_us: int          # release period in microseconds


@dataclass
class SchedulabilityResult:
    """Result of the temporal isolation schedulability check."""

    feasible: bool
    total_utilization: float
    per_class: dict[TaskClass, float] = field(default_factory=dict)
    reason: str = ""


@dataclass
class IsolationModel:
    """Asymmetric temporal isolation model.

    Critical tasks are scheduled with a fixed-priority, non-preemptive
    window (the ``pulse_window``) inside which no background work runs.
    Monitoring tasks are bandwidth-limited so their cumulative CPU cannot
    be used as a side channel, and they can never delay a critical release.
    """

    pulse_window_us: int = 250          # worst-case pulse decision + delivery
    critical_period_us: int = 1_000     # worst-case pacing cycle
    verification_budget_us: int = 200   # SMT proof bounded budget
    monitoring_budget_per_sec_us: int = 50_000  # 5% CPU cap for monitoring
    switching_cost_us: int = 5

    def __post_init__(self) -> None:
        if self.pulse_window_us <= 0:
            raise ValueError("pulse_window_us must be positive")
        if self.pulse_window_us >= self.critical_period_us:
            raise ValueError("pulse_window_us must be shorter than critical_period_us")
        if self.switching_cost_us < 0:
            raise ValueError("switching_cost_us cannot be negative")

    def critical_utilization(self) -> float:
        """Utilization of the critical pacing path."""
        u_crit = (self.pulse_window_us + self.switching_cost_us) / self.critical_period_us
        u_ver = self.verification_budget_us / self.critical_period_us
        return u_crit + u_ver

    def monitoring_utilization(self) -> float:
        """Monitoring CPU bound per pacing cycle."""
        return self.monitoring_budget_per_sec_us / 1_000_000.0

    def can_delay_pulse(self, task_class: TaskClass) -> bool:
        """Whether a task class may ever delay a critical pulse delivery.

        Asymmetry: only critical and verification work exists inside the
        pulse window; monitoring can never delay a pulse.
        """
        if task_class is TaskClass.CRITICAL:
            return False  # critical is the pulse itself
        if task_class is TaskClass.VERIFICATION:
            return True   # bounded by budget; part of the pulse decision
        return False      # monitoring: never delays a pulse

    def schedulability_check(self) -> SchedulabilityResult:
        """Check overall schedulability of the asymmetry model.

        Feasibility criterion (sufficient, not necessary):
        1. Critical + verification utilization < 1 per critical period.
        2. Monitoring budget never exhausts the remaining CPU.
        3. The verification budget fits inside the pulse window.
        """
        per_class = {
            TaskClass.CRITICAL: self.pulse_window_us / self.critical_period_us,
            TaskClass.VERIFICATION: self.verification_budget_us / self.critical_period_us,
            TaskClass.MONITORING: self.monitoring_utilization(),
        }
        total = sum(per_class.values())

        if self.verification_budget_us > self.pulse_window_us:
            return SchedulabilityResult(
                feasible=False,
                total_utilization=total,
                per_class=per_class,
                reason=(
                    f"verification budget {self.verification_budget_us}µs "
                    f"exceeds pulse window {self.pulse_window_us}µs"
                ),
            )

        critical_side = per_class[TaskClass.CRITICAL] + per_class[TaskClass.VERIFICATION]
        if critical_side >= 1.0:
            return SchedulabilityResult(
                feasible=False,
                total_utilization=total,
                per_class=per_class,
                reason=f"critical+verification utilization {critical_side:.3f} >= 1.0",
            )

        if total >= 1.0:
            return SchedulabilityResult(
                feasible=False,
                total_utilization=total,
                per_class=per_class,
                reason=f"total utilization {total:.3f} >= 1.0",
            )

        return SchedulabilityResult(
            feasible=True,
            total_utilization=total,
            per_class=per_class,
            reason="asymmetric schedule feasible",
        )
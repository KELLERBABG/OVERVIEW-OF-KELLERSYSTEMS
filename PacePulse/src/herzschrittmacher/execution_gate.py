"""Trustless execution gate.

Blocks the physical impulse to the cardiac muscle unless a valid
mathematical proof for the absolute safety of the action exists. In this
research prototype the gate is software: a pulse command is only forwarded
once the Z3 verification engine returns ``safe=True`` for the decision path.
"""

from __future__ import annotations

from dataclasses import dataclass

from .constraints import PathConstraint
from .pacing import PacingDecision
from .verification import VerificationEngine, VerificationResult


@dataclass(frozen=True)
class GateResult:
    """Outcome of the execution gate for a pulse request."""

    released: bool
    reason: str
    verification: VerificationResult | None = None
    decision: PacingDecision | None = None

    def as_dict(self) -> dict:
        return {
            "released": self.released,
            "reason": self.reason,
            "verification": self.verification.as_dict() if self.verification else None,
            "decision_action": self.decision.action if self.decision else None,
            "decision_reason": self.decision.reason if self.decision else None,
        }


class ExecutionGate:
    """Automated control system blocking unsafe pulse commands.

    Policy (fail-closed):
    - If the pacing engine itself blocked the decision → deny.
    - If the SMT proof is missing, timed out, or returns ``unknown`` → deny.
    - Only a full ``safe=True`` verification releases the pulse command.
    """

    def __init__(self, engine: VerificationEngine | None = None) -> None:
        self.engine = engine or VerificationEngine()
        self._history: list[GateResult] = []

    @property
    def history(self) -> tuple[GateResult, ...]:
        return tuple(self._history)

    def release(
        self,
        decision: PacingDecision,
        chamber,
        amplitude_ma: float,
        pulse_width_ms: float,
    ) -> GateResult:
        """Evaluate whether the pulse command may be released."""
        if decision is None or decision.action != "pace":
            result = GateResult(
                released=False,
                reason=(
                    f"pacing engine did not authorize a pace "
                    f"(action={decision.action if decision else 'none'})"
                ),
                decision=decision,
            )
            self._history.append(result)
            return result

        if decision.path is None:
            result = GateResult(
                released=False,
                reason="missing kinetic path constraint — cannot prove safety",
                decision=decision,
            )
            self._history.append(result)
            return result

        result = self.verify_and_release(
            decision.path,
            chamber,
            amplitude_ma,
            pulse_width_ms,
            decision=decision,
        )
        return result

    def verify_and_release(
        self,
        path: PathConstraint,
        chamber,
        amplitude_ma: float,
        pulse_width_ms: float,
        decision: PacingDecision | None = None,
    ) -> GateResult:
        """Verify the path with the SMT engine, then release only if safe."""
        verification = self.engine.check_decision(
            path=path,
            chamber=chamber,
            amplitude_ma=amplitude_ma,
            pulse_width_ms=pulse_width_ms,
        )
        if verification.safe:
            result = GateResult(
                released=True,
                reason=(
                    "proof obtained: all lethal invariants hold "
                    "(all UNSAT)"
                ),
                verification=verification,
                decision=decision,
            )
        else:
            result = GateResult(
                released=False,
                reason=(
                    f"no proof: blocked by "
                    f"{', '.join(verification.blocked_invariants) or 'unknown'}"
                ),
                verification=verification,
                decision=decision,
            )
        self._history.append(result)
        return result
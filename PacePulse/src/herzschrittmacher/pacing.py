"""Pacing engine — DDD timing-cycle state machine.

Implements the sense → inhibit/trigger → pulse decision loop with AV delay,
PVARP, blanking, and escape-interval timing. Every decision emits a kinetic
path constraint (see :mod:`herzschrittmacher.constraints`) that downstream
verification checks against the lethal invariants.
"""

from __future__ import annotations

from dataclasses import dataclass

from .constraints import ConstraintGenerator, PathConstraint
from .invariants import InvariantVerdict, check_all_invariants
from .modes import DEFAULT_TIMING, CardiacState, Chamber, PacingMode, TimingCycles
from .pulse_analysis import SymbolicPulse


@dataclass(frozen=True)
class PacingEvent:
    """A sensed or paced event recorded by the engine."""

    chamber: Chamber
    time_ms: int
    kind: str  # "sense" | "pace"
    tainted: bool = False


@dataclass(frozen=True)
class PacingDecision:
    """Outcome of a pacing-engine decision point."""

    action: str  # "pace" | "inhibit" | "trigger" | "blocked"
    chamber: Chamber | None
    time_ms: int
    reason: str
    path: PathConstraint | None = None
    verdicts: tuple[InvariantVerdict, ...] = ()


class PacingSystem:
    """Timing-cycle pacing engine for the single-chamber / dual-chamber subset.

    The engine itself performs no SMT solving — it emits path constraints and
    checks the *concrete* state against the lethal invariants via
    :func:`check_all_invariants`. The SMT layer (VerificationEngine) and the
    trustless gate (ExecutionGate) wrap this engine for the formal decision.
    """

    def __init__(
        self,
        mode: PacingMode = PacingMode.DDD,
        cycles: TimingCycles = DEFAULT_TIMING,
        state: CardiacState | None = None,
    ) -> None:
        self.mode = mode
        self.cycles = cycles
        self.state = state or CardiacState()
        self.running = False
        self.events: list[PacingEvent] = []
        self._av_timer_start_ms: int | None = None
        self._av_timer_active = False

    # -- lifecycle -------------------------------------------------------

    def start(self) -> None:
        """Start the pacing engine."""
        self.running = True

    def stop(self) -> None:
        self.running = False

    # -- sensing ---------------------------------------------------------

    def sense(self, chamber: Chamber, time_ms: int, tainted: bool = False) -> PacingDecision:
        """Handle a sensed intrinsic event at ``time_ms``.

        Returns the engine's decision (usually ``inhibit`` when a sensed
        event resets an escape interval, or ``trigger`` for VDD tracking).
        """
        if not self.running:
            raise RuntimeError("pacing engine not started")
        if time_ms < self.state.time_ms:
            raise ValueError("time_ms must be non-decreasing")
        self.state.time_ms = time_ms

        gen = ConstraintGenerator(self.mode, self.cycles, self.state, time_ms)
        path = gen.sense_path(chamber)

        if not self.mode.can_sense(chamber):
            self.events.append(PacingEvent(chamber, time_ms, "sense", tainted))
            return PacingDecision(
                action="inhibit",
                chamber=chamber,
                time_ms=time_ms,
                reason=f"mode {self.mode.value} does not sense {chamber.value}",
                path=path,
            )

        if self.state.in_blanking(chamber, time_ms, self.cycles):
            self.events.append(PacingEvent(chamber, time_ms, "sense", tainted))
            return PacingDecision(
                action="inhibit",
                chamber=chamber,
                time_ms=time_ms,
                reason="sensed event in blanking window ignored",
                path=path,
            )

        if chamber is Chamber.ATRIUM:
            self.state.apply_atrial_event(time_ms, self.cycles)
            self._av_timer_start_ms = time_ms
            self._av_timer_active = True
        else:
            self.state.apply_ventricular_event(time_ms, self.cycles)
            self._av_timer_active = False

        self.events.append(PacingEvent(chamber, time_ms, "sense", tainted))
        return PacingDecision(
            action="inhibit",
            chamber=chamber,
            time_ms=time_ms,
            reason=f"intrinsic {chamber.value} sense inhibits pacing",
            path=path,
        )

    # -- pacing ----------------------------------------------------------

    def check_escape(self, chamber: Chamber, time_ms: int) -> PacingDecision:
        """Check whether the escape interval for ``chamber`` has elapsed.

        Called by the simulated timer at each millisecond; if the interval
        has elapsed and the chamber is not refractory, the engine emits a
        ``pace`` decision (subject to downstream formal verification).
        """
        if not self.running:
            raise RuntimeError("pacing engine not started")
        if time_ms < self.state.time_ms:
            raise ValueError("time_ms must be non-decreasing")
        self.state.time_ms = time_ms

        gen = ConstraintGenerator(self.mode, self.cycles, self.state, time_ms)
        path = gen.pulse_path(chamber)

        if not self.mode.can_pace(chamber):
            return PacingDecision(
                action="inhibit",
                chamber=chamber,
                time_ms=time_ms,
                reason=f"mode {self.mode.value} cannot pace {chamber.value}",
                path=path,
            )

        if self.state.is_refractory(chamber, time_ms, self.cycles):
            return PacingDecision(
                action="blocked",
                chamber=chamber,
                time_ms=time_ms,
                reason=f"{chamber.value} refractory at t={time_ms}ms",
                path=path,
            )

        escape = self._escape_interval(chamber)
        reference = self._escape_reference(chamber)
        if reference is None:
            # No prior event in this chamber and no AV window: pace at the
            # next opportunity per the lower-rate interval.
            return PacingDecision(
                action="pace",
                chamber=chamber,
                time_ms=time_ms,
                reason=f"no prior event; escape interval {escape}ms applies",
                path=path,
            )

        if (time_ms - reference) >= escape:
            return PacingDecision(
                action="pace",
                chamber=chamber,
                time_ms=time_ms,
                reason=f"escape interval {escape}ms elapsed",
                path=path,
            )

        return PacingDecision(
            action="inhibit",
            chamber=chamber,
            time_ms=time_ms,
            reason=f"escape interval {escape}ms not yet elapsed",
            path=path,
        )

    def request_pace(
        self,
        chamber: Chamber,
        time_ms: int,
        amplitude_ma: float,
        pulse_width_ms: float,
    ) -> PacingDecision:
        """Full pulse-request path: timing + invariant checks.

        This performs the *concrete* invariant check (all lethal invariants
        against the current state). The SMT-level proof happens in the
        verification engine; this method is the engine-side guard.
        """
        decision = self.check_escape(chamber, time_ms)
        if decision.action != "pace":
            return decision

        pulse = SymbolicPulse(
            chamber=chamber,
            time_ms=time_ms,
            amplitude_ma=amplitude_ma,
            pulse_width_ms=pulse_width_ms,
        )
        verdicts = check_all_invariants(self.state, pulse, self.cycles)

        if not all(v.holds for v in verdicts):
            failures = [v.name for v in verdicts if not v.holds]
            return PacingDecision(
                action="blocked",
                chamber=chamber,
                time_ms=time_ms,
                reason=f"invariant violation: {', '.join(failures)}",
                path=decision.path,
                verdicts=verdicts,
            )

        if chamber is Chamber.VENTRICLE:
            self.state.apply_ventricular_pace(time_ms, self.cycles)
            self._av_timer_active = False
        else:
            self.state.apply_atrial_pace(time_ms, self.cycles)
            self._av_timer_start_ms = time_ms
            self._av_timer_active = True

        self.events.append(PacingEvent(chamber, time_ms, "pace"))
        return PacingDecision(
            action="pace",
            chamber=chamber,
            time_ms=time_ms,
            reason="pulse delivered (invariants hold)",
            path=decision.path,
            verdicts=verdicts,
        )

    # -- internal --------------------------------------------------------

    def _escape_interval(self, chamber: Chamber) -> int:
        """Escape interval currently in effect for ``chamber``.

        In a tracking mode (DDD/VDD), the ventricular escape is the AV delay
        while the AV timer is running; otherwise the programmed lower-rate
        interval applies.
        """
        if chamber is Chamber.VENTRICLE and self._av_timer_active:
            if self._av_timer_start_ms is not None:
                elapsed = self.state.time_ms - self._av_timer_start_ms
                if elapsed < self.cycles.av_delay:
                    return self.cycles.av_delay
            # AV delay elapsed or timer just started: fire at AV delay.
            return self.cycles.av_delay
        return self.cycles.lower_rate_interval

    def _escape_reference(self, chamber: Chamber) -> int | None:
        """Reference time the current escape interval is measured from."""
        if chamber is Chamber.VENTRICLE and self._av_timer_active:
            if self._av_timer_start_ms is not None:
                return self._av_timer_start_ms
        return self._last_event(chamber)

    def _last_event(self, chamber: Chamber) -> int | None:
        if chamber is Chamber.VENTRICLE:
            return max(
                [
                    t
                    for t in (
                        self.state.last_ventricular_sense_ms,
                        self.state.last_ventricular_pace_ms,
                    )
                    if t is not None
                ],
                default=None,
            )
        return max(
            [
                t
                for t in (
                    self.state.last_atrial_sense_ms,
                    self.state.last_atrial_pace_ms,
                )
                if t is not None
            ],
            default=None,
        )
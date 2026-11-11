"""Kinetic path constraint generation for pacing decisions.

Every pacing decision (sense → inhibit/trigger → pulse) is translated into a
boolean expression describing the entire pacing decision path. These path
conditions are handed to the SMT solver to check satisfiability against the
lethal invariants (see :mod:`herzschrittmacher.verification`).
"""

from __future__ import annotations

from dataclasses import dataclass

from .modes import CardiacState, Chamber, PacingMode, TimingCycles


@dataclass(frozen=True)
class PathConstraint:
    """Boolean expression describing a pacing decision path.

    ``components`` is the conjunction of symbolic predicates describing the
    decision path. All boolean-valued predicates are rendered in lowercase
    (``true``/``false``) so the Z3 path encoder can parse them exactly.
    """

    components: tuple[str, ...]

    def __str__(self) -> str:
        return " ∧ ".join(self.components) if self.components else "⊤"

    @property
    def is_trivial(self) -> bool:
        return not self.components

    def and_with(self, *others: str) -> "PathConstraint":
        return PathConstraint(tuple(dict.fromkeys(self.components + others)))


class ConstraintGenerator:
    """Generates kinetic path constraints for pacing decisions.

    The generated path condition captures the *physiological conditions*
    under which a pulse decision was made, in symbolic form. The Z3 layer
    then checks whether this path is consistent with the lethal invariants.
    """

    def __init__(
        self,
        mode: PacingMode,
        cycles: TimingCycles,
        state: CardiacState,
        time_ms: int,
    ) -> None:
        self.mode = mode
        self.cycles = cycles
        self.state = state
        self.time_ms = time_ms

    def pulse_path(self, chamber: Chamber) -> PathConstraint:
        """Build the full path constraint for a pulse to ``chamber``.

        The path asserts the *physiological case* the pacing engine
        believes it is in — e.g. escape interval elapsed, chamber not
        refractory/blanked, no sensed intrinsic beat, mode permits pacing.
        """
        comps: list[str] = [f"mode={self.mode.value}"]

        if not self.mode.can_pace(chamber):
            comps.append(f"can_pace({chamber.value})=false")
            return PathConstraint(tuple(comps))

        comps.append(f"can_pace({chamber.value})=true")

        # Chamber not refractory / not blanked.
        refr = self.state.is_refractory(chamber, self.time_ms, self.cycles)
        blank = self.state.in_blanking(chamber, self.time_ms, self.cycles)
        comps.append(f"refractory({chamber.value})={str(refr).lower()}")
        comps.append(f"blanking({chamber.value})={str(blank).lower()}")

        # Escape interval state: no intrinsic sense since last event.
        last = self._last_event_time(chamber)
        escape = self._escape_interval(chamber)
        if last is not None:
            elapsed = self.time_ms - last
            comps.append(f"elapsed_ms={elapsed}")
            comps.append(
                f"elapsed>=escape_interval={str(elapsed >= escape).lower()}"
            )
        else:
            comps.append("elapsed_ms=∞")
            comps.append("elapsed>=escape_interval=true")

        # Triggering / inhibition semantics.
        if self.mode.response.value == "triggered" and chamber is Chamber.VENTRICLE:
            comps.append("trigger=av_delay_elapsed")
        if self.mode.response.value == "inhibited":
            comps.append("inhibited_by_intrinsic=false")

        return PathConstraint(tuple(comps))

    def sense_path(self, chamber: Chamber) -> PathConstraint:
        """Build the path constraint for a sensed intrinsic event."""
        comps = [f"sense({chamber.value})=true", f"time_ms={self.time_ms}"]

        if not self.mode.can_sense(chamber):
            comps.append(f"can_sense({chamber.value})=false")
        else:
            comps.append(f"can_sense({chamber.value})=true")
            blanked = self.state.in_blanking(chamber, self.time_ms, self.cycles)
            comps.append(f"blanking({chamber.value})={str(blanked).lower()}")

        return PathConstraint(tuple(comps))

    def mode_switch_path(self, target: PacingMode, reason: str) -> PathConstraint:
        """Path constraint for a mode switch decision."""
        return PathConstraint(
            (
                f"mode_switch={self.mode.value}->{target.value}",
                f"reason={reason}",
            )
        )

    def _last_event_time(self, chamber: Chamber) -> int | None:
        if chamber is Chamber.ATRIUM:
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

    def _escape_interval(self, chamber: Chamber) -> int:
        """Escape interval the pacing engine is operating on.

        Ventricular escape = lower-rate interval unless an atrial event has
        started the AV window, in which case it is the AV delay. Atrial
        escape = lower-rate interval (VA interval in DDD).
        """
        if chamber is Chamber.VENTRICLE and self.mode in (
            PacingMode.DDD,
            PacingMode.DDDR,
            PacingMode.VDD,
        ):
            last_a = self.state.last_atrial_sense_ms or self.state.last_atrial_pace_ms
            if last_a is not None and self.time_ms - last_a < self.cycles.av_delay:
                return self.cycles.av_delay
            return self.cycles.lower_rate_interval
        return self.cycles.lower_rate_interval

    def coalesce(self, *constraints: PathConstraint) -> PathConstraint:
        """Combine several path constraints into one boolean expression."""
        seen: set[str] = set()
        merged: list[str] = []
        for c in constraints:
            for comp in c.components:
                if comp not in seen:
                    seen.add(comp)
                    merged.append(comp)
        return PathConstraint(tuple(merged))
"""Lethal invariant enforcement — unbreakable safety rules.

The invariants in this module are the non-negotiable safety properties that
must hold over the entire runtime: no pulse into a refractory period, no
escape-interval (rate) violation, and no pulse-energy excursion. The
verification engine checks every pacing decision against all invariants.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol

from .modes import CardiacState, Chamber, TimingCycles


class PulseRequest(Protocol):
    """Structural type for a pulse request accepted by invariants."""

    chamber: Chamber
    time_ms: int
    amplitude_ma: float
    pulse_width_ms: float


@dataclass(frozen=True)
class InvariantVerdict:
    """Result of checking one lethal invariant against a pulse request."""

    name: str
    holds: bool
    detail: str = ""


class LethalInvariant(Protocol):
    """A lethal invariant: check a pulse request against a cardiac state."""

    name: str

    def check(
        self,
        state: CardiacState,
        request: PulseRequest,
        cycles: TimingCycles,
    ) -> InvariantVerdict:
        """Return whether the invariant holds for ``request``."""
        ...


@dataclass(frozen=True)
class NoPulseInRefractory:
    """Lethal invariant: never deliver a pulse into a refractory chamber."""

    name: str = "no_pulse_in_refractory"

    def check(
        self,
        state: CardiacState,
        request: PulseRequest,
        cycles: TimingCycles,
    ) -> InvariantVerdict:
        if state.is_refractory(request.chamber, request.time_ms, cycles):
            return InvariantVerdict(
                self.name,
                holds=False,
                detail=(
                    f"pulse to {request.chamber.value} at t={request.time_ms}ms "
                    "would land in a refractory period"
                ),
            )
        return InvariantVerdict(self.name, holds=True)


@dataclass(frozen=True)
class NoPulseInBlanking:
    """Lethal invariant: never pace into a blanked sense amplifier window.

    Pacing into a blanking window is not inherently tissue-unsafe, but it
    indicates the pacing engine is violating its own sensing model, which
    is a failure precondition for oversensing/undersensing faults.
    """

    name: str = "no_pulse_in_blanking"

    def check(
        self,
        state: CardiacState,
        request: PulseRequest,
        cycles: TimingCycles,
    ) -> InvariantVerdict:
        if state.in_blanking(request.chamber, request.time_ms, cycles):
            return InvariantVerdict(
                self.name,
                holds=False,
                detail=(
                    f"pulse to {request.chamber.value} at t={request.time_ms}ms "
                    "would land in the blanking window"
                ),
            )
        return InvariantVerdict(self.name, holds=True)


@dataclass(frozen=True)
class RateWithinEnvelope:
    """Lethal invariant: escape interval stays within the rate envelope.

    Checks the lower-rate floor (never pace faster than the floor). The
    interval is the time since the last event in the target chamber.
    """

    name: str = "rate_within_envelope"

    def check(
        self,
        state: CardiacState,
        request: PulseRequest,
        cycles: TimingCycles,
    ) -> InvariantVerdict:
        last = _last_event_in_chamber(state, request.chamber)
        if last is not None:
            interval = request.time_ms - last
            if interval < cycles.lower_rate_interval:
                return InvariantVerdict(
                    self.name,
                    holds=False,
                    detail=(
                        f"pulse interval {interval}ms is faster than "
                        f"lower-rate floor {cycles.lower_rate_interval}ms"
                    ),
                )
        return InvariantVerdict(self.name, holds=True)


@dataclass(frozen=True)
class EnergyWithinEnvelope:
    """Lethal invariant: symbolic pulse energy inside the tissue-safety envelope.

    Pulse energy in a capacitive-discharge pacemaker is:

        E = I² · t / C

    where I is the delivered current, t the pulse width, C the output
    capacitance. The invariant bounds E below the programmed maximum.
    All failure modes are accumulated so the verdict reports every
    violated bound.
    """

    name: str = "pulse_energy_within_envelope"

    output_capacitance_uf: float = 15.0
    max_energy_uj: float = 30.0
    max_pulse_width_ms: float = 2.0
    max_amplitude_ma: float = 8.0

    def check(
        self,
        state: CardiacState,
        request: PulseRequest,
        cycles: TimingCycles,
    ) -> InvariantVerdict:
        reasons: list[str] = []

        if request.pulse_width_ms <= 0:
            reasons.append("pulse_width_ms must be positive")
        if request.amplitude_ma < 0:
            reasons.append("amplitude_ma cannot be negative")
        if request.pulse_width_ms > cycles.MAX_PULSE_WIDTH_MS:
            reasons.append(
                f"pulse width {request.pulse_width_ms}ms exceeds hard "
                f"limit {cycles.MAX_PULSE_WIDTH_MS}ms"
            )
        if request.pulse_width_ms > self.max_pulse_width_ms:
            reasons.append(
                f"pulse width {request.pulse_width_ms}ms exceeds envelope "
                f"limit {self.max_pulse_width_ms}ms"
            )
        if request.amplitude_ma > self.max_amplitude_ma:
            reasons.append(
                f"amplitude {request.amplitude_ma}mA exceeds envelope "
                f"limit {self.max_amplitude_ma}mA"
            )

        # E = I² · t / C ; I in mA, t in ms, C in µF → E in µJ.
        energy_uj = (
            request.amplitude_ma ** 2 * request.pulse_width_ms / self.output_capacitance_uf
        )
        if energy_uj > self.max_energy_uj:
            reasons.append(
                f"symbolic pulse energy {energy_uj:.3f}µJ exceeds "
                f"envelope limit {self.max_energy_uj}µJ"
            )

        if reasons:
            return InvariantVerdict(self.name, holds=False, detail="; ".join(reasons))
        return InvariantVerdict(
            self.name,
            holds=True,
            detail=f"energy={energy_uj:.3f}µJ",
        )


@dataclass(frozen=True)
class NoSensedPulseInBlanking:
    """Lethal invariant: sensed events during blanking never trigger pacing.

    A sense that arrives during an amplifier-blanking window shall not be
    used as the basis of a pace decision; doing so is the classic
    oversensing/mode-switch failure catalogue entry.

    The blanking window started by a *paced* event is the well-known
    post-pace blanking where a health sense is expected to be suppressed.
    A blanked sense during such a window is benign. The lethal case is a
    sense falling inside a blanking window started by an *intrinsic*
    sensed event — that indicates the engine processed a sense it should
    not have (the classic oversensing pre-condition).
    """

    name: str = "no_sensed_pulse_in_blanking"

    def check(
        self,
        state: CardiacState,
        request: PulseRequest,
        cycles: TimingCycles,
    ) -> InvariantVerdict:
        # Sense happened inside a blanking window, and that window was
        # started by an intrinsic event (not a pace) → lethal candidate.
        last_sense = _last_sense_in_chamber(state, request.chamber)
        if last_sense is None:
            return InvariantVerdict(self.name, holds=True)

        if not state.in_blanking(request.chamber, last_sense, cycles):
            return InvariantVerdict(self.name, holds=True)

        if state.blanking_started_by(request.chamber, last_sense):
            # The sense itself started its own blanking window (e.g. the
            # post-V-sense PVARP/blanking). This is the normal physiological
            # blanking — benign.
            return InvariantVerdict(self.name, holds=True)

        return InvariantVerdict(
            self.name,
            holds=False,
            detail=(
                f"sensed event at t={last_sense}ms fell inside a blanking "
                "window not started by the sense itself (oversensing "
                "pre-condition)"
            ),
        )


def _last_event_in_chamber(state: CardiacState, chamber: Chamber) -> int | None:
    if chamber is Chamber.VENTRICLE:
        return max(
            [
                t
                for t in (
                    state.last_ventricular_sense_ms,
                    state.last_ventricular_pace_ms,
                )
                if t is not None
            ],
            default=None,
        )
    return max(
        [
            t
            for t in (
                state.last_atrial_sense_ms,
                state.last_atrial_pace_ms,
            )
            if t is not None
        ],
        default=None,
    )


def _last_sense_in_chamber(state: CardiacState, chamber: Chamber) -> int | None:
    if chamber is Chamber.VENTRICLE:
        return state.last_ventricular_sense_ms
    return state.last_atrial_sense_ms


DEFAULT_INVARIANTS: tuple[LethalInvariant, ...] = (
    NoPulseInRefractory(),
    NoPulseInBlanking(),
    RateWithinEnvelope(),
    EnergyWithinEnvelope(),
    NoSensedPulseInBlanking(),
)


def check_all_invariants(
    state: CardiacState,
    request: PulseRequest,
    cycles: TimingCycles,
    invariants: tuple[LethalInvariant, ...] = DEFAULT_INVARIANTS,
) -> tuple[InvariantVerdict, ...]:
    """Check ``request`` against every lethal invariant."""
    return tuple(inv.check(state, request, cycles) for inv in invariants)
"""Formal cardiac model — NBG pacing modes, chambers, and timing cycles.

Implements the formal model of the heart's electrical conduction states for
the DDD-mode pacing subset: refractory periods (PVARP, blanking), AV delay,
lower/upper rate limits, and mode-switching rules.

This is a research model. It is NOT medical device software.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class Chamber(Enum):
    """Cardiac chambers relevant to pacing."""

    ATRIUM = "atrium"
    VENTRICLE = "ventricle"


CHAMBER_FROM_STRING: dict[str, Chamber] = {
    "atrium": Chamber.ATRIUM,
    "atrial": Chamber.ATRIUM,
    "a": Chamber.ATRIUM,
    "ventricle": Chamber.VENTRICLE,
    "ventricular": Chamber.VENTRICLE,
    "v": Chamber.VENTRICLE,
}


class Polarity(Enum):
    """NBG classification: chambers paced/sensed."""

    NONE = "none"
    ATRIUM = "atrium"
    VENTRICLE = "ventricle"
    DUAL = "dual"


class Response(Enum):
    """NBG response modes: inhibition, triggering, dual."""

    INHIBITED = "inhibited"
    TRIGGERED = "triggered"
    DUAL = "dual"
    NONE = "none"


class PacingMode(Enum):
    """NBG pacing modes supported by the prototype."""

    VVI = "VVI"
    AAI = "AAI"
    DDD = "DDD"
    VDD = "VDD"
    DDDR = "DDDR"
    OFF = "OFF"

    @property
    def paced(self) -> Polarity:
        """Chambers that can deliver pulses in this mode."""
        if self in (PacingMode.VVI,):
            return Polarity.VENTRICLE
        if self in (PacingMode.AAI,):
            return Polarity.ATRIUM
        if self in (PacingMode.DDD, PacingMode.DDDR):
            return Polarity.DUAL
        if self in (PacingMode.VDD,):
            return Polarity.VENTRICLE
        return Polarity.NONE

    @property
    def sensed(self) -> Polarity:
        """Chambers from which sensing is enabled."""
        if self in (PacingMode.VVI,):
            return Polarity.VENTRICLE
        if self in (PacingMode.AAI,):
            return Polarity.ATRIUM
        if self in (PacingMode.DDD, PacingMode.DDDR, PacingMode.VDD):
            return Polarity.DUAL
        return Polarity.NONE

    @property
    def response(self) -> Response:
        """NBG response mode."""
        return {
            PacingMode.VVI: Response.INHIBITED,
            PacingMode.AAI: Response.INHIBITED,
            PacingMode.DDD: Response.DUAL,
            PacingMode.DDDR: Response.DUAL,
            PacingMode.VDD: Response.TRIGGERED,
            PacingMode.OFF: Response.NONE,
        }[self]

    @property
    def rate_adaptive(self) -> bool:
        """Whether the mode uses rate-adaptive sensors."""
        return self is PacingMode.DDDR

    def can_pace(self, chamber: Chamber) -> bool:
        """Whether this mode may deliver a pulse to ``chamber``."""
        if self is PacingMode.OFF:
            return False
        if self is PacingMode.VVI:
            return chamber is Chamber.VENTRICLE
        if self is PacingMode.AAI:
            return chamber is Chamber.ATRIUM
        if self in (PacingMode.DDD, PacingMode.DDDR):
            return True
        if self is PacingMode.VDD:
            return chamber is Chamber.VENTRICLE
        return False

    def can_sense(self, chamber: Chamber) -> bool:
        """Whether this mode enables sensing in ``chamber``."""
        if self is PacingMode.OFF:
            return False
        if self in (PacingMode.VVI, PacingMode.VDD):
            if chamber is Chamber.ATRIUM and self is PacingMode.VDD:
                return True
            return chamber is Chamber.VENTRICLE
        if self is PacingMode.AAI:
            return chamber is Chamber.ATRIUM
        if self in (PacingMode.DDD, PacingMode.DDDR):
            return True
        return False


@dataclass(frozen=True)
class TimingCycles:
    """Programmed timing cycles for the pacing model (milliseconds, int).

    All timing values are strict integers (ms) to keep the SMT encoding
    decidable and the invariants exact.
    """

    av_delay: int = 180
    pvarp: int = 350
    post_ventricular_blanking: int = 40
    post_atrial_blanking: int = 30
    lower_rate_interval: int = 1000  # 60 bpm
    upper_rate_interval: int = 300   # 200 bpm
    ventricular_refractory: int = 320
    atrial_refractory: int = 320

    # Safety envelope — never programmed below these floors.
    MIN_AV_DELAY = 50
    MIN_PVARP = 150
    MAX_PULSE_WIDTH_MS = 2
    MIN_ESCAPE_INTERVAL_MS = 150  # hard floor for any pacing interval

    def __post_init__(self) -> None:
        if self.av_delay < self.MIN_AV_DELAY:
            raise ValueError(f"AV delay {self.av_delay} below floor {self.MIN_AV_DELAY}")
        if self.pvarp < self.MIN_PVARP:
            raise ValueError(f"PVARP {self.pvarp} below floor {self.MIN_PVARP}")
        if self.lower_rate_interval <= 0:
            raise ValueError("lower_rate_interval must be positive")
        if self.upper_rate_interval >= self.lower_rate_interval:
            raise ValueError("upper_rate_interval must be shorter than lower_rate_interval")
        if self.lower_rate_interval < self.MIN_ESCAPE_INTERVAL_MS:
            raise ValueError(
                f"lower_rate_interval {self.lower_rate_interval} below "
                f"floor {self.MIN_ESCAPE_INTERVAL_MS}"
            )


@dataclass
class CardiacState:
    """Formal model of the current cardiac electrical state."""

    time_ms: int = 0
    last_atrial_sense_ms: int | None = None
    last_ventricular_sense_ms: int | None = None
    last_atrial_pace_ms: int | None = None
    last_ventricular_pace_ms: int | None = None
    pvarp_until_ms: int = 0
    ventricular_blanking_until_ms: int = 0
    atrial_blanking_until_ms: int = 0
    ventricular_blanking_start_ms: int | None = None
    atrial_blanking_start_ms: int | None = None

    def is_refractory(self, chamber: Chamber, t: int, cycles: TimingCycles) -> bool:
        """True if ``chamber`` is in a refractory period at time ``t``.

        Ventricular: PVARP and ventricular refractory after V event.
        Atrial: PVARP (post-ventricular atrial refractory) after V event.
        """
        last_v = self._last_ventricular_time()
        last_a = self._last_atrial_time()

        if chamber is Chamber.VENTRICLE:
            if last_v is not None and t - last_v < cycles.ventricular_refractory:
                return True
            return False

        # Atrial refractoriness = PVARP window after any ventricular event.
        if last_v is not None and t < self.pvarp_until_ms:
            return True
        if last_a is not None and t - last_a < cycles.atrial_refractory:
            return True
        return False

    def in_blanking(self, chamber: Chamber, t: int, cycles: TimingCycles) -> bool:
        """True if the sense amplifier for ``chamber`` is blanked at ``t``."""
        if chamber is Chamber.VENTRICLE:
            return t < self.ventricular_blanking_until_ms
        return t < self.atrial_blanking_until_ms

    def blanking_started_by(self, chamber: Chamber, t: int) -> bool:
        """True if the current blanking window for ``chamber`` was started
        by the event at time ``t`` itself (rather than a prior event)."""
        if chamber is Chamber.VENTRICLE:
            return self.ventricular_blanking_start_ms == t
        return self.atrial_blanking_start_ms == t

    def can_sense(self, chamber: Chamber, t: int, cycles: TimingCycles) -> bool:
        """Whether a sense event at ``t`` should be honoured (not blanked)."""
        return not self.in_blanking(chamber, t, cycles)

    def _last_ventricular_time(self) -> int | None:
        v = self.last_ventricular_pace_ms
        s = self.last_ventricular_sense_ms
        if v is None:
            return s
        if s is None:
            return v
        return max(v, s)

    def _last_atrial_time(self) -> int | None:
        p = self.last_atrial_pace_ms
        s = self.last_atrial_sense_ms
        if p is None:
            return s
        if s is None:
            return p
        return max(p, s)

    def apply_ventricular_event(self, t: int, cycles: TimingCycles) -> None:
        """Apply a ventricular-paced or sensed event at time ``t``."""
        self.last_ventricular_sense_ms = t
        self.pvarp_until_ms = t + cycles.pvarp
        self.ventricular_blanking_until_ms = t + cycles.post_ventricular_blanking
        self.ventricular_blanking_start_ms = t

    def apply_ventricular_pace(self, t: int, cycles: TimingCycles) -> None:
        self.apply_ventricular_event(t, cycles)
        self.last_ventricular_pace_ms = t

    def apply_atrial_event(self, t: int, cycles: TimingCycles) -> None:
        self.last_atrial_sense_ms = t
        self.atrial_blanking_until_ms = t + cycles.post_atrial_blanking
        self.atrial_blanking_start_ms = t

    def apply_atrial_pace(self, t: int, cycles: TimingCycles) -> None:
        self.apply_atrial_event(t, cycles)
        self.last_atrial_pace_ms = t


DEFAULT_TIMING = TimingCycles()
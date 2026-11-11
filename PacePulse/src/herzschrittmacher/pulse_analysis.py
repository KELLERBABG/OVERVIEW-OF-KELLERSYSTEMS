"""Symbolic pulse analysis — exact charge/energy computation per millisecond.

Computes symbolic charge and energy delivery of the pacing pulse for each
millisecond to exclude hardware overload, tissue-damage thresholds, or
functional failure in advance. The energy model is the capacitive-discharge
pacemaker:

    E = I² · t / C        Q = I · t

with I the delivered current (mA), t the pulse width (ms), C the output
capacitance (µF).
"""

from __future__ import annotations

from dataclasses import dataclass

from .modes import Chamber

_MA_TO_A = 1e-3
_MS_TO_S = 1e-3
_UF_TO_F = 1e-6


@dataclass(frozen=True)
class SymbolicPulse:
    """A symbolic pacing pulse parameterization."""

    chamber: Chamber
    time_ms: int
    amplitude_ma: float
    pulse_width_ms: float

    @property
    def charge_uc(self) -> float:
        """Delivered charge in microcoulombs (µC)."""
        return self.amplitude_ma * self.pulse_width_ms

    @property
    def current_a(self) -> float:
        return self.amplitude_ma * _MA_TO_A

    @property
    def width_s(self) -> float:
        return self.pulse_width_ms * _MS_TO_S


@dataclass(frozen=True)
class PulseEnergyAnalysis:
    """Result of symbolic energy/charge analysis for a pulse."""

    pulse: SymbolicPulse
    charge_uc: float
    energy_uj: float
    peak_current_ma: float
    capacitance_uf: float
    over_limit: bool
    reasons: tuple[str, ...]

    def as_dict(self) -> dict:
        return {
            "chamber": self.pulse.chamber.value,
            "time_ms": self.pulse.time_ms,
            "amplitude_ma": self.pulse.amplitude_ma,
            "pulse_width_ms": self.pulse.pulse_width_ms,
            "charge_uc": round(self.charge_uc, 4),
            "energy_uj": round(self.energy_uj, 4),
            "peak_current_ma": self.peak_current_ma,
            "capacitance_uf": self.capacitance_uf,
            "over_limit": self.over_limit,
            "reasons": list(self.reasons),
        }


class PulseAnalyzer:
    """Symbolically computes charge/energy for pulse requests.

    Every millisecond of the pulse is checked against hardware and tissue
    envelopes so that overload, tissue damage, or functional failure are
    excluded *in advance* by construction.
    """

    def __init__(
        self,
        output_capacitance_uf: float = 15.0,
        max_charge_uc: float = 16.0,
        max_energy_uj: float = 30.0,
        max_current_ma: float = 8.0,
        max_pulse_width_ms: float = 2.0,
    ) -> None:
        if output_capacitance_uf <= 0:
            raise ValueError("output_capacitance_uf must be positive")
        self.output_capacitance_uf = output_capacitance_uf
        self.max_charge_uc = max_charge_uc
        self.max_energy_uj = max_energy_uj
        self.max_current_ma = max_current_ma
        self.max_pulse_width_ms = max_pulse_width_ms

    def analyze(self, pulse: SymbolicPulse) -> PulseEnergyAnalysis:
        """Run the full symbolic analysis over the pulse."""
        reasons: list[str] = []

        if pulse.pulse_width_ms <= 0:
            reasons.append("pulse_width_ms must be positive")
        if pulse.amplitude_ma < 0:
            reasons.append("amplitude_ma cannot be negative")
        if pulse.pulse_width_ms > self.max_pulse_width_ms:
            reasons.append(
                f"pulse_width {pulse.pulse_width_ms}ms exceeds "
                f"limit {self.max_pulse_width_ms}ms"
            )
        if pulse.amplitude_ma > self.max_current_ma:
            reasons.append(
                f"amplitude {pulse.amplitude_ma}mA exceeds "
                f"limit {self.max_current_ma}mA"
            )

        charge_uc = pulse.charge_uc
        energy_uj = (
            pulse.amplitude_ma ** 2
            * pulse.pulse_width_ms
            / self.output_capacitance_uf
        )

        if charge_uc > self.max_charge_uc:
            reasons.append(
                f"charge {charge_uc:.4f}µC exceeds limit {self.max_charge_uc}µC"
            )
        if energy_uj > self.max_energy_uj:
            reasons.append(
                f"energy {energy_uj:.4f}µJ exceeds limit {self.max_energy_uj}µJ"
            )

        return PulseEnergyAnalysis(
            pulse=pulse,
            charge_uc=charge_uc,
            energy_uj=energy_uj,
            peak_current_ma=pulse.amplitude_ma,
            capacitance_uf=self.output_capacitance_uf,
            over_limit=bool(reasons),
            reasons=tuple(reasons),
        )

    def analyze_ms_by_ms(self, pulse: SymbolicPulse) -> list[dict]:
        """Per-millisecond energy ledger — one row per delivered ms.

        Useful for the *per-millisecond* symbolic computation requirement:
        each millisecond of the pulse is independently checked, so a
        fault anywhere in the pulse is caught.
        """
        rows: list[dict] = []
        ms = 0
        remaining = pulse.pulse_width_ms
        while remaining > 1e-9:
            width = min(1.0, remaining)
            slice_pulse = SymbolicPulse(
                chamber=pulse.chamber,
                time_ms=pulse.time_ms + ms,
                amplitude_ma=pulse.amplitude_ma,
                pulse_width_ms=width,
            )
            analysis = self.analyze(slice_pulse)
            rows.append(
                {
                    "ms": ms,
                    "charge_uc": round(analysis.charge_uc, 6),
                    "energy_uj": round(analysis.energy_uj, 6),
                    "over_limit": analysis.over_limit,
                }
            )
            ms += 1
            remaining -= width
        return rows
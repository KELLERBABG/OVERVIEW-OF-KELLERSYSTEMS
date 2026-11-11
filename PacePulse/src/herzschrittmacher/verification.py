"""Real-time SMT-solver integration (Z3) for pulse-decision verification.

The verification engine checks whether a pacing decision path is consistent
with a *violation* of any lethal invariant. For each invariant ``i`` it asks
Z3:

    satisfiable( path_constraints ∧ ¬invariant_i ) ?

- **SAT**    → there exists an assignment where the pacing engine's
               decision path permits a lethal-invariant violation.
               The pulse is **BLOCKED**.
- **UNSAT**  → no assignment violates the invariant along this path.
               The pulse is **ALLOWED** (conditionally safe).

Only the all-UNSAT case lets a pulse through the trustless execution gate.
"""

from __future__ import annotations

import re
import time
from dataclasses import dataclass, field
from typing import Any

from z3 import (
    And,
    Bool,
    Int,
    Or,
    Real,
    Solver,
    sat,
    unsat,
    unknown,
)

from .constraints import PathConstraint
from .modes import Chamber


@dataclass(frozen=True)
class VerificationResult:
    """Outcome of the SMT check for one pacing decision."""

    safe: bool
    all_invariants_hold: bool
    blocked_invariants: tuple[str, ...] = ()
    sat_models: dict[str, dict[str, Any]] = field(default_factory=dict)
    checks: dict[str, str] = field(default_factory=dict)  # invariant -> sat/unsat/unknown
    runtime_ms: float = 0.0
    detail: str = ""

    def as_dict(self) -> dict:
        return {
            "safe": self.safe,
            "all_invariants_hold": self.all_invariants_hold,
            "blocked_invariants": list(self.blocked_invariants),
            "checks": dict(self.checks),
            "runtime_ms": round(self.runtime_ms, 3),
            "detail": self.detail,
        }


# ---------------------------------------------------------------------------
# Path-constraint parsing into Z3 expressions
# ---------------------------------------------------------------------------

_REFractORY_RE = re.compile(r"^refractory\((\w+)\)=(true|false)$")
_BLANKING_RE = re.compile(r"^blanking\((\w+)\)=(true|false)$")
_CAN_PACE_RE = re.compile(r"^can_pace\((\w+)\)=(true|false)$")
_CAN_SENSE_RE = re.compile(r"^can_sense\((\w+)\)=(true|false)$")
_SENSE_RE = re.compile(r"^sense\((\w+)\)=(true|false)$")
_ELAPSED_RE = re.compile(r"^elapsed_ms=(\d+)$")
_ESCAPE_RE = re.compile(r"^elapsed>=escape_interval=(true|false)$")
_TIME_RE = re.compile(r"^time_ms=(\d+)$")


def _chamber_var_suffix(chamber: str) -> str:
    return "a" if chamber in ("atrium", "atrial") else "v"


class PathEncoder:
    """Encodes a kinetic path constraint into Z3 assertions.

    Symbolic variables created per path:
        refractory_v, blanking_v, can_pace_v, can_sense_v, sense_v : Bool
        elapsed_ms, time_ms                                          : Int
        escape_elapsed                                              : Bool
    """

    def __init__(self) -> None:
        self.vars: dict[str, Any] = {}

    def _bool(self, name: str) -> Bool:
        if name not in self.vars:
            self.vars[name] = Bool(name)
        return self.vars[name]

    def _int(self, name: str) -> Int:
        if name not in self.vars:
            self.vars[name] = Int(name)
        return self.vars[name]

    def encode(self, path: PathConstraint) -> list[Any]:
        """Return a list of Z3 assertions encoding the path components."""
        assertions: list[Any] = []
        for comp in path.components:
            m = _REFractORY_RE.match(comp)
            if m:
                val = m.group(2) == "true"
                assertions.append(
                    self._bool(f"refractory_{_chamber_var_suffix(m.group(1))}") == val
                )
                continue

            m = _BLANKING_RE.match(comp)
            if m:
                val = m.group(2) == "true"
                assertions.append(
                    self._bool(f"blanking_{_chamber_var_suffix(m.group(1))}") == val
                )
                continue

            m = _CAN_PACE_RE.match(comp)
            if m:
                val = m.group(2) == "true"
                assertions.append(
                    self._bool(f"can_pace_{_chamber_var_suffix(m.group(1))}") == val
                )
                continue

            m = _CAN_SENSE_RE.match(comp)
            if m:
                val = m.group(2) == "true"
                assertions.append(
                    self._bool(f"can_sense_{_chamber_var_suffix(m.group(1))}") == val
                )
                continue

            m = _SENSE_RE.match(comp)
            if m:
                val = m.group(2) == "true"
                assertions.append(
                    self._bool(f"sense_{_chamber_var_suffix(m.group(1))}") == val
                )
                continue

            m = _ELAPSED_RE.match(comp)
            if m:
                assertions.append(self._int("elapsed_ms") == int(m.group(1)))
                continue

            m = _TIME_RE.match(comp)
            if m:
                assertions.append(self._int("time_ms") == int(m.group(1)))
                continue

            m = _ESCAPE_RE.match(comp)
            if m:
                val = m.group(1) == "true"
                assertions.append(self._bool("escape_elapsed") == val)
                continue

            # "∞" elapsed — no prior cardiac event; encode as a very large
            # interval so rate invariants trivially hold.
            if comp == "elapsed_ms=∞":
                assertions.append(self._int("elapsed_ms") == 10**9)
                continue

            # Informational components — no SMT semantics.
            continue

        return assertions


# ---------------------------------------------------------------------------
# Verification engine
# ---------------------------------------------------------------------------


@dataclass
class VerificationEngine:
    """Z3-based verification of pacing decisions against lethal invariants."""

    lower_rate_interval_ms: int = 1000
    upper_rate_interval_ms: int = 300
    output_capacitance_uf: float = 15.0
    max_energy_uj: float = 30.0
    max_amplitude_ma: float = 8.0
    max_pulse_width_ms: float = 2.0
    timeout_ms: int = 50

    def check_decision(
        self,
        path: PathConstraint,
        chamber: Chamber,
        amplitude_ma: float,
        pulse_width_ms: float,
    ) -> VerificationResult:
        """Verify a pulse decision; returns the SMT verdict for each invariant.

        Returns ``safe=True`` only if every lethal invariant is UNSAT under
        the path — i.e., no assignment violating safety exists.
        """
        start = time.perf_counter()
        encoder = PathEncoder()
        path_assertions = encoder.encode(path)

        blocked: list[str] = []
        checks: dict[str, str] = {}
        sat_models: dict[str, dict[str, Any]] = {}
        all_hold = True

        # Energy symbolic variables.
        amp = Real("A_ma")
        width = Real("W_ms")
        cap = Real("C_uf")
        energy = amp * amp * width / cap
        energy_assertions = [
            amp == amplitude_ma,
            width == pulse_width_ms,
            cap == self.output_capacitance_uf,
            amp >= 0,
            width >= 0,
        ]

        suffix = _chamber_var_suffix(chamber.value)
        refr_var = encoder._bool(f"refractory_{suffix}")
        blk_var = encoder._bool(f"blanking_{suffix}")
        sense_var = encoder._bool(f"sense_{suffix}")
        elapsed = encoder._int("elapsed_ms")

        invariant_negations: dict[str, Any] = {}

        # 1. no_pulse_in_refractory: path ∧ refractory == True
        invariant_negations["no_pulse_in_refractory"] = And(
            *path_assertions, refr_var == True  # noqa: E712
        )

        # 2. no_pulse_in_blanking: path ∧ blanking == True
        invariant_negations["no_pulse_in_blanking"] = And(
            *path_assertions, blk_var == True  # noqa: E712
        )

        # 3. rate_within_envelope: path ∧ elapsed < lower_rate_interval
        invariant_negations["rate_within_envelope"] = And(
            *path_assertions, elapsed < self.lower_rate_interval_ms
        )

        # 4. pulse_energy_within_envelope: path ∧ energy > max (or amp/width over)
        invariant_negations["pulse_energy_within_envelope"] = And(
            *path_assertions,
            *energy_assertions,
            Or(
                energy > self.max_energy_uj,
                amp > self.max_amplitude_ma,
                width > self.max_pulse_width_ms,
            ),
        )

        # 5. no_sensed_pulse_in_blanking: a sensed event with the target
        #    chamber inside its blanking window drove this decision.
        #    Negation = path ∧ sense ∧ blanking. For a healthy escape-interval
        #    pulse path (blanking=false) this is UNSAT; for a buggy
        #    sense-triggered decision during blanking it is SAT → blocked.
        invariant_negations["no_sensed_pulse_in_blanking"] = And(
            *path_assertions,
            sense_var == True,  # noqa: E712
            blk_var == True,  # noqa: E712
        )

        for name, negation in invariant_negations.items():
            solver = Solver()
            solver.set("timeout", self.timeout_ms)
            solver.add(negation)
            status = solver.check()

            if status == sat:
                checks[name] = "sat"
                all_hold = False
                blocked.append(name)
                sat_models[name] = self._extract_model(solver, encoder)
            elif status == unsat:
                checks[name] = "unsat"
            else:
                checks[name] = "unknown"
                all_hold = False
                blocked.append(name)
                sat_models[name] = {"error": "solver returned unknown"}

        runtime_ms = (time.perf_counter() - start) * 1000.0
        safe = all_hold and not blocked
        detail = (
            "all lethal invariants hold (all UNSAT)"
            if safe
            else f"blocked by invariant(s): {', '.join(blocked)}"
        )
        return VerificationResult(
            safe=safe,
            all_invariants_hold=all_hold,
            blocked_invariants=tuple(blocked),
            sat_models=sat_models,
            checks=checks,
            runtime_ms=runtime_ms,
            detail=detail,
        )

    def check_refractory(self, path: PathConstraint, chamber: Chamber) -> VerificationResult:
        """Convenience: verify only the no-pulse-in-refractory invariant."""
        return self.check_decision(
            path, chamber, amplitude_ma=1.0, pulse_width_ms=0.5
        )

    def verify_symbolic_energy_envelope(
        self,
        chamber: Chamber,
        amplitude_range: tuple[float, float],
        width_range: tuple[float, float],
    ) -> dict[str, Any]:
        """Symbolically search for any amplitude/width inside the envelope
        bounds that would violate the energy invariant along the path.

        This is the *critical state space exploration* facet: instead of a
        single concrete pulse, Z3 searches the whole programmable output
        envelope for a violation.
        """
        amp = Real("A_ma")
        width = Real("W_ms")
        cap = Real("C_uf")
        energy = amp * amp * width / cap

        solver = Solver()
        solver.set("timeout", self.timeout_ms)
        solver.add(amp >= amplitude_range[0], amp <= amplitude_range[1])
        solver.add(width >= width_range[0], width <= width_range[1])
        solver.add(cap == self.output_capacitance_uf)
        solver.add(energy > self.max_energy_uj)

        status = solver.check()
        if status == sat:
            model = solver.model()
            return {
                "envelope_violation": True,
                "amplitude_ma": float(model.eval(amp).as_fraction()),
                "pulse_width_ms": float(model.eval(width).as_fraction()),
                "energy_uj": float(model.eval(energy).as_fraction()),
            }
        if status == unsat:
            return {
                "envelope_violation": False,
                "detail": (
                    "no amplitude/width combination within the programmed "
                    "envelope violates the energy invariant"
                ),
            }
        return {"envelope_violation": None, "detail": "solver returned unknown"}

    @staticmethod
    def _extract_model(solver: Solver, encoder: PathEncoder) -> dict[str, Any]:
        model = solver.model()
        out: dict[str, Any] = {}
        for name, var in encoder.vars.items():
            try:
                val = model.eval(var)
                out[name] = (
                    val.as_fraction().__str__()
                    if hasattr(val, "as_fraction")
                    else str(val)
                )
            except Exception:  # pragma: no cover - defensive
                continue
        return out
# API Reference

Reference for the public API of `herzschrittmacher`.

## Package Exports (`herzschrittmacher/__init__.py`)

```python
from herzschrittmacher import (
    Chamber, PacingMode, Polarity, Response,
    PacingSystem, PacingEvent, PacingDecision,
    VerificationResult, VerificationEngine,
    ExecutionGate,
)
```

## `herzschrittmacher.modes`

### `Chamber(Enum)`
- `Chamber.ATRIUM` = `"atrium"`
- `Chamber.VENTRICLE` = `"ventricle"`

### `CHAMBER_FROM_STRING: dict[str, Chamber]`
Maps `"atrium"`, `"atrial"`, `"a"` → `ATRIUM`; `"ventricle"`,
`"ventricular"`, `"v"` → `VENTRICLE`.

### `Polarity(Enum)` / `Response(Enum)`
NBG classifications: `NONE`, `ATRIUM`, `VENTRICLE`, `DUAL`;
`INHIBITED`, `TRIGGERED`, `DUAL`, `NONE`.

### `PacingMode(Enum)`
`VVI`, `AAI`, `DDD`, `VDD`, `DDDR`, `OFF`.

Properties:
- `paced: Polarity` — chambers that may pace
- `sensed: Polarity` — chambers with sensing enabled
- `response: Response` — NBG response mode
- `rate_adaptive: bool`

Methods:
- `can_pace(chamber: Chamber) -> bool`
- `can_sense(chamber: Chamber) -> bool`

### `TimingCycles` (frozen dataclass)
| Field | Type | Default |
|-------|------|---------|
| `av_delay` | int | 180 |
| `pvarp` | int | 350 |
| `post_ventricular_blanking` | int | 40 |
| `post_atrial_blanking` | int | 30 |
| `lower_rate_interval` | int | 1000 |
| `upper_rate_interval` | int | 300 |
| `ventricular_refractory` | int | 320 |
| `atrial_refractory` | int | 320 |

Class constants: `MIN_AV_DELAY=50`, `MIN_PVARP=150`,
`MAX_PULSE_WIDTH_MS=2`, `MIN_ESCAPE_INTERVAL_MS=150`.
Raises `ValueError` on invalid configurations.

### `CardiacState` (dataclass)
State fields (see `docs/formal_model.md` §4). Key methods:
- `is_refractory(chamber, t, cycles) -> bool`
- `in_blanking(chamber, t, cycles) -> bool`
- `blanking_started_by(chamber, t) -> bool`
- `can_sense(chamber, t, cycles) -> bool`
- `apply_ventricular_event(t, cycles)`
- `apply_ventricular_pace(t, cycles)`
- `apply_atrial_event(t, cycles)`
- `apply_atrial_pace(t, cycles)`

### `DEFAULT_TIMING: TimingCycles`
The default configuration.

## `herzschrittmacher.constraints`

### `PathConstraint` (frozen dataclass)
- `components: tuple[str, ...]`
- `__str__()` → conjunction string (e.g. `"a=x ∧ b=y"`)
- `is_trivial: bool`
- `and_with(*others: str) -> PathConstraint`

### `ConstraintGenerator`
Constructor:
```python
ConstraintGenerator(mode, cycles, state, time_ms)
```
Methods:
- `pulse_path(chamber: Chamber) -> PathConstraint`
- `sense_path(chamber: Chamber) -> PathConstraint`
- `mode_switch_path(target: PacingMode, reason: str) -> PathConstraint`
- `coalesce(*constraints) -> PathConstraint`

## `herzschrittmacher.invariants`

### `InvariantVerdict` (frozen dataclass)
`name: str`, `holds: bool`, `detail: str`

### Lethal invariants
All implement `LethalInvariant` protocol with
`check(state, request, cycles) -> InvariantVerdict`:
- `NoPulseInRefractory`
- `NoPulseInBlanking`
- `RateWithinEnvelope`
- `EnergyWithinEnvelope` (fields: `output_capacitance_uf=15.0`,
  `max_energy_uj=30.0`, `max_pulse_width_ms=2.0`, `max_amplitude_ma=8.0`)
- `NoSensedPulseInBlanking`

### Module functions
- `DEFAULT_INVARIANTS: tuple[LethalInvariant, ...]`
- `check_all_invariants(state, request, cycles,
  invariants=DEFAULT_INVARIANTS) -> tuple[InvariantVerdict, ...]`

## `herzschrittmacher.pacing`

### `PacingEvent` (frozen dataclass)
`chamber: Chamber`, `time_ms: int`, `kind: str` (`"sense"`/`"pace"`),
`tainted: bool = False`

### `PacingDecision` (frozen dataclass)
`action: str` (`"pace"`/`"inhibit"`/`"trigger"`/`"blocked"`),
`chamber: Chamber | None`, `time_ms: int`, `reason: str`,
`path: PathConstraint | None = None`, `verdicts: tuple[InvariantVerdict, ...] = ()`

### `PacingSystem`
Constructor:
```python
PacingSystem(mode=PacingMode.DDD, cycles=DEFAULT_TIMING, state=None)
```
- `start()`, `stop()`
- `sense(chamber, time_ms, tainted=False) -> PacingDecision`
- `check_escape(chamber, time_ms) -> PacingDecision`
- `request_pace(chamber, time_ms, amplitude_ma, pulse_width_ms) -> PacingDecision`
- `events: list[PacingEvent]`
- `state: CardiacState`

Raises `RuntimeError` when not started; `ValueError` on non-monotonic time.

## `herzschrittmacher.verification`

### `VerificationResult` (frozen dataclass)
`safe: bool`, `all_invariants_hold: bool`,
`blocked_invariants: tuple[str, ...] = ()`,
`sat_models: dict[str, dict[str, Any]] = {}`,
`checks: dict[str, str] = {}` (`"sat"`/`"unsat"`/`"unknown"` per invariant),
`runtime_ms: float = 0.0`, `detail: str = ""`
- `as_dict() -> dict`

### `PathEncoder`
- `encode(path: PathConstraint) -> list[Any]` — Z3 assertions
- internal `vars` registry of created Z3 symbols

### `VerificationEngine` (dataclass)
Fields: `lower_rate_interval_ms=1000`, `upper_rate_interval_ms=300`,
`output_capacitance_uf=15.0`, `max_energy_uj=30.0`,
`max_amplitude_ma=8.0`, `max_pulse_width_ms=2.0`, `timeout_ms=50`

Methods:
- `check_decision(path, chamber, amplitude_ma, pulse_width_ms) -> VerificationResult`
- `check_refractory(path, chamber) -> VerificationResult`
- `verify_symbolic_energy_envelope(chamber, amplitude_range, width_range) -> dict`

## `herzschrittmacher.execution_gate`

### `GateResult` (frozen dataclass)
`released: bool`, `reason: str`,
`verification: VerificationResult | None = None`,
`decision: PacingDecision | None = None`
- `as_dict() -> dict`

### `ExecutionGate`
```python
ExecutionGate(engine: VerificationEngine | None = None)
```
- `release(decision, chamber, amplitude_ma, pulse_width_ms) -> GateResult`
- `verify_and_release(path, chamber, amplitude_ma, pulse_width_ms,
  decision=None) -> GateResult`
- `history: tuple[GateResult, ...]`

## `herzschrittmacher.pulse_analysis`

### `SymbolicPulse` (frozen dataclass)
`chamber: Chamber`, `time_ms: int`, `amplitude_ma: float`,
`pulse_width_ms: float`
- `charge_uc: float` (`I·t`)
- `current_a: float`, `width_s: float`

### `PulseEnergyAnalysis` (frozen dataclass)
`pulse`, `charge_uc`, `energy_uj`, `peak_current_ma`, `capacitance_uf`,
`over_limit: bool`, `reasons: tuple[str, ...]`
- `as_dict() -> dict`

### `PulseAnalyzer`
```python
PulseAnalyzer(output_capacitance_uf=15.0, max_charge_uc=16.0,
              max_energy_uj=30.0, max_current_ma=8.0,
              max_pulse_width_ms=2.0)
```
- `analyze(pulse) -> PulseEnergyAnalysis`
- `analyze_ms_by_ms(pulse) -> list[dict]`

## `herzschrittmacher.taint`

### `SensorSource(Enum)`
`EGM_ATRIAL`, `EGM_VENTRICULAR`, `RATE_SENSOR`, `TELEMETRY`,
`IMPLANTED_HISTORY`

### `TaintLabel(Enum)`
`UNTAINTED`, `TAINTED`, `SUSPECT`, `BLOCKED`

### `TaintedValue` (frozen dataclass)
`value: float`, `source: SensorSource`, `label: TaintLabel = TAINTED`,
`origin_trail: tuple[SensorSource, ...] = ()`
- `propagate(new_value=None) -> TaintedValue`

### `TaintAuditRecord` (dataclass)
`value: TaintedValue`, `used_by: str`, `allowed: bool`, `reason: str = ""`

### `SensorStreamAuditor` (dataclass)
Fields: `max_propagation_depth=8`, `telemetry_never_critical=True`,
`plausibility_bounds: dict[SensorSource, tuple[float, float]]`
- `records: tuple[TaintAuditRecord, ...]` (property)
- `audit_for_critical_use(value, used_by) -> TaintAuditRecord`

### `taint_summary(record) -> str`

## `herzschrittmacher.isolation`

### `TaskClass(Enum)`
`CRITICAL`, `VERIFICATION`, `MONITORING`

### `TaskBudget` (dataclass)
`task_class`, `budget_us`, `deadline_us`, `period_us`

### `SchedulabilityResult` (dataclass)
`feasible: bool`, `total_utilization: float`,
`per_class: dict[TaskClass, float]`, `reason: str`

### `IsolationModel` (dataclass)
Fields: `pulse_window_us=250`, `critical_period_us=1000`,
`verification_budget_us=200`, `monitoring_budget_per_sec_us=50_000`,
`switching_cost_us=5`

Methods:
- `critical_utilization() -> float`
- `monitoring_utilization() -> float`
- `can_delay_pulse(task_class) -> bool`
- `schedulability_check() -> SchedulabilityResult`

## `herzschrittmacher.cli`

### `main(argv=None) -> int`
CLI entrypoint. Subcommands: `demo`, `verify`, `verify-symbolic`.

## `herzschrittmacher.__main__`

Enables `python -m herzschrittmacher …`.
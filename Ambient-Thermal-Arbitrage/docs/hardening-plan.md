# Hardening Plan — Based on report.json Vulnerabilities

## Vulnerability Analysis Summary

### 1. Arithmetic Overflow Detection (Add) — AUTO-GENERATED serde code (bulk of reports)
**Affected**: ~50+ blocks in serde-derived `Serialize` impls for:
- `api::SystemStatus`, `config::*`, `control::ControlDecision`, `data_ingestion::*`
- `financial::*`, `simulation_data::HourlyDataPoint`, `thermal::ThermalState`

**Root cause**: Serde's derive macros increment internal counters (`+1` on `usize`). The analyzer flags these as potential overflow paths. These are **false positives** since serde always writes to Vec slots that exist — but we should still enable runtime overflow detection.

### 2. Integer Overflow in BUSINESS LOGIC — REAL issues to fix
- `hour_of_year()`: `dt.ordinal() - 1` can underflow if ordinal is 0
- `sunshine_probability()`: `month - 1` when month = 0 → underflow
- `SimulationEngine::step()`: `self.simulated_hour += ...` — f64 could overflow
- `record_hour()`: `self.hours_tracked += 1` — u64 overflow after ~10¹⁹ hours
- Various: `hour_idx + 1` without ensuring ≤ len

### 3. Division/Remainder by Zero — false positives on literal divisors
- `x % 24`, `x / 24`, `x % 8760`, `x / 3600` — divisors are non-zero literals
- **Fix**: Use `checked_rem` / `checked_div` for formal safety

### 4. Slice Boundary Analysis — REAL bug
- `daily_temp_base` closure: safe due to `rem_euclid(12)`, but no explicit `get()` guard
- `sunshine_probability(idx)`: REAL bug if month is 0 → underflow on `(month - 1)`

## Hardening Steps

1. ✅ Enable overflow checks at compile time + runtime
2. ✅ Fix underflow bugs in `sunshine_probability` and `hour_of_year`
3. ✅ Add defensive slice bounds checking with `.get()` fallbacks
4. ✅ Use saturating/checked arithmetic for business logic counters
5. ✅ Configure release profile with overflow checks
6. ✅ Add clippy lints for arithmetic safety
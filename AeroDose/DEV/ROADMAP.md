# AeroDose — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** The project has a working audit hash-chain (Ed25519 + SHA-256), a compliance report generator, and a dose tracker skeleton. Missing are the thermodynamic drift compensation engine, taint-coded sensor stream validation, non-disableable haptic emergency veto state machine, full CLI wiring, and integration test coverage.

## Task Queue

- [x] **TASK-01: Implement thermodynamic drift compensation module**
  - **Target Files:** `src/thermo.rs`
  - **Context Files:** `Cargo.toml`, `src/sensor.rs`, `src/lib.rs`
  - **Verify:** `cargo check --quiet`
  - **Spec:** Create `src/thermo.rs` implementing hygroscopic growth correction for Sensirion SPS30 optical particle counts.

**Structs:**
```rust
#[derive(Debug, Clone, Copy)]
pub struct AmbientConditions {
    pub temperature_c: f64,   // °C, range -20..60
    pub relative_humidity_pct: f64, // 0.0..100.0
}

#[derive(Debug, Clone, Copy)]
pub struct CorrectedReading {
    pub pm1_0_ug_m3: f64,
    pub pm2_5_ug_m3: f64,
    pub pm4_0_ug_m3: f64,
    pub pm10_ug_m3: f64,
    pub particle_count_p_cc: f64,
}

#[derive(Debug, Clone)]
pub struct ThermoCompensator;
```

**Functions on `ThermoCompensator`:**
- `pub fn new() -> Self` — unit struct, no state.
- `pub fn correct(&self, raw: &CorrectedReading, ambient: &AmbientConditions) -> CorrectedReading` — applies a multiplicative hygroscopic growth factor to mass concentrations. The growth factor is computed as:
  - `g = 1.0 + 0.0045 * (rh - 50.0).max(0.0)` where rh is relative_humidity_pct. This models the well-known ~0.45%/RH-point mass inflation above 50% RH for NaCl-like aerosols.
  - Temperature correction: `t_factor = 1.0 + 0.002 * (25.0 - temperature_c)` — compensates for laser diode power drift (SPS30 datasheet specifies ~0.2%/°C around 25 °C reference).
  - Final corrected value = raw_value / (g * t_factor). Clamp all outputs to >= 0.0.
- `pub fn validate_ambient(&self, ambient: &AmbientConditions) -> Result<(), ThermoError>` — returns Err if temperature outside [-20, 60] or RH outside [0, 100].

**Error type:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum ThermoError {
    #[error("temperature {0}°C out of sensor range [-20, 60]")]
    TemperatureOutOfRange(f64),
    #[error("relative humidity {0}% out of range [0, 100]")]
    HumidityOutOfRange(f64),
}
```

**Unit tests (in `#[cfg(test)] mod tests`):**
- Test that at 25 °C / 50% RH the corrected reading equals the raw reading exactly.
- Test that at 80% RH the PM2.5 value is reduced by approximately the expected factor (1/(1+0.0045*30) ≈ 0.847).
- Test that out-of-range ambient returns Err.
- Test that negative raw values are clamped to 0.0 in output.

Add `pub mod thermo;` to `src/lib.rs`.
  - **Telemetry:** `20.37 tok/s` (qwen3.8-coder:fast)

- [x] **TASK-02: Implement taint-coded sensor stream audit module**
  - **Target Files:** `src/taint.rs`
  - **Context Files:** `Cargo.toml`, `src/sensor.rs`, `src/lib.rs`
  - **Verify:** `cargo check --quiet`
  - **Spec:** Create `src/taint.rs` implementing continuous data-integrity validation of the raw sensor stream to detect clogged intake, stuck optical window, or spoofed telemetry.

**Enums and structs:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaintFlag {
    Clean,
    CloggedIntake,
    StuckOpticalWindow,
    SpoofedTelemetry,
}

#[derive(Debug, Clone)]
pub struct TaintDetector {
    window_size: usize,
    recent_readings: Vec<f64>, // rolling PM2.5 values
    min_variance_threshold: f64,
    max_jump_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct TaintReport {
    pub flag: TaintFlag,
    pub window_stddev: f64,
    pub max_consecutive_equal: usize,
    pub max_single_step_jump_pct: f64,
}
```

**Functions on `TaintDetector`:**
- `pub fn new(window_size: usize) -> Self` — sets `min_variance_threshold = 0.5` (µg/m³), `max_jump_threshold = 200.0` (%). The window must be >= 4.
- `pub fn push_sample(&mut self, pm25_ug_m3: f64)` — appends to rolling window, evicts oldest if len > window_size.
- `pub fn evaluate(&self) -> TaintReport` — computes:
  - `window_stddev`: population standard deviation of the window. If stddev < min_variance_threshold AND window is full → `StuckOpticalWindow`.
  - `max_consecutive_equal`: longest run of identical values (exact f64 equality). If >= window_size/2 → `CloggedIntake`.
  - `max_single_step_jump_pct`: max absolute percentage change between consecutive samples in the window. If > max_jump_threshold → `SpoofedTelemetry`.
  - Priority: SpoofedTelemetry > CloggedIntake > StuckOpticalWindow > Clean.
- `pub fn reset(&mut self)` — clears the window.

**Unit tests:**
- Feed 10 identical values → expect `StuckOpticalWindow` or `CloggedIntake`.
- Feed a sequence with one 500% jump → expect `SpoofedTelemetry`.
- Feed a noisy realistic sequence (e.g., 12.3, 14.1, 13.8, 15.2, ...) → expect `Clean`.
- Test that `reset()` clears state.

Add `pub mod taint;` to `src/lib.rs`.
  - **Telemetry:** `19.95 tok/s` (qwen3.8-coder:fast)

- [x] **TASK-03: Implement non-disableable haptic emergency veto state machine**
  - **Target Files:** `src/veto.rs`
  - **Context Files:** `Cargo.toml`, `src/dose.rs`, `src/lib.rs`
  - **Verify:** `cargo check --quiet`
  - **Spec:** Create `src/veto.rs` implementing the autonomous safety mechanism that enforces haptic + piezo alerts at critical toxic thresholds and CANNOT be muted by the wearer.

**Enums and structs:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VetoState {
    Nominal,
    Advisory,      // TWA approaching limit
    Mandatory,     // TWA exceeded — alert cannot be dismissed
    EmergencyVeto, // Acute spike — maximum intensity, no software override
}

#[derive(Debug, Clone)]
pub struct HapticVeto {
    state: VetoState,
    /// Peak PM2.5 in the current acute window (last 60 s)
    acute_peak_ug_m3: f64,
    /// Running TWA in µg/m³
    twa_ug_m3: f64,
    /// OSHA 8-hr PEL for PM2.5 (µg/m³) — configurable
    pel_ug_m3: f64,
    /// Acute spike threshold (µg/m³) — triggers EmergencyVeto
    acute_spike_threshold_ug_m3: f64,
    /// Whether the wearer attempted to mute (logged, never acted on)
    mute_attempts: u32,
}
```

**Functions:**
- `pub fn new(pel_ug_m3: f64, acute_spike_threshold_ug_m3: f64) -> Self` — defaults state to Nominal.
- `pub fn update(&mut self, current_pm25: f64, twa: f64)` — transitions state:
  - If `current_pm25 >= acute_spike_threshold_ug_m3` → `EmergencyVeto` (irreversible until PM drops below 0.5× threshold for 3 consecutive calls — track with a counter).
  - Else if `twa >= pel_ug_m3` → `Mandatory`.
  - Else if `twa >= 0.8 * pel_ug_m3` → `Advisory`.
  - Else → `Nominal` (can only downgrade from Advisory, never from Mandatory/EmergencyVeto without the acute recovery condition).
- `pub fn attempt_mute(&mut self) -> bool` — increments `mute_attempts`, ALWAYS returns `false` (mute is never honored). This is the invariant: the function exists to log intent but never change state.
- `pub fn current_state(&self) -> VetoState`
- `pub fn mute_attempt_count(&self) -> u32`
- `pub fn haptic_intensity(&self) -> f64` — returns 0.0 (Nominal), 0.3 (Advisory), 0.7 (Mandatory), 1.0 (EmergencyVeto).

**Unit tests:**
- Start Nominal, push TWA at 90% PEL → Advisory.
- Push TWA at 110% PEL → Mandatory.
- Push PM spike above acute threshold → EmergencyVeto.
- Call `attempt_mute()` in EmergencyVeto → state unchanged, returns false, count increments.
- After 3 calls below 0.5× threshold → transitions out of EmergencyVeto to Mandatory (if TWA still > PEL).

Add `pub mod veto;` to `src/lib.rs`.
  - **Telemetry:** `20.19 tok/s` (qwen3.8-coder:fast)

- [x] **TASK-04: Wire thermo, taint, and veto modules into lib.rs with integration tests**
  - **Target Files:** `src/lib.rs`
  - **Context Files:** `src/thermo.rs`, `src/taint.rs`, `src/veto.rs`, `src/dose.rs`, `src/audit.rs`, `src/compliance.rs`
  - **Verify:** `cargo check --quiet`
  - **Spec:** Update `src/lib.rs` to:

1. Ensure all public modules are declared: `pub mod audit; pub mod compliance; pub mod dose; pub mod sensor; pub mod thermo; pub mod taint; pub mod veto;`
2. Add a top-level integration test module `#[cfg(test)] mod integration_tests` that exercises the full pipeline:

**Test: `full_shift_pipeline_with_thermo_and_taint`:**
- Create a `ThermoCompensator`, feed a raw reading at 35 °C / 70% RH, verify corrected PM2.5 < raw PM2.5.
- Create a `TaintDetector` with window_size=10, push 10 corrected readings that are realistic (varying by ±2 µg/m³), assert `evaluate().flag == TaintFlag::Clean`.
- Then push 8 identical values, assert flag is no longer Clean.

**Test: `veto_escalation_and_mute_invariant`:**
- Create `HapticVeto::new(50.0, 300.0)` (PEL=50, spike=300).
- Update with twa=45 → Advisory.
- Update with twa=55 → Mandatory.
- Update with pm25=350 → EmergencyVeto.
- Call `attempt_mute()` 5 times → state still EmergencyVeto, `mute_attempt_count() == 5`.

**Test: `audit_chain_survives_thermo_correction`:**
- Create an `AuditLog` with a fresh `SigningKey` (use `ed25519_dalek::SigningKey::generate(&mut rand::thread_rng())`).
- Append 3 entries using thermo-corrected values.
- Call `verify_chain` with the corresponding `VerifyingKey` → Ok.

Keep all existing module declarations intact. Do not modify any other source files.
  - **Telemetry:** `19.94 tok/s` (qwen3.8-coder:fast)

- [ ] **TASK-05: Complete CLI with subcommands for dose, audit, compliance, and veto simulation**
  - **Target Files:** `src/main.rs`
  - **Context Files:** `Cargo.toml`, `src/lib.rs`, `src/dose.rs`, `src/audit.rs`, `src/compliance.rs`, `src/veto.rs`
  - **Verify:** `cargo check --quiet`
  - **Spec:** Rewrite `src/main.rs` as a full CLI using `clap` derive API.

**Top-level struct:**
```rust
#[derive(clap::Parser)]
#[command(name = "smokeshift-cli", version, about = "AeroDose occupational PM2.5 shift dosimeter")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Calculate cumulative shift dose from a JSON sample file
    Dose {
        /// Path to JSON file containing an array of {"t": seconds, "pm25": f64} objects
        #[arg(short, long)]
        input: std::path::PathBuf,
        /// OSHA PEL in µg/m³ (default 50)
        #[arg(short = 'p', long, default_value_t = 50.0)]
        pel: f64,
    },
    /// Verify an audit log JSON file for chain integrity
    Audit {
        /// Path to JSON file containing the serialized audit entries array
        #[arg(short, long)]
        input: std::path::PathBuf,
        /// Hex-encoded Ed25519 verifying public key (32 bytes)
        #[arg(short, long)]
        pubkey_hex: String,
    },
    /// Generate a compliance report from a completed shift
    Report {
        /// Path to JSON sample file (same format as Dose)
        #[arg(short, long)]
        input: std::path::PathBuf,
        /// Worker pseudonym UUID
        #[arg(short, long)]
        worker_id: uuid::Uuid,
        /// Output path for the JSON report (default: stdout)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Simulate haptic veto state transitions from a sample stream
    Veto {
        /// Path to JSON sample file
        #[arg(short, long)]
        input: std::path::PathBuf,
        /// Acute spike threshold µg/m³ (default 300)
        #[arg(short = 's', long, default_value_t = 300.0)]
        spike: f64,
    },
}
```

**Implementation:**
- `Dose`: Read JSON file as `Vec<Sample>` where `Sample { t: u64, pm25: f64 }`. Use `ShiftDoseTracker` (from `crate::dose`) to integrate. Print a summary table to stdout: total seconds, TWA, peak, cumulative dose, final alert tier.
- `Audit`: Read JSON as `Vec<AuditEntry>`. Reconstruct the hash chain manually (recompute each entry's SHA-256 from its fields + previous_hash), verify each Ed25519 signature against `pubkey_hex`. Print PASS/FAIL per entry and overall verdict.
- `Report`: Same input parsing as Dose. Build a `ShiftDoseTracker`, run it, create an `AuditLog` (generate ephemeral key), append entries at each sample, then call `ComplianceEngine::generate_report`. Serialize the report to JSON and write to output path or stdout.
- `Veto`: Parse samples, feed into `HapticVeto`, print state transitions as they occur (only print when state changes).

**Error handling:** Use `Box<dyn std::error::Error>` for main return. Print errors to stderr with `eprintln!` and return non-zero exit code.

Ensure `cargo check --quiet` passes with no warnings.

- [ ] **TASK-06: Add property-based edge-case tests for dose integration and audit chain**
  - **Target Files:** `src/dose.rs`, `src/audit.rs`
  - **Context Files:** `Cargo.toml`, `src/lib.rs`
  - **Verify:** `cargo test --quiet`
  - **Spec:** Append a `#[cfg(test)] mod edge_tests` block to BOTH `src/dose.rs` and `src/audit.rs`. Do not modify any existing function signatures or logic — only add test modules.

**In `src/dose.rs` edge tests:**
- `test_zero_duration_no_panic`: Create a tracker, call `twa_pm25_ug_m3()` with zero samples → must return 0.0, not panic or divide by zero.
- `test_single_sample_twa_equals_value`: One sample of 42.0 µg/m³ at t=60s → TWA should be 42.0 (or very close, within 1e-9).
- `test_monotonic_dose_accumulation`: Feed 100 samples with increasing PM2.5 (1.0, 2.0, ..., 100.0) at 60s intervals. Assert that `cumulative_dose_ug_h_m3` is strictly monotonically non-decreasing after each push.
- `test_extreme_concentration`: Feed a single sample of 1e6 µg/m³ (physically impossible but tests numeric stability). Assert no NaN, no infinity, dose > 0.
- `test_negative_pm25_clamped`: If the tracker accepts negative input (or if you can push -5.0), assert the resulting TWA and dose are >= 0.0.

**In `src/audit.rs` edge tests:**
- `test_empty_chain_verifies`: Create an AuditLog, call `verify_chain` with a valid key → Ok (empty chain is trivially valid).
- `test_single_entry_chain`: Append one entry, verify → Ok.
- `test_tampered_hash_detected`: Append 3 entries. Manually mutate `entries[1].pm25_ug_m3` to a different value (you'll need a way to access — if entries is private, add a `pub fn entries(&self) -> &[AuditEntry]` accessor). Verify → must return Err.
- `test_tampered_signature_detected`: Append 2 entries. Mutate `entries[1].signature_hex` to a different valid hex string of same length. Verify → Err.
- `test_sequence_continuity`: Append 5 entries, assert `entries[i].sequence == i as u64` for all i, and `entries[i].previous_hash == entries[i-1].entry_hash` for i > 0.

If `entries` field is private in `AuditLog`, add a public accessor: `pub fn entries(&self) -> &[AuditEntry] { &self.entries }`. This is the only non-test code change allowed.

Ensure `cargo check --quiet` and `cargo test --quiet` both pass.

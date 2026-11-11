# ABOS — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 13-crate Rust SDR workspace implementing OFDM/DSSS/FHSS physical layers, LDPC forward error correction, DTN bundle store-and-forward, and simulated NVIS skywave channels. While DSP, FEC, and protocol crates pass tests, the ionospheric sounding modules (`fo_f2.rs`, `meteor.rs`, `nvis.rs`, `propagation.rs`) and cognitive jammer detection (`jammer_detect.rs`) are thin 3–9 line stubs, and hardware SDR surfaces remain mock-only.

## Task Queue

- [x] **TASK-01: Implement Appleton-Hartree NVIS propagation & critical frequency foF2 solver**
  - **Target Files:** `abos-iono/src/fo_f2.rs`, `abos-iono/src/nvis.rs`, `abos-iono/src/propagation.rs`
  - **Context Files:** `abos-iono/src/lib.rs`, `abos-iono/src/sounder.rs`, `WHITEPAPER.md`
  - **Verify:** `cargo test -p abos-iono --quiet`
  - **Spec:** Replace the stubs in `abos-iono/src/fo_f2.rs`, `nvis.rs`, and `propagation.rs` with deterministic NVIS skywave propagation physics.

1. In `fo_f2.rs`, implement `CriticalFrequencyModel` computing plasma frequency $f_p = 8.98 \sqrt{N_e}$ (Hz) from electron density $N_e$ (m^-3) and maximum usable frequency $f_{MUF}(\theta_i) = f_oF2 \cdot \sec(\theta_i)$ for incidence angle $\theta_i \in [0, 25^\circ]$.
2. In `nvis.rs`, implement `NvisLinkBudget` calculating virtual reflection height $h' \in [200, 400] km, ground coverage radius, free-space path loss $L_{FSPL}$, and D-layer absorption loss as a function of solar zenith angle.
3. In `propagation.rs`, implement `MultipathDelayProfile` modeling ordinary (O-mode) and extraordinary (X-mode) magneto-ionic splitting delay spread. Note on test parameters: For tests calculating delay spread, ensure carrier frequency $f > f_p$ (e.g., $N_e = 10^{11} m^{-3} \implies f_p \approx 2.84 MHz$ tested at $f = 5.0 MHz$ and $f = 8.0 MHz$, or carrier $f = 12 MHz$ if $f_p \approx 9 MHz$) so that the wave actually propagates without evanescent clamping to zero.
4. Add unit tests verifying that frequencies above $f_{MUF}$ penetrate (no reflection) while frequencies in $[0.65 f_oF2, 0.95 f_oF2]$ achieve optimal NVIS return, and that propagation delay spread tests use carrier frequencies above plasma cutoff.
  - **Telemetry:** `19.93 tok/s` (qwen3.8-coder:fast)

- [ ] **TASK-02: Implement meteor-scatter ionization trail burst window estimator**
  - **Target Files:** `abos-iono/src/meteor.rs`
  - **Context Files:** `abos-iono/src/lib.rs`, `abos-protocol/src/schedule.rs`
  - **Verify:** `cargo test -p abos-iono --quiet`
  - **Spec:** Expand `abos-iono/src/meteor.rs` (currently 9 lines) to model underdense and overdense meteor-scatter trails at $85\text{--}105\text{ km}$ altitude.

1. Define `MeteorTrail { line_density_q: f64, altitude_km: f64, ambipolar_diffusion_d: f64, carrier_freq_hz: f64, scatter_angle_rad: f64 }`.
2. Compute decay time constant $\tau = \frac{\lambda^2}{16 \pi^2 D \sin^2(\phi/2)}$ (typically $50\text{--}500\text{ ms}$) and peak received SNR.
3. Provide `usable_burst_window_ms(&self, min_snr_db: f64) -> f64` returning the millisecond handshake window for high-priority shard synchronization up to $2000\text{ km}$.
4. Include unit tests for both underdense ($q < 10^{14}\text{ e/m}$) and overdense ($q > 10^{14}\text{ e/m}$) trails.

- [x] **TASK-03: Implement cognitive radio jammer classification and whitespace hopping**
  - **Target Files:** `abos-cognitive/src/jammer_detect.rs`, `abos-cognitive/src/whitespace.rs`, `abos-cognitive/src/adapt.rs`
  - **Context Files:** `abos-cognitive/src/scanner.rs`, `abos-cognitive/src/lib.rs`
  - **Verify:** `cargo test -p abos-cognitive --quiet`
  - **Spec:** Replace the 7-line stub in `abos-cognitive/src/jammer_detect.rs` and expand `whitespace.rs` and `adapt.rs`.

1. In `jammer_detect.rs`, implement `JammerClassifier` detecting Continuous Wave (CW) tone jammers, swept-chirp jammers, and barrage noise jammers from power spectral density (PSD) bins using peak-to-average power ratio (PAPR) and spectral entropy.
2. In `whitespace.rs`, implement `WhitespaceFinder` that ranks contiguous clean sub-bands in the $2\text{--}10\text{ MHz}$ NVIS spectrum by lowest interference temperature.
3. In `adapt.rs`, wire `CognitiveController` to automatically switch carrier frequency and spreading factor when a jammer is detected.
4. Add unit tests proving automatic evasion of simulated tone and sweep jammers.
  - **Telemetry:** `20.55 tok/s` (qwen3.8-coder:fast)

- [ ] **TASK-04: Harden cyclostationary feature masking and sub-noise-floor stealth invariants**
  - **Target Files:** `abos-stealth/src/mask_cyclo.rs`, `abos-stealth/src/amp_dither.rs`, `abos-stealth/src/phase_noise.rs`
  - **Context Files:** `abos-stealth/src/lib.rs`, `abos-phy/src/dsss.rs`
  - **Verify:** `cargo test -p abos-stealth --quiet`
  - **Spec:** Expand `abos-stealth` modules so DSSS/OFDM frames erase cyclic autocorrelation peaks.

1. In `mask_cyclo.rs`, implement cyclic prefix jitter and symbol-rate randomization so the spectral correlation function $S_x^\alpha(f)$ exhibits no detectable peak at chip or symbol rate $\alpha = 1/T_c$.
2. In `amp_dither.rs` and `phase_noise.rs`, add bounded Gaussian amplitude dithering and continuous phase perturbation that preserves coherent demodulation for holders of the shared PRNG seed while flattening fourth-order cumulants ($C_{42}$) for blind interceptors.
3. Add unit tests verifying that the masked signal passes a Kolmogorov-Smirnov Gaussianity check.

- [ ] **TASK-05: Couple NVIS elevation beamforming to ionospheric sounder feedback**
  - **Target Files:** `abos-phy/src/beamforming.rs`
  - **Context Files:** `abos-phy/src/lib.rs`, `abos-iono/src/sounder.rs`
  - **Verify:** `cargo test -p abos-phy --quiet`
  - **Spec:** Enhance `abos-phy/src/beamforming.rs` to compute complex steering weights $w_n = a_n e^{-j k (d_x \sin\theta \cos\phi + d_y \sin\theta \sin\phi)}$ for planar NVIS dipole arrays.

1. Implement null-steering (MVDR / Frost projection) to place a spatial zero toward ground-wave interceptors ($\theta \approx 90^\circ$ horizon) while maximizing zenith gain ($\theta \in [0^\circ, 20^\circ]$).
2. Add unit tests confirming $>20\text{ dB}$ horizon suppression relative to zenith main lobe.

- [x] **TASK-06: End-to-end asynchronous buffer-bounce DTN mesh verification**
  - **Target Files:** `abos-protocol/src/bounce.rs`, `abos-tests/tests/e2e_roundtrip.rs`
  - **Context Files:** `abos-protocol/src/mesh.rs`, `abos-protocol/src/shard.rs`, `abos-tests/src/channel.rs`
  - **Verify:** `cargo test -p abos-tests --quiet`
  - **Spec:** Extend `abos-protocol/src/bounce.rs` and `abos-tests/tests/e2e_roundtrip.rs` to verify asynchronous skywave buffer-bounce relay across 3 disconnected nodes under dynamic $f_oF2$ fading and simulated jammer injection.

1. Verify that Reed-Solomon / LDPC shards transmitted during open NVIS windows are buffered by intermediate relay nodes and reconstructed bit-exact at the destination.
  - **Telemetry:** `19.3 tok/s` (qwen3.8-coder:fast)

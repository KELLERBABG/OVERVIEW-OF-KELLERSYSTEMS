# Ambient Thermal Arbitrage (ATA)

**Version 0.2.0 — Production Pilot (Phase 2)**

Ambient Thermal Arbitrage transforms any temperature-controlled facility into a virtual battery that participates in energy arbitrage, frequency regulation, and demand-side management — without compromising product integrity.

The repository contains two Rust crates:

| Crate | Directory | Purpose |
|---|---|---|
| `thermal-arbitrage-mvp` | `crates/gateway/` | Single-store production pilot gateway (v0.2.0) |
| `thermal-fleet` | `crates/fleet/` | Fleet simulation engine across 21 facility types (v2.0.0) |

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Repository Layout](#repository-layout)
3. [Building & Running](#building--running)
4. [Configuration](#configuration)
5. [Control Engine](#control-engine)
6. [HTTP API](#http-api)
7. [Security](#security)
8. [Modbus PLC Interface](#modbus-plc-interface)
9. [Alerts](#alerts)
10. [OTA Firmware Updates](#ota-firmware-updates)
11. [Live Data Pipeline](#live-data-pipeline)
12. [Metrics & Observability](#metrics--observability)
13. [Fleet Simulation](#fleet-simulation)
14. [Testing](#testing)
15. [Roadmap](#roadmap)

---

## System Overview

The ATA system is a control software layer that activates the latent thermal storage capacity of refrigeration infrastructure:

1. **Modulates** refrigeration compressors — continuous VFD speed control is modeled in the fleet engine and planned for the gateway (Phase 2.5); the production pilot gateway currently commands **on/off** with min-runtime/min-offtime short-cycling protection, replacing wasteful rapid thermostat cycling.
2. **Shifts** cooling energy consumption from high-price to low-price hours using Model Predictive Control (MPC) with a 12-hour lookahead.
3. **Participates** in grid balancing using thermal inertia as a fast-responding resource.
4. **Preserves** product integrity through a multi-layer safety cascade.

The production pilot gateway runs the real-time control loop with live market/weather data, a secure HTTP API, Modbus PLC connectivity, alerting, OTA updates, and Prometheus metrics. The fleet engine simulates thousands of stores across 21 facility types through a full 8760-hour year of NRW 2025 data.

## Repository Layout

```
thermal-arbitrage-mvp/
├── Cargo.toml              # Main crate manifest (v0.2.0)
├── config.toml             # Runtime configuration
├── src/
│   ├── main.rs             # Gateway entrypoint: control loop + subsystems
│   ├── lib.rs              # Module declarations
│   ├── config.rs           # Configuration structs & defaults
│   ├── control/mod.rs      # 6-mode control engine + MPC + ASCT governor
│   ├── thermal/mod.rs      # RC thermal model, dynamic COP, RLS estimator
│   ├── sensor/mod.rs       # Sensor abstraction + simulated sensors
│   ├── financial/mod.rs    # Baseline vs. actual financial tracking
│   ├── api/mod.rs          # Axum HTTP API (status, health, metrics, config)
│   ├── security.rs         # HMAC-SHA256 bearer token authentication
│   ├── modbus_client.rs    # Native Modbus TCP client (PLC interface)
│   ├── alerts.rs           # Webhook alert dispatcher with retry
│   ├── ota.rs              # OTA firmware update (Ed25519 + SHA-256)
│   ├── live_data.rs        # Real-time price/weather ingestion
│   ├── metrics.rs          # Prometheus metrics registry
│   ├── logging/mod.rs      # CSV logging
│   ├── simulator/mod.rs    # 8760-hour simulation dashboard
│   ├── simulation_data/    # Synthetic fallback data generator
│   └── data_ingestion/     # Real-world data fetch + caching
├── standalone-fleet/       # Fleet simulation engine (v2.0.0)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Fleet simulator entrypoint
│       ├── store_types.rs  # 21 facility type definitions
│       ├── physics.rs      # RC, RLS, COP, defrost, stratification
│       ├── control.rs      # 7-mode priority cascade
│       ├── simulation.rs   # 8760-hour NRW 2025 dataset
│       └── financial.rs    # Per-store financial tracking
└── data_cache/             # Cached real-world datasets (offline support)
```

## Building & Running

### Single-Store Production Pilot

```bash
cargo build --release
cargo run --release
```

The gateway starts the control loop, HTTP API, and any enabled subsystems. In simulation mode it runs the 8760-hour NRW 2025 scenario at accelerated speed. In live-data mode (`live_data.enabled = true`) it uses real prices and weather.

> **Real-data self-test:** `cargo run --release -- --validate-live` probes every
> live-data source (ENTSO-E → energy-charts → SMARD → cache, Open-Meteo weather,
> frequency URL) through the real fetch + parse code and prints a JSON report,
> then exits 0 when at least one price source and the weather source succeeded.
> Set `ENTSOE_API_KEY` to include the official ENTSO-E path in the test.

> **Windows build note:** the `ring`/`aws-lc` crypto dependencies need a C compiler.
> On Windows, either use the MSVC toolchain **with the Windows SDK installed**, or
> install [MSYS2](https://www.msys2.org) + `mingw-w64-x86_64-gcc` and add
> `C:\msys64\mingw64\bin` to `PATH`. The GNU `dlltool` also breaks on paths
> containing spaces — build with a space-free target dir:
>
> ```bash
> export PATH="/c/msys64/mingw64/bin:$PATH"
> export CARGO_TARGET_DIR='C:/ata-target'
> cargo +nightly-x86_64-pc-windows-gnu test --quiet
> ```
>
> A ready-made wrapper exists at `thermal-arbitrage-mvp/scripts/build-windows.bat`
> (requires MSYS2 gcc on `PATH`). The canonical build/test
> verification is the GitHub Actions CI (Linux).

### Fleet Simulation

```bash
cd standalone-fleet
cargo run --release -- "400 aldi + 5 edeka + 2 marktkauf"
cargo run --release -- "10 datacenter + 1 chemistry + 1 industry"
cargo run --release -- --volatile "400 aldi + 5 edeka"   # unlocked research mode
```

Outputs a results table, per-store economics, the 50% revenue-share slice, a bar chart, and a timestamped CSV (`fleet_results_v2_*.csv`).

## Configuration

All parameters are externalized in `config.toml` (or via `Config::default()` when the file is absent/incomplete). The full configuration surface:

| Section | Key Parameters | Description |
|---|---|---|
| `[system]` | `unit_id`, `loop_interval_sec` | System identity and control loop rate |
| `[thermal]` | `r_th`, `c_th`, `t_min`, `t_max`, `t_setpoint_*` | Thermal model parameters |
| `[control]` | `thermal_integral_max`, `ttf_critical_sec`, `reaction_time_sec`, `droop_gain_k`, `freq_nominal` | Safety and grid tuning |
| `[control]` | `min_runtime_sec`, `min_offtime_sec`, `grid_balancing_deadband_hz`, `mpc_cheap_threshold_eur_per_mwh`, `vfd_enabled`, `vfd_oil_return_pulse_*`, `mpc_intraday_deviation_threshold_eur_per_mwh` | ASCT governor, grid, MPC, VFD modulation + oil-return pulse + intraday fast-path |
| `[sensor]` | `simulate`, `ambient_temp_c`, `initial_temp_c`, `cooling_power_w`, `heat_leak_w` | Sensor/simulation parameters |
| `[building]` | `name`, `volume_m3`, `surface_area_m2`, `u_value_w_per_m2k`, `product_mass_kg`, `product_specific_heat_j_per_kgk`, `cop`, `installed_cooling_power_w`, `air_ach_per_hour` | Building physics model |
| `[logging]` | `csv_path`, `interval_sec` | CSV data logging |
| `[api]` | `bind_address`, `tls_cert_path`, `tls_key_path` | HTTP API server (TLS optional) |
| `[security]` | `api_secret`, `auth_enabled` | API authentication (see note below) |
| `[modbus]` | `enabled`, `host`, `port`, `poll_interval_sec`, `registers.*` (incl. optional `compressor_speed_addr`) | Modbus TCP PLC interface (VFD speed register supported) |
| `[alerts]` | `enabled`, `webhook_url`, `max_retries`, `retry_backoff_sec` | Alert webhook dispatch |
| `[ota]` | `enabled`, `manifest_url`, `verify_key_hex`, `check_interval_sec`, `staging_dir` | OTA firmware updates |
| `[live_data]` | `enabled`, `price_refresh_sec`, `weather_refresh_sec`, `open_meteo_url`, `latitude`, `longitude`, `entsoe_api_key`, `price_max_age_hours`, `frequency_url` | Real-time data ingestion (ENTSO-E/energy-charts/SMARD chain + staleness guard) |

**Planned — RLS ambient-variance guard:** The RLS estimator updates C_th from temperature decay observed during compressor OFF periods. Rapid ambient swings (door openings, loading events) can distort these observations. A future enhancement adds a variance check on T_amb during the OFF window — updates are skipped or down-weighted when ambient variance exceeds a configurable threshold — in addition to the existing C_th clamping to [0.1×, 10×] nominal.

**Implemented — intraday market fast-path:** `control.mpc_intraday_deviation_threshold_eur_per_mwh`
(> 0 enables): when the LIVE price deviates from the day-ahead forecast for the
current hour beyond the threshold, the pre-cool / shed decision is re-triggered
immediately — pre-cool on a cheap deviation, shed on an expensive deviation
(shed only with the product below setpoint and ≥ 2× critical TTF margin).
Complements the day-ahead ENTSO-E / energy-charts chain.

> **Security note:** The API secret is read from the `ATA_SECRET` environment variable at startup, overriding any value in `config.toml`. Never bake production secrets into configuration files.

## Control Engine

The single-store pilot uses a **6-mode priority cascade**. Higher-numbered modes can only activate when all lower-priority constraints are satisfied:

| Priority | Mode | Trigger | Action |
|---|---|---|---|
| **1** | `Emergency` | TTF < 900s or T_int ≥ T_max | Forced 100% cooling, bypasses governor |
| **2** | `ThermalGuard` | Thermal integral exceeds threshold | Enforce cooling to prevent cascade failure |
| **3** | `GridBalancing` | \|freq − 50\| > deadband | Frequency response using thermal mass |
| **4** | `MpcPreCool` | MPC window < 2h + cheap price | Pre-cool to T_min anticipating expensive hours |
| **5** | `Arbitrage` | Price < €30/MWh or > €80/MWh | Pre-cool (cheap) or shed (expensive) |
| **6** | `Idle` | No opportunity | Normal thermostat; on/off with governor protection by default, or continuous VFD modulation when `control.vfd_enabled` + `compressor_speed_addr` are set |

Key mechanisms:

- **MPC solver** — brute-force optimal pre-cooling start search over the 12-hour lookahead `(price, temperature)` horizon, with a confidence metric `1 − best_cost / worst_cost`. Only acts when `confidence > 0.2` and price < `mpc_cheap_threshold_eur_per_mwh`.
- **Intraday fast-path** — re-triggers pre-cool/shed immediately when the live price deviates from the day-ahead forecast for the current hour beyond `mpc_intraday_deviation_threshold_eur_per_mwh` (0 = off).
- **VFD oil-return pulse** — hardware-protection overlay (not a numbered priority): forces a brief high-speed run on the pulse cadence, sitting between GridBalancing and the economic modes — never overrides Emergency, ThermalGuard, or an active grid response.
- **ASCT governor** — enforces minimum compressor runtime (900s) and off-time (600s) to protect against short-cycling, oil-return failure, and contactor wear. **All time values use the simulated clock** to avoid the time-domain collision between real and simulated time.
- **Thermal integral** — `∫ max(T−T_setpoint, 0) dt` with exponential decay (`0.99^dt`) engages ThermalGuard when the excursion budget is exhausted.
- **Time-To-Failure (TTF)** — `(T_max − T_int) / (dT/dt)` drives Emergency mode.

The fleet engine uses a **7-mode cascade** that additionally includes `DualMarket` (charging TES tanks while earning FCR revenue) between GridBalancing and MPC Pre-Cool.

## HTTP API

Served via Axum on the configured bind address (TLS optional via rustls):

| Method | Route | Description | Auth |
|---|---|---|---|
| GET | `/status` | Current system state (JSON) | — |
| GET | `/health` | Liveness probe | — |
| GET | `/metrics` | Prometheus text exposition | — |
| GET | `/config` | Effective config (secrets redacted) | — |
| PUT | `/config` | Remote config update (persisted) | Bearer token |
| POST | `/price` | Update virtual electricity price | Bearer token |
| POST | `/alerts/test` | Send a test alert through the pipeline | Bearer token |
| POST | `/ota/check` | Trigger an immediate OTA manifest check | Bearer token |

`GET /status` returns: version, uptime, internal/ambient temperature, compressor state, active controller mode, TTF, thermal integral, grid frequency, price, MPC confidence, and RLS-estimated C_th.

## Security

Mutating API routes are protected by **HMAC-SHA256 bearer tokens**:

```
token = base64url( HMAC-SHA256( api_secret, route_scope ) )
```

- Tokens are **per-route capability tokens** derived from a single secret.
- Verification uses **constant-time comparison** to resist timing attacks.
- `security.auth_enabled = false` disables auth (development only).
- The production secret should be set via the `ATA_SECRET` environment variable.

> **Fail-closed:** when `auth_enabled = true`, the gateway **refuses to start**
> unless the API secret is set to a non-default value (via `ATA_SECRET` or a
> configured `api_secret`). The well-known placeholder secret
> `"change-me-in-production"` is rejected so a public secret can never ship.
> `GET /config` redacts all secrets (`api_secret`, `entsoe_api_key`,
> `webhook_url`, `verify_key_hex`), and `PUT /config` persistence writes a
> **sanitized** config to disk (secrets stay in the environment).

## Known Limitations (updated 2026-08-11 — matches current code)

Honest status of the production pilot after a safety + security audit:

- **MPC pre-cooling uses the real day-ahead price series in live mode** —
  `MpcPreCool` activates when the 12-hour ENTSO-E/energy-charts lookahead shows
  a cheap window (`mpc_lookahead_from_live`); it falls back to a flat lookahead
  only when no series is available.
- **`POST /price` is transient in simulation mode** — the simulation engine
  overwrites the price each frame; use it for tests, not live tuning.
- **CSV log rotates daily** — per-run file per date (`maybe_rotate` in
  `logging/mod.rs`); the file is truncated only on first start of a new day.
- **Stale-price guard active** — a price older than `price_max_age_hours`
  suppresses arbitrage/MPC, raises a `price_stale` alert, and thermostat/safety
  modes keep running.
- **Hardware mode dashboard** shows the internal thermal *model* while the
  control decisions use real PLC readings (display-only cosmetic).
- **Modbus loop is read-optimistic** — write failures log + retry; there is no
  end-to-end ack of the compressor state read back (coil echo is verified).

## Modbus PLC Interface

A minimal native Modbus TCP client (`src/modbus_client.rs`) implements the subset needed by the ATA gateway:

- **FC 0x03 — Read Holding Registers** for internal and ambient temperature (signed int16, °C × 100).
- **FC 0x05 — Write Single Coil** for the compressor command and alarm outputs.
- **FC 0x06 — Write Single Register** for the VFD compressor speed command (uint16, 0..1000 = 0.0..100.0 %) when `registers.compressor_speed_addr` is configured. When the speed register is set it is the AUTHORITATIVE compressor command (on/off decisions map to 1000/0, modulated decisions write their speed); otherwise the legacy coil path is used. The write is verified against the PLC echo (fail-closed).

The background polling loop mirrors the gateway control state onto the PLC coils/registers while logging PLC sensor readings. It reconnects automatically with exponential backoff (up to 30s) and degrades gracefully to log-only mode when the PLC is unreachable.

**Implemented — VFD oil-return pulse sequence:** with `control.vfd_enabled` +
`vfd_oil_return_pulse_every_sec > 0`, the gateway periodically forces a short
high-speed burst (`vfd_oil_return_pulse_duration_sec` at
`vfd_oil_return_pulse_speed`) so oil returns to the compressor sump on legacy
racks operating at low speed. The ASCT governor schedules the pulses respecting
min-run/min-offtime locks (never violating short-cycling protection) and never
overrides an active grid response; each pulse is logged as
`ControllerMode::OilReturnPulse` and counted in `ata_oil_return_pulses_total`
for predictive-maintenance analytics.

## Alerts

The alert system emits structured JSON alerts to a webhook (Slack/Discord/Teams) with:

- **Severity levels:** `critical` (cold-chain risk: emergency / thermal guard), `warning` (abnormal conditions), `info` (operational events).
- **Bounded channel** (capacity 256, non-blocking `try_send`).
- **Exponential retry** with configurable max retries and backoff base.
- **Log-only fallback** when alerts are disabled or no webhook is configured.

Alert factory helpers: `emergency`, `thermal_guard`, `grid_balancing`, `sensor_loss`, `ota`, `startup`, `startup_off`.

## OTA Firmware Updates

The OTA module checks a remote manifest for newer firmware versions, downloads the bundle, and stages it for the bootloader:

```
Manifest JSON:
{
  "version": "0.3.0",
  "url": "https://.../firmware-0.3.0.bin",
  "sha256": "<hex>",
  "signature_hex": "<ed25519 signature over bundle, hex>"
}
```

- **Ed25519 signature verification** via `ring` (fail-closed when a key is configured).
- **SHA-256 checksum verification** when provided.
- Version comparison is semantic (`major.minor.patch`, optional leading `v`).
- Bundles are staged to `ota_staging/` for the bootloader to apply on next restart.
- Current firmware version: `0.2.0`.

## Live Data Pipeline

In production mode (`live_data.enabled = true`), the gateway continuously ingests:

| Data | Source chain |
|---|---|
| Electricity price | **ENTSO-E Transparency Platform** (official, free key via `ENTSOE_API_KEY`) → energy-charts.info → SMARD → persisted last-known-good cache (`data_cache/last_price.json`) |
| Weather | Open-Meteo forecast API |

Refresh intervals are configurable (`price_refresh_sec`, `weather_refresh_sec`).
Failover is automatic; consecutive failures are tracked in `LiveState` and
exposed via `/status`. A 12-hour MPC lookahead is built from the live
day-ahead price/temperature series (flat fallback only when no series is
available).

> **Note (Aug 2026):** the legacy `https://api.energy-charts.info` endpoint
> returned HTTP 503 and the SMARD `chart_data` JSON endpoint returned HTTP 404
> in end-to-end tests. **Configure an ENTSO-E API key for reliable production
> prices** — it is now tried first when `ENTSOE_API_KEY` is set.

In simulation mode, the full 8760-hour NRW 2025 dataset (real data cached in
`data_cache/`, synthetic fallback in `simulation_data/`) drives MPC with real
price/temperature profiles.

## Metrics & Observability

Prometheus metrics are exposed at `GET /metrics`:

| Metric | Type | Description |
|---|---|---|
| `ata_internal_temp_c` | Gauge | Internal temperature (×100) |
| `ata_compressor_on` | Gauge | Compressor state (0/1) |
| `ata_compressor_speed` | Gauge | VFD speed (0–100 = 0–100%, when `vfd_enabled`) |
| `ata_price_eur_per_mwh` | Gauge | Electricity price (×100) |
| `ata_ttf_seconds` | Gauge | Time-to-failure |
| `ata_control_mode_total{mode}` | Counter | Control mode activations |
| `ata_alerts_total{severity}` | Counter | Alerts raised |
| `ata_ota_total{status}` | Counter | OTA attempts |
| `ata_http_requests_total{path,status}` | Counter | HTTP requests |
| `ata_loop_duration_seconds{phase}` | Histogram | Control loop timing |
| `ata_oil_return_pulses_total` | Counter | VFD oil-return pulses triggered |

Configure a Prometheus scrape (e.g. `prometheus.yml`) pointing at the gateway `/metrics` endpoint, then visualize with Grafana.

## Fleet Simulation

The standalone fleet engine (`thermal-fleet` v2.0.0) simulates stores through 8760 hours of NRW 2025 data using:

- **20 facility types (+ HVAC)** — retail freezers (Aldi, Edeka, Marktkauf), cold storage (freezer, cold, frozen, facility, warehouse), industrial/process (datacenter, chemistry, industry, blast freezer, brewery, arena, marine, immersion), and specialty (pharma, cryo, mortuary, ripening, hvac). Each has full physical parameters (volume, U-value, cooling power, COP, thermal mass, defrost, TES, shed limits).
- **7-mode priority cascade** with dual-market (arbitrage + FCR) revenue stacking.
- **ASCT governor** with shed-hour tracking and state-change counting.
- **RLS fingerprinting** of each store's thermal capacitance.
- **Defrost cycles**, **thermal stratification**, and **cargo thermal lag** (critical cooling chain).
- **TES tanks** with charge state for FCR-capable sites.
- **Retail hours integration** (closed-store heat gain reduction) and **eco-drift** (retail sheds limited to setpoint +1°C to avoid the Carnot recovery penalty).
- **Dynamic price thresholds** — 30th/70th percentiles for retail, 15th/85th for industrial; TES sites use 80%/120% of the lookahead average.
- **Baseline gaming prevention** — the baseline is computed from physical heat load (wall leakage + internal heat + defrost) × 1.3 ON/OFF cycling penalty, not from historical consumption.
- **Volatile mode** (`--volatile`) — removes `max_shed_hours`, thermal-runaway checks, and retail shed limits for research/what-if analysis.

Results include per-type and fleet totals for energy, cost, savings (%), FCR/droop revenue, defrost energy, and runtime hours, plus the 50% service-fee slice and a timestamped CSV export.

## Testing

```bash
# Main engine — 78 unit tests
cargo test --manifest-path thermal-arbitrage-mvp/Cargo.toml

# Fleet engine — 30 unit tests
cargo test --manifest-path thermal-arbitrage-mvp/standalone-fleet/Cargo.toml
```

Test coverage includes: governor min-on/min-off, MPC optimal-window search, MPC confidence, emergency TTF, grid balancing, API auth (missing/rejected/valid tokens), config secret redaction, alert serialization, OTA version parsing/SHA-256/hex decode, Modbus register decoding and connection failure handling, RLS convergence, dynamic COP, thermal TTF bounds, sensor simulation, and synthetic data generation — all deterministic and passing (78/78 on the gateway + 30/30 on the fleet engine, verified 2026-08-11).

## Roadmap

| Phase | Status |
|---|---|
| **Phase 0 — MVP** (thermal model, 6-mode cascade, MPC, CSV logging, RPi 3 optimized build) | ✅ Complete |
| **Phase 1 — Fleet Simulation** (21 types, 7-mode cascade, RLS, defrost, stratification, TES, dual-market, baseline gaming prevention) | ✅ Complete |
| **Phase 2 — Production Pilot** (Modbus PLC, live data, alerts, remote config, secure API, OTA, metrics) | 🔄 In progress (majority implemented) |
| **Phase 2.5 — Hardening** (VFD oil-return pulses for legacy racks, RLS ambient-variance guard, intraday fast-path trigger) | 🔮 Planned |
| **Phase 3 — Fleet Orchestration** (central controller, portfolio FCR bidding, ML price forecasting, DER registration, multi-market optimization) | 🔮 Planned |
| **Phase 4 — Enterprise Platform** (white-label API, AI store classification, BMS integration, fleet-wide DR) | 🌟 Vision |

---

*Ambient Thermal Arbitrage — v0.2.0 Production Pilot*
*© 2026 Ambient Thermal Arbitrage. All rights reserved.*
# Ambient Thermal Arbitrage (ATA)

Transform temperature-controlled industrial facilities into **virtual batteries** for energy arbitrage, frequency regulation (FCR/Droop), and demand-side management — without compromising product integrity.

Website: [ambient.kellersystems.dev](https://ambient.kellersystems.dev)

---

## Architecture Overview

| Artifact | Location | Status |
|---|---|---|
| **Production Pilot Gateway** (v0.2.0) | `crates/gateway/` | Rust-based edge gateway. Closed-loop Modbus RTU / TCP controller, PLC watchdog, COV write-filtering, Slew-rate limiting, ASCT governor, HTTP API, OTA updates. 99 unit tests passing. |
| **Fleet Simulation Engine** (v2.0.0) | `crates/fleet/` | High-performance full-year (8,760 h) simulator across 21 facility types and 12 enterprise tiers (Startup/KMU/Enterprise). Fair, un-gamed baseline physics, dual-market arbitrage + FCR accounting. 30 unit tests passing. |
| **Technical Whitepaper** | `docs/whitepaper-ATA.md` | Full mathematical and thermodynamic specification. |
| **Field Verification & Audit** | `SANITY_CHECK.md` | Complete third-party review, baseline correction, and safety validation report. |

---

## Validated Financial & Thermodynamic Profile

ATA focuses on facilities with **large thermal inertia and process cooling** (frozen logistics, breweries, data centers, chemical/industrial cooling), where cold storage mass acts as a reliable battery:

- **Tier 1 (Small / Startup, 150–400 kW):** 1,000 € – 2,800 € / yr energy arbitrage (+ up to 6,700 € FCR upside)
- **Tier 2 (Mittelstand / KMU, 1,000–3,000 kW):** 5,000 € – 13,800 € / yr energy arbitrage (+ up to 75,000 € FCR upside)
- **Tier 3 (Enterprise / Hyperscale, 3,500–16,000 kW):** 15,000 € – 47,900 € / yr energy arbitrage (+ up to 267,000 € FCR upside)

*(Note: Standard retail grocery stores with open shelves like Aldi/Edeka have small thermal mass and fast rebound heat losses, yielding neutral to slightly negative net spot margins. ATA is specifically optimized for bulk thermal storage).*

---

## Safety & Industrial Controls

1. **Hardware & PLC Protection (Modbus RTU / TCP):**
   - **Change-of-Value (COV) Write Filtering:** Registers are only written on true setpoint delta, preventing EEPROM/Flash wear on Carel, Danfoss, and WAGO controllers.
   - **PLC Watchdog Heartbeat:** Cyclical heartbeat register fallback — on gateway communication loss (> 60 s), the PLC immediately reverts to autonomous local thermostat control.
2. **Refrigeration Circuit & Compressor Guards:**
   - **Slew-Rate Limiting:** Setpoint ramps capped at $\le 6.0^\circ\text{C}$ per hour to prevent liquid slugging, evaporator freezing, and pressure shock.
   - **High-Ambient Derating ($T_{\text{amb}} \ge 28^\circ\text{C}$):** Arbitrage shedding automatically locks out during extreme outdoor heat to prevent high-pressure trips.
   - **Active State Constraint Tracking (ASCT):** Hardware-enforced minimum runtime and minimum downtime guards.
   - **Oil Return Pulse:** Cyclical high-velocity sweep cycles ensure compressor lubrication during extended low-load phases.
3. **Peak Shaving & Grid Fee Protection (§ 19 Abs. 2 StromNEV):**
   - Pre-cooling is locked out during the local DSO's (VNB) specific high-load concession windows (e.g. 11:00–13:00 / 17:00–19:30 in central/southern grids, 17:00–20:00 in coastal grids) to prevent peak demand surcharges.
   - Comprehensive profiles for all **16 German Federal States (Bundesländer)** with local DWD climate baselines, Open-Meteo weather fetching, and regional VNB profiles (Westnetz, Bayernwerk, Netze BW, Avacon, SH Netz, E.DIS, Stromnetz Berlin, etc.).

---

## Quick Start

### Build & Run Test Suite
```bash
# Build release binaries
cargo build --release --workspace

# Run complete workspace test suite
cargo test --workspace
```

### Run Enterprise Fleet Simulation (Regional 16-State Dispatch)
```bash
# Default (Nordrhein-Westfalen / Westnetz)
cargo run --release -p thermal-fleet -- \
  "1 freezer_small + 1 freezer_mid + 1 freezer_large + 1 brewery_small + 1 brewery_mid + 1 brewery_large + 1 dc_small + 1 dc_mid + 1 dc_large + 1 industry_small + 1 industry_mid + 1 industry_large"

# Simulate for Bayern (Bayernwerk Netz) or Schleswig-Holstein (SH Netz)
cargo run --release -p thermal-fleet -- --state BY "10 freezer_mid + 2 brewery_large"
cargo run --release -p thermal-fleet -- --state SH "50 aldi + 20 edeka"
```

### Launch Production Gateway (Simulation Mode)
```bash
export ATA_SECRET=$(openssl rand -base64 32)
cargo run --release -p thermal-arbitrage-mvp
```

For production deployment onto industrial PCs or Raspberry Pi edge devices, refer to [`crates/gateway/DEPLOYMENT.md`](crates/gateway/DEPLOYMENT.md).

---

## License

Proprietary — see `LICENSE`. © 2026 Ambient Thermal Arbitrage. All rights reserved.

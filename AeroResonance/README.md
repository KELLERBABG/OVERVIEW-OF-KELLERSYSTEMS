# Energy Harvesting via Atmospheric Resonance (EHAR)

> **Status: concept-stage, pre-Gate-1, now with a buildable hardware MVP.**
> The simulation honestly shows the atmospheric-resonance harvesters do **not**
> meet the Phase-1 energy-density gate (> 1 mW/m²). The project's mitigation
> (risk R1) is a **hybrid solar + atmospheric** design — and that hybrid is
> now concrete: a self-powered atmospheric observatory node you can build for
> ~120 €. See [`hardware/`](hardware/README.md).

## Repository layout

```
docs/WHITEPAPER.md                  the concept whitepaper (24-month roadmap & physics)
BUILD_SPECIFICATION.md              complete build & engineering specification
simulation/                         Rust simulation MVP (workspace)
  src/                              config, physics, subsystems, engine, CLI
  simulation_core/                  no_std core: crypto, entropy, scheduler, cap model
  docs/GATE1_ENERGY_DENSITY_REPORT.md   the honest Gate-1 verdict + hybrid sizing
hardware/                           MVP build: BOM, power budget, assembly, safety, bring-up
firmware/                           Rust firmware: host-tested core + ESP32-C3 binary
```

## The three layers

1. **Simulate** — `cd simulation && ./dev.sh run --release -- --gate-report`
   (Windows note: use `dev.cmd`/`dev.sh` if the repo path contains spaces).
   `--hybrid-report` prints the risk-R1 decision material (PV sizing vs the
   plan's Phase-2/3 gates, resonance top-up ≈ 0.4 %).
2. **Build** — order [`hardware/BOM.md`](hardware/BOM.md) (~120 €), follow
   [`hardware/ASSEMBLY.md`](hardware/ASSEMBLY.md), keep
   [`hardware/SAFETY.md`](hardware/SAFETY.md) handy.
3. **Run** — flash [`firmware/esp32c3/`](firmware/esp32c3/), accept per
   [`hardware/BRINGUP.md`](hardware/BRINGUP.md). The firmware reuses
   `eh-sim-core` (same scheduler/entropy/crypto/cap model as the simulator),
   so the sim is a digital twin: compare the node's V_cap trace with the
   sim's and tune the model.

## Key decisions (recorded)

| Date | Decision |
|------|----------|
| 2026-08-13 | WP0–WP7 production-readiness executed; Gate-1 verdict: **not met** for resonance-based harvesters (recorded honestly) |
| 2026-08-13 | Risk-R1 mitigation chosen: **hybrid solar + atmospheric** — proceed to a buildable MVP (this hardware/firmware work) |
| 2026-08-13 | MVP form factor: ESP32-C3 + BQ25570 + supercap bank + M1.3 instrumentation payload (ELF/piezo/T/H/P) |

## License

Proprietary. See [`LICENSE`](LICENSE) (root) and `simulation/LICENSE`. Private
research artifact — no deployment intended at this stage.

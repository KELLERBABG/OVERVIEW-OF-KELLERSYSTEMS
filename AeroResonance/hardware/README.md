# EHAR Field Node — MVP Hardware

**A solar-powered atmospheric observatory node you can build on a breadboard
and then move to a perfboard.** ~120–150 € of off-the-shelf parts, hand tools
only.

## Why this design (read this first — it is the honest answer)

The Gate-1 report (`simulation/docs/GATE1_ENERGY_DENSITY_REPORT.md`) shows the
atmospheric-resonance harvesters produce **micro-watts** — three orders of
magnitude below the plan's 1 mW/m² gate. No practical device can be powered by
them. The project plan's own mitigation for this (risk R1) is a **hybrid
solar + atmospheric** design:

| Component | Role | Viable? |
|-----------|------|---------|
| **PV panel + supercapacitor + PMIC** | the power source (kW/m²-class density) | ✅ **yes** — this is what makes the MVP actually run |
| **ELF/Schumann receiver** (loop + preamp) | *instrumentation* — measures the atmospheric resonance spectrum | ✅ yes, and it fulfils plan milestone **M1.3** (spectrum campaign) |
| **Piezo disc + amplifier** | experimental infrasound/vibration channel | ⚠️ demonstration only — do not expect usable power |
| **TEG / ground ΔT** (DS18B20 pair) | measures the real temperature gradient | ✅ yes as a *sensor* — the sim predicts ~µW from a 0.2 °C ΔT |
| Any resonance harvester **as the power source** | — | ❌ **no** — the sim is clear about this, don't spend money on it |

The node therefore *is* the M1.3 measurement campaign plus a real, dependable
power train. Every µW the resonance channels extract is logged and can be
compared against the simulation — that comparison is the project's science.

## Architecture

```
                 ┌────────────────────────────────────────────────┐
  6V 2W PV ─────►│ BQ25570 (boost charger, MPPT)                  │
                 │   PV-IN ── VSTOR ── supercap bank (2×50F=25F)  │
                 │   VOUT 3.3 V ──► ESP32-C3 SuperMini (3V3 pin)  │
                 └────────────────────────────────────────────────┘
                                     │ 3.3 V
        ┌────────────────────────────┼───────────────────────────┐
        ▼ I2C                        ▼ ADC1 GPIO0                 ▼ ADC1 GPIO2/3
  BME280 (T/H/P)        V_cap divider (GPIO0)      ELF preamp out / piezo amp out
  DS18B20 (ground T)    PV divider (GPIO1)         (M1.3 channels)
        └────────────────────────────────────────────────────────┘
                                     │ USB (native USB-serial-JTAG)
                                   PC / laptop (v1 telemetry)
```

- **Firmware:** `firmware/` — Rust, `no_std`. The ESP32-C3 binary
  (`firmware/esp32c3/`) is the thin HAL glue; all logic lives in
  `ehar-firmware-core`, which reuses `eh-sim-core` (the *same* scheduler,
  entropy, crypto and capacitor model the desktop simulator uses) — the
  simulation is a digital twin of this device.
- **Telemetry v1:** the ESP32-C3's built-in USB-serial port — plug in a USB
  cable and read frames on any terminal (zero extra hardware).
- **Telemetry v2 (documented upgrade):** ESP-NOW (`esp-radio` crate) or a LoRa
  module — no design change, the frame format is identical.

## Docs in this folder

| File | What you get |
|------|--------------|
| [`BOM.md`](BOM.md) | Full parts list with prices & where to buy (~120–150 €) |
| [`POWER_BUDGET.md`](POWER_BUDGET.md) | The viability math: harvest, storage, autonomy, duty cycle |
| [`ASSEMBLY.md`](ASSEMBLY.md) | Step-by-step build: bench → breadboard → perfboard, pin tables |
| [`SAFETY.md`](SAFETY.md) | Grounding, lightning, capacitors, Li-ion — read before building |
| [`BRINGUP.md`](BRINGUP.md) | Bring-up sequence + acceptance tests (what "viable" means) |
| `../firmware/` | Rust firmware: host-tested core + ESP32-C3 binary |
| `../simulation/` | The simulator that predicts what this node measures |

## Status

- [x] Architecture + power budget (this folder)
- [x] Firmware core (`ehar-firmware-core`) — host-tested, builds for the C3 target
- [x] ESP32-C3 binary skeleton (USB telemetry, ADC sampling)
- [ ] Buy parts (see BOM)
- [ ] Bench bring-up (see BRINGUP)
- [ ] v2: BME280/DS18B20 over I2C, ESP-NOW radio, light-sleep duty cycle

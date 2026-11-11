# Piezoelectric Energy Harvester — Circuit Description

## Topology

```
PZT Element → Bridge Rectifier → BQ25504 Boost Converter → Supercapacitor → LDO → MCU
```

## Circuit Details

### Stage 1: Piezoelectric Element (PZT-5A)
- 20 mm diameter, 1 mm thickness disk
- Resonance: ~60 Hz (tuned to walking/bridge vibration)
- Open-circuit voltage: up to 20 V peak at 1g
- Capacitance: ~10 nF

### Stage 2: Bridge Rectifier (DF10S)
- Full-wave rectification of AC piezo output
- Schottky diodes for low forward voltage (~0.3 V)
- Output: pulsed DC, 0–20 V depending on vibration

### Stage 3: Boost Converter with MPPT (BQ25504)
- Ultra-low quiescent current: 330 nA (typical)
- Cold-start voltage: 330 mV (minimum)
- MPPT set to 80% of open-circuit voltage via voltage divider
  - R_MPPT1 = 10 MΩ, R_MPPT2 = 2.5 MΩ → V_MPPT = 0.8 * VOC
- Output: regulated 3.3 V
- Storage: 1 F supercapacitor (C1)
- Overvoltage protection: 3.3 V (set by OV pin resistor divider)

### Stage 4: LDO Regulator (MCP1700-3302E)
- Input: 3.3 V from supercapacitor (via BQ25504 output)
- Output: 1.8 V for MCU
- Quiescent current: 2 μA
- Dropout voltage: 180 mV at 250 mA

### Stage 5: MCU Power Management
- MCU monitors storage voltage via voltage divider (R3 = 100 kΩ, R4 = 100 kΩ)
- ADC reading: V_STORAGE * (R4 / (R3 + R4))
- Thresholds (1.8 V MCU supply):
  - Surplus: V_STORAGE > 3.3 V → full operation
  - Nominal: 2.7 V < V_STORAGE < 3.3 V → normal operation
  - Low: 2.2 V < V_STORAGE < 2.7 V → reduced functionality
  - Critical: V_STORAGE < 2.2 V → wake-on-vibration only

## Power Budget

| Operation | Current | Voltage | Power | Duration |
|-----------|---------|---------|-------|----------|
| MCU sleep (deep) | 0.4 μA | 1.8 V | 0.72 μW | indefinite |
| MCU active (48 MHz) | 4.6 mA | 1.8 V | 8.3 mW | 10 ms |
| Radio TX (0 dBm) | 11.3 mA | 1.8 V | 20.3 mW | 0.5 ms |
| Radio RX | 13.5 mA | 1.8 V | 24.3 mW | 1 ms |
| Sensor read (LIS3DH) | 2 μA | 1.8 V | 3.6 μW | 1 ms |

## Piezoelectric Tuning Considerations

### Narrow Bandwidth Risk

PZT-5A elements tuned to 60 Hz have a high Q-factor (Q ≈ 30–50), meaning they only efficiently harvest energy within a narrow bandwidth (~2–4 Hz) around the resonant frequency. Real-world civil infrastructure vibrations drift significantly:

| Source | Frequency Range | Typical Amplitude |
|--------|----------------|-------------------|
| Pedestrian footfall | 1.5–3 Hz | 0.1–0.5 g |
| Bridge vibration (traffic) | 10–40 Hz | 0.05–0.3 g |
| Road traffic (heavy vehicles) | 5–30 Hz | 0.1–0.8 g |
| Wind-induced (structures) | 0.1–1 Hz | 0.01–0.1 g |
| Railway | 40–120 Hz | 0.2–1.0 g |

### Mitigation Strategies

1. **Mechanical Cantilever Beam (Recommended)** — Replace the rigid disk with a cantilever-mounted PZT bimorph. The beam can be tuned to a wider frequency range by adjusting the proof mass and beam length. A bandwidth of 10–40 Hz is achievable.

2. **Dynamic Resonance Tuning (Phase 3)** — Use the accelerometer to measure the dominant vibration frequency, then adjust a digital trim capacitor bank to shift the piezo's electrical resonance to match. This is described in the whitepaper as "Crystalline Lattice Resonance Tuning."

3. **Array of Elements** — Use multiple PZT elements with staggered resonant frequencies (e.g., 20 Hz, 40 Hz, 60 Hz) to cover a wider range. This increases cost and complexity but provides more consistent energy harvesting.

4. **Nonlinear Harvesting** — Use a bistable or snap-through mechanism that produces power across a wider frequency range. This is an active research area but may be deployable in Phase 4.

### Initial Prototype Recommendation

Start with a single PZT-5A disk at 60 Hz for laboratory characterization. If field testing shows insufficient energy at the deployment site (due to frequency mismatch), switch to a cantilever beam design with adjustable proof mass.

## Energy Per Vibration Event
- Typical footstep on bridge: ~1 mJ harvested (at 60 Hz resonance)
- Sufficient for: 1× sensor read + 1× radio TX (μJ scale)

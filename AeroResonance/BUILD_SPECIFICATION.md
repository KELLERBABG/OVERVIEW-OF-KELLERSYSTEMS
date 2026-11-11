# EHAR Field Node — Complete Build & Engineering Specification

> **Everything you have to consider, buy, assemble, flash, calibrate, and verify before and during a real-world build.**

---

## 1. Executive Summary & Reality Check

Before buying components, you must know what the physics actually allows:
1. **The Power Train is Solar + Supercapacitor**:
   Atmospheric resonance harvesters (ELF loop, piezo infrasound cantilever, Seebeck TEG) produce **sub-microwatts** ($\approx 10^{-9}$ to $10^{-6}\text{ W}$). They cannot power microcontrollers or radios.
   The node's actual energy source is a **$6\text{V}$, $2\text{W}$ photovoltaic panel** buffered by a **$25\text{F}$ supercapacitor bank** managed by a **TI BQ25570 nano-power PMIC**.
2. **The Resonance Array is an Instrumentation Payload**:
   The ELF magnetic loop, piezo vibration disc, and dual temperature sensors serve as **scientific measurement channels** (Plan Milestone M1.3). They log real atmospheric spectra, quantify ambient resonance, and ground-truth the digital twin simulation.

---

## 2. Master Bill of Materials (BOM) & Sourcing

### A. Power Harvest & Storage (~45 €)
| Item | Part / Spec | Qty | Unit Cost | Function & Sourcing Notes |
|---|---|---|---|---|
| **P1** | 6V 2W Polycrystalline PV Panel (~140×110 mm) | 1 | ~10 € | Powers the node. Open-circuit voltage $V_{oc} \approx 7.2\text{V}$, $I_{sc} \approx 350\text{mA}$. Any 5–6V 1.5–3W panel works. |
| **P2** | **TI BQ25570** Nano-Power Boost Harvester + Buck Breakout | 1 | ~18 € | Cold-starts from $330\text{mV}$, MPPT tracking, charges supercap to $5.4\text{V}$, outputs regulated $3.3\text{V}$ rail. |
| **P3** | Supercapacitor 2.7V 50F (Radial snap-in / leads) | 2 | ~7 € | Wired in series $\to \mathbf{25\text{F} @ 5.4\text{V}}$ ($200\text{J}$ usable energy). Low ESR, $-40^\circ\text{C}$ to $+65^\circ\text{C}$ rated (e.g. KEMET FT series). |
| **P4** | 1N5819 Schottky Diodes | 2 | ~0.30 € | Reverse-current and reverse-polarity protection on PV+ and VSTOR. |
| **P5** | 0.5A PTC Resettable Fuse | 1 | ~1 € | Overcurrent protection installed directly at the PV panel feedline. |
| **P6** | 100 kΩ 0.1% Metal Film Resistors | 4 | ~0.20 € | Precision voltage dividers for $V_{\text{cap}}$ and $V_{\text{pv}}$ monitoring. |

### B. Core Electronics & Digital Subsystem (~35 €)
| Item | Part / Spec | Qty | Unit Cost | Function & Sourcing Notes |
|---|---|---|---|---|
| **E1** | **ESP32-C3 SuperMini** Development Board | 1 | ~5 € | RISC-V 32-bit core, 400 KiB SRAM, native USB-serial-JTAG, WiFi/BLE hardware. |
| **E2** | **Bosch BME280** Sensor Breakout (I2C) | 1 | ~6 € | High-precision barometric pressure, ambient temperature, relative humidity. |
| **E3** | **Dallas DS18B20** Waterproof Probe (1m lead) | 1 | ~4 € | Buried in subsoil to measure deep-ground temperature and real $\Delta T$. |
| **E4** | 4.7 kΩ Resistors | 3 | ~0.10 € | Pull-up resistors for I2C (SDA/SCL) and 1-Wire (DS18B20 data). |
| **E5** | Decoupling & Filter Caps (100nF, 1µF, 10µF MLCC) | 1 set | ~2 € | Bypass capacitors placed right next to IC power pins. |

### C. M1.3 Resonance Instrumentation Payload (~20 €)
| Item | Part / Spec | Qty | Unit Cost | Function & Sourcing Notes |
|---|---|---|---|---|
| **I1** | **MCP6002** or **LMV358** Dual Rail-to-Rail Op-Amp | 1 | ~1.50 € | **CRITICAL FIX**: Do NOT use TL072! Powered from single 3.3V, you need a true rail-to-rail single-supply op-amp. |
| **I2** | Ferrite Rod Search Coil (10–14 cm rod, 0.2 mm magnet wire) | 1 | ~4 € | 500–1000 turns wound on rod for Schumann resonance / ELF magnetic pickup. |
| **I3** | 27 mm PZT Piezoelectric Ceramic Disc | 1 | ~1 € | Cantilever/diaphragm mounted inside acoustic cavity for micro-vibration. |
| **I4** | 1 MΩ & 10 kΩ 1% Resistors | 1 set | ~1 € | Op-amp non-inverting bias network and gain feedback loop. |
| **I5** | 10 nF & 100 nF Film / C0G Ceramic Caps | 1 set | ~1 € | AC-coupling and bandpass filter caps for op-amp stages. |
| **I6** | 3.5 mm TRS Audio Jack + Shielded Cable | 1 set | ~3 € | Shielded external feedline for the outdoor ELF loop antenna. |

### D. Enclosure, Protection & Hardware (~30 €)
| Item | Part / Spec | Qty | Unit Cost | Function & Sourcing Notes |
|---|---|---|---|---|
| **H1** | IP65 Weatherproof Junction Box (approx. 150×100×70 mm) | 1 | ~10 € | UV-stabilized polycarbonate or ABS enclosure. Transparent lid recommended to view LEDs. |
| **H2** | M12 & M16 IP68 Cable Glands | 4 | ~4 € | Hermetic pass-throughs for PV line, ELF antenna feedline, and ground probe. |
| **H3** | Gore-Tex ePTFE Pressure Vent Plug (M12) | 1 | ~3 € | Prevents internal condensation while maintaining IP67 water/dust seal. |
| **H4** | **SMBJ5.0A** or **P6KE6.8CA** TVS Surge Diodes | 4 | ~2 € | Bi-directional transient voltage suppressors across PV+, VSTOR, and antenna lines. |
| **H5** | FR4 Double-Sided Perfboard (70×90 mm) + Terminal Blocks | 1 | ~4 € | Vibration-resistant point-to-point solder assembly. |
| **H6** | Copper Clad Earth Ground Stake (0.5m–1m) + 10 AWG wire | 1 | ~7 € | Dedicated station earth reference for antenna surge safety. |

**Total Estimated Hardware Cost: ~130 €**

---

## 3. Schematics & Wiring Pinout

### System Block Diagram
```
           ┌────────────────────────────────────────────────────────┐
  PV (+) ──►[0.5A PTC]──►[1N5819]──► VIN_DC                          │
  PV (-) ──────────────────────────► GND       TI BQ25570           │
                                     VSTOR ──►[2× 50F in series]    │
                                     VOUT (3.3V Regulated) ─────────┼──┐
                                     GND ───────────────────────────┼──┼──┐
           └────────────────────────────────────────────────────────┘  │  │
                                                                       │  │
   ┌───────────────────────────────────────────────────────────────────┘  │
   │  3.3V Power Rail                                                     │
   │  ┌───────────────────────────────────────────────────────────────────┘
   │  │ Star Ground Rail
   ▼  ▼
┌───────────────────────────────┐
│     ESP32-C3 SuperMini        │
│                               │
│ 3V3   ◄── 3.3V Rail           │
│ GND   ◄── Star Ground         │
│                               │
│ GPIO0 ◄── V_cap Divider (2:1) │ ◄── VSTOR (0..5.4V) via 100kΩ/100kΩ
│ GPIO1 ◄── PV Divider (2:1)    │ ◄── PV+ (0..7.2V) via 220kΩ/100kΩ (3.2:1)
│ GPIO2 ◄── ELF Preamp Output   │ ◄── MCP6002 Op-Amp Out (AC coupled, 1.65V bias)
│ GPIO3 ◄── Piezo Amp Output    │ ◄── MCP6002 Buffer Out (AC coupled, 1.65V bias)
│ GPIO4 ──► I2C SDA             │ ──► BME280 SDA (with 4.7kΩ pull-up)
│ GPIO5 ──► I2C SCL             │ ──► BME280 SCL (with 4.7kΩ pull-up)
│ GPIO6 ──► 1-Wire Data         │ ──► DS18B20 DQ (with 4.7kΩ pull-up)
└───────────────────────────────┘
```

### Critical Resistor Divider Values
The ESP32-C3 ADC has an input ceiling of approximately $2.5\text{V}$ to $3.1\text{V}$ under $11\text{ dB}$ attenuation:
1. **$V_{\text{cap}}$ Divider (GPIO0)**:
   - Max $V_{\text{cap}} = 5.4\text{V}$.
   - $R_1 = 100\text{ k}\Omega$, $R_2 = 100\text{ k}\Omega$ (Divider ratio $= 0.5$).
   - Max pin voltage $= 2.70\text{V}$ (safe).
2. **PV Panel Divider (GPIO1)**:
   - Max open-circuit $V_{oc} = 7.5\text{V}$ in cold sunlight.
   - Do **NOT** use $100\text{k}/100\text{k}$ (which would yield $3.75\text{V}$, frying the ADC pin).
   - Use $R_1 = 220\text{ k}\Omega$, $R_2 = 100\text{ k}\Omega$ (Divider ratio $= \frac{100}{320} = 0.3125$).
   - Max pin voltage $= 7.5 \times 0.3125 = 2.34\text{V}$ (safe).

### Preamplifier Circuit (MCP6002 Rail-to-Rail)
```
               3.3V
                │
              [100k]
                │
Ferrite Coil ──[10nF]──┬────────(+) MCP6002
                       │        (-)─────┬─────────┐
                     [100k]             │         │
                       │              [10k]     [100k] // Gain = 11
                      GND               │         │
                                       GND        ├────[1µF]──── GPIO2
                                                  │
                                                [100k]
                                                  │
                                                 GND
```

---

## 4. Mechanical & Environmental Build Guidelines

1. **Condensation & Breathing**:
   - An airtight outdoor box will cycle hot and cold, pulling moist air past seals and causing internal droplets on PCBs.
   - **Requirement**: Drill an M12 hole in the bottom face of the box and install an **ePTFE hydrophobic vent plug**. It breathes air while blocking liquid water and dust.
2. **Thermal Management**:
   - Supercapacitors age exponentially with temperature (capacitance drops, leakage doubles every $+10^\circ\text{C}$).
   - Mount the enclosure in shaded ambient air or underneath the solar panel tilt bracket — never directly exposed to noon bake.
3. **Cable Glands & Drip Loops**:
   - All cable entries must enter through the **bottom** of the enclosure.
   - Form a physical **drip loop** (U-bend hanging below the gland) on every cable so rainwater drips off before reaching the threads.
4. **Star Grounding & Lightning Safety**:
   - Run all ground returns to a **single common brass ground bus** at the PMIC GND terminal.
   - Install bidirectional TVS diodes (SMBJ5.0A) directly at the bulkhead where sensor/antenna wires enter the box.
   - Connect the chassis ground to an external copper earth stake.
   - **Storm protocol**: Always provide a quick-disconnect jack on the ELF loop antenna. Unplug during thunderstorms.

---

## 5. Firmware Flashing, Calibration & Bring-Up

### Prerequisites
Install the Rust RISC-V bare-metal toolchain and ESP flasher:
```bash
rustup target add riscv32imc-unknown-none-elf
cargo install espflash
```

### Step 1: Calibration Constants
Measure actual divider resistors with your multimeter and update `firmware/esp32c3/src/main.rs`:
```rust
const V_CAP_DIVIDER: f64 = 100.0 / (100.0 + 100.0); // 0.500
const PV_DIVIDER: f64    = 100.0 / (220.0 + 100.0); // 0.3125
```

### Step 2: Flash Firmware over USB
Connect the ESP32-C3 SuperMini directly via USB-C to your PC:
```bash
cd firmware/esp32c3
cargo run --release --target riscv32imc-unknown-none-elf
```

### Step 3: Bench Acceptance Checklist
- [ ] **Serial Output**: Open serial monitor at $115200\text{ baud}$. Verify `EHAR Field Node v0.1.0` banner appears.
- [ ] **Entropy Check**: Observe that ADC noise produces non-zero entropy bits every cycle.
- [ ] **Voltage Verification**: Verify with a multimeter that BQ25570 VOUT is exactly $3.30\text{V} \pm 0.05\text{V}$.
- [ ] **Ghost-Net Authentication**: Feed output hex strings to `cargo run --example decode -- "<hex>"` and verify valid frame decoding with dev key `0x42`.
- [ ] **Dark Autonomy Test**: Charge supercaps to $5.0\text{V}$, disconnect panel, and confirm continuous logging for $\ge 45\text{ minutes}$.

---

## 6. Common Failure Modes & Troubleshooting

| Symptom | Root Cause | Solution |
|---|---|---|
| BQ25570 outputs 0V on VOUT | Voltage below cold-start ($< 330\text{mV}$) or backwards supercaps | Illuminate panel with 60W lamp; verify supercap series polarity. |
| ESP32-C3 resets when transmitting | Instantaneous voltage sag on VOUT | Add a $100\ \mu\text{F}$ low-ESR electrolytic capacitor directly across ESP32 3V3 and GND pins. |
| ADC reading saturates at 4095 | Divider ratio too low; voltage exceeds pin limit | Check resistor values; replace upper resistor with larger value. |
| ELF channel shows only 50/60 Hz hum | Mains EMI coupling into high-impedance coil | Shield preamp in copper tape; move search coil $\ge 2\text{m}$ away from AC mains wiring. |
| Zero telemetry frames decode | Dev key mismatch or corrupted byte | Ensure `firmware/examples/decode.rs` uses identical `[0x42; 32]` key. |

---

## 7. Roadmap to Field Node v2 (Light Sleep & LoRa)

To transition from bench prototype to long-term autonomous field station:
1. **Enable ESP-HAL Light Sleep**: Replace busy-wait loops with timer-driven light sleep. Drops idle draw from $20\text{ mA}$ to $0.8\text{ mA}$, extending dark autonomy from **45 minutes to 4.3 hours**.
2. **Wire BME280 I2C**: Activate `esp_hal::i2c::master::I2c` on GPIO4/GPIO5 to populate temperature, pressure, and humidity in telemetry packets.
3. **Wireless Telemetry**: Connect an SX1262 LoRa module on SPI to broadcast telemetry frames over kilometers at $< 0.01\text{ Wh/day}$.
4. **Provision NVS Session Keys**: Replace the hardcoded `0x42` dev key with asymmetric key exchange or factory-flashed NVS keys.

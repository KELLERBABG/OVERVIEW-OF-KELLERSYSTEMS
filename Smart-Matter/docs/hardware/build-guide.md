# Smart Matter Hardware Build Guide

## Tools Required

- Soldering station with fine tip (≤1 mm)
- Hot air rework station (for QFN/LGA packages)
- Digital multimeter
- Oscilloscope (recommended: 2-channel, ≥50 MHz)
- Tweezers, flux, solder wick
- 3D printer for enclosure (optional)

## Assembly Steps

### 1. PCB Preparation
1. Order PCBs with ENIG finish, 1.6 mm thickness, 2-layer minimum
2. Apply solder paste using a stencil (recommended for QFN/LGA)
3. Inspect under magnification before soldering

### 2. Soldering Order (bottom to top)

1. **0402 components** (C3, R1, R2) — use fine tip iron at 320°C
2. **0805 components** (C2, L1) — slightly more paste
3. **SOIC-8** (U1, STM32) — tack corners, then drag solder
4. **SOT-23** (U4, MCP1700) — similar to SOIC
5. **QFN-20** (U2, nRF24L01+) — hot air at 280°C, flux under package
6. **LGA-16** (U3, LIS3DH) — hot air, same as QFN
7. **QFN-16** (U5, BQ25504) — hot air
8. **SDIP-4** (D1, bridge rectifier) — through-hole, solder on bottom
9. **Supercapacitor** (C1) — radial leads, solder on bottom
10. **Piezo element** (PZ1) — solder wires to PCB pads
11. **Antenna** — PCB trace, no assembly needed

### 3. First Power-Up

1. **Visual inspection:** Check for bridges, cold joints, shorts
2. **Continuity test:** Check VDD and GND for shorts (should be >1 kΩ)
3. **Apply vibration:** Tap the piezo element
4. **Check voltages:**
   - BQ25504 output: 3.3 V (once supercap charges above 330 mV)
   - MCP1700 output: 1.8 V
5. **Check current:** Should be < 1 μA in deep sleep

### 4. Firmware Flashing

1. Connect SWD programmer (ST-Link) to SWDIO, SWCLK, GND
2. Flash bootloader first:
   ```bash
   probe-rs run --chip STM32G031J6 firmware/bootloader/target/bootloader.bin
   ```
3. Flash application firmware:
   ```bash
   probe-rs run --chip STM32G031J6 firmware/node/target/smart-matter-node.bin
   ```

## Enclosure

For concrete embedding:
1. 3D print enclosure from ABS or PETG
2. Apply silicone conformal coating to PCB
3. Pot with epoxy for moisture protection
4. Test vibration transmission: enclosure must not dampen piezo response
# Smart Matter Compute Node — Bill of Materials

## Base Configuration (2.4 GHz, for prototyping / free-air deployment)

| Ref | Component | Description | Package | Qty | Cost (USD) |
|-----|-----------|-------------|---------|-----|------------|
| U1  | STM32G031J6 | Ultra-low-power ARM Cortex-M0+ MCU, 64 KB Flash, 8 KB RAM | SOIC-8 | 1 | $1.50 |
| U2  | nRF24L01+ | 2.4 GHz transceiver, 250 kbps–2 Mbps, 11.3 mA TX peak | QFN-20 | 1 | $1.80 |
| U3  | LIS3DH | 3-axis accelerometer, ±2g/±4g/±8g/±16g, low-power | LGA-16 | 1 | $1.20 |
| U4  | MCP1700-3302E | 3.3V LDO regulator, 250 mA max, 2 μA quiescent | SOT-23 | 1 | $0.35 |
| U5  | BQ25504 | Ultra-low-power boost converter with MPPT for energy harvesting | QFN-16 | 1 | $3.50 |
| D1  | DF10S | Bridge rectifier, 1 A, 100 V | SDIP-4 | 1 | $0.25 |
| C1  | SCMR18C105MRBA0 | Supercapacitor, 1 F, 5.5 V, radial lead | Radial | 1 | $2.50 |
| C2  | 10 μF ceramic | Input/output decoupling | 0805 | 3 | $0.15 |
| C3  | 100 nF ceramic | Bypass capacitor | 0402 | 5 | $0.10 |
| R1  | 10 kΩ 1% | Pull-up, I²C | 0402 | 2 | $0.05 |
| R2  | 1 MΩ 1% | Discharge resistor for supercap | 0402 | 1 | $0.05 |
| L1  | 22 μH | Boost converter inductor | 0805 | 1 | $0.30 |
| Y1  | 16 MHz | MCU crystal, ±10 ppm | HC-49S | 1 | $0.40 |
| ANT1 | 2.4 GHz PCB trace antenna | Inverted-F, 50 Ω | PCB | 1 | — |
| PZ1 | PZT-5A disk | Piezoelectric element, 20 mm × 1 mm | Disk | 1 | $2.00 |

**Total per node (2.4 GHz):** ~$14.15 (qty 50+)

## Concrete-Embedded Configuration (Sub-1 GHz for better concrete penetration)

| Ref | Component | Description | Change | Delta |
|-----|-----------|-------------|--------|-------|
| U2  | **CC1101** | **Sub-1 GHz transceiver, 868/915 MHz, better concrete penetration** | Replace nRF24L01+ | +$0.70 |
| ANT1 | **Helical PCB antenna for 868/915 MHz** | Quarter-wave, 50 Ω | Replace 2.4 GHz trace | +$0.00 |

> **Recommendation:** Use 2.4 GHz for prototyping and free-air nodes. Switch to CC1101 (Sub-1 GHz) for nodes embedded in concrete or asphalt. The Sub-1 GHz band penetrates structural materials significantly better (5–10× range improvement through concrete).

## Optional Components
| Ref | Component | Description | Cost |
|-----|-----------|-------------|------|
| U6  | DS18B20 | Digital temperature sensor, ±0.5°C | $1.50 |
| U7  | AT25SF041 | 4 Mbit SPI Flash for data logging (external, reduce wear on internal Flash) | $0.80 |
| U8  | CC1101 | Sub-1 GHz transceiver (alternative to nRF24L01+ for concrete embedding) | $2.50 |
| LED1 | Red 0603 | Status indicator | $0.05 |

## Assembly Notes
- All components are hand-solderable with basic equipment (hot air + fine tip iron).
- PCB should be ENIG finish for reliability in concrete embedding.
- Conformal coating recommended for moisture protection.
- **For concrete embedding:** Use the Sub-1 GHz configuration. Test range before final encapsulation.

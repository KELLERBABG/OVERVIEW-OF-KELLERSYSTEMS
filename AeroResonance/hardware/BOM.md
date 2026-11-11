# Bill of Materials — EHAR Field Node MVP (~120–150 €)

Prices are typical street prices (DigiKey/Mouser for components, Amazon/eBay
for breakout boards); **verify at purchase** — they drift. Every board/module
is breadboard-compatible. Quantities for one node + small spares.

## 1. Power train (the part that makes it work)

| # | Part | Qty | Unit € | Source | Notes |
|---|------|-----|--------|--------|-------|
| 1 | 6 V 2 W polycrystalline PV panel (~140×110 mm) | 1 | 10 | Amazon/eBay | 2 W is 30× the node's need; any 5–6 V 1–3 W panel works |
| 2 | **BQ25570** boost charger breakout (TI eval / generic) | 1 | 18 | DigiKey, Mouser, Tindie | MPPT + supercap storage + 3.3 V regulated out. *Budget alternative: CN3065/CN3791 solar Li-ion charger board (4 €) + 18650 LiFePO4 (8 €).* |
| 3 | Supercap 2.7 V 50 F (×2, series ⇒ 25 F @ 5.4 V) | 2 | 7 | DigiKey/Mouser | Kemet/Illinois Capacitor FT-series or generic; ≤ 5.5 V total |
| 4 | Schottky diode 1N5819 (reverse-polarity protection) | 2 | 0.3 | any | one on PV+, one on VSTOR |
| 5 | Fuse 0.5 A PTC resettable | 1 | 1 | any | on the PV line |

**Power total ≈ 44 €** (or ≈ 28 € with the budget Li-ion option).

## 2. Core electronics

| # | Part | Qty | Unit € | Source | Notes |
|---|------|-----|--------|--------|-------|
| 6 | **ESP32-C3 SuperMini** dev board | 1 | 5 | Amazon/eBay | RISC-V, WiFi/BLE-capable, native USB-serial; 3.3 V logic |
| 7 | **BME280** breakout (I2C) | 1 | 6 | Amazon | T/H/P sensor — the "thermodynamic" channel |
| 8 | **DS18B20** waterproof probe | 1 | 4 | Amazon | ground temperature → real ΔT measurement |
| 9 | 4.7 kΩ resistors (I2C pull-ups, DS18B20 pull-up) | 3 | 0.1 | any | |
| 10 | 100 kΩ ×4, 1 MΩ ×2, 10 kΩ ×2 resistors | 1 set | 3 | any | dividers + ELF/piezo bias |
| 11 | 100 nF, 1 µF, 10 µF MLCC + 100 µF electrolytic | 1 set | 3 | any | decoupling, PV ripple |
| 12 | **MCP6002** or **LMV358** dual rail-to-rail op-amp (DIP) + 8-pin socket | 2 | 1.5 | any | ELF preamp + piezo amp (CRITICAL: 3.3V single-supply rail-to-rail; do not use TL072) |
| 13 | 2.5 mm audio jack + shielded cable | 1 set | 4 | any | ELF loop connection |
| 14 | TVS diode SMBJ5.0A or P6KE6.8CA | 3 | 0.5 | any | clamp on PV, ELF input, antenna |
| 15 | Breadboard + jumper wires, or perfboard + headers | 1 | 8 | any | start breadboard, move to perfboard later |
| 16 | IP65 junction box (~120×80×50 mm) | 1 | 8 | hardware store | outdoor enclosure |
| 17 | USB-C cable (data) + small USB power bank (bench tests) | 1 | 10 | any | bench bring-up |

**Core total ≈ 53 €**

## 3. M1.3 instrumentation payload (the "atmospheric resonance" part)

| # | Part | Qty | Unit € | Source | Notes |
|---|------|-----|--------|--------|-------|
| 18 | Ferrite rod antenna (AM radio type, ~10 cm) | 1 | 3 | electronics shop | ELF/Schumann pickup |
| 19 | Air-core loop (20–50 turns enameled wire, 30 cm hoop) | 1 | 2 | DIY | better low-frequency response than ferrite; both optional |
| 20 | Piezo disc (Ø 27 mm) + lead wire | 1 | 1 | any | experimental vibration/infrasound channel |
| 21 | (Optional, v2) RTL-SDR dongle | 1 | 30 | Amazon | the *proper* M1.3 spectrum tool; the on-board ELF amp is the MVP stand-in |

**Payload total ≈ 6 €** (36 € with RTL-SDR)

## 4. Tools (assume you have some)

Soldering iron + solder, multimeter, wire strippers, helping hands,
hot glue gun. Breadboard needs no soldering at all for step 1.

---

## Grand total

| | € |
|---|---|
| Power train | ~44 |
| Core electronics | ~53 |
| M1.3 payload | ~6 |
| **Total (without RTL-SDR)** | **~103 €** |
| With RTL-SDR | ~133 € |

## Ordering checklist (one cart, ~10 lines)

1. 6 V 2 W solar panel
2. BQ25570 breakout
3. 2 × 50 F 2.7 V supercap
4. ESP32-C3 SuperMini
5. BME280 breakout
6. DS18B20 waterproof probe
7. MCP6002 / LMV358 rail-to-rail op-amp + socket, 1N5819, TVS diodes, resistor/capacitor assorted
8. Breadboard + wires, IP65 box, USB-C cable
9. Ferrite rod, piezo disc, hookup wire
10. (optional) RTL-SDR

> ⚠️ **Compatibility note:** the ESP32-C3 SuperMini appears in many variants
> (antenna type, LED pin, GPIO breakout). Buy one with castellated GPIO
> headers broken out on *both* sides; the firmware's pin table is adjustable
> in `firmware/esp32c3/src/main.rs` (`PIN_*` consts).

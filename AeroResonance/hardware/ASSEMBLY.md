# Assembly Guide — EHAR Field Node MVP

Three stages: **bench** (USB power, prove the loop) → **breadboard** (all
wiring, panel still bench-lit) → **perfboard + box** (outdoor). Read
[SAFETY.md](SAFETY.md) first. Every stage is testable on its own.

> **Grounding rule of thumb:** one star ground point (the BQ25570 GND pad).
> All sensor/analog grounds return there. The ESP32-C3's USB ground connects
> to the same rail while on the bench — do NOT connect USB and mains-powered
> equipment to the node at the same time.

## Stage 0 — Bench bring-up (USB only, no solar)

1. Flash the firmware (see `firmware/esp32c3/README.md`; needs `espflash`).
2. Plug the ESP32-C3 SuperMini into a USB port. Open a serial terminal at
   115200 baud (the C3's native USB-serial needs no driver on Win10+/Linux).
3. Expect the boot banner + one `[seq=0] … GHOST …` line per interval. The
   "GHOST" line is the **encrypted** frame (hex) — you cannot read it without
   the key, by design; the plaintext header above it is for humans.

## Stage 1 — Breadboard: sensors + dividers

Pin map (the GPIO literals in `firmware/esp32c3/src/main.rs`):

| ESP32-C3 GPIO | Function | Wiring |
|---------------|----------|--------|
| 3V3 | 3.3 V rail | BQ25570 VOUT (Stage 2) or USB 3V3 (Stage 1) |
| GND | ground | star ground |
| GPIO0 (ADC1_CH0) | V_cap monitor | divider: VSTOR ─100kΩ─┼─100kΩ─GND, junction → GPIO0 |
| GPIO1 (ADC1_CH1) | PV monitor | divider: PV+ ─100kΩ─┼─100kΩ─GND, junction → GPIO1 |
| GPIO2 (ADC1_CH2) | ELF preamp out | AC-coupled: preamp out ─1µF─ GPIO2, 100 kΩ to GND |
| GPIO3 (ADC1_CH3) | Piezo amp out | AC-coupled: amp out ─1µF─ GPIO3, 100 kΩ to GND |
| GPIO4 / GPIO5 | I2C SDA / SCL (v2, BME280) | 4.7 kΩ pull-ups to 3V3 |
| GPIO6 | DS18B20 data (v2) | 4.7 kΩ pull-up to 3V3 |

**Dividers:** both use 100 kΩ/100 kΩ ⇒ halve the voltage. VSTOR can reach
5.4 V ⇒ 2.7 V at the pin — inside the ADC's ~2.5–3.1 V range at 11 dB
attenuation, but *check with a multimeter*; if your supercaps reach 5.4 V,
use 220 kΩ/100 kΩ (÷3.2) instead and update `V_CAP_DIVIDER` in the firmware.

### ELF preamplifier (M1.3 channel, ~10 Hz–1 kHz)

```
ferrite rod / air loop ── 10 nF ──┬── 100 kΩ ──┬── MCP6002 A (+)   MCP6002 A (−) ─ 10 kΩ ─ out
        (two ends)                └─ 100 kΩ ───┘   (non-inv)       MCP6002 A (−) ─ 10 kΩ ─ 100 kΩ ─ out
```
Use the classic non-inverting amplifier with a 100 kΩ/100 kΩ bias network (setting a mid-rail 1.65 V virtual ground) and
~10× gain (100 kΩ / 10 kΩ feedback). Note: **do NOT use TL072** on a 3.3 V single-supply; use a true 3.3 V rail-to-rail
op-amp such as MCP6002 or LMV358. Band-limit with a 100 nF
across the feedback resistor (f₋₃dB ≈ 1/(2πRC)). Keep leads short and twisted;
the loop is the antenna — keep it ≥ 30 cm from the MCU and PV wiring.

### Piezo channel

Piezo disc → 1 MΩ bias to GND → MCP6002 B buffer (unity gain) → AC-couple to
GPIO3. This channel is experimental (infrasound demo); it drives no power
logic.

## Stage 2 — Breadboard: the power train

```
PV panel (+) ──[fuse]── 1N5819 ──► BQ25570 VBAT_IN? (see note) ──► VSTOR ──► supercap +
PV panel (−) ───────────────────► GND
supercap − ───────────────────► GND
BQ25570 VOUT (3.3 V) ──► ESP32-C3 3V3
BQ25570 VOUT ──► BME280 VCC, pull-ups to 3V3 (v2)
```

> **BQ25570 note:** the generic "BQ25570 breakout" boards differ in exposed
> pads (some bring out `VBAT`, some `VSTOR`/`VOUT`, some `VBAT_OK`).
> Wire per the *board's* silkscreen: PV+ → `VIN_DC` (or `VBAT_IN` via the
> board's own diode), storage → `VSTOR`, load → `VOUT` (or `VBAT`).
> If your breakout exposes jumpers for MPPT/VBAT_OK dividers, leave them at
> factory defaults for a first run; tweak after Stage 2 works.
> If you chose the budget Li-ion path instead: CN3065 module IN ← PV,
> B+/B− ← 18650 LiFePO4, OUT → ESP32-C3 3V3 — no supercaps.

Set the board's `VOUT` to 3.3 V if adjustable (many breakouts are fixed 3.3 V).
Measure with a multimeter before connecting the ESP32-C3.

## Stage 3 — Perfboard + enclosure (outdoor)

1. Move the circuit to perfboard: power first (panel → BQ25570 → supercap →
   regulator), verify each rail, then sensors, then the ESP32-C3.
2. Mount the PV panel outside the box (its own bracket), route its cable
   through a cable gland. Point it south (northern hemisphere), tilted
   ≈ latitude + 10°.
3. Supercaps inside the box (they are fine from −40 °C; derate at high temp).
4. ELF loop: either outdoors on a small mast or indoors at a window — keep it
   away from mains wiring and the PV panel. Surge-clamp its leads (TVS to
   ground at the box entry).
5. DS18B20 probe (v2): buried/grounded outside, cable through a gland.
6. Seal the box (IP65), strain-relieve all cables. Ground the box to a real
   earth stake if it contains any antenna feed — see SAFETY.md.
7. Solar cable: fuse at the panel end (0.5 A PTC), diode for reverse
   protection, TVS at the box entry.

## Quick wiring checklist (before power-up)

- [ ] Polarity: PV and supercaps connected the right way (Schottky diode
      protects VSTOR; verify diode direction)
- [ ] No 3.3 V rail shorted to GND (ohmmeter)
- [ ] VSTOR not above 5.5 V (2 × 2.7 V caps = 5.4 V max — correct)
- [ ] GPIO0 divider junction ≤ 3.1 V at max VSTOR (multimeter)
- [ ] I2C pull-ups on the 3.3 V rail, DS18B20 pull-up too
- [ ] USB unplugged while the panel is connected (no dual-power)

# Bring-up & Acceptance — EHAR Field Node MVP

A numbered sequence. Each step has a pass/fail criterion; do not skip ahead
on a fail. "Viable" is defined in [POWER_BUDGET.md](POWER_BUDGET.md) §6.

## Stage 0 — Bench (USB, no solar)

1. **Flash.** `cd firmware/esp32c3 && cargo run --release --target
   riscv32imc-unknown-none-elf` (installs `espflash` on first use). Pass: boot
   banner appears in the serial terminal (115200 baud).
2. **Telemetry loop.** Pass: one `[seq=N] … GHOST <hex>` line per interval,
   `seq` increments, no panics for 1 h.
3. **Tier sanity.** With the SuperMini powered from USB only (V_cap pin reads
   ~0, tier = T-1), the header must report `tier=T-1-DeepHibernate` and the
   longest interval (3600 s — the no-harvest fallback). This proves the
   scheduler + planner path end-to-end.
4. **Entropy.** The TRNG line at boot shows a non-zero bit rate; two identical
   USB power-ups must give *different* telemetry (fresh entropy), while the
   core's unit tests prove determinism for identical inputs.

## Stage 1 — Sensors on the bench

5. **V_cap divider.** Feed 5.0 V from a bench supply (or the charged bank)
   into the divider; compare reported `vcap=…mV` against the multimeter.
   Pass: within ±10 %; if not, adjust `ADC_11DB_REF_V` / `V_CAP_DIVIDER` and
   record the calibration in this file.
6. **ELF channel.** Wave a phone playing a 20–50 Hz tone near the loop, or
   run a mains-free DC motor nearby. Pass: the `elf=` value visibly changes
   with the source on/off. (Real Schumann lines need the v2 FFT path or the
   RTL-SDR — the raw ADC value is a presence detector, not a spectrum.)
7. **Charge from bench light.** Point a 60 W LED work lamp at the panel.
   Pass: `pv=` voltage rises with light, and the VSTOR voltage climbs over
   minutes on a drained bank.

## Stage 2 — Solar + storage

8. **Autonomy.** Fully charge the bank (VSTOR ≈ 5.0 V), disconnect the panel,
   run the node. **v1 pass: node keeps reporting for ≥ 45 min** (predicted
   ≈ 50 min at ~20 mA busy-wait; measure the actual draw with a USB meter in
   the same step and record it here — it should be 15–25 mA). After the v2
   light-sleep lands, this test becomes ≥ 4 h (predicted 4.3 h).
9. **Charge rate.** From fully discharged, panel in full sun. Pass: VSTOR
   reaches 5.0 V within one good day (predicted: ~1 h of sun).
10. **Full cycle.** Leave it running 24 h indoors with the panel on a windowsill.
    Pass: no reset, `seq` gap-free, tier rises during the day and falls at
    night — the V_cap trace should look like the simulator's `V_cap` column.

## Stage 3 — Field (7-day acceptance)

11. **Deploy.** Box outdoors, panel south-facing, loop at a window/mast,
    USB unplugged. Log via... (v1 has no radio: either a laptop reconnected
    daily, or skip to v2 ESP-NOW for the real 7-day test — record which).
12. **7-day pass criteria**
    - [ ] 100 % uptime (no watchdog resets; `seq` continuous)
    - [ ] All frames decrypt with the dev key (or: at least the header's
          auth path exercises ChaCha20-Poly1305 on every packet — tamper test:
          flip one hex byte, the frame must fail to parse)
    - [ ] Supercap never below the T-1 threshold for > 24 h straight (planner
          keeps the node alive through one bad day)
    - [ ] Measured daily energy ≈ predicted (POWER_BUDGET §2) within a factor
          of 2 — update POWER_BUDGET.md with the real numbers
13. **Sim cross-check (the science).** Run
    `cargo run -- --config 50x50mm_config.json --duration 86400 --dt 60 --seed 42
    --gate-report` and compare the sim's V_cap / tier trajectory with the
    node's logged trace. Discrepancies > 20 % mean the model needs tuning
    (that is a *finding*, not a failure — record it in the gate report
    addendum).

## v2 roadmap (after acceptance)

- BME280 + DS18B20 over I2C (real T/H/P + ground ΔT → compare with TEG model)
- ESP-NOW telemetry (`esp-radio` crate) — no frame-format change
- Light-sleep duty cycle (20 mA → ~1 mA), then deep sleep between hourly
  frames (→ ~0.3 Wh/day → ~0.2 Wh/day)
- On-device rolling average of harvest → replace the `HARVEST_AVG_W` const
  with measured data (closes the loop with `plan_duty`)
- NVS provisioning of the Ghost-Net key (replaces the dev key)

## Definition of done ("viable")

A node that, once built, **runs unattended for 7 days on solar + supercaps,
reports authenticated telemetry every cycle, and produces data that can be
compared against the simulator.** Everything in this file is the checklist
for that sentence.

# Power Budget — EHAR Field Node MVP

Why this node is **viable** (and the resonance harvesters, on their own, are
not). All numbers are deliberately conservative; the point is the margin, not
the precision. Update this file with measured numbers after BRINGUP step 7.

## 1. The load (what the node actually consumes)

ESP32-C3 figures (typical, from datasheet + community measurements):

| State | Current @3.3 V | Power |
|-------|---------------|-------|
| Active, radio off, ADC+I2C sampling | ~20 mA | 66 mW |
| Active, WiFi TX burst | ~80–100 mA | 264–330 mW |
| Light sleep | ~0.8 mA | 2.6 mW |
| Deep sleep | ~5 µA | 0.017 mW |

**Two firmware generations — read the right column:**

| | **v1 (current code)** | v2 (light-sleep, roadmap) |
|---|---|---|
| Sleep mechanism | busy-wait `delay_millis` loop | esp-hal light-sleep |
| Idle draw | ~20 mA (CPU at 160 MHz) | ~0.8 mA |
| Duty (60 s interval, 5 s active burst) | ~66 mW continuous | 5 s @ 66 mW + 55 s @ 2.6 mW |
| **Daily energy need** | **≈ 1.6 Wh** | ≈ 0.31 Wh |
| Autonomy on a full 25 F bank (5.0→3.0 V) | **≈ 0.8–1 h** | ≈ 4.3 h |

> The firmware's `LoadProfile` in `esp32c3/src/main.rs` is set to the v1
> (busy-wait) figures so the on-device autonomy/interval reports are honest
> for the code that actually runs. The v1 duty *planner* is unchanged — only
> the numbers it is fed.
>
> **Verdict:** v1 is a bench / good-sun prototype; **v2 light-sleep is what
> makes the field deployment robust** (that is the first item on the v2
> roadmap, before ESP-NOW).

## 2. The harvest (what the PV delivers)

2 W panel, 4 peak-sun-equivalent hours (central-Europe yearly average ≈ 3.3 h):

| Condition | Daily energy (2 W panel) |
|-----------|--------------------------|
| Ideal (2 W × 4 h) | 8 Wh |
| System efficiency 50 % (charge + storage + regulation losses) | **4 Wh** |
| Worst month, overcast (2 W × 1.5 h × 0.5) | 1.5 Wh |

**Compare (v2 / v1):**

| Node draw | Harvest on a good day (4 Wh) | Harvest, worst month (1.5 Wh) |
|-----------|------------------------------|-------------------------------|
| v2 ≈ 0.31 Wh/day | 13× margin | 5× margin |
| v1 ≈ 1.6 Wh/day | 2.5× margin | ~1× (marginal — needs a bigger panel or v2) |

The sim's PV micro-cell (0.25 mW/cell) is *not* the reference here: that
models a single 20×20 mm cell clamped at 1 mW. The MVP uses a real 2 W
panel; the firmware's `estimate_daily_harvest_wh` and the hybrid report use
panel-scale numbers.

## 3. The store (supercapacitor bank)

2 × 50 F 2.7 V in series ⇒ **25 F @ 5.4 V**.

| Metric | Value |
|--------|-------|
| Energy at 5.0 V | ½·25·5² = 312 J = 87 mWh |
| Usable down to 3.0 V (PMIC cutoff) | ½·25·(25−9) = 200 J = **56 mWh** |
| Autonomy on stored energy alone (no sun) — **v1 busy-wait** | 200 J ÷ 66 mW ≈ **0.8–1 h** |
| Autonomy on stored energy alone (no sun) — **v2 light-sleep** | **~2.5–19 h**, cadence-dependent: ~2.5 h reporting every ~60 s under light cloud, ~19 h throttled to hourly with zero sun (planner behavior, see `firmware/tests/day_cycle.rs`) |

Supercap choice rationale: no lithium to manage, 500k+ cycle life, no fire
risk, works from −40 °C, and it is exactly what the simulation models
(`buffer_capacitance_f`, `cap_aging`). The Li-ion budget alternative gives
~10× more storage but adds charge/discharge management.

## 4. Duty-cycle plan (from the firmware)

`ehar_firmware_core::plan_duty(store_energy, harvest_avg, load)`:

- With a **full store** (312 J) and **0.02 W average harvest** (very cloudy),
  the planner proposes an interval of ≈ 0.33 J / 0.02 W ≈ **17 s** (clamped
  to the 30 s minimum) → heavy reporting, ~5–8 mW average.
- With a **low store** and **zero harvest**, the planner falls back to the
  3600 s maximum → one sample per hour, ~2.6 mW sleep-dominated draw.
- The tier mapping (`T-1 … T3` by V_cap) is the *same* `eh-sim-core`
  scheduler the desktop simulation uses — run `cargo run -- --hybrid-report`
  and the sim's V_cap trajectory predicts the node's tier behaviour.

## 5. The resonance channels (honest numbers)

From the simulator (24 h reference run, gate report §6):

| Channel | Average power |
|---------|---------------|
| ELF/Schumann + all resonance harvesters combined | ≈ 0.001 mW (≈ 0.4 % of the PV budget) |
| TEG at the *measured* 0.2 °C ΔT | ~µW — sensor only |

These channels are **measurement payload**, not power. Their µW cannot run the
ESP32's radio; treat any harvested fraction as a bonus to log and compare with
the sim.

## 6. Acceptance threshold ("viable" is defined, not vibes)

| Test | Threshold | Where |
|------|-----------|-------|
| Bench: autonomy from full store, no sun — **v1** | ≥ 45 min (we predict ~50 min) | BRINGUP step 8 |
| Bench: autonomy from full store, no sun — **v2 light-sleep** | ≥ 2 h (planner throttles; see day_cycle test) | v2 |
| Field: 7-day run without touching it | 100 % uptime, all frames authenticated | BRINGUP step 9 |
| Charge: full store from dead in ≤ 1 good day | ≤ 1 day at 4 Wh | BRINGUP step 7 |

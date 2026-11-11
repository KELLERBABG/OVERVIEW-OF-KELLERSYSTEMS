# ATA refrigeration thermostat — Tinkercad Circuits build guide

A browser-buildable prototype of **Ambient Thermal Arbitrage (ATA)** using an
**Arduino Uno R3** in [Tinkercad Circuits](https://www.tinkercad.com/circuits).
It runs the real control logic from `thermal-arbitrage-mvp/src/control/mod.rs`
on a simulated cold room.

Sketch: [`refrigeration_thermostat.ino`](./refrigeration_thermostat.ino)

> **Why this project fits Tinkercad.** Tinkercad only simulates an Arduino Uno
> and a fixed part library (LEDs, TMP36 temperature sensor, potentiometers,
> pushbuttons, LCD, piezo, motors…). ATA's *control loop* is "temperature in →
> decision → compressor/fan/alarm out", which maps cleanly onto those parts. The
> market/PLC/cloud parts of ATA are out of scope here.

---

## 1. Parts list (add from the Tinkercad component panel)

| Qty | Tinkercad part | Purpose |
|----|----------------|---------|
| 1 | Arduino Uno R3 | runs the plant model + control engine |
| 1 | Small Breadboard | wiring |
| 1 | LCD 16 x 2 | shows temp, setpoint, mode, compressor, lock timer |
| 1 | Temperature sensor [TMP36] | **ambient (outside) temperature** input |
| 2 | Potentiometer | electricity price (A1) and grid-frequency deviation (A2) |
| 1 | Pushbutton | "door open" heat-load disturbance |
| 3 | LED (green / blue / red) | compressor / evaporator fan / alarm |
| 3 | Resistor 220 Ω | LED current limiting |
| 1 | Piezo (buzzer) | emergency alarm |
| 1 | Resistor 1 kΩ (or a 10 kΩ pot) | LCD contrast (V0) |

---

## 2. Wiring

**Arduino Uno R3 pin labels** (as they appear in Tinkercad): the digital header
reads **AREF, GND, 13 → 0** — digits only, no `D` prefix (the six PWM pins are
marked `~`). The power/analog header reads **IOREF, RESET, 3.3V, 5V, GND, GND,
Vin**, then **A0 … A5**.

**LCD (4-bit mode)** — the Tinkercad LCD pins run left→right:
`GND, VCC, V0, RS, RW, E, DB0, DB1, … DB7, LED, LED`.

| LCD pin (left→right) | Connect to |
|---|---|
| GND | Arduino **GND** |
| VCC | Arduino **5V** |
| V0 | GND via 1 kΩ (contrast) |
| RS | Arduino **12** |
| RW | Arduino **GND** |
| E | Arduino **11** |
| DB0–DB3 | *not connected* (4-bit mode) |
| DB4 | Arduino **5** |
| DB5 | Arduino **4** |
| DB6 | Arduino **3** |
| DB7 | Arduino **2** |
| LED (backlight +) | 5V via 220 Ω |
| LED (backlight −) | GND |

**Inputs**

| Part | Connection |
|---|---|
| TMP36 | flat face toward you: left→**5V**, **middle→A0**, right→**GND** |
| Price pot | outer1→5V, outer2→GND, **wiper→A1** |
| Grid-freq pot | outer1→5V, outer2→GND, **wiper→A2** |
| Door button (4 legs) | Arduino **6** to one leg, **GND** to the *diagonally opposite* leg; the other two stay unconnected (code uses `INPUT_PULLUP`) |

> **4-leg pushbutton.** A tactile switch has two internally-connected pairs —
> the two legs on each side of the body are permanently joined. So take one leg
> from *each* side; the easy rule is the two **diagonally opposite** legs. If you
> instead pick two legs that sit on the same side, the button reads as
> permanently pressed (the room will alarm the moment you start the sim).

**Outputs**

| Part | Connection |
|---|---|
| Green LED (compressor) | anode→220 Ω→**8**, cathode→GND |
| Blue LED (fan) | anode→220 Ω→**9**, cathode→GND |
| Red LED (alarm) | anode→220 Ω→**10**, cathode→GND |
| Piezo | + →**7**, − →GND |

Pins **0/1** (RX/TX) are left free so the USB serial monitor keeps working.

---

## 3. Build steps

1. Tinkercad → **Create new → Circuits**. Drag in the parts above.
2. Wire per section 2 (the LCD is the fiddliest part — take it slow).
3. Click the code panel, switch to **Text**, and paste the whole
   `refrigeration_thermostat.ino` (delete the default sketch first).
4. **Start Simulation.** The red LED + buzzer should stay off; the LCD shows
   `T-20.0 SET-21.0` then a mode line.
5. Open **Serial Monitor** (9600 baud) for the CSV log.

---

## 4. What to try (the demo scenarios)

| Action | What you should see |
|---|---|
| Let it run | It toggles between cooling and warming; the compressor LED turns **on/off** and the `L…` number on the LCD counts down the **anti-short-cycle lock**. |
| Watch a switch | After the compressor turns off, it will **not** restart until the 600 s (10 s) min-off lock expires — even if the temp already demands it. That is the short-cycling governor. |
| Hold the **door** button | Temperature ramps up fast; if it keeps rising the room climbs toward −18 °C. |
| Turn the **price** pot low (< 30 €/MWh) | Mode → `MPC` → **pre-cool**: compressor runs hard to bank cold energy while power is cheap (temp drives down toward −24 °C). |
| Turn the **price** pot high (> 80 €/MWh) | Mode → `ARBIT`: compressor **sheds** (stays off) while energy is expensive. Hold it there and the room warms past −18 °C → `EMERG` — the product-integrity risk of shedding. |
| Hold the **door** button (easiest alarm) | Heat load spikes → `ttf` drops below the trip → mode flips to `EMERG`: red LED + buzzer, setpoint pinned to −24 °C. Release and it latches off only once temp recovers below −21 °C. |
| Drag the **TMP36** slider toward its max (≈ 60 °C) | The outside load exceeds what the plant can remove → `EMERG` + buzzer. |
| Turn the **grid-freq** pot to a side | Mode → `GRID`: +Hz = absorb energy (cool), −Hz = shed load. The safety clamps stop it over-cooling or shedding near the limits. |
| Watch serial | CSV row per second: sim time, Tin, Tamb, price, freq dev, mode, comp, fan, setpoint, lock s, kWh, cost €. |

---

## 5. How the prototype maps to the real system

| ATA concept (`control/mod.rs`, `config.toml`) | In the Tinkercad sketch |
|---|---|
| Cold room thermal model (`r_th`, `c_th`, cooling/heat-leak power) | 1st-order plant: `tInternal += (LEAK_K·(Tamb−Tin) − COOL_K·compOn)·dt` |
| `t_min/t_max/t_setpoint*` (−24/−18/−21) | same constants |
| `min_runtime_sec` / `min_off_time_sec` (900/600 s) | `governorLock()` — min run/off lock on every switch |
| Emergency via `ttf < ttf_critical_sec` | `ttf = (T_MAX − Tin)/rate`, latched until recovery |
| `ThermalGuard` (thermal integral) | excursion integral of `(Tin − setpoint)⁺`, resets when back in spec |
| `GridBalancing` (+deadband, safety clamps) | price-of-grid pot; won't over-cool/over-shed near limits |
| `MpcPreCool` / `Arbitrage` thresholds (30/80, cheap 40 €/MWh) | price pot drives pre-cool / shed / thermostat |
| `normal_thermostat` hysteresis (±1 °C) | same ±1 °C band |
| Evaporator fan run-on after compressor stops | `FAN_RUNON_S` fan run-on |
| `fail_safe_decision` (keep cooling on bad data) | emergency branch forces `compOn = true` |
| PLC / Modbus / cloud / MPC-12h-horizon | **not modelled** (out of scope for Tinkercad) |

---

## 6. Deliberate simplifications

- **Time**: the Uno integrates at 60× real time (`TIME_SCALE`) so 900 s/600 s
  locks are visible in seconds. Change `TIME_SCALE` for slower/faster.
- **Plant constants** (`LEAK_K`, `COOL_K`) are demo-tuned stand-ins for the real
  `r_th`/`c_th` and 50 kW cooling power — they are *not* the physical values
  (the real τ ≈ 17 days would never finish a cycle on screen).
- **Emergency trip** (`TTF_CRITICAL_S`) is scaled down to 400 s for the same
  reason: the real 900 s assumes the 17-day tau. The trip *logic* is unchanged,
  and the hysteresis/lock durations (`MIN_RUNTIME_S`/`MIN_OFF_S`) keep their real
  values.
- **Internal temperature** is modelled in software rather than read from the
  TMP36; the TMP36 supplies **ambient** temperature so you can play the weather.
  (If you prefer to hand-drive the room, wire the TMP36 into A0 as the internal
  sensor and set its slider manually.)
- The **12-hour MPC price lookahead** is reduced to a cheap-window pre-cool.
- Real electricity prices, PLC/Modbus, OTA, alerts, and the ledger are omitted.

---

## 7. Tinkercad gotchas

- Set the **Serial Monitor baud to 9600**.
- If the LCD is blank, adjust the **V0 contrast** resistor (1 kΩ → try 220 Ω–2 kΩ).
- The **TMP36 slider** is the ambient temperature — negative values are fine
  (the real sensor reads −40…+125 °C).
- Both potentiometers default to mid-scale: price ≈ 60 €/MWh (plain thermostat),
  grid deviation ≈ 0 Hz — a good starting state.
- The **4-leg button** must use diagonal legs (see §2). If the sim starts in
  `EMERG` / the door looks stuck, move one button wire to the diagonal leg.

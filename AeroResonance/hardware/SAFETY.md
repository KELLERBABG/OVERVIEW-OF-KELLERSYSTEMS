# Safety — EHAR Field Node MVP

This node is a **low-voltage device** (≤ 5.4 V storage, 3.3 V logic, ~2 W PV)
and is safe to build at home *if* you follow the rules below. The project plan
§8 (lightning, arc flash, HV isolation) applies to the *full-scale* tower
concept — the MVP deliberately stays far away from those hazards. Do not let
the plan's HV ambition leak into this build.

## 1. Electrical — hard rules

- **Never connect the node to USB and to mains-powered equipment at the same
  time.** The supercap bank can deliver tens of amperes into a short. Bench
  testing: USB cable into a *laptop or power bank*, node unplugged from the
  panel. Solar testing: panel connected, USB unplugged.
- **Supercapacitors are not batteries.** They can dump their charge almost
  instantly. Keep the 5.4 V bank inside the box; never touch the terminals
  while charged; discharge through a 10 Ω resistor (or just leave the node
  running) before handling.
- **Polarity is fatal to the BQ25570.** Double-check PV and supercap polarity
  before first power-up (multimeter, then visual check of the Schottky diode
  orientation).
- **Fuse the PV line** (0.5 A PTC) at the panel end — a shorted panel lead
  into a charged bank is a fire/heat risk.
- **Do not exceed 5.5 V on VSTOR** (2 × 2.7 V caps in series). If you use a
  different cap count, recompute.
- **Li-ion/LiFePO4 alternative:** use a cell with its own protection circuit;
  charge only with the CN3065/CN3791 board; never charge below 0 °C; keep the
  cell in the fireproof bag it shipped in until the box is sealed.
- **TVS clamps** (SMBJ5.0A on VSTOR, P6KE on PV and ELF inputs) are cheap
  insurance against static/inductive spikes — fit them.

## 2. The ELF loop / antenna — real lightning rules

The MVP's loop is small, but *any outdoor antenna is a lightning magnet*:

- **Outdoor loop:** route its feedline through the box, clamp both leads to
  ground with TVS at the box entry, and **disconnect the loop when a storm is
  within ~10 km**. A small loop is not a ground rod — do not rely on it.
- **Indoor loop:** at a window is fine; still unplug during storms.
- **Never** mount the loop on a mast that could be the highest point nearby,
  and never ground the node's electronics to a lightning rod.
- If you add the optional RTL-SDR: it is USB-powered and grounded through the
  PC; unplug its antenna in storms too.

## 3. Solar panel

- 2 W panels produce ≤ ~0.4 A — safe, but the open-circuit voltage (≈ 7 V)
  deserves respect: don't short it (sparks), use insulated connectors.
- Mount so water can't track into the connectors (drip loop below the box).
- The panel is glass: handle with gloves; dispose of broken panels properly.

## 4. Tools & environment

- Soldering: lead-free solder, fume extraction or a window, wash hands after.
- Work on a non-conductive mat; remove metal jewelry when working on the
  bench near charged caps.
- The node is a *measurement* device, not a toy — keep it out of reach of
  children and pets (box closed, cable glands tight).

## 5. If something goes wrong

| Symptom | Likely cause | Action |
|---------|--------------|--------|
| Nothing powers up | polarity / fuse | disconnect panel, check fuse, re-check polarity |
| ESP32-C3 hot | 3.3 V over-voltage or short | unplug, check VOUT with multimeter (must be 3.3 V) |
| Supercap bank warm | over-voltage (>5.4 V) or wrong caps | discharge, re-verify series wiring |
| Radio interference on bench | 1 kHz switcher | move loop away; ferrite bead on PV leads |
| Smoke/smell | serious fault | disconnect EVERYTHING immediately, let it sit, find the fault before powering again |

When in doubt, power down, photograph the build, and ask (the repo is a
private research project — an issue/PR description works).

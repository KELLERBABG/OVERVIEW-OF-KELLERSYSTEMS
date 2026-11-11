# HoloForge Safety Documentation

> **STATUS: NOT CERTIFIED.** HoloForge is research-stage software. Nothing in
> this document constitutes a certified safety system, a regulatory
> submission, or compliance with any laser or machine-safety standard. The
> interlock and authentication behaviors described here are implemented in
> software and verified by tests; they have **not** been validated on
> physical hardware, and software alone can never be the sole safety layer
> for a laser product. See [Compliance status](#compliance-status).

## Scope

This document covers the two safety-relevant surfaces of the HoloForge
software:

1. The **405 nm laser controller state machine** in
   `holoforge/hardware/laser.py` (shutter, interlock, emergency stop).
2. The **web control-center hardware-command policy** in
   `holoforge/web/server.py` (laser, stage, and print endpoints).

It does **not** cover electrical safety, resin toxicity, UV exposure to
skin, or machine guarding. Those remain the integrator's responsibility
when building physical hardware.

## Laser safety state machine

The controller implements an explicit state machine with two independent
safety layers that gate every path to optical emission.

### States

| State | Meaning |
|---|---|
| `DISARMED` | Default. Shutter closed, emission refused. |
| `ARMED` | Emission permitted once interlock is closed and e-stop is clear. |
| `EMITTING` | Shutter open, light present. |
| `ERROR` | Reserved for hardware fault reporting. |

### Safety layers

| Layer | Set by | Cleared by | Effect while active |
|---|---|---|---|
| **Interlock** | `set_interlock(False)` (enclosure switch, hardware loop) | `set_interlock(True)` only | Shutter is forced closed immediately; `arm()` and `shutter_open()` raise `RuntimeError`. |
| **E-stop latch** | `emergency_stop()` | `reset_estop()` **only** (manual, explicit) | `arm()`, `shutter_open()`, and `pulse()` all raise `RuntimeError`; power is set to 0; state is `DISARMED`. Not cleared by `disarm()`. |

```
                     arm()                      shutter_open()
   DISARMED ─────────────────► ARMED ──────────────────────────► EMITTING
      ▲                        │  ▲                                  │
      │ disarm()               │  └──────── shutter_close() ─────────┘
      │                        │
      │   emergency_stop()     │ interlock opens: shutter_close()
      └────────────────────────┴── (latched; reset_estop() required)
```

Rules enforced in code, each proven by `tests/test_safety.py`:

- Every activation path (`arm`, `shutter_open`, `pulse`) is refused while
  the e-stop is latched, and while the interlock circuit is open.
- Opening the interlock forces the shutter closed even mid-emission.
- `emergency_stop()` latches; `disarm()` does **not** clear the latch;
  only an explicit `reset_estop()` does, and re-arming is still a
  separate step that re-checks the interlock.

### Hardware requirements this software assumes

Software is **not** a safety layer on its own. A physical HoloForge build
must provide, in hardware, independent of any CPU:

- A **normally-closed interlock loop** (enclosure lid, viewport shutter)
  wired in series with the laser diode driver, such that any open loop
  de-energizes emission without software involvement.
- A **hardware e-stop** (latching, red mushroom switch) that cuts diode
  power directly. The software `emergency_stop()` is a *supplemental*
  control, not a substitute.
- **Lab procedure**: goggles rated for 405 nm at the diode's power class,
  beam enclosure, and no exposure with the interlock defeated.

## Web control-center hardware-command policy

The HTTP API can drive the laser, jog the rotary stage, and execute
prints. Policy (implemented in `_hardware_command_denied`,
`holoforge/web/server.py`):

1. **Cross-origin requests are refused.** If a browser-style `Origin`
   header is present and does not match the request `Host`, the command
   is rejected with `403` (CSRF protection for the local UI).
2. **Token authentication.** If the environment variable
   `HOLOFORGE_API_TOKEN` is set, every hardware command
   (`POST /api/hardware/laser`, `POST /api/hardware/stage`,
   `POST /api/print`) must present it as `Authorization: Bearer <token>`;
   comparison is constant-time. Missing or wrong token → `403`.
3. **Safe by default beyond loopback.** If no token is configured and the
   server is bound to anything other than `127.0.0.1`/`localhost`/`::1`,
   all hardware commands are refused with `403` and an explanatory error.
   Read-only endpoints (`/api/status`, `/api/resins`, `/api/jobs`) remain
   available.
4. **Safety refusals surface as `409`.** When the laser state machine
   refuses an activation (e-stop latched, interlock open), the API
   returns `409 Conflict` with the reason, never a dropped connection.

### Laser endpoint actions

`POST /api/hardware/laser` accepts:

- `arm` — arm the laser (refused while the e-stop is latched or the interlock is open).
- `disarm` — close the shutter and disarm; does **not** clear the e-stop latch.
- `pulse` — timed exposure (auto-arms when disarmed; the same gates apply).
- `estop` — engage `emergency_stop()`: shutter closed, power dropped, **e-stop latched**. The response reports the resulting safety state (`estop_latched`, `is_armed`). Clearing the latch requires a direct `reset_estop()` call on the controller — deliberately not exposed over HTTP, so a remote client can engage but never clear the emergency stop.

The dashboard's E-Stop button dispatches `estop`.

Example:

```bash
export HOLOFORGE_API_TOKEN="$(python -c 'import secrets; print(secrets.token_urlsafe(32))')"
python -m holoforge.cli ui --host 0.0.0.0
# hardware commands now require:
#   Authorization: Bearer <token>
```

## Compliance status

**Not certified. Not submitted. Not validated on hardware.** HoloForge has
not been tested or certified under IEC 60825-1 (or any regional
equivalent such as 21 CFR 1040 in the US). The checklist below is a
*path to follow* for anyone building real hardware, not a record of
completed work.

### IEC 60825 classification path (to be completed by the hardware builder)

- [ ] Determine the laser class of the built system at the point of
      service (a 120–200 mW, 405 nm single-mode diode with the optical
      chain described in the README will typically land in **Class 3B or
      Class 4**).
- [ ] Commission a competent person / notified body assessment for the
      actual emitted power, accessible emission, and beam divergence.
- [ ] Apply the required classification label, aperture label, and
      warning signage to the enclosure.
- [ ] Publish an instruction handbook covering interlock defeat risks,
      eyewear (OD at 405 nm, optical density matched to the measured
      class), and service procedures.
- [ ] Implement engineering controls (enclosed beam path, interlocked
      access) before any powered operation; administrative controls alone
      are insufficient for Class 3B/4.
- [ ] Record measurements and file the classification documentation for
      the specific build; revisit after any optical modification.

### Interlock requirements checklist

- [ ] Enclosure interlock is normally-closed and fails safe (wire break
      = laser off).
- [ ] E-stop is hardware-latched and cuts diode power independent of
      software and independent of the host computer.
- [ ] Software interlock/e-stop state (this document's state machine) is
      treated as a *third*, redundant layer only.
- [ ] Startup behavior: system boots in `DISARMED` with shutter closed
      (verified: `LaserTelemetry` defaults).
- [ ] Interlock defeat (service mode) is keyed/controlled and documented.

## Verification

Every software behavior claimed in this document is asserted by
`tests/test_safety.py` in the automated suite (`python -m pytest -q`):

- unauthenticated hardware commands rejected when exposed or token-guarded;
- cross-origin hardware commands rejected;
- valid token restores hardware command access;
- e-stop latches, gates all activation, and is cleared only by
  `reset_estop()`;
- interlock opening forces the shutter closed and blocks re-activation.

These tests run against the **mock** hardware layer. They prove the
software's behavior, not the safety of any physical build.

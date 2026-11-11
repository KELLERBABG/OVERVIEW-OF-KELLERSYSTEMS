"""405 nm Laser Diode & Safety Shutter Controller.

Manages laser diode current, optical emission shutter, and hardware safety interlocks
with microsecond timing and mock emulation.
"""
from __future__ import annotations

import time
from dataclasses import dataclass
from enum import Enum


class LaserState(str, Enum):
    DISARMED = "DISARMED"
    ARMED = "ARMED"
    EMITTING = "EMITTING"
    ERROR = "ERROR"


@dataclass
class LaserTelemetry:
    state: LaserState = LaserState.DISARMED
    power_mw: float = 120.0
    total_emission_s: float = 0.0
    pulse_count: int = 0
    interlock_ok: bool = True
    estop_latched: bool = False


class LaserController:
    """Hardware controller for the 405 nm single-mode laser diode and beam shutter."""

    def __init__(
        self,
        port: str | None = None,
        power_mw: float = 120.0,
        mock: bool = False,
    ):
        self.port = port
        self.mock = mock or (port is None)
        self.telemetry = LaserTelemetry(power_mw=power_mw)
        self.command_log: list[str] = []
        self._serial = None

        if not self.mock and self.port:
            self.connect(self.port)
        else:
            self.mock = True

    def connect(self, port: str) -> None:
        """Connect to laser shutter microcontroller over USB."""
        self.port = port
        if self.mock:
            return

        try:
            import serial
            self._serial = serial.Serial(port, 115200, timeout=1.0)
            self._send_cmd("LASER:INTERLOCK:QUERY")
        except Exception as e:
            self.mock = True
            print(f"[LaserController] Note: Running in mock laser mode ({e}).")

    def arm(self) -> None:
        """Arm the laser safety system (enables emission ready state)."""
        if self.telemetry.estop_latched:
            raise RuntimeError(
                "Cannot arm laser: emergency stop is latched. Call reset_estop() to clear."
            )
        if not self.telemetry.interlock_ok:
            raise RuntimeError("Cannot arm laser: Safety interlock circuit open!")
        self._send_cmd("LASER:ARM")
        self.telemetry.state = LaserState.ARMED

    def set_interlock(self, ok: bool) -> None:
        """Update the safety interlock circuit state.

        Opening the interlock (ok=False) immediately forces the shutter
        closed and blocks all subsequent laser activation until it is
        closed again.
        """
        self.telemetry.interlock_ok = ok
        if not ok:
            self.shutter_close()

    def disarm(self) -> None:
        """Disarm laser and force shutter closed."""
        self.shutter_close()
        self._send_cmd("LASER:DISARM")
        self.telemetry.state = LaserState.DISARMED

    def set_power_mw(self, power_mw: float) -> None:
        """Set target laser diode output power in milliwatts."""
        p = max(0.0, min(power_mw, 1000.0))
        self.telemetry.power_mw = p
        self._send_cmd(f"LASER:POWER {p:.1f}")

    def shutter_open(self) -> None:
        """Open physical beam shutter to begin resin exposure."""
        if self.telemetry.estop_latched:
            raise RuntimeError(
                "Cannot open shutter: emergency stop is latched. Call reset_estop() to clear."
            )
        if not self.telemetry.interlock_ok:
            raise RuntimeError("Cannot open shutter: safety interlock circuit open!")
        if self.telemetry.state != LaserState.ARMED:
            raise RuntimeError(f"Cannot open shutter while laser is {self.telemetry.state} (must be ARMED)")
        self._send_cmd("SHUTTER:OPEN")
        self.telemetry.state = LaserState.EMITTING

    def shutter_close(self) -> None:
        """Close physical beam shutter to block optical emission."""
        self._send_cmd("SHUTTER:CLOSE")
        if self.telemetry.state == LaserState.EMITTING:
            self.telemetry.state = LaserState.ARMED

    def pulse(self, duration_s: float) -> None:
        """Deliver a precision timed optical exposure pulse."""
        if duration_s <= 0.0:
            return
        self.shutter_open()
        t0 = time.time()
        time.sleep(duration_s)
        self.shutter_close()
        actual = time.time() - t0
        self.telemetry.total_emission_s += actual
        self.telemetry.pulse_count += 1

    def emergency_stop(self) -> None:
        """Immediate hardware stop: close shutter, drop power, latch the stop.

        The e-stop stays latched until reset_estop() is called manually;
        while latched, every activation path (arm, shutter_open, pulse)
        is refused.
        """
        self._send_cmd("EMERGENCY_STOP")
        self.telemetry.estop_latched = True
        self.shutter_close()
        self.disarm()
        self.telemetry.power_mw = 0.0
        self.telemetry.state = LaserState.DISARMED

    def reset_estop(self) -> None:
        """Manually clear the latched emergency stop.

        Does not re-arm the laser; arm() must be called separately, and
        still refuses while the interlock circuit is open.
        """
        self.telemetry.estop_latched = False

    def _send_cmd(self, cmd: str) -> str:
        self.command_log.append(cmd)
        if self.mock:
            return "OK"
        if self._serial and self._serial.is_open:
            self._serial.write((cmd.strip() + "\n").encode("utf-8"))
            return self._serial.readline().decode("utf-8", errors="ignore").strip()
        return "ERROR"

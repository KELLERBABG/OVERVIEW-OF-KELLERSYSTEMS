"""Rotary Stage Motion Controller.

Controls the rotary vat stage using G-code over USB-serial or provides a high-fidelity
in-memory emulator for headless simulation, dry runs, and automated testing.
"""
from __future__ import annotations

import time
from dataclasses import dataclass


@dataclass
class StageStatus:
    is_connected: bool = False
    is_homed: bool = False
    is_enabled: bool = False
    current_angle_deg: float = 0.0
    target_angle_deg: float = 0.0
    command_count: int = 0


class RotaryStageController:
    """Motion controller for the precision cylindrical resin vat rotary axis."""

    def __init__(
        self,
        port: str | None = None,
        baud: int = 115200,
        mock: bool = False,
        steps_per_deg: float = 88.888,
        settle_time_s: float = 0.03,
    ):
        self.port = port
        self.baud = baud
        self.mock = mock or (port is None)
        self.steps_per_deg = steps_per_deg
        self.settle_time_s = settle_time_s

        self.status = StageStatus()
        self.command_log: list[str] = []
        self._serial = None

        if not self.mock and self.port:
            self.connect(self.port)
        else:
            self.mock = True
            self.status.is_connected = True

    def connect(self, port: str) -> None:
        """Establish serial connection to stepper controller."""
        self.port = port
        if self.mock:
            self.status.is_connected = True
            return

        try:
            import serial
            self._serial = serial.Serial(port, self.baud, timeout=2.0)
            time.sleep(1.0)  # Wait for bootloader reset
            self.status.is_connected = True
            self._send_gcode("M17")  # Enable steppers
            self.status.is_enabled = True
        except Exception as e:
            self.mock = True
            self.status.is_connected = True
            print(f"[RotaryStage] Warning: Failed to connect to '{port}' ({e}). Falling back to mock driver.")

    def disconnect(self) -> None:
        """Close connection and disable motor."""
        if self.status.is_connected and not self.mock and self._serial:
            try:
                self._send_gcode("M18")  # Disable steppers
                self._serial.close()
            except Exception:
                pass
        self.status.is_connected = False
        self.status.is_enabled = False

    def home(self) -> None:
        """Home the rotary stage to optical zero position (0.0 degrees)."""
        self._send_gcode("G28 A")
        self.status.is_homed = True
        self.status.current_angle_deg = 0.0
        self.status.target_angle_deg = 0.0
        time.sleep(self.settle_time_s)

    def rotate_to(self, angle_deg: float, wait: bool = True) -> None:
        """Rotate the vat stage to absolute angle in degrees [0, 360)."""
        target = float(angle_deg % 360.0)
        self.status.target_angle_deg = target
        self._send_gcode(f"G0 A{target:.3f}")
        self.status.current_angle_deg = target
        if wait:
            time.sleep(self.settle_time_s)

    def rotate_by(self, delta_deg: float, wait: bool = True) -> None:
        """Rotate relative to current position."""
        target = (self.status.current_angle_deg + delta_deg) % 360.0
        self.rotate_to(target, wait=wait)

    def _send_gcode(self, cmd: str) -> str:
        """Dispatch G-code command and return response."""
        self.status.command_count += 1
        self.command_log.append(cmd)

        if self.mock:
            return "ok"

        if self._serial and self._serial.is_open:
            self._serial.write((cmd.strip() + "\n").encode("utf-8"))
            resp = self._serial.readline().decode("utf-8", errors="ignore").strip()
            return resp
        return "error: not connected"

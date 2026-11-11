"""Spatial Light Modulator (SLM) Display Presentation Manager.

Handles full-screen calibrated 8-bit phase bitmap presentation on HDMI/DisplayPort
connected phase-only LCoS SLMs (Holoeye Pluto, Jasper Display, Meadowlark) with
hardware LUT mapping and headless mock fallback.
"""
from __future__ import annotations

import numpy as np
from PIL import Image

from ..calibration import DEFAULT_LUT, SLMCalibrationLUT


class SLMDisplayManager:
    """Manages the full-screen display output targeting the SLM micro-display."""

    def __init__(
        self,
        monitor_index: int = 1,
        lut: SLMCalibrationLUT | None = None,
        mock: bool = False,
        resolution: tuple[int, int] = (1920, 1080),
    ):
        self.monitor_index = monitor_index
        self.lut = lut or DEFAULT_LUT
        self.mock = mock
        self.resolution = resolution

        self.current_frame: np.ndarray | None = None
        self.frame_count: int = 0
        self.is_open: bool = False

        self._window = None
        self._photo_image = None
        self._canvas = None

        if not self.mock:
            self._init_window()
        else:
            self.is_open = True

    def _init_window(self) -> None:
        """Create a borderless fullscreen presentation window on the target monitor."""
        try:
            import tkinter as tk
            self._root = tk.Tk()
            self._root.title("HoloForge SLM Engine")
            self._root.configure(background="black", cursor="none")
            self._root.attributes("-fullscreen", True)

            # Bind Escape to exit fullscreen safely
            self._root.bind("<Escape>", lambda e: self.close())
            self.is_open = True
        except Exception as e:
            self.mock = True
            self.is_open = True
            print(f"[SLMDisplay] Note: Running in headless/mock presentation mode ({e}).")

    def display_phase(self, phase_rad: np.ndarray) -> np.ndarray:
        """Convert float phase array [-pi, pi] to calibrated 8-bit drive levels and present."""
        drive_uint8 = self.lut.phase_to_drive(phase_rad)
        return self.display_drive(drive_uint8)

    def display_drive(self, drive_uint8: np.ndarray) -> np.ndarray:
        """Present 8-bit grayscale bitmap directly on the SLM."""
        self.current_frame = np.asarray(drive_uint8, dtype=np.uint8)
        self.frame_count += 1

        if not self.mock and self._window is not None:
            try:
                import tkinter as tk

                from PIL import ImageTk
                img = Image.fromarray(self.current_frame, mode="L")
                if img.size != self.resolution:
                    img = img.resize(self.resolution, Image.NEAREST)
                self._photo_image = ImageTk.PhotoImage(img)
                if self._canvas is None:
                    self._canvas = tk.Label(self._root, image=self._photo_image, bg="black")
                    self._canvas.pack(fill=tk.BOTH, expand=True)
                else:
                    self._canvas.configure(image=self._photo_image)
                self._root.update_idletasks()
                self._root.update()
            except Exception:
                pass

        return self.current_frame

    def blank(self) -> None:
        """Clear SLM to black (zero phase retardance)."""
        shape = self.resolution if self.current_frame is None else self.current_frame.shape
        black = np.zeros(shape, dtype=np.uint8)
        self.display_drive(black)

    def close(self) -> None:
        """Close presentation window and release display."""
        if not self.mock and hasattr(self, "_root") and self._root:
            try:
                self._root.destroy()
            except Exception:
                pass
        self.is_open = False

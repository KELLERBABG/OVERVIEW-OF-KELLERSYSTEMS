"""Hardware-agnostic compute backend for CPU (NumPy/SciPy) and GPU (CuPy / PyTorch).

Provides a unified abstraction (`ComputeBackend`) for:
  - Device auto-detection (`auto`, `cpu`, `cuda`)
  - Array creation & host/device transfers (`asarray`, `zeros`, `ones`, `to_numpy`)
  - Batched 2D Fourier transforms (`fft2`, `ifft2`, `fftshift`, `ifftshift`)
  - 3D tomographic volume rotation around the vertical y-axis (`rotate_volume_y`)

Automatically falls back to CPU (`numpy`/`scipy`) when no GPU runtime or CUDA
device is detected on the system.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any

import numpy as np


class DeviceType(str, Enum):
    CPU = "cpu"
    CUDA = "cuda"


@dataclass(frozen=True)
class DeviceInfo:
    device_type: DeviceType
    backend_name: str  # "numpy", "cupy", or "torch"
    device_name: str
    is_gpu: bool


def detect_available_backends() -> dict[str, bool]:
    """Check whether cupy or torch with CUDA are available."""
    has_cupy = False
    has_torch_cuda = False

    try:
        import cupy as cp  # type: ignore
        if cp.cuda.runtime.getDeviceCount() > 0:
            has_cupy = True
    except Exception:
        pass

    if not has_cupy:
        try:
            import torch  # type: ignore
            if torch.cuda.is_available() and torch.cuda.device_count() > 0:
                has_torch_cuda = True
        except Exception:
            pass

    return {
        "numpy": True,
        "cupy": has_cupy,
        "torch_cuda": has_torch_cuda,
    }


class ComputeBackend:
    """Unified tensor/array backend supporting NumPy/SciPy, CuPy, and PyTorch CUDA."""

    def __init__(self, requested_device: str = "auto"):
        req = requested_device.lower().strip()
        if req not in ("auto", "cpu", "cuda"):
            raise ValueError(f"Unsupported device '{requested_device}'. Expected 'auto', 'cpu', or 'cuda'.")

        self.requested_device = req
        self._cp = None
        self._torch = None
        self.backend_name = "numpy"
        self.device_type = DeviceType.CPU
        self.device_name = "CPU (NumPy / SciPy)"

        if req in ("auto", "cuda"):
            # 1. Try CuPy first
            try:
                import cupy as cp  # type: ignore
                if cp.cuda.runtime.getDeviceCount() > 0:
                    self._cp = cp
                    self.backend_name = "cupy"
                    self.device_type = DeviceType.CUDA
                    props = cp.cuda.runtime.getDeviceProperties(0)
                    name = props.get("name", b"CUDA GPU")
                    self.device_name = name.decode("utf-8") if isinstance(name, bytes) else str(name)
            except Exception:
                self._cp = None

            # 2. Try PyTorch CUDA if CuPy wasn't available
            if self._cp is None:
                try:
                    import torch  # type: ignore
                    if torch.cuda.is_available() and torch.cuda.device_count() > 0:
                        self._torch = torch
                        self.backend_name = "torch"
                        self.device_type = DeviceType.CUDA
                        self.device_name = torch.cuda.get_device_name(0)
                except Exception:
                    self._torch = None

    @property
    def is_gpu(self) -> bool:
        return self.device_type == DeviceType.CUDA

    def info(self) -> DeviceInfo:
        return DeviceInfo(
            device_type=self.device_type,
            backend_name=self.backend_name,
            device_name=self.device_name,
            is_gpu=self.is_gpu,
        )

    # ------------------------------------------------------------------
    # Array allocation & conversion
    # ------------------------------------------------------------------
    def asarray(self, a: Any, dtype: Any = None) -> Any:
        if self.backend_name == "cupy":
            return self._cp.asarray(a, dtype=dtype)
        return np.asarray(a, dtype=dtype)

    def zeros(self, shape: tuple[int, ...], dtype: Any = np.float64) -> Any:
        if self.backend_name == "cupy":
            return self._cp.zeros(shape, dtype=dtype)
        return np.zeros(shape, dtype=dtype)

    def ones(self, shape: tuple[int, ...], dtype: Any = np.float64) -> Any:
        if self.backend_name == "cupy":
            return self._cp.ones(shape, dtype=dtype)
        return np.ones(shape, dtype=dtype)

    def to_numpy(self, a: Any) -> np.ndarray:
        if self.backend_name == "cupy" and hasattr(a, "get"):
            return a.get()
        if self._torch is not None and isinstance(a, self._torch.Tensor):
            return a.detach().cpu().numpy()
        return np.asarray(a)

    # ------------------------------------------------------------------
    # Batched 2D FFT / IFFT
    # ------------------------------------------------------------------
    def fft2(self, a: Any, axes: tuple[int, int] = (-2, -1)) -> Any:
        if self.backend_name == "cupy":
            return self._cp.fft.fft2(self._cp.asarray(a), axes=axes)
        if self.backend_name == "torch":
            t = self._torch.as_tensor(a, device="cuda")
            res = self._torch.fft.fft2(t, dim=axes)
            return res.detach().cpu().numpy()
        return np.fft.fft2(a, axes=axes)

    def ifft2(self, a: Any, axes: tuple[int, int] = (-2, -1)) -> Any:
        if self.backend_name == "cupy":
            return self._cp.fft.ifft2(self._cp.asarray(a), axes=axes)
        if self.backend_name == "torch":
            t = self._torch.as_tensor(a, device="cuda")
            res = self._torch.fft.ifft2(t, dim=axes)
            return res.detach().cpu().numpy()
        return np.fft.ifft2(a, axes=axes)

    def fftshift(self, a: Any, axes: tuple[int, int] = (-2, -1)) -> Any:
        if self.backend_name == "cupy":
            return self._cp.fft.fftshift(self._cp.asarray(a), axes=axes)
        return np.fft.fftshift(a, axes=axes)

    def ifftshift(self, a: Any, axes: tuple[int, int] = (-2, -1)) -> Any:
        if self.backend_name == "cupy":
            return self._cp.fft.ifftshift(self._cp.asarray(a), axes=axes)
        return np.fft.ifftshift(a, axes=axes)

    # ------------------------------------------------------------------
    # 3D Volume Rotation around Y-axis
    # ------------------------------------------------------------------
    def rotate_volume_y(self, volume: np.ndarray, angle_deg: float, order: int = 1) -> np.ndarray:
        """Rotate a 3D volume (nz, ny, nx) around the vertical y-axis by angle_deg.

        Dispatches to cupyx.scipy.ndimage.rotate when CuPy is active, or falls back
        to scipy.ndimage.rotate / NumPy bilinear interpolation.
        """
        if abs(angle_deg % 360.0) < 1e-4:
            return np.asarray(volume).copy()

        nz, ny, nx = volume.shape
        need_pad = (nz != nx)
        if need_pad:
            max_dim = max(nz, nx)
            pad_z = ((max_dim - nz) // 2, max_dim - nz - (max_dim - nz) // 2)
            pad_x = ((max_dim - nx) // 2, max_dim - nx - (max_dim - nx) // 2)
            vol_pad = np.pad(np.asarray(volume), (pad_z, (0, 0), pad_x), mode="constant", constant_values=0.0)
        else:
            vol_pad = np.asarray(volume)

        rot = None
        if self.backend_name == "cupy":
            try:
                import cupyx.scipy.ndimage as cp_ndimage  # type: ignore
                gpu_vol = self._cp.asarray(vol_pad)
                gpu_rot = cp_ndimage.rotate(
                    gpu_vol, angle_deg, axes=(0, 2), reshape=False, order=order, mode="constant", cval=0.0
                )
                rot = gpu_rot.get()
            except Exception:
                rot = None

        if rot is None:
            try:
                from scipy.ndimage import rotate as _scipy_rotate
                rot = _scipy_rotate(
                    vol_pad, angle_deg, axes=(0, 2), reshape=False, order=order, mode="constant", cval=0.0
                )
            except ImportError:
                angle_rad = np.radians(angle_deg)
                cos_a, sin_a = np.cos(angle_rad), np.sin(angle_rad)
                dim_z, _, dim_x = vol_pad.shape
                cz, cx = (dim_z - 1) / 2.0, (dim_x - 1) / 2.0
                z_idx, x_idx = np.indices((dim_z, dim_x))
                z_orig = (cos_a * (z_idx - cz) + sin_a * (x_idx - cx) + cz).round().astype(int)
                x_orig = (-sin_a * (z_idx - cz) + cos_a * (x_idx - cx) + cx).round().astype(int)
                valid = (z_orig >= 0) & (z_orig < dim_z) & (x_orig >= 0) & (x_orig < dim_x)
                rot = np.zeros_like(vol_pad)
                for y in range(ny):
                    rot[valid, y] = vol_pad[z_orig[valid], y, x_orig[valid]]

        if need_pad:
            z_start = (max_dim - nz) // 2
            x_start = (max_dim - nx) // 2
            return rot[z_start:z_start + nz, :, x_start:x_start + nx].copy()
        return rot


def get_backend(device: str = "auto") -> ComputeBackend:
    """Factory helper returning a configured ComputeBackend instance."""
    return ComputeBackend(requested_device=device)

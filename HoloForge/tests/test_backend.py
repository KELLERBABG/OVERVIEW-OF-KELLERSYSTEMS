"""Unit tests for Tier 3: GPU/CPU ComputeBackend abstraction."""
from __future__ import annotations

import numpy as np
import pytest

from holoforge.backend import ComputeBackend, DeviceType, detect_available_backends, get_backend
from holoforge.config import RESINS, GridConfig, OpticsConfig
from holoforge.optimize import TomographicHolographicOptimizer
from holoforge.volume import sphere


def test_backend_auto_and_cpu_detection():
    avail = detect_available_backends()
    assert avail["numpy"] is True

    b_cpu = get_backend("cpu")
    assert b_cpu.device_type == DeviceType.CPU
    assert b_cpu.backend_name == "numpy"
    assert b_cpu.is_gpu is False

    b_auto = get_backend("auto")
    info = b_auto.info()
    assert info.backend_name in ("numpy", "cupy", "torch")

    with pytest.raises(ValueError):
        get_backend("invalid_device")


def test_backend_fft_and_array_operations():
    b = ComputeBackend("cpu")
    arr = np.random.default_rng(42).normal(size=(4, 16, 16)) + 1j * np.zeros((4, 16, 16))
    f = b.fft2(arr, axes=(1, 2))
    inv = b.to_numpy(b.ifft2(f, axes=(1, 2)))
    np.testing.assert_allclose(inv, arr, atol=1e-10)

    shifted = b.fftshift(f, axes=(1, 2))
    unshifted = b.to_numpy(b.ifftshift(shifted, axes=(1, 2)))
    np.testing.assert_allclose(unshifted, b.to_numpy(f), atol=1e-10)


def test_backend_rotate_volume_y():
    b = ComputeBackend("cpu")
    vol = sphere(shape=(24, 24, 24), radius=0.3).mask.astype(np.float64)
    rot_0 = b.rotate_volume_y(vol, 0.0)
    np.testing.assert_allclose(rot_0, vol)

    rot_90 = b.rotate_volume_y(vol, 90.0)
    rot_back = b.rotate_volume_y(rot_90, -90.0)
    # Center mass should remain intact
    assert rot_back[12, 12, 12] > 0.8


def test_tomographic_optimizer_with_explicit_device():
    optics = OpticsConfig(slm_pixels=64)
    grid = GridConfig(n_xy=32, n_z=32, slice_step_um=2.0)
    resin = RESINS["hf-standard-405"]
    target = sphere(shape=(32, 32, 32), radius=0.28).mask

    cgh = TomographicHolographicOptimizer(
        optics, grid, resin, target, n_patterns=12, device="cpu"
    )
    res = cgh.optimize(iterations=2, verbose=False)
    assert res.final_metrics["cure_iou"] > 0.85

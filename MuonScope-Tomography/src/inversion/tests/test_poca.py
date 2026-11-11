import numpy as np
import pytest

from muonscope.reconstructors.poca import compute_poca_point, PoCAReconstructor

def test_poca_exact_intersection():
    p_in = np.array([0.0, 0.0, 300.0])
    d_in = np.array([0.0, 0.0, -1.0])

    d_out_raw = np.array([100.0, 0.0, -300.0])
    d_out = d_out_raw / np.linalg.norm(d_out_raw)
    p_out = np.array([0.0, 0.0, 0.0]) + (-300.0 / d_out[2]) * d_out

    poca_pt, miss_dist, theta = compute_poca_point(p_in, d_in, p_out, d_out)

    np.testing.assert_allclose(poca_pt, [0.0, 0.0, 0.0], atol=1e-3)
    assert miss_dist < 1e-3
    assert theta > 0.0

def test_poca_reconstructor_accumulation():
    recon = PoCAReconstructor(
        grid_size=(32, 32, 32),
        bounds_min_mm=(-100.0, -100.0, -100.0),
        bounds_max_mm=(100.0, 100.0, 100.0),
    )
    
    p_in = np.array([0.0, 0.0, 100.0])
    d_in = np.array([0.0, 0.0, -1.0])
    p_out = np.array([10.0, 0.0, -100.0])
    d_out = np.array([0.1, 0.0, -0.99])
    d_out /= np.linalg.norm(d_out)

    success = recon.process_event(p_in, d_in, p_out, d_out)
    assert success is True
    
    grid = recon.get_reconstructed_density()
    assert np.max(grid) > 0.0

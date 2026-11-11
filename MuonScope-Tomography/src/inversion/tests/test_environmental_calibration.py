import pytest
import numpy as np
from muonscope.physics.environmental_calibration import (
    EnvironmentalFluxCalibrator,
    KalmanFluxBaselineTracker,
)

def test_environmental_flux_calibrator_pressure():
    calib = EnvironmentalFluxCalibrator(ref_pressure_hpa=1013.25, beta_p=-0.0015)
    
    # At standard pressure and temp, correction factor is 1.0
    factor_ref = calib.compute_flux_correction_factor(pressure_hpa=1013.25, temp_c=15.0)
    assert pytest.approx(factor_ref, rel=1e-3) == 1.0

    # High pressure (e.g. 1033.25 hPa, +20 hPa) reduces natural flux by ~3% (0.0015 * 20 = 0.03)
    # Correction factor should be > 1.0 (to scale up raw counts to standard baseline)
    factor_high_p = calib.compute_flux_correction_factor(pressure_hpa=1033.25, temp_c=15.0)
    assert factor_high_p > 1.0
    assert pytest.approx(factor_high_p, rel=1e-2) == 1.031

    # Low pressure (e.g. 993.25 hPa, -20 hPa) increases natural flux by ~3%
    # Correction factor should be < 1.0
    factor_low_p = calib.compute_flux_correction_factor(pressure_hpa=993.25, temp_c=15.0)
    assert factor_low_p < 1.0
    assert pytest.approx(factor_low_p, rel=1e-2) == 0.970

def test_environmental_flux_diurnal_solar_modulation():
    calib = EnvironmentalFluxCalibrator(solar_amplitude=0.02, solar_peak_hour_utc=14.0)
    
    # 14:00 UTC (peak)
    t_peak = 14.0 * 3600.0
    factor_peak = calib.compute_flux_correction_factor(1013.25, 15.0, timestamp_unix=t_peak)
    
    # 02:00 UTC (trough, 12h later)
    t_trough = 2.0 * 3600.0
    factor_trough = calib.compute_flux_correction_factor(1013.25, 15.0, timestamp_unix=t_trough)
    
    assert factor_peak < factor_trough

def test_kalman_flux_baseline_tracker():
    tracker = KalmanFluxBaselineTracker(initial_flux_rate=10000.0)
    
    rng = np.random.default_rng(42)
    true_flux = 10000.0
    
    for t in range(50):
        # Simulated noisy measurement with Poisson-like variance
        noisy_meas = true_flux + rng.normal(0.0, 150.0)
        filtered = tracker.update(noisy_meas, timestamp_unix=float(t * 60))
        assert 9500.0 <= filtered <= 10500.0
        
    stats = tracker.get_summary_stats()
    assert "baseline" in stats
    assert "stability_pct" in stats
    assert stats["stability_pct"] > 90.0

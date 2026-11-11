from typing import Optional, Tuple, Dict, List
import numpy as np

class EnvironmentalFluxCalibrator:
    """
    Atmospheric, Thermal, and Solar Diurnal Cosmic Muon Flux Calibration Engine.
    
    Physics Models:
      1. Barometric Inversion: dPhi / dP ≈ -0.15% / hPa
         Higher barometric pressure increases upper atmospheric column depth,
         raising the minimum cutoff momentum for muons to reach sea level.
      2. Thermal Expansion: dPhi / dT ≈ +0.05% / °C
         Tropospheric temperature increases expand the pion production layer,
         modulating pion decay vs interaction probability.
      3. Diurnal Solar Modulation: ~1.5% sinusoidal 24h variation due to solar wind.
      4. Recursive 1D Kalman Baseline Filter for drift compensation across multi-day scans.
    """
    def __init__(
        self,
        ref_pressure_hpa: float = 1013.25,
        beta_p: float = -0.0015,       # -0.15% per hPa
        ref_temp_c: float = 15.0,
        alpha_t: float = 0.0005,       # +0.05% per °C
        solar_amplitude: float = 0.015, # 1.5% diurnal modulation
        solar_peak_hour_utc: float = 14.0, # Peak cosmic flux around 14:00 UTC
    ):
        self.ref_p = ref_pressure_hpa
        self.beta_p = beta_p
        self.ref_t = ref_temp_c
        self.alpha_t = alpha_t
        self.solar_amp = solar_amplitude
        self.solar_peak_hour = solar_peak_hour_utc

    def compute_flux_correction_factor(
        self,
        pressure_hpa: float,
        temp_c: float,
        timestamp_unix: Optional[float] = None
    ) -> float:
        """
        Calculates the multiplicative correction factor to apply to measured muon count rates
        to normalize them to standard reference sea-level conditions.
        """
        dp = pressure_hpa - self.ref_p
        dt = temp_c - self.ref_t
        
        solar_mod = 0.0
        if timestamp_unix is not None:
            # 24-hour periodic wave
            seconds_in_day = timestamp_unix % 86400.0
            hours_utc = seconds_in_day / 3600.0
            phase = 2.0 * np.pi * (hours_utc - self.solar_peak_hour) / 24.0
            solar_mod = self.solar_amp * np.cos(phase)

        relative_flux = 1.0 + (self.beta_p * dp) + (self.alpha_t * dt) + solar_mod
        
        # Clamp relative flux to physically realistic bounds [0.5, 1.5]
        relative_flux = max(0.5, min(1.5, relative_flux))
        return float(1.0 / relative_flux)

    def normalize_rate(
        self,
        raw_counts: int,
        duration_seconds: float,
        pressure_hpa: float,
        temp_c: float,
        area_m2: float = 0.25,
        timestamp_unix: Optional[float] = None
    ) -> float:
        """
        Returns normalized flux in units of [muons / m^2 / minute].
        """
        if duration_seconds <= 0.0:
            return 0.0
        raw_rate_m2_min = (raw_counts / (duration_seconds / 60.0)) / area_m2
        correction = self.compute_flux_correction_factor(pressure_hpa, temp_c, timestamp_unix)
        return float(raw_rate_m2_min * correction)


class KalmanFluxBaselineTracker:
    """
    Real-time 1D Kalman Filter to track true background cosmic-ray flux baseline
    in the presence of Poisson shot noise and slow environmental drift.
    """
    def __init__(
        self,
        initial_flux_rate: float = 10000.0, # ~10,000 muons/m^2/min at sea level
        process_variance_q: float = 25.0,   # Slow physical drift
        measurement_variance_r: float = 400.0 # Poisson measurement noise
    ):
        self.x = initial_flux_rate  # State estimate
        self.p = 100.0              # State estimation error covariance
        self.q = process_variance_q # Process noise variance
        self.r = measurement_variance_r # Measurement noise variance
        self.history: List[Dict[str, float]] = []

    def update(self, measured_normalized_rate: float, timestamp_unix: float) -> float:
        # 1. Predict
        x_pred = self.x
        p_pred = self.p + self.q

        # 2. Update (Kalman Gain)
        k = p_pred / (p_pred + self.r)
        self.x = x_pred + k * (measured_normalized_rate - x_pred)
        self.p = (1.0 - k) * p_pred

        self.history.append({
            "timestamp": timestamp_unix,
            "measured": measured_normalized_rate,
            "filtered_baseline": self.x,
            "uncertainty_sigma": float(np.sqrt(self.p))
        })

        if len(self.history) > 1000:
            self.history.pop(0)

        return float(self.x)

    def get_summary_stats(self) -> Dict[str, float]:
        if not self.history:
            return {"baseline": self.x, "sigma": float(np.sqrt(self.p)), "drift_pct": 0.0}
        recent = [h["measured"] for h in self.history[-50:]]
        return {
            "baseline": round(float(self.x), 2),
            "uncertainty_sigma": round(float(np.sqrt(self.p)), 2),
            "snr_db": round(float(10.0 * np.log10(self.x / (np.std(recent) + 1e-6))), 2),
            "stability_pct": round(float(100.0 * (1.0 - np.std(recent) / (np.mean(recent) + 1e-6))), 2)
        }

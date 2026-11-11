/// Appleton-Hartree critical frequency model for NVIS skywave propagation.
pub struct CriticalFrequencyModel {
    /// Peak electron density in the F2 layer (m⁻³).
    pub electron_density: f64,
}

impl CriticalFrequencyModel {
    pub fn new(electron_density: f64) -> Self {
        Self { electron_density }
    }

    /// Plasma (critical) frequency: f_p = 8.98 · √N_e  (Hz).
    pub fn plasma_frequency(&self) -> f64 {
        8.98 * self.electron_density.sqrt()
    }

    /// foF2 equals the plasma frequency at peak F2 electron density.
    pub fn fo_f2(&self) -> f64 {
        self.plasma_frequency()
    }

    /// Maximum usable frequency for incidence angle θ_i (degrees, 0–25°).
    /// f_MUF = foF2 · sec(θ_i)
    pub fn muf(&self, incidence_angle_deg: f64) -> f64 {
        self.fo_f2() / incidence_angle_deg.to_radians().cos()
    }

    /// True if the wave is reflected (frequency ≤ MUF for the given angle).
    pub fn reflects(&self, freq_hz: f64, incidence_angle_deg: f64) -> bool {
        freq_hz <= self.muf(incidence_angle_deg)
    }

    /// True if frequency falls in the optimal NVIS band [0.65·foF2, 0.95·foF2].
    pub fn is_optimal_nvis(&self, freq_hz: f64) -> bool {
        let lo = 0.65 * self.fo_f2();
        let hi = 0.95 * self.fo_f2();
        (lo..=hi).contains(&freq_hz)
    }
}

/// Convenience free function (backward-compatible with prior API).
pub fn estimate_fo_f2(electron_density: f64) -> f64 {
    CriticalFrequencyModel::new(electron_density).fo_f2()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plasma_frequency_known_value() {
        // N_e = 1e11 m^-3 => f_p ≈ 2.84 MHz
        let m = CriticalFrequencyModel::new(1.0e11);
        assert!((m.plasma_frequency() - 2_839_725.0).abs() < 1.0e4);
    }

    #[test]
    fn muf_increases_with_incidence_angle() {
        let m = CriticalFrequencyModel::new(1.0e11);
        let muf_0 = m.muf(0.0);
        let muf_10 = m.muf(10.0);
        let muf_25 = m.muf(25.0);
        assert!(muf_0 < muf_10);
        assert!(muf_10 < muf_25);
    }

    #[test]
    fn above_muf_penetrates() {
        let m = CriticalFrequencyModel::new(1.0e11); // foF2 ≈ 2.84 MHz
        // 3.0 MHz > MUF at 0° (≈2.84 MHz) → penetrates
        assert!(!m.reflects(3.0e6, 0.0));
        // 2.0 MHz < MUF at 0° → reflects
        assert!(m.reflects(2.0e6, 0.0));
    }

    #[test]
    fn optimal_nvis_band() {
        let m = CriticalFrequencyModel::new(1.0e11); // foF2 ≈ 2.84 MHz
        // 0.65*2.84 ≈ 1.85 MHz, 0.95*2.84 ≈ 2.70 MHz
        assert!(m.is_optimal_nvis(2.0e6));
        assert!(!m.is_optimal_nvis(3.0e6)); // above band
        assert!(!m.is_optimal_nvis(1.0e6)); // below band
    }

    #[test]
    fn backward_compat_free_fn() {
        assert!((estimate_fo_f2(1.0e11) - 2_839_725.0).abs() < 1.0e4);
    }
}

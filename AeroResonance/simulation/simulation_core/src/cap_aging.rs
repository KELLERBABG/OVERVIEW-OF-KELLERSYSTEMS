//! Supercapacitor aging model: C(T, cycles), I_leak(T).
//! no_std compatible with libm.

use libm::pow;

#[derive(Debug, Clone, Copy)]
pub struct CapState {
    pub c_nom_f: f64,
    pub cycles: u64,
    pub i_leak_nom_a: f64,
    pub t_ref_c: f64,
    pub degradation_per_cycle: f64,
}

impl CapState {
    pub fn new(c_nom_f: f64, i_leak_nom_a: f64) -> Self {
        Self {
            c_nom_f,
            cycles: 0,
            i_leak_nom_a,
            t_ref_c: 25.0,
            degradation_per_cycle: 0.0002,
        }
    }

    pub fn effective_c(&self) -> f64 {
        let loss = (self.degradation_per_cycle * self.cycles as f64).min(0.8);
        (self.c_nom_f * (1.0 - loss)).max(self.c_nom_f * 0.2)
    }

    /// Leakage at temperature: doubles every 10°C above reference.
    pub fn leakage_at_temp(&self, temp_c: f64) -> f64 {
        let dt = temp_c - self.t_ref_c;
        self.i_leak_nom_a * pow(2.0, dt / 10.0)
    }

    pub fn record_cycle(&mut self) {
        self.cycles += 1;
    }

    pub fn usable_energy(&self, v_high: f64, v_low: f64, temp_c: f64) -> f64 {
        let c = self.effective_c();
        let leak_a = self.leakage_at_temp(temp_c);
        let e = 0.5 * c * (v_high * v_high - v_low * v_low).max(0.0);
        (e - leak_a * v_high * 1.0).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh() {
        let cap = CapState::new(0.47, 2e-6);
        assert!((cap.effective_c() - 0.47).abs() < 1e-6);
    }

    #[test]
    fn test_aged() {
        let mut cap = CapState::new(0.47, 2e-6);
        for _ in 0..1000 {
            cap.record_cycle();
        }
        assert!((cap.effective_c() - 0.376).abs() < 1e-2);
    }

    #[test]
    fn test_leakage_doubles() {
        let cap = CapState::new(0.47, 2e-6);
        let r = cap.leakage_at_temp(35.0) / cap.leakage_at_temp(25.0);
        assert!((r - 2.0).abs() < 1e-6);
    }
}

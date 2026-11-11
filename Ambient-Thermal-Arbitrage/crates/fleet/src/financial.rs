// ============================================================================
// Financial tracking — cost, savings, baseline gaming prevention
// ============================================================================

/// Per-store financial results across a full year simulation.
#[derive(Debug, Clone)]
pub struct StoreFinancials {
    pub type_name: String,
    pub count: u64,

    // Energy
    pub actual_energy_mwh: f64,
    pub baseline_energy_mwh: f64,
    pub defrost_energy_mwh: f64,

    // Costs
    pub actual_cost_eur: f64,
    pub baseline_cost_eur: f64,
    pub fcr_revenue_eur: f64,
    pub droop_revenue_eur: f64,

    // Savings
    pub savings_eur: f64,
    pub savings_pct: f64,

    // Operational
    pub runtime_hours: f64,
    pub defrost_count: u64,
}

impl StoreFinancials {
    pub fn new(name: &str, count: u64) -> Self {
        Self {
            type_name: name.to_string(),
            count,
            actual_energy_mwh: 0.0,
            baseline_energy_mwh: 0.0,
            defrost_energy_mwh: 0.0,
            actual_cost_eur: 0.0,
            baseline_cost_eur: 0.0,
            fcr_revenue_eur: 0.0,
            droop_revenue_eur: 0.0,
            savings_eur: 0.0,
            savings_pct: 0.0,
            runtime_hours: 0.0,
            defrost_count: 0,
        }
    }

    /// Record one hour.
    /// Both baseline and actual are priced at hourly spot price.
    /// Baseline: compressor runs `duty` fraction of the hour at full power × price.
    /// Actual: VFD-modulated real energy × price.
    /// Savings come from running LESS during expensive hours and MORE during cheap hours.
    #[allow(clippy::too_many_arguments)]
    pub fn record_hour(
        &mut self,
        baseline_duty_cycle: f64,
        compressor_power_kw: f64,
        price: f64,
        actual_energy_mwh: f64,
        defrost_mwh: f64,
        fcr_rev: f64,
        droop_rev: f64,
    ) {
        let duty = baseline_duty_cycle.clamp(0.0, 0.9);
        let baseline_energy = compressor_power_kw * duty / 1000.0;
        let baseline_cost = baseline_energy * price; // spot price!
        let actual_cost = actual_energy_mwh * price;

        self.actual_energy_mwh += actual_energy_mwh;
        self.baseline_energy_mwh += baseline_energy;
        self.actual_cost_eur += actual_cost;
        self.baseline_cost_eur += baseline_cost;
        self.defrost_energy_mwh += defrost_mwh;
        self.fcr_revenue_eur += fcr_rev;
        self.droop_revenue_eur += droop_rev;
        if actual_energy_mwh > 0.0 { self.runtime_hours += 1.0; }
        if defrost_mwh > 0.0 { self.defrost_count += 1; }
    }

    pub fn finalize(&mut self) {
        self.savings_eur = self.baseline_cost_eur - self.actual_cost_eur;
        self.savings_pct = if self.baseline_cost_eur > 0.0 {
            self.savings_eur / self.baseline_cost_eur * 100.0
        } else { 0.0 };
    }
}

#[allow(dead_code)]
pub fn compute_baseline_flat_tariff(prices: &[f64]) -> f64 {
    if prices.is_empty() { return 63.0; }
    let sum: f64 = prices.iter().sum();
    sum / prices.len() as f64
}

/// Compute baseline duty cycle — what a simple ON/OFF thermostat consumes.
/// Combines wall leakage + internal heat + defrost to get total heat load.
/// Uses the exact same dynamic COP at normal setpoint as the physical simulation,
/// ensuring that savings reflect genuine price arbitrage and load shifting rather
/// than an artificial baseline COP mismatch.
pub fn baseline_duty_cycle(t: &crate::store_types::StoreType, amb_c: f64) -> f64 {
    let cop_baseline = crate::physics::compute_cop(t, t.t_setpoint, amb_c);
    let max_cooling_kw = t.cooling_power_kw * cop_baseline;
    if max_cooling_kw <= 0.0 { return 0.0; }

    // Wall leakage: heat flow through insulation + infiltration
    let r_th = 1.0 / (t.u_value * t.surface_m2 + t.volume_m3 * t.air_change_rate_ach / 3600.0 * 1.2 * 1005.0).max(0.001);
    let delta_t = (amb_c - t.t_setpoint).max(0.0);
    let wall_leak_kw = delta_t / r_th / 1000.0;

    // Total steady-state thermal load to reject: wall leakage + internal generation
    let total_heat_kw = wall_leak_kw + t.internal_heat_kw;

    // Fair baseline duty cycle under identical thermal and thermodynamic conditions:
    (total_heat_kw / max_cooling_kw).clamp(0.0, 0.9)
}

pub fn fmt_euro(v: f64) -> String {
    let abs = v.abs();
    let sign = if v < 0.0 { "-" } else { "" };
    if abs >= 1_000_000.0 { format!("{}{:.1}M", sign, abs / 1_000_000.0) }
    else if abs >= 1_000.0 { format!("{}{:.0}K", sign, abs / 1_000.0) }
    else { format!("{}{:.0}", sign, abs) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store_types::STORE_TYPES;

    #[test]
    fn record_hour_accumulates_and_finalize_computes_savings() {
        let mut f = StoreFinancials::new("aldi", 1);
        f.record_hour(0.5, 100.0, 80.0, 0.04, 0.01, 1.0, 0.5);
        f.finalize();
        // baseline: 100 kW × 0.5 duty / 1000 = 0.05 MWh @ 80 €/MWh = 4.00 €
        // actual:   0.04 MWh @ 80 €/MWh = 3.20 € → savings 0.80 € = 20%
        assert!((f.baseline_energy_mwh - 0.05).abs() < 1e-9);
        assert!((f.baseline_cost_eur - 4.0).abs() < 1e-9);
        assert!((f.actual_cost_eur - 3.2).abs() < 1e-9);
        assert!((f.savings_eur - 0.8).abs() < 1e-9);
        assert!((f.savings_pct - 20.0).abs() < 1e-9);
        assert!((f.fcr_revenue_eur - 1.0).abs() < 1e-9);
        assert!((f.droop_revenue_eur - 0.5).abs() < 1e-9);
        assert_eq!(f.runtime_hours, 1.0);
        assert_eq!(f.defrost_count, 1);
    }

    #[test]
    fn duty_cycle_is_clamped_to_thermostat_reality() {
        let mut f = StoreFinancials::new("x", 1);
        // A duty cycle above 0.9 is physically impossible for a thermostat.
        f.record_hour(5.0, 100.0, 10.0, 0.0, 0.0, 0.0, 0.0);
        assert!((f.baseline_energy_mwh - 100.0 * 0.9 / 1000.0).abs() < 1e-9);
    }

    #[test]
    fn baseline_duty_cycle_bounds_and_zero_power() {
        let aldi = &STORE_TYPES[0];
        for amb in [5.0, 20.0, 35.0] {
            let d = baseline_duty_cycle(aldi, amb);
            assert!((0.0..=0.9).contains(&d), "duty {d} out of bounds at amb {amb}");
        }
        let mut zero = aldi.clone();
        zero.cooling_power_kw = 0.0;
        assert_eq!(baseline_duty_cycle(&zero, 25.0), 0.0);
    }

    #[test]
    fn fmt_euro_formats() {
        assert_eq!(fmt_euro(1_200_000.0), "1.2M");
        assert_eq!(fmt_euro(5_000.0), "5K");
        assert_eq!(fmt_euro(-5_000.0), "-5K");
        assert_eq!(fmt_euro(999.0), "999");
        assert_eq!(fmt_euro(0.0), "0");
    }
}
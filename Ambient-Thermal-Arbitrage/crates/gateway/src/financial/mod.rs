use serde::{Deserialize, Serialize};

/// Running financial state tracking baseline vs. arbitrage costs.
///
/// KEY FIXES vs previous:
/// - Baseline now uses REALISTIC compressor runtime (~40% duty cycle at setpoint)
/// - Costs are tracked consistently: electrical power draw × price × time
/// - All units: kW for power, MWh for energy, €/MWh for price, € for cost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialState {
    /// Baseline: energy if running normal thermostat (MWh)
    pub baseline_energy_mwh: f64,
    /// Baseline: cost if running normal thermostat (€)
    pub baseline_cost_eur: f64,
    /// Actual: energy consumed with arbitrage (MWh)
    pub actual_energy_mwh: f64,
    /// Actual: cost with arbitrage (€)
    pub actual_cost_eur: f64,
    /// Energy saved vs baseline (MWh)
    pub energy_saved_mwh: f64,
    /// Money saved vs baseline (€) — positive = savings
    pub money_saved_eur: f64,
    /// Average price paid (€/MWh)
    pub avg_price_paid: f64,
    /// Hours tracked
    pub hours_tracked: u64,
    /// Compressor hours of runtime
    pub compressor_hours: f64,
}

impl Default for FinancialState {
    fn default() -> Self {
        Self::new()
    }
}

impl FinancialState {
    pub fn new() -> Self {
        Self {
            baseline_energy_mwh: 0.0,
            baseline_cost_eur: 0.0,
            actual_energy_mwh: 0.0,
            actual_cost_eur: 0.0,
            energy_saved_mwh: 0.0,
            money_saved_eur: 0.0,
            avg_price_paid: 0.0,
            hours_tracked: 0,
            compressor_hours: 0.0,
        }
    }

    /// Record one simulated hour.
    ///
    /// Parameters:
    /// - `dt_hours`: simulation time step (usually 1.0 hour)
    /// - `compressor_on`: is compressor running?
    /// - `compressor_power_kw`: electrical power draw when running (e.g. 50 kW)
    /// - `price_eur_per_mwh`: electricity price this hour
    /// - `baseline_duty_cycle`: fraction of time baseline runs (e.g. 0.4 = 40%)
    ///
    /// The baseline assumes normal thermostat cycles the compressor ~40% of the time
    /// to maintain setpoint. The actual tracks the real compressor state from arbitrage.
    pub fn record_hour(
        &mut self,
        dt_hours: f64,
        compressor_on: bool,
        compressor_power_kw: f64,
        price_eur_per_mwh: f64,
        baseline_duty_cycle: f64,
    ) {
        // Fail-safe: non-finite inputs would permanently NaN-poison every
        // accumulator (a NaN baseline cost cascades into savings %, averages...).
        if !dt_hours.is_finite() || !compressor_power_kw.is_finite()
            || !price_eur_per_mwh.is_finite() || !baseline_duty_cycle.is_finite()
        {
            return;
        }
        let duty = baseline_duty_cycle.clamp(0.0, 0.9);

        // Baseline: compressor runs `duty` fraction of time
        let baseline_avg_power_kw = compressor_power_kw * duty;
        let baseline_energy = baseline_avg_power_kw * dt_hours / 1000.0; // MWh
        let baseline_cost = baseline_energy * price_eur_per_mwh;

        // Actual: compressor state from arbitrage decision
        let actual_power_kw = if compressor_on { compressor_power_kw } else { 0.0 };
        let actual_energy = actual_power_kw * dt_hours / 1000.0; // MWh
        let actual_cost = actual_energy * price_eur_per_mwh;

        // Accumulate
        self.baseline_energy_mwh += baseline_energy;
        self.baseline_cost_eur += baseline_cost;
        self.actual_energy_mwh += actual_energy;
        self.actual_cost_eur += actual_cost;

        // Savings: positive = saving money vs baseline
        self.energy_saved_mwh += baseline_energy - actual_energy;
        self.money_saved_eur += baseline_cost - actual_cost;

        self.hours_tracked += 1;
        if compressor_on {
            self.compressor_hours += dt_hours;
        }

        // Running average price paid per MWh (only hours with consumption)
        if actual_energy > 0.0 && self.actual_energy_mwh > 0.0 {
            self.avg_price_paid = self.actual_cost_eur / self.actual_energy_mwh;
        }
    }

    pub fn savings_percentage(&self) -> f64 {
        if self.baseline_cost_eur > 0.0 {
            // Can be negative = losing money, positive = saving
            self.money_saved_eur / self.baseline_cost_eur * 100.0
        } else {
            0.0
        }
    }

    pub fn projected_yearly_savings(&self) -> f64 {
        if self.hours_tracked > 0 {
            self.money_saved_eur / (self.hours_tracked as f64) * 8760.0
        } else {
            0.0
        }
    }
}

/// Building dimensions and thermal properties.
/// Used to compute R_th, C_th, cooling power, and baseline behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildingConfig {
    pub name: String,
    pub volume_m3: f64,
    pub surface_area_m2: f64,
    pub u_value_w_per_m2k: f64,
    pub product_mass_kg: f64,
    pub product_specific_heat_j_per_kgk: f64,
    pub cop: f64,
    pub installed_cooling_power_w: f64,
    pub air_ach_per_hour: f64,
}

impl Default for BuildingConfig {
    fn default() -> Self {
        Self::medium_storage()
    }
}

impl BuildingConfig {
    pub fn medium_storage() -> Self {
        Self {
            name: "Medium Cold Storage NRW".into(),
            volume_m3: 1000.0,
            surface_area_m2: 600.0,
            u_value_w_per_m2k: 0.25,
            product_mass_kg: 200_000.0,
            product_specific_heat_j_per_kgk: 2000.0,
            cop: 2.5,
            installed_cooling_power_w: 50_000.0,
            air_ach_per_hour: 0.5,
        }
    }

    pub fn small_walk_in() -> Self {
        Self {
            name: "Small Walk-In Cooler".into(),
            volume_m3: 50.0,
            surface_area_m2: 80.0,
            u_value_w_per_m2k: 0.4,
            product_mass_kg: 5_000.0,
            product_specific_heat_j_per_kgk: 3500.0,
            cop: 3.0,
            installed_cooling_power_w: 5_000.0,
            air_ach_per_hour: 1.0,
        }
    }

    pub fn large_warehouse() -> Self {
        Self {
            name: "Large Warehouse Cold Storage".into(),
            volume_m3: 10_000.0,
            surface_area_m2: 2_800.0,
            u_value_w_per_m2k: 0.2,
            product_mass_kg: 2_000_000.0,
            product_specific_heat_j_per_kgk: 2000.0,
            cop: 2.0,
            installed_cooling_power_w: 200_000.0,
            air_ach_per_hour: 0.2,
        }
    }

    pub fn compute_r_th(&self) -> f64 {
        let insulation_conductance = self.u_value_w_per_m2k * self.surface_area_m2;
        let air_volume_per_sec = self.volume_m3 * self.air_ach_per_hour / 3600.0;
        let air_density = 1.2;
        let air_specific_heat = 1005.0;
        let air_conductance = air_volume_per_sec * air_density * air_specific_heat;
        1.0 / (insulation_conductance + air_conductance).max(0.001)
    }

    pub fn compute_c_th(&self) -> f64 {
        let product_cap = self.product_mass_kg * self.product_specific_heat_j_per_kgk;
        let air_mass = self.volume_m3 * 1.2;
        let air_cap = air_mass * 1005.0;
        let structure_cap = product_cap * 0.2;
        product_cap + air_cap + structure_cap
    }

    pub fn time_constant_sec(&self) -> f64 {
        self.compute_r_th() * self.compute_c_th()
    }

    pub fn effective_cooling_power_w(&self) -> f64 {
        self.installed_cooling_power_w * self.cop
    }

    /// Heat leak at a given temperature differential (W)
    pub fn heat_leak_w(&self, ambient_temp_c: f64, internal_temp_c: f64) -> f64 {
        let delta_t = (ambient_temp_c - internal_temp_c).max(0.0);
        delta_t / self.compute_r_th()
    }

    /// Baseline duty cycle (0.0 to 1.0): fraction of time compressor needs to run
    /// to maintain setpoint against heat leak.
    ///
    /// At steady state: heat_leak = cooling_provided
    /// cooling_provided = compressor_power * cop * duty_cycle
    /// duty_cycle = heat_leak / (compressor_power * cop)
    pub fn baseline_duty_cycle(&self, ambient_temp_c: f64, setpoint_c: f64) -> f64 {
        let heat_leak = self.heat_leak_w(ambient_temp_c, setpoint_c);
        let max_cooling = self.effective_cooling_power_w();
        if max_cooling <= 0.0 {
            return 0.0;
        }
        let duty = heat_leak / max_cooling;
        // Cap at 90% max runtime. No artificial minimum — when ambient is cold,
        // heat leak is minimal and the compressor may not need to run at all.
        duty.clamp(0.0, 0.9)
    }

    /// Not used anymore — kept for backward compatibility
    pub fn baseline_power_kw(&self, ambient_temp_c: f64, setpoint_c: f64) -> f64 {
        self.baseline_duty_cycle(ambient_temp_c, setpoint_c)
    }

    pub fn description(&self) -> String {
        format!(
            "{} ({:.0}m³, U={:.2}, τ={:.0}h, {}kW compressors)",
            self.name,
            self.volume_m3,
            self.u_value_w_per_m2k,
            self.time_constant_sec() / 3600.0,
            self.installed_cooling_power_w / 1000.0
        )
    }
}
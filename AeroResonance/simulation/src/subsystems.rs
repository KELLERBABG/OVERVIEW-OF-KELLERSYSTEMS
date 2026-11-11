//! All energy-harvesting subsystem models.
//! Updated with: Tier -1 deep-hibernate, ground-coupled TEG, solar PV microcell.
//! INTEGRATION: Uses eh_sim_core::cap_aging for capacitor degradation.

use crate::config::SimConfig;
use crate::physics;
use esim_core::cap_aging::CapState;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// =========================  Shared State  =========================

#[derive(Debug, Clone)]
pub struct EnvState {
    pub time_s: f64,
    pub temp_ground_k: f64,
    pub temp_tower_top_k: f64,
    pub schumann_e_field_vpm: f64,
    pub solar_activity_index: f64,
    pub lightning_proximity: f64,
    pub diurnal_factor: f64,
    pub d_layer_abs_factor: f64,
    pub barometric_boost: f64,
    pub solar_irradiance_wpm2: f64,
    pub v_cap: f64,
    pub pmic_active: bool,
}

impl EnvState {
    pub fn from_config(cfg: &SimConfig, time_s: f64, rng: &mut impl Rng) -> Self {
        let t_ground_k = cfg.ground_temp_c + 273.15;
        let t_top = physics::temp_at_height(
            cfg.ground_temp_c,
            cfg.tower_height_m,
            cfg.lapse_rate_k_per_km,
        ) + 273.15;
        let diurnal = physics::diurnal_modulation(time_s, 0.5);
        let d_layer = physics::d_layer_absorption(time_s);
        let baro = physics::barometric_transient(time_s);
        let base_e = physics::schumann_e_field(1, cfg) * diurnal * d_layer;
        let solar_g = physics::solar_irradiance(time_s, rng, cfg);
        Self {
            time_s,
            temp_ground_k: t_ground_k,
            temp_tower_top_k: t_top,
            schumann_e_field_vpm: base_e + rng.gen_range(-1e-4..1e-4),
            solar_activity_index: rng.gen_range(0.0..0.3) + (time_s / 86400.0 % 27.0 / 27.0 * 0.2),
            lightning_proximity: if rng.gen_bool(0.001) {
                rng.gen_range(0.5..1.0)
            } else {
                0.0
            },
            diurnal_factor: diurnal,
            d_layer_abs_factor: d_layer,
            barometric_boost: baro,
            solar_irradiance_wpm2: solar_g,
            v_cap: 2.8,
            pmic_active: false,
        }
    }
}

// =========================  Subsystem Trait  =========================

pub trait Subsystem: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64;
    fn total_energy_j(&self) -> f64;
}

// =========================  4.1 Ionospheric Plasma Coupling  =========================
#[derive(Debug, Clone)]
pub struct IonosphericPlasmaCoupling {
    total_energy: f64,
}
impl IonosphericPlasmaCoupling {
    pub fn new() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Default for IonosphericPlasmaCoupling {
    fn default() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Subsystem for IonosphericPlasmaCoupling {
    fn name(&self) -> &'static str {
        "IonosphericPlasmaCoupling"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        let e = env.schumann_e_field_vpm.max(0.0);
        let d = e.powi(2) / 377.0;
        let g = 10.0_f64.powf(cfg.pc_antenna_gain_dbi / 10.0);
        let a = (cfg.nvis_element_length_m.powi(2) / (4.0 * std::f64::consts::PI)) * g;
        let p = cfg.pc_efficiency * d * a * cfg.pc_q_factor * env.d_layer_abs_factor;
        let c = p.min(1e-3);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.2 NVIS  =========================
#[derive(Debug, Clone)]
pub struct NearVerticalSkywaveInductor {
    total_energy: f64,
}
impl NearVerticalSkywaveInductor {
    pub fn new() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Default for NearVerticalSkywaveInductor {
    fn default() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Subsystem for NearVerticalSkywaveInductor {
    fn name(&self) -> &'static str {
        "NearVerticalSkywaveInductor"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        let pl = physics::nvis_path_loss_linear(cfg);
        let d = env.schumann_e_field_vpm.max(1e-15).powi(2) / 377.0;
        let a = 0.1 * (cfg.nvis_element_length_m / 2.0).powi(2) * cfg.nvis_antenna_count as f64;
        let p = cfg.pc_efficiency * d * a / pl.clamp(1.0, f64::MAX)
            * env.d_layer_abs_factor
            * env.diurnal_factor;
        let c = p.min(5e-3);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.3 Maxwellian  =========================
#[derive(Debug, Clone)]
pub struct MaxwellianFieldRelaxation {
    total_energy: f64,
    fdtd: physics::FDTD1D,
}
impl MaxwellianFieldRelaxation {
    pub fn new(cfg: &SimConfig) -> Self {
        let fdtd = physics::FDTD1D::new(
            cfg.mfr_fdtd_grid_cells,
            cfg.mfr_fdtd_courant,
            0.5, // dx [m] coarse grid, validated later against the physics gate
        );
        Self {
            total_energy: 0.0,
            fdtd,
        }
    }
}
impl Subsystem for MaxwellianFieldRelaxation {
    fn name(&self) -> &'static str {
        "MaxwellianFieldRelaxation"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        // Drive the local FDTD Maxwell kernel with the ambient Schumann field
        // and read back the instantaneous Poynting flux (E·H) as the available
        // energy density at the source point.
        let e = env.schumann_e_field_vpm.max(1e-10);
        let poynting = self
            .fdtd
            .step(physics::schumann_freq(1, cfg), e, env.time_s);
        let available_w_m2 = poynting.abs();
        let g = 1.0 + 10.0 * available_w_m2.max(1e-12) / 1e-3;
        let p = 1e-6
            * g
            * cfg.mfr_impedance_match_efficiency
            * env.d_layer_abs_factor
            * env.diurnal_factor;
        let c = p.min(1e-3);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.4 Acoustic  =========================
#[derive(Debug, Clone)]
pub struct AcousticAtmosphericTransduction {
    total_energy: f64,
}
impl AcousticAtmosphericTransduction {
    pub fn new() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Default for AcousticAtmosphericTransduction {
    fn default() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Subsystem for AcousticAtmosphericTransduction {
    fn name(&self) -> &'static str {
        "AcousticAtmosphericTransduction"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        let ep = cfg.infrasound_pressure_pa * env.barometric_boost;
        let ppt = physics::piezo_tile_power(
            ep,
            cfg.piezo_tile_area_m2,
            0.5e-3,
            cfg.piezo_strain_coeff_pc_n,
            cfg.infrasound_freq_hz,
            cfg.piezo_efficiency,
        );
        let t = ppt * cfg.piezo_tile_count as f64;
        let c = t.min(1e-3);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.5 NLM  =========================
#[derive(Debug, Clone)]
pub struct NonLinearWaveModulation {
    total_energy: f64,
}
impl NonLinearWaveModulation {
    pub fn new() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Default for NonLinearWaveModulation {
    fn default() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Subsystem for NonLinearWaveModulation {
    fn name(&self) -> &'static str {
        "NonLinearWaveModulation"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        let pw = 10.0_f64.powf((cfg.nlm_broadband_power_dbm - 30.0) / 10.0);
        let nd = env.schumann_e_field_vpm.max(1e-10).powi(2) / 377.0;
        let col = (pw + nd * cfg.nlm_antenna_aperture_m2) * env.d_layer_abs_factor;
        let r = col * cfg.nlm_schottky_efficiency * 0.8;
        let c = r.min(5e-4);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.6 Cyclostationary  =========================
#[derive(Debug, Clone)]
pub struct CyclostationaryFeatureFiltering {
    total_energy: f64,
}
impl CyclostationaryFeatureFiltering {
    pub fn new() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Default for CyclostationaryFeatureFiltering {
    fn default() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Subsystem for CyclostationaryFeatureFiltering {
    fn name(&self) -> &'static str {
        "CyclostationaryFeatureFiltering"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        let td = env.schumann_e_field_vpm.powi(2) / 377.0;
        let ex = td * 0.1 * cfg.cs_extraction_ratio * env.d_layer_abs_factor * env.diurnal_factor;
        let c = ex.min(1e-4);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.7 Plasma-Wake Buffer (cap_aging integrated)  =========================
#[derive(Debug)]
pub struct PlasmaWakeEnergyBuffering {
    total_energy: f64,
    stored_energy: f64,
    pub cap_state: CapState, // INTEGRATION: eh_sim_core aging model
}
impl PlasmaWakeEnergyBuffering {
    pub fn new(cfg: &SimConfig) -> Self {
        Self {
            total_energy: 0.0,
            stored_energy: 0.0,
            cap_state: CapState::new(cfg.buffer_capacitance_f, cfg.cap_leakage_current_a),
        }
    }
}
impl Subsystem for PlasmaWakeEnergyBuffering {
    fn name(&self) -> &'static str {
        "PlasmaWakeEnergyBuffering"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        // Use cap_aging: effective capacitance and temperature-dependent leakage
        let temp_c = cfg.ground_temp_c;
        let leak_a = self.cap_state.leakage_at_temp(temp_c);
        let leak = leak_a * env.v_cap;

        // Usable energy accounting for aging
        let usable = self
            .cap_state
            .usable_energy(cfg.pmic_v_high, cfg.pmic_v_low, temp_c);

        let surge = env.lightning_proximity.min(1.0);
        let capture_power = 0.1 * surge;
        let eff = physics::pmic_efficiency(env.v_cap, cfg);

        let out = if surge > 0.1 {
            let net = capture_power * cfg.buffer_charge_efficiency * eff;
            self.stored_energy += net * cfg.dt;
            self.stored_energy = self.stored_energy.min(usable * 2.0);
            // Record a charge cycle when surge events happen
            self.cap_state.record_cycle();
            capture_power
        } else {
            let drain = (usable / 3600.0).min(self.stored_energy / cfg.dt).min(1e-4);
            self.stored_energy = (self.stored_energy - drain * cfg.dt / eff).max(0.0);
            drain * eff
        };
        let c = (out - leak).clamp(0.0, 1e-3);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.8 Resonant Substrate (ADC-I + Tier -1)  =========================
#[derive(Debug, Clone)]
pub struct ResonantSubstrateAdaptation {
    total_energy: f64,
}
impl ResonantSubstrateAdaptation {
    pub fn new() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Default for ResonantSubstrateAdaptation {
    fn default() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Subsystem for ResonantSubstrateAdaptation {
    fn name(&self) -> &'static str {
        "ResonantSubstrateAdaptation"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        if env.v_cap < cfg.rs_v_cap_deep_hibernate {
            self.total_energy += -cfg.rs_deep_hibernate_power_w * cfg.dt;
            return -cfg.rs_deep_hibernate_power_w;
        }
        let (_interval, effective_dc) = if env.v_cap >= cfg.rs_v_cap_high {
            (cfg.rs_fast_interval_s, cfg.rs_duty_cycle.max(0.01))
        } else if env.v_cap >= cfg.rs_v_cap_low {
            (cfg.rs_sparse_interval_s, cfg.rs_duty_cycle * 0.1)
        } else {
            if cfg.rs_passive_fallback {
                return -cfg.rs_sleep_power_w;
            } else {
                (cfg.rs_sparse_interval_s * 10.0, cfg.rs_duty_cycle * 0.01)
            }
        };
        let a = cfg.rs_active_power_w;
        let s = cfg.rs_sleep_power_w;
        let dc = effective_dc.clamp(0.0, 1.0);
        let avg = dc * a + (1.0 - dc) * s;
        let jp = (cfg.rs_clock_jitter_ns * 10.0).min(1.0);
        let rtos = cfg.rs_rtos_overhead_pct / 100.0;
        let alpha = (1.0 - jp) * (1.0 - rtos);
        let net = -avg * (1.0 + (1.0 - alpha) * dc);
        self.total_energy += net * cfg.dt;
        net
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.9 TEG (ground-coupled)  =========================
#[derive(Debug, Clone)]
pub struct ThermodynamicCapacityGradient {
    total_energy: f64,
}
impl ThermodynamicCapacityGradient {
    pub fn new() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Default for ThermodynamicCapacityGradient {
    fn default() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Subsystem for ThermodynamicCapacityGradient {
    fn name(&self) -> &'static str {
        "ThermodynamicCapacityGradient"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        let dt = physics::teg_delta_t_with_env(env.time_s, cfg);
        if dt < 0.5 {
            return 0.0;
        }
        let sv = cfg.teg_seebeck_uv_per_k * 1e-6;
        let ri = physics::teg_internal_resistance(
            cfg.teg_thermal_cond_w_per_k,
            cfg.teg_zt,
            (env.temp_ground_k + (cfg.teg_ground_temp_c + 273.15)) / 2.0,
        );
        let (hot_k, cold_k) = if cfg.teg_ground_coupling {
            (env.temp_ground_k, cfg.teg_ground_temp_c + 273.15)
        } else {
            (
                env.temp_ground_k,
                (env.temp_tower_top_k).min(cfg.teg_cold_side_c + 273.15),
            )
        };
        let voc = physics::teg_voltage(sv, hot_k, cold_k);
        let v_opt = physics::mppt_teg_operating_v(voc, cfg.mppt_teg_voc_ratio);
        let v_op = env.v_cap * 0.5;
        let mp = physics::mppt_mismatch_penalty(v_op, v_opt);
        let ppm = physics::teg_power(sv, hot_k, cold_k, ri);
        let total = ppm * cfg.teg_module_count as f64 * cfg.teg_efficiency * mp;
        let c = total.min(0.1);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  4.10 Static Invariant Discharge  =========================
#[derive(Debug, Clone)]
pub struct StaticInvariantDischarge {
    total_energy: f64,
    total_discharged: f64,
}
impl StaticInvariantDischarge {
    pub fn new() -> Self {
        Self {
            total_energy: 0.0,
            total_discharged: 0.0,
        }
    }
}
impl Default for StaticInvariantDischarge {
    fn default() -> Self {
        Self {
            total_energy: 0.0,
            total_discharged: 0.0,
        }
    }
}
impl Subsystem for StaticInvariantDischarge {
    fn name(&self) -> &'static str {
        "StaticInvariantDischarge"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        // Duty-cycled protection monitor. The crowbar does not self-discharge
        // a healthy (below-threshold) bus; if overvoltage is detected, the
        // safety circuit clamps the excess charge.
        let dc = cfg.safety_duty_cycle.clamp(0.0, 1.0);
        let ap = if env.v_cap >= cfg.safety_overvoltage_v {
            500e-6 // active crowbar/response power draw
        } else {
            100e-6
        };
        let sp = cfg.safety_sleep_power_w;
        let avg = dc * ap + (1.0 - dc) * sp;

        // Static leakage through the protection network (GDT/MOV) at V_cap.
        let leakage_w = (cfg.safety_leakage_ma / 1000.0) * env.v_cap;

        // Energy removed when the safety circuit dumps excess charge
        // (response time in µs → energy = P·t_safety).
        if env.lightning_proximity > 0.7 || env.solar_activity_index > 0.8 {
            let dump_w = env.v_cap * env.v_cap / 60.0; // 60 Ω dump path
            let dump_j = dump_w * cfg.safety_response_us * 1e-6;
            self.total_discharged += dump_j;
        }

        let net = -(avg + leakage_w);
        self.total_energy += net * cfg.dt;
        net
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  Option C: 4.11 Solar Photovoltaic  =========================
#[derive(Debug, Clone)]
pub struct SolarPhotovoltaic {
    total_energy: f64,
}
impl SolarPhotovoltaic {
    pub fn new() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Default for SolarPhotovoltaic {
    fn default() -> Self {
        Self { total_energy: 0.0 }
    }
}
impl Subsystem for SolarPhotovoltaic {
    fn name(&self) -> &'static str {
        "SolarPhotovoltaic"
    }
    fn harvest_power(&mut self, env: &EnvState, cfg: &SimConfig) -> f64 {
        let irrad = env.solar_irradiance_wpm2;
        if irrad < 0.01 {
            return 0.0;
        }
        let power = physics::solar_pv_power(irrad, cfg.pv_cell_area_m2, cfg.pv_efficiency)
            * cfg.pv_cell_count as f64;
        let pmic_eff = physics::pmic_efficiency(env.v_cap, cfg);
        let net = power * pmic_eff;
        let c = net.min(1e-3);
        self.total_energy += c * cfg.dt;
        c
    }
    fn total_energy_j(&self) -> f64 {
        self.total_energy
    }
}

// =========================  2E. Energy Tiers — now re-exports eh_sim_core::scheduler  =========================
// INTEGRATION: Use eh_sim_core::scheduler::TaskTier directly.
use esim_core::scheduler::{TaskSchedulerConfig, TaskTier};

pub fn highest_executable_tier(v_cap: f64, cfg: &SimConfig) -> TaskTier {
    let sc = TaskSchedulerConfig {
        tier0_energy_j: cfg.tier0_energy_j,
        tier1_energy_j: cfg.tier1_energy_j,
        tier2_energy_j: cfg.tier2_energy_j,
        tier3_energy_j: cfg.tier3_energy_j,
        v_cap_tier0_min: cfg.v_cap_tier0_min,
        v_cap_tier1_min: cfg.v_cap_tier1_min,
        v_cap_tier2_min: cfg.v_cap_tier2_min,
        v_cap_tier3_min: cfg.v_cap_tier3_min,
    };
    sc.highest_tier(v_cap)
}

// =========================  Aggregate System  =========================

#[derive(Debug)]
pub struct System {
    pub subsystems: Vec<Box<dyn Subsystem>>,
    pub env: EnvState,
    pub v_cap: f64,
    pub v_cap_energy_j: f64,
    pub active_tier: TaskTier,
    pub seed: Option<u64>,
}

/// Build a seeded or unseeded RNG. With a seed the simulation is
/// bit-deterministic across runs (used by integration tests and `--seed`).
fn seeded_rng(seed: Option<u64>) -> Box<dyn FnMut() -> StdRng> {
    let seed_raw = seed.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0xE0)
    });
    // Return a closure so the per-step RNG is derived from one fixed seed.
    Box::new(move || StdRng::seed_from_u64(seed_raw))
}

impl System {
    pub fn new(cfg: &SimConfig, seed: Option<u64>) -> Self {
        let subsystems: Vec<Box<dyn Subsystem>> = vec![
            Box::new(IonosphericPlasmaCoupling::new()),
            Box::new(NearVerticalSkywaveInductor::new()),
            Box::new(MaxwellianFieldRelaxation::new(cfg)),
            Box::new(AcousticAtmosphericTransduction::new()),
            Box::new(NonLinearWaveModulation::new()),
            Box::new(CyclostationaryFeatureFiltering::new()),
            Box::new(PlasmaWakeEnergyBuffering::new(cfg)), // now takes cfg for CapState
            Box::new(ResonantSubstrateAdaptation::new()),
            Box::new(ThermodynamicCapacityGradient::new()),
            Box::new(StaticInvariantDischarge::new()),
            Box::new(SolarPhotovoltaic::new()),
        ];
        let mut rng_factory = seeded_rng(seed);
        let mut rng = rng_factory();
        let env = EnvState::from_config(cfg, 0.0, &mut rng);
        Self {
            subsystems,
            env,
            v_cap: cfg.pmic_v_low,
            v_cap_energy_j: 0.0,
            active_tier: TaskTier::TierMinus1,
            seed,
        }
    }

    pub fn step(&mut self, time_s: f64, cfg: &SimConfig) -> (Vec<(String, f64)>, f64, f64) {
        let mut rng_factory = seeded_rng(self.seed);
        let mut rng = rng_factory();
        self.env = EnvState::from_config(cfg, time_s, &mut rng);
        self.env.v_cap = self.v_cap;
        self.env.pmic_active = physics::pmic_is_active(self.v_cap, cfg);
        self.active_tier = highest_executable_tier(self.v_cap, cfg);

        let mut results = Vec::with_capacity(self.subsystems.len());
        let mut th = 0.0_f64;
        let mut tc = 0.0_f64;

        for sub in self.subsystems.iter_mut() {
            let power = sub.harvest_power(&self.env, cfg);
            results.push((sub.name().to_string(), power));
            if power > 0.0 {
                th += power;
            } else {
                tc -= power;
            }
        }

        let net = th - tc;
        let max_e = 0.5 * cfg.buffer_capacitance_f * cfg.pmic_v_high.powi(2);
        self.v_cap_energy_j = (self.v_cap_energy_j + net * cfg.dt).clamp(0.0, max_e);
        self.v_cap = (2.0 * self.v_cap_energy_j / cfg.buffer_capacitance_f)
            .sqrt()
            .max(0.0);
        let leak = physics::capacitor_leakage_power(cfg.cap_leakage_current_a, self.v_cap);
        self.v_cap_energy_j = (self.v_cap_energy_j - leak * cfg.dt).max(0.0);
        self.v_cap = (2.0 * self.v_cap_energy_j / cfg.buffer_capacitance_f)
            .sqrt()
            .max(0.0);

        (results, th, tc)
    }
}

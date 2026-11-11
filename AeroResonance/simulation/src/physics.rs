//! Physics primitives for the Atmospheric Resonance Simulation.
//!
//! Contains:
//!   • Earth-ionosphere cavity (Schumann resonances, Q-factor)
//!   • Atmospheric electric circuit
//!   • Ionospheric plasma
//!   • NVIS path loss model
//!   • 1D FDTD Maxwell solver kernel
//!   • Piezoelectric transduction model
//!   • Thermoelectric (Seebeck) model
//!   • Atmospheric lapse rate / temperature profile
//!   • 2A: PMIC hysteresis, capacitor ESR/leakage, cold-start
//!   • 2B: Diurnal cycles, thermal inversion, D-layer absorption
//!   • 2D: MS-MPPT helpers
//!   • Option B: Ground-coupled TEG (deep soil thermal mass)
//!   • Option C: Solar photovoltaic microcell

use crate::config::SimConfig;
use std::f64::consts::PI;

// =========================  Physical Constants  =========================
pub const C0: f64 = 299_792_458.0;
pub const EPS0: f64 = 8.854_187_817e-12;
pub const MU0: f64 = 1.256_637_062_12e-6;
pub const RE: f64 = 6_371_000.0;
pub const H_IONO: f64 = 80_000.0;

// =========================  1. Earth–Ionosphere Cavity (Schumann)  =========================

pub fn schumann_freq(n: u32, _cfg: &SimConfig) -> f64 {
    let r_eff = RE;
    let base = C0 / (2.0 * PI * r_eff);
    let nf = n as f64;
    base * (nf * (nf + 1.0)).sqrt()
}

pub fn schumann_e_field(n: u32, cfg: &SimConfig) -> f64 {
    let v_io = cfg.earth_ionosphere_potential_kv * 1000.0;
    let base_e = v_io / H_IONO;
    let coupling: f64 = match n {
        1 => 3.0e-4,
        2 => 1.5e-4,
        3 => 8.0e-5,
        4 => 4.0e-5,
        5 => 2.0e-5,
        _ => 1.0e-5,
    };
    base_e * coupling
}

pub fn temp_at_height(ground_c: f64, height_m: f64, lapse_k_per_km: f64) -> f64 {
    ground_c - lapse_k_per_km * (height_m / 1000.0)
}

// =========================  4. NVIS Path Loss  =========================

pub fn nvis_path_loss_linear(cfg: &SimConfig) -> f64 {
    let f_mhz = cfg.fof2_mhz * 0.85;
    let lambda = C0 / (f_mhz * 1e6);
    let dist = 2.0 * H_IONO;
    let free_space = (4.0 * PI * dist / lambda).powi(2);
    free_space * 2.0 * 1.5 // reflection + ground loss
}

// =========================  5. 1D FDTD Maxwell Kernel  =========================

#[derive(Debug, Clone)]
pub struct FDTD1D {
    pub ez: Vec<f64>,
    pub hy: Vec<f64>,
    pub n: usize,
    pub dx: f64,
    pub dt: f64,
    pub source_idx: usize,
}

impl FDTD1D {
    pub fn new(n_cells: usize, courant: f64, dx: f64) -> Self {
        let dt = courant * dx / C0;
        Self {
            ez: vec![0.0; n_cells],
            hy: vec![0.0; n_cells],
            n: n_cells,
            dx,
            dt,
            source_idx: n_cells / 2,
        }
    }

    pub fn step(&mut self, f_hz: f64, amplitude: f64, time_s: f64) -> f64 {
        for i in 0..(self.n - 1) {
            self.hy[i] += (self.dt / (MU0 * self.dx)) * (self.ez[i + 1] - self.ez[i]);
        }
        for i in 1..self.n {
            self.ez[i] += (self.dt / (EPS0 * self.dx)) * (self.hy[i] - self.hy[i - 1]);
            if i == self.source_idx {
                let omega = 2.0 * PI * f_hz;
                let source = amplitude * (omega * time_s).sin();
                self.ez[i] += source * self.dt / (EPS0 * self.dx);
            }
        }
        self.ez[self.source_idx] * self.hy[self.source_idx]
    }
}

// =========================  6. Piezoelectric Transduction  =========================

pub fn piezo_tile_power(
    pressure_pa: f64,
    area_m2: f64,
    thickness_m: f64,
    d33_pc_n: f64,
    frequency_hz: f64,
    efficiency: f64,
) -> f64 {
    let d33 = d33_pc_n * 1e-12;
    let eps_r = 1200.0;
    let cap = EPS0 * eps_r * area_m2 / thickness_m;
    let charge = d33 * pressure_pa * area_m2;
    let voltage = charge / cap;
    efficiency * 0.5 * cap * voltage.powi(2) * frequency_hz
}

// =========================  7. Thermoelectric (Seebeck) Model  =========================

pub fn teg_voltage(seebeck_v_per_k: f64, t_hot_k: f64, t_cold_k: f64) -> f64 {
    let dt = (t_hot_k - t_cold_k).max(0.0);
    seebeck_v_per_k * dt
}

pub fn teg_power(seebeck_v_per_k: f64, t_hot_k: f64, t_cold_k: f64, r_internal_ohm: f64) -> f64 {
    let voc = teg_voltage(seebeck_v_per_k, t_hot_k, t_cold_k);
    voc.powi(2) / (4.0 * r_internal_ohm)
}

pub fn teg_internal_resistance(_thermal_cond_w_per_k: f64, zt: f64, t_mean_k: f64) -> f64 {
    if zt <= 0.0 {
        2.0
    } else {
        (t_mean_k * zt).recip().clamp(0.5, 10.0)
    }
}

pub fn teg_temperature_difference(cfg: &SimConfig) -> f64 {
    let t_ground_k = cfg.ground_temp_c + 273.15;
    let t_height_k = temp_at_height(
        cfg.ground_temp_c,
        cfg.tower_height_m,
        cfg.lapse_rate_k_per_km,
    ) + 273.15;
    let t_cold = cfg.teg_cold_side_c + 273.15;
    let effective_cold = t_height_k.min(t_cold);
    (t_ground_k - effective_cold).max(0.0)
}

// =========================  2A. PMIC / Capacitor Model  =========================

pub fn capacitor_leakage_power(leak_a: f64, v_cap: f64) -> f64 {
    leak_a * v_cap
}

pub fn pmic_efficiency(v_cap: f64, cfg: &SimConfig) -> f64 {
    if v_cap < cfg.pmic_v_cold {
        cfg.pmic_cold_start_efficiency
    } else {
        cfg.pmic_main_efficiency
    }
}

pub fn pmic_is_active(v_cap: f64, cfg: &SimConfig) -> bool {
    v_cap >= cfg.pmic_v_low
}

// =========================  2B. Diurnal & Weather Model  =========================

pub fn diurnal_modulation(time_s: f64, phase_offset_rad: f64) -> f64 {
    let day_phase = 2.0 * PI * time_s / 86400.0 + phase_offset_rad;
    (day_phase.sin() * 0.5 + 0.5).max(0.0)
}

pub fn thermal_inversion_factor(time_s: f64) -> f64 {
    let day_phase = (2.0 * PI * time_s / 86400.0) % (2.0 * PI);
    let dawn_dip = ((day_phase - 0.25 * PI) / (0.1 * PI))
        .cos()
        .max(0.0)
        .powi(4);
    let dusk_dip = ((day_phase - 0.75 * PI) / (0.1 * PI))
        .cos()
        .max(0.0)
        .powi(4);
    let dip = dawn_dip + dusk_dip;
    (1.0 - dip * 0.8).max(0.2)
}

pub fn d_layer_absorption(time_s: f64) -> f64 {
    let solar_elevation = diurnal_modulation(time_s, 0.0);
    let absorption = solar_elevation * 0.6;
    (1.0 - absorption).max(0.4)
}

pub fn barometric_transient(time_s: f64) -> f64 {
    let cycle = 86400.0 * 4.0;
    let phase = (2.0 * PI * time_s / cycle).sin();
    1.0 + (phase.max(0.0) * 3.0)
}

// =========================  Option B: Ground-Coupled TEG ΔT  =========================

/// Combined environmental TEG temperature difference.
/// Option B: If ground coupling is enabled, uses deep ground (~11°C) as cold side.
/// This guarantees a minimum ΔT ~2.5°C even with thermal inversions or short towers.
pub fn teg_delta_t_with_env(time_s: f64, cfg: &SimConfig) -> f64 {
    let inv_factor = thermal_inversion_factor(time_s);
    if cfg.teg_ground_coupling {
        let t_ground_k = cfg.ground_temp_c + 273.15;
        let t_deep_ground_k = cfg.teg_ground_temp_c + 273.15;
        let dt = (t_ground_k - t_deep_ground_k).max(0.0) * inv_factor;
        // Ground thermal inertia minimum. The value is UNVALIDATED; it is
        // configured via `teg_min_dt_degc` and documented in the gate
        // report rather than hardcoded ("guarantee") here.
        dt.max(cfg.teg_min_dt_degc)
    } else {
        let base_dt = teg_temperature_difference(cfg);
        base_dt * inv_factor
    }
}

// =========================  Option C: Solar Photovoltaic Microcell  =========================

/// Power from a photovoltaic cell [W].
/// P = η · G · A
pub fn solar_pv_power(irradiance_wpm2: f64, area_m2: f64, efficiency: f64) -> f64 {
    irradiance_wpm2 * area_m2 * efficiency
}

/// Combined solar irradiance based on diurnal cycle and overcast probability.
/// Returns [W/m²].
pub fn solar_irradiance(time_s: f64, rng: &mut impl rand::Rng, cfg: &SimConfig) -> f64 {
    let diurnal = diurnal_modulation(time_s, 0.0);
    if diurnal < 0.01 {
        return 0.0;
    } // night-time
    let is_overcast = rng.gen_bool(cfg.pv_overcast_probability);
    let base = if is_overcast {
        cfg.pv_irradiance_overcast_wpm2
    } else {
        cfg.pv_irradiance_clear_wpm2
    };
    base * diurnal
}

// =========================  2D. MS-MPPT Helpers  =========================

pub fn mppt_teg_operating_v(voc: f64, ratio: f64) -> f64 {
    voc * ratio.clamp(0.3, 0.7)
}

pub fn mppt_mismatch_penalty(v_op: f64, v_opt: f64) -> f64 {
    if v_opt <= 0.0 {
        return 0.5;
    }
    let ratio = (v_op / v_opt).clamp(0.1, 2.0);
    if ratio >= 1.0 {
        (1.0 / ratio).min(1.0)
    } else {
        ratio
    }
}

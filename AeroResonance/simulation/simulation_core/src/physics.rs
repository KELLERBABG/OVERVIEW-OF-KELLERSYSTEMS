//! Core physical constants and models — no_std compatible with libm.

use libm::sin;

// =====================  Constants  =====================
pub const C0: f64 = 299_792_458.0;
pub const EPS0: f64 = 8.854_187_817e-12;
pub const H_IONO: f64 = 80_000.0;
pub const PI: f64 = core::f64::consts::PI;

// =====================  Schumann  =====================
pub fn schumann_e_field_surface(v_io_kv: f64) -> f64 {
    let base = (v_io_kv * 1000.0) / H_IONO;
    base * 3.0e-4
}

// =====================  TEG  =====================
pub fn teg_voltage(seebeck_vk: f64, hot_k: f64, cold_k: f64) -> f64 {
    let dt = (hot_k - cold_k).max(0.0);
    seebeck_vk * dt
}

pub fn teg_power(seebeck_vk: f64, hot_k: f64, cold_k: f64, r_int: f64) -> f64 {
    let voc = teg_voltage(seebeck_vk, hot_k, cold_k);
    voc * voc / (4.0 * r_int)
}

pub fn teg_internal_resistance(zt: f64, t_mean_k: f64) -> f64 {
    if zt <= 0.0 {
        2.0
    } else {
        (t_mean_k * zt).recip().clamp(0.5, 10.0)
    }
}

// =====================  Piezo  =====================
pub fn piezo_tile_power(
    pressure_pa: f64,
    area_m2: f64,
    thick_m: f64,
    d33_pc_n: f64,
    freq_hz: f64,
    eff: f64,
) -> f64 {
    let d33 = d33_pc_n * 1e-12;
    let eps_r = 1200.0;
    let cap = EPS0 * eps_r * area_m2 / thick_m;
    let charge = d33 * pressure_pa * area_m2;
    let voltage = charge / cap;
    eff * 0.5 * cap * (voltage * voltage) * freq_hz
}

// =====================  PMIC / Capacitor  =====================
pub fn cap_usable_energy(c: f64, v_high: f64, v_low: f64) -> f64 {
    if v_high <= v_low {
        0.0
    } else {
        0.5 * c * (v_high * v_high - v_low * v_low)
    }
}

pub fn cap_leakage_power(leak_a: f64, v: f64) -> f64 {
    leak_a * v
}

pub fn pmic_eff(v: f64, v_cold: f64, cold_eff: f64, main_eff: f64) -> f64 {
    if v < v_cold {
        cold_eff
    } else {
        main_eff
    }
}

// =====================  Solar  =====================
pub fn solar_pv_power(g_wpm2: f64, area_m2: f64, eff: f64) -> f64 {
    g_wpm2 * area_m2 * eff
}

// =====================  Diurnal / Environment  =====================
pub fn diurnal(time_s: f64, phase: f64) -> f64 {
    (sin(2.0 * PI * time_s / 86400.0 + phase) * 0.5 + 0.5).max(0.0)
}

pub fn d_layer_absorption(time_s: f64) -> f64 {
    let sol = diurnal(time_s, 0.0);
    (1.0 - sol * 0.6).max(0.4)
}

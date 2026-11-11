// ============================================================================
// Store type definitions — 20 facility types with full ATA.canvas physics params
// ============================================================================

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StoreType {
    pub name: &'static str,
    pub volume_m3: f64,
    pub surface_m2: f64,
    pub u_value: f64,
    pub cooling_power_kw: f64,
    pub cop_base: f64,
    pub thermal_mass_kj_k: f64,
    pub t_min: f64,
    pub t_max: f64,
    pub t_setpoint: f64,
    pub internal_heat_kw: f64,
    pub _price_threshold_cheap: f64,
    pub _price_threshold_expensive: f64,
    pub is_retail: bool,
    pub has_defrost: bool,
    pub defrost_interval_h: f64,
    pub defrost_duration_h: f64,
    pub defrost_energy_kw: f64,
    pub has_tes: bool,
    pub tes_capacity_kwh: f64,
    pub cargo_mass_kg: f64,
    pub cargo_specific_heat: f64,
    pub fan_power_kw: f64,
    pub air_change_rate_ach: f64,
    pub _vfd_enabled: bool,
    pub min_runtime_sec: f64,
    pub min_offtime_sec: f64,
    pub max_shed_hours: f64,
}

const CHEAP: f64 = 30.0;
const EXPENSIVE: f64 = 80.0;

pub const STORE_TYPES: &[StoreType] = &[
    StoreType { name: "aldi", volume_m3: 4_000.0, surface_m2: 2_000.0, u_value: 0.25, cooling_power_kw: 200.0, cop_base: 1.8, thermal_mass_kj_k: 8_000_000.0, t_min: -25.0, t_max: -18.0, t_setpoint: -22.0, internal_heat_kw: 60.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: true, has_defrost: true, defrost_interval_h: 8.0, defrost_duration_h: 0.5, defrost_energy_kw: 40.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 200_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 3.0, air_change_rate_ach: 0.5, _vfd_enabled: true, min_runtime_sec: 600.0, min_offtime_sec: 600.0, max_shed_hours: 2.5 },
    StoreType { name: "edeka", volume_m3: 8_000.0, surface_m2: 3_500.0, u_value: 0.22, cooling_power_kw: 350.0, cop_base: 2.0, thermal_mass_kj_k: 16_000_000.0, t_min: -25.0, t_max: -18.0, t_setpoint: -22.0, internal_heat_kw: 120.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: true, has_defrost: true, defrost_interval_h: 6.0, defrost_duration_h: 0.5, defrost_energy_kw: 70.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 400_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 8.0, air_change_rate_ach: 1.0, _vfd_enabled: true, min_runtime_sec: 600.0, min_offtime_sec: 600.0, max_shed_hours: 2.0 },
    StoreType { name: "marktkauf", volume_m3: 25_000.0, surface_m2: 10_000.0, u_value: 0.25, cooling_power_kw: 900.0, cop_base: 2.1, thermal_mass_kj_k: 50_000_000.0, t_min: -25.0, t_max: -18.0, t_setpoint: -22.0, internal_heat_kw: 400.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: true, has_defrost: true, defrost_interval_h: 6.0, defrost_duration_h: 0.5, defrost_energy_kw: 180.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 1_250_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 25.0, air_change_rate_ach: 1.5, _vfd_enabled: true, min_runtime_sec: 600.0, min_offtime_sec: 600.0, max_shed_hours: 2.0 },
    StoreType { name: "freezer", volume_m3: 100_000.0, surface_m2: 15_000.0, u_value: 0.12, cooling_power_kw: 1500.0, cop_base: 1.5, thermal_mass_kj_k: 60_000_000.0, t_min: -28.0, t_max: -20.0, t_setpoint: -25.0, internal_heat_kw: 50.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 12.0, defrost_duration_h: 0.5, defrost_energy_kw: 180.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 5_000_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 35.0, air_change_rate_ach: 0.1, _vfd_enabled: true, min_runtime_sec: 7200.0, min_offtime_sec: 7200.0, max_shed_hours: 4.0 },
    StoreType { name: "cold", volume_m3: 120_000.0, surface_m2: 18_000.0, u_value: 0.18, cooling_power_kw: 1800.0, cop_base: 2.8, thermal_mass_kj_k: 45_000_000.0, t_min: 0.0, t_max: 6.0, t_setpoint: 4.0, internal_heat_kw: 80.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 8.0, defrost_duration_h: 0.5, defrost_energy_kw: 200.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 4_000_000.0, cargo_specific_heat: 3500.0, fan_power_kw: 45.0, air_change_rate_ach: 0.2, _vfd_enabled: true, min_runtime_sec: 5400.0, min_offtime_sec: 5400.0, max_shed_hours: 2.0 },
    StoreType { name: "frozen", volume_m3: 50_000.0, surface_m2: 8_500.0, u_value: 0.15, cooling_power_kw: 800.0, cop_base: 1.8, thermal_mass_kj_k: 25_000_000.0, t_min: -25.0, t_max: -18.0, t_setpoint: -22.0, internal_heat_kw: 35.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 8.0, defrost_duration_h: 0.5, defrost_energy_kw: 100.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 2_000_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 18.0, air_change_rate_ach: 0.15, _vfd_enabled: true, min_runtime_sec: 7200.0, min_offtime_sec: 7200.0, max_shed_hours: 3.0 },
    StoreType { name: "pharma", volume_m3: 15_000.0, surface_m2: 4_500.0, u_value: 0.14, cooling_power_kw: 300.0, cop_base: 3.0, thermal_mass_kj_k: 12_000_000.0, t_min: 2.0, t_max: 8.0, t_setpoint: 5.0, internal_heat_kw: 15.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 12.0, defrost_duration_h: 0.3, defrost_energy_kw: 30.0, has_tes: true, tes_capacity_kwh: 2000.0, cargo_mass_kg: 300_000.0, cargo_specific_heat: 3000.0, fan_power_kw: 10.0, air_change_rate_ach: 0.05, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.5 },
    StoreType { name: "cryo", volume_m3: 5_000.0, surface_m2: 2_200.0, u_value: 0.08, cooling_power_kw: 3500.0, cop_base: 0.4, thermal_mass_kj_k: 8_000_000.0, t_min: -196.0, t_max: -150.0, t_setpoint: -180.0, internal_heat_kw: 150.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 10_000.0, cargo_mass_kg: 50_000.0, cargo_specific_heat: 1500.0, fan_power_kw: 50.0, air_change_rate_ach: 0.01, _vfd_enabled: true, min_runtime_sec: 1800.0, min_offtime_sec: 1800.0, max_shed_hours: 0.1 },
    StoreType { name: "immersion", volume_m3: 15_000.0, surface_m2: 4_000.0, u_value: 0.15, cooling_power_kw: 5000.0, cop_base: 5.5, thermal_mass_kj_k: 65_000_000.0, t_min: 35.0, t_max: 50.0, t_setpoint: 42.0, internal_heat_kw: 4500.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 20_000.0, cargo_mass_kg: 400_000.0, cargo_specific_heat: 1900.0, fan_power_kw: 120.0, air_change_rate_ach: 0.0, _vfd_enabled: true, min_runtime_sec: 900.0, min_offtime_sec: 900.0, max_shed_hours: 0.02 },
    StoreType { name: "mortuary", volume_m3: 3_000.0, surface_m2: 1_800.0, u_value: 0.18, cooling_power_kw: 120.0, cop_base: 3.0, thermal_mass_kj_k: 3_500_000.0, t_min: 1.0, t_max: 6.0, t_setpoint: 3.0, internal_heat_kw: 10.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 12.0, defrost_duration_h: 0.3, defrost_energy_kw: 15.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 150_000.0, cargo_specific_heat: 3400.0, fan_power_kw: 4.0, air_change_rate_ach: 0.2, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 1.0 },
    StoreType { name: "datacenter", volume_m3: 80_000.0, surface_m2: 12_000.0, u_value: 0.30, cooling_power_kw: 6000.0, cop_base: 4.8, thermal_mass_kj_k: 8_500_000.0, t_min: 18.0, t_max: 27.0, t_setpoint: 22.0, internal_heat_kw: 4000.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 32_000.0, cargo_mass_kg: 500_000.0, cargo_specific_heat: 900.0, fan_power_kw: 200.0, air_change_rate_ach: 0.2, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.08 },
    StoreType { name: "chemistry", volume_m3: 150_000.0, surface_m2: 20_000.0, u_value: 0.45, cooling_power_kw: 9000.0, cop_base: 3.4, thermal_mass_kj_k: 45_000_000.0, t_min: 15.0, t_max: 30.0, t_setpoint: 20.0, internal_heat_kw: 5000.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 40_000.0, cargo_mass_kg: 2_000_000.0, cargo_specific_heat: 1500.0, fan_power_kw: 300.0, air_change_rate_ach: 0.5, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.17 },
    StoreType { name: "industry", volume_m3: 500_000.0, surface_m2: 50_000.0, u_value: 0.70, cooling_power_kw: 12000.0, cop_base: 3.8, thermal_mass_kj_k: 95_000_000.0, t_min: 15.0, t_max: 35.0, t_setpoint: 22.0, internal_heat_kw: 7000.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 56_000.0, cargo_mass_kg: 5_000_000.0, cargo_specific_heat: 800.0, fan_power_kw: 500.0, air_change_rate_ach: 1.0, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.5 },
    StoreType { name: "facility", volume_m3: 30_000.0, surface_m2: 5_000.0, u_value: 0.15, cooling_power_kw: 450.0, cop_base: 1.7, thermal_mass_kj_k: 18_000_000.0, t_min: -25.0, t_max: -18.0, t_setpoint: -22.0, internal_heat_kw: 25.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 12.0, defrost_duration_h: 0.5, defrost_energy_kw: 90.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 1_000_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 5.0, air_change_rate_ach: 0.1, _vfd_enabled: true, min_runtime_sec: 7200.0, min_offtime_sec: 7200.0, max_shed_hours: 5.0 },
    StoreType { name: "warehouse", volume_m3: 200_000.0, surface_m2: 25_000.0, u_value: 0.28, cooling_power_kw: 1200.0, cop_base: 2.9, thermal_mass_kj_k: 55_000_000.0, t_min: 2.0, t_max: 8.0, t_setpoint: 4.0, internal_heat_kw: 180.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 8.0, defrost_duration_h: 0.5, defrost_energy_kw: 240.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 3_000_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 40.0, air_change_rate_ach: 0.5, _vfd_enabled: true, min_runtime_sec: 7200.0, min_offtime_sec: 7200.0, max_shed_hours: 1.5 },
    StoreType { name: "blast_freezer", volume_m3: 8_000.0, surface_m2: 3_200.0, u_value: 0.10, cooling_power_kw: 2500.0, cop_base: 1.1, thermal_mass_kj_k: 15_000_000.0, t_min: -40.0, t_max: -30.0, t_setpoint: -35.0, internal_heat_kw: 120.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 6.0, defrost_duration_h: 0.5, defrost_energy_kw: 150.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 100_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 150.0, air_change_rate_ach: 0.5, _vfd_enabled: true, min_runtime_sec: 1800.0, min_offtime_sec: 1800.0, max_shed_hours: 0.25 },
    StoreType { name: "brewery", volume_m3: 80_000.0, surface_m2: 10_000.0, u_value: 0.15, cooling_power_kw: 2000.0, cop_base: 3.2, thermal_mass_kj_k: 120_000_000.0, t_min: -10.0, t_max: -2.0, t_setpoint: -5.0, internal_heat_kw: 150.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 15_000.0, cargo_mass_kg: 3_000_000.0, cargo_specific_heat: 4184.0, fan_power_kw: 30.0, air_change_rate_ach: 0.1, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 4.0 },
    StoreType { name: "arena", volume_m3: 150_000.0, surface_m2: 22_000.0, u_value: 0.35, cooling_power_kw: 1200.0, cop_base: 2.5, thermal_mass_kj_k: 200_000_000.0, t_min: -12.0, t_max: -5.0, t_setpoint: -8.0, internal_heat_kw: 250.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 500_000.0, cargo_specific_heat: 2108.0, fan_power_kw: 40.0, air_change_rate_ach: 1.0, _vfd_enabled: true, min_runtime_sec: 7200.0, min_offtime_sec: 7200.0, max_shed_hours: 6.0 },
    StoreType { name: "marine", volume_m3: 120_000.0, surface_m2: 15_000.0, u_value: 0.16, cooling_power_kw: 3000.0, cop_base: 1.8, thermal_mass_kj_k: 80_000_000.0, t_min: -30.0, t_max: -18.0, t_setpoint: -25.0, internal_heat_kw: 100.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 8.0, defrost_duration_h: 0.5, defrost_energy_kw: 250.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 6_000_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 80.0, air_change_rate_ach: 0.2, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 2.0 },
    StoreType { name: "ripening", volume_m3: 12_000.0, surface_m2: 3_800.0, u_value: 0.20, cooling_power_kw: 400.0, cop_base: 4.5, thermal_mass_kj_k: 10_000_000.0, t_min: 12.0, t_max: 18.0, t_setpoint: 15.0, internal_heat_kw: 45.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 250_000.0, cargo_specific_heat: 3600.0, fan_power_kw: 15.0, air_change_rate_ach: 2.0, _vfd_enabled: true, min_runtime_sec: 1800.0, min_offtime_sec: 1800.0, max_shed_hours: 0.75 },
    StoreType { name: "hvac", volume_m3: 50_000.0, surface_m2: 15_000.0, u_value: 0.35, cooling_power_kw: 1000.0, cop_base: 3.5, thermal_mass_kj_k: 20_000_000.0, t_min: 20.0, t_max: 26.0, t_setpoint: 22.0, internal_heat_kw: 300.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 0.0, cargo_specific_heat: 0.0, fan_power_kw: 80.0, air_change_rate_ach: 1.5, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.5 },

    // ── 3-TIER ENTERPRISE TIERS (Klein / Startup -> Mittelstand -> Großkonzern) ──
    // Tier 1: Kleinbetrieb / Startup (150 - 300 kW)
    StoreType { name: "freezer_small", volume_m3: 15_000.0, surface_m2: 4_500.0, u_value: 0.15, cooling_power_kw: 250.0, cop_base: 1.6, thermal_mass_kj_k: 10_000_000.0, t_min: -28.0, t_max: -20.0, t_setpoint: -25.0, internal_heat_kw: 15.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 12.0, defrost_duration_h: 0.5, defrost_energy_kw: 30.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 750_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 6.0, air_change_rate_ach: 0.15, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 3.5 },
    StoreType { name: "brewery_small", volume_m3: 12_000.0, surface_m2: 3_500.0, u_value: 0.18, cooling_power_kw: 250.0, cop_base: 3.2, thermal_mass_kj_k: 15_000_000.0, t_min: -10.0, t_max: -2.0, t_setpoint: -5.0, internal_heat_kw: 25.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 2_000.0, cargo_mass_kg: 350_000.0, cargo_specific_heat: 4184.0, fan_power_kw: 5.0, air_change_rate_ach: 0.1, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 4.0 },
    StoreType { name: "dc_small", volume_m3: 6_000.0, surface_m2: 2_200.0, u_value: 0.30, cooling_power_kw: 400.0, cop_base: 4.8, thermal_mass_kj_k: 1_200_000.0, t_min: 18.0, t_max: 27.0, t_setpoint: 22.0, internal_heat_kw: 250.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 2_500.0, cargo_mass_kg: 40_000.0, cargo_specific_heat: 900.0, fan_power_kw: 15.0, air_change_rate_ach: 0.2, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.08 },
    StoreType { name: "industry_small", volume_m3: 35_000.0, surface_m2: 6_000.0, u_value: 0.60, cooling_power_kw: 800.0, cop_base: 3.6, thermal_mass_kj_k: 12_000_000.0, t_min: 15.0, t_max: 35.0, t_setpoint: 22.0, internal_heat_kw: 500.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 5_000.0, cargo_mass_kg: 400_000.0, cargo_specific_heat: 800.0, fan_power_kw: 35.0, air_change_rate_ach: 0.8, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.5 },

    // Tier 2: Mittelstand (KMU / Regional Champion) (1.000 - 2.500 kW)
    StoreType { name: "freezer_mid", volume_m3: 70_000.0, surface_m2: 12_000.0, u_value: 0.12, cooling_power_kw: 1_200.0, cop_base: 1.5, thermal_mass_kj_k: 45_000_000.0, t_min: -28.0, t_max: -20.0, t_setpoint: -25.0, internal_heat_kw: 40.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 12.0, defrost_duration_h: 0.5, defrost_energy_kw: 140.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 3_500_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 28.0, air_change_rate_ach: 0.1, _vfd_enabled: true, min_runtime_sec: 7200.0, min_offtime_sec: 7200.0, max_shed_hours: 4.0 },
    StoreType { name: "brewery_mid", volume_m3: 60_000.0, surface_m2: 8_500.0, u_value: 0.15, cooling_power_kw: 1_500.0, cop_base: 3.2, thermal_mass_kj_k: 80_000_000.0, t_min: -10.0, t_max: -2.0, t_setpoint: -5.0, internal_heat_kw: 110.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 10_000.0, cargo_mass_kg: 2_000_000.0, cargo_specific_heat: 4184.0, fan_power_kw: 22.0, air_change_rate_ach: 0.1, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 4.0 },
    StoreType { name: "dc_mid", volume_m3: 40_000.0, surface_m2: 7_500.0, u_value: 0.30, cooling_power_kw: 3_000.0, cop_base: 4.8, thermal_mass_kj_k: 4_500_000.0, t_min: 18.0, t_max: 27.0, t_setpoint: 22.0, internal_heat_kw: 2_000.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 16_000.0, cargo_mass_kg: 250_000.0, cargo_specific_heat: 900.0, fan_power_kw: 100.0, air_change_rate_ach: 0.2, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.08 },
    StoreType { name: "industry_mid", volume_m3: 200_000.0, surface_m2: 25_000.0, u_value: 0.65, cooling_power_kw: 4_500.0, cop_base: 3.8, thermal_mass_kj_k: 40_000_000.0, t_min: 15.0, t_max: 35.0, t_setpoint: 22.0, internal_heat_kw: 2_800.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 22_000.0, cargo_mass_kg: 2_000_000.0, cargo_specific_heat: 800.0, fan_power_kw: 180.0, air_change_rate_ach: 0.9, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.5 },

    // Tier 3: Großkonzern / Hyperscale (5.000 - 15.000 kW)
    StoreType { name: "freezer_large", volume_m3: 250_000.0, surface_m2: 30_000.0, u_value: 0.10, cooling_power_kw: 3_500.0, cop_base: 1.5, thermal_mass_kj_k: 150_000_000.0, t_min: -28.0, t_max: -20.0, t_setpoint: -25.0, internal_heat_kw: 120.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: true, defrost_interval_h: 12.0, defrost_duration_h: 0.5, defrost_energy_kw: 400.0, has_tes: false, tes_capacity_kwh: 0.0, cargo_mass_kg: 12_000_000.0, cargo_specific_heat: 2000.0, fan_power_kw: 80.0, air_change_rate_ach: 0.08, _vfd_enabled: true, min_runtime_sec: 7200.0, min_offtime_sec: 7200.0, max_shed_hours: 4.5 },
    StoreType { name: "brewery_large", volume_m3: 180_000.0, surface_m2: 20_000.0, u_value: 0.14, cooling_power_kw: 4_500.0, cop_base: 3.2, thermal_mass_kj_k: 260_000_000.0, t_min: -10.0, t_max: -2.0, t_setpoint: -5.0, internal_heat_kw: 350.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 35_000.0, cargo_mass_kg: 7_000_000.0, cargo_specific_heat: 4184.0, fan_power_kw: 70.0, air_change_rate_ach: 0.08, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 4.5 },
    StoreType { name: "dc_large", volume_m3: 150_000.0, surface_m2: 22_000.0, u_value: 0.28, cooling_power_kw: 12_000.0, cop_base: 4.8, thermal_mass_kj_k: 18_000_000.0, t_min: 18.0, t_max: 27.0, t_setpoint: 22.0, internal_heat_kw: 8_500.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 65_000.0, cargo_mass_kg: 1_200_000.0, cargo_specific_heat: 900.0, fan_power_kw: 400.0, air_change_rate_ach: 0.2, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.08 },
    StoreType { name: "industry_large", volume_m3: 650_000.0, surface_m2: 65_000.0, u_value: 0.70, cooling_power_kw: 16_000.0, cop_base: 3.8, thermal_mass_kj_k: 130_000_000.0, t_min: 15.0, t_max: 35.0, t_setpoint: 22.0, internal_heat_kw: 9_500.0, _price_threshold_cheap: CHEAP, _price_threshold_expensive: EXPENSIVE, is_retail: false, has_defrost: false, defrost_interval_h: 0.0, defrost_duration_h: 0.0, defrost_energy_kw: 0.0, has_tes: true, tes_capacity_kwh: 80_000.0, cargo_mass_kg: 7_000_000.0, cargo_specific_heat: 800.0, fan_power_kw: 700.0, air_change_rate_ach: 1.0, _vfd_enabled: true, min_runtime_sec: 3600.0, min_offtime_sec: 3600.0, max_shed_hours: 0.5 },
];

pub fn find_type(name: &str) -> &'static StoreType {
    let lower = name.to_lowercase();
    for t in STORE_TYPES {
        if lower == t.name { return t; }
    }
    let mut best: Option<&'static StoreType> = None;
    let mut best_len: usize = 0;
    for t in STORE_TYPES {
        if (lower.contains(t.name) || t.name.contains(&lower)) && t.name.len() > best_len {
            best = Some(t);
            best_len = t.name.len();
        }
    }
    if let Some(t) = best { return t; }
    eprintln!("\n❌ Unknown type '{}'. Available types: {}", name,
        STORE_TYPES.iter().map(|t| t.name).collect::<Vec<_>>().join(", "));
    std::process::exit(1);
}

pub fn parse_fleet(spec: &str) -> Vec<(&'static StoreType, u64)> {
    let mut fleet = Vec::new();
    for part in spec.split('+') {
        let part = part.trim();
        if part.is_empty() { continue; }
        let words: Vec<&str> = part.split_whitespace().collect();
        if words.len() < 2 { continue; }
        // Strict count validation: a garbage or non-positive count must not be
        // silently coerced (unwrap_or(1) made "-3 aldi" run 1 store).
        let count: u64 = match words[0].parse() {
            Ok(c) if c >= 1 => c,
            Ok(_) => {
                eprintln!("\n❌ Count must be >= 1, got '{}' in spec: {}", words[0], spec);
                std::process::exit(1);
            }
            Err(_) => {
                eprintln!("\n❌ Could not parse count '{}' in spec: {}", words[0], spec);
                std::process::exit(1);
            }
        };
        let name = words[1..].join(" ");
        fleet.push((find_type(&name), count));
    }
    if fleet.is_empty() {
        eprintln!("\n❌ Could not parse fleet spec. Use: \"400 aldi + 5 edeka + 2 marktkauf\"");
        std::process::exit(1);
    }
    // DoS guard: cap the total fleet size (a typo like 999999999 would otherwise
    // start a multi-day simulation).
    let total: u64 = fleet.iter().map(|(_, c)| c).sum();
    if total > 100_000 {
        eprintln!("\n❌ Total fleet size {} exceeds the 100,000 store cap.", total);
        std::process::exit(1);
    }
    fleet
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_types_have_sane_physical_params() {
        assert!(STORE_TYPES.len() >= 20, "expected 20+ facility types, got {}", STORE_TYPES.len());
        let mut names = HashSet::new();
        for t in STORE_TYPES {
            assert!(names.insert(t.name), "duplicate type name: {}", t.name);
            assert!(t.t_min < t.t_setpoint && t.t_setpoint < t.t_max,
                "{}: need t_min < setpoint < t_max ({} < {} < {})", t.name, t.t_min, t.t_setpoint, t.t_max);
            assert!(t.cooling_power_kw > 0.0, "{}: cooling power must be positive", t.name);
            assert!(t.thermal_mass_kj_k > 0.0, "{}: thermal mass must be positive", t.name);
            assert!(t.max_shed_hours >= 0.0, "{}: max_shed_hours must be >= 0", t.name);
            let r_th = 1.0 / (t.u_value * t.surface_m2 + t.volume_m3 * t.air_change_rate_ach / 3600.0 * 1.2 * 1005.0).max(0.001);
            assert!(r_th * t.thermal_mass_kj_k * 1000.0 > 0.0, "{}: time constant must be positive", t.name);
            if t.has_defrost {
                assert!(t.defrost_interval_h > 0.0 && t.defrost_energy_kw > 0.0,
                    "{}: defrost interval/energy must be positive", t.name);
            }
            if t.has_tes {
                assert!(t.tes_capacity_kwh > 0.0, "{}: TES capacity must be positive", t.name);
            }
        }
    }

    #[test]
    fn find_type_matches_exact_case_insensitive_and_fuzzy() {
        assert_eq!(find_type("aldi").name, "aldi");
        assert_eq!(find_type("ALDI").name, "aldi");
        assert_eq!(find_type("freezer").name, "freezer");
        assert_eq!(find_type("kauf").name, "marktkauf", "substring match should resolve to marktkauf");
    }

    #[test]
    fn parse_fleet_counts_and_types() {
        let fleet = parse_fleet("400 aldi + 5 edeka + 2 marktkauf");
        assert_eq!(fleet.len(), 3);
        assert_eq!(fleet[0].0.name, "aldi");
        assert_eq!(fleet[0].1, 400);
        assert_eq!(fleet[1].0.name, "edeka");
        assert_eq!(fleet[1].1, 5);
        assert_eq!(fleet[2].0.name, "marktkauf");
        assert_eq!(fleet[2].1, 2);

        let single = parse_fleet("3 freezer");
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].0.name, "freezer");
        assert_eq!(single[0].1, 3);
    }
}
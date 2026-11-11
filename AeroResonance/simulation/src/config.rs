/// Configuration — all tunable simulation parameters defaulting to realistic physical values.
/// Deserialization fills any missing field with its [`Default`] value, so a
/// scenario that predates a new parameter still loads.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SimConfig {
    // ----- Time stepping -----
    pub dt: f64,
    pub total_time: f64,
    pub output_interval: f64,

    // ----- Environment -----
    pub ground_temp_c: f64,
    pub lapse_rate_k_per_km: f64,
    pub tower_height_m: f64,

    // ----- Global EM / Ionosphere -----
    pub fof2_mhz: f64,
    pub earth_ionosphere_potential_kv: f64,

    // ----- 4.1 Ionospheric Plasma Coupling -----
    pub pc_antenna_gain_dbi: f64,
    pub pc_q_factor: f64,
    pub pc_efficiency: f64,

    // ----- 4.2 NVIS -----
    pub nvis_antenna_count: u32,
    pub nvis_element_length_m: f64,

    // ----- 4.3 Maxwellian Field Relaxation -----
    pub mfr_fdtd_grid_cells: usize,
    pub mfr_fdtd_courant: f64,
    pub mfr_impedance_match_efficiency: f64,

    // ----- 4.4 Acoustic-Atmospheric Transduction -----
    pub piezo_tile_count: u32,
    pub piezo_tile_area_m2: f64,
    pub piezo_strain_coeff_pc_n: f64,
    pub piezo_efficiency: f64,
    pub infrasound_pressure_pa: f64,
    pub infrasound_freq_hz: f64,

    // ----- 4.5 Non-Linear Wave Modulation -----
    pub nlm_broadband_power_dbm: f64,
    pub nlm_schottky_efficiency: f64,
    pub nlm_antenna_aperture_m2: f64,

    // ----- 4.6 Cyclostationary Feature Filtering -----
    pub cs_extraction_ratio: f64,

    // ----- 2A/4.7 Plasma-Wake Energy Buffering (ENHANCED PMIC MODEL) -----
    pub buffer_capacitance_f: f64,
    pub buffer_charge_efficiency: f64,
    pub pmic_v_high: f64,
    pub pmic_v_low: f64,
    pub pmic_v_cold: f64,
    pub pmic_cold_start_efficiency: f64,
    pub pmic_main_efficiency: f64,
    pub cap_esr_ohm: f64,
    pub cap_leakage_current_a: f64,

    // ----- 2C/4.8 Resonant Substrate Adaptation (ADC-I + Tier -1) -----
    pub rs_clock_jitter_ns: f64,
    pub rs_rtos_overhead_pct: f64,
    pub rs_duty_cycle: f64,
    pub rs_sleep_power_w: f64,
    pub rs_active_power_w: f64,
    pub rs_v_cap_high: f64,
    pub rs_v_cap_low: f64,
    pub rs_fast_interval_s: f64,
    pub rs_sparse_interval_s: f64,
    pub rs_passive_fallback: bool,
    /// Option A: Deep-hibernate emergency threshold [V].
    /// When V_cap < this, ALL active control shuts off — only PMIC quiescent remains.
    pub rs_v_cap_deep_hibernate: f64,
    /// Quiescent power during deep hibernate [W] — analog PMIC only, no digital.
    pub rs_deep_hibernate_power_w: f64,

    // ----- 2D. Multi-Source MPPT -----
    pub mppt_teg_voc_ratio: f64,

    // ----- 4.9 Thermodynamic Capacity Gradient (+ Option B: ground thermal mass) -----
    pub teg_module_count: u32,
    pub teg_seebeck_uv_per_k: f64,
    pub teg_thermal_cond_w_per_k: f64,
    pub teg_zt: f64,
    pub teg_cold_side_c: f64,
    pub teg_efficiency: f64,
    /// Option B: Enable ground-inertia coupling for cold-side thermal mass [bool].
    /// If true, the cold-side temperature is coupled to deep ground (stable ~11°C)
    /// instead of free-air tower-top temperature.
    pub teg_ground_coupling: bool,
    /// Deep ground temperature [°C] — stable year-round at ~1-2m depth.
    pub teg_ground_temp_c: f64,
    /// Minimum ground-coupled TEG temperature difference [°C] assumed by the model.
    /// **UNVALIDATED:** this replaces a former hardcoded `dt.max(2.5)` "guarantee".
    /// It is an engineering assumption pending field measurement (plan M1.4);
    /// see docs/GATE1_ENERGY_DENSITY_REPORT.md.
    pub teg_min_dt_degc: f64,

    // ----- Option C: Solar Photovoltaic Microcell (10th harvester, enumerated as 4.11) -----
    pub pv_cell_count: u32,
    pub pv_cell_area_m2: f64,
    pub pv_efficiency: f64,
    /// Overcast irradiance [W/m²] — typical heavily overcast winter: 5–20 W/m².
    pub pv_irradiance_overcast_wpm2: f64,
    /// Clear sky irradiance [W/m²] — typical: 200–1000 W/m².
    pub pv_irradiance_clear_wpm2: f64,
    /// Probability of overcast [0.0–1.0] — for stochastic weather.
    pub pv_overcast_probability: f64,

    // ----- 4.10 Static Invariant Discharge (DUTY-CYCLED + deep-hibernate) -----
    pub safety_overvoltage_v: f64,
    pub safety_response_us: f64,
    pub safety_leakage_ma: f64,
    pub safety_duty_cycle: f64,
    pub safety_sleep_power_w: f64,

    // ----- 2E. Cross-Subsystem Energy Routing (Task Tiers + Tier -1) -----
    pub tier0_energy_j: f64,
    pub tier1_energy_j: f64,
    pub tier2_energy_j: f64,
    pub tier3_energy_j: f64,
    pub v_cap_tier0_min: f64,
    pub v_cap_tier1_min: f64,
    pub v_cap_tier2_min: f64,
    pub v_cap_tier3_min: f64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            dt: 0.001,
            total_time: 3600.0,
            output_interval: 1.0,

            ground_temp_c: 15.0,
            lapse_rate_k_per_km: 6.5,
            tower_height_m: 30.0,

            fof2_mhz: 6.0,
            earth_ionosphere_potential_kv: 280.0,

            pc_antenna_gain_dbi: 3.0,
            pc_q_factor: 50.0,
            pc_efficiency: 0.15,

            nvis_antenna_count: 4,
            nvis_element_length_m: 40.0,

            mfr_fdtd_grid_cells: 1000,
            mfr_fdtd_courant: 0.9,
            mfr_impedance_match_efficiency: 0.7,

            piezo_tile_count: 50,
            piezo_tile_area_m2: 0.01,
            piezo_strain_coeff_pc_n: 400.0,
            piezo_efficiency: 0.6,
            infrasound_pressure_pa: 5.0,
            infrasound_freq_hz: 0.2,

            nlm_broadband_power_dbm: -40.0,
            nlm_schottky_efficiency: 0.3,
            nlm_antenna_aperture_m2: 0.5,

            cs_extraction_ratio: 0.4,

            buffer_capacitance_f: 0.47,
            buffer_charge_efficiency: 0.85,
            pmic_v_high: 3.3,
            pmic_v_low: 2.8,
            pmic_v_cold: 1.8,
            pmic_cold_start_efficiency: 0.35,
            pmic_main_efficiency: 0.87,
            cap_esr_ohm: 10.0,
            cap_leakage_current_a: 2e-6,

            rs_clock_jitter_ns: 0.5,
            rs_rtos_overhead_pct: 2.0,
            rs_duty_cycle: 0.005,
            rs_sleep_power_w: 3e-6,
            rs_active_power_w: 500e-6,
            rs_v_cap_high: 3.2,
            rs_v_cap_low: 2.9,
            rs_fast_interval_s: 1.0,
            rs_sparse_interval_s: 60.0,
            rs_passive_fallback: true,
            rs_v_cap_deep_hibernate: 0.5,
            rs_deep_hibernate_power_w: 1.2e-6,

            mppt_teg_voc_ratio: 0.5,

            teg_module_count: 50,
            teg_seebeck_uv_per_k: 200.0,
            teg_thermal_cond_w_per_k: 0.5,
            teg_zt: 1.0,
            teg_cold_side_c: -5.0,
            teg_efficiency: 0.05,
            teg_ground_coupling: true,
            teg_ground_temp_c: 11.0,
            teg_min_dt_degc: 2.5, // documented UNVALIDATED assumption (see gate report)

            pv_cell_count: 1,
            pv_cell_area_m2: 0.0004, // 20×20 mm micro cell
            pv_efficiency: 0.18,
            pv_irradiance_overcast_wpm2: 10.0,
            pv_irradiance_clear_wpm2: 500.0,
            pv_overcast_probability: 0.7,

            safety_overvoltage_v: 60.0,
            safety_response_us: 0.5,
            safety_leakage_ma: 0.1,
            safety_duty_cycle: 0.01,
            safety_sleep_power_w: 1e-6,

            tier0_energy_j: 1e-6,
            tier1_energy_j: 10e-6,
            tier2_energy_j: 500e-6,
            tier3_energy_j: 5e-3,
            v_cap_tier0_min: 2.5,
            v_cap_tier1_min: 2.8,
            v_cap_tier2_min: 3.1,
            v_cap_tier3_min: 3.25,
        }
    }
}

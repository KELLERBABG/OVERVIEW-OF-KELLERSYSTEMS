//! Simulation engine — runs the time-stepping loop and collects results.
//! INTEGRATION: Uses eh_sim_core::entropy for TRNG and eh_sim_core::crypto for Ghost-Net.

use crate::config::SimConfig;
use crate::subsystems::System;
use esim_core::crypto::{encrypt_packet, packet_energy_cost};
use esim_core::entropy::EntropyPipeline;
use esim_core::scheduler::{TaskSchedulerConfig, TaskTier};
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SimRecord {
    pub time_s: f64,
    pub sub_name: String,
    pub power_w: f64,
    pub v_cap: f64,
    pub tier: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EnergyRecord {
    pub sub_name: String,
    pub total_energy_j: f64,
    pub avg_power_w: f64,
    pub peak_power_w: f64,
    pub min_power_w: f64,
}

pub fn run_simulation(cfg: &SimConfig, seed: Option<u64>) -> (Vec<SimRecord>, Vec<EnergyRecord>) {
    let mut system = System::new(cfg, seed);
    let mut records: Vec<SimRecord> = Vec::new();
    let n_subs = system.subsystems.len();

    // INTEGRATION: Entropy pipeline
    let mut entropy = EntropyPipeline::new();
    let mut total_entropy_bytes: u64 = 0;
    let mut ghost_net_packets_sent: u64 = 0;
    let mut ghost_net_energy_j: f64 = 0.0;

    let radio_tx_power_w: f64 = 0.01;
    let radio_tx_time_per_byte_s: f64 = 8.0 / 10000.0;

    // Derive the Ghost-Net telemetry key from the entropy pipeline rather than
    // a hardcoded constant. In simulation this is deterministic from the raw
    // ADC noise stream; a production device would provision a key out-of-band.
    let ghost_key_seed = {
        let mut k = [0u8; 32];
        k.copy_from_slice(&entropy.feed_adc_samples(&[0xBEEF, 0xCAFE, 0x1200, 0x8899]));
        k
    };
    let ghost_key: [u8; 32] = ghost_key_seed;

    let mut sub_names: Vec<String> = Vec::with_capacity(n_subs);
    let mut sub_peak: Vec<f64> = vec![f64::NEG_INFINITY; n_subs];
    let mut sub_min: Vec<f64> = vec![f64::INFINITY; n_subs];
    let mut sub_samples: Vec<u64> = vec![0; n_subs];
    let mut sub_power_sum: Vec<f64> = vec![0.0; n_subs];

    let dt = cfg.dt;
    let total_steps = (cfg.total_time / dt) as u64;
    let output_steps = (cfg.output_interval / dt).round() as u64;
    let ghost_net_interval_steps = (60.0 / dt).round() as u64;
    let start = Instant::now();

    let sched_cfg = TaskSchedulerConfig {
        tier0_energy_j: cfg.tier0_energy_j,
        tier1_energy_j: cfg.tier1_energy_j,
        tier2_energy_j: cfg.tier2_energy_j,
        tier3_energy_j: cfg.tier3_energy_j,
        v_cap_tier0_min: cfg.v_cap_tier0_min,
        v_cap_tier1_min: cfg.v_cap_tier1_min,
        v_cap_tier2_min: cfg.v_cap_tier2_min,
        v_cap_tier3_min: cfg.v_cap_tier3_min,
    };

    eprintln!(
        "Starting simulation: {} steps (Δt = {} s, total = {} s simulated)",
        total_steps, dt, cfg.total_time
    );
    eprintln!(
        "PMIC: V_high={}V, V_low={}V, C={}F, ESR={}Ω",
        cfg.pmic_v_high, cfg.pmic_v_low, cfg.buffer_capacitance_f, cfg.cap_esr_ohm
    );
    eprintln!(
        "ADC-I: fast={}s, sparse={}s, duty={}%, passive_fallback={}",
        cfg.rs_fast_interval_s,
        cfg.rs_sparse_interval_s,
        cfg.rs_duty_cycle * 100.0,
        cfg.rs_passive_fallback
    );
    eprintln!(
        "Tier min voltages: T0={}V, T1={}V, T2={}V, T3={}V\n",
        cfg.v_cap_tier0_min, cfg.v_cap_tier1_min, cfg.v_cap_tier2_min, cfg.v_cap_tier3_min
    );

    for step in 0..total_steps {
        let t = step as f64 * dt;

        let (results, _th, _tc) = system.step(t, cfg);

        // INTEGRATION: Feed entropy pipeline with ADC noise
        let adc_sample = (system.env.schumann_e_field_vpm * 1e8).abs() as u16 % 4096;
        let adc_noise = (system.env.lightning_proximity * 4095.0) as u16;
        let extracted = entropy.feed_adc_samples(&[adc_sample, adc_noise]);
        total_entropy_bytes += extracted.len() as u64;

        // INTEGRATION: Ghost-Net at Tier 2+
        if step % ghost_net_interval_steps == 0 && step > 0 {
            let ct = sched_cfg.highest_tier(system.v_cap);
            if ct as i32 >= TaskTier::Tier2 as i32 {
                let telemetry = format!(
                    "EHAR|t={:.1}|v={:.3}|harv={:.3e}|ent={}",
                    t, system.v_cap, _th, total_entropy_bytes
                );
                let _packet =
                    encrypt_packet(&ghost_key, telemetry.as_bytes(), ghost_net_packets_sent);
                let cost =
                    packet_energy_cost(telemetry.len(), radio_tx_power_w, radio_tx_time_per_byte_s);
                ghost_net_energy_j += cost;
                ghost_net_packets_sent += 1;
            }
        }

        if step % output_steps == 0 || step == total_steps - 1 {
            let tier_str = system.active_tier.name();
            for (i, (name, power)) in results.iter().enumerate() {
                records.push(SimRecord {
                    time_s: t,
                    sub_name: name.clone(),
                    power_w: *power,
                    v_cap: system.v_cap,
                    tier: tier_str.to_string(),
                });
                if i < n_subs {
                    if sub_names.len() <= i {
                        sub_names.push(name.clone());
                    }
                    if *power > sub_peak[i] {
                        sub_peak[i] = *power;
                    }
                    if *power < sub_min[i] {
                        sub_min[i] = *power;
                    }
                    sub_power_sum[i] += *power;
                    sub_samples[i] += 1;
                }
            }
        }

        if total_steps >= 100 && step % (total_steps / 10) == 0 && step > 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let pct = (step as f64 / total_steps as f64) * 100.0;
            let eta = if pct > 0.0 {
                elapsed / pct * (100.0 - pct)
            } else {
                0.0
            };
            eprintln!(
                "  {:.0}% — V_cap={:.3}V tier={} — elap {:.1}s ETA {:.1}s",
                pct,
                system.v_cap,
                system.active_tier.name(),
                elapsed,
                eta
            );
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    eprintln!("\nSimulation finished in {:.2} seconds.", elapsed);
    eprintln!(
        "Final V_cap: {:.3}V (tier {})",
        system.v_cap,
        system.active_tier.name()
    );

    let entropy_bps = entropy
        .sponge
        .entropy_rate_bps(total_entropy_bytes, cfg.total_time);
    eprintln!(
        "TRNG: {} bytes extracted ({:.1} bits/s, {:.1}% efficiency)",
        total_entropy_bytes,
        entropy_bps,
        entropy.efficiency() * 100.0
    );
    eprintln!(
        "Ghost-Net: {} packets sent ({} J total, {:.3} µJ avg)",
        ghost_net_packets_sent,
        ghost_net_energy_j,
        if ghost_net_packets_sent > 0 {
            ghost_net_energy_j / ghost_net_packets_sent as f64 * 1e6
        } else {
            0.0
        }
    );

    let mut summaries: Vec<EnergyRecord> = Vec::with_capacity(n_subs);
    for i in 0..n_subs {
        let name = if i < sub_names.len() {
            &sub_names[i]
        } else {
            "Unknown"
        };
        let samples = sub_samples[i].max(1);
        let avg = sub_power_sum[i] / samples as f64;
        summaries.push(EnergyRecord {
            sub_name: name.to_string(),
            total_energy_j: system.subsystems[i].total_energy_j(),
            avg_power_w: avg,
            peak_power_w: sub_peak[i],
            min_power_w: sub_min[i],
        });
    }
    (records, summaries)
}

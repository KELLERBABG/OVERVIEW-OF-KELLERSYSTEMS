//! Simulation engine that drives the system through all of 2025 using
//! real weather models, price data, and grid frequency — with accelerated time.
//!
//! By default, 1 real second = 1 simulated hour (8760× acceleration).
//! That means the entire year of 2025 runs in ~2.5 hours.
//! Set speed_multiplier in config to adjust.

use crate::financial::FinancialState;
use crate::simulation_data::SimulationDataset;

/// Simulation engine running through the entire 2025 dataset.
pub struct SimulationEngine {
    dataset: SimulationDataset,
    /// Current simulated hour (0..8759), may be fractional for sub-hour interpolation.
    simulated_hour: f64,
    /// Real-time start of the simulation.
    start_real_time: std::time::Instant,
    /// How many simulated hours per real second (default: 3600 = 1 hour/sec).
    speed_hours_per_sec: f64,
    /// Have we looped back to Jan 1?
    loops_completed: u32,
}

impl SimulationEngine {
    /// Create a new simulation engine. The dataset is generated asynchronously
    /// (trying real API data first, falling back to synthetic).
    pub async fn new() -> Self {
        let dataset = SimulationDataset::generate().await;

        log::info!("📊 2025 dataset generated: {} data points", dataset.data.len());

        // Verify: print some stats
        let temps: Vec<f64> = dataset.data.iter().map(|d| d.ambient_temp_c).collect();
        let prices: Vec<f64> = dataset.data.iter().map(|d| d.price_eur_per_mwh).collect();
        let min_temp = temps.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_temp = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        log::info!("📊 Temperature range: {:.1}°C to {:.1}°C", min_temp, max_temp);
        log::info!("📊 Price range: {:.1} to {:.1} €/MWh", min_price, max_price);

        Self {
            dataset,
            simulated_hour: 0.0,
            start_real_time: std::time::Instant::now(),
            speed_hours_per_sec: 1.0, // 1 real second = 1 simulated hour
            loops_completed: 0,
        }
    }

    /// Advance the simulation by one step.
    /// `dt_real_secs`: real time elapsed since last step.
    /// Returns the current data point (interpolated) — does NOT lock control engine.
    pub fn step(&mut self, dt_real_secs: f64) -> SimState {
        // Advance simulated time
        self.simulated_hour += dt_real_secs * self.speed_hours_per_sec;

        // Handle looping past 8760 hours
        if self.simulated_hour >= 8760.0 {
            self.simulated_hour -= 8760.0;
            self.loops_completed += 1;
        }

        // Get the current interpolated data point
        self.current_data_point()
    }


    /// Interpolate between hourly datapoints for smooth transitions.
    fn current_data_point(&self) -> SimState {
        let hour_idx = self.simulated_hour.floor() as usize;
        let fraction = self.simulated_hour.fract();

        let len = self.dataset.data.len();
        let curr = &self.dataset.data[hour_idx % len];
        let next = &self.dataset.data[(hour_idx + 1) % len];

        // Linear interpolation
        let ambient = curr.ambient_temp_c + (next.ambient_temp_c - curr.ambient_temp_c) * fraction;
        let price = curr.price_eur_per_mwh + (next.price_eur_per_mwh - curr.price_eur_per_mwh) * fraction;
        let freq = curr.grid_frequency_hz + (next.grid_frequency_hz - curr.grid_frequency_hz) * fraction;
        let ghi = curr.solar_ghi_wm2 + (next.solar_ghi_wm2 - curr.solar_ghi_wm2) * fraction;

        let real_elapsed = self.start_real_time.elapsed();

        SimState {
            ambient_temp_c: ambient,
            price_eur_per_mwh: price,
            grid_frequency_hz: freq,
            solar_ghi_wm2: ghi,
            is_night: curr.is_night,
            season: curr.season,
            timestamp: curr.timestamp,
            simulated_hour: self.simulated_hour,
            total_hours_simulated: (self.loops_completed as f64 * 8760.0) + self.simulated_hour,
            real_elapsed,
            simulation_speed: self.speed_hours_per_sec,
        }
    }

    /// Get the current simulation state (read-only).
    pub fn current_state(&self) -> SimState {
        self.current_data_point()
    }

    /// Get a lookahead window of the next N hours of price + ambient temp data.
    /// Returns Vec<(price_eur_per_mwh, ambient_temp_c)> of length `hours`.
    /// Wraps around the 8760-hour dataset if needed.
    pub fn get_mpc_lookahead(&self, hours: usize) -> Vec<(f64, f64)> {
        let current_hour = self.simulated_hour.floor() as usize;
        let len = self.dataset.data.len();
        let mut lookahead = Vec::with_capacity(hours);

        for offset in 1..=hours {
            let idx = (current_hour + offset) % len;
            let dp = &self.dataset.data[idx];
            lookahead.push((dp.price_eur_per_mwh, dp.ambient_temp_c));
        }

        lookahead
    }
}

/// Current state of the simulation.
#[derive(Debug, Clone)]
pub struct SimState {
    pub ambient_temp_c: f64,
    pub price_eur_per_mwh: f64,
    pub grid_frequency_hz: f64,
    pub solar_ghi_wm2: f64,
    pub is_night: bool,
    pub season: u8,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub simulated_hour: f64,
    pub total_hours_simulated: f64,
    pub real_elapsed: std::time::Duration,
    pub simulation_speed: f64,
}

impl SimState {
    pub fn season_name(&self) -> &'static str {
        match self.season {
            0 => "Spring",
            1 => "Summer",
            2 => "Autumn",
            _ => "Winter",
        }
    }

    pub fn season_name_de(&self) -> &'static str {
        match self.season {
            0 => "Frühling",
            1 => "Sommer",
            2 => "Herbst",
            _ => "Winter",
        }
    }

    pub fn speed_label(&self) -> String {
        format!("{:.0}× realtime (1s = {:.1}h)", self.simulation_speed, self.simulation_speed)
    }

    pub fn speed_label_de(&self) -> String {
        format!("{:.0}× Echtzeit (1s = {:.1}h)", self.simulation_speed, self.simulation_speed)
    }
}

/// Print the enhanced simulation dashboard with financial data (in German).
pub fn print_dashboard(
    decision: &crate::control::ControlDecision,
    reading: &crate::sensor::SensorReading,
    state: &SimState,
    finances: &FinancialState,
    api_bind: &str,
) {
    // Clear screen
    print!("\x1B[2J\x1B[1;1H");

    let border = "═".repeat(72);

    println!("╔{}╗", border);
    println!("║  🌡️  AMBIENT THERMAL ARBITRAGE — NRW 2025 SIMULATION               ║");
    println!("╠{}╣", border);

    // Timestamp & simulation progress
    let pct = ((state.simulated_hour / 8760.0) * 100.0) as u8;
    let elapsed_real = state.real_elapsed;
    let hrs = elapsed_real.as_secs() / 3600;
    let mins = (elapsed_real.as_secs() % 3600) / 60;
    let secs = elapsed_real.as_secs() % 60;

    let sim_date = state.timestamp.format("%Y-%m-%d %H:00");
    println!("║  📅 {}  Jahreszeit: {:<8}  {:>3}% Jahr              ║",
        sim_date, state.season_name_de(), pct);
    println!("║  ⏱️  Echtzeit: {:02}h {:02}m {:02}s  |  Sim: {:>4.0}h  |  {:<26}  ║",
        hrs, mins, secs, state.total_hours_simulated, state.speed_label_de());
    println!("╠{}╣", border);

    // Weather + Storage
    let temp = state.ambient_temp_c;
    let tw = 26; let tmn = -10.0; let tmx = 40.0;
    let tf = ((temp - tmn) / (tmx - tmn) * tw as f64).clamp(0.0, tw as f64) as usize;
    let tb: String = (0..tw).map(|i| if i < tf { '█' } else { '░' }).collect();
    println!("║  🌤️  Aussen: {:>5.1}°C  [{}]  Solar:{:>4.0}W/m²   ║",
        temp, tb, state.solar_ghi_wm2);

    let s_temp = reading.internal_temp_c;
    let t_min = -24.0; let t_max = -18.0;
    let sf = ((s_temp - t_min) / (t_max - t_min) * 20.0).clamp(0.0, 20.0) as usize;
    let sb: String = (0..20).map(|i| if i < sf { '█' } else { '░' }).collect();
    println!("║  🧊  Lager:  {:>5.1}°C  [{}]  Soll:{:>4.0}°C  TTF:{:>6.0}s  ║",
        s_temp, sb, decision.setpoint_c, decision.ttf_sec);
    println!("╠{}╣", border);

    // Compressor
    let comp = if decision.compressor_on { "🟢 EIN " } else { "🔴 AUS " };
    let mode_str = match decision.active_controller {
        crate::control::ControllerMode::Emergency => "Notfall",
        crate::control::ControllerMode::ThermalGuard => "Temperaturschutz",
        crate::control::ControllerMode::GridBalancing => "Netzstabilisierung",
        crate::control::ControllerMode::Arbitrage => "Arbitrage (Spot)",
        crate::control::ControllerMode::MpcPreCool => "MPC Vor-Kühlung",
        crate::control::ControllerMode::OilReturnPulse => "Ölrückführung",
        crate::control::ControllerMode::Idle => "Standby / Regel",
    };
    println!("║  🔧 Verdichter: {}  |  {:>5.0}W  |  Modus: {:<20}  ║",
        comp, reading.power_consumption_w, mode_str);
    println!("╠{}╣", border);

    // Grid & Price
    let freq = reading.grid_frequency_hz;
    let ff = ((freq - 49.5) / 1.0 * 20.0).clamp(0.0, 20.0) as usize;
    let fb: String = (0..20).map(|i| if i < ff { '█' } else { '░' }).collect();
    println!("║  ⚡ Netz: {:.3} Hz  [{}]  💶 Spot: {:>5.1} €/MWh    ║",
        freq, fb, decision.virtual_price_eur_per_mwh);
    println!("║  📈 Therm. Integral: {:>6.0} / {:.0} °C·s                                ║",
        decision.thermal_integral, 3600.0);
    println!("╠{}╣", border);

    // ─── FINANCIAL SECTION ─────────────────────────────────
    let savings_pct = finances.savings_percentage();
    let proj = finances.projected_yearly_savings();
    println!("║  💰 FINANZIELLE ÜBERSICHT — Kumuliert seit Start:                    ║");
    println!("║  ┌──────────────────────┬──────────────┬────────────────────┐        ║");
    println!("║  │                      │ Baseline     │ Mit Arbitrage      │        ║");
    println!("║  ├──────────────────────┼──────────────┼────────────────────┤        ║");
    println!("║  │ Energieverbrauch     │ {:>10.1} MWh │ {:>16.1} MWh │        ║",
        finances.baseline_energy_mwh, finances.actual_energy_mwh);
    println!("║  │ Stromkosten          │ {:>10.1} €   │ {:>16.1} €   │        ║",
        finances.baseline_cost_eur, finances.actual_cost_eur);
    println!("║  ├──────────────────────┴──────────────┴────────────────────┤        ║");
    println!("║  │ Ersparnis:            {:>+10.1} €  ({:>5.1}%)           │        ║",
        finances.money_saved_eur, savings_pct);
    println!("║  │ Hochrechnung / Jahr:  {:>+10.1} €                         │        ║",
        proj);
    println!("║  │ Ø Strompreis bezahlt: {:>10.1} €/MWh                     │        ║",
        finances.avg_price_paid);
    println!("║  │ Verdichter-Laufzeit:  {:>10.1} h / {} h erfasst     │        ║",
        finances.compressor_hours, finances.hours_tracked);
    println!("║  └──────────────────────────────────────────────────────────┘        ║");
    println!("╠{}╣", border);

    // Action hint
    let action = if decision.active_controller == crate::control::ControllerMode::Emergency {
        "🚨 NOTFALL — Manueller / Kritischer Sicherheits-Override!"
    } else if decision.active_controller == crate::control::ControllerMode::ThermalGuard {
        "🛡️  TEMPERATURSCHUTZ — Sicherheits-Vorrang aktiv (Sollwert)"
    } else if decision.active_controller == crate::control::ControllerMode::GridBalancing {
        "⚡ NETZSTABILISIERUNG — Primärregelleistung (Frequenzgang)"
    } else if decision.active_controller == crate::control::ControllerMode::MpcPreCool {
        "🔮 MPC VOR-KÜHLUNG — Prädiktive Kälteaufladung vor Preisspitze"
    } else if decision.active_controller == crate::control::ControllerMode::OilReturnPulse {
        "🔄 ÖLRÜCKFÜHRUNG — Kurzzeitiger Hochdrehzahl-Spülzyklus"
    } else if decision.active_controller == crate::control::ControllerMode::Arbitrage {
        if decision.compressor_on {
            "💰 ARBITRAGE — Vor-Kühlung (Günstiger Strom / Netzüberschuss)"
        } else {
            "💰 ARBITRAGE — Lastabwurf (Teure Spitzenpreis-Phase)"
        }
    } else {
        if decision.compressor_on {
            "❄️  REGELBETRIEB — Normale Kühlung im Sollband"
        } else {
            "⏸️  STANDBY — Temperatur optimal im Sollbereich"
        }
    };
    println!("║  {:<67}║", action);
    println!("╚{}╝", border);

    // Legend
    println!();
    let port = api_bind.split(':').nth(1).unwrap_or("8085");
    println!("  🌐 Web-Dashboard: http://localhost:{}/ (im Browser öffnen)", port);
    println!("  🔗 API Status:    curl http://localhost:{}/status", port);
    println!("  ⚙️  Tempo: {:.0}×  |  1 Jahr in ca. {:.0} Min. Echtzeit",
        state.simulation_speed, 8760.0 / state.simulation_speed / 60.0);
    println!("  💡 Gebäude-Parameter in crates/gateway/config.toml anpassen");
}

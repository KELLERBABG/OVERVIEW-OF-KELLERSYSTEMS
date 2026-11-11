use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use thermal_arbitrage_mvp::{
    alerts::{self, factory},
    api::{self, AppState},
    config::Config,
    control::ControlEngine,
    financial::FinancialState,
    live_data::{self, LiveStateHandle},
    logging::CsvLogger,
    modbus_client,
    ota,
    security::AuthState,
    sensor::{SensorHardware, SimulatedSensors},
    simulator::{self, SimulationEngine},
    watchdog::Watchdog,
};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let config = Config::load("config.toml");

    // ─── Live-data validation mode ─────────────────────────
    // One-shot real-world self-test: probes every price/weather/frequency
    // source through the SAME fetch+parse code the gateway uses, prints a
    // JSON report, and exits. With an ENTSO-E key this validates the official
    // price path end-to-end:
    //   ENTSOE_API_KEY=... cargo run --release -- --validate-live
    if std::env::args().any(|a| a == "--validate-live") {
        let report = live_data::validate_live_sources(&config.live_data).await;
        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_else(|_| "report serialization failed".into()));
        let price_ok = report.checks.iter().any(|c| c.role == "price" && c.ok);
        let weather_ok = report.checks.iter().any(|c| c.role == "weather" && c.ok);
        std::process::exit(if price_ok && weather_ok { 0 } else { 1 });
    }

    let building = &config.building;

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║   Ambient Thermal Arbitrage — Production Pilot v{}                   ║", ota::CURRENT_VERSION);
    println!("║   NRW 2025 · Live-data · Modbus PLC · Alerts · OTA · Monitoring           ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝");
    println!();

    // ─── Building Configuration ────────────────────────────
    let r_th_building = building.compute_r_th();
    let c_th_building = building.compute_c_th();
    let tau_building_hours = building.time_constant_sec() / 3600.0;
    let installed_power_w = building.installed_cooling_power_w;
    let effective_cooling_w = building.effective_cooling_power_w();

    let mut effective_config = config.clone();
    effective_config.thermal.r_th = r_th_building;
    effective_config.thermal.c_th = c_th_building;
    effective_config.sensor.cooling_power_w = installed_power_w;
    let heat_leak_w = 25.0 / r_th_building;
    effective_config.sensor.heat_leak_w = heat_leak_w;

    println!("🏢 Building: {}", building.description());
    println!("   R_th = {:.4} K/W    C_th = {:.0} J/K    τ = {:.1} hours", r_th_building, c_th_building, tau_building_hours);
    println!("   Electrical: {:.1} kW compressors  |  Thermal: {:.1} kW (COP {})", installed_power_w / 1000.0, effective_cooling_w / 1000.0, building.cop);
    println!();

    let unit_id = effective_config.system.unit_id.clone();

    // ─── Fail-closed authentication ────────────────────────
    // Refuse to run with auth enabled but an empty or well-known default secret:
    // the token derivation is public, so "change-me-in-production" (or no
    // secret) would let anyone forge bearer tokens for every mutating route.
    if effective_config.security.auth_enabled {
        let secret = &effective_config.security.api_secret;
        if secret.is_empty() || secret == "change-me-in-production" {
            eprintln!("[FATAL] security.auth_enabled=true but the API secret is empty or the well-known default.");
            eprintln!("        Set ATA_SECRET to a random value (e.g. `openssl rand -base64 32`) and restart.");
            std::process::exit(1);
        }
    }

    // ─── Alert Dispatcher ──────────────────────────────────
    let (alert_sender, _alert_task) = alerts::spawn_alert_dispatcher(unit_id.clone(), effective_config.alerts.clone());
    let _ = alert_sender.send(factory::startup(&unit_id));
    println!("🚨 Alert system: {} (webhook: {})", if effective_config.alerts.enabled { "enabled" } else { "log-only" }, if effective_config.alerts.webhook_url.is_some() { "configured" } else { "none" });

    // ─── Live Data Pipeline ────────────────────────────────
    let live_state = LiveStateHandle::new();
    let live_task = live_data::run_live_data_loop(effective_config.live_data.clone(), live_state.clone()).await;
    if effective_config.live_data.enabled {
        println!("📡 Live data: enabled (price {}s, weather {}s)", effective_config.live_data.price_refresh_sec, effective_config.live_data.weather_refresh_sec);
    }

    // ─── Initialize Thermal Model ──────────────────────────
    let mut sensors = SimulatedSensors::new(
        effective_config.sensor.initial_temp_c,
        effective_config.sensor.ambient_temp_c,
        effective_config.thermal.r_th,
        effective_config.thermal.c_th,
        installed_power_w,
        effective_cooling_w,
        effective_config.sensor.heat_leak_w,
        effective_config.thermal.rls_ambient_variance_max_k,
    );

    // ─── Control Engine ────────────────────────────────────
    let control_engine = ControlEngine::new(effective_config.clone());
    println!("🧠 Control engine: 6-mode priority cascade loaded");

    // ─── Hardware Watchdog ─────────────────────────────────
    let mut watchdog = Watchdog::new(effective_config.watchdog.clone());

    // ─── Financial Tracking ────────────────────────────────
    let mut finances = FinancialState::new();

    // ─── CSV Logger ────────────────────────────────────────
    let csv_logger = match CsvLogger::new(&effective_config.logging.csv_path, effective_config.logging.interval_sec) {
        Ok(logger) => {
            println!("📝 CSV logging to {}", effective_config.logging.csv_path);
            Some(logger)
        }
        Err(e) => {
            println!("⚠️  CSV log disabled: {}", e);
            None
        }
    };

    // ─── Shared API State ──────────────────────────────────
    let app_state = Arc::new(AppState {
        unit_id: unit_id.clone(),
        control_engine: Mutex::new(control_engine),
        last_reading: Mutex::new(None),
        last_decision: Mutex::new(None),
        config: RwLock::new(effective_config.clone()),
        live_data: live_state.clone(),
        alert_sender: Some(alert_sender.clone()),
        auth: AuthState {
            secret: effective_config.security.api_secret.clone(),
            auth_enabled: effective_config.security.auth_enabled,
        },
    });

    // ─── HTTP API Server (TLS optional) ────────────────────
    {
        let api_state = app_state.clone();
        let api_cfg = effective_config.api.clone();
        tokio::spawn(async move {
            if let Err(e) = api::serve(api_state, &api_cfg).await {
                eprintln!("⚠️  API Server Fehler: {}", e);
                log::error!("🌐 API server error: {}", e);
            }
        });
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    let proto = if effective_config.api.tls_cert_path.is_some() { "https" } else { "http" };
    println!("🌐 Web-Dashboard: {}://{}/", proto, effective_config.api.bind_address);
    println!("🌐 API: {}://{} (status, health, metrics, config, price, alerts/test, ota/check)", proto, effective_config.api.bind_address);
    println!();

    // ─── Real-Hardware Sensor Mode ────────────────────────
    // When `sensor.simulate = false`, the gateway closes the loop on real PLC
    // sensor readings (published by the Modbus loop) instead of the simulated
    // thermal model. If the PLC data goes stale it falls back to the model and
    // raises a throttled sensor_loss alert.
    let hardware_sensor_mode = !effective_config.sensor.simulate;
    // (internal_temp, ambient_temp, grid_frequency_opt, received_at)
    type HwReading = Arc<Mutex<Option<(f64, f64, Option<f64>, std::time::Instant)>>>;
    let hw_reading: HwReading = Arc::new(Mutex::new(None));
    if hardware_sensor_mode && !effective_config.modbus.enabled {
        println!("⚠️  sensor.simulate=false but modbus is disabled — no real sensor source. Enable [modbus] in config.toml.");
    }

    // ─── Modbus PLC Loop ───────────────────────────────────
    if effective_config.modbus.enabled {
        let modbus_cfg = effective_config.modbus.clone();
        let app_state_for_modbus = app_state.clone();
        let state_for_modbus = Arc::new(move || {
            let reading = app_state_for_modbus.last_reading.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let decision = app_state_for_modbus.last_decision.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let internal_c = reading.as_ref().map(|r| r.internal_temp_c).unwrap_or(-20.0);
            let ambient_c = reading.as_ref().map(|r| r.ambient_temp_c).unwrap_or(20.0);
            let comp_on = decision.as_ref().map(|d| d.compressor_on).unwrap_or(false);
            let comp_speed = decision.as_ref().and_then(|d| d.compressor_speed);
            let alarm = decision.as_ref().map(|d| d.active_controller == thermal_arbitrage_mvp::control::ControllerMode::Emergency).unwrap_or(false);
            (internal_c, ambient_c, comp_on, comp_speed, alarm)
        });
        // Publish real PLC readings for the control loop (hardware sensor mode).
        let hw_slot = hw_reading.clone();
        let on_reading: modbus_client::ModbusReadingCallback = Some(Arc::new(move |internal, ambient, frequency| {
            *hw_slot.lock().unwrap_or_else(|e| e.into_inner()) = Some((internal, ambient, frequency, std::time::Instant::now()));
        }));
        tokio::spawn(modbus_client::run_modbus_loop(modbus_cfg, state_for_modbus, on_reading));
        println!("🔌 Modbus TCP: enabled → {}:{} (sensor mode: {})", effective_config.modbus.host, effective_config.modbus.port, if hardware_sensor_mode { "REAL PLC" } else { "simulated" });
    }

    // ─── OTA Loop ──────────────────────────────────────────
    if effective_config.ota.enabled {
        let ota_task = ota::run_ota_loop(effective_config.ota.clone());
        std::mem::drop(ota_task); // fire-and-forget: task runs until process exit
        println!("📦 OTA: enabled (check every {}s)", effective_config.ota.check_interval_sec);
    }

    // ─── Simulation Engine ─────────────────────────────────
    let mut sim_engine = SimulationEngine::new().await;
    println!();

    // ─── Graceful Shutdown Flag ────────────────────────────
    // Ctrl+C is watched by a detached task so that the control loop NEVER
    // blocks on the signal future. In interactive terminals the flag is set
    // on Ctrl+C; in background/service/Docker contexts (no console attached)
    // the signal simply never fires and the loop keeps running as a daemon.
    // (A naive `select! { ctrl_c(), sleep(3600s) }` in the loop would stall
    // the entire control loop for up to an hour per iteration when no console
    // is attached, because the ctrl_c future can never resolve.)
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
    }

    // ─── Main Control Loop ─────────────────────────────────
    let loop_start = std::time::Instant::now();
    let frame_interval = Duration::from_secs_f64(effective_config.system.loop_interval_sec.max(0.1));
    let mut last_frame = std::time::Instant::now();
    let mut last_alert_time: std::time::Instant = std::time::Instant::now();
    let mut last_sensor_loss_alert: std::time::Instant = std::time::Instant::now() - Duration::from_secs(301);
    let mut last_emergency = false;
    // Hardware mode: the compressor state is our own last command (the plant's
    // state is not read back except via coils), so carry it across frames.
    let mut last_comp_on = false;
    // Hardware mode: last known-good PLC reading. On PLC loss we FREEZE this
    // value (never fall back to the simulated model — winter ambient + latched
    // cooling drove it to -69°C in testing, corrupting decisions).
    let mut last_good_hw: Option<thermal_arbitrage_mvp::sensor::SensorReading> = None;

    loop {
        let frame_start = std::time::Instant::now();
        let dt_real = last_frame.elapsed().as_secs_f64();
        last_frame = std::time::Instant::now();
        let live_mode = effective_config.live_data.enabled;
        let dt_seconds = if live_mode { dt_real } else { dt_real * 3600.0 };
        let dt_hours = if live_mode { dt_real / 3600.0 } else { dt_real };

        // 1. Advance simulation → get weather, price, frequency
        let sim_state = sim_engine.step(dt_real);
        let current_sim_time_sec = sim_state.total_hours_simulated * 3600.0;

        // Real-time control base: in LIVE mode the governor locks, thermal
        // integral and emergency bookkeeping must run on WALL time.
        let control_time_sec = if live_mode {
            frame_start.duration_since(loop_start).as_secs_f64()
        } else {
            current_sim_time_sec
        };
        let fin_hours = dt_hours;

        // 2. In live mode, override simulated price/weather/frequency with real data
        let live = live_state.read().await;
        let price = if effective_config.live_data.enabled { live.price_eur_per_mwh } else { sim_state.price_eur_per_mwh };
        let ambient = if effective_config.live_data.enabled { live.ambient_temp_c } else { sim_state.ambient_temp_c };
        // Live frequency (URL source) is used only while fresh (< 5 min).
        let live_freq = if effective_config.live_data.enabled {
            live.frequency_updated_at
                .filter(|t| (chrono::Utc::now() - *t).num_seconds() < 300)
                .map(|_| live.grid_frequency_hz)
        } else {
            None
        };
        // Price staleness: while stale, suppress arbitrage + MPC (thermostat
        // and safety modes keep running) and alert, like the PLC-loss freeze.
        let price_stale = effective_config.live_data.enabled
            && !live.is_price_fresh(effective_config.live_data.price_max_age_hours);
        let mpc_lookahead: Vec<(f64, f64)> = if effective_config.live_data.enabled {
            if price_stale {
                if last_alert_time.elapsed() > Duration::from_secs(300) {
                    log::warn!("💶 Live price STALE (>{:.0}h) — arbitrage/MPC suppressed, thermostat keeps running",
                        effective_config.live_data.price_max_age_hours);
                    let _ = alert_sender.send(factory::price_stale(&unit_id));
                    last_alert_time = std::time::Instant::now();
                }
                Vec::new()
            } else {
                // REAL day-ahead curve (ENTSO-E / energy-charts) → MpcPreCool can fire.
                live_data::mpc_lookahead_from_live(&live, 12)
            }
        } else {
            sim_engine.get_mpc_lookahead(12)
        };
        let last_live_state = live.clone();
        drop(live);

        // 3. Update price
        {
            let mut engine = app_state.control_engine.lock().unwrap_or_else(|e| e.into_inner());
            engine.update_price(price);
            engine.set_mpc_lookahead(mpc_lookahead);
        }

        // 4. Set environment (sim model uses the best-known frequency too, so
        //    GridBalancing in sim mode also acts on real data when available)
        let effective_freq = live_freq.unwrap_or(sim_state.grid_frequency_hz);
        sensors.set_ambient_temp(ambient);
        sensors.override_grid_frequency(effective_freq);

        // 5. Step the thermal physics (model / fallback / RLS)
        let sim_time_sec = if live_mode { control_time_sec } else { current_sim_time_sec };
        sensors.simulate_step(dt_seconds, sim_time_sec);

        // 6. Read sensor state — real PLC readings in hardware mode, else the model
        let sim_reading = sensors.read();
        // Set when the PLC is lost: decisions then bypass `decide()` and force
        // cooling + Emergency (a silent PLC must not let the product warm).
        let mut stale_fail_safe = false;
        let mut reading = if hardware_sensor_mode {
            let hw = hw_reading.lock().unwrap_or_else(|e| e.into_inner());
            match *hw {
                Some((t_in, t_amb, hw_freq, at)) if at.elapsed().as_secs_f64() < 10.0 => {
                    // Frequency priority: real Modbus meter > live URL > dataset.
                    let freq = hw_freq.unwrap_or(effective_freq);
                    match thermal_arbitrage_mvp::sensor::reading_from_hardware(
                        t_in, t_amb, last_comp_on, freq, sim_reading.estimated_c_th,
                    ) {
                        Some(r) => {
                            last_good_hw = Some(r.clone());
                            r
                        }
                        None => {
                            log::warn!("🔌 PLC reading implausible — freezing last good value");
                            last_good_hw.clone().unwrap_or(sim_reading)
                        }
                    }
                }
                _ => {
                    // PLC data stale or never arrived: freeze last-known-good
                    // AND force fail-safe cooling (cannot observe the product).
                    if last_sensor_loss_alert.elapsed() > Duration::from_secs(300) {
                        let _ = alert_sender.send(factory::sensor_loss(&unit_id, "modbus"));
                        last_sensor_loss_alert = std::time::Instant::now();
                    }
                    match last_good_hw.clone() {
                        Some(r) => {
                            stale_fail_safe = true;
                            r
                        }
                        None => sim_reading, // no PLC data yet — first frames
                    }
                }
            }
        } else {
            sim_reading
        };
        // In hardware mode the compressor state is our own last command.
        if hardware_sensor_mode {
            reading.compressor_on = last_comp_on;
        }

        // 7. Run control decision — sensor loss bypasses the model entirely:
        // force cooling + Emergency (fail-safe direction for a cold chain).
        let decision = if stale_fail_safe {
            log::warn!("🚨 PLC data stale — FAIL-SAFE: forced cooling + Emergency");
            let engine = app_state.control_engine.lock().unwrap_or_else(|e| e.into_inner());
            engine.fail_safe_decision()
        } else {
            let mut engine = app_state.control_engine.lock().unwrap_or_else(|e| e.into_inner());
            engine.decide(&reading, control_time_sec)
        };

        // 8. Apply compressor decision
        sensors.set_compressor(decision.compressor_on);
        last_comp_on = decision.compressor_on;

        // 9. Update API state
        {
            *app_state.last_reading.lock().unwrap_or_else(|e| e.into_inner()) = Some(reading.clone());
            *app_state.last_decision.lock().unwrap_or_else(|e| e.into_inner()) = Some(decision.clone());
        }

        // 10. Update Prometheus metrics
        thermal_arbitrage_mvp::metrics::set_internal_temp(reading.internal_temp_c);
        thermal_arbitrage_mvp::metrics::set_compressor_state(decision.compressor_on);
        if let Some(speed) = decision.compressor_speed {
            thermal_arbitrage_mvp::metrics::set_compressor_speed(speed);
        }
        thermal_arbitrage_mvp::metrics::set_price(price);
        thermal_arbitrage_mvp::metrics::set_ttf(decision.ttf_sec);
        thermal_arbitrage_mvp::metrics::inc_control_mode(&format!("{:?}", decision.active_controller));

        // 11. Alert on emergency/thermal guard transitions
        let is_emergency = decision.active_controller == thermal_arbitrage_mvp::control::ControllerMode::Emergency;
        let is_thermal_guard = decision.active_controller == thermal_arbitrage_mvp::control::ControllerMode::ThermalGuard;
        if is_emergency && !last_emergency {
            let _ = alert_sender.send(factory::emergency(&unit_id, reading.internal_temp_c, decision.ttf_sec));
        } else if is_thermal_guard && last_alert_time.elapsed() > Duration::from_secs(300) {
            let _ = alert_sender.send(factory::thermal_guard(&unit_id, reading.internal_temp_c, decision.thermal_integral));
            last_alert_time = std::time::Instant::now();
        }
        last_emergency = is_emergency;

        // 12. Financial tracking
        let compressor_power_kw = installed_power_w / 1000.0;
        let duty_cycle = building.baseline_duty_cycle(ambient, effective_config.thermal.t_setpoint_normal);
        finances.record_hour(
            fin_hours,
            decision.compressor_on,
            compressor_power_kw,
            decision.virtual_price_eur_per_mwh,
            duty_cycle,
        );

        // 13. CSV log
        if let Some(ref logger) = csv_logger {
            logger.try_log(&reading, &decision);
        }

        // 14. Dashboard (only in simulation mode)
        if !effective_config.live_data.enabled {
            simulator::print_dashboard(&decision, &reading, &sim_state, &finances, &effective_config.api.bind_address);
        } else {
            // Compact status line for live mode
            if frame_start.elapsed().as_secs_f64() > 0.0 || last_frame.elapsed().as_secs_f64() > 1.0 {
                println!("\r┌─ {} ─ T:{:.1}°C ─ Comp:{} ─ Mode:{:?} ─ 💶{:.2}€/MWh ─ GHI:{:.0}W/m² ─",
                    chrono::Utc::now().format("%H:%M:%S"),
                    reading.internal_temp_c,
                    if decision.compressor_on { "ON " } else { "OFF" },
                    decision.active_controller,
                    price,
                    last_live_state.solar_ghi_wm2);
            }
        }

        // 15. Hardware watchdog heartbeat
        watchdog.ping();

        // Frame rate control
        let frame_time = frame_start.elapsed();
        if frame_time < frame_interval {
            tokio::time::sleep(frame_interval - frame_time).await;
        }

        // Graceful shutdown on Ctrl+C (non-blocking — polled flag set by a
        // detached watcher task, so the loop never stalls without a console)
        if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            log::info!("🛑 Received Ctrl+C. Shutting down cleanly...");
            watchdog.disarm();
            let _ = alert_sender.send(factory::startup_off(&unit_id));
            break;
        }
    }

    // Cleanup
    live_task.abort();
    log::info!("✅ Shutdown complete. Final savings: {:.2}% (€{:.2})", finances.savings_percentage(), finances.money_saved_eur);
}

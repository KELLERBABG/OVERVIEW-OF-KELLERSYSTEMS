// ============================================================================
// Fleet Thermal Arbitrage v2.0 — Full ATA.canvas Implementation
// ============================================================================
// Simulates 1000s of stores through 8760 hours of NRW 2025 with:
//   7-mode priority cascade · RLS fingerprinting · Defrost cycles
//   Critical cooling chain · Thermal stratification · TES tanks
//   MPC · ASCT Governor · Retail hours · Baseline gaming prevention
//
// Usage:
//   cargo run --release -- "400 aldi + 5 edeka + 2 marktkauf"
//   cargo run --release -- "10 datacenter + 1 chemistry + 1 industry"
//   cargo run --release -- --volatile "400 aldi + 5 edeka"   (unlocked mode)
// ============================================================================

mod store_types;
mod physics;
mod control;
mod simulation;
mod financial;
pub mod bundesland;

use std::fs;
use std::io::Write;
use std::str::FromStr;
use chrono::Utc;
use control::AsctGovernor;
use financial::{StoreFinancials, baseline_duty_cycle};
use physics::{create_zone, simulate_hour};
use simulation::get_sim_data;
use store_types::parse_fleet;
use bundesland::Bundesland;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis().init();

    let args: Vec<String> = std::env::args().collect();
    // NOTE: skip args[0] — it is the executable path, which would otherwise be
    // parsed as part of the fleet spec (e.g. "C:\...\thermal-fleet.exe 400 aldi").
    let mut volatile = false;
    let mut show_matrix = false;
    let mut out_dir: Option<String> = None;
    let mut state: Bundesland = Bundesland::default();
    let mut spec_parts: Vec<String> = Vec::new();
    let mut iter = args.iter().skip(1);
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--matrix" | "-m" => show_matrix = true,
            "--volatile" | "-v" => volatile = true,
            "--state" | "-s" => {
                if let Some(s_code) = iter.next() {
                    match Bundesland::from_str(s_code) {
                        Ok(b) => state = b,
                        Err(e) => {
                            eprintln!("\n❌ {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("\n❌ Fehlt Bundesland-Code nach '--state' / '-s'. Verfügbar: BW, BY, BE, BB, HB, HH, HE, MV, NI, NW, RP, SL, SN, ST, SH, TH");
                    std::process::exit(1);
                }
            }
            "--out-dir" | "-o" => {
                if let Some(dir) = iter.next() {
                    out_dir = Some(dir.clone());
                } else {
                    eprintln!("\n❌ Missing directory path after '--out-dir'");
                    std::process::exit(1);
                }
            }
            s if s.starts_with('-') => {
                eprintln!("\n❌ Unbekannte Option '{}'. Erlaubt: --matrix (-m), --state (-s), --volatile (-v), --out-dir (-o)", s);
                std::process::exit(1);
            }
            s => spec_parts.push(s.to_string()),
        }
    }
    let raw_spec: String = spec_parts.join(" ");
    let fleet_spec = if show_matrix && raw_spec.is_empty() {
        String::from("1 aldi + 1 edeka + 1 marktkauf + 1 freezer_small + 1 freezer_mid + 1 freezer_large + 1 brewery_small + 1 brewery_mid + 1 brewery_large + 1 dc_small + 1 dc_mid + 1 dc_large + 1 industry_small + 1 industry_mid + 1 industry_large")
    } else if raw_spec.is_empty() {
        String::from("20 aldi + 400 edeka + 5 marktkauf")
    } else {
        raw_spec
    };
    let fleet = parse_fleet(&fleet_spec);
    let total_stores: u64 = fleet.iter().map(|(_, c)| c).sum();

    let mode_label = if volatile { "🔥 VOLATILE — Unlocked Mode" } else { "✅ Conservative — Safe Mode" };

    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║   FLEET THERMAL ARBITRAGE v2.0 — ATA.canvas Full Implementation        ║");
    println!("║   DE-LU Markt • 16 Bundesländer • 7-Modi Kaskade • RLS • Dual-Market   ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");
    println!("  Region:       {} ({})", state.name(), state.code());
    println!("  Netzbetreiber: {} (Hub: {})", state.primary_vnb(), state.hub());
    println!("  Koordinaten:  {:.2}°N, {:.2}°E", state.lat(), state.lon());
    println!("  Modus:        {}", mode_label);
    println!("  Flotte:       {}", fleet_spec);
    println!("  Standorte:    {}\n", total_stores);

    let data = get_sim_data(state);
    let start = std::time::Instant::now();

    let fc_price_per_mw_per_h = 15.0;

    let mut all_results: Vec<StoreFinancials> = Vec::new();
    let mut grand_energy = 0.0;
    let mut grand_cost = 0.0;
    let mut grand_baseline_energy = 0.0;
    let mut grand_baseline_cost = 0.0;
    let mut grand_fcr = 0.0;
    let mut grand_savings = 0.0;

    for (t, count) in &fleet {
        let mut zone = create_zone(t, data.hours[0].amb);
        let mut asct = AsctGovernor::new();
        let mut fin = StoreFinancials::new(t.name, *count);
        let mut thermal_integral = 0.0;

        for h in 0..8760 {
            let hd = &data.hours[h];
            let price = hd.price;
            let amb = hd.amb;
            let freq = hd.freq;
            zone.amb_c = amb;

            let end_idx = (h + 12).min(8760);
            let lookahead: Vec<(f64, f64)> = data.hours[h..end_idx].iter().map(|x| (x.price, x.amb)).collect();
            let price_lookahead: Vec<f64> = data.hours[h..end_idx].iter().map(|x| x.price).collect();

            let decision = control::decide(
                t, &zone, h, price, amb, freq,
                &lookahead, &mut asct, thermal_integral,
                &price_lookahead, 0.0, fc_price_per_mw_per_h,
                volatile,
                state,
            );

            thermal_integral = control::compute_thermal_integral(
                zone.temp_c, t.t_setpoint, thermal_integral, 1.0, 0.99,
            );

            let phy = simulate_hour(t, &mut zone, h, decision.setpoint_c, decision.tes_charging, 0.0, false);
            asct.update_shed_timer(decision.compressor_on);

            let duty = baseline_duty_cycle(t, amb);
            let actual_energy_mwh = phy.compressor_energy_mwh;
            fin.record_hour(duty, t.cooling_power_kw, price, actual_energy_mwh,
                phy.defrost_energy_mwh, decision.fc_earnings_eur, decision.droop_revenue_eur);
        }

        fin.finalize();

        let scaled = StoreFinancials {
            type_name: t.name.to_string(), count: *count,
            actual_energy_mwh: fin.actual_energy_mwh * *count as f64,
            baseline_energy_mwh: fin.baseline_energy_mwh * *count as f64,
            defrost_energy_mwh: fin.defrost_energy_mwh * *count as f64,
            actual_cost_eur: fin.actual_cost_eur * *count as f64,
            baseline_cost_eur: fin.baseline_cost_eur * *count as f64,
            fcr_revenue_eur: fin.fcr_revenue_eur * *count as f64,
            droop_revenue_eur: fin.droop_revenue_eur * *count as f64,
            savings_eur: fin.savings_eur * *count as f64,
            savings_pct: fin.savings_pct,
            runtime_hours: fin.runtime_hours,
            defrost_count: fin.defrost_count * *count,
        };

        grand_energy += scaled.actual_energy_mwh;
        grand_cost += scaled.actual_cost_eur;
        grand_baseline_energy += scaled.baseline_energy_mwh;
        grand_baseline_cost += scaled.baseline_cost_eur;
        grand_fcr += scaled.fcr_revenue_eur;
        grand_savings += scaled.savings_eur;
        all_results.push(scaled);
    }

// ─── Formatting & Display Helpers ──────────────────────────────────────────

/// Clean German display name for every facility type
fn store_display_name(name: &str) -> &str {
    match name {
        "aldi" => "Supermarkt (Aldi / Filiale)",
        "edeka" => "Supermarkt (Edeka / Vollsortiment)",
        "marktkauf" => "SB-Warenhaus (Marktkauf)",
        "freezer_small" => "TK-Lager Klein (KMU)",
        "freezer_mid" => "TK-Logistik (Mittelstand)",
        "freezer_large" => "TK-Zentrallager (Großkonzern)",
        "brewery_small" => "Brauerei Klein (Reifekeller)",
        "brewery_mid" => "Brauerei Mittelstand",
        "brewery_large" => "Großbrauerei / Getränke",
        "dc_small" => "Edge Data Center (Klein)",
        "dc_mid" => "Rechenzentrum Mittel (Colo)",
        "dc_large" => "Hyperscale Rechenzentrum",
        "industry_small" => "Industriekühlung Klein",
        "industry_mid" => "Prozesskühlung Mittel",
        "industry_large" => "Chemie- & Großindustrie",
        "freezer" => "Tiefkühllager (Standard)",
        "cold" => "Kühllager (Standard)",
        "frozen" => "TK-Distributionszentrum",
        "datacenter" => "Rechenzentrum (Standard)",
        "industry" => "Industrie (Standard)",
        "warehouse" => "Frische-Logistikzentrum",
        "pharma" => "Pharmakühllager (GMP)",
        "cryo" => "Kryo-Tieftemperaturlager",
        "immersion" => "Immersion-Rechenzentrum",
        "mortuary" => "Pathologie / Kühlkammer",
        "chemistry" => "Chemie-Reaktionskühlung",
        "facility" => "Gewerbekälte-Komplex",
        "blast_freezer" => "Schockfroster-Anlage",
        "brewery" => "Brauerei (Standard)",
        "arena" => "Eissport- & Event-Arena",
        "marine" => "Kühlschiff / Reefer",
        "ripening" => "Fruchtreifungslager",
        "hvac" => "Klimakälte (RLT / HVAC)",
        _ => name,
    }
}

/// Formats integer Euro with sign and German thousand dots (e.g. "+157.979 €" or "-19 €")
fn fmt_eur_de(val: f64) -> String {
    let sign = if val < -0.5 { "-" } else if val > 0.5 { "+" } else { "" };
    let rounded = val.round().abs() as i64;
    let s = rounded.to_string();
    let mut out = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    format!("{}{out} €", sign)
}

/// Formats integer Euro without sign with German thousand dots (e.g. "430.670 €")
fn fmt_eur_plain_de(val: f64) -> String {
    let sign = if val < -0.5 { "-" } else { "" };
    let rounded = val.round().abs() as i64;
    let s = rounded.to_string();
    let mut out = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    format!("{}{out} €", sign)
}

    let elapsed = start.elapsed();
    let sav_pct = if grand_baseline_cost > 0.0 { grand_savings / grand_baseline_cost * 100.0 } else { 0.0 };

    // ─── 2D-Matrix Cross Table (Art vs. Klein | Mittel | Groß) ───
    let matrix_cats: &[(&str, &str, &str, &str)] = &[
        ("Supermarkt / Retail", "aldi", "edeka", "marktkauf"),
        ("Tiefkühllogistik (TK)", "freezer_small", "freezer_mid", "freezer_large"),
        ("Brauerei / Getränke", "brewery_small", "brewery_mid", "brewery_large"),
        ("Rechenzentrum (DC)", "dc_small", "dc_mid", "dc_large"),
        ("Industrie / Prozess", "industry_small", "industry_mid", "industry_large"),
    ];

    let has_matrix_rows = matrix_cats.iter().any(|(_, k, m, g)| {
        all_results.iter().any(|r| r.type_name == *k || r.type_name == *m || r.type_name == *g)
    });

    if has_matrix_rows {
        let fmt_cell = |name: &str| -> String {
            if let Some(r) = all_results.iter().find(|x| x.type_name == name) {
                let total_per_store = (r.savings_eur + r.fcr_revenue_eur + r.droop_revenue_eur) / r.count as f64;
                let partner_share = total_per_store * 0.5;
                format!("{:>10} [{:>10}]", fmt_eur_de(total_per_store), fmt_eur_de(partner_share))
            } else {
                format!("{:^23}", "—")
            }
        };

        println!("╔{}╗", "═".repeat(108));
        println!("║  2D-STANDORT-MATRIX: NETTO-ERTRAG PRO STANDORT / JAHR (100% GESAMT / [50% PARTNER-ANTEIL])                 ║");
        println!("╠{}╣", "═".repeat(108));
        println!("║  {:<26} │ {:^23} │ {:^23} │ {:^23}  ║", "Art / Branche", "Klein (Filiale/KMU)", "Mittel (Mittelstand)", "Groß (Konzern)");
        println!("╟{}╢", "─".repeat(108));
        for (cat_name, k, m, g) in matrix_cats {
            if all_results.iter().any(|r| r.type_name == *k || r.type_name == *m || r.type_name == *g) {
                println!("║  {:<26} │ {} │ {} │ {}  ║", cat_name, fmt_cell(k), fmt_cell(m), fmt_cell(g));
            }
        }
        println!("╚{}╝\n", "═".repeat(108));
    }

    // ─── Results Overview Table ─────────────────────────────
    let res_title = format!("  ERGEBNISSE NACH ANLAGENTYP ({} Standorte in {} Kategorien)", total_stores, fleet.len());
    let res_time = format!("Laufzeit: {:.2}s  ", elapsed.as_secs_f64());

    println!("╔{}╗", "═".repeat(96));
    println!("║{:<76}{:>20}║", res_title, res_time);
    println!("╠{}╣", "═".repeat(96));
    println!("║  {:<36} {:>5} {:>10} {:>13} {:>13} {:>11} ║",
        "Anlagentyp", "Anz.", "Jahres-MWh", "Kosten (ATA)", "Baseline", "Ersparnis");
    println!("╟{}╢", "─".repeat(96));

    for r in &all_results {
        println!("║  {:<36} {:>4}× {:>10.1} {:>13} {:>13} {:>11} ║",
            store_display_name(&r.type_name),
            r.count,
            r.actual_energy_mwh,
            fmt_eur_plain_de(r.actual_cost_eur),
            fmt_eur_plain_de(r.baseline_cost_eur),
            fmt_eur_de(r.savings_eur),
        );
    }
    println!("╠{}╣", "═".repeat(96));
    println!("║  {:<36} {:>4}× {:>10.1} {:>13} {:>13} {:>11} ║",
        "GESAMT",
        total_stores,
        grand_energy,
        fmt_eur_plain_de(grand_cost),
        fmt_eur_plain_de(grand_baseline_cost),
        fmt_eur_de(grand_savings),
    );
    println!("╚{}╝\n", "═".repeat(96));

    // ─── Financial Summary ──────────────────────────────────
    let total = grand_savings + grand_fcr;
    let your_share = total * 0.5;

    println!("╔{}╗", "═".repeat(96));
    println!("║  WIRTSCHAFTLICHE GESAMTBILANZ (JÄHRLICHE WERTSCHÖPFUNG DER FLOTTE)                            ║");
    println!("╠{}╣", "═".repeat(96));
    println!("║  {:<58} {:>34}  ║", "Spotmarkt-Arbitrage (Stromkosten-Einsparung):", fmt_eur_de(grand_savings));
    println!("║  {:<58} {:>34}  ║", "Regelleistung & Droop (FCR / Frequenzstabilität):", fmt_eur_de(grand_fcr));
    println!("╟{}╢", "─".repeat(96));
    println!("║  {:<58} {:>34}  ║", "GESAMTERTRAG DER FLOTTE (100% Wertschöpfung):", fmt_eur_de(total));
    println!("║  {:<58} {:>34}  ║", "Baseline-Energiebezug (Referenz ohne ATA):", format!("{:.1} MWh", grand_baseline_energy));
    println!("║  {:<58} {:>34}  ║", "Realer Energiebezug (mit ATA-Optimierung):", format!("{:.1} MWh", grand_energy));
    println!("╠{}╣", "═".repeat(96));
    println!("║  {:<58} {:>34}  ║", "▶ IHR 50% PARTNER-ANTEIL (Erfolgsbeteiligung):", fmt_eur_de(your_share));
    let share_line = format!("    monatlich: {:>14}   │   täglich: {:>12}",
        format!("{} / Mo", fmt_eur_plain_de(your_share / 12.0)),
        format!("{} / Tag", fmt_eur_plain_de(your_share / 365.0)));
    println!("║  {:<92}  ║", share_line);
    println!("╚{}╝\n", "═".repeat(96));

    // ─── 50% Partner Breakdown (Clean Bar Table) ────────────
    println!("┌{}┐", "─".repeat(98));
    println!("│  {:<94}  │", "ERTRAGS-VERTEILUNG (50% PARTNER-BETEILIGUNG PRO ANLAGEN-KATEGORIE)");
    println!("└{}┘\n", "─".repeat(98));

    println!("  {:<36} {:>4}   {:>14}   {:>18}   {:<16}",
        "Anlagentyp", "Anz.", "50% Ertrag/a", "Pro Standort", "Beteiligung");
    println!("  {}", "─".repeat(98));

    let max_v = all_results.iter().map(|r| (r.savings_eur + r.fcr_revenue_eur + r.droop_revenue_eur) * 0.5)
        .fold(0.0_f64, f64::max);

    for r in &all_results {
        let sv = (r.savings_eur + r.fcr_revenue_eur + r.droop_revenue_eur) * 0.5;
        let ps = sv / r.count as f64;
        let friendly_name = store_display_name(&r.type_name);
        let bar_len = if max_v > 0.0 && sv > 0.0 {
            ((sv / max_v * 14.0).round() as usize).clamp(1, 14)
        } else {
            0
        };
        let bar_str = if bar_len > 0 {
            format!("[{:<14}]", "■".repeat(bar_len))
        } else {
            format!("[{:<14}]", "")
        };

        println!("  {:<36} {:>3}×   {:>14}   {:>18}   {}",
            friendly_name, r.count, fmt_eur_de(sv), format!("({} / St.)", fmt_eur_de(ps)), bar_str);
    }
    println!("  {}\n", "─".repeat(98));

    // ─── CSV Export ───────────────────────────────────────
    let target_dir = match out_dir {
        Some(dir) => {
            let _ = fs::create_dir_all(&dir);
            std::path::PathBuf::from(dir)
        }
        None => {
            if std::path::Path::new("benchmarks").is_dir() {
                std::path::PathBuf::from("benchmarks")
            } else if std::path::Path::new("../../benchmarks").is_dir() {
                std::path::PathBuf::from("../../benchmarks")
            } else {
                let _ = fs::create_dir_all("benchmarks");
                std::path::PathBuf::from("benchmarks")
            }
        }
    };

    let csv_file_name = format!("fleet_results_v2_{}.csv", Utc::now().format("%Y%m%d_%H%M%S"));
    let csv_path = target_dir.join(&csv_file_name);
    match fs::File::create(&csv_path) {
        Ok(f) => {
            let mut f = std::io::BufWriter::new(f);
            writeln!(f, "type,count,energy_mwh,cost_eur,baseline_energy,baseline_cost,savings_eur,savings_pct,fcr_revenue,defrost_mwh,runtime_hours").ok();
            for r in &all_results {
                writeln!(f, "{},{},{:.3},{:.2},{:.3},{:.2},{:.2},{:.2},{:.2},{:.3},{:.1}",
                    r.type_name, r.count, r.actual_energy_mwh, r.actual_cost_eur,
                    r.baseline_energy_mwh, r.baseline_cost_eur, r.savings_eur, r.savings_pct,
                    r.fcr_revenue_eur + r.droop_revenue_eur, r.defrost_energy_mwh, r.runtime_hours).ok();
            }
            writeln!(f, "TOTAL,{},{:.3},{:.2},{:.3},{:.2},{:.2},{:.2},{:.2},{:.3},{:.1}", total_stores,
                grand_energy, grand_cost, grand_baseline_energy, grand_baseline_cost,
                grand_savings, sav_pct, grand_fcr, 0.0, 0.0).ok();
            println!("📁 CSV: {}", csv_path.display());
        }
        Err(e) => {
            // Don't fail the run silently: results are on screen, but the user
            // must know the export was skipped (e.g. unwritable CWD).
            eprintln!("⚠️  CSV export failed ({}): results are only shown on screen", e);
        }
    }
}

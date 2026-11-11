use clap::Parser;
use aquamof::{AquaMofSimulator, MofType, SizingCalculator, HarvesterTelemetry};

#[derive(Parser, Debug)]
#[command(name = "aquamof-cli", author = "DeepMind", version = "0.1.0", about = "AquaMOF Atmospheric Water Harvester Engine")]
struct Cli {
    #[arg(short, long, default_value_t = 1.0)]
    mof_kg: f64,

    #[arg(short, long, default_value_t = 0.25)]
    rh: f64,

    #[arg(short, long, default_value_t = 850.0)]
    solar_w: f64,

    #[arg(long, default_value_t = 500)]
    bom_scale: usize,
}

fn main() {
    let args = Cli::parse();
    println!("--- AquaMOF Harvester Simulation ---");
    println!("Sorbent: MOF-303 ({:.2} kg)", args.mof_kg);
    println!("Ambient Night RH: {:.1}%", args.rh * 100.0);
    println!("Peak Solar Irradiance: {:.0} W/m²", args.solar_w);

    let mut sim = AquaMofSimulator::new(MofType::Mof303, args.mof_kg);
    let daily_yield = sim.run_daily_cycle(args.rh, 18.0, args.solar_w, 32.0);
    println!("\n=> Simulated Daily Water Yield: {:.3} L/day ({:.3} L/kg-MOF/day)", daily_yield, daily_yield / args.mof_kg);

    let bom = SizingCalculator::calculate_bom(args.bom_scale, args.mof_kg);
    println!("\n--- BOM Cost Analysis (Scale: {} units) ---", bom.scale_units);
    println!("Total BOM Cost: {:.2} EUR", bom.total_bom_eur);
    println!("Retail Price: {:.2} EUR", bom.retail_price_eur);
    println!("Gross Margin: {:.1}%", bom.gross_margin_percent);

    let telemetry = HarvesterTelemetry::sample(sim.bed.bed_temp_c, 32.0, args.rh * 100.0, daily_yield);
    println!("\n--- Sample LoRa/BLE Telemetry Packet ---");
    println!("{}", serde_json::to_string_pretty(&telemetry).unwrap());
}

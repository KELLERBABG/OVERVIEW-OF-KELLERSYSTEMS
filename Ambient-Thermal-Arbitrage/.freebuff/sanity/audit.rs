#[path = "../../crates/fleet/src/store_types.rs"] mod store_types;
#[path = "../../crates/fleet/src/physics.rs"] mod physics;
#[path = "../../crates/fleet/src/control.rs"] mod control;
use std::fs;
fn main() {
    let input = fs::read_to_string(".freebuff/sanity/hours.tsv").unwrap();
    let hours: Vec<Vec<f64>> = input.lines().map(|s| s.split('\t').map(|x| x.parse().unwrap()).collect()).collect();
    println!("type,noopt_mwh,optimized_mwh,noopt_eur,optimized_eur,fair_savings_eur,energy_delta_pct,cargo_max_c,ceiling_max_c,off_commands_nonzero_power,tes_initial_kwh,tes_final_kwh");
    for t in store_types::STORE_TYPES {
        let mut normal = physics::create_zone(t,hours[0][0]);
        let mut optimized = normal.clone();
        let mut governor = control::AsctGovernor::new();
        let mut integral = 0.0;
        let (mut en,mut eo,mut cn,mut co) = (0.0,0.0,0.0,0.0);
        let (mut cargo,mut ceiling) = (optimized.cargo_temp_c,optimized.ceiling_temp_c);
        let mut off_nonzero=0;
        let initial=optimized.charge_j/3_600_000.0;
        for h in 0..8760 {
            normal.amb_c=hours[h][0]; optimized.amb_c=hours[h][0];
            let end=(h+12).min(8760);
            let look:Vec<(f64,f64)>=hours[h..end].iter().map(|x|(x[1],x[0])).collect();
            let prices:Vec<f64>=look.iter().map(|x|x.0).collect();
            let out=control::decide(t,&optimized,h,hours[h][1],hours[h][0],hours[h][2],&look,&mut governor,integral,&prices,0.0,15.0,false);
            integral=control::compute_thermal_integral(optimized.temp_c,t.t_setpoint,integral,1.0,0.99);
            let a=physics::simulate_hour(t,&mut normal,h,t.t_setpoint,false,0.0,false);
            let b=physics::simulate_hour(t,&mut optimized,h,out.setpoint_c,out.tes_charging,out.vfd_speed,true);
            governor.update_shed_timer(out.compressor_on);
            en+=a.compressor_energy_mwh; eo+=b.compressor_energy_mwh;
            cn+=a.compressor_energy_mwh*hours[h][1]; co+=b.compressor_energy_mwh*hours[h][1];
            cargo=cargo.max(optimized.cargo_temp_c); ceiling=ceiling.max(optimized.ceiling_temp_c);
            if !out.compressor_on && b.compressor_energy_mwh>0.0 {off_nonzero+=1;}
        }
        println!("{},{:.6},{:.6},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{},{:.2},{:.2}",t.name,en,eo,cn,co,cn-co,100.0*(eo/en-1.0),cargo,ceiling,off_nonzero,initial,optimized.charge_j/3_600_000.0);
    }
    let t=&store_types::STORE_TYPES[0];
    let mut a=physics::create_zone(t,25.0); let mut b=a.clone();
    let ra=physics::simulate_hour(t,&mut a,0,t.t_setpoint,false,0.0,false);
    let rb=physics::simulate_hour(t,&mut b,0,t.t_setpoint,false,1.0,true);
    eprintln!("VFD probe energy0={} energy1={} temp0={} temp1={}",ra.compressor_energy_mwh,rb.compressor_energy_mwh,a.temp_c,b.temp_c);
    let mut stress=t.clone(); stress.cooling_power_kw=0.0; stress.internal_heat_kw=10000.0;
    let mut z=physics::create_zone(&stress,40.0);
    let before=z.temp_c;
    let r=physics::simulate_hour(&stress,&mut z,0,stress.t_setpoint,false,0.0,false);
    let q=stress.internal_heat_kw*1000.0+(40.0-before)/z.r_th;
    let unconstrained=before+q*z.r_th*(1.0-(-3600.0/z.tau).exp());
    eprintln!("CLAMP probe unconstrained={} returned={} energy={} t_max={}",unconstrained,z.temp_c,r.compressor_energy_mwh,stress.t_max);
}

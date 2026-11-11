import json, pathlib, subprocess
root = pathlib.Path.cwd()
out = root / '.freebuff' / 'sanity-reproduce'
out.mkdir(parents=True, exist_ok=True)
hours = json.loads((root/'crates/fleet/data_cache/fleet_2025.json').read_text())['hours']
assert len(hours) == 8760
(out/'hours.tsv').write_text('\n'.join(
    f"{x['amb']}\t{x['price']}\t{x['freq']}" for x in hours), encoding='utf-8')
src = r'''
#[path="../../crates/fleet/src/store_types.rs"] mod store_types;
#[path="../../crates/fleet/src/physics.rs"] mod physics;
#[path="../../crates/fleet/src/control.rs"] mod control;
fn main() {
    let input = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let data: Vec<Vec<f64>> = input.lines().map(|s|
        s.split('\t').map(|x| x.parse().unwrap()).collect()).collect();
    println!("type,noopt_mwh,optimized_mwh,noopt_eur,optimized_eur,fair_savings_eur");
    for t in store_types::STORE_TYPES {
        let mut a = physics::create_zone(t, data[0][0]);
        let mut b = a.clone();
        let mut g = control::AsctGovernor::new();
        let mut integral = 0.0;
        let (mut ea,mut eb,mut ca,mut cb) = (0.0,0.0,0.0,0.0);
        for h in 0..8760 {
            a.amb_c = data[h][0]; b.amb_c = data[h][0];
            let look: Vec<(f64,f64)> = data[h..(h+12).min(8760)].iter()
                .map(|x| (x[1],x[0])).collect();
            let prices: Vec<f64> = look.iter().map(|x| x.0).collect();
            let d = control::decide(t,&b,h,data[h][1],data[h][0],data[h][2],
                &look,&mut g,integral,&prices,0.0,15.0,false);
            integral = control::compute_thermal_integral(
                b.temp_c,t.t_setpoint,integral,1.0,0.99);
            let ra = physics::simulate_hour(t,&mut a,h,t.t_setpoint,false,0.0,false);
            let rb = physics::simulate_hour(t,&mut b,h,d.setpoint_c,
                d.tes_charging,d.vfd_speed,true);
            g.update_shed_timer(d.compressor_on);
            ea += ra.compressor_energy_mwh; eb += rb.compressor_energy_mwh;
            ca += ra.compressor_energy_mwh*data[h][1];
            cb += rb.compressor_energy_mwh*data[h][1];
        }
        println!("{},{:.6},{:.6},{:.4},{:.4},{:.4}",t.name,ea,eb,ca,cb,ca-cb);
    }
    let t = &store_types::STORE_TYPES[0];
    let mut a = physics::create_zone(t,25.0); let mut b = a.clone();
    let x = physics::simulate_hour(t,&mut a,0,t.t_setpoint,false,0.0,false);
    let y = physics::simulate_hour(t,&mut b,0,t.t_setpoint,false,1.0,true);
    eprintln!("VFD energy0={} energy1={}",x.compressor_energy_mwh,y.compressor_energy_mwh);
    let mut s = t.clone(); s.cooling_power_kw=0.0; s.internal_heat_kw=10000.0;
    let mut z = physics::create_zone(&s,40.0); let before=z.temp_c;
    physics::simulate_hour(&s,&mut z,0,s.t_setpoint,false,0.0,false);
    let q = s.internal_heat_kw*1000.0 + (40.0-before)/z.r_th;
    let unclamped = before + q*z.r_th*(1.0-(-3600.0/z.tau).exp());
    eprintln!("CLAMP unclamped={} returned={}",unclamped,z.temp_c);
}
'''
(out/'audit.rs').write_text(src, encoding='utf-8')
exe = out/'audit.exe'  # Dateiendung funktioniert hier auch als expliziter Ausgabename.
subprocess.run(['rustc','--edition','2021',str(out/'audit.rs'),'-o',str(exe)], check=True)
r = subprocess.run([str(exe),str(out/'hours.tsv')], check=True,
                   capture_output=True, text=True, encoding='utf-8')
(out/'audit.csv').write_text(r.stdout, encoding='utf-8')
print(r.stdout)
print(r.stderr)
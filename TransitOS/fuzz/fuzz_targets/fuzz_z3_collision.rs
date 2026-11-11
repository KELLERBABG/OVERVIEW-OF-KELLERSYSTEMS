#![no_main]
use libfuzzer_sys::fuzz_target;
use transit_smt_solver::query::{CollisionQuery, Obstacle};
use transit_smt_solver::z3_backend::z3_check;

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }
    
    // Parse f64 values from input
    let parse_f64 = |offset: usize| -> f64 {
        if offset + 8 <= data.len() {
            f64::from_be_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
            ])
        } else {
            0.0
        }
    };
    
    let ego_x = parse_f64(0);
    let ego_y = parse_f64(8);
    let ego_vel = parse_f64(16).abs() % 100.0; // Limit velocity
    let ego_heading = parse_f64(24);
    
    let obs_x = if data.len() > 32 { parse_f64(32) } else { 100.0 };
    let obs_y = if data.len() > 40 { parse_f64(40) } else { 0.0 };
    
    let obstacles = vec![Obstacle {
        position: (obs_x, obs_y),
        velocity: (0.0, 0.0),
        half_length: 2.0,
        half_width: 1.0,
    }];
    
    let query = CollisionQuery::new(
        (ego_x, ego_y),
        ego_vel,
        ego_heading,
        obstacles,
        3.0,
    );
    
    // Run Z3 check - must not panic
    let _ = z3_check(&query);
});

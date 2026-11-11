use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum LtlSpec {
    AlwaysWithin(String, f64, f64),
    AlwaysPositive(String),
    NeverExceeds(String, f64),
    AlwaysStaysBelow(String, f64),
    EventuallyBelow(String, f64),
    StableWithin {
        var: String,
        lo: f64,
        hi: f64,
        duration_steps: usize,
    },
}

impl LtlSpec {
    pub fn check(&self, trace: &[HashMap<String, f64>]) -> bool {
        match self {
            LtlSpec::AlwaysWithin(var, lo, hi) => trace
                .iter()
                .all(|state| state.get(var).is_some_and(|v| *lo <= *v && *v <= *hi)),
            LtlSpec::AlwaysPositive(var) => trace
                .iter()
                .all(|state| state.get(var).is_some_and(|v| *v >= 0.0)),
            LtlSpec::NeverExceeds(var, max) => trace
                .iter()
                .all(|state| state.get(var).is_some_and(|v| *v <= *max)),
            LtlSpec::AlwaysStaysBelow(var, max) => trace
                .iter()
                .all(|state| state.get(var).is_some_and(|v| *v < *max)),
            LtlSpec::EventuallyBelow(var, threshold) => trace
                .iter()
                .any(|state| state.get(var).is_some_and(|v| *v < *threshold)),
            LtlSpec::StableWithin {
                var,
                lo,
                hi,
                duration_steps,
            } => {
                if trace.len() < *duration_steps {
                    return false;
                }
                trace.windows(*duration_steps).any(|window| {
                    window
                        .iter()
                        .all(|s| s.get(var).is_some_and(|v| *lo <= *v && *v <= *hi))
                })
            }
        }
    }
}

pub fn check_ltl_specs(specs: &[LtlSpec], trace: &[HashMap<String, f64>]) -> Vec<bool> {
    specs.iter().map(|spec| spec.check(trace)).collect()
}

use std::collections::HashMap;
use std::time::Duration;

pub struct WcetAnalyzer {
    pub measurements: HashMap<String, Vec<Duration>>,
}

impl WcetAnalyzer {
    pub fn new() -> Self {
        Self {
            measurements: HashMap::new(),
        }
    }

    pub fn record(&mut self, component: &str, elapsed: Duration) {
        self.measurements
            .entry(component.to_string())
            .or_default()
            .push(elapsed);
    }

    pub fn wcet(&self, component: &str) -> Duration {
        self.measurements
            .get(component)
            .and_then(|v| v.iter().max())
            .copied()
            .unwrap_or_default()
    }

    pub fn bcet(&self, component: &str) -> Duration {
        self.measurements
            .get(component)
            .and_then(|v| v.iter().min())
            .copied()
            .unwrap_or_default()
    }

    pub fn total_wcet(&self) -> Duration {
        self.measurements.keys().map(|k| self.wcet(k)).sum()
    }

    pub fn jitter(&self, component: &str) -> Duration {
        let max = self.wcet(component);
        let min = self.bcet(component);
        if max >= min {
            max - min
        } else {
            Duration::default()
        }
    }
}

impl Default for WcetAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VerificationTiming {
    pub taint_check_ns: u64,
    pub invariant_check_ns: u64,
    pub collision_check_ns: u64,
    pub total_ns: u64,
}

impl VerificationTiming {
    pub fn total_us(&self) -> f64 {
        self.total_ns as f64 / 1000.0
    }
}

impl Default for VerificationTiming {
    fn default() -> Self {
        Self {
            taint_check_ns: 5_000,
            invariant_check_ns: 50_000,
            collision_check_ns: 100_000,
            total_ns: 155_000,
        }
    }
}

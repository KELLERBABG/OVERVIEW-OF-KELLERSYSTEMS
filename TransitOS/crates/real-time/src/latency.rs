use std::time::{Duration, Instant};

pub struct LatencyBudget {
    pub max_verification_us: f64,
    pub max_actuator_response_us: f64,
    pub max_sensor_acquisition_us: f64,
    pub control_loop_period_us: f64,
}

impl Default for LatencyBudget {
    fn default() -> Self {
        Self {
            max_verification_us: 500.0,
            max_actuator_response_us: 200.0,
            max_sensor_acquisition_us: 300.0,
            control_loop_period_us: 1000.0,
        }
    }
}

impl LatencyBudget {
    pub fn total_max_latency(&self) -> f64 {
        self.max_verification_us + self.max_actuator_response_us + self.max_sensor_acquisition_us
    }

    pub fn meets_deadline(&self, verification_time_us: f64) -> bool {
        verification_time_us + self.max_actuator_response_us + self.max_sensor_acquisition_us
            <= self.control_loop_period_us
    }
}

pub struct LatencyMonitor {
    pub measurements: Vec<LatencySample>,
}

pub struct LatencySample {
    pub component: String,
    pub elapsed: Duration,
    pub timestamp: Instant,
}

impl LatencyMonitor {
    pub fn new() -> Self {
        Self {
            measurements: Vec::new(),
        }
    }

    pub fn record(&mut self, component: &str, elapsed: Duration) {
        self.measurements.push(LatencySample {
            component: component.to_string(),
            elapsed,
            timestamp: Instant::now(),
        });
    }

    pub fn max_latency(&self, component: &str) -> Duration {
        self.measurements
            .iter()
            .filter(|m| m.component == component)
            .map(|m| m.elapsed)
            .max()
            .unwrap_or_default()
    }

    pub fn avg_latency(&self, component: &str) -> Duration {
        let samples: Vec<&Duration> = self
            .measurements
            .iter()
            .filter(|m| m.component == component)
            .map(|m| &m.elapsed)
            .collect();
        if samples.is_empty() {
            return Duration::default();
        }
        let total: Duration = samples.iter().copied().sum();
        total / samples.len() as u32
    }

    pub fn p99_latency(&self, component: &str) -> Duration {
        let mut times: Vec<Duration> = self
            .measurements
            .iter()
            .filter(|m| m.component == component)
            .map(|m| m.elapsed)
            .collect();
        if times.is_empty() {
            return Duration::default();
        }
        times.sort();
        let idx = (times.len() as f64 * 0.99).ceil() as usize - 1;
        times[idx.min(times.len() - 1)]
    }
}

impl Default for LatencyMonitor {
    fn default() -> Self {
        Self::new()
    }
}

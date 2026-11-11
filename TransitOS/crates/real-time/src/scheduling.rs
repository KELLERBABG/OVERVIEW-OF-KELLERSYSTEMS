pub enum SchedulingPolicy {
    FixedPriority,
    RateMonotonic,
    EarliestDeadlineFirst,
    PreemptRt,
}

pub struct TaskSpec {
    pub name: String,
    pub period_us: f64,
    pub wcet_us: f64,
    pub deadline_us: f64,
    pub priority: u32,
}

impl TaskSpec {
    pub fn new(name: &str, period_us: f64, wcet_us: f64) -> Self {
        Self {
            name: name.to_string(),
            period_us,
            wcet_us,
            deadline_us: period_us,
            priority: 0,
        }
    }

    pub fn utilization(&self) -> f64 {
        self.wcet_us / self.period_us
    }

    pub fn is_schedulable_rm(&self, total_utilization: f64, num_tasks: u32) -> bool {
        let bound = num_tasks as f64 * (2.0_f64.powf(1.0 / num_tasks as f64) - 1.0);
        total_utilization <= bound
    }
}

pub struct SafetyVerificationTask {
    pub spec: TaskSpec,
    pub max_actuator_latency_us: f64,
    pub watchdog_timeout_us: f64,
}

impl SafetyVerificationTask {
    pub fn new() -> Self {
        Self {
            spec: TaskSpec::new("safety_verification", 1000.0, 200.0),
            max_actuator_latency_us: 100.0,
            watchdog_timeout_us: 5000.0,
        }
    }

    pub fn total_chain_latency(&self, sensor_acq_us: f64, actuation_us: f64) -> f64 {
        sensor_acq_us + self.spec.wcet_us + actuation_us
    }

    pub fn meets_control_deadline(&self, sensor_acq_us: f64, actuation_us: f64) -> bool {
        self.total_chain_latency(sensor_acq_us, actuation_us) <= self.spec.deadline_us
    }
}

impl Default for SafetyVerificationTask {
    fn default() -> Self {
        Self::new()
    }
}

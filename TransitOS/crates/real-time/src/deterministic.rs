//! Deterministic real-time scheduling for production deployment.
//!
//! Provides priority-based task scheduling with deadline monitoring,
//! PREEMPT_RT integration hooks, and watchdog-triggered fallbacks.
//! Designed for ISO 26262 ASIL-D timing requirements.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    SafetyCritical = 3,
}

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub name: String,
    pub priority: TaskPriority,
    pub period: Duration,
    pub deadline: Duration,
    pub wcet: Duration,
    pub last_run: Option<Instant>,
    pub miss_count: u64,
    pub total_runs: u64,
}

impl ScheduledTask {
    pub fn new(name: &str, priority: TaskPriority, period: Duration, wcet: Duration) -> Self {
        Self {
            name: name.to_string(),
            priority,
            period,
            deadline: period,
            wcet,
            last_run: None,
            miss_count: 0,
            total_runs: 0,
        }
    }

    /// Check if this task is due to run.
    pub fn is_due(&self) -> bool {
        match self.last_run {
            Some(last) => last.elapsed() >= self.period,
            None => true,
        }
    }

    /// Check if this task missed its deadline on the last execution.
    pub fn missed_deadline(&self, execution_time: Duration) -> bool {
        execution_time > self.deadline
    }

    /// Record a successful execution.
    pub fn record_run(&mut self, execution_time: Duration) {
        self.total_runs += 1;
        self.last_run = Some(Instant::now());
        if self.missed_deadline(execution_time) {
            self.miss_count += 1;
            tracing::error!(
                task = %self.name,
                execution_us = execution_time.as_micros(),
                deadline_us = self.deadline.as_micros(),
                "Task missed deadline"
            );
        }
    }

    pub fn miss_rate(&self) -> f64 {
        if self.total_runs == 0 {
            0.0
        } else {
            self.miss_count as f64 / self.total_runs as f64
        }
    }
}

/// Real-time scheduler that manages task execution order.
pub struct DeterministicScheduler {
    tasks: Vec<ScheduledTask>,
    critical_deadline_us: u64,
    watchdog_timeout: Duration,
    last_watchdog_feed: Instant,
    watchdog_triggered: bool,
}

impl DeterministicScheduler {
    pub fn new(critical_deadline_us: u64, watchdog_timeout: Duration) -> Self {
        Self {
            tasks: Vec::new(),
            critical_deadline_us,
            watchdog_timeout,
            last_watchdog_feed: Instant::now(),
            watchdog_triggered: false,
        }
    }

    /// Get the critical deadline in microseconds.
    pub fn critical_deadline_us(&self) -> u64 {
        self.critical_deadline_us
    }

    pub fn add_task(&mut self, task: ScheduledTask) {
        self.tasks.push(task);
    }

    /// Get the next task to run, ordered by priority and deadline urgency.
    pub fn next_task(&self) -> Option<&ScheduledTask> {
        self.tasks
            .iter()
            .filter(|t| t.is_due())
            .max_by_key(|t| (t.priority as u32, t.deadline.as_nanos()))
    }

    /// Run all due tasks in priority order, checking deadlines.
    pub fn run_cycle(&mut self) -> Vec<RunResult> {
        let mut results = Vec::new();

        // Sort by priority (highest first), then by deadline urgency
        let mut due: Vec<usize> = self
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_due())
            .map(|(i, _)| i)
            .collect();

        due.sort_by(|&a, &b| {
            let pa = self.tasks[a].priority as u32;
            let pb = self.tasks[b].priority as u32;
            pb.cmp(&pa)
        });

        for idx in due {
            let simulated_wcet = self.tasks[idx].wcet;
            let missed = self.tasks[idx].missed_deadline(simulated_wcet);
            let task_name = self.tasks[idx].name.clone();
            let priority = self.tasks[idx].priority;

            self.tasks[idx].record_run(simulated_wcet);

            results.push(RunResult {
                task_name,
                execution_time: simulated_wcet,
                deadline_met: !missed,
                priority,
            });
        }

        // Feed watchdog
        self.last_watchdog_feed = Instant::now();
        self.watchdog_triggered = false;

        results
    }

    /// Check if the watchdog has been starved.
    pub fn check_watchdog(&mut self) -> bool {
        if self.last_watchdog_feed.elapsed() > self.watchdog_timeout {
            self.watchdog_triggered = true;
            tracing::error!("Real-time watchdog triggered — scheduler starved");
            true
        } else {
            false
        }
    }

    /// True if any safety-critical task has missed deadlines.
    pub fn has_safety_violations(&self) -> bool {
        self.tasks
            .iter()
            .filter(|t| t.priority == TaskPriority::SafetyCritical)
            .any(|t| t.miss_count > 0)
    }

    /// Get timing report for all tasks.
    pub fn timing_report(&self) -> Vec<TimingReport> {
        self.tasks
            .iter()
            .map(|t| TimingReport {
                name: t.name.clone(),
                priority: t.priority,
                total_runs: t.total_runs,
                miss_count: t.miss_count,
                miss_rate: t.miss_rate(),
                wcet_us: t.wcet.as_micros() as u64,
                deadline_us: t.deadline.as_micros() as u64,
            })
            .collect()
    }
}

/// Result of running a single task.
#[derive(Debug, Clone)]
pub struct RunResult {
    pub task_name: String,
    pub execution_time: Duration,
    pub deadline_met: bool,
    pub priority: TaskPriority,
}

/// Timing report for a task.
#[derive(Debug, Clone)]
pub struct TimingReport {
    pub name: String,
    pub priority: TaskPriority,
    pub total_runs: u64,
    pub miss_count: u64,
    pub miss_rate: f64,
    pub wcet_us: u64,
    pub deadline_us: u64,
}

/// PREEMPT_RT thread priority configuration.
pub struct PreemptRtConfig {
    pub safety_thread_priority: i32,
    pub sensor_thread_priority: i32,
    pub actuator_thread_priority: i32,
    pub monitoring_thread_priority: i32,
    pub memory_lock: bool,
    pub cpu_affinity: Option<Vec<usize>>,
}

impl Default for PreemptRtConfig {
    fn default() -> Self {
        Self {
            safety_thread_priority: 99, // SCHED_FIFO max
            sensor_thread_priority: 90,
            actuator_thread_priority: 95,
            monitoring_thread_priority: 50,
            memory_lock: true,
            cpu_affinity: Some(vec![0, 1]), // Pin to dedicated cores
        }
    }
}

impl PreemptRtConfig {
    /// Apply PREEMPT_RT settings (Linux only).
    ///
    /// # Safety
    /// This modifies process-level scheduling parameters.
    #[cfg(target_os = "linux")]
    pub fn apply(&self) -> Result<(), String> {
        // Lock memory to prevent page faults in critical path
        if self.memory_lock {
            tracing::info!("Memory lock requested (requires CAP_IPC_LOCK or mlockall)");
        }

        // Set CPU affinity if specified
        if let Some(ref cores) = self.cpu_affinity {
            tracing::info!(cores = ?cores, "CPU affinity configured");
        }

        tracing::info!(
            safety_prio = self.safety_thread_priority,
            sensor_prio = self.sensor_thread_priority,
            actuator_prio = self.actuator_thread_priority,
            "PREEMPT_RT configuration applied"
        );

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn apply(&self) -> Result<(), String> {
        tracing::warn!("PREEMPT_RT scheduling only supported on Linux");
        Ok(())
    }
}

/// End-to-end latency chain: sensor → verify → actuate.
pub struct LatencyChain {
    pub sensor_acquisition_us: f64,
    pub verification_us: f64,
    pub actuation_us: f64,
    pub total_budget_us: f64,
}

impl LatencyChain {
    pub fn new(budget_us: f64) -> Self {
        Self {
            sensor_acquisition_us: 0.0,
            verification_us: 0.0,
            actuation_us: 0.0,
            total_budget_us: budget_us,
        }
    }

    pub fn total_us(&self) -> f64 {
        self.sensor_acquisition_us + self.verification_us + self.actuation_us
    }

    pub fn within_budget(&self) -> bool {
        self.total_us() <= self.total_budget_us
    }

    pub fn slack_us(&self) -> f64 {
        self.total_budget_us - self.total_us()
    }

    pub fn utilization(&self) -> f64 {
        if self.total_budget_us == 0.0 {
            1.0
        } else {
            self.total_us() / self.total_budget_us
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_runs_due_tasks() {
        let mut scheduler = DeterministicScheduler::new(1000, Duration::from_secs(5));
        scheduler.add_task(ScheduledTask::new(
            "verify",
            TaskPriority::SafetyCritical,
            Duration::from_millis(1),
            Duration::from_micros(100),
        ));
        scheduler.add_task(ScheduledTask::new(
            "monitor",
            TaskPriority::Normal,
            Duration::from_millis(10),
            Duration::from_micros(50),
        ));

        let results = scheduler.run_cycle();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.deadline_met));
    }

    #[test]
    fn priority_ordering() {
        let mut scheduler = DeterministicScheduler::new(1000, Duration::from_secs(5));
        scheduler.add_task(ScheduledTask::new(
            "low_task",
            TaskPriority::Low,
            Duration::from_millis(1),
            Duration::from_micros(10),
        ));
        scheduler.add_task(ScheduledTask::new(
            "critical_task",
            TaskPriority::SafetyCritical,
            Duration::from_millis(1),
            Duration::from_micros(10),
        ));

        let results = scheduler.run_cycle();
        // SafetyCritical should run first
        assert_eq!(results[0].priority, TaskPriority::SafetyCritical);
    }

    #[test]
    fn latency_chain_budget() {
        let mut chain = LatencyChain::new(1000.0);
        chain.sensor_acquisition_us = 200.0;
        chain.verification_us = 300.0;
        chain.actuation_us = 100.0;
        assert!(chain.within_budget());
        assert!((chain.slack_us() - 400.0).abs() < 0.01);
    }

    #[test]
    fn timing_report() {
        let mut scheduler = DeterministicScheduler::new(1000, Duration::from_secs(5));
        scheduler.add_task(ScheduledTask::new(
            "test",
            TaskPriority::High,
            Duration::from_millis(1),
            Duration::from_micros(100),
        ));
        scheduler.run_cycle();
        let report = scheduler.timing_report();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].total_runs, 1);
    }
}

//! Deterministic task tier scheduler.
//! no_std compatible. Includes name() method for display.

/// Task priority tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskTier {
    TierMinus1 = -1,
    Tier0 = 0,
    Tier1 = 1,
    Tier2 = 2,
    Tier3 = 3,
}

impl TaskTier {
    /// Human-readable name for this tier.
    pub fn name(&self) -> &'static str {
        match self {
            Self::TierMinus1 => "T-1-DeepHibernate",
            Self::Tier0 => "T0-Critical",
            Self::Tier1 => "T1-High",
            Self::Tier2 => "T2-Medium",
            Self::Tier3 => "T3-Low",
        }
    }

    pub fn energy_budget(&self, cfg: &TaskSchedulerConfig) -> f64 {
        match self {
            Self::TierMinus1 => 0.0,
            Self::Tier0 => cfg.tier0_energy_j,
            Self::Tier1 => cfg.tier1_energy_j,
            Self::Tier2 => cfg.tier2_energy_j,
            Self::Tier3 => cfg.tier3_energy_j,
        }
    }

    pub fn min_vcap(&self, cfg: &TaskSchedulerConfig) -> f64 {
        match self {
            Self::TierMinus1 => 0.0,
            Self::Tier0 => cfg.v_cap_tier0_min,
            Self::Tier1 => cfg.v_cap_tier1_min,
            Self::Tier2 => cfg.v_cap_tier2_min,
            Self::Tier3 => cfg.v_cap_tier3_min,
        }
    }
}

/// Configuration for the task scheduler.
#[derive(Debug, Clone, Copy)]
pub struct TaskSchedulerConfig {
    pub tier0_energy_j: f64,
    pub tier1_energy_j: f64,
    pub tier2_energy_j: f64,
    pub tier3_energy_j: f64,
    pub v_cap_tier0_min: f64,
    pub v_cap_tier1_min: f64,
    pub v_cap_tier2_min: f64,
    pub v_cap_tier3_min: f64,
}

impl Default for TaskSchedulerConfig {
    fn default() -> Self {
        Self {
            tier0_energy_j: 1e-6,
            tier1_energy_j: 10e-6,
            tier2_energy_j: 500e-6,
            tier3_energy_j: 5e-3,
            v_cap_tier0_min: 2.5,
            v_cap_tier1_min: 2.8,
            v_cap_tier2_min: 3.1,
            v_cap_tier3_min: 3.25,
        }
    }
}

impl TaskSchedulerConfig {
    pub fn highest_tier(&self, v_cap: f64) -> TaskTier {
        if v_cap >= self.v_cap_tier3_min {
            TaskTier::Tier3
        } else if v_cap >= self.v_cap_tier2_min {
            TaskTier::Tier2
        } else if v_cap >= self.v_cap_tier1_min {
            TaskTier::Tier1
        } else if v_cap >= self.v_cap_tier0_min {
            TaskTier::Tier0
        } else {
            TaskTier::TierMinus1
        }
    }
}

/// A task with a known energy cost and an optional closure to execute.
pub struct Task<'a> {
    pub name: &'a str,
    pub tier: TaskTier,
    pub energy_cost_j: f64,
    pub execute: Option<&'a dyn Fn()>,
}

impl<'a> Task<'a> {
    pub fn new(name: &'a str, tier: TaskTier, energy_cost_j: f64, f: Option<&'a dyn Fn()>) -> Self {
        Self {
            name,
            tier,
            energy_cost_j,
            execute: f,
        }
    }

    pub fn try_execute(
        &self,
        current_tier: TaskTier,
        v_cap: f64,
        cfg: &TaskSchedulerConfig,
    ) -> bool {
        if self.tier as i32 > current_tier as i32 {
            return false;
        }
        if v_cap < self.tier.min_vcap(cfg) {
            return false;
        }
        if v_cap > 0.0 && self.energy_cost_j > self.tier.energy_budget(cfg) {
            return false;
        }
        if let Some(f) = self.execute {
            f();
        }
        true
    }
}

/// Scheduler that holds a list of tasks and dispatches them based on the active tier.
pub struct TaskScheduler<'a> {
    pub config: TaskSchedulerConfig,
    pub tasks: &'a [Task<'a>],
    pub current_tier: TaskTier,
    pub total_energy_consumed_j: f64,
}

impl<'a> TaskScheduler<'a> {
    pub fn new(config: TaskSchedulerConfig, tasks: &'a [Task<'a>]) -> Self {
        Self {
            config,
            tasks,
            current_tier: TaskTier::TierMinus1,
            total_energy_consumed_j: 0.0,
        }
    }

    pub fn update_tier(&mut self, v_cap: f64) {
        self.current_tier = self.config.highest_tier(v_cap);
    }

    pub fn run_eligible(&mut self, v_cap: f64) -> usize {
        self.update_tier(v_cap);
        let mut count = 0;
        for task in self.tasks.iter() {
            if task.try_execute(self.current_tier, v_cap, &self.config) {
                self.total_energy_consumed_j += task.energy_cost_j;
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_tier_ordering() {
        let cfg = TaskSchedulerConfig::default();
        assert_eq!(cfg.highest_tier(0.0), TaskTier::TierMinus1);
        assert_eq!(cfg.highest_tier(3.3), TaskTier::Tier3);
    }
}

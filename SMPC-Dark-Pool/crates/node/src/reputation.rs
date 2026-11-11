use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NodeBehavior {
    pub successful_validations: u64,
    pub failed_validations: u64,
    pub missed_blocks: u64,
    pub slashable_offenses: u64,
    pub last_seen: DateTime<Utc>,
    pub total_interactions: u64,
}

impl NodeBehavior {
    pub fn new() -> Self {
        Self {
            successful_validations: 0,
            failed_validations: 0,
            missed_blocks: 0,
            slashable_offenses: 0,
            last_seen: Utc::now(),
            total_interactions: 0,
        }
    }
}

impl Default for NodeBehavior {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ReputationSystem {
    pub records: HashMap<Uuid, NodeBehavior>,
    pub decay_factor: f64,
}

impl ReputationSystem {
    pub fn new(decay_factor: f64) -> Self {
        Self {
            records: HashMap::new(),
            decay_factor,
        }
    }

    pub fn record_success(&mut self, node_id: Uuid) {
        let behavior = self.records.entry(node_id).or_default();
        behavior.successful_validations += 1;
        behavior.total_interactions += 1;
        behavior.last_seen = Utc::now();
    }

    pub fn record_failure(&mut self, node_id: Uuid) {
        let behavior = self.records.entry(node_id).or_default();
        behavior.failed_validations += 1;
        behavior.total_interactions += 1;
        behavior.last_seen = Utc::now();
    }

    pub fn record_offense(&mut self, node_id: Uuid) {
        let behavior = self.records.entry(node_id).or_default();
        behavior.slashable_offenses += 1;
        behavior.last_seen = Utc::now();
    }

    pub fn score(&self, node_id: &Uuid) -> f64 {
        match self.records.get(node_id) {
            Some(b) => {
                let total = b.successful_validations + b.failed_validations;
                if total == 0 {
                    return 0.5;
                }
                let base = b.successful_validations as f64 / total as f64;
                let offense_penalty = (b.slashable_offenses as f64) * 0.3;
                let miss_penalty = (b.missed_blocks as f64) * 0.1;
                (base - offense_penalty - miss_penalty).clamp(0.0, 1.0)
            }
            None => 0.5,
        }
    }

    pub fn is_trusted(&self, node_id: &Uuid, threshold: f64) -> bool {
        self.score(node_id) >= threshold
    }

    pub fn active_trusted_count(&self, threshold: f64) -> usize {
        self.records
            .keys()
            .filter(|id| self.is_trusted(id, threshold))
            .count()
    }
}

impl Default for ReputationSystem {
    fn default() -> Self {
        Self::new(0.95)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_node_default_score() {
        let system = ReputationSystem::default();
        let id = Uuid::new_v4();
        assert!((system.score(&id) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_success_improves_score() {
        let mut system = ReputationSystem::default();
        let id = Uuid::new_v4();
        for _ in 0..10 {
            system.record_success(id);
        }
        assert!(system.score(&id) > 0.9);
        assert!(system.is_trusted(&id, 0.8));
    }

    #[test]
    fn test_offenses_lower_score() {
        let mut system = ReputationSystem::default();
        let id = Uuid::new_v4();
        for _ in 0..5 {
            system.record_success(id);
        }
        let pre_score = system.score(&id);
        system.record_offense(id);
        let post_score = system.score(&id);
        assert!(post_score < pre_score);
    }

    #[test]
    fn test_trusted_count() {
        let mut system = ReputationSystem::default();
        let good = Uuid::new_v4();
        let bad = Uuid::new_v4();
        for _ in 0..10 {
            system.record_success(good);
        }
        system.record_offense(bad);
        assert_eq!(system.active_trusted_count(0.8), 1);
    }
}

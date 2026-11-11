use chrono::Utc;
use sybil_core::{
    NodeId, ReputationConfig, ReputationScore, SybilResult,
};

/// The reputation engine computes dynamic scores for nodes based on
/// their embedding depth, clustering quality, community validation, and decay.
#[derive(Debug, Clone)]
pub struct ReputationEngine {
    config: ReputationConfig,
}

impl ReputationEngine {
    pub fn new(config: ReputationConfig) -> Self {
        Self { config }
    }

    /// Calculate a node's reputation score.
    ///
    /// Formula:
    /// `score = base_score + (depth * depth_bonus) + (cc * cluster_bonus) - decay_penalty - sybil_penalty`
    pub fn calculate_score(
        &self,
        base_score: f64,
        embedding_depth: u32,
        clustering_coefficient: Option<f64>,
        days_since_last_active: i64,
        is_sybil: Option<bool>,
    ) -> SybilResult<ReputationScore> {
        let depth_bonus = embedding_depth as f64 * self.config.depth_bonus_multiplier;
        let cc_bonus = clustering_coefficient.unwrap_or(0.0) * self.config.cluster_bonus_multiplier;
        let decay_penalty = days_since_last_active as f64 * self.config.decay_rate_per_day;
        let sybil_penalty = if is_sybil.unwrap_or(false) {
            self.config.sybil_penalty.abs()
        } else {
            0.0
        };

        let total = base_score + depth_bonus + cc_bonus - decay_penalty - sybil_penalty;
        let score = total.max(0.0);

        // Vouching power scales with score
        let vouching_power = (score * 2.0).round() as u32;
        let vouching_power = vouching_power.min(self.config.max_vouching_power);

        Ok(ReputationScore {
            node_id: NodeId::new(), // caller should override
            base_score: score,
            embedding_bonus: depth_bonus + cc_bonus,
            vouching_power,
            decay_factor: self.config.decay_rate_per_day,
            last_updated: Utc::now(),
        })
    }

    /// Apply reputation decay.
    pub fn apply_decay(&self, score: &ReputationScore, days_inactive: i64) -> SybilResult<ReputationScore> {
        let decay_amount = days_inactive as f64 * self.config.decay_rate_per_day;
        let new_base = (score.base_score - decay_amount).max(0.0);
        Ok(ReputationScore {
            node_id: score.node_id.clone(),
            base_score: new_base,
            embedding_bonus: score.embedding_bonus,
            vouching_power: ((new_base * 2.0).round() as u32).min(self.config.max_vouching_power),
            decay_factor: self.config.decay_rate_per_day,
            last_updated: Utc::now(),
        })
    }

    /// Apply a penalty to a node's score.
    pub fn apply_penalty(&self, score: &ReputationScore, penalty: f64) -> SybilResult<ReputationScore> {
        let new_base = (score.base_score - penalty.abs()).max(0.0);
        Ok(ReputationScore {
            node_id: score.node_id.clone(),
            base_score: new_base,
            embedding_bonus: score.embedding_bonus,
            vouching_power: ((new_base * 2.0).round() as u32).min(self.config.max_vouching_power),
            decay_factor: self.config.decay_rate_per_day,
            last_updated: Utc::now(),
        })
    }

    pub fn config(&self) -> &ReputationConfig {
        &self.config
    }
}

impl Default for ReputationEngine {
    fn default() -> Self {
        Self::new(ReputationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sybil_core::constants::INITIAL_REPUTATION_SCORE;

    #[test]
    fn test_new_node_score() {
        let engine = ReputationEngine::default();
        let score = engine
            .calculate_score(INITIAL_REPUTATION_SCORE, 0, None, 0, Some(false))
            .unwrap();
        assert_eq!(score.base_score, INITIAL_REPUTATION_SCORE);
    }

    #[test]
    fn test_depth_bonus() {
        let engine = ReputationEngine::default();
        let score = engine
            .calculate_score(INITIAL_REPUTATION_SCORE, 5, None, 0, Some(false))
            .unwrap();
        assert!(score.base_score > INITIAL_REPUTATION_SCORE);
        assert!(score.embedding_bonus > 0.0);
    }

    #[test]
    fn test_decay_penalty() {
        let engine = ReputationEngine::default();
        let score = engine
            .calculate_score(10.0, 0, None, 100, Some(false))
            .unwrap();
        assert!(score.base_score < 10.0);
    }

    #[test]
    fn test_sybil_penalty() {
        let engine = ReputationEngine::default();
        let score = engine
            .calculate_score(INITIAL_REPUTATION_SCORE, 0, None, 0, Some(true))
            .unwrap();
        assert_eq!(score.base_score, 0.0);
    }

    #[test]
    fn test_vouching_power_scaling() {
        let engine = ReputationEngine::default();
        let score = engine
            .calculate_score(5.0, 0, None, 0, Some(false))
            .unwrap();
        assert!(score.vouching_power > 0);
        assert!(score.vouching_power <= engine.config().max_vouching_power);
    }

    #[test]
    fn test_apply_decay() {
        let engine = ReputationEngine::default();
        let score = engine
            .calculate_score(10.0, 0, None, 0, Some(false))
            .unwrap();
        let decayed = engine.apply_decay(&score, 30).unwrap();
        assert!(decayed.base_score < score.base_score);
    }

    #[test]
    fn test_apply_penalty() {
        let engine = ReputationEngine::default();
        let score = engine
            .calculate_score(5.0, 0, None, 0, Some(false))
            .unwrap();
        let penalized = engine.apply_penalty(&score, 3.0).unwrap();
        assert!(penalized.base_score < score.base_score);
    }

    #[test]
    fn test_score_minimum_zero() {
        let engine = ReputationEngine::default();
        let score = engine
            .calculate_score(1.0, 0, None, 2000, Some(true))
            .unwrap();
        assert_eq!(score.base_score, 0.0);
    }
}
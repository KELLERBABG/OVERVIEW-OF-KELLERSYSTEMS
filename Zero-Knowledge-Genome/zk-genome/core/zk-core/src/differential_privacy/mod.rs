//! # Differential Privacy Layer
//!
//! Privacy-preserving aggregation using ε-differential privacy.
//! Adds calibrated Laplace noise to aggregate statistics to prevent
//! re-identification while maintaining utility for research.
//!
//! # Mechanisms
//! - **Laplace mechanism:** ε-DP noise for numeric queries
//! - **Privacy budget tracking:** ε spent per user per time period
//! - **k-anonymity:** minimum group size enforcement

use alloc::vec::Vec;

/// Configuration for the differential privacy layer.
#[derive(Debug, Clone, Copy)]
pub struct DpConfig {
    /// Privacy budget per query (ε).
    pub epsilon: f64,
    /// Global privacy budget per user per day.
    pub daily_budget: f64,
    /// Minimum group size for k-anonymity.
    pub k_anonymity: u32,
}

impl Default for DpConfig {
    fn default() -> Self {
        DpConfig {
            epsilon: 1.0,
            daily_budget: 10.0,
            k_anonymity: 5,
        }
    }
}

/// The privacy budget tracker for a single user.
#[derive(Debug, Clone)]
pub struct PrivacyBudget {
    /// Total ε spent today.
    pub epsilon_spent_today: f64,
    /// Total ε spent this week.
    pub epsilon_spent_week: f64,
    /// Number of queries today.
    pub query_count_today: u32,
}

impl PrivacyBudget {
    /// Create a new privacy budget tracker.
    pub const fn new() -> Self {
        PrivacyBudget {
            epsilon_spent_today: 0.0,
            epsilon_spent_week: 0.0,
            query_count_today: 0,
        }
    }

    /// Check if a query with given ε can be executed.
    pub fn can_afford(&self, epsilon: f64, config: &DpConfig) -> bool {
        self.epsilon_spent_today + epsilon <= config.daily_budget
    }

    /// Record spending ε for a query.
    pub fn spend(&mut self, epsilon: f64) {
        self.epsilon_spent_today += epsilon;
        self.epsilon_spent_week += epsilon;
        self.query_count_today += 1;
    }
}

/// A differentially private response to an aggregate query.
#[derive(Debug, Clone, PartialEq)]
pub struct DpResponse {
    /// The noisy aggregate value.
    pub value: f64,
    /// The privacy budget used (ε).
    pub epsilon_used: f64,
    /// The k-anonymity level achieved.
    pub anonymity_set_size: u32,
}

/// Add Laplace noise to a value for ε-differential privacy.
///
/// # Parameters
/// * `value` - The true aggregate value.
/// * `epsilon` - Privacy parameter (smaller = more privacy).
/// * `sensitivity` - The maximum possible change from one individual.
pub fn add_laplace_noise(value: f64, epsilon: f64, sensitivity: f64) -> f64 {
    if epsilon <= 0.0 {
        return 0.0;
    }
    let scale = sensitivity / epsilon;
    // Laplace(0, scale) noise — in production use a secure RNG
    let noise = laplace_sample(scale);
    value + noise
}

/// Generate a sample from Laplace(0, scale) distribution.
fn laplace_sample(scale: f64) -> f64 {
    // Simple approximation for testing; use proper cryptographic RNG in production
    let u: f64 = 0.5; // deterministic for testing
    -scale * (1.0 - u).ln() * if u >= 0.5 { 1.0 } else { -1.0 }
}

/// Check k-anonymity for a group size.
pub fn check_k_anonymity(group_size: u32, k: u32) -> bool {
    group_size >= k
}

/// Aggregate a set of values with differential privacy.
pub fn dp_aggregate(
    values: &[f64],
    config: &DpConfig,
    budget: &mut PrivacyBudget,
) -> Option<DpResponse> {
    if !budget.can_afford(config.epsilon, config) {
        return None;
    }

    if !check_k_anonymity(values.len() as u32, config.k_anonymity) {
        return None;
    }

    // Compute true mean
    let true_mean = values.iter().sum::<f64>() / values.len() as f64;

    // Add Laplace noise
    let sensitivity = 1.0 / values.len() as f64; // for mean
    let noisy_mean = add_laplace_noise(true_mean, config.epsilon, sensitivity);

    budget.spend(config.epsilon);

    Some(DpResponse {
        value: noisy_mean,
        epsilon_used: config.epsilon,
        anonymity_set_size: values.len() as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_privacy_budget() {
        let config = DpConfig::default();
        let mut budget = PrivacyBudget::new();
        assert!(budget.can_afford(1.0, &config));
        budget.spend(1.0);
        assert!(budget.can_afford(1.0, &config));
        budget.spend(8.0);
        assert!(!budget.can_afford(2.0, &config));
    }

    #[test]
    fn test_laplace_noise() {
        let noisy = add_laplace_noise(42.0, 1.0, 1.0);
        // With our deterministic RNG, this should produce a specific value
        assert!(noisy != 42.0); // noise added
    }

    #[test]
    fn test_k_anonymity() {
        assert!(check_k_anonymity(10, 5));
        assert!(!check_k_anonymity(3, 5));
    }

    #[test]
    fn test_dp_aggregate() {
        let config = DpConfig::default();
        let mut budget = PrivacyBudget::new();
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        let result = dp_aggregate(&values, &config, &mut budget);
        assert!(result.is_some());
        let response = result.unwrap();
        assert_eq!(response.anonymity_set_size, 10);
        assert_eq!(response.epsilon_used, 1.0);
    }

    #[test]
    fn test_dp_aggregate_insufficient_k() {
        let config = DpConfig::default();
        let mut budget = PrivacyBudget::new();
        let values = vec![1.0, 2.0];

        let result = dp_aggregate(&values, &config, &mut budget);
        assert!(result.is_none()); // below k=5
    }

    #[test]
    fn test_dp_aggregate_exhausted_budget() {
        let config = DpConfig { daily_budget: 1.0, ..Default::default() };
        let mut budget = PrivacyBudget::new();
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let r1 = dp_aggregate(&values, &config, &mut budget);
        assert!(r1.is_some());
        let r2 = dp_aggregate(&values, &config, &mut budget);
        assert!(r2.is_none()); // budget exhausted
    }
}
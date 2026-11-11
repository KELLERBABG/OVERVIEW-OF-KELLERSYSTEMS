/// Reputation decay and reactivation utilities.
///
/// Handles score decay over inactivity and the reactivation process
/// where nodes must be re-vouched to regain reputation.

use sybil_core::constants::*;

/// Calculate effective days inactive based on last active timestamp.
pub fn days_since_last_active(last_active: chrono::DateTime<chrono::Utc>) -> i64 {
    let now = chrono::Utc::now();
    let duration = now - last_active;
    duration.num_days()
}

/// Determine if a node needs reactivation.
pub fn needs_reactivation(days_inactive: i64) -> bool {
    days_inactive > MAX_INACTIVITY_DAYS
}

/// Calculate the decay multiplier for a score given inactivity.
pub fn decay_multiplier(days_inactive: i64, decay_rate_per_day: f64) -> f64 {
    (1.0 - decay_rate_per_day * days_inactive as f64).max(0.0)
}

/// Calculate the number of new anchor vouches needed for reactivation.
pub fn anchor_vouches_needed_for_reactivation(days_inactive: i64) -> u32 {
    if days_inactive <= MAX_INACTIVITY_DAYS {
        0
    } else {
        let extra_periods = (days_inactive - MAX_INACTIVITY_DAYS) / 30;
        (extra_periods as u32 + 1).min(MIN_ANCHOR_VOUCHES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_days_since_last_active() {
        let old = Utc::now() - Duration::days(30);
        let days = days_since_last_active(old);
        assert!(days >= 29 && days <= 31);
    }

    #[test]
    fn test_needs_reactivation() {
        assert!(needs_reactivation(MAX_INACTIVITY_DAYS + 1));
        assert!(!needs_reactivation(MAX_INACTIVITY_DAYS));
    }

    #[test]
    fn test_decay_multiplier() {
        let m = decay_multiplier(30, 0.001);
        assert!((m - 0.97).abs() < 0.01);
    }

    #[test]
    fn test_anchor_vouches_needed() {
        assert_eq!(anchor_vouches_needed_for_reactivation(90), 0);
        assert!(anchor_vouches_needed_for_reactivation(200) > 0);
    }
}
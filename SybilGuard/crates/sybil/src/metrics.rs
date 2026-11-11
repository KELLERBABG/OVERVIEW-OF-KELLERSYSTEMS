use sybil_core::{SybilResult, ValidationMetrics};

/// Compute validation metrics comparing detection results against ground truth.
pub fn compute_validation_metrics(
    ground_truth: &[(sybil_core::NodeId, bool)],
    predictions: &[(sybil_core::NodeId, bool, f64)],
) -> SybilResult<ValidationMetrics> {
    let mut tp = 0u64;
    let mut fp = 0u64;
    let mut tn = 0u64;
    let mut false_neg = 0u64;

    for (node_id, actual_is_sybil) in ground_truth {
        let predicted = predictions.iter().find(|(id, _, _)| id == node_id);
        match (actual_is_sybil, predicted) {
            (true, Some((_, pred, _))) if *pred => tp += 1,
            (true, Some((_, pred, _))) if !*pred => false_neg += 1,
            (true, None) => false_neg += 1,
            (false, Some((_, pred, _))) if *pred => fp += 1,
            (false, Some((_, pred, _))) if !*pred => tn += 1,
            (false, None) => tn += 1,
            _ => {}
        }
    }

    let total = tp + fp + tn + false_neg;
    let accuracy = if total > 0 {
        (tp + tn) as f64 / total as f64
    } else {
        0.0
    };
    let precision = if tp + fp > 0 {
        tp as f64 / (tp + fp) as f64
    } else {
        0.0
    };
    let recall = if tp + false_neg > 0 {
        tp as f64 / (tp + false_neg) as f64
    } else {
        0.0
    };
    let f1_score = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    Ok(ValidationMetrics {
        true_positives: tp,
        false_positives: fp,
        true_negatives: tn,
        false_negatives: false_neg,
        accuracy,
        precision,
        recall,
        f1_score,
        detection_latency_ms: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sybil_core::NodeId;

    #[test]
    fn test_perfect_detection() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let ground_truth = vec![(n1.clone(), true), (n2.clone(), false)];
        let predictions = vec![(n1, true, 0.95), (n2, false, 0.1)];
        let metrics = compute_validation_metrics(&ground_truth, &predictions).unwrap();
        assert_eq!(metrics.accuracy, 1.0);
        assert_eq!(metrics.precision, 1.0);
        assert_eq!(metrics.recall, 1.0);
        assert_eq!(metrics.f1_score, 1.0);
    }

    #[test]
    fn test_no_detection() {
        let n1 = NodeId::new();
        let ground_truth = vec![(n1.clone(), true)];
        let predictions = vec![(n1, false, 0.2)];
        let metrics = compute_validation_metrics(&ground_truth, &predictions).unwrap();
        assert_eq!(metrics.true_positives, 0);
        assert_eq!(metrics.false_negatives, 1);
    }

    #[test]
    fn test_false_positive() {
        let n1 = NodeId::new();
        let ground_truth = vec![(n1.clone(), false)];
        let predictions = vec![(n1, true, 0.9)];
        let metrics = compute_validation_metrics(&ground_truth, &predictions).unwrap();
        assert_eq!(metrics.false_positives, 1);
        assert_eq!(metrics.true_negatives, 0);
    }
}
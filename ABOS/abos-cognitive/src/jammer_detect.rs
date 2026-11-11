/// Jammer type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JammerType {
    CwTone,
    SweptChirp,
    BarrageNoise,
}

/// Classifies jammer type from a power spectral density vector using
/// peak-to-average power ratio (PAPR) and spectral entropy.
pub struct JammerClassifier {
    /// Minimum total power (sum of bins) to consider jamming present.
    pub power_threshold: f64,
}

impl JammerClassifier {
    pub fn new() -> Self {
        Self { power_threshold: 1e-6 }
    }

    /// Returns `Some(JammerType)` if a jammer is detected in the PSD, else `None`.
    pub fn classify(&self, psd: &[f64]) -> Option<JammerType> {
        let total: f64 = psd.iter().sum();
        if total < self.power_threshold {
            return None;
        }

        let papr = Self::compute_papr(psd);
        let entropy_ratio = Self::compute_entropy_ratio(psd);
        let bandwidth = Self::effective_bandwidth(psd);
        let n = psd.len();

        if papr > 5.0 && bandwidth <= 3 {
            Some(JammerType::CwTone)
        } else if papr < 2.0 && entropy_ratio > 0.7 {
            Some(JammerType::BarrageNoise)
        } else if n >= 4 {
            Some(JammerType::SweptChirp)
        } else {
            None
        }
    }

    /// Peak-to-average power ratio: max(psd) / mean(psd).
    fn compute_papr(psd: &[f64]) -> f64 {
        let peak = psd.iter().cloned().fold(f64::MIN, f64::max);
        let mean = psd.iter().sum::<f64>() / psd.len() as f64;
        if mean > 0.0 {
            peak / mean
        } else {
            0.0
        }
    }

    /// Normalized Shannon entropy ratio in [0, 1].
    /// 0 = all power in one bin, 1 = perfectly uniform.
    fn compute_entropy_ratio(psd: &[f64]) -> f64 {
        let total: f64 = psd.iter().sum();
        if total <= 0.0 {
            return 0.0;
        }
        let n = psd.len() as f64;
        let h_max = n.log2();
        if h_max <= 0.0 {
            return 0.0;
        }

        let mut entropy = 0.0f64;
        for &p in psd {
            if p > 0.0 {
                let prob = p / total;
                entropy -= prob * prob.log2();
            }
        }
        (entropy / h_max).clamp(0.0, 1.0)
    }

    /// Number of bins whose power is at least 50% of the peak.
    fn effective_bandwidth(psd: &[f64]) -> usize {
        let peak = psd.iter().cloned().fold(f64::MIN, f64::max);
        let threshold = 0.5 * peak;
        psd.iter().filter(|&&p| p >= threshold).count()
    }
}

impl Default for JammerClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Legacy helper: returns indices of bins exceeding `threshold`.
pub fn detect_jammer(power_spectrum: &[f64], threshold: f64) -> Vec<usize> {
    power_spectrum
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p > threshold { Some(i) } else { None })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cw_tone() {
        let mut psd = vec![0.001f64; 64];
        psd[20] = 10.0; // narrow peak
        let clf = JammerClassifier::new();
        assert_eq!(clf.classify(&psd), Some(JammerType::CwTone));
    }

    #[test]
    fn detects_barrage_noise() {
        let psd = vec![1.0f64; 64]; // uniform high power
        let clf = JammerClassifier::new();
        assert_eq!(clf.classify(&psd), Some(JammerType::BarrageNoise));
    }

    #[test]
    fn detects_swept_chirp() {
        let mut psd = vec![0.001f64; 64];
        for i in 25..45 {
            psd[i] = 5.0; // broad elevated band
        }
        let clf = JammerClassifier::new();
        assert_eq!(clf.classify(&psd), Some(JammerType::SweptChirp));
    }

    #[test]
    fn no_jammer_on_quiet_spectrum() {
        let psd = vec![1e-9f64; 64];
        let clf = JammerClassifier::new();
        assert_eq!(clf.classify(&psd), None);
    }
}

use abos_common::types::MCS;

use crate::jammer_detect::{JammerClassifier, JammerType};
use crate::whitespace::WhitespaceFinder;

/// Result of a cognitive adaptation cycle.
#[derive(Debug, Clone)]
pub struct AdaptationResult {
    pub jammer_detected: bool,
    pub jammer_type: Option<JammerType>,
    pub new_carrier_bin: usize,
    pub new_spreading_factor: u32,
    pub mcs: MCS,
}

/// Cognitive radio controller that automatically switches carrier frequency
/// and spreading factor when a jammer is detected.
pub struct CognitiveController {
    pub carrier_bin: usize,
    pub spreading_factor: u32,
    pub current_mcs: MCS,
    classifier: JammerClassifier,
    finder: WhitespaceFinder,
    num_bins: usize,
}

impl CognitiveController {
    pub fn new(num_bins: usize, initial_carrier_bin: usize) -> Self {
        let sub_bands = (num_bins / 8).max(2);
        Self {
            carrier_bin: initial_carrier_bin,
            spreading_factor: 7,
            current_mcs: MCS::Qpsk12,
            classifier: JammerClassifier::new(),
            finder: WhitespaceFinder::new(sub_bands),
            num_bins,
        }
    }

    /// Runs one adaptation cycle on the given PSD.
    /// If a jammer is detected, hops to the cleanest sub-band and increases
    /// spreading factor for additional processing gain.
    pub fn process(&mut self, psd: &[f64], snr: f64, ber: f64) -> AdaptationResult {
        let jammer_type = self.classifier.classify(psd);

        let (new_carrier, new_sf) = match jammer_type {
            Some(jt) => {
                let ranked = self.finder.rank(psd);
                // Pick the cleanest sub-band that is NOT the current one.
                let target = ranked
                    .iter()
                    .find(|sb| sb.start_bin != self.carrier_bin)
                    .or_else(|| ranked.first())
                    .map(|sb| (sb.start_bin + sb.end_bin) / 2)
                    .unwrap_or(0);

                // Increase SF for more processing gain (cap at 31).
                let sf = (self.spreading_factor * 2).min(31);
                (target, sf)
            }
            None => {
                // No jammer: can reduce SF for higher throughput.
                let sf = (self.spreading_factor / 2).max(7);
                (self.carrier_bin, sf)
            }
        };

        self.carrier_bin = new_carrier;
        self.spreading_factor = new_sf;
        self.current_mcs = Self::select_mcs(snr, ber);

        AdaptationResult {
            jammer_detected: jammer_type.is_some(),
            jammer_type,
            new_carrier_bin: self.carrier_bin,
            new_spreading_factor: self.spreading_factor,
            mcs: self.current_mcs,
        }
    }

    fn select_mcs(snr: f64, ber: f64) -> MCS {
        if ber > 0.1 || snr < 5.0 {
            MCS::Bpsk12
        } else if ber > 0.05 || snr < 10.0 {
            MCS::Qpsk12
        } else if ber > 0.01 || snr < 15.0 {
            MCS::Qpsk34
        } else if ber > 0.005 || snr < 20.0 {
            MCS::Qam1612
        } else if ber > 0.001 || snr < 25.0 {
            MCS::Qam1634
        } else {
            MCS::Qam6434
        }
    }
}

/// Legacy adaptive MCS controller (kept for backward compatibility).
pub struct AdaptiveController {
    pub current_mcs: MCS,
    pub current_symbol_rate: f64,
}

impl AdaptiveController {
    pub fn new(initial_mcs: MCS, symbol_rate: f64) -> Self {
        Self {
            current_mcs: initial_mcs,
            current_symbol_rate: symbol_rate,
        }
    }

    pub fn adapt_to_channel(&mut self, snr: f64, ber: f64) -> MCS {
        if ber > 0.1 || snr < 5.0 {
            self.current_mcs = MCS::Bpsk12;
        } else if ber > 0.05 || snr < 10.0 {
            self.current_mcs = MCS::Qpsk12;
        } else if ber > 0.01 || snr < 15.0 {
            self.current_mcs = MCS::Qpsk34;
        } else if ber > 0.005 || snr < 20.0 {
            self.current_mcs = MCS::Qam1612;
        } else if ber > 0.001 || snr < 25.0 {
            self.current_mcs = MCS::Qam1634;
        } else {
            self.current_mcs = MCS::Qam6434;
        }
        self.current_mcs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jammer_detect::JammerType;

    /// Simulate a CW tone jammer at bin 10 in a 64-bin PSD.
    fn psd_with_tone_jammer() -> Vec<f64> {
        let mut psd = vec![0.001f64; 64];
        psd[10] = 50.0;
        psd
    }

    /// Simulate a swept-chirp jammer spanning bins 30–50.
    fn psd_with_sweep_jammer() -> Vec<f64> {
        let mut psd = vec![0.001f64; 64];
        for i in 30..50 {
            psd[i] = 8.0;
        }
        psd
    }

    #[test]
    fn evades_tone_jammer() {
        let mut ctrl = CognitiveController::new(64, 10); // carrier on jammed bin
        let psd = psd_with_tone_jammer();
        let result = ctrl.process(&psd, 3.0, 0.2);

        assert!(result.jammer_detected);
        assert_eq!(result.jammer_type, Some(JammerType::CwTone));
        // Must have hopped away from bin 10.
        assert_ne!(ctrl.carrier_bin, 10);
        // Spreading factor increased for processing gain.
        assert!(ctrl.spreading_factor > 7);
    }

    #[test]
    fn evades_sweep_jammer() {
        let mut ctrl = CognitiveController::new(64, 35); // carrier inside sweep band
        let psd = psd_with_sweep_jammer();
        let result = ctrl.process(&psd, 4.0, 0.15);

        assert!(result.jammer_detected);
        assert_eq!(result.jammer_type, Some(JammerType::SweptChirp));
        // Hopped to a cleaner region.
        assert_ne!(ctrl.carrier_bin, 35);
        assert!(ctrl.spreading_factor > 7);
    }

    #[test]
    fn no_action_on_clean_spectrum() {
        let mut ctrl = CognitiveController::new(64, 5);
        let psd = vec![1e-9f64; 64];
        let result = ctrl.process(&psd, 20.0, 0.001);

        assert!(!result.jammer_detected);
        assert_eq!(ctrl.carrier_bin, 5); // unchanged
    }
}

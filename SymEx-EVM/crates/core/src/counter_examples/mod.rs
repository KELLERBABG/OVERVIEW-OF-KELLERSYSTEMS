//! Counter-Example Generation
//!
//! When a vulnerability is found, synthesizes exact transaction inputs
//! (calldata, callvalue) that trigger the exploit. Uses SMT solver models
//! to extract concrete values for all symbolic variables along the
//! exploit path.

use crate::smt::{SmtModelValue, SmtSolver, SmtResult};

/// A synthesized exploit input that triggers a vulnerability.
#[derive(Debug, Clone)]
pub struct CounterExample {
    /// The calldata (ABI-encoded function call with arguments).
    pub calldata: Vec<u8>,
    /// The value (in wei) to send with the transaction.
    pub callvalue: Vec<u8>,
    /// Human-readable description of the exploit.
    pub description: String,
    /// Gas limit for the exploit transaction.
    pub gas_limit: u64,
}

/// Generates counter-examples from SMT solver models.
#[derive(Debug, Clone)]
pub struct CounterExampleGenerator {
    /// Reference to the SMT solver.
    pub solver: SmtSolver,
}

impl CounterExampleGenerator {
    /// Create a new counter-example generator.
    pub fn new(solver: SmtSolver) -> Self {
        Self { solver }
    }

    /// Generate a counter-example from an SMT model.
    ///
    /// Takes the satisfying assignment from the solver and reconstructs
    /// concrete calldata that triggers the vulnerable path.
    pub fn generate_from_model(
        &self,
        model: &[SmtModelValue],
        function_selector: Option<[u8; 4]>,
    ) -> CounterExample {
        let mut calldata = Vec::new();

        // Prepend function selector if known
        if let Some(sel) = function_selector {
            calldata.extend_from_slice(&sel);
        }

        // Extract calldata values from the model, ABI-encode them
        for entry in model {
            if entry.variable.starts_with("sym_") {
                // Parse the hex value and append as 32-byte word
                if let Ok(decoded) = hex::decode(entry.value.trim_start_matches("#x")) {
                    let mut word = vec![0u8; 32];
                    let len = decoded.len().min(32);
                    word[32 - len..].copy_from_slice(&decoded[..len]);
                    calldata.extend_from_slice(&word);
                }
            }
        }

        CounterExample {
            calldata,
            callvalue: vec![0u8; 32], // default: 0 wei
            description: "Automatically generated exploit input".to_string(),
            gas_limit: 1_000_000,
        }
    }

    /// Format a counter-example as a human-readable report.
    pub fn format_report(&self, example: &CounterExample) -> String {
        format!(
            "Counter-Example Report\n\
             =====================\n\
             Description: {}\n\
             Calldata (hex): 0x{}\n\
             Callvalue (hex): 0x{}\n\
             Gas Limit: {}\n",
            example.description,
            hex::encode(&example.calldata),
            hex::encode(&example.callvalue),
            example.gas_limit,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smt::SmtConfig;

    #[test]
    fn test_generate_from_empty_model() {
        let solver = SmtSolver::new(SmtConfig::default());
        let gen = CounterExampleGenerator::new(solver);
        let example = gen.generate_from_model(&[], Some([0xaa, 0xbb, 0xcc, 0xdd]));
        assert_eq!(example.calldata[..4], [0xaa, 0xbb, 0xcc, 0xdd]);
    }

    #[test]
    fn test_format_report() {
        let solver = SmtSolver::new(SmtConfig::default());
        let gen = CounterExampleGenerator::new(solver);
        let example = CounterExample {
            calldata: vec![0xde, 0xad, 0xbe, 0xef],
            callvalue: vec![0; 32],
            description: "test exploit".to_string(),
            gas_limit: 100_000,
        };
        let report = gen.format_report(&example);
        assert!(report.contains("test exploit"));
        assert!(report.contains("deadbeef"));
    }
}
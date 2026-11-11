//! Hardware acceleration roadmap for zk-SNARK proving.
//!
//! ## Current Status
//! - CPU proving: ✅ Implemented (ark-groth16)
//! - GPU acceleration: ❌ Not yet implemented (CUDA/Metal)
//! - FPGA acceleration: ❌ Not yet implemented
//! - Remote proving: ❌ Not yet implemented (gRPC endpoint)
//!
//! ## Roadmap
//! 1. **GPU (Q3 2026)**: Offload MSM operations to GPU via CUDA.
//!    Estimated speedup: 10-50x for multi-scalar multiplication.
//! 2. **FPGA (Q4 2026)**: Custom NTT accelerator for proof generation.
//!    Estimated speedup: 5-20x for NTT-heavy operations.
//! 3. **Remote Prover (Q1 2027)**: Dedicated proving cluster with gRPC API.
//!    Enables mobile/thin clients to submit proofs.
//!
//! ## Integration Points
//! - `HardwareAccelerator` trait: Implement for each backend
//! - `ProvingStrategy` enum: Configure at startup
//! - `ProvingService`: Async proof queue with configurable strategy

use crate::circuits::BalanceCircuit;
use ark_bn254::Fr;
use tokio::sync::mpsc;

/// Strategy for proof generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvingStrategy {
    /// CPU-based proving (default, no acceleration)
    Cpu,
    /// GPU acceleration (CUDA for NVIDIA, Metal for Apple)
    Gpu,
    /// FPGA acceleration
    Fpga,
    /// Remote proving service (RPC to dedicated prover)
    Remote,
}

/// Hardware accelerator trait for offloading MSM/NTT operations
pub trait HardwareAccelerator: Send + Sync {
    /// Accelerate multi-scalar multiplication
    fn msm(&self, scalars: &[Fr], bases: &[Fr]) -> Result<Fr, String>;
    /// Accelerate number-theoretic transform
    fn ntt(&self, values: &mut [Fr]) -> Result<(), String>;
    /// Check if accelerator is available
    fn is_available(&self) -> bool;
    /// Name of the accelerator (for logging)
    fn name(&self) -> &'static str;
}

/// CPU fallback — no acceleration
pub struct CpuAccelerator;

impl HardwareAccelerator for CpuAccelerator {
    fn msm(&self, _scalars: &[Fr], _bases: &[Fr]) -> Result<Fr, String> {
        Err("MSM not available on CPU".to_string())
    }
    fn ntt(&self, _values: &mut [Fr]) -> Result<(), String> {
        Err("NTT not available on CPU".to_string())
    }
    fn is_available(&self) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "cpu"
    }
}

/// Status of a proof generation request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofStatus {
    Queued,
    Proving,
    Completed,
    Failed(String),
}

/// A request to generate a proof
pub struct ProofRequest {
    pub request_id: u64,
    pub circuit: BalanceCircuit,
    pub result_tx: tokio::sync::oneshot::Sender<Result<crate::verifier::Proof, String>>,
}

/// Async proving service that queues and processes proofs
pub struct ProvingService {
    request_tx: mpsc::Sender<ProofRequest>,
}

impl ProvingService {
    /// Create a new proving service with the given strategy.
    /// Spawns a background task that processes proofs sequentially.
    pub fn new(strategy: ProvingStrategy, queue_size: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<ProofRequest>(queue_size);
        tokio::spawn(async move {
            while let Some(req) = rx.recv().await {
                tracing::info!(
                    "[ProvingService] Proving request {} with strategy {:?}",
                    req.request_id,
                    strategy
                );
                let circuit = req.circuit;
                let result_tx = req.result_tx;
                let result = tokio::task::spawn_blocking(move || {
                    let prover = crate::prover::Prover::new();
                    prover.generate_proof_inner(&circuit)
                })
                .await;
                match result {
                    Ok(Ok(proof)) => {
                        let _ = result_tx.send(Ok(proof));
                    }
                    Ok(Err(e)) => {
                        let _ = result_tx.send(Err(e));
                    }
                    Err(e) => {
                        let _ = result_tx.send(Err(format!("task join error: {e}")));
                    }
                }
            }
        });
        Self { request_tx: tx }
    }

    /// Submit a proof request and get a oneshot receiver for the result
    pub async fn submit(&self, circuit: BalanceCircuit) -> Result<crate::verifier::Proof, String> {
        static REQUEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let req = ProofRequest {
            request_id: id,
            circuit,
            result_tx: tx,
        };
        self.request_tx
            .send(req)
            .await
            .map_err(|_| "proving service channel closed".to_string())?;
        rx.await.map_err(|_| "prover dropped".to_string())?
    }
}

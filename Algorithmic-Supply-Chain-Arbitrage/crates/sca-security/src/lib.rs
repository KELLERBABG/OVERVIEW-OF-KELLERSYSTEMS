//! Platform security primitives.

pub mod build_verify;
pub mod capabilities;
pub mod ipc;
pub mod memory_encryption;
pub mod panic_zero;
pub mod temporal;
pub mod zk_auth;

pub use capabilities::CapabilityManager;
pub use build_verify::BuildVerifier;
pub use ipc::IpcChannel;
pub use memory_encryption::MemoryEncryptor;
pub use panic_zero::{install_panic_zeroing, SensitiveBuffer};
pub use temporal::TemporalScheduler;
pub use zk_auth::{ZkProver, ZkVerifier, zk_authenticate};

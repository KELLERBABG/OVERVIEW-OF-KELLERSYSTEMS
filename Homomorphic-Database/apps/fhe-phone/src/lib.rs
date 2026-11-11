//! # fhe-phone — FHE-DB as an Android-style phone
//!
//! A native `.exe` that brings the whole FHE-DB stack to life inside a
//! phone shell:
//!
//! - A **real embedded [`fhe_server`]** runs on a private loopback port.
//! - A **real [`fhe_client::FheClient`]** drives it over HTTP: keygen,
//!   server-key upload, encryption, homomorphic SUM/AVG/COUNT, encrypted
//!   read-back, and encrypted XNOR inference.
//! - A **wry/tao webview** renders a Material-You Android shell with six apps
//!   (Vault, Wallet, Health, Messages, Neural, Monitor, Settings).
//!
//! Nothing is simulated: every number the phone displays was encrypted on the
//! device, aggregated homomorphically by the embedded server, and decrypted
//! only back on the device. The Monitor app shows the live request feed; it
//! records method/path/status only, not bodies, so it is not payload proof.

pub mod engine;
pub mod keystore;
pub mod server;
pub mod shell;
pub mod wire;

pub use engine::{dispatch, Engine, SharedEngine};
pub use keystore::{KeyStore, PinCheck, StoredKeys, Unlock};
pub use wire::{WireRecord, WireRing};

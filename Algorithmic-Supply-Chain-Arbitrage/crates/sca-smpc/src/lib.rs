//! # sca-smpc
//!
//! Secure Multi-Party Computation (SMPC) Price Discovery.
//!
//! Implements Shamir's Secret Sharing over a finite field to enable
//! computation of the fair market clearing price from encrypted
//! shares of bid and ask data — without any party revealing their
//! internal calculations.
//!
//! ## How it works
//!
//! 1. **Secret Sharing**: Each party splits their private value
//!    (bid price, ask price, quantity) into N shares using a
//!    random polynomial of degree K-1.
//!
//! 2. **Share Distribution**: Each share is sent to a different
//!    compute node. No single node sees enough shares to reconstruct.
//!
//! 3. **Homomorphic Computation**: Nodes compute on shares locally.
//!    Sum of shares = shares of sum (additive homomorphism).
//!
//! 4. **Reconstruction**: K nodes combine their results to reveal
//!    the final market clearing price.
//!
//! ## Key types
//!
//! - **`FiniteField`** — Arithmetic in Z_p for polynomial operations
//! - **`ShamirSharing`** — Split and reconstruct secrets
//! - **`SecretShare`** — A single share of a secret
//! - **`PriceDiscoveryProtocol`** — The full multi-party protocol
//! - **`BidMessage`/`AskMessage`** — Encrypted bid/ask data

pub mod field;
pub mod protocol;
pub mod shamir;

pub use field::FiniteField;
pub use protocol::{AskMessage, BidMessage, MarketClearingPrice, PriceDiscoveryProtocol};
pub use shamir::{SecretShare, ShamirSharing};

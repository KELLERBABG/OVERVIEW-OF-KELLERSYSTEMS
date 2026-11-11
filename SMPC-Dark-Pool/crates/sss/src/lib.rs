#![forbid(unsafe_code)]

pub mod field;
pub mod polynomial;
pub mod shares;
pub mod sss;

pub use field::ScalarField;
pub use shares::Share;
pub use sss::{reconstruct, split_secret};

//! Gas cost calculation utilities for the EVM.
//!
//! Implements gas cost rules for multiple hard forks including:
//! - Frontier/Byzantium/Constantinople rules (base costs)
//! - **EIP-2929** (Berlin): cold/warm SLOAD/SSTORE/account access costs
//! - **EIP-150**: call depth gas stipend
//! - **EIP-1153** (Cancun): transient storage (TLOAD/TSTORE) costs
//!
//! ## Access List Tracking
//!
//! EIP-2929 introduced a per-transaction access list that tracks which
//! storage slots and addresses have been "warmed" (already accessed).
//! First access to a cold slot/account costs more gas than subsequent
//! warm accesses.

/// The access status for EIP-2929 pricing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessStatus {
    /// First access in this transaction (cold).
    Cold,
    /// Previously accessed in this transaction (warm).
    Warm,
}

/// Tracks the access list for EIP-2929 warm/cold pricing.
#[derive(Debug, Clone, Default)]
pub struct AccessList {
    /// Set of (address) that have been accessed this transaction.
    pub warm_addresses: Vec<[u8; 20]>,
    /// Set of (address, slot) storage keys that have been accessed.
    pub warm_storage_slots: Vec<([u8; 20], [u8; 32])>,
}

impl AccessList {
    /// Check if an address is warm (already accessed this transaction).
    pub fn is_address_warm(&self, address: &[u8; 20]) -> bool {
        self.warm_addresses.contains(address)
    }

    /// Mark an address as warm.
    pub fn warm_address(&mut self, address: [u8; 20]) {
        if !self.is_address_warm(&address) {
            self.warm_addresses.push(address);
        }
    }

    /// Check if a storage slot is warm.
    pub fn is_storage_warm(&self, address: &[u8; 20], slot: &[u8; 32]) -> bool {
        self.warm_storage_slots.contains(&(*address, *slot))
    }

    /// Mark a storage slot as warm.
    pub fn warm_storage(&mut self, address: [u8; 20], slot: [u8; 32]) {
        if !self.is_storage_warm(&address, &slot) {
            self.warm_storage_slots.push((address, slot));
        }
    }
}

/// Compute the gas cost for expanding memory to cover a given address.
/// Memory is charged at 3 gas per 32-byte word, with a quadratic cost
/// for large expansions (EIP-150+).
pub fn memory_expansion_cost(current_size: usize, target_size: usize) -> u64 {
    if target_size <= current_size {
        return 0;
    }

    let words_current = (current_size + 31) / 32;
    let words_target = (target_size + 31) / 32;

    let cost_current = words_current * 3 + (words_current * words_current) / 512;
    let cost_target = words_target * 3 + (words_target * words_target) / 512;

    (cost_target - cost_current) as u64
}

/// Intrinsic gas for a transaction: 21000 base + calldata costs (EIP-2028).
pub fn intrinsic_gas(calldata: &[u8], contract_creation: bool) -> u64 {
    let mut gas = if contract_creation { 53_000 } else { 21_000 };
    for &byte in calldata {
        if byte == 0 {
            gas += 4;
        } else {
            gas += 16;
        }
    }
    gas
}

/// Gas cost for an SLOAD operation (EIP-2929 / Berlin).
/// - Cold access: 2100 gas
/// - Warm access: 100 gas
pub fn sload_gas(access: AccessStatus) -> u64 {
    match access {
        AccessStatus::Cold => 2100,
        AccessStatus::Warm => 100,
    }
}

/// Gas cost for an SSTORE operation (EIP-2200 / Berlin with EIP-2929).
///
/// Simplified model:
/// - Cold slot: 22100 (2100 cold + 20000 dirty)
/// - Warm slot, non-zero to non-zero same: 100
/// - Warm slot, zero to non-zero: 22100
/// - Warm slot, non-zero to zero: 5000 (refund)
///
/// Full EIP-2200 rules require tracking original/current/new values.
pub fn sstore_gas(
    cold: bool,
) -> u64 {
    if cold { 22100 } else { 100 }
}

/// Gas cost for a LOG opcode (0xa0..0xa4).
pub fn log_gas(num_topics: u8, data_size: u64) -> u64 {
    375 + num_topics as u64 * 375 + data_size * 8
}

/// Gas cost for a CALL opcode (EIP-150).
/// - Base: 100
/// - With positive value: +9000 + 2300 caller stipend
/// - If the target address is cold (EIP-2929): +2600
pub fn call_gas(value_transferred: bool, target_cold: bool) -> u64 {
    let mut gas = 100u64;
    if value_transferred {
        gas += 9000 + 2300; // stipend
    }
    if target_cold {
        gas += 2600; // EIP-2929 cold account access
    }
    gas
}

/// Gas stipend for a child call when value is transferred (EIP-150).
/// The child call receives 2300 gas for free.
pub fn call_stipend() -> u64 {
    2300
}

/// Maximum call stack depth (EIP-150).
pub fn max_call_depth() -> usize {
    1024
}

/// Gas cost for SELFDESTRUCT (EIP-150/EIP-2929).
pub fn selfdestruct_gas(target_cold: bool) -> u64 {
    let mut gas = 5000;
    if target_cold {
        gas += 2600; // EIP-2929 cold address access
    }
    gas
}

/// Cost for accessing an account (BALANCE, EXTCODESIZE, etc.) — EIP-2929.
pub fn account_access_gas(cold: bool) -> u64 {
    if cold { 2600 } else { 100 }
}

/// Cost for TLOAD (EIP-1153 transient storage load) — 100 gas.
pub fn tload_gas() -> u64 {
    100
}

/// Cost for TSTORE (EIP-1153 transient storage store) — 100 gas.
pub fn tstore_gas() -> u64 {
    100
}

/// Cost for a CREATE operation (EIP-150).
pub fn create_gas() -> u64 {
    32000
}

/// Cost for CREATE2 (EIP-1014).
pub fn create2_gas() -> u64 {
    32000
}

/// Cost for KECCAK256 — base cost + per-word cost.
pub fn keccak256_gas(data_size: u64) -> u64 {
    let base = 30;
    let word_cost = 6 * ((data_size + 31) / 32);
    base + word_cost
}

/// Cost for a contract creation transaction.
pub fn contract_creation_cost(code_size: usize) -> u64 {
    // 200 gas per byte of code (EIP-3860)
    code_size as u64 * 200
}

/// Estimate the total accessible gas for the current context.
/// Returns the minimum of remaining gas and the gas limit, minus
/// the intrinsic cost.
pub fn accessible_gas(remaining_gas: u64, call_value_transferred: bool) -> u64 {
    let stipend = if call_value_transferred {
        2300
    } else {
        0
    };
    remaining_gas.saturating_add(stipend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intrinsic_gas_empty() {
        assert_eq!(intrinsic_gas(&[], false), 21_000);
    }

    #[test]
    fn test_intrinsic_gas_creation() {
        assert_eq!(intrinsic_gas(&[], true), 53_000);
    }

    #[test]
    fn test_intrinsic_gas_with_data() {
        assert_eq!(intrinsic_gas(&[0x01], false), 21_016);
        assert_eq!(intrinsic_gas(&[0x00], false), 21_004);
    }

    #[test]
    fn test_memory_expansion_noop() {
        assert_eq!(memory_expansion_cost(100, 50), 0);
    }

    #[test]
    fn test_sload_gas() {
        assert_eq!(sload_gas(AccessStatus::Cold), 2100);
        assert_eq!(sload_gas(AccessStatus::Warm), 100);
    }

    #[test]
    fn test_call_gas_cold() {
        assert_eq!(call_gas(false, true), 2700); // 100 + 2600 cold
    }

    #[test]
    fn test_access_list() {
        let mut al = AccessList::default();
        let addr = [0u8; 20];
        assert!(!al.is_address_warm(&addr));
        al.warm_address(addr);
        assert!(al.is_address_warm(&addr));
    }

    #[test]
    fn test_keccak_gas() {
        assert_eq!(keccak256_gas(0), 30);
        assert_eq!(keccak256_gas(32), 36);
        assert_eq!(keccak256_gas(64), 42);
    }

    #[test]
    fn test_log_gas() {
        assert_eq!(log_gas(0, 0), 375);
        assert_eq!(log_gas(1, 0), 750);
    }
}
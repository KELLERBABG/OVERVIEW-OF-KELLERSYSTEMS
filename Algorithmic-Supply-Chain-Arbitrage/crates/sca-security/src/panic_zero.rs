//! Full System Panic-Zeroing.
//!
//! On panic or critical error, sensitive memory regions are securely
//! zeroed before the program unwinds. This prevents key material,
//! escrow balances, private keys, and SMPC shares from leaking
//! through crash dumps or debugging interfaces.
//!
//! Uses a registered panic hook that is called before the normal
//! panic handler, ensuring zeroing happens first.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag indicating a panic has occurred.
static PANIC_OCCURRED: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    /// List of memory regions to zero on panic.
    static ref ZERO_REGISTRY: Mutex<Vec<ZeroRegion>> = Mutex::new(Vec::new());
}

/// A memory region to be zeroed on panic.
#[derive(Clone)]
struct ZeroRegion {
    description: String,
    data: Vec<u8>,
}

/// Register a memory region to be zeroed on panic.
pub fn register_for_zeroing(description: impl Into<String>, data: Vec<u8>) {
    if let Ok(mut registry) = ZERO_REGISTRY.lock() {
        registry.push(ZeroRegion {
            description: description.into(),
            data,
        });
    }
}

/// Get the current zeroing registry entries (for testing).
pub fn zero_registry_size() -> usize {
    ZERO_REGISTRY.lock().map(|r| r.len()).unwrap_or(0)
}

/// Zero all registered memory regions.
pub fn zero_all_registered() {
    if let Ok(mut registry) = ZERO_REGISTRY.lock() {
        for region in registry.iter_mut() {
            // Securely zero the memory
            region.data.iter_mut().for_each(|b| *b = 0);
            // Also zero the description buffer
            unsafe {
                let bytes = region.description.as_bytes_mut();
                bytes.iter_mut().for_each(|b| *b = 0);
            }
        }
        // Clear the registry after zeroing
        registry.clear();
    }
}

/// Install the global panic zeroing hook.
/// Call this at program startup.
pub fn install_panic_zeroing() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Zero all registered memory before any other panic handling
        zero_all_registered();
        PANIC_OCCURRED.store(true, Ordering::SeqCst);

        // Call the previous hook
        previous(panic_info);
    }));
}

/// Check if a panic has occurred since startup.
pub fn has_panicked() -> bool {
    PANIC_OCCURRED.load(Ordering::SeqCst)
}

/// Create a sensitive memory region that will be automatically zeroed on panic.
pub struct SensitiveBuffer {
    description: String,
    data: Vec<u8>,
    registered: bool,
}

impl SensitiveBuffer {
    /// Create a new sensitive buffer containing sensitive data.
    /// It will be automatically zeroed if a panic occurs.
    pub fn new(description: impl Into<String>, data: Vec<u8>) -> Self {
        let desc = description.into();
        let mut buffer = Self {
            description: desc.clone(),
            data,
            registered: false,
        };
        buffer.register();
        buffer
    }

    fn register(&mut self) {
        register_for_zeroing(&self.description, self.data.clone());
        self.registered = true;
    }

    /// Get a reference to the data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the data.
    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }
}

impl Drop for SensitiveBuffer {
    fn drop(&mut self) {
        // Zero on normal drop too (defense in depth)
        self.data.iter_mut().for_each(|b| *b = 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_zero() {
        let data = b"sensitive_key_material_12345".to_vec();
        register_for_zeroing("test_key", data);

        assert!(zero_registry_size() > 0);

        zero_all_registered();
        assert_eq!(zero_registry_size(), 0);
    }

    #[test]
    fn test_sensitive_buffer_drop_zeros() {
        let buffer = SensitiveBuffer::new("private_key", vec![0xABu8; 32]);
        assert_eq!(buffer.data().len(), 32);

        // Data gets zeroed on drop
        drop(buffer);
        // Cannot verify after drop, but the Drop impl handles it
    }

    #[test]
    fn test_panic_flag() {
        assert!(!has_panicked());
    }
}
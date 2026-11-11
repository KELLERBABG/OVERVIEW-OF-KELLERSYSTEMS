//! Property tests over the homomorphic algebra.
//!
//! These pin the algebraic invariants of [`fhe_core::CiphertextAlgebra`] over
//! random plaintexts: addition/multiplication must behave like their plaintext
//! counterparts after decryption (mod `2^width`). They run on 8-bit
//! ciphertexts with small value ranges so each case stays fast; the server
//! key is installed once per test (proptest runs on a single thread by
//! default, so the thread-local pin covers every case).

use fhe_core::{CiphertextAlgebra, ClientKey, Encryptor, ScalarWidth, ServerKeyGuard};
use proptest::prelude::*;

/// Value range kept well inside the 8-bit modulus so sums never wrap.
const SMALL: std::ops::RangeInclusive<u8> = 0..=100;

proptest! {
    #![proptest_config(ProptestConfig { cases: 16, ..ProptestConfig::default() })]

    /// `x + 0 == x` over random plaintexts.
    #[test]
    fn additive_identity(x in SMALL) {
        let (client_key, server_key) = ClientKey::new();
        let _guard = ServerKeyGuard::new(&server_key);
        let encryptor = Encryptor::new(&client_key);
        let algebra = CiphertextAlgebra;

        let a = encryptor.encrypt_u64(x as u64, ScalarWidth::U8).unwrap();
        let zero = encryptor.encrypt_u64(0, ScalarWidth::U8).unwrap();
        let sum = algebra.add(&a, &zero).unwrap();
        prop_assert_eq!(sum.decrypt(&client_key).as_u64(), x as u64);
    }

    /// `x * 1 == x` over random plaintexts.
    #[test]
    fn multiplicative_identity(x in SMALL) {
        let (client_key, server_key) = ClientKey::new();
        let _guard = ServerKeyGuard::new(&server_key);
        let encryptor = Encryptor::new(&client_key);
        let algebra = CiphertextAlgebra;

        let a = encryptor.encrypt_u64(x as u64, ScalarWidth::U8).unwrap();
        let one = encryptor.encrypt_u64(1, ScalarWidth::U8).unwrap();
        let product = algebra.mul(&a, &one).unwrap();
        prop_assert_eq!(product.decrypt(&client_key).as_u64(), x as u64);
    }

    /// `a + b` decrypts to the plaintext sum (no wrap: `a+b <= 200 < 256`).
    #[test]
    fn sum_of_two_is_plaintext_sum(a in SMALL, b in SMALL) {
        let (client_key, server_key) = ClientKey::new();
        let _guard = ServerKeyGuard::new(&server_key);
        let encryptor = Encryptor::new(&client_key);
        let algebra = CiphertextAlgebra;

        let ca = encryptor.encrypt_u64(a as u64, ScalarWidth::U8).unwrap();
        let cb = encryptor.encrypt_u64(b as u64, ScalarWidth::U8).unwrap();
        let sum = algebra.add(&ca, &cb).unwrap();
        prop_assert_eq!(sum.decrypt(&client_key).as_u64(), (a + b) as u64);
    }

    /// Envelope deserialization is total: arbitrary bytes yield a typed
    /// error or a valid ciphertext — never a panic. This is the same
    /// invariant the cargo-fuzz target `fuzz_envelope` hammers with
    /// libFuzzer (see `fuzz/`); proptest pins it on hosts without clang.
    /// Inputs are capped at 4 KiB so a length-prefix bomb cannot cause an
    /// absurd allocation even if the validation regresses.
    #[test]
    fn envelope_deserialization_never_panics(
        bytes in proptest::collection::vec(any::<u8>(), 0..4096),
    ) {
        let _ = fhe_core::Ciphertext::from_envelope_bytes(&bytes);
    }
}

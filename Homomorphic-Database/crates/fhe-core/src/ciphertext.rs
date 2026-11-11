//! Encrypted values and the wire/storage envelope.
//!
//! The core challenge of a multi-width FHE layer is that TFHE-rs provides
//! **distinct Rust types** per bit width (`FheUint8`, `FheUint64`, ...).
//! Higher layers (a database schema, a REST API) want to handle ciphertexts
//! uniformly. This module solves that with:
//!
//! - [`Ciphertext`] — a width-tagged enum wrapping the concrete TFHE-rs
//!   ciphertext types.
//! - [`CiphertextEnvelope`] — a versioned frame around a serialized
//!   [`Ciphertext`] that is safe to store at rest and send over the wire.
//! - [`PlaintextValue`] — a width-tagged decrypted value.
//! - [`EncryptedColumn`] — a column of ciphertexts (the storage unit of
//!   `fhe-db`).

use serde::{Deserialize, Serialize};

use bincode::Options as _;

use crate::error::{FheError, FheResult};
use crate::keys::ClientKey;
use crate::ScalarWidth;

/// The current on-the-wire format version.
pub const CURRENT_ENVELOPE_VERSION: u8 = 1;

/// Hard cap on a single envelope accepted by
/// [`Ciphertext::from_envelope_bytes`].
///
/// Today's widest ciphertext (U64) serializes to roughly half a MiB; the cap
/// is a generous 64 MiB so a crafted envelope cannot force a giant
/// allocation. The server additionally enforces its own per-envelope cap
/// (see `fhe_server::routes::Limits::max_envelope_bytes`).
const MAX_ENVELOPE_BYTES: usize = 64 * 1024 * 1024;

/// A hand-parsed envelope header. Parsed *before* bincode touches the bytes
/// so a claimed `payload_len` can never drive an allocation.
struct EnvelopeHeader {
    /// Format version byte.
    version: u8,
    /// Declared width.
    width: ScalarWidth,
    /// Byte range of the payload within the envelope.
    payload: std::ops::Range<usize>,
}

/// Parse the envelope header. bincode's free-function framing is
/// little-endian fixed-width integers, so the layout is stable:
/// `version: u8`, `width: ScalarWidth` (u32 variant index), `payload_len:
/// u64`, then the payload.
///
/// # Errors
///
/// Returns [`FheError::MalformedEnvelope`] if the framing is invalid, the
/// width tag is unknown, or the claimed payload length exceeds the input.
fn parse_envelope_header(bytes: &[u8]) -> FheResult<EnvelopeHeader> {
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(FheError::MalformedEnvelope);
    }
    if bytes.len() < 13 {
        // version (1) + width tag (4) + payload length (8) must fit.
        return Err(FheError::MalformedEnvelope);
    }
    let version = bytes[0];
    let width_tag = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
    let width = match width_tag {
        0 => ScalarWidth::U8,
        1 => ScalarWidth::U16,
        2 => ScalarWidth::U32,
        3 => ScalarWidth::U64,
        _ => return Err(FheError::MalformedEnvelope),
    };
    let payload_len = u64::from_le_bytes([
        bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12],
    ]) as usize;
    let end = 13usize
        .checked_add(payload_len)
        .ok_or(FheError::MalformedEnvelope)?;
    if end > bytes.len() {
        return Err(FheError::MalformedEnvelope);
    }
    Ok(EnvelopeHeader {
        version,
        width,
        payload: 13..end,
    })
}

/// Validate the length prefixes at the top of a serialized [`Ciphertext`]
/// payload **before** bincode gets a chance to allocate from them.
///
/// serde's `Vec` visitor preallocates from the length prefix (clamped at
/// 1 GiB per sequence), so a crafted payload claiming e.g. 2^60 blocks
/// would otherwise force a huge transient allocation per request. The first
/// two prefixes sit at fixed offsets because `Ciphertext` is our own enum
/// and the wrapped `BaseRadixCiphertext` begins with its
/// `blocks: Vec<Block>` field:
///
/// - `payload[0..4]` — `Ciphertext` variant tag (u32, fixint LE)
/// - `payload[4..12]` — `blocks` length (u64, fixint LE); every block is at
///   least 8 bytes in the stream, so `blocks_len * 8 <= len - 12` must hold
/// - `payload[12..20]` — first block's LWE vector length (u64); every
///   element is a `u32`, so `ct_len * 4 <= len - 20` must hold
///
/// Deeper prefixes inside later blocks follow tfhe-internal layout and are
/// not validated here; they stay bounded by serde's 1 GiB per-sequence
/// clamp, which the server's envelope cap + rate limiter (and this crate's
/// fuzz harness) keep in check.
///
/// # Errors
///
/// Returns [`FheError::CorruptedCiphertext`] for implausible length
/// prefixes (length-prefix bombs).
fn check_payload_lengths(payload: &[u8]) -> FheResult<()> {
    if payload.len() < 12 {
        // Too short to carry even a length prefix; bincode rejects it.
        return Ok(());
    }
    let variant_tag = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    if variant_tag > 3 {
        // Unknown variant: bincode rejects it; nothing to validate.
        return Ok(());
    }
    let blocks_len = u64::from_le_bytes([
        payload[4],
        payload[5],
        payload[6],
        payload[7],
        payload[8],
        payload[9],
        payload[10],
        payload[11],
    ]);
    // Each block must occupy at least 8 bytes in the stream, so a valid
    // stream satisfies blocks_len * 8 <= remaining bytes.
    if blocks_len as usize > (payload.len() - 12) / 8 {
        return Err(FheError::CorruptedCiphertext(
            "implausible ciphertext block count (possible length-prefix bomb)".into(),
        ));
    }
    if blocks_len > 0 && payload.len() >= 20 {
        let ct_len = u64::from_le_bytes([
            payload[12],
            payload[13],
            payload[14],
            payload[15],
            payload[16],
            payload[17],
            payload[18],
            payload[19],
        ]);
        // Each LWE element is a u32, so a valid block satisfies
        // ct_len * 4 <= remaining bytes.
        if ct_len as usize > (payload.len() - 20) / 4 {
            return Err(FheError::CorruptedCiphertext(
                "implausible LWE vector length (possible length-prefix bomb)".into(),
            ));
        }
    }
    Ok(())
}

/// A width-tagged encrypted unsigned integer.
///
/// This is the "any ciphertext" type used throughout the rest of the
/// workspace. It wraps one of the four TFHE-rs concrete ciphertext types.
#[derive(Clone, Serialize, Deserialize)]
pub enum Ciphertext {
    /// 8-bit ciphertext
    U8(tfhe::FheUint8),
    /// 16-bit ciphertext
    U16(tfhe::FheUint16),
    /// 32-bit ciphertext
    U32(tfhe::FheUint32),
    /// 64-bit ciphertext
    U64(tfhe::FheUint64),
}

impl Ciphertext {
    /// The arithmetic width of this ciphertext.
    pub fn width(&self) -> ScalarWidth {
        match self {
            Ciphertext::U8(_) => ScalarWidth::U8,
            Ciphertext::U16(_) => ScalarWidth::U16,
            Ciphertext::U32(_) => ScalarWidth::U32,
            Ciphertext::U64(_) => ScalarWidth::U64,
        }
    }

    /// Decrypt this ciphertext with the client key.
    ///
    /// The returned [`PlaintextValue`] preserves the ciphertext's width so a
    /// caller that knows the schema can extract an exact value.
    pub fn decrypt(&self, client_key: &ClientKey) -> PlaintextValue {
        use tfhe::prelude::*;
        match self {
            Ciphertext::U8(ct) => PlaintextValue::U8(ct.decrypt(&client_key.inner)),
            Ciphertext::U16(ct) => PlaintextValue::U16(ct.decrypt(&client_key.inner)),
            Ciphertext::U32(ct) => PlaintextValue::U32(ct.decrypt(&client_key.inner)),
            Ciphertext::U64(ct) => PlaintextValue::U64(ct.decrypt(&client_key.inner)),
        }
    }

    /// Serialize this ciphertext into the versioned envelope byte format.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::Serialization`] if bincode cannot encode the
    /// underlying TFHE-rs ciphertext.
    pub fn to_envelope_bytes(&self) -> FheResult<Vec<u8>> {
        let payload = bincode::serialize(self)?;
        let envelope = CiphertextEnvelope {
            version: CURRENT_ENVELOPE_VERSION,
            width: self.width(),
            payload,
        };
        bincode::serialize(&envelope).map_err(Into::into)
    }

    /// Deserialize a ciphertext from the versioned envelope byte format.
    ///
    /// The envelope header is hand-parsed first so a claimed payload length
    /// can never drive an allocation, the payload's top-level length
    /// prefixes are validated against the input size (see
    /// [`check_payload_lengths`]), and the payload decode is bounded by the
    /// payload size.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::UnknownEnvelopeVersion`] for unsupported envelope
    /// versions, [`FheError::MalformedEnvelope`] for corrupt frames, and
    /// [`FheError::CorruptedCiphertext`] if the inner payload fails to
    /// deserialize or carries an implausible length prefix.
    pub fn from_envelope_bytes(bytes: &[u8]) -> FheResult<Self> {
        let header = parse_envelope_header(bytes)?;
        if header.version != CURRENT_ENVELOPE_VERSION {
            return Err(FheError::UnknownEnvelopeVersion(header.version));
        }

        let payload = &bytes[header.payload];
        check_payload_lengths(payload)?;

        // Bound the decode to the actual payload bytes and reject trailing
        // garbage so a crafted envelope cannot consume more than it carries.
        // `with_fixint_encoding` matches `bincode::serialize` (the free
        // function), which is what `to_envelope_bytes` uses to write.
        let parsed: Ciphertext = bincode::options()
            .with_fixint_encoding()
            .with_limit(payload.len() as u64)
            .reject_trailing_bytes()
            .deserialize(payload)
            .map_err(|e| FheError::CorruptedCiphertext(format!("failed to decode payload: {e}")))?;

        // Reject width mismatch before returning the ciphertext.
        if parsed.width() != header.width {
            return Err(FheError::WidthMismatch {
                expected: header.width.bits(),
                found: parsed.width().bits(),
            });
        }

        Ok(parsed)
    }
}

/// A versioned, self-describing frame for serialized ciphertexts.
///
/// The version byte lets future versions of the format be detected and
/// rejected cleanly. The width byte lets callers validate a ciphertext before
/// parsing its (potentially large) payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiphertextEnvelope {
    /// Format version ([`CURRENT_ENVELOPE_VERSION`]).
    pub version: u8,
    /// Arithmetic width of the wrapped ciphertext.
    pub width: ScalarWidth,
    /// Serialized [`Ciphertext`] payload.
    pub payload: Vec<u8>,
}

/// A decrypted (plaintext) value that preserves its source width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaintextValue {
    /// 8-bit plaintext
    U8(u8),
    /// 16-bit plaintext
    U16(u16),
    /// 32-bit plaintext
    U32(u32),
    /// 64-bit plaintext
    U64(u64),
}

impl PlaintextValue {
    /// The width of this plaintext value.
    pub fn width(&self) -> ScalarWidth {
        match self {
            PlaintextValue::U8(_) => ScalarWidth::U8,
            PlaintextValue::U16(_) => ScalarWidth::U16,
            PlaintextValue::U32(_) => ScalarWidth::U32,
            PlaintextValue::U64(_) => ScalarWidth::U64,
        }
    }

    /// Convert to `u64` without shortening.
    ///
    /// Always succeeds — every width fits in `u64`.
    pub fn as_u64(self) -> u64 {
        match self {
            PlaintextValue::U8(v) => v as u64,
            PlaintextValue::U16(v) => v as u64,
            PlaintextValue::U32(v) => v as u64,
            PlaintextValue::U64(v) => v,
        }
    }

    /// Convert to `u64`, rejecting values that do not fit in the requested
    /// width.
    pub fn as_u64_checked(self, width: ScalarWidth) -> FheResult<u64> {
        let value = self.as_u64();
        if value <= width.max_value() {
            Ok(value)
        } else {
            Err(FheError::ValueOutOfRange {
                value,
                width: width.bits(),
                max: width.max_value(),
            })
        }
    }

    /// Compare two plaintext values numerically, regardless of their widths.
    pub fn numeric_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_u64().cmp(&other.as_u64())
    }
}

/// A column of ciphertexts — the storage unit used by `fhe-db`.
///
/// All entries in a column share the same width; this is enforced by the
/// [`EncryptedColumn::push`] accessor.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct EncryptedColumn {
    /// The common width of all ciphertexts in this column.
    width: Option<ScalarWidth>,
    /// The ciphertexts.
    pub values: Vec<Ciphertext>,
}

impl EncryptedColumn {
    /// Create an empty column of the given width.
    pub fn new(width: ScalarWidth) -> Self {
        Self {
            width: Some(width),
            values: Vec::new(),
        }
    }

    /// The declared width of the column, if any is set.
    pub fn width(&self) -> Option<ScalarWidth> {
        self.width
    }

    /// Number of ciphertexts in the column.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the column is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Append a ciphertext, verifying it matches the column width.
    ///
    /// If the column has no width yet (freshly deserialized), the first
    /// element's width is adopted.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::WidthMismatch`] if the ciphertext width differs
    /// from the column's declared width.
    pub fn push(&mut self, ct: Ciphertext) -> FheResult<()> {
        match self.width {
            None => {
                let w = ct.width();
                self.width = Some(w);
                self.values.push(ct);
                Ok(())
            }
            Some(w) => {
                if ct.width() != w {
                    return Err(FheError::WidthMismatch {
                        expected: w.bits(),
                        found: ct.width().bits(),
                    });
                }
                self.values.push(ct);
                Ok(())
            }
        }
    }

    /// Iterate over the ciphertexts in this column.
    pub fn iter(&self) -> impl Iterator<Item = &Ciphertext> {
        self.values.iter()
    }
}

/// A column of decrypted plaintext values.
#[derive(Debug, Clone, Default)]
pub struct PlaintextColumn {
    /// Decrypted values.
    pub values: Vec<PlaintextValue>,
}

impl PlaintextColumn {
    /// Decrypt every ciphertext in the column with the client key.
    pub fn decrypt(&self, _client_key: &ClientKey) -> FheResult<Vec<PlaintextValue>> {
        // PlaintextColumn holds already-decrypted values; present for API
        // symmetry with EncryptedColumn.
        Ok(self.values.clone())
    }
}

impl FromIterator<PlaintextValue> for PlaintextColumn {
    fn from_iter<I: IntoIterator<Item = PlaintextValue>>(iter: I) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::ClientKey;

    fn keypair() -> (ClientKey, crate::ServerKey) {
        ClientKey::new()
    }

    #[test]
    fn envelope_roundtrip_u32() {
        let (client_key, _server_key) = keypair();
        use crate::Encryptor;
        let encryptor = Encryptor::new(&client_key);
        let ct = encryptor
            .encrypt_u32(42, ScalarWidth::U32)
            .expect("encrypt");
        assert_eq!(ct.width(), ScalarWidth::U32);

        let bytes = ct.to_envelope_bytes().expect("serialize");
        let back = Ciphertext::from_envelope_bytes(&bytes).expect("deserialize");
        assert_eq!(back.width(), ScalarWidth::U32);
        assert_eq!(back.decrypt(&client_key).as_u64(), 42);
    }

    #[test]
    fn envelope_roundtrip_all_widths() {
        let (client_key, _server_key) = keypair();
        use crate::Encryptor;
        let encryptor = Encryptor::new(&client_key);

        let cases: &[(u64, ScalarWidth)] = &[
            (200, ScalarWidth::U8),
            (30_000, ScalarWidth::U16),
            (4_000_000_000, ScalarWidth::U32),
            (9_000_000_000_000_000_000, ScalarWidth::U64),
        ];
        for (value, width) in cases {
            let ct = encryptor.encrypt_u64(*value, *width).expect("encrypt");
            let bytes = ct.to_envelope_bytes().expect("serialize");
            let back = Ciphertext::from_envelope_bytes(&bytes).expect("deserialize");
            assert_eq!(back.decrypt(&client_key).as_u64(), *value);
        }
    }

    #[test]
    fn envelope_rejects_unknown_version() {
        let (client_key, _server_key) = keypair();
        use crate::Encryptor;
        let encryptor = Encryptor::new(&client_key);
        let ct = encryptor.encrypt_u32(1, ScalarWidth::U8).expect("encrypt");

        let mut envelope: CiphertextEnvelope =
            bincode::deserialize(&ct.to_envelope_bytes().expect("serialize"))
                .expect("decode envelope");
        envelope.version = 99;
        let bytes = bincode::serialize(&envelope).expect("encode envelope");

        match Ciphertext::from_envelope_bytes(&bytes) {
            Err(FheError::UnknownEnvelopeVersion(99)) => {}
            Err(other) => panic!("expected UnknownEnvelopeVersion, got {other}"),
            Ok(_) => panic!("expected UnknownEnvelopeVersion, got Ok"),
        }
    }

    #[test]
    fn envelope_rejects_malformed() {
        let garbage = b"this is not a ciphertext envelope";
        match Ciphertext::from_envelope_bytes(garbage) {
            Err(FheError::MalformedEnvelope) => {}
            Err(other) => panic!("expected MalformedEnvelope, got {other}"),
            Ok(_) => panic!("expected MalformedEnvelope, got Ok"),
        }
    }

    #[test]
    fn envelope_rejects_length_prefix_bomb() {
        // A real envelope whose payload's `blocks` length prefix is set to a
        // huge value must be rejected *before* bincode allocates from it
        // (serde preallocates up to 1 GiB per sequence from the prefix).
        let (client_key, _server_key) = keypair();
        use crate::Encryptor;
        let encryptor = Encryptor::new(&client_key);
        let ct = encryptor
            .encrypt_u32(42, ScalarWidth::U32)
            .expect("encrypt");
        let mut bytes = ct.to_envelope_bytes().expect("serialize");

        // Envelope framing: version (1) + width tag (4) + payload_len (8);
        // the payload's blocks length prefix is at payload[4..12].
        let payload_start = 13usize;
        let payload_len = u64::from_le_bytes(bytes[5..13].try_into().unwrap()) as usize;
        let bomb = 1u64 << 60;
        bytes[payload_start + 4..payload_start + 12].copy_from_slice(&bomb.to_le_bytes());
        assert_eq!(payload_len, bytes.len() - payload_start);

        match Ciphertext::from_envelope_bytes(&bytes) {
            Err(FheError::CorruptedCiphertext(_)) => {}
            Err(other) => panic!("expected CorruptedCiphertext, got {other}"),
            Ok(_) => panic!("length-prefix bomb must be rejected"),
        }

        // The LWE-vector length inside the first block is bombed the same way.
        let mut bytes2 = ct.to_envelope_bytes().expect("serialize");
        bytes2[payload_start + 12..payload_start + 20].copy_from_slice(&bomb.to_le_bytes());
        match Ciphertext::from_envelope_bytes(&bytes2) {
            Err(FheError::CorruptedCiphertext(_)) => {}
            Err(other) => panic!("expected CorruptedCiphertext, got {other}"),
            Ok(_) => panic!("LWE length-prefix bomb must be rejected"),
        }
    }

    #[test]
    fn envelope_rejects_oversized_claim() {
        // A claimed payload length far beyond the actual input must be
        // rejected by the header parser before any allocation.
        let (client_key, _server_key) = keypair();
        use crate::Encryptor;
        let encryptor = Encryptor::new(&client_key);
        let ct = encryptor
            .encrypt_u32(42, ScalarWidth::U32)
            .expect("encrypt");
        let mut bytes = ct.to_envelope_bytes().expect("serialize");

        // Claim a payload of u64::MAX bytes.
        bytes[5..13].copy_from_slice(&u64::MAX.to_le_bytes());
        match Ciphertext::from_envelope_bytes(&bytes) {
            Err(FheError::MalformedEnvelope) => {}
            Err(other) => panic!("expected MalformedEnvelope, got {other}"),
            Ok(_) => panic!("oversized payload claim must be rejected"),
        }
    }

    #[test]
    fn encrypted_column_enforces_width() {
        let (client_key, _server_key) = keypair();
        use crate::Encryptor;
        let encryptor = Encryptor::new(&client_key);

        let mut col = EncryptedColumn::new(ScalarWidth::U16);
        col.push(encryptor.encrypt_u64(5, ScalarWidth::U16).unwrap())
            .expect("push matching width");
        assert_eq!(col.len(), 1);

        let err = col.push(encryptor.encrypt_u64(5, ScalarWidth::U32).unwrap());
        assert!(matches!(err, Err(FheError::WidthMismatch { .. })));
    }

    #[test]
    fn plaintext_cmp_across_widths() {
        let small = PlaintextValue::U8(200);
        let large = PlaintextValue::U64(u64::MAX);
        assert_eq!(small.numeric_cmp(&large), std::cmp::Ordering::Less);
    }
}

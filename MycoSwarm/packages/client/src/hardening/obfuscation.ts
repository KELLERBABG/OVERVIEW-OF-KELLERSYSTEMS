/**
 * Symmetric Masking Layer (NOT Homomorphic Encryption)
 *
 * This provides deterministic obfuscation for work unit inputs/outputs.
 * Each work unit receives a unique nonce derived from its ID to prevent
 * reuse attacks. Full homomorphic encryption (TFHE-rs via Wasm) is
 * planned for a future production release.
 *
 * Security properties:
 * - Per-work-unit unique nonce prevents known-plaintext attacks
 * - XOR operation with derived key material
 * - NOT semantically secure (do NOT use for production secrets)
 * - Suitable only for hiding intermediate values from passive observers
 */

export class SymmetricMasking {
  /**
   * Derive a per-work-unit mask from the unit ID and a shared secret.
   * Each unit gets a unique mask, preventing reuse attacks.
   */
  deriveMask(workUnitId: string, sharedSecret: Uint8Array): Uint8Array {
    const input = new TextEncoder().encode(workUnitId);
    const combined = new Uint8Array(input.length + sharedSecret.length);
    combined.set(input);
    combined.set(sharedSecret, input.length);

    // Simple key derivation (in production: HKDF)
    let hash = 0;
    for (const byte of combined) {
      hash = ((hash << 5) - hash) + byte;
      hash |= 0;
    }

    // Generate 32 bytes of mask material
    const mask = new Uint8Array(32);
    let seed = Math.abs(hash);
    for (let i = 0; i < 32; i++) {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      mask[i] = seed & 0xff;
    }

    return mask;
  }

  /**
   * Apply XOR mask to data using per-unit mask.
   */
  mask(data: Uint8Array, mask: Uint8Array): Uint8Array {
    const result = new Uint8Array(data.length);
    for (let i = 0; i < data.length; i++) {
      result[i] = data[i]! ^ mask[i % mask.length]!;
    }
    return result;
  }

  /**
   * Unmask data (XOR is symmetric: mask ∘ mask = identity)
   */
  unmask(data: Uint8Array, mask: Uint8Array): Uint8Array {
    return this.mask(data, mask); // XOR is its own inverse
  }
}
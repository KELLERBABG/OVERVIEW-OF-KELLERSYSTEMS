import { SnarkVerifier } from './verifier.js';
import type { ProofResult } from './types.js';

export class BatchVerifier {
  private verifier: SnarkVerifier;

  constructor(verifier: SnarkVerifier) {
    this.verifier = verifier;
  }

  /**
   * Verify multiple proofs in sequence (MVP approach).
   * For production, this would use recursive SNARK aggregation.
   */
  async verifyBatch(proofs: ProofResult[]): Promise<boolean> {
    for (const { proof, publicSignals } of proofs) {
      const valid = await this.verifier.verify(proof, publicSignals);
      if (!valid) return false;
    }
    return true;
  }

  /**
   * Verify proofs in parallel for performance.
   */
  async verifyBatchParallel(proofs: ProofResult[]): Promise<boolean> {
    const results = await Promise.all(
      proofs.map((p) => this.verifier.verify(p.proof, p.publicSignals)),
    );
    return results.every(Boolean);
  }
}
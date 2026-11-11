import { describe, it, expect } from 'vitest';
import { BatchVerifier } from '../src/batch-verifier.js';
import { SnarkVerifier } from '../src/verifier.js';
import { SnarkProver } from '../src/prover.js';

describe('BatchVerifier', () => {
  it('should verify a batch of proofs', async () => {
    const verifier = new SnarkVerifier({});
    const batchVerifier = new BatchVerifier(verifier);
    const prover = new SnarkProver({ wasmPath: '', zkeyPath: '' });

    const proofs = await Promise.all([
      prover.prove({ publicInputs: { a: '1' }, privateInputs: { b: '2' } }),
      prover.prove({ publicInputs: { a: '3' }, privateInputs: { b: '4' } }),
    ]);

    const valid = await batchVerifier.verifyBatch(proofs);
    expect(valid).toBe(true);
  });

  it('should return false if any proof is invalid', async () => {
    const verifier = new SnarkVerifier({});
    const batchVerifier = new BatchVerifier(verifier);
    const prover = new SnarkProver({ wasmPath: '', zkeyPath: '' });

    const validProof = await prover.prove({
      publicInputs: { a: '1' },
      privateInputs: { b: '2' },
    });
    const invalidProof = { proof: { bad: true } as any, publicSignals: ['x'] };

    const valid = await batchVerifier.verifyBatch([validProof, invalidProof]);
    expect(valid).toBe(false);
  });

  it('should verify in parallel', async () => {
    const verifier = new SnarkVerifier({});
    const batchVerifier = new BatchVerifier(verifier);
    const prover = new SnarkProver({ wasmPath: '', zkeyPath: '' });

    const proofs = await Promise.all([
      prover.prove({ publicInputs: { a: '1' }, privateInputs: {} }),
      prover.prove({ publicInputs: { a: '2' }, privateInputs: {} }),
      prover.prove({ publicInputs: { a: '3' }, privateInputs: {} }),
    ]);

    const valid = await batchVerifier.verifyBatchParallel(proofs);
    expect(valid).toBe(true);
  });
});
import { describe, it, expect } from 'vitest';
import { SnarkVerifier } from '../src/verifier.js';
import { SnarkProver } from '../src/prover.js';

describe('SnarkVerifier', () => {
  it('should verify a mock proof successfully', async () => {
    const vk = { type: 'mock', curve: 'bn128' };
    const verifier = new SnarkVerifier(vk);
    const prover = new SnarkProver({ wasmPath: '', zkeyPath: '' });
    const { proof, publicSignals } = await prover.prove({
      publicInputs: { a: '10' },
      privateInputs: { b: '20' },
    });
    const isValid = await verifier.verify(proof, publicSignals);
    expect(isValid).toBe(true);
  });

  it('should reject an invalid proof', async () => {
    const vk = { type: 'mock', curve: 'bn128' };
    const verifier = new SnarkVerifier(vk);
    const isValid = await verifier.verify({ bad: 'proof' }, ['signal']);
    expect(isValid).toBe(false);
  });

  it('should return the verification key', () => {
    const vk = { key: 'test-key' };
    const verifier = new SnarkVerifier(vk);
    expect(verifier.getVerificationKey()).toEqual(vk);
  });
});
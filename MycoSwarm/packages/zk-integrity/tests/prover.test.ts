import { describe, it, expect } from 'vitest';
import { SnarkProver, verifyMockProof } from '../src/prover.js';

describe('SnarkProver', () => {
  it('should generate a mock proof without snarkjs', async () => {
    const prover = new SnarkProver({
      wasmPath: '/circuits/test.wasm',
      zkeyPath: '/circuits/test.zkey',
    });
    const { proof, publicSignals } = await prover.prove({
      publicInputs: { op: '0' },
      privateInputs: { a: '5', b: '3' },
    });
    expect(proof).toBeDefined();
    expect(publicSignals).toBeDefined();
    expect(publicSignals).toContain('0');
    expect((proof as any).protocol).toBe('groth16');
  });

  it('should produce different proofs for different inputs', async () => {
    const prover = new SnarkProver({ wasmPath: '', zkeyPath: '' });
    const r1 = await prover.prove({
      publicInputs: { x: '1' },
      privateInputs: { secret: '42' },
    });
    const r2 = await prover.prove({
      publicInputs: { x: '2' },
      privateInputs: { secret: '99' },
    });
    expect(JSON.stringify(r1.proof)).not.toBe(JSON.stringify(r2.proof));
  });
});

describe('verifyMockProof', () => {
  it('should return true for a valid mock proof', () => {
    const proof = {
      pi_a: ['12345', '67890', '1'],
      pi_b: [['abc', 'def'], ['ghi', 'jkl'], ['1', '0']],
      pi_c: ['mno', 'pqr', '1'],
      protocol: 'groth16',
      curve: 'bn128',
    };
    expect(verifyMockProof(proof, ['0'])).toBe(true);
  });

  it('should return false for an invalid mock proof', () => {
    expect(verifyMockProof({}, [])).toBe(false);
  });
});
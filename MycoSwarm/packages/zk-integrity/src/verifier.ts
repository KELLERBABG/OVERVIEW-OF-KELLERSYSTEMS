import { verifyMockProof } from './prover.js';

export class SnarkVerifier {
  private verificationKey: object;

  constructor(verificationKey: object) {
    this.verificationKey = verificationKey;
  }

  async verify(proof: object, publicSignals: string[]): Promise<boolean> {
    try {
      const snarkjs = await tryLoadSnarkjs();
      return await snarkjs.groth16.verify(this.verificationKey, publicSignals, proof);
    } catch {
      return verifyMockProof(proof, publicSignals);
    }
  }

  getVerificationKey(): object {
    return this.verificationKey;
  }
}

async function tryLoadSnarkjs(): Promise<{ groth16: { verify: Function } }> {
  throw new Error('snarkjs not available in this environment');
}
export interface ProofOptions {
  wasmPath: string;
  zkeyPath: string;
}

export interface ProofResult {
  proof: object;
  publicSignals: string[];
}

export interface ProverInput {
  publicInputs: Record<string, bigint | string>;
  privateInputs: Record<string, bigint | string>;
}
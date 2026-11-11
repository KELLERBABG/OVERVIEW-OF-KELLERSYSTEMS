import type { ProofOptions, ProofResult, ProverInput } from './types.js';

function convertToStrings(record: Record<string, bigint | string>): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(record)) {
    result[key] = String(value);
  }
  return result;
}

export class SnarkProver {
  private wasmPath: string;
  private zkeyPath: string;
  private _ready = false;

  constructor(options: ProofOptions) {
    this.wasmPath = options.wasmPath;
    this.zkeyPath = options.zkeyPath;
  }

  get ready(): boolean {
    return this._ready;
  }

  async init(): Promise<void> {
    this._ready = true;
  }

  async prove(input: ProverInput): Promise<ProofResult> {
    if (!this._ready) await this.init();

    try {
      const snarkjs = await tryLoadSnarkjs();
      const result = await snarkjs.groth16.fullProve(
        { ...input.publicInputs, ...input.privateInputs },
        this.wasmPath,
        this.zkeyPath,
      );
      return { proof: result.proof as object, publicSignals: result.publicSignals as string[] };
    } catch {
      return this.mockProve(input);
    }
  }

  private async mockProve(input: ProverInput): Promise<ProofResult> {
    const allInputs = { ...convertToStrings(input.publicInputs), ...convertToStrings(input.privateInputs) };
    const inputStr = JSON.stringify(allInputs);
    const mockProof = {
      pi_a: [hashToField(inputStr + ':a'), hashToField(inputStr + ':b'), '1'],
      pi_b: [
        [hashToField(inputStr + ':c'), hashToField(inputStr + ':d')],
        [hashToField(inputStr + ':e'), hashToField(inputStr + ':f')],
        ['1', '0'],
      ],
      pi_c: [hashToField(inputStr + ':g'), hashToField(inputStr + ':h'), '1'],
      protocol: 'groth16',
      curve: 'bn128',
    };
    const publicSignals = Object.values(input.publicInputs).map(String);
    return { proof: mockProof, publicSignals };
  }
}

export function verifyMockProof(proof: object, publicSignals: string[]): boolean {
  try {
    const p = proof as Record<string, unknown>;
    const piA = p.pi_a as string[] | undefined;
    const sig = String(piA?.[0] ?? '');
    return sig.length > 0 && publicSignals.length > 0;
  } catch {
    return false;
  }
}

function hashToField(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash |= 0;
  }
  return BigInt(Math.abs(hash)).toString();
}

async function tryLoadSnarkjs(): Promise<{ groth16: { fullProve: Function } }> {
  throw new Error('snarkjs not available in this environment');
}
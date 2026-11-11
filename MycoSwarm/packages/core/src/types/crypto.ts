export interface KeyPair {
  publicKey: string;
  privateKey: string;
}

export interface ZKProof {
  proof: Uint8Array;
  publicSignals: Uint8Array[];
  verificationKey: Uint8Array;
}

export interface Ciphertext {
  data: Uint8Array;
  /** Which scheme was used (e.g. "BFV", "CKKS") */
  scheme: string;
  /** Public key used for encryption */
  publicKey: string;
}
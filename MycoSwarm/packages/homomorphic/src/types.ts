export interface HEKeyPair {
  clientKey: Uint8Array;
  serverKey: Uint8Array;
}

export interface Ciphertext {
  data: Uint8Array;
  scheme: string;
  publicKey: string;
}
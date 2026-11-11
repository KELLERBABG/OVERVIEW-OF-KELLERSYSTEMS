export interface HEConfig {
  keySizeBits: number;
  maxOperations: number;
  supportedSchemes: string[];
}

export const defaultConfig: HEConfig = {
  keySizeBits: 128,
  maxOperations: 100,
  supportedSchemes: ['BFV', 'CKKS'],
};
export interface ZKConfig {
  verificationProbability: number;
  maxBatchSize: number;
  provingTimeoutMs: number;
}

export const defaultConfig: ZKConfig = {
  verificationProbability: 0.05,
  maxBatchSize: 10,
  provingTimeoutMs: 120_000,
};
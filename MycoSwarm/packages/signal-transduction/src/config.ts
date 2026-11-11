export interface TransductionConfig {
  fanout: number;
  maxTtl: number;
  heartbeatIntervalMs: number;
}

export const defaultConfig: TransductionConfig = {
  fanout: 3,
  maxTtl: 5,
  heartbeatIntervalMs: 30_000,
};
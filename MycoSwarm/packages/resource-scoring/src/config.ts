export interface ScoringConfig {
  benchmarkDurationMs: number;
  broadcastIntervalMs: number;
  primeLimit: number;
  memoryBufferSize: number;
}

export const defaultConfig: ScoringConfig = {
  benchmarkDurationMs: 500,
  broadcastIntervalMs: 30_000,
  primeLimit: 15_000_000,
  memoryBufferSize: 100_000_000,
};
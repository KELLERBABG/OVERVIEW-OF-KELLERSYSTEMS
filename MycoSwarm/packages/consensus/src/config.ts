export interface ConsensusConfig {
  votingWindowMs: number;
  quorumRatio: number;
  approvalRatio: number;
  saturationReportIntervalMs: number;
  maxQueueDepth: number;
  highSaturationThreshold: number;
  fullSaturationThreshold: number;
}

export const defaultConfig: ConsensusConfig = {
  votingWindowMs: 30_000,
  quorumRatio: 0.5,
  approvalRatio: 0.66,
  saturationReportIntervalMs: 30_000,
  maxQueueDepth: 50,
  highSaturationThreshold: 0.9,
  fullSaturationThreshold: 0.95,
};
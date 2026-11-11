export interface RedundancyConfig {
  windowSize: number;
  defaultDropoutRate: number;
  baseReliability: number;
  maxFactor: number;
}

export const defaultConfig: RedundancyConfig = {
  windowSize: 100,
  defaultDropoutRate: 0.3,
  baseReliability: 0.95,
  maxFactor: 5,
};
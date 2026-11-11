export interface DistributionConfig {
  minUnitSize: number;
  targetUnitCount: number;
  defaultRedundancy: number;
  maxUnits: number;
  maxRecursionDepth: number;
}

export const defaultConfig: DistributionConfig = {
  minUnitSize: 100,
  targetUnitCount: 10,
  defaultRedundancy: 2,
  maxUnits: 1000,
  maxRecursionDepth: 8,
};
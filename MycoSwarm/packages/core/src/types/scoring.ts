export interface BioScore {
  value: number;
  cpuScore: number;
  memoryScore: number;
  networkScore: number;
  stabilityScore: number;
  entropy: number;
  timestamp: number;
}

export interface BenchmarkResult {
  cpu: {
    aluOpsPerSecond: number;
    fpOpsPerSecond: number;
    primeBenchMs: number;
  };
  memory: {
    readLatencyNs: number;
    writeBandwidthMBs: number;
  };
  network?: {
    downloadSpeedMbps: number;
    uploadSpeedMbps: number;
    latencyMs: number;
  };
}
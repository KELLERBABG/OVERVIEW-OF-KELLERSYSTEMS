export interface CpuBenchResult {
  aluOpsPerSecond: number;
  fpOpsPerSecond: number;
  primeBenchMs: number;
}

export interface MemoryBenchResult {
  readLatencyNs: number;
  writeBandwidthMBs: number;
}

export interface NetworkBenchResult {
  downloadSpeedMbps: number;
  uploadSpeedMbps: number;
  latencyMs: number;
}

export interface BioScoreInput {
  cpu: CpuBenchResult;
  memory: MemoryBenchResult;
  network?: NetworkBenchResult;
  upvotes: number;
  sessionAge: number;
}
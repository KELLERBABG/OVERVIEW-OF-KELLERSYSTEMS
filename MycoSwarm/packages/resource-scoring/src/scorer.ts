import { benchmarkCpu } from './benchmarks/cpu.js';
import { benchmarkMemory } from './benchmarks/memory.js';
import { calculateBioScore } from './entropy-score.js';
import type { BioScore, BenchmarkResult } from '@fungal/core';
import type { ScoringConfig } from './config.js';
import { defaultConfig } from './config.js';

export class ResourceScorer {
  private config: ScoringConfig;
  private lastScore: BioScore | null = null;
  private startedAt = Date.now();

  constructor(config?: Partial<ScoringConfig>) {
    this.config = { ...defaultConfig, ...config };
  }

  async runBenchmarks(): Promise<BenchmarkResult> {
    const cpu = await benchmarkCpu(this.config);
    const memory = await benchmarkMemory(this.config);

    return { cpu, memory };
  }

  async getScore(): Promise<BioScore> {
    const benchmark = await this.runBenchmarks();
    const sessionAge = Date.now() - this.startedAt;

    const value = calculateBioScore({
      cpu: benchmark.cpu,
      memory: benchmark.memory,
      upvotes: 0,
      sessionAge,
    });

    this.lastScore = {
      value,
      cpuScore: benchmark.cpu.aluOpsPerSecond / 2000,
      memoryScore: benchmark.memory.readLatencyNs > 0
        ? Math.max(0, 1 - benchmark.memory.readLatencyNs / 500)
        : 0,
      networkScore: 0.5,
      stabilityScore: Math.min(sessionAge / 300_000, 1),
      entropy: value / 25,
      timestamp: Date.now(),
    };

    return this.lastScore;
  }

  getLastScore(): BioScore | null {
    return this.lastScore;
  }
}
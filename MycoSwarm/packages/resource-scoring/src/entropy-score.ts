import type { BioScoreInput, CpuBenchResult, MemoryBenchResult, NetworkBenchResult } from './types.js';

/**
 * Bio-Score (0–100) using an entropy-weighted formula.
 *
 * A balanced machine (good CPU + good RAM + long session) scores higher
 * than one that's imbalanced (great CPU but terrible RAM).
 *
 * Formula:
 *   normalized = [cpuScore, memScore, netScore, stabilityScore]
 *   entropy = -Σ(p_i * log₂(p_i))
 *   bioScore = entropy * 25
 */
export function calculateBioScore(input: BioScoreInput): number {
  const cpuScore = normalizeCpu(input.cpu);
  const memScore = normalizeMemory(input.memory);
  const netScore = input.network ? normalizeNetwork(input.network) : 0.5;
  const stabilityScore = Math.min(input.sessionAge / 300_000, 1);

  const components = [cpuScore, memScore, netScore, stabilityScore];
  const total = components.reduce((a, b) => a + b, 0);

  // Shannon entropy
  let entropy = 0;
  for (const c of components) {
    if (c === 0) continue;
    const p = c / total;
    entropy -= p * Math.log2(p);
  }

  // Normalize to 0-100
  const maxEntropy = Math.log2(components.length);
  const normalizedEntropy = maxEntropy > 0 ? entropy / maxEntropy : 0;

  return Math.round(Math.min(1, normalizedEntropy) * 100);
}

function normalizeCpu(cpu: CpuBenchResult): number {
  const aluScore = Math.min(cpu.aluOpsPerSecond / 2000, 1);
  const fpScore = Math.min(cpu.fpOpsPerSecond / 500, 1);
  return aluScore * 0.6 + fpScore * 0.4;
}

function normalizeMemory(mem: MemoryBenchResult): number {
  const latencyScore = Math.max(0, 1 - (mem.readLatencyNs - 10) / 490);
  return latencyScore;
}

function normalizeNetwork(net: NetworkBenchResult): number {
  const speedScore = Math.min(net.downloadSpeedMbps / 100, 1);
  const latencyScore = Math.max(0, 1 - net.latencyMs / 200);
  return speedScore * 0.5 + latencyScore * 0.5;
}

export function calculateBioScoreSimple(components: number[]): number {
  const total = components.reduce((a, b) => a + b, 0);
  if (total === 0) return 0;

  let entropy = 0;
  for (const c of components) {
    if (c === 0) continue;
    const p = c / total;
    entropy -= p * Math.log2(p);
  }

  const maxEntropy = Math.log2(components.length);
  const normalizedEntropy = maxEntropy > 0 ? entropy / maxEntropy : 0;
  return Math.round(Math.min(1, normalizedEntropy) * 100);
}
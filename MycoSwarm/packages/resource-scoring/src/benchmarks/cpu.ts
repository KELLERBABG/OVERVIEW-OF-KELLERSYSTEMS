import type { CpuBenchResult } from '../types.js';
import type { ScoringConfig } from '../config.js';

export async function benchmarkCpu(config?: Partial<ScoringConfig>): Promise<CpuBenchResult> {
  const duration = config?.benchmarkDurationMs ?? 500;
  const primeLimit = config?.primeLimit ?? 15_000_000;

  // ALU benchmark: integer arithmetic
  const aluStart = performance.now();
  let aluCount = 0;
  while (performance.now() - aluStart < duration) {
    for (let i = 0; i < 10000; i++) {
      let x = i * 3 + 7 - 2 / 1;
      x = (x << 2) | (x >> 1);
      void (x ^ 0xff);
    }
    aluCount += 10000;
  }
  const aluElapsed = (performance.now() - aluStart) / 1000;
  const aluOpsPerSecond = aluCount / aluElapsed / 1_000_000;

  // FP benchmark: float operations
  const fpStart = performance.now();
  let fpCount = 0;
  while (performance.now() - fpStart < duration) {
    for (let i = 0; i < 10000; i++) {
      let x = Math.sqrt(i * 1.0);
      x = Math.sin(x) * Math.cos(x);
      x = Math.exp(Math.log(x + 1));
      void (x * 0.5);
    }
    fpCount += 10000;
  }
  const fpElapsed = (performance.now() - fpStart) / 1000;
  const fpOpsPerSecond = fpCount / fpElapsed / 1_000_000;

  // Prime sieve benchmark (~1M primes)
  const primeStart = performance.now();
  const limit = primeLimit;
  const sieve = new Uint8Array(limit);
  let primeCount = 0;
  for (let i = 2; i < limit; i++) {
    if (!sieve[i]) {
      primeCount++;
      for (let j = i * i; j < limit && j > 0; j += i) {
        if (j < limit) sieve[j] = 1;
      }
    }
    if (primeCount >= 1_000_000) break;
  }
  const primeBenchMs = performance.now() - primeStart;

  return { aluOpsPerSecond, fpOpsPerSecond, primeBenchMs };
}
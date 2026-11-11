import type { MemoryBenchResult } from '../types.js';
import type { ScoringConfig } from '../config.js';
import { defaultConfig } from '../config.js';

export async function benchmarkMemory(config?: Partial<ScoringConfig>): Promise<MemoryBenchResult> {
  const cfg = { ...defaultConfig, ...config };
  const bufSize = Math.min(cfg.memoryBufferSize, 50_000_000);
  const buf = new Uint8Array(bufSize);

  // Write bandwidth: fill buffer
  const writeStart = performance.now();
  for (let i = 0; i < buf.length; i += 64) {
    buf[i] = 0xff;
  }
  const writeElapsed = performance.now() - writeStart;
  const writeBandwidthMBs = buf.length / writeElapsed / 1000;

  // Read latency: random access
  const step = Math.max(1, Math.floor(buf.length / 10000));
  const readStart = performance.now();
  let sum = 0;
  for (let i = 0; i < buf.length; i += step) {
    sum += buf[i] as number;
  }
  const readElapsed = performance.now() - readStart;
  const readLatencyNs = (readElapsed * 1_000_000) / (buf.length / step);

  return { readLatencyNs, writeBandwidthMBs };
}
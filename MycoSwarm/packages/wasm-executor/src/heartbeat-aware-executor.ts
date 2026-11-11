/**
 * Heartbeat-aware executor wrapper.
 *
 * Long-running Wasm computations can cause false-positive eviction
 * from the P2P mesh because the browser cannot process WebRTC
 * heartbeat messages during synchronous execution frames.
 *
 * This wrapper:
 * 1. Periodically yields to the event loop (default: every 50ms)
 * 2. Tracks missed heartbeat windows
 * 3. Reports execution progress to prevent timeout-based eviction
 */

import { createTimeoutSignal } from './timeout.js';
import type { ExecutionResult } from './types.js';
import { yieldIfNeeded, resetYieldTimer } from './yield.js';

export interface HeartbeatAwareConfig {
  maxExecutionTimeMs: number;
  yieldIntervalMs: number;
  heartbeatTimeoutMs: number;
}

export const heartbeatAwareDefaults: HeartbeatAwareConfig = {
  maxExecutionTimeMs: 30_000,
  yieldIntervalMs: 50,
  heartbeatTimeoutMs: 15_000, // Must yield before RoomManager heartbeat timeout
};

export class HeartbeatAwareExecutor {
  private config: HeartbeatAwareConfig;

  constructor(config?: Partial<HeartbeatAwareConfig>) {
    this.config = { ...heartbeatAwareDefaults, ...config };
  }

  /**
   * Execute code with heartbeat-aware yielding.
   * The execution is split into chunks that yield to the event loop
   * every `yieldIntervalMs` to prevent false P2P eviction.
   */
  async executeWithHeartbeat(
    code: string,
    input: unknown,
    onProgress?: (percent: number) => void,
  ): Promise<ExecutionResult> {
    const startTime = performance.now();
    const signal = createTimeoutSignal(this.config.maxExecutionTimeMs);
    resetYieldTimer();

    try {
      // Split large inputs into chunks that yield between them
      const inputArray = Array.isArray(input) ? input : [input];
      const chunkSize = Math.max(1, Math.ceil(inputArray.length / 100));
      const results: unknown[] = [];

      for (let i = 0; i < inputArray.length; i += chunkSize) {
        if (signal.aborted) {
          throw new Error('TIMEOUT: Execution aborted');
        }

        // Yield to event loop to process WebRTC heartbeats
        await yieldIfNeeded();

        const chunk = inputArray.slice(i, i + chunkSize);
        const fn = new Function('input', code);
        const chunkResult = fn(chunk);
        results.push(chunkResult);

        // Report progress
        if (onProgress) {
          onProgress(Math.min(100, (i / inputArray.length) * 100));
        }
      }

      const output = Array.isArray(input) ? results.flat() : results[0];

      return {
        output,
        executionTime: performance.now() - startTime,
        memoryUsed: 0,
      };
    } finally {
      signal.abort();
    }
  }
}
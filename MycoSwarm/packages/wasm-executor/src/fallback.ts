import type { ExecutionResult } from './types.js';
import { createTimeoutSignal } from './timeout.js';
import type { ExecutorConfig } from './config.js';

export async function executeJsFallback(
  code: string,
  input: unknown,
  config: ExecutorConfig,
  startTime: number,
): Promise<ExecutionResult> {
  const signal = createTimeoutSignal(config.maxExecutionTime);

  try {
    const fn = new Function('input', code);
    const output = await fn(input);
    return {
      output,
      executionTime: performance.now() - startTime,
      memoryUsed: 0,
    };
  } finally {
    signal.abort();
  }
}
export interface ExecutionResult {
  output: unknown;
  executionTime: number;
  memoryUsed: number;
  instructionsExecuted?: number;
}

export interface ExecutionError {
  code: 'TIMEOUT' | 'OOM' | 'PANIC' | 'INVALID_CODE' | 'SANDBOX_CRASH';
  message: string;
}
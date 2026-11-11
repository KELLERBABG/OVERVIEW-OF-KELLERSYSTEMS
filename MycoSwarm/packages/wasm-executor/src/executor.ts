import { Sandbox } from './sandbox.js';
import { createTimeoutSignal } from './timeout.js';
import { executeJsFallback } from './fallback.js';
import { MemoryManager } from './memory-manager.js';
import type { ExecutionResult } from './types.js';
import type { ExecutorConfig } from './config.js';
import { defaultConfig } from './config.js';

export class WasmExecutor {
  private config: ExecutorConfig;
  private sandbox: Sandbox;
  private memoryManager: MemoryManager;

  constructor(config?: Partial<ExecutorConfig>) {
    this.config = { ...defaultConfig, ...config };
    this.sandbox = new Sandbox(this.config);
    this.memoryManager = new MemoryManager(this.config.maxMemory);
  }

  async execute(code: Uint8Array | string, input: unknown): Promise<ExecutionResult> {
    const startTime = performance.now();

    if (code instanceof Uint8Array) {
      return this.executeWasm(code, input, startTime);
    }
    return executeJsFallback(code, input, this.config, startTime);
  }

  private async executeWasm(
    bytecode: Uint8Array,
    input: unknown,
    startTime: number,
  ): Promise<ExecutionResult> {
    if (!this.memoryManager.allocate(bytecode.length + 1024 * 1024)) {
      throw new Error('OOM: Cannot allocate memory for bytecode');
    }

    try {
      const instance = await this.sandbox.compile(bytecode, this.config.maxMemory);
      const signal = createTimeoutSignal(this.config.maxExecutionTime);

      try {
        const output = await this.sandbox.run(instance, 'main', [input], signal);
        const executionTime = performance.now() - startTime;
        const memoryUsed =
          (instance.exports.memory as WebAssembly.Memory | undefined)?.buffer.byteLength ?? 0;

        return { output, executionTime, memoryUsed };
      } finally {
        signal.abort();
      }
    } finally {
      this.memoryManager.release(bytecode.length + 1024 * 1024);
    }
  }

  destroy(): void {
    this.sandbox.terminate();
    this.memoryManager.reset();
  }
}
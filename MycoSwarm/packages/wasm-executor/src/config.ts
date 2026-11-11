export interface ExecutorConfig {
  maxMemory: number;
  maxExecutionTime: number;
  maxStackDepth: number;
  sandboxType: 'worker' | 'iframe' | 'same-thread';
}

export const defaultConfig: ExecutorConfig = {
  maxMemory: 64 * 1024 * 1024,
  maxExecutionTime: 30_000,
  maxStackDepth: 1000,
  sandboxType: 'same-thread',
};
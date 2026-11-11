import type { SubstrateAdapter, RuntimeInfo, StorageAdapter, WorkerAdapter, SystemMetrics, MeshConfigLike } from './types.js';

export class Substrate {
  private adapter: SubstrateAdapter;

  constructor(adapter: SubstrateAdapter) {
    this.adapter = adapter;
  }

  get type(): string {
    return this.adapter.type;
  }

  get runtime(): RuntimeInfo {
    return this.adapter.runtime;
  }

  async connect(config: MeshConfigLike): Promise<unknown> {
    return this.adapter.connect(config);
  }

  async initStorage(namespace: string): Promise<StorageAdapter> {
    return this.adapter.initStorage(namespace);
  }

  async createWorker(script: string): Promise<WorkerAdapter> {
    return this.adapter.createWorker(script);
  }

  async getSystemMetrics(): Promise<SystemMetrics> {
    return this.adapter.getSystemMetrics();
  }
}
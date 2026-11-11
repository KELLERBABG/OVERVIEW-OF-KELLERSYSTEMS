import type { SubstrateAdapter, RuntimeInfo, StorageAdapter, WorkerAdapter, SystemMetrics, MeshConfigLike } from '../types.js';

export class BrowserAdapter implements SubstrateAdapter {
  readonly type = 'browser' as const;

  readonly runtime: RuntimeInfo = {
    name: 'Browser',
    version: typeof navigator !== 'undefined' ? navigator.userAgent : 'unknown',
    hasWasm: typeof WebAssembly !== 'undefined',
    hasWebRTC: typeof RTCPeerConnection !== 'undefined',
    hasIndexedDB: typeof indexedDB !== 'undefined',
    hasServiceWorker: typeof navigator !== 'undefined' && 'serviceWorker' in navigator,
    memoryLimit: typeof navigator !== 'undefined'
      ? ((navigator as any).deviceMemory ?? 4) * 1024
      : 4096,
  };

  async connect(_config: MeshConfigLike): Promise<unknown> {
    return { type: 'browser', connected: true };
  }

  async initStorage(namespace: string): Promise<StorageAdapter> {
    return new MapStorage(namespace);
  }

  async createWorker(script: string): Promise<WorkerAdapter> {
    const blob = new Blob([script], { type: 'application/javascript' });
    const worker = new Worker(URL.createObjectURL(blob));
    return {
      postMessage: (msg: unknown) => worker.postMessage(msg),
      onMessage: (handler: (msg: unknown) => void) => {
        worker.onmessage = (e) => handler(e.data);
      },
      terminate: () => worker.terminate(),
    };
  }

  async getSystemMetrics(): Promise<SystemMetrics> {
    const memory = (performance as any).memory;
    return {
      memory: {
        total: (navigator as any).deviceMemory ?? 4 * 1024,
        used: memory?.usedJSHeapSize ?? 0,
        jsHeap: memory?.totalJSHeapSize ?? 0,
      },
      cpu: {
        cores: navigator.hardwareConcurrency ?? 4,
        speed: 0,
      },
      network: {
        type: (navigator as any).connection?.effectiveType ?? 'unknown',
        downlink: (navigator as any).connection?.downlink ?? 0,
      },
    };
  }
}

class MapStorage implements StorageAdapter {
  private map = new Map<string, unknown>();

  constructor(private namespace: string) {}

  async get(key: string): Promise<unknown | null> {
    return this.map.get(`${this.namespace}:${key}`) ?? null;
  }

  async set(key: string, value: unknown): Promise<void> {
    this.map.set(`${this.namespace}:${key}`, value);
  }

  async delete(key: string): Promise<void> {
    this.map.delete(`${this.namespace}:${key}`);
  }

  async clear(): Promise<void> {
    this.map.clear();
  }
}
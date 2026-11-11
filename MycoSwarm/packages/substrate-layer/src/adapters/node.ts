import type { SubstrateAdapter, RuntimeInfo, StorageAdapter, WorkerAdapter, SystemMetrics, MeshConfigLike } from '../types.js';

const isNode = typeof process !== 'undefined' && process.versions?.node;

function getNodeOS() {
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const os = require('os');
    return os;
  } catch {
    return null;
  }
}

export class NodeAdapter implements SubstrateAdapter {
  readonly type = 'node' as const;

  readonly runtime: RuntimeInfo = {
    name: isNode ? `Node.js ${process.version}` : 'Node.js',
    version: isNode ? process.version : 'unknown',
    hasWasm: true,
    hasWebRTC: false,
    hasIndexedDB: false,
    hasServiceWorker: false,
    memoryLimit: (() => {
      const os = getNodeOS();
      return os ? os.totalmem() / (1024 * 1024) : 4096;
    })(),
  };

  async connect(_config: MeshConfigLike): Promise<unknown> {
    throw new Error('Node.js adapter — use TCP transport');
  }

  async initStorage(namespace: string): Promise<StorageAdapter> {
    return new MapStorage(namespace);
  }

  async createWorker(_script: string): Promise<WorkerAdapter> {
    throw new Error('Workers not available in Node.js adapter');
  }

  async getSystemMetrics(): Promise<SystemMetrics> {
    const os = getNodeOS();
    if (os) {
      return {
        memory: {
          total: os.totalmem(),
          used: os.totalmem() - os.freemem(),
          jsHeap: isNode ? process.memoryUsage().heapUsed : 0,
        },
        cpu: {
          cores: os.cpus().length,
          speed: os.cpus()[0]?.speed ?? 0,
        },
        network: {
          type: 'ethernet',
          downlink: 1000,
        },
      };
    }
    return {
      memory: { total: 0, used: 0, jsHeap: 0 },
      cpu: { cores: 0, speed: 0 },
      network: { type: 'unknown', downlink: 0 },
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
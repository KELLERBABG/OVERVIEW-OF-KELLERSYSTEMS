export interface SubstrateAdapter {
  type: 'browser' | 'node' | 'headless';
  runtime: RuntimeInfo;
  connect(config: MeshConfigLike): Promise<unknown>;
  initStorage(namespace: string): Promise<StorageAdapter>;
  createWorker(script: string): Promise<WorkerAdapter>;
  getSystemMetrics(): Promise<SystemMetrics>;
}

export interface RuntimeInfo {
  name: string;
  version: string;
  hasWasm: boolean;
  hasWebRTC: boolean;
  hasIndexedDB: boolean;
  hasServiceWorker: boolean;
  memoryLimit: number;
}

export interface StorageAdapter {
  get(key: string): Promise<unknown | null>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
}

export interface WorkerAdapter {
  postMessage(msg: unknown): void;
  onMessage(handler: (msg: unknown) => void): void;
  terminate(): void;
}

export interface SystemMetrics {
  memory: { total: number; used: number; jsHeap: number };
  cpu: { cores: number; speed: number };
  network: { type: string; downlink: number };
}

export interface MeshConfigLike {
  signalingUrl: string;
  roomId?: string;
  topology?: string;
}
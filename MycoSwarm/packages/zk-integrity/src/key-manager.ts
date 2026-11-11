export class KeyManager {
  private storage: Storage | null = null;

  constructor() {
    if (typeof indexedDB !== 'undefined') {
      // In production, use IndexedDB. For now use in-memory.
      this.storage = new MapStorage();
    } else {
      this.storage = new MapStorage();
    }
  }

  async storeVerificationKey(circuitId: string, key: object): Promise<void> {
    await this.storage!.set(`vk:${circuitId}`, key);
  }

  async getVerificationKey(circuitId: string): Promise<object | null> {
    return this.storage!.get(`vk:${circuitId}`) as Promise<object | null>;
  }

  async storeProvingKey(circuitId: string, key: ArrayBuffer): Promise<void> {
    await this.storage!.set(`pk:${circuitId}`, key);
  }

  async getProvingKey(circuitId: string): Promise<ArrayBuffer | null> {
    return this.storage!.get(`pk:${circuitId}`) as Promise<ArrayBuffer | null>;
  }

  async downloadKeys(circuitId: string, urls: { vk: string; pk: string }): Promise<void> {
    const vkResp = await fetch(urls.vk);
    const vk = await vkResp.json();
    await this.storeVerificationKey(circuitId, vk);

    const pkResp = await fetch(urls.pk);
    const pk = await pkResp.arrayBuffer();
    await this.storeProvingKey(circuitId, pk);
  }

  clear(): void {
    this.storage!.clear();
  }
}

interface Storage {
  get(key: string): Promise<unknown>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
}

class MapStorage implements Storage {
  private map = new Map<string, unknown>();

  async get(key: string): Promise<unknown> {
    return this.map.get(key) ?? null;
  }

  async set(key: string, value: unknown): Promise<void> {
    this.map.set(key, value);
  }

  async delete(key: string): Promise<void> {
    this.map.delete(key);
  }

  async clear(): Promise<void> {
    this.map.clear();
  }
}
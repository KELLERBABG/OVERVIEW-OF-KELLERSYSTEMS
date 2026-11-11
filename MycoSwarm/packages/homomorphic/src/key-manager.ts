import type { HEKeyPair } from './types.js';

export class HeKeyManager {
  private keyStore = new Map<string, HEKeyPair>();

  async generateForTask(taskId: string): Promise<{ serverKey: Uint8Array }> {
    const keys = generateHeKeypair();
    this.keyStore.set(taskId, keys);
    return { serverKey: keys.serverKey };
  }

  async decryptResult(taskId: string, encrypted: Uint8Array): Promise<number> {
    const keys = this.keyStore.get(taskId);
    if (!keys) throw new Error(`No keys for task ${taskId}`);
    return decryptValue(encrypted, keys.clientKey);
  }

  getClientKey(taskId: string): Uint8Array | null {
    return this.keyStore.get(taskId)?.clientKey ?? null;
  }

  destroyKeys(taskId: string): void {
    this.keyStore.delete(taskId);
  }

  clear(): void {
    this.keyStore.clear();
  }

  get size(): number {
    return this.keyStore.size;
  }
}

// Simplified HE key generation using Web Crypto
function generateHeKeypair(): HEKeyPair {
  const clientKey = new Uint8Array(32);
  const serverKey = new Uint8Array(32);
  crypto.getRandomValues(clientKey);
  crypto.getRandomValues(serverKey);
  return { clientKey, serverKey };
}

function decryptValue(encrypted: Uint8Array, _clientKey: Uint8Array): number {
  // Simplified: XOR-based mock decryption
  let value = 0;
  for (let i = 0; i < encrypted.length; i++) {
    value = (value << 8) | encrypted[i]!;
  }
  return value;
}
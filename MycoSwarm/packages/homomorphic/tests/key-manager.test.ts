import { describe, it, expect } from 'vitest';
import { HeKeyManager } from '../src/key-manager.js';

describe('HeKeyManager', () => {
  it('should generate keys for a task', async () => {
    const mgr = new HeKeyManager();
    const { serverKey } = await mgr.generateForTask('task1');
    expect(serverKey).toBeInstanceOf(Uint8Array);
    expect(serverKey.length).toBe(32);
  });

  it('should throw when decrypting without keys', async () => {
    const mgr = new HeKeyManager();
    await expect(mgr.decryptResult('unknown', new Uint8Array(8))).rejects.toThrow('No keys');
  });

  it('should return client key for a task', async () => {
    const mgr = new HeKeyManager();
    await mgr.generateForTask('task1');
    expect(mgr.getClientKey('task1')).toBeInstanceOf(Uint8Array);
    expect(mgr.getClientKey('unknown')).toBeNull();
  });

  it('should clean up keys', async () => {
    const mgr = new HeKeyManager();
    await mgr.generateForTask('task1');
    expect(mgr.size).toBe(1);
    mgr.destroyKeys('task1');
    expect(mgr.size).toBe(0);
  });
});
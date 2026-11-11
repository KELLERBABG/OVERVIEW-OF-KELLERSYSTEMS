import { describe, it, expect } from 'vitest';
import { KeyManager } from '../src/key-manager.js';

describe('KeyManager', () => {
  it('should store and retrieve verification keys', async () => {
    const km = new KeyManager();
    await km.storeVerificationKey('circuit1', { key: 'test-vk' });
    const vk = await km.getVerificationKey('circuit1');
    expect(vk).toEqual({ key: 'test-vk' });
  });

  it('should return null for non-existent keys', async () => {
    const km = new KeyManager();
    const vk = await km.getVerificationKey('nonexistent');
    expect(vk).toBeNull();
  });

  it('should store and retrieve proving keys', async () => {
    const km = new KeyManager();
    const pk = new ArrayBuffer(8);
    await km.storeProvingKey('circuit1', pk);
    const retrieved = await km.getProvingKey('circuit1');
    expect(retrieved).toBeInstanceOf(ArrayBuffer);
  });

  it('should clear all keys', async () => {
    const km = new KeyManager();
    await km.storeVerificationKey('c1', { a: 1 });
    await km.storeVerificationKey('c2', { b: 2 });
    km.clear();
    expect(await km.getVerificationKey('c1')).toBeNull();
    expect(await km.getVerificationKey('c2')).toBeNull();
  });
});
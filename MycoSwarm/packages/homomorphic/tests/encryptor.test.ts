import { describe, it, expect } from 'vitest';
import { HomomorphicEncryptor } from '../src/encryptor.js';

describe('HomomorphicEncryptor', () => {
  it('should generate keys', async () => {
    const encryptor = new HomomorphicEncryptor();
    const keys = await encryptor.generateKeys();
    expect(keys.clientKey).toBeInstanceOf(Uint8Array);
    expect(keys.serverKey).toBeInstanceOf(Uint8Array);
    expect(keys.clientKey.length).toBe(32);
  });

  it('should encrypt and decrypt numbers', async () => {
    const encryptor = new HomomorphicEncryptor();
    const keys = await encryptor.generateKeys();
    const encrypted = await encryptor.encrypt(42, keys.clientKey);
    const decrypted = await encryptor.decrypt(encrypted, keys.clientKey);
    expect(decrypted).toBe(42);
  });
});
import { describe, it, expect } from 'vitest';
import { TokenClient } from '../src/token-client.js';
import { defaultConfig } from '../src/config.js';

describe('TokenClient', () => {
  it('should connect to wallet', async () => {
    const client = new TokenClient(defaultConfig);
    const address = await client.connectWallet();
    expect(address).toMatch(/^0x[a-f0-9]{40}$/);
    expect(client.isConnected).toBe(true);
  });

  it('should return balance', async () => {
    const client = new TokenClient(defaultConfig);
    await client.connectWallet();
    const balance = await client.getBalance('0x' + '0'.repeat(40));
    expect(balance).toBeGreaterThanOrEqual(0n);
  });

  it('should deposit for a task', async () => {
    const client = new TokenClient(defaultConfig);
    await client.connectWallet();
    const txHash = await client.depositForTask('task1', 10, 1000n);
    expect(txHash).toMatch(/^0x[a-f0-9]{64}$/);
  });

  it('should throw if not connected', async () => {
    const client = new TokenClient(defaultConfig);
    await expect(client.depositForTask('task1', 10, 1000n)).rejects.toThrow('Wallet not connected');
  });
});
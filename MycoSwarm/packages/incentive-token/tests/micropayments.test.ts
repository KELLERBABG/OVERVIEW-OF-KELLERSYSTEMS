import { describe, it, expect } from 'vitest';
import { MicropaymentChannel } from '../src/micropayments.js';
import { TokenClient } from '../src/token-client.js';
import { defaultConfig } from '../src/config.js';

describe('MicropaymentChannel', () => {
  it('should open a channel', async () => {
    const client = new TokenClient(defaultConfig);
    await client.connectWallet();
    const channel = new MicropaymentChannel(client);
    const txHash = await channel.openChannel(10000n);
    expect(txHash).toBeDefined();
  });

  it('should sign receipts', async () => {
    const client = new TokenClient(defaultConfig);
    await client.connectWallet();
    const channel = new MicropaymentChannel(client);
    await channel.signReceipt('unit-1', 100n, '0xworker');
    expect(channel.pendingCount).toBe(1);
    expect(channel.pendingTotal).toBe(100n);
  });

  it('should submit batch and clear receipts', async () => {
    const client = new TokenClient(defaultConfig);
    await client.connectWallet();
    const channel = new MicropaymentChannel(client);
    await channel.signReceipt('unit-1', 100n, '0xworker');
    await channel.signReceipt('unit-2', 200n, '0xworker');
    const result = await channel.submitBatch();
    expect(result).toBeDefined();
    expect(channel.pendingCount).toBe(0);
  });

  it('should throw on empty batch submission', async () => {
    const client = new TokenClient(defaultConfig);
    await client.connectWallet();
    const channel = new MicropaymentChannel(client);
    await expect(channel.submitBatch()).rejects.toThrow('No receipts');
  });
});
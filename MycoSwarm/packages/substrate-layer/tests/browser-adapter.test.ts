import { describe, it, expect } from 'vitest';
import { BrowserAdapter } from '../src/adapters/browser.js';

describe('BrowserAdapter', () => {
  it('should detect runtime capabilities', () => {
    const adapter = new BrowserAdapter();
    expect(adapter.type).toBe('browser');
    expect(adapter.runtime.hasWasm).toBe(true);
    expect(adapter.runtime.name).toBe('Browser');
  });

  it('should return system metrics', async () => {
    const adapter = new BrowserAdapter();
    const metrics = await adapter.getSystemMetrics();
    expect(metrics.cpu).toBeDefined();
    expect(metrics.memory).toBeDefined();
    expect(metrics.network).toBeDefined();
  });

  it('should create storage adapter', async () => {
    const adapter = new BrowserAdapter();
    const storage = await adapter.initStorage('test');
    await storage.set('key1', 'value1');
    const val = await storage.get('key1');
    expect(val).toBe('value1');
    await storage.delete('key1');
    expect(await storage.get('key1')).toBeNull();
  });
});
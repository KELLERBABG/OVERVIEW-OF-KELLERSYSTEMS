import { describe, it, expect } from 'vitest';
import { NodeAdapter } from '../src/adapters/node.js';

describe('NodeAdapter', () => {
  it('should detect Node.js runtime', () => {
    const adapter = new NodeAdapter();
    expect(adapter.type).toBe('node');
    expect(adapter.runtime.name).toContain('Node.js');
    expect(adapter.runtime.hasWebRTC).toBe(false);
  });

  it('should return system metrics', async () => {
    const adapter = new NodeAdapter();
    const metrics = await adapter.getSystemMetrics();
    expect(metrics.cpu.cores).toBeGreaterThan(0);
    expect(metrics.memory.total).toBeGreaterThan(0);
  });
});
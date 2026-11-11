import { describe, it, expect } from 'vitest';
import { createTimeoutSignal } from '../src/timeout.js';
import { MemoryManager } from '../src/memory-manager.js';

describe('createTimeoutSignal', () => {
  it('should create a signal that is not initially aborted', () => {
    const signal = createTimeoutSignal(1000);
    expect(signal.aborted).toBe(false);
    signal.abort();
  });

  it('should abort after timeout', async () => {
    const signal = createTimeoutSignal(10);
    await new Promise((r) => setTimeout(r, 20));
    expect(signal.aborted).toBe(true);
    signal.abort();
  });

  it('should abort immediately when called', () => {
    const signal = createTimeoutSignal(1000);
    signal.abort();
    expect(signal.aborted).toBe(true);
  });
});

describe('MemoryManager', () => {
  it('should track allocations', () => {
    const mgr = new MemoryManager(100);
    expect(mgr.allocate(30)).toBe(true);
    expect(mgr.used).toBe(30);
    expect(mgr.available).toBe(70);
  });

  it('should reject allocations over limit', () => {
    const mgr = new MemoryManager(100);
    expect(mgr.allocate(150)).toBe(false);
  });

  it('should release memory', () => {
    const mgr = new MemoryManager(100);
    mgr.allocate(60);
    mgr.release(20);
    expect(mgr.used).toBe(40);
  });

  it('should reset correctly', () => {
    const mgr = new MemoryManager(100);
    mgr.allocate(80);
    mgr.reset();
    expect(mgr.used).toBe(0);
  });
});
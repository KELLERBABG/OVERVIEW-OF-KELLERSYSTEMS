import { describe, it, expect, vi } from 'vitest';
import { EventBus } from '../src/events/event-bus.js';

describe('EventBus', () => {
  it('should emit and receive events', () => {
    const bus = new EventBus();
    const handler = vi.fn();
    bus.on('test', handler);
    bus.emit('test', { data: 42 });
    expect(handler).toHaveBeenCalledWith({ data: 42 });
  });

  it('should support once listeners', () => {
    const bus = new EventBus();
    const handler = vi.fn();
    bus.once('test', handler);
    bus.emit('test', 'first');
    bus.emit('test', 'second');
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith('first');
  });

  it('should allow removing listeners', () => {
    const bus = new EventBus();
    const handler = vi.fn();
    bus.on('test', handler);
    bus.off('test', handler);
    bus.emit('test', 'data');
    expect(handler).not.toHaveBeenCalled();
  });

  it('should support multiple handlers for same event', () => {
    const bus = new EventBus();
    const handler1 = vi.fn();
    const handler2 = vi.fn();
    bus.on('test', handler1);
    bus.on('test', handler2);
    bus.emit('test', 'data');
    expect(handler1).toHaveBeenCalledWith('data');
    expect(handler2).toHaveBeenCalledWith('data');
  });

  it('should remove all listeners', () => {
    const bus = new EventBus();
    const handler1 = vi.fn();
    const handler2 = vi.fn();
    bus.on('a', handler1);
    bus.on('b', handler2);
    bus.removeAllListeners();
    bus.emit('a', 'data');
    bus.emit('b', 'data');
    expect(handler1).not.toHaveBeenCalled();
    expect(handler2).not.toHaveBeenCalled();
  });

  it('should count listeners', () => {
    const bus = new EventBus();
    expect(bus.listenerCount('test')).toBe(0);
    bus.on('test', vi.fn());
    bus.on('test', vi.fn());
    expect(bus.listenerCount('test')).toBe(2);
    bus.once('test', vi.fn());
    expect(bus.listenerCount('test')).toBe(3);
  });
});
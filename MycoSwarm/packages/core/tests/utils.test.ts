import { describe, it, expect } from 'vitest';
import { generateNodeId, generateTaskId, generateMessageId, generateRoomId } from '../src/utils/id.js';
import { now, remaining, isExpired, sleep, formatDuration } from '../src/utils/time.js';
import { serialize, deserialize, estimateSize, createEnvelope } from '../src/utils/serialization.js';
import { MessageType } from '../src/types/message.js';

describe('generateNodeId', () => {
  it('should return a 21-character string', () => {
    const id = generateNodeId();
    expect(id).toHaveLength(21);
  });

  it('should produce unique IDs', () => {
    const ids = new Set(Array.from({ length: 100 }, () => generateNodeId()));
    expect(ids.size).toBe(100);
  });
});

describe('generateTaskId', () => {
  it('should prefix with task_', () => {
    expect(generateTaskId()).toMatch(/^task_/);
  });
});

describe('generateMessageId', () => {
  it('should prefix with msg_', () => {
    expect(generateMessageId()).toMatch(/^msg_/);
  });
});

describe('generateRoomId', () => {
  it('should return an 8-character string', () => {
    expect(generateRoomId()).toHaveLength(8);
  });
});

describe('time utilities', () => {
  it('now() should return a number', () => {
    expect(typeof now()).toBe('number');
  });

  it('remaining() should return 0 for past deadlines', () => {
    expect(remaining(now() - 5000)).toBe(0);
  });

  it('isExpired() should return true for expired timestamps', () => {
    expect(isExpired(now() - 100_000, 10_000)).toBe(true);
  });

  it('isExpired() should return false for non-expired timestamps', () => {
    expect(isExpired(now(), 100_000)).toBe(false);
  });

  it('sleep() should resolve after the specified time', async () => {
    const start = now();
    await sleep(10);
    expect(now() - start).toBeGreaterThanOrEqual(5);
  });

  it('formatDuration() should format ms correctly', () => {
    expect(formatDuration(500)).toBe('500ms');
    expect(formatDuration(1500)).toBe('1.5s');
    expect(formatDuration(120_000)).toBe('2.0min');
  });
});

describe('serialization', () => {
  it('should round-trip serialize/deserialize', () => {
    const obj = { hello: 'world', count: 42, nested: { a: [1, 2, 3] } };
    const bytes = serialize(obj);
    const result = deserialize(bytes);
    expect(result).toEqual(obj);
  });

  it('should handle Uint8Array', () => {
    const obj = { data: new Uint8Array([1, 2, 3, 4]) };
    const bytes = serialize(obj);
    const result = deserialize(bytes) as typeof obj;
    expect(result.data).toBeInstanceOf(Uint8Array);
    expect(Array.from(result.data)).toEqual([1, 2, 3, 4]);
  });

  it('should handle bigint', () => {
    const obj = { big: BigInt('12345678901234567890') };
    const bytes = serialize(obj);
    const result = deserialize(bytes) as typeof obj;
    expect(result.big).toBe(BigInt('12345678901234567890'));
  });

  it('estimateSize should return approximate byte size', () => {
    const size = estimateSize({ a: 1, b: 'hello' });
    expect(size).toBeGreaterThan(10);
  });
});

describe('createEnvelope', () => {
  it('should create a valid message envelope', () => {
    const payload = new Uint8Array([1, 2, 3]);
    const envelope = createEnvelope(MessageType.HELLO, 'sender1', '*', payload);
    expect(envelope.type).toBe(MessageType.HELLO);
    expect(envelope.senderId).toBe('sender1');
    expect(envelope.targetId).toBe('*');
    expect(envelope.payload).toEqual(payload);
    expect(envelope.ttl).toBe(5);
    expect(envelope.messageId).toBeDefined();
    expect(envelope.timestamp).toBeGreaterThan(0);
  });
});
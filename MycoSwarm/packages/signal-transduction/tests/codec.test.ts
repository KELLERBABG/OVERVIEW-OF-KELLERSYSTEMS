import { describe, it, expect } from 'vitest';
import { encodeEnvelope, decodeEnvelope, encodeEnvelopeJSON, decodeEnvelopeJSON } from '../src/codec.js';
import { MessageType, generateMessageId } from '@fungal/core';

describe('SignalTransduction codec', () => {
  it('should round-trip encode/decode a binary envelope', () => {
    const original = {
      type: MessageType.STATUS_UPDATE as number,
      senderId: 'abc123',
      targetId: '*',
      messageId: generateMessageId(),
      timestamp: Date.now(),
      ttl: 5,
      payload: new Uint8Array([1, 2, 3, 4]),
    };
    const encoded = encodeEnvelope(original as any);
    const decoded = decodeEnvelope(encoded);
    expect(decoded.type).toBe(original.type);
    expect(decoded.senderId).toBe(original.senderId);
    expect(decoded.targetId).toBe(original.targetId);
    expect(decoded.ttl).toBe(original.ttl);
    expect(Array.from(decoded.payload)).toEqual([1, 2, 3, 4]);
  });

  it('should handle empty payload', () => {
    const original = {
      type: MessageType.HELLO,
      senderId: 'test',
      targetId: '*',
      messageId: generateMessageId(),
      timestamp: Date.now(),
      ttl: 1,
      payload: new Uint8Array(0),
    };
    const encoded = encodeEnvelope(original as any);
    const decoded = decodeEnvelope(encoded);
    expect(decoded.payload.length).toBe(0);
  });

  it('should handle long sender/target IDs', () => {
    const longId = 'a'.repeat(50);
    const original = {
      type: MessageType.TASK_ASSIGNMENT,
      senderId: longId,
      targetId: longId,
      messageId: generateMessageId(),
      timestamp: Date.now(),
      ttl: 3,
      payload: new Uint8Array([0xde, 0xad]),
    };
    const encoded = encodeEnvelope(original as any);
    const decoded = decodeEnvelope(encoded);
    expect(decoded.senderId).toBe(longId);
    expect(decoded.targetId).toBe(longId);
  });

  it('should be smaller than JSON equivalent for large payloads', () => {
    const payload = new Uint8Array(512).fill(0x42);
    const envelope = {
      type: MessageType.SCORE_BROADCAST,
      senderId: 'node123',
      targetId: '*',
      messageId: generateMessageId(),
      timestamp: Date.now(),
      ttl: 5,
      payload,
    };
    const encoded = encodeEnvelope(envelope as any);
    const jsonEncoded = new TextEncoder().encode(JSON.stringify(envelope));
    expect(encoded.length).toBeLessThan(jsonEncoded.length);
  });

  it('should round-trip JSON encode/decode', () => {
    const payload = new Uint8Array([10, 20, 30]);
    const json = encodeEnvelopeJSON(MessageType.HELLO, 'sender1', 'target1', payload, 3);
    const decoded = decodeEnvelopeJSON(json);
    expect(decoded.type).toBe(MessageType.HELLO);
    expect(decoded.senderId).toBe('sender1');
    expect(decoded.targetId).toBe('target1');
    expect(decoded.ttl).toBe(3);
    expect(Array.from(decoded.payload)).toEqual([10, 20, 30]);
  });
});
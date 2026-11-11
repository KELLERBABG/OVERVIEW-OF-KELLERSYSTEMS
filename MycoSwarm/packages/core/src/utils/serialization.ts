import { generateMessageId } from './id.js';
import type { MessageEnvelope, MessageType } from '../types/message.js';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export function serialize(obj: unknown): Uint8Array {
  const json = JSON.stringify(obj, (_key, value) => {
    if (value instanceof Uint8Array) {
      return { __uint8array: true, data: Array.from(value) };
    }
    if (typeof value === 'bigint') {
      return { __bigint: true, value: value.toString() };
    }
    return value;
  });
  return encoder.encode(json);
}

export function deserialize(bytes: Uint8Array): unknown {
  const json = decoder.decode(bytes);
  return JSON.parse(json, (_key, value) => {
    if (value && typeof value === 'object' && value.__uint8array) {
      return new Uint8Array(value.data);
    }
    if (value && typeof value === 'object' && value.__bigint) {
      return BigInt(value.value);
    }
    return value;
  });
}

export function estimateSize(obj: unknown): number {
  const str = JSON.stringify(obj);
  return encoder.encode(str).length;
}

export function createEnvelope(
  type: MessageType,
  senderId: string,
  targetId: string | '*',
  payload: Uint8Array,
  ttl = 5,
): MessageEnvelope {
  return {
    type,
    senderId,
    targetId,
    messageId: generateMessageId(),
    timestamp: Date.now(),
    ttl,
    payload,
  };
}
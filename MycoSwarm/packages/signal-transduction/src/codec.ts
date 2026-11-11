import { MessageType } from '@fungal/core';
import type { MessageEnvelope } from '@fungal/core';
import { generateMessageId } from '@fungal/core';

/**
 * Binary codec for the Signal Transduction Protocol.
 *
 * Wire format (28-byte minimum header + payload):
 * ┌───────────────────────────────────────────────────────┐
 * │ Byte 0:    Message Type (1 byte)                      │
 * │ Byte 1:    Sender ID length (1 byte)                  │
 * │ Byte 2+:   Sender ID (var string)                     │
 * │ Next:      Target ID length (1 byte)                  │
 * │ Next+:     Target ID (var string)                     │
 * │ Next 8:    Timestamp (8 bytes, big-endian)            │
 * │ Next:      TTL (1 byte)                               │
 * │ Next 2:    Payload length (2 bytes, big-endian)       │
 * │ Next+:     Payload (raw bytes)                        │
 * └───────────────────────────────────────────────────────┘
 */

export function encodeEnvelope(envelope: MessageEnvelope): Uint8Array {
  const encoder = new TextEncoder();
  const senderBytes = encoder.encode(envelope.senderId);
  const targetBytes = encoder.encode(envelope.targetId);

  const headerSize = 1 + 1 + senderBytes.length + 1 + targetBytes.length + 8 + 1 + 2;
  const buf = new Uint8Array(headerSize + envelope.payload.length);
  let offset = 0;

  buf[offset] = envelope.type;
  offset += 1;

  buf[offset] = senderBytes.length;
  offset += 1;
  buf.set(senderBytes, offset);
  offset += senderBytes.length;

  buf[offset] = targetBytes.length;
  offset += 1;
  buf.set(targetBytes, offset);
  offset += targetBytes.length;

  // Timestamp: 8 bytes big-endian
  let ts = envelope.timestamp;
  for (let i = 7; i >= 0; i--) {
    buf[offset + i] = ts & 0xff;
    ts >>>= 8;
  }
  offset += 8;

  buf[offset] = envelope.ttl;
  offset += 1;

  // Payload length: 2 bytes big-endian
  const payloadLen = envelope.payload.length;
  buf[offset] = (payloadLen >> 8) & 0xff;
  offset += 1;
  buf[offset] = payloadLen & 0xff;
  offset += 1;

  buf.set(envelope.payload, offset);

  return buf;
}

export function decodeEnvelope(buf: Uint8Array): MessageEnvelope {
  let offset = 0;

  const type: number = buf[offset]!;
  offset += 1;

  const senderLen = buf[offset]!;
  offset += 1;
  const senderId = new TextDecoder().decode(buf.slice(offset, offset + senderLen));
  offset += senderLen;

  const targetLen = buf[offset]!;
  offset += 1;
  const targetId = new TextDecoder().decode(buf.slice(offset, offset + targetLen));
  offset += targetLen;

  let timestamp = 0;
  for (let i = 0; i < 8; i++) {
    timestamp = (timestamp << 8) | buf[offset]!;
    offset += 1;
  }

  const ttl = buf[offset]!;
  offset += 1;

  const payloadLen = (buf[offset]! << 8) | buf[offset + 1]!;
  offset += 2;
  const payload = buf.slice(offset, offset + payloadLen);

  return {
    type,
    senderId,
    targetId,
    messageId: generateMessageId(),
    timestamp,
    ttl,
    payload,
  } as MessageEnvelope;
}

export function encodeEnvelopeJSON(
  type: MessageType,
  senderId: string,
  targetId: string | '*',
  payload: Uint8Array,
  ttl = 5,
): string {
  return JSON.stringify({
    type,
    senderId,
    targetId,
    messageId: generateMessageId(),
    timestamp: Date.now(),
    ttl,
    payload: Array.from(payload),
  });
}

export function decodeEnvelopeJSON(json: string): MessageEnvelope {
  const parsed = JSON.parse(json);
  return {
    type: parsed.type as MessageType,
    senderId: parsed.senderId || '',
    targetId: parsed.targetId || '*',
    messageId: parsed.messageId || generateMessageId(),
    timestamp: parsed.timestamp || Date.now(),
    ttl: parsed.ttl ?? 5,
    payload: new Uint8Array(parsed.payload || []),
  } as MessageEnvelope;
}
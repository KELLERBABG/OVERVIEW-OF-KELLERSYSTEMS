import { describe, it, expect, vi } from 'vitest';
import { GossipEngine } from '../src/gossip.js';
import type { GossipTransport } from '../src/gossip.js';
import { EventBus, MessageType } from '@fungal/core';

describe('GossipEngine', () => {
  it('should deduplicate messages by ID', () => {
    const transport = createMockTransport();
    const bus = new EventBus();
    const engine = new GossipEngine(transport, bus);
    engine.start();

    const handler = vi.fn();
    bus.on('gossip:message', handler);

    const envelope = createTestEnvelope();
    engine.receive(envelope);
    engine.receive(envelope); // duplicate
    engine.receive(envelope); // duplicate

    expect(handler).toHaveBeenCalledTimes(1);
    engine.stop();
  });

  it('should forward messages with TTL > 1', () => {
    const transport = createMockTransport(['peer1', 'peer2', 'peer3']);
    const bus = new EventBus();
    const engine = new GossipEngine(transport, bus, { fanout: 2, maxTtl: 3 });

    const envelope = createTestEnvelope(3);
    engine.receive(envelope);

    // Should forward to 2 peers (fanout = 2)
    expect(transport.send).toHaveBeenCalledTimes(2);
    engine.stop();
  });

  it('should not forward messages with TTL <= 1', () => {
    const transport = createMockTransport(['peer1', 'peer2']);
    const bus = new EventBus();
    const engine = new GossipEngine(transport, bus);

    const envelope = createTestEnvelope(1);
    engine.receive(envelope);

    expect(transport.send).not.toHaveBeenCalled();
    engine.stop();
  });
});

function createMockTransport(peers: string[] = []): GossipTransport & { send: ReturnType<typeof vi.fn> } {
  const send = vi.fn();
  return {
    peerId: 'local-node',
    getConnectedPeers: () => peers.map((id) => ({ id })),
    send,
    broadcast: vi.fn(),
  };
}

function createTestEnvelope(ttl = 5) {
  return {
    type: MessageType.STATUS_UPDATE,
    senderId: 'sender1',
    targetId: '*',
    messageId: `msg-${Math.random().toString(36).slice(2)}`,
    timestamp: Date.now(),
    ttl,
    payload: new Uint8Array([0x01]),
  };
}
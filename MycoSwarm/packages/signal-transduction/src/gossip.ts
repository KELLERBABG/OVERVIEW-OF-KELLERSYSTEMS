import type { MessageEnvelope, MessageType, EventBus } from '@fungal/core';
import { generateMessageId, createEnvelope, DEFAULTS } from '@fungal/core';
import type { TransductionConfig } from './config.js';
import { encodeEnvelope } from './codec.js';

export interface GossipTransport {
  getConnectedPeers(): Array<{ id: string }>;
  send(peerId: string, data: Uint8Array): void;
  broadcast(data: Uint8Array): void;
  readonly peerId: string;
}

export class GossipEngine {
  private seenMessages = new Set<string>();
  private config: TransductionConfig;
  private transport: GossipTransport;
  private eventBus: EventBus;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;

  constructor(transport: GossipTransport, eventBus: EventBus, config?: Partial<TransductionConfig>) {
    this.transport = transport;
    this.eventBus = eventBus;
    this.config = {
      fanout: config?.fanout ?? DEFAULTS.GOSSIP_FANOUT,
      maxTtl: config?.maxTtl ?? DEFAULTS.GOSSIP_MAX_TTL,
      heartbeatIntervalMs: config?.heartbeatIntervalMs ?? DEFAULTS.GOSSIP_HEARTBEAT_MS,
    };
  }

  start(): void {
    this.heartbeatTimer = setInterval(() => {
      this.broadcastStatus();
    }, this.config.heartbeatIntervalMs);
  }

  stop(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
    this.seenMessages.clear();
  }

  receive(envelope: MessageEnvelope): void {
    if (this.seenMessages.has(envelope.messageId)) return;

    this.seenMessages.add(envelope.messageId);

    // Trim seen messages set to prevent memory leak
    if (this.seenMessages.size > 10_000) {
      const entries = Array.from(this.seenMessages);
      this.seenMessages = new Set(entries.slice(entries.length - 5_000));
    }

    // Emit to local subscribers
    this.eventBus.emit('gossip:message', envelope);

    // Forward (gossip) if TTL > 1
    if (envelope.ttl > 1) {
      envelope.ttl--;
      const targets = this.selectGossipTargets(envelope);
      const encoded = encodeEnvelope(envelope);
      for (const target of targets) {
        try {
          this.transport.send(target.id, encoded);
        } catch {
          // peer might be gone
        }
      }
    }
  }

  private selectGossipTargets(envelope: MessageEnvelope): Array<{ id: string }> {
    const candidates = this.transport
      .getConnectedPeers()
      .filter((p) => p.id !== envelope.senderId);

    // Shuffle and take fanout
    const shuffled: Array<{ id: string }> = [];
    const pool = [...candidates];
    while (pool.length > 0) {
      const idx = Math.floor(Math.random() * pool.length);
      shuffled.push(pool[idx]!);
      pool.splice(idx, 1);
    }

    return shuffled.slice(0, this.config.fanout);
  }

  broadcast(type: MessageType, payload: Uint8Array, ttl?: number): void {
    const envelope: MessageEnvelope = {
      type,
      senderId: this.transport.peerId,
      targetId: '*',
      messageId: generateMessageId(),
      timestamp: Date.now(),
      ttl: ttl ?? this.config.maxTtl,
      payload,
    };

    // Send to direct peers
    const encoded = encodeEnvelope(envelope);
    for (const peer of this.transport.getConnectedPeers()) {
      try {
        this.transport.send(peer.id, encoded);
      } catch {
        // peer might be gone
      }
    }

    this.seenMessages.add(envelope.messageId);
  }

  private broadcastStatus(): void {
    const payload = new Uint8Array([0x01]); // simple heartbeat
    this.broadcast(0x20 as MessageType, payload, 1);
  }

  getSeenCount(): number {
    return this.seenMessages.size;
  }
}
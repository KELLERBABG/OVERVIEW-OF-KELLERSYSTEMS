import { EventBus, MessageType, serialize, deserialize, createEnvelope } from '@fungal/core';
import type { MessageEnvelope, PeerDescriptor } from '@fungal/core';
import { SignalingClient } from './signaling-client.js';
import { PeerConnectionWrapper } from './peer-connection.js';
import { FullMeshTopology, SparseMeshTopology, HybridTopology } from './topology.js';
import type { TopologyStrategy } from './topology.js';
import type { MeshConfig } from './config.js';
import type { ConnectionEvent } from './types.js';
import { encodeEnvelope, decodeEnvelope } from '@fungal/signal-transduction';

export class MeshManager {
  private signalingClient: SignalingClient;
  private peers = new Map<string, PeerConnectionWrapper>();
  private config: MeshConfig;
  private _eventBus: EventBus;
  private topologyManager: TopologyStrategy;
  private _peerId: string = '';
  private incomingHandlers = new Map<string, Array<(envelope: MessageEnvelope) => void>>();

  constructor(config: MeshConfig, eventBus?: EventBus) {
    this.config = config;
    this._eventBus = eventBus ?? new EventBus();
    this.signalingClient = new SignalingClient(config);
    this.topologyManager = this.createTopology();

    this.setupSignalingHandlers();
  }

  get peerId(): string {
    return this._peerId;
  }

  get eventBus(): EventBus {
    return this._eventBus;
  }

  private createTopology(): TopologyStrategy {
    switch (this.config.topology) {
      case 'full':
        return new FullMeshTopology();
      case 'sparse':
        return new SparseMeshTopology();
      case 'hybrid':
        return new HybridTopology(0);
      default:
        return new FullMeshTopology();
    }
  }

  private setupSignalingHandlers(): void {
    this.signalingClient.onPeerList = (peerList) => {
      // Connect to peers we don't have connections to
      for (const peer of peerList) {
        if (peer.id !== this._peerId && !this.peers.has(peer.id)) {
          if (this.topologyManager.shouldConnectTo(peer.id, this.getConnectedPeerIds())) {
            this.initiateConnection(peer.id).catch(() => {
              // connection failed
            });
          }
        }
      }
    };

    this.signalingClient.onSignal = (senderId, type, payload) => {
      if (type === MessageType.OFFER || type === MessageType.ANSWER || type === MessageType.ICE_CANDIDATE) {
        this.handleIncomingSignal(senderId, type, payload).catch(() => {
          // signal handling failed
        });
      }
    };

    this.signalingClient.onDisconnect = () => {
      this._eventBus.emit('mesh:disconnected', {});
    };
  }

  async join(): Promise<void> {
    await this.signalingClient.connect();
    this._peerId = this.signalingClient.peerId;
    this._eventBus.emit('mesh:ready', { peerCount: this.peers.size });
  }

  getConnectedPeers(): PeerDescriptor[] {
    return Array.from(this.peers.entries()).map(([id, wrapper]) => ({
      id,
      connectionState: wrapper.connected ? 'connected' : 'disconnected',
      latency: 0,
      messagesExchanged: 0,
      connectedAt: Date.now(),
    }));
  }

  getConnectedPeerIds(): string[] {
    return Array.from(this.peers.keys());
  }

  on(event: string, handler: (payload: unknown) => void): void {
    this._eventBus.on(event, handler);
  }

  off(event: string, handler: (payload: unknown) => void): void {
    this._eventBus.off(event, handler);
  }

  private async initiateConnection(targetId: string): Promise<void> {
    const wrapper = new PeerConnectionWrapper(this.config);
    this.peers.set(targetId, wrapper);

    wrapper.onSignal = (signalData) => {
      // Send signal via signaling server
      const payload = serialize({ targetId, signal: signalData });
      this.signalingClient.sendSignal(targetId, MessageType.OFFER, payload);
    };

    wrapper.onConnect = () => {
      this._eventBus.emit('peer-joined', { peerId: targetId });
    };

    wrapper.onData = (data) => {
      try {
        const envelope = decodeEnvelope(data);
        this._eventBus.emit('message', { peerId: targetId, envelope });
        this.dispatchIncoming(targetId, envelope);
      } catch {
        // invalid message
      }
    };

    wrapper.onClose = () => {
      this.peers.delete(targetId);
      this._eventBus.emit('peer-left', { peerId: targetId });
      this.topologyManager.onPeerDisconnected(targetId);
    };

    await wrapper.createOffer(true);
  }

  private async handleIncomingSignal(
    senderId: string,
    _type: MessageType,
    payload: Uint8Array,
  ): Promise<void> {
    const data = deserialize(payload) as { signal: { type: string; sdp?: any; candidate?: any } };

    let wrapper = this.peers.get(senderId);
    const isNew = !wrapper;

    if (isNew) {
      wrapper = new PeerConnectionWrapper(this.config);
      this.peers.set(senderId, wrapper);

      wrapper.onSignal = (signalData) => {
        const relayPayload = serialize({ targetId: senderId, signal: signalData });
        this.signalingClient.sendSignal(senderId, MessageType.ANSWER, relayPayload);
      };

      wrapper.onConnect = () => {
        this._eventBus.emit('peer-joined', { peerId: senderId });
      };

      wrapper.onData = (data) => {
        try {
          const envelope = decodeEnvelope(data);
          this._eventBus.emit('message', { peerId: senderId, envelope });
          this.dispatchIncoming(senderId, envelope);
        } catch {
          // invalid message
        }
      };

      wrapper.onClose = () => {
        this.peers.delete(senderId);
        this._eventBus.emit('peer-left', { peerId: senderId });
        this.topologyManager.onPeerDisconnected(senderId);
      };
    }

    if (data.signal.type === 'offer' && isNew) {
      await wrapper!.createOffer(false);
    }
    await wrapper!.handleSignal(data.signal);
  }

  send(peerId: string, data: Uint8Array): void {
    const wrapper = this.peers.get(peerId);
    if (wrapper) {
      wrapper.send(data);
    }
  }

  sendEnvelope(peerId: string, envelope: MessageEnvelope): void {
    const data = encodeEnvelope(envelope);
    this.send(peerId, data);
  }

  broadcast(data: Uint8Array): void {
    for (const [id, wrapper] of this.peers) {
      if (wrapper.connected) {
        wrapper.send(data);
      }
    }
  }

  broadcastEnvelope(envelope: MessageEnvelope): void {
    const data = encodeEnvelope(envelope);
    this.broadcast(data);
  }

  onMessage(handler: (peerId: string, envelope: MessageEnvelope) => void): void {
    this._eventBus.on('message', (payload: unknown) => {
      const msg = payload as { peerId: string; envelope: MessageEnvelope };
      handler(msg.peerId, msg.envelope);
    });
  }

  private dispatchIncoming(peerId: string, envelope: MessageEnvelope): void {
    const handlers = this.incomingHandlers.get(peerId);
    if (handlers) {
      for (const handler of handlers) {
        try {
          handler(envelope);
        } catch {
          // handler error
        }
      }
    }
  }

  addIncomingHandler(peerId: string, handler: (envelope: MessageEnvelope) => void): void {
    const handlers = this.incomingHandlers.get(peerId) ?? [];
    handlers.push(handler);
    this.incomingHandlers.set(peerId, handlers);
  }

  async leave(): Promise<void> {
    for (const wrapper of this.peers.values()) {
      wrapper.destroy();
    }
    this.peers.clear();
    this.signalingClient.disconnect();
    this._eventBus.emit('mesh:left', {});
  }
}
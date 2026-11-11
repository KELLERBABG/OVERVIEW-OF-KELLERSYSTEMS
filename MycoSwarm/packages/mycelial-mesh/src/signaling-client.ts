import { MessageType, createEnvelope, serialize, deserialize } from '@fungal/core';
import { encodeEnvelopeJSON, decodeEnvelopeJSON } from '@fungal/signal-transduction';
import type { MessageEnvelope } from '@fungal/core';
import type { MeshConfig } from './config.js';

export type SignalEventType = 'hello-ack' | 'peer-list' | 'signal' | 'disconnect';

export class SignalingClient {
  private ws: WebSocket | null = null;
  private url: string;
  private config: MeshConfig;
  private _peerId: string = '';
  private reconnectAttempts = 0;
  private pendingResolve: ((value: void) => void) | null = null;
  private shouldReconnect = true;

  // Callbacks
  onPeerList: ((peers: Array<{ id: string; capabilities?: string[] }>) => void) | null = null;
  onSignal: ((senderId: string, type: MessageType, payload: Uint8Array) => void) | null = null;
  onDisconnect: (() => void) | null = null;

  constructor(config: MeshConfig) {
    this.config = config;
    this.url = config.signalingUrl;
  }

  get peerId(): string {
    return this._peerId;
  }

  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  async connect(): Promise<void> {
    this.shouldReconnect = true;

    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);
        this.ws.binaryType = 'arraybuffer';

        this.ws.onopen = () => {
          this.reconnectAttempts = 0;

          // Send HELLO
          const helloPayload = serialize({
            roomId: this.config.roomId || '',
            capabilities: [],
          });
          const msg = encodeEnvelopeJSON(MessageType.HELLO, '', '*', helloPayload);
          this.ws!.send(msg);

          // Wait for HELLO_ACK
          const checkAck = (event: MessageEvent) => {
            const envelope = decodeEnvelopeJSON(event.data as string);
            if (envelope.type === MessageType.HELLO_ACK) {
              const data = deserialize(envelope.payload) as {
                assignedId: string;
                roomId: string;
                peers: Array<{ id: string; capabilities?: string[] }>;
              };
              this._peerId = data.assignedId;
              this.onPeerList?.(data.peers);
              this.ws?.removeEventListener('message', checkAck);
              resolve();
            }
          };
          this.ws!.addEventListener('message', checkAck);
        };

        this.ws.onmessage = (event: MessageEvent) => {
          try {
            const data = event.data as string;
            const envelope = decodeEnvelopeJSON(data);

            switch (envelope.type) {
              case MessageType.HELLO_ACK: {
                const data = deserialize(envelope.payload) as {
                  assignedId: string;
                  peers: Array<{ id: string; capabilities?: string[] }>;
                };
                this._peerId = data.assignedId;
                this.onPeerList?.(data.peers);
                break;
              }
              case MessageType.PEER_LIST: {
                const data = deserialize(envelope.payload) as {
                  peers: Array<{ id: string; capabilities?: string[] }>;
                  joined?: { id: string };
                  left?: string;
                };
                if (data.peers) {
                  this.onPeerList?.(data.peers);
                }
                break;
              }
              case MessageType.OFFER:
              case MessageType.ANSWER:
              case MessageType.ICE_CANDIDATE: {
                this.onSignal?.(envelope.senderId, envelope.type, envelope.payload);
                break;
              }
            }
          } catch (err) {
            console.error('[SignalingClient] Error parsing message:', err);
          }
        };

        this.ws.onclose = () => {
          this.onDisconnect?.();
          if (this.shouldReconnect) {
            this.attemptReconnect();
          }
        };

        this.ws.onerror = () => {
          reject(new Error('WebSocket connection failed'));
        };
      } catch (err) {
        reject(err);
      }
    });
  }

  sendSignal(targetId: string, type: MessageType, payload: Uint8Array): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    const msg = encodeEnvelopeJSON(type, this._peerId, targetId, payload);
    this.ws.send(msg);
  }

  disconnect(): void {
    this.shouldReconnect = false;
    this.reconnectAttempts = this.config.maxReconnectAttempts;
    if (this.ws) {
      const goodbye = encodeEnvelopeJSON(MessageType.GOODBYE, this._peerId, '*', new Uint8Array(0));
      try {
        this.ws.send(goodbye);
      } catch {
        // ignore
      }
      this.ws.close();
      this.ws = null;
    }
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) return;
    this.reconnectAttempts++;
    setTimeout(() => {
      this.connect().catch(() => {
        // reconnect failed, will retry again
      });
    }, this.config.reconnectIntervalMs);
  }
}
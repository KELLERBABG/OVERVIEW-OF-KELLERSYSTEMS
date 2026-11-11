# Phase 1 — P2P Mesh Network (Weeks 3–5)

**Goal:** Browser nodes discover each other via a signaling server, form a WebRTC mesh, and exchange messages using the Signal Transduction Protocol.

---

## 1. Packages Involved

| Package | Responsibility |
|---------|---------------|
| `@fungal/signaling-server` | Node.js WebSocket server for WebRTC handshake |
| `@fungal/mycelial-mesh` | Browser-side WebRTC peer connection management |
| `@fungal/signal-transduction` | Lightweight binary messaging over data channels |

---

## 2. Signaling Server (`packages/signaling-server`)

### 2.1 Purpose
WebRTC requires a signaling channel for peers to exchange SDP offers/answers and ICE candidates before a direct P2P connection can form. This server is that bootstrap channel. It is intentionally **stateless** (no stored messages) and **ephemeral** (no database).

### 2.2 File Structure

```
packages/signaling-server/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts              # Entry point — starts the server
│   ├── server.ts             # WebSocket server setup (ws library)
│   ├── room.ts               # Room management (join/leave/list)
│   ├── peer.ts               # Peer session tracking
│   ├── handlers/
│   │   ├── hello.ts          # HELLO/HELLO_ACK handshake
│   │   ├── signal.ts         # OFFER/ANSWER/ICE_CANDIDATE relay
│   │   └── goodbye.ts        # GOODBYE / disconnect cleanup
│   ├── types.ts              # Server-specific types
│   └── config.ts             # Config (port, room limits, TURN URLs)
├── tests/
│   ├── room.test.ts
│   ├── handlers.test.ts
│   └── integration.test.ts
```

### 2.3 Dependencies

```bash
pnpm add @fungal/core ws dotenv
pnpm add -D @types/ws vitest
```

### 2.4 Core Data Structures

```typescript
// src/types.ts
export interface Room {
  id: string;
  peers: Map<string, PeerSession>;
  createdAt: number;
  maxPeers: number;
}

export interface PeerSession {
  id: string;
  ws: WebSocket;
  roomId: string;
  joinedAt: number;
  userAgent?: string;
  capabilities?: string[];
}
```

### 2.5 Server Implementation

#### `src/config.ts`
```typescript
export const config = {
  port: parseInt(process.env.PORT || '8765', 10),
  maxPeersPerRoom: parseInt(process.env.MAX_PEERS_PER_ROOM || '50', 10),
  roomIdLength: 8,
  heartbeatIntervalMs: 15_000,
  peerTimeoutMs: 30_000,
  turnUrls: (process.env.TURN_URLS || '').split(',').filter(Boolean),
  logLevel: process.env.LOG_LEVEL || 'info',
};
```

#### `src/server.ts` — Main server logic
```typescript
import { WebSocketServer, WebSocket } from 'ws';
import { config } from './config.js';
import { RoomManager } from './room.js';
import { handleHello } from './handlers/hello.js';
import { handleSignal } from './handlers/signal.js';
import { handleGoodbye } from './handlers/goodbye.js';

export function createServer() {
  const wss = new WebSocketServer({ port: config.port });
  const rooms = new RoomManager();

  wss.on('connection', (ws: WebSocket, req) => {
    // Each connection is temporarily anonymous until HELLO
    let peerId: string | null = null;

    ws.on('message', (data: Buffer) => {
      // Parse binary message into envelope
      const envelope = deserializeEnvelope(data);

      switch (envelope.type) {
        case MessageType.HELLO:
          peerId = handleHello(ws, envelope, rooms);
          break;
        case MessageType.OFFER:
        case MessageType.ANSWER:
        case MessageType.ICE_CANDIDATE:
          handleSignal(envelope, rooms);
          break;
        case MessageType.GOODBYE:
          handleGoodbye(peerId, rooms);
          break;
        default:
          // Unknown message type — ignore
          break;
      }
    });

    ws.on('close', () => {
      if (peerId) handleGoodbye(peerId, rooms);
    });

    ws.on('error', () => {
      if (peerId) handleGoodbye(peerId, rooms);
    });
  });

  // Heartbeat to detect dead peers
  setInterval(() => {
    rooms.cleanupStalePeers(config.peerTimeoutMs);
  }, config.heartbeatIntervalMs);

  return wss;
}
```

#### `src/room.ts` — Room Manager
```typescript
export class RoomManager {
  private rooms = new Map<string, Room>();

  createRoom(): Room { ... }
  getRoom(id: string): Room | undefined { ... }
  joinRoom(roomId: string, peer: PeerSession): Room { ... }
  leaveRoom(peerId: string): void { ... }
  getPeerList(roomId: string): PeerSession[] { ... }
  cleanupStalePeers(timeoutMs: number): void { ... }
  getStats(): { roomCount: number; peerCount: number } { ... }
}
```

**Room join flow:**
1. Client sends `HELLO` with desired roomId (or empty for random)
2. Server creates room if needed (up to maxPeersPerRoom)
3. Server adds peer to room, responds with `HELLO_ACK` containing peer list
4. Server broadcasts `PEER_LIST` to all other peers in room (new peer joined)

### 2.6 Handler Details

#### `src/handlers/hello.ts`
- Parse `HELLO` envelope → extract `roomId` and `peerId`
- Call `rooms.joinRoom(roomId, peerSession)`
- Send `HELLO_ACK` with `{ assignedId, roomId, peers: [...] }`
- Broadcast `PEER_LIST` to other room members

#### `src/handlers/signal.ts`
- Extract `targetId` from envelope
- Look up target peer in same room
- Forward the signal message verbatim
- If target not found, send error back to sender

#### `src/handlers/goodbye.ts`
- Call `rooms.leaveRoom(peerId)`
- Broadcast updated `PEER_LIST` to remaining room members
- Clean up peer session

### 2.7 Tests

```typescript
// tests/room.test.ts
describe('RoomManager', () => {
  it('should create a room with unique ID');
  it('should allow peers to join a room');
  it('should reject join when room is full');
  it('should remove peer on leave and update peer list');
  it('should clean up stale peers after timeout');
});

// tests/handlers.test.ts (use mock WebSocket)
describe('hello handler', () => {
  it('should assign a peer to a room and return peer list');
  it('should create a new room if roomId does not exist');
});

// tests/integration.test.ts
describe('signaling server integration', () => {
  it('two clients can connect, exchange offers, and disconnect');
  it('client receives updated peer list when another joins/leaves');
  it('server handles 10+ concurrent connections');
});
```

---

## 3. Mycelial Mesh (`packages/mycelial-mesh`)

### 3.1 Purpose
Browser-side library that manages WebRTC peer connections, forms the mesh topology, and provides a clean API for higher layers to send/receive messages.

### 3.2 File Structure

```
packages/mycelial-mesh/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── mesh.ts               # MeshManager — central orchestrator
│   ├── peer-connection.ts    # Wraps RTCPeerConnection
│   ├── signaling-client.ts   # WebSocket client to signaling server
│   ├── topology.ts           # Mesh topology strategies
│   ├── connection-pool.ts    # Manage pool of active connections
│   ├── types.ts
│   └── config.ts
├── tests/
│   ├── mesh.test.ts
│   ├── peer-connection.test.ts
│   └── signaling-client.test.ts
```

### 3.3 Dependencies

```bash
pnpm add @fungal/core @fungal/signal-transport
# simple-peer is optional but recommended for easier WebRTC API
pnpm add simple-peer
pnpm add -D @types/simple-peer vitest
```

### 3.4 Core Data Structures

```typescript
// src/types.ts
export interface MeshConfig {
  signalingUrl: string;
  roomId?: string;
  stunServers: string[];
  turnServers: { urls: string; username?: string; credential?: string }[];
  maxPeers: number;
  topology: 'full' | 'sparse' | 'hybrid';
  reconnectIntervalMs: number;
  maxReconnectAttempts: number;
}

export type ConnectionEvent =
  | { type: 'peer-joined'; peerId: string }
  | { type: 'peer-left'; peerId: string }
  | { type: 'connection-state'; peerId: string; state: string }
  | { type: 'message'; peerId: string; data: Uint8Array }
  | { type: 'mesh-ready' }
  | { type: 'error'; error: Error };
```

### 3.5 MeshManager Implementation

```typescript
// src/mesh.ts
export class MeshManager {
  private signalingClient: SignalingClient;
  private peers = new Map<string, PeerConnectionWrapper>();
  private config: MeshConfig;
  private eventBus: EventBus;
  private topologyManager: TopologyManager;

  constructor(config: MeshConfig, eventBus: EventBus) { ... }

  /** Connect to signaling server and join room */
  async join(): Promise<void> {
    await this.signalingClient.connect();
    const peerList = await this.signalingClient.join(this.config.roomId);
    for (const peer of peerList) {
      if (peer.id !== this.signalingClient.peerId) {
        await this.initiateConnection(peer.id);
      }
    }
    this.eventBus.emit('mesh-ready', { peerCount: this.peers.size });
  }

  /** Initiate outgoing WebRTC connection to a peer */
  private async initiateConnection(targetId: string): Promise<void> {
    const wrapper = new PeerConnectionWrapper(this.config);
    this.peers.set(targetId, wrapper);

    wrapper.on('signal', (signalData) => {
      this.signalingClient.sendSignal(targetId, signalData);
    });

    wrapper.on('connect', () => {
      this.eventBus.emit('peer-joined', { peerId: targetId });
      this.topologyManager.onPeerConnected(targetId);
    });

    wrapper.on('data', (data) => {
      this.eventBus.emit('message', { peerId: targetId, data });
    });

    wrapper.on('close', () => {
      this.peers.delete(targetId);
      this.eventBus.emit('peer-left', { peerId: targetId });
      this.topologyManager.onPeerDisconnected(targetId);
    });

    await wrapper.createOffer();
  }

  /** Handle incoming WebRTC connection from another peer */
  async handleIncomingConnection(peerId: string, signalData: unknown): Promise<void> {
    const wrapper = new PeerConnectionWrapper(this.config);
    this.peers.set(peerId, wrapper);
    // ... similar event wiring ...
    await wrapper.handleSignal(signalData);
  }

  /** Send a message to a specific peer */
  send(peerId: string, data: Uint8Array): void { ... }

  /** Broadcast to all connected peers */
  broadcast(data: Uint8Array): void {
    for (const [id, peer] of this.peers) {
      peer.send(data);
    }
  }

  /** Get list of connected peer IDs */
  getConnectedPeers(): string[] {
    return Array.from(this.peers.keys());
  }

  /** Disconnect from the mesh */
  async leave(): Promise<void> {
    for (const peer of this.peers.values()) {
      peer.destroy();
    }
    this.peers.clear();
    this.signalingClient.disconnect();
  }
}
```

### 3.6 PeerConnectionWrapper

```typescript
// src/peer-connection.ts
export class PeerConnectionWrapper {
  private pc: RTCPeerConnection;
  private dataChannel: RTCDataChannel | null = null;
  private config: MeshConfig;

  // Callbacks
  onSignal: ((data: unknown) => void) | null = null;
  onConnect: (() => void) | null = null;
  onData: ((data: Uint8Array) => void) | null = null;
  onClose: (() => void) | null = null;

  constructor(config: MeshConfig) {
    this.config = config;
    this.pc = new RTCPeerConnection({
      iceServers: [
        ...config.stunServers.map(url => ({ urls: url })),
        ...config.turnServers,
      ],
    });
    this.setupListeners();
  }

  private setupListeners(): void {
    this.pc.onicecandidate = (e) => {
      if (e.candidate) {
        this.onSignal?.({ type: 'ice', candidate: e.candidate });
      }
    };

    this.pc.ondatachannel = (e) => {
      this.setupDataChannel(e.channel);
    };

    this.pc.onconnectionstatechange = () => {
      if (this.pc.connectionState === 'disconnected' || this.pc.connectionState === 'failed') {
        this.onClose?.();
      }
    };
  }

  private setupDataChannel(channel: RTCDataChannel): void {
    this.dataChannel = channel;
    channel.binaryType = 'arraybuffer';
    channel.onopen = () => this.onConnect?.();
    channel.onclose = () => this.onClose?.();
    channel.onmessage = (e) => {
      this.onData?.(new Uint8Array(e.data as ArrayBuffer));
    };
  }

  async createOffer(): Promise<void> {
    this.dataChannel = this.pc.createDataChannel('fungal', {
      ordered: true,
    });
    this.setupDataChannel(this.dataChannel);
    const offer = await this.pc.createOffer();
    await this.pc.setLocalDescription(offer);
    this.onSignal?.({ type: 'offer', sdp: offer });
  }

  async handleSignal(data: { type: string; sdp?: RTCSessionDescriptionInit; candidate?: RTCIceCandidateInit }): Promise<void> {
    if (data.type === 'offer') {
      await this.pc.setRemoteDescription(new RTCSessionDescription(data.sdp!));
      const answer = await this.pc.createAnswer();
      await this.pc.setLocalDescription(answer);
      this.onSignal?.({ type: 'answer', sdp: answer });
    } else if (data.type === 'answer') {
      await this.pc.setRemoteDescription(new RTCSessionDescription(data.sdp!));
    } else if (data.type === 'ice') {
      await this.pc.addIceCandidate(new RTCIceCandidate(data.candidate!));
    }
  }

  send(data: Uint8Array): void {
    if (this.dataChannel?.readyState === 'open') {
      this.dataChannel.send(data);
    }
  }

  destroy(): void {
    this.dataChannel?.close();
    this.pc.close();
  }
}
```

### 3.7 Topology Strategies

```typescript
// src/topology.ts
export interface TopologyStrategy {
  /** Called when a new peer connects — should we connect to them? */
  shouldConnectTo(peerId: string, currentPeers: string[]): boolean;
  /** Called when a peer disconnects */
  onPeerDisconnected(peerId: string): void;
  /** Max number of direct connections to maintain */
  maxConnections: number;
}

/**
 * Full Mesh: every node connects to every other node.
 * Pros: O(1) latency, simple. Cons: O(n²) connections.
 */
export class FullMeshTopology implements TopologyStrategy {
  maxConnections = Infinity;
  shouldConnectTo(_peerId: string, _currentPeers: string[]): boolean { return true; }
  onPeerDisconnected(_peerId: string): void {}
}

/**
 * Sparse Mesh: each node connects to a subset.
 * Uses a consistent-hashing ring to determine neighbors.
 */
export class SparseMeshTopology implements TopologyStrategy {
  maxConnections = 6; // Each node maintains ~6 connections
  // Implementation uses Kademlia-style routing table
  shouldConnectTo(peerId: string, currentPeers: string[]): boolean {
    if (currentPeers.length < this.maxConnections) return true;
    // Replace the farthest peer in ID-space
    return this.isCloserThanFarthest(peerId, currentPeers);
  }
}

/**
 * Hybrid: full mesh for small networks (<10), sparse for larger.
 */
export class HybridTopology implements TopologyStrategy {
  private inner: TopologyStrategy;
  constructor(peerCount: number) {
    this.inner = peerCount < 10 ? new FullMeshTopology() : new SparseMeshTopology();
  }
  get maxConnections() { return this.inner.maxConnections; }
  shouldConnectTo(peerId: string, currentPeers: string[]): boolean {
    return this.inner.shouldConnectTo(peerId, currentPeers);
  }
  onPeerDisconnected(peerId: string): void { this.inner.onPeerDisconnected(peerId); }
}
```

### 3.8 SignalingClient

```typescript
// src/signaling-client.ts
export class SignalingClient {
  private ws: WebSocket | null = null;
  private url: string;
  private peerId: string = '';
  private pendingSignals = new Map<string, Array<unknown>>();
  private reconnectAttempts = 0;
  private maxReconnectAttempts: number;
  private reconnectIntervalMs: number;

  // Events
  onPeerList: ((peers: string[]) => void) | null = null;
  onSignal: ((senderId: string, data: unknown) => void) | null = null;
  onDisconnect: (() => void) | null = null;

  constructor(config: MeshConfig) { ... }

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);
      this.ws.binaryType = 'arraybuffer';

      this.ws.onopen = () => {
        this.reconnectAttempts = 0;
        this.sendEnvelope(MessageType.HELLO, {
          roomId: this.config.roomId || '',
          capabilities: [],
        });
      };

      this.ws.onmessage = (event) => {
        const envelope = deserializeEnvelope(new Uint8Array(event.data));
        this.handleEnvelope(envelope);
      };

      this.ws.onclose = () => {
        this.onDisconnect?.();
        this.attemptReconnect();
      };

      // Wait for HELLO_ACK to resolve
      this.once('hello-ack', (data) => {
        this.peerId = data.assignedId;
        this.onPeerList?.(data.peers);
        resolve();
      });
    });
  }

  private handleEnvelope(envelope: MessageEnvelope): void {
    switch (envelope.type) {
      case MessageType.HELLO_ACK: { ... }
      case MessageType.PEER_LIST: { ... }
      case MessageType.OFFER:
      case MessageType.ANSWER:
      case MessageType.ICE_CANDIDATE: {
        this.onSignal?.(envelope.senderId, envelope.payload);
        break;
      }
    }
  }

  sendSignal(targetId: string, data: unknown): void {
    this.sendEnvelope(MessageType.OFFER, { targetId, signal: data });
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) return;
    this.reconnectAttempts++;
    setTimeout(() => this.connect(), this.reconnectIntervalMs);
  }

  disconnect(): void {
    this.sendEnvelope(MessageType.GOODBYE, {});
    this.ws?.close();
  }

  private sendEnvelope(type: MessageType, payload: Record<string, unknown>): void {
    // Serialize using @fungal/core serialization utils
    const envelope = createEnvelope(type, this.peerId, '*', payload);
    this.ws?.send(serialize(envelope));
  }
}
```

---

## 4. Signal Transduction Protocol (`packages/signal-transduction`)

### 4.1 Purpose
An extremely lightweight binary messaging standard that propagates status changes (node goes offline, score updates) faster than HTTP requests could. Think of it as a gossip protocol optimized for browser P2P.

### 4.2 File Structure

```
packages/signal-transduction/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── codec.ts             # Binary encode/decode for envelopes
│   ├── gossip.ts            # Gossip propagation engine
│   ├── types.ts             # Signal-specific type aliases
│   └── config.ts
├── tests/
│   ├── codec.test.ts
│   └── gossip.test.ts
```

### 4.3 Binary Codec

Messages use a simple binary format (not JSON) to minimize overhead:

```
┌───────────────────────────────────────────────────────┐
│ Byte 0:    Message Type (1 byte)                      │
│ Byte 1-8:  Sender ID length (1 byte) + ID (var str)  │
│ Byte 9-16: Target ID length (1 byte) + ID (var str)   │
│ Byte 17-24: Timestamp (8 bytes, big-endian)           │
│ Byte 25:   TTL (1 byte)                               │
│ Byte 26-27: Payload length (2 bytes, big-endian)      │
│ Byte 28+:  Payload (raw bytes)                        │
└───────────────────────────────────────────────────────┘
```

Total header: 28 bytes minimum (for 1-char IDs).

```typescript
// src/codec.ts
export function encodeEnvelope(envelope: MessageEnvelope): Uint8Array {
  const senderBytes = new TextEncoder().encode(envelope.senderId);
  const targetBytes = new TextEncoder().encode(envelope.targetId);
  const headerSize = 1 + 1 + senderBytes.length + 1 + targetBytes.length + 8 + 1 + 2;
  const buf = new Uint8Array(headerSize + envelope.payload.length);
  let offset = 0;

  buf[offset++] = envelope.type;
  buf[offset++] = senderBytes.length;
  buf.set(senderBytes, offset); offset += senderBytes.length;
  buf[offset++] = targetBytes.length;
  buf.set(targetBytes, offset); offset += targetBytes.length;
  // Timestamp: 8 bytes big-endian
  const ts = envelope.timestamp;
  for (let i = 7; i >= 0; i--) { buf[offset + i] = ts & 0xff; ts >>= 8; }
  offset += 8;
  buf[offset++] = envelope.ttl;
  // Payload length: 2 bytes
  buf[offset++] = (envelope.payload.length >> 8) & 0xff;
  buf[offset++] = envelope.payload.length & 0xff;
  buf.set(envelope.payload, offset);

  return buf;
}

export function decodeEnvelope(buf: Uint8Array): MessageEnvelope {
  let offset = 0;
  const type = buf[offset++];
  const senderLen = buf[offset++];
  const senderId = new TextDecoder().decode(buf.slice(offset, offset + senderLen));
  offset += senderLen;
  const targetLen = buf[offset++];
  const targetId = new TextDecoder().decode(buf.slice(offset, offset + targetLen));
  offset += targetLen;
  let timestamp = 0;
  for (let i = 0; i < 8; i++) { timestamp = (timestamp << 8) | buf[offset++]; }
  const ttl = buf[offset++];
  const payloadLen = (buf[offset++] << 8) | buf[offset++];
  const payload = buf.slice(offset, offset + payloadLen);

  return { type, senderId, targetId, timestamp, ttl, payload, messageId: '' };
}
```

### 4.4 Gossip Engine

```typescript
// src/gossip.ts
export interface GossipConfig {
  /** Fan-out: number of peers to forward each message to */
  fanout: number;
  /** Maximum TTL (hops) before a message is dropped */
  maxTtl: number;
  /** Interval (ms) for periodic gossip of node status */
  heartbeatInterval: number;
}

export class GossipEngine {
  private seenMessages = new Set<string>();  // Dedup by messageId
  private mesh: MeshManager;
  private config: GossipConfig;

  constructor(mesh: MeshManager, config: GossipConfig) { ... }

  /** Incoming message from the mesh */
  receive(envelope: MessageEnvelope): void {
    // Dedup
    if (this.seenMessages.has(envelope.messageId)) return;
    this.seenMessages.add(envelope.messageId);

    // Emit to local subscribers
    this.eventBus.emit('message', envelope);

    // Forward (gossip) if TTL > 0
    if (envelope.ttl > 1) {
      envelope.ttl--;
      const targets = this.selectGossipTargets(envelope);
      for (const target of targets) {
        this.mesh.send(target.id, encodeEnvelope(envelope));
      }
    }
  }

  /** Select which peers to forward to (avoid sender) */
  private selectGossipTargets(envelope: MessageEnvelope): PeerDescriptor[] {
    const candidates = this.mesh.getConnectedPeers()
      .filter(p => p.id !== envelope.senderId);
    // Shuffle and take fanout
    for (let i = candidates.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [candidates[i], candidates[j]] = [candidates[j], candidates[i]];
    }
    return candidates.slice(0, this.config.fanout);
  }

  /** Broadcast a message into the network */
  broadcast(type: MessageType, payload: Uint8Array): void {
    const envelope = createEnvelope(type, this.mesh.peerId, '*', payload, this.config.maxTtl);
    // Send to direct peers
    for (const peer of this.mesh.getConnectedPeers()) {
      this.mesh.send(peer.id, encodeEnvelope(envelope));
    }
    this.seenMessages.add(envelope.messageId);
  }
}
```

### 4.5 Codec Tests

```typescript
describe('SignalTransduction codec', () => {
  it('should round-trip encode/decode an envelope', () => {
    const original: MessageEnvelope = {
      type: MessageType.STATUS_UPDATE,
      senderId: 'abc123',
      targetId: '*',
      messageId: 'msg1',
      timestamp: Date.now(),
      ttl: 5,
      payload: new Uint8Array([1, 2, 3, 4]),
    };
    const encoded = encodeEnvelope(original);
    const decoded = decodeEnvelope(encoded);
    expect(decoded.type).toBe(original.type);
    expect(decoded.senderId).toBe(original.senderId);
    expect(decoded.payload).toEqual(original.payload);
  });

  it('should handle empty payload', () => {
    // ...
  });

  it('should be smaller than JSON equivalent', () => {
    const envelope = createTestEnvelope(512);
    const encoded = encodeEnvelope(envelope);
    const jsonEncoded = new TextEncoder().encode(JSON.stringify(envelope));
    expect(encoded.length).toBeLessThan(jsonEncoded.length);
  });
});
```

---

## 5. Integration: Tying It Together

### 5.1 Mesh + Signaling + Gossip Integration Test

```typescript
// In packages/mycelial-mesh/tests/integration.test.ts
describe('Full mesh integration', () => {
  it('three browser-like nodes can form a mesh', async () => {
    // 1. Start signaling server on random port
    const server = createServer({ port: 0 }); // random port
    const port = (server.address() as any).port;

    // 2. Create three mesh instances (simulating browsers)
    const mesh1 = new MeshManager({ signalingUrl: `ws://localhost:${port}`, ... });
    const mesh2 = new MeshManager({ signalingUrl: `ws://localhost:${port}`, ... });
    const mesh3 = new MeshManager({ signalingUrl: `ws://localhost:${port}`, ... });

    // 3. All join the same room
    await Promise.all([mesh1.join(), mesh2.join(), mesh3.join()]);

    // 4. Verify each node sees 2 peers
    expect(mesh1.getConnectedPeers()).toHaveLength(2);
    expect(mesh2.getConnectedPeers()).toHaveLength(2);
    expect(mesh3.getConnectedPeers()).toHaveLength(2);

    // 5. Send a message from node 1 to node 2
    const msg = new Uint8Array([0xDE, 0xAD]);
    const received = new Promise<void>((resolve) => {
      mesh2.on('message', ({ peerId, data }) => {
        expect(peerId).toBe(mesh1.peerId);
        expect(data).toEqual(msg);
        resolve();
      });
    });
    mesh1.send(mesh2.peerId, msg);
    await received;

    // 6. Gossip broadcast reaches all nodes
    const gossipReceived = Promise.all([
      new Promise(r => mesh2.on('message', r)),
      new Promise(r => mesh3.on('message', r)),
    ]);
    mesh1.broadcast(MessageType.STATUS_UPDATE, new Uint8Array([0x01]));
    const results = await gossipReceived;
    expect(results).toHaveLength(2);

    // Cleanup
    await Promise.all([mesh1.leave(), mesh2.leave(), mesh3.leave()]);
    server.close();
  });
});
```

---

## 6. Development Workflow for Phase 1

### Step-by-step build order

1. **Week 3: Signaling Server**
   - Day 1–2: Scaffold `@fungal/signaling-server`, implement `RoomManager`
   - Day 3: Implement handlers (hello, signal, goodbye)
   - Day 4: Add heartbeat + stale peer cleanup
   - Day 5: Unit tests + first manual test with `wscat` or browser DevTools

2. **Week 4: Mycelial Mesh**
   - Day 1–2: Scaffold `@fungal/mycelial-mesh`, implement `SignalingClient`
   - Day 3–4: Implement `PeerConnectionWrapper` (WebRTC setup)
   - Day 5: Implement `MeshManager` (orchestration)

3. **Week 5: Signal Transduction + Integration**
   - Day 1–2: Implement binary codec + gossip engine
   - Day 3: Integration test — 3 nodes forming a mesh
   - Day 4: Edge cases — reconnect, peer timeout, room full
   - Day 5: Benchmark latency vs HTTP baseline, document results

### Testing strategy

| Test type | What it covers | Tool |
|-----------|---------------|------|
| Unit | RoomManager logic, codec round-trip, topology decisions | Vitest |
| Integration | Full mesh formation with 3–10 nodes | Vitest + ws (in-memory server) |
| E2E | Browser tabs forming a real WebRTC mesh | Playwright (headless Chrome) |
| Performance | Message latency vs payload size, gossip overhead | Benchmark.js |

---

## 7. Deliverables Checklist (End of Phase 1)

- [ ] Signaling server running on configurable port, WebSocket-based
- [ ] Room management with join/leave/list and max peer limits
- [ ] HELLO/HELLO_ACK handshake implemented
- [ ] OFFER/ANSWER/ICE_CANDIDATE relay between peers
- [ ] Heartbeat + stale peer cleanup
- [ ] MeshManager with connect/disconnect/send/broadcast API
- [ ] PeerConnectionWrapper wrapping WebRTC lifecycle
- [ ] Full-mesh and sparse-mesh topology strategies
- [ ] SignalingClient with automatic reconnection
- [ ] Binary codec (encode/decode) for signal transduction
- [ ] GossipEngine with fan-out, TTL, dedup
- [ ] Integration test: 3+ nodes form mesh and exchange messages
- [ ] Integration test: gossip broadcast reaches all nodes
- [ ] Integration test: node disconnect propagates to mesh
- [ ] Latency benchmark: gossip vs HTTP baseline
- [ ] All `@fungal/signaling-server` tests passing
- [ ] All `@fungal/mycelial-mesh` tests passing
- [ ] All `@fungal/signal-transduction` tests passing
- [ ] `pnpm build` succeeds across all packages

---

## 8. Known Edge Cases & Handling

| Edge case | Handling |
|-----------|----------|
| Two nodes send offer simultaneously | Both create offers; peer that receives offer while expecting answer handles it gracefully |
| Peer disconnects during signaling | Signaling client reconnects; ongoing WebRTC connections remain (they're P2P) |
| Room full | Signaling server rejects with `ROOM_FULL` error code; client can retry or create new room |
| All ICE candidates fail | PeerConnectionWrapper reports connection failure; MeshManager removes peer and retries later |
| Message too large for data channel | Chunk large messages (>16KB) into fragments with sequence numbers; reassemble on receiver |
| Browser doesn't support WebRTC | Feature detection at mesh entry; fallback to a relay-node mode (data relayed through server) |

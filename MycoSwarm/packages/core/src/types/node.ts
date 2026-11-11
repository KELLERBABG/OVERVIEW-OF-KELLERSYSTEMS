/** Unique node identifier (nanoid, 21 chars) */
export type NodeId = string;

export enum NodeRole {
  LEAF = 'leaf',
  RELAY = 'relay',
  BOOTSTRAP = 'bootstrap',
}

export enum NodeStatus {
  OFFLINE = 'offline',
  ONLINE = 'online',
  BUSY = 'busy',
  DEGRADED = 'degraded',
}

export interface NodeInfo {
  id: NodeId;
  role: NodeRole;
  status: NodeStatus;
  version: string;
  /** Public key for crypto operations (hex string) */
  publicKey: string;
  /** List of capabilities advertised by this node */
  capabilities: string[];
  /** Human-readable label (optional) */
  label?: string;
}

export interface PeerDescriptor {
  id: NodeId;
  connectionState: 'new' | 'connecting' | 'connected' | 'disconnected' | 'failed';
  /** Round-trip time in ms (latest measurement) */
  latency: number;
  /** Number of messages exchanged */
  messagesExchanged: number;
  /** When the connection was established (epoch ms) */
  connectedAt: number;
}
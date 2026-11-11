import type { WebSocket } from 'ws';

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
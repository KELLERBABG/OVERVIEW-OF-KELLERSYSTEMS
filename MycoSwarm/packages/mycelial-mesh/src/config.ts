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

export const defaultConfig: MeshConfig = {
  signalingUrl: 'ws://localhost:8765',
  stunServers: ['stun:stun.l.google.com:19302'],
  turnServers: [],
  maxPeers: 50,
  topology: 'full',
  reconnectIntervalMs: 3_000,
  maxReconnectAttempts: 5,
};
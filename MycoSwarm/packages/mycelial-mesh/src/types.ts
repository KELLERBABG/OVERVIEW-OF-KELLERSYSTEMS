export interface ConnectionEvent {
  type: 'peer-joined' | 'peer-left' | 'connection-state' | 'message' | 'mesh-ready' | 'error';
  peerId?: string;
  state?: string;
  data?: Uint8Array;
  error?: Error;
}
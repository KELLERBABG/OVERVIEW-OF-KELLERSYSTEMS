import type { MessageType, NodeId } from '@fungal/core';

export interface SignalMessage {
  type: MessageType;
  senderId: NodeId;
  targetId: NodeId | '*';
  timestamp: number;
  ttl: number;
  payload: Uint8Array;
}
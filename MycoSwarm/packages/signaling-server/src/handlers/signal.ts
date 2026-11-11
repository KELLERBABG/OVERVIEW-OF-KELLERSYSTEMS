import { MessageType } from '@fungal/core';
import { RoomManager } from '../room.js';
import { makeMsg } from '../server.js';

/**
 * Relay signaling messages (OFFER, ANSWER, ICE_CANDIDATE) to the target peer.
 */
export function handleSignal(
  envelope: {
    type: MessageType;
    senderId: string;
    targetId: string;
    payload: Uint8Array;
  },
  rooms: RoomManager,
): void {
  // Extract targetId from envelope
  const targetId = envelope.targetId;

  // Find which room the sender is in
  for (const room of (rooms as any).rooms.values()) {
    if (room.peers.has(envelope.senderId)) {
      const target = room.peers.get(targetId);
      if (target) {
        const relay = makeMsg(envelope.type, envelope.senderId, targetId, {
          data: Array.from(envelope.payload),
        });
        try {
          target.ws.send(relay);
        } catch {
          // target disconnected
        }
      }
      return;
    }
  }
}
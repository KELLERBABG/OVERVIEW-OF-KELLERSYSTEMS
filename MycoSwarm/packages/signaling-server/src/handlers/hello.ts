import type { WebSocket } from 'ws';
import { MessageType, serialize, deserialize, generateMessageId, generateNodeId } from '@fungal/core';
import { RoomManager } from '../room.js';
import type { PeerSession } from '../types.js';
import { makeMsg } from '../server.js';

export function handleHello(
  ws: WebSocket,
  envelope: { type: MessageType; senderId: string; targetId: string; payload: Uint8Array },
  rooms: RoomManager,
): string {
  const payload = deserialize(envelope.payload) as { roomId?: string; capabilities?: string[] };
  const peerId = generateNodeId();
  const roomId = payload.roomId || 'default';

  const session: PeerSession = {
    id: peerId,
    ws,
    roomId,
    joinedAt: Date.now(),
    capabilities: payload.capabilities,
  };

  try {
    const room = rooms.joinRoom(roomId, session);

    const peerList = rooms.getPeerList(roomId).map((p) => ({
      id: p.id,
      capabilities: p.capabilities,
    }));

    // Send HELLO_ACK with peer list
    const ack = makeMsg(MessageType.HELLO_ACK, 'server', peerId, {
      assignedId: peerId,
      roomId,
      peers: peerList,
    });
    ws.send(ack);

    // Broadcast PEER_LIST to other members
    const broadcast = makeMsg(MessageType.PEER_LIST, 'server', '*', {
      joined: { id: peerId, capabilities: payload.capabilities },
      peers: peerList,
    });
    for (const p of room.peers.values()) {
      if (p.id !== peerId) {
        try {
          p.ws.send(broadcast);
        } catch {
          // ignore send errors
        }
      }
    }
  } catch (err) {
    const errMsg = makeMsg(MessageType.GOODBYE, 'server', peerId, {
      error: (err as Error).message,
    });
    ws.send(errMsg);
  }

  return peerId;
}
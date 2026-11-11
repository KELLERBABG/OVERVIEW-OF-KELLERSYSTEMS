import { MessageType, serialize } from '@fungal/core';
import { RoomManager } from '../room.js';
import { makeMsg } from '../server.js';

export function handleGoodbye(peerId: string | null, rooms: RoomManager): void {
  if (!peerId) return;

  // Find the room this peer is in before removing
  const roomInfo = findRoomByPeerId(peerId, rooms);
  if (!roomInfo) return;

  rooms.leaveRoom(peerId);

  // Broadcast updated peer list to remaining members
  const remainingPeers = rooms.getPeerList(roomInfo.roomId).map((p) => ({
    id: p.id,
    capabilities: p.capabilities,
  }));
  const broadcast = makeMsg(MessageType.PEER_LIST, 'server', '*', {
    left: peerId,
    peers: remainingPeers,
  });

  for (const peer of remainingPeers) {
    const session = roomInfo.room.get(peer.id);
    if (session) {
      try {
        session.ws.send(broadcast);
      } catch {
        // ignore
      }
    }
  }
}

function findRoomByPeerId(
  peerId: string,
  rooms: RoomManager,
): { roomId: string; room: Map<string, { id: string; ws: import('ws').WebSocket }> } | null {
  const roomsMap = (rooms as any).rooms as Map<string, any>;
  for (const [id, room] of roomsMap) {
    if (room.peers.has(peerId)) {
      return { roomId: id, room: room.peers };
    }
  }
  return null;
}
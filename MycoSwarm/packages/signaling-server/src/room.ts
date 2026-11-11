import { generateRoomId } from '@fungal/core';
import type { Room, PeerSession } from './types.js';
import { config } from './config.js';

export class RoomManager {
  private rooms = new Map<string, Room>();

  createRoom(): Room {
    const id = generateRoomId();
    const room: Room = {
      id,
      peers: new Map(),
      createdAt: Date.now(),
      maxPeers: config.maxPeersPerRoom,
    };
    this.rooms.set(id, room);
    return room;
  }

  getRoom(id: string): Room | undefined {
    return this.rooms.get(id);
  }

  joinRoom(roomId: string, peer: PeerSession): Room {
    let room = this.rooms.get(roomId);
    if (!room) {
      room = {
        id: roomId,
        peers: new Map(),
        createdAt: Date.now(),
        maxPeers: config.maxPeersPerRoom,
      };
      this.rooms.set(roomId, room);
    }
    if (room.peers.size >= room.maxPeers) {
      throw new Error(`Room ${roomId} is full (max ${room.maxPeers} peers)`);
    }
    room.peers.set(peer.id, peer);
    return room;
  }

  leaveRoom(peerId: string): void {
    for (const [roomId, room] of this.rooms) {
      if (room.peers.has(peerId)) {
        room.peers.delete(peerId);
        if (room.peers.size === 0) {
          this.rooms.delete(roomId);
        }
        return;
      }
    }
  }

  getPeerList(roomId: string): PeerSession[] {
    const room = this.rooms.get(roomId);
    if (!room) return [];
    return Array.from(room.peers.values());
  }

  cleanupStalePeers(timeoutMs: number): void {
    const now = Date.now();
    for (const [roomId, room] of this.rooms) {
      for (const [peerId, peer] of room.peers) {
        if (now - peer.joinedAt > timeoutMs) {
          try {
            peer.ws.close();
          } catch {
            // already closed
          }
          room.peers.delete(peerId);
        }
      }
      if (room.peers.size === 0) {
        this.rooms.delete(roomId);
      }
    }
  }

  getStats(): { roomCount: number; peerCount: number } {
    let peerCount = 0;
    for (const room of this.rooms.values()) {
      peerCount += room.peers.size;
    }
    return { roomCount: this.rooms.size, peerCount };
  }
}
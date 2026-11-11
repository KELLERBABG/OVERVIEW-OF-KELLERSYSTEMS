import { describe, it, expect, vi } from 'vitest';
import { RoomManager } from '../src/room.js';
import type { PeerSession } from '../src/types.js';

function createMockPeer(id: string): PeerSession {
  return {
    id,
    ws: { close: vi.fn(), send: vi.fn() } as any,
    roomId: 'test-room',
    joinedAt: Date.now(),
  };
}

describe('RoomManager', () => {
  it('should create a room when joining', () => {
    const rooms = new RoomManager();
    const peer = createMockPeer('peer1');
    const room = rooms.joinRoom('test-room', peer);
    expect(room.id).toBeDefined();
    expect(room.peers.size).toBe(1);
  });

  it('should allow multiple peers in the same room', () => {
    const rooms = new RoomManager();
    rooms.joinRoom('test-room', createMockPeer('peer1'));
    rooms.joinRoom('test-room', createMockPeer('peer2'));
    rooms.joinRoom('test-room', createMockPeer('peer3'));
    const peerList = rooms.getPeerList('test-room');
    expect(peerList).toHaveLength(3);
  });

  it('should remove peer on leave', () => {
    const rooms = new RoomManager();
    rooms.joinRoom('test-room', createMockPeer('peer1'));
    rooms.joinRoom('test-room', createMockPeer('peer2'));
    rooms.leaveRoom('peer1');
    const peerList = rooms.getPeerList('test-room');
    expect(peerList).toHaveLength(1);
    expect(peerList[0]?.id).toBe('peer2');
  });

  it('should delete room when last peer leaves', () => {
    const rooms = new RoomManager();
    rooms.joinRoom('test-room', createMockPeer('peer1'));
    rooms.leaveRoom('peer1');
    const peerList = rooms.getPeerList('test-room');
    expect(peerList).toHaveLength(0);
  });

  it('should report correct stats', () => {
    const rooms = new RoomManager();
    rooms.joinRoom('room-a', createMockPeer('peer1'));
    rooms.joinRoom('room-a', createMockPeer('peer2'));
    rooms.joinRoom('room-b', createMockPeer('peer3'));
    const stats = rooms.getStats();
    expect(stats.roomCount).toBe(2);
    expect(stats.peerCount).toBe(3);
  });

  it('should clean up stale peers', () => {
    const rooms = new RoomManager();
    const oldPeer = createMockPeer('old-peer');
    oldPeer.joinedAt = Date.now() - 100_000;
    rooms.joinRoom('test-room', oldPeer);
    rooms.joinRoom('test-room', createMockPeer('new-peer'));
    rooms.cleanupStalePeers(50_000);
    const peerList = rooms.getPeerList('test-room');
    expect(peerList).toHaveLength(1);
    expect(peerList[0]?.id).toBe('new-peer');
  });
});
import { WebSocketServer, WebSocket } from 'ws';
import { config } from './config.js';
import { RoomManager } from './room.js';
import { handleHello } from './handlers/hello.js';
import { handleSignal } from './handlers/signal.js';
import { handleGoodbye } from './handlers/goodbye.js';
import { MessageType, serialize, generateMessageId } from '@fungal/core';

export function makeMsg(
  type: MessageType,
  sender: string,
  target: string,
  payload: Record<string, unknown>,
): string {
  return JSON.stringify({
    type,
    senderId: sender,
    targetId: target,
    messageId: generateMessageId(),
    timestamp: Date.now(),
    ttl: 5,
    payload: Array.from(serialize(payload)),
  });
}

export function createServer() {
  const wss = new WebSocketServer({ port: config.port });
  const rooms = new RoomManager();

  console.log(`[Signaling Server] Starting on port ${config.port}`);

  wss.on('connection', (ws: WebSocket, _req) => {
    let peerId: string | null = null;

    ws.on('message', (data: Buffer) => {
      try {
        const envelope = parseMessage(data);

        switch (envelope.type) {
          case MessageType.HELLO:
            peerId = handleHello(ws, envelope, rooms);
            break;
          case MessageType.OFFER:
          case MessageType.ANSWER:
          case MessageType.ICE_CANDIDATE:
            if (peerId) {
              envelope.senderId = peerId;
              handleSignal(envelope, rooms);
            }
            break;
          case MessageType.GOODBYE:
            handleGoodbye(peerId, rooms);
            peerId = null;
            break;
          default:
            // Unknown message type — ignore
            break;
        }
      } catch (err) {
        console.error('[Signaling Server] Error processing message:', err);
      }
    });

    ws.on('close', () => {
      if (peerId) {
        handleGoodbye(peerId, rooms);
        peerId = null;
      }
    });

    ws.on('error', () => {
      if (peerId) {
        handleGoodbye(peerId, rooms);
        peerId = null;
      }
    });
  });

  // Heartbeat to detect dead peers
  const heartbeatInterval = setInterval(() => {
    const before = rooms.getStats().peerCount;
    rooms.cleanupStalePeers(config.peerTimeoutMs);
    const after = rooms.getStats().peerCount;
    if (before !== after) {
      console.log(`[Signaling Server] Cleaned up ${before - after} stale peers`);
    }
  }, config.heartbeatIntervalMs);

  return {
    wss,
    rooms,
    stop: () => {
      clearInterval(heartbeatInterval);
      wss.close();
    },
  };
}

function parseMessage(data: Buffer): {
  type: MessageType;
  senderId: string;
  targetId: string;
  payload: Uint8Array;
} {
  const text = data.toString('utf-8');
  const parsed = JSON.parse(text);
  return {
    type: parsed.type as MessageType,
    senderId: parsed.senderId || '',
    targetId: parsed.targetId || '*',
    payload: new Uint8Array(parsed.payload || []),
  };
}
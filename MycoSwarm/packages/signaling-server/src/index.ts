import { createServer } from './server.js';
import { config } from './config.js';

const server = createServer();

console.log(`🐚 Fungal Compute Swarm — Signaling Server`);
console.log(`   Listening on port ${config.port}`);
console.log(`   Max peers per room: ${config.maxPeersPerRoom}`);
console.log(`   Heartbeat interval: ${config.heartbeatIntervalMs}ms`);
console.log(`   Peer timeout: ${config.peerTimeoutMs}ms`);

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('\n[Signaling Server] Shutting down...');
  server.stop();
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.log('\n[Signaling Server] Shutting down...');
  server.stop();
  process.exit(0);
});

export { createServer, config };
export type { Room, PeerSession } from './types.js';
export { RoomManager } from './room.js';
export const config = {
  port: parseInt(process.env['PORT'] || '8765', 10),
  maxPeersPerRoom: parseInt(process.env['MAX_PEERS_PER_ROOM'] || '50', 10),
  roomIdLength: 8,
  heartbeatIntervalMs: 15_000,
  peerTimeoutMs: 30_000,
  turnUrls: (process.env['TURN_URLS'] || '').split(',').filter(Boolean),
  logLevel: process.env['LOG_LEVEL'] || 'info',
};
export const DEFAULTS = {
  /** Signaling server default port */
  SIGNALING_PORT: 8765,
  /** Max peers per room */
  MAX_PEERS_PER_ROOM: 50,
  /** Heartbeat interval (ms) */
  HEARTBEAT_INTERVAL_MS: 15_000,
  /** Peer timeout (ms) before considered stale */
  PEER_TIMEOUT_MS: 30_000,
  /** Default gossip fan-out */
  GOSSIP_FANOUT: 3,
  /** Max gossip TTL (hops) */
  GOSSIP_MAX_TTL: 5,
  /** Default gossip heartbeat interval (ms) */
  GOSSIP_HEARTBEAT_MS: 30_000,
  /** Max execution time per work unit (ms) */
  MAX_EXECUTION_TIME_MS: 30_000,
  /** Max sandbox memory (bytes) */
  MAX_SANDBOX_MEMORY: 64 * 1024 * 1024,
  /** Default Bio-Score broadcast interval (ms) */
  SCORE_BROADCAST_INTERVAL_MS: 30_000,
  /** Default redundancy reliability target */
  DEFAULT_RELIABILITY: 0.95,
  /** Default historical dropout rate (before data) */
  DEFAULT_DROPOUT_RATE: 0.3,
  /** Max redundancy factor */
  MAX_REDUNDANCY: 5,
  /** Voting window (ms) */
  VOTING_WINDOW_MS: 30_000,
  /** Saturation threshold for admission control */
  SATURATION_HIGH: 0.9,
  /** Saturation threshold for full rejection */
  SATURATION_FULL: 0.95,
  /** Nanoid length for IDs */
  ID_LENGTH: 21,
} as const;

export const MAX_MESSAGE_SIZE = 16 * 1024; // 16KB max per data channel message
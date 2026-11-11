import type { NodeId } from './node.js';

export enum MessageType {
  // Discovery
  HELLO = 0x01,
  HELLO_ACK = 0x02,
  GOODBYE = 0x03,
  PEER_LIST = 0x04,

  // Signaling (WebRTC)
  OFFER = 0x10,
  ANSWER = 0x11,
  ICE_CANDIDATE = 0x12,

  // Status
  STATUS_UPDATE = 0x20,
  SCORE_BROADCAST = 0x21,

  // Task
  TASK_ASSIGNMENT = 0x30,
  TASK_ACCEPT = 0x31,
  TASK_REJECT = 0x32,
  TASK_RESULT = 0x33,
  TASK_PROGRESS = 0x34,

  // Consensus
  VOTE = 0x40,
  PROPOSAL = 0x41,
  COMMIT = 0x42,

  // Governance
  TOKEN_TRANSFER = 0x50,
  STAKE_UPDATE = 0x51,
}

export interface MessageEnvelope {
  type: MessageType;
  senderId: NodeId;
  targetId: NodeId | '*';
  messageId: string;
  /** Unix timestamp (ms) */
  timestamp: number;
  /** Time-to-live in hops (for gossip) */
  ttl: number;
  /** Optional signature (hex string) */
  signature?: string;
  payload: Uint8Array;
}
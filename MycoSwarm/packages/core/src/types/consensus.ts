import type { NodeId } from './node.js';

export interface Proposal {
  id: string;
  type: ProposalType;
  proposerId: NodeId;
  subject: string;
  payload: unknown;
  timestamp: number;
  deadline: number;
  status: 'active' | 'accepted' | 'rejected' | 'expired';
}

export enum ProposalType {
  PRIORITY_CHANGE = 'priority_change',
  TASK_QUEUE_REORDER = 'task_queue_reorder',
  NODE_EXPULSION = 'node_expulsion',
  CONFLICT_RESOLUTION = 'conflict_resolution',
  SATURATION_REPORT = 'saturation_report',
}

export interface Vote {
  proposalId: string;
  voterId: NodeId;
  weight: number;
  decision: 'approve' | 'reject' | 'abstain';
  signature: string;
  timestamp: number;
}

export interface SaturationMetrics {
  busyRatio: number;
  localQueueDepth: number;
  avgAvailableScore: number;
  avgCompletionTime: number;
  reportedAt: number;
}

export interface ConsensusState {
  activeProposals: number;
  pendingVotes: number;
  networkSaturation: number;
  lastBlockHeight?: number;
}
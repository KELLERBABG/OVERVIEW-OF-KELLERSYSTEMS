import type { Proposal, Vote, ProposalType, NodeId } from '@fungal/core';
import { generateProposalId } from '@fungal/core';
import type { EventBus } from '@fungal/core';

export interface NodeRegistry {
  getScore(nodeId: string): number;
  getActiveNodeCount(): number;
  getTotalScore(): number;
}

export class VotingEngine {
  private activeProposals = new Map<string, Proposal>();
  private votes = new Map<string, Vote[]>();
  private nodeRegistry: NodeRegistry;
  private eventBus: EventBus;
  private config: { votingWindowMs: number; quorumRatio: number; approvalRatio: number };

  constructor(
    nodeRegistry: NodeRegistry,
    eventBus: EventBus,
    config?: Partial<{ votingWindowMs: number; quorumRatio: number; approvalRatio: number }>,
  ) {
    this.nodeRegistry = nodeRegistry;
    this.eventBus = eventBus;
    this.config = {
      votingWindowMs: 30_000,
      quorumRatio: 0.5,
      approvalRatio: 0.66,
      ...config,
    };
  }

  createProposal(
    type: ProposalType,
    subject: string,
    payload: unknown,
    proposerId: NodeId,
  ): Proposal {
    const proposal: Proposal = {
      id: generateProposalId(),
      type,
      proposerId,
      subject,
      payload,
      timestamp: Date.now(),
      deadline: Date.now() + this.config.votingWindowMs,
      status: 'active',
    };
    this.activeProposals.set(proposal.id, proposal);

    // Auto-expire after voting window
    setTimeout(() => {
      const p = this.activeProposals.get(proposal.id);
      if (p && p.status === 'active') {
        p.status = 'expired';
        this.activeProposals.delete(proposal.id);
        this.votes.delete(proposal.id);
        this.eventBus.emit('proposal:expired', { proposalId: proposal.id });
      }
    }, this.config.votingWindowMs);

    return proposal;
  }

  castVote(
    proposalId: string,
    voterId: NodeId,
    decision: 'approve' | 'reject' | 'abstain',
  ): void {
    const proposal = this.activeProposals.get(proposalId);
    if (!proposal || proposal.status !== 'active') return;

    const weight = this.nodeRegistry.getScore(voterId) ?? 1;
    const vote: Vote = {
      proposalId,
      voterId,
      weight,
      decision,
      signature: '',
      timestamp: Date.now(),
    };

    const proposalVotes = this.votes.get(proposalId) ?? [];
    // Don't allow duplicate votes
    if (proposalVotes.some((v) => v.voterId === voterId)) return;
    proposalVotes.push(vote);
    this.votes.set(proposalId, proposalVotes);

    this.checkProposal(proposal);
  }

  private checkProposal(proposal: Proposal): void {
    const votes = this.votes.get(proposal.id) ?? [];
    const totalWeight = this.nodeRegistry.getTotalScore();
    if (totalWeight <= 0) return;

    const votedWeight = votes.reduce((s, v) => s + v.weight, 0);

    // Quorum: >50% of total weight (strict)
    if (votedWeight / totalWeight <= this.config.quorumRatio) return;

    const nonAbstain = votes.filter((v) => v.decision !== 'abstain');
    const approvals = nonAbstain.filter((v) => v.decision === 'approve');
    const approvalWeight = approvals.reduce((s, v) => s + v.weight, 0);
    const nonAbstainWeight = nonAbstain.reduce((s, v) => s + v.weight, 0);

    const approvalRatio = nonAbstainWeight > 0 ? approvalWeight / nonAbstainWeight : 0;
    proposal.status = approvalRatio >= this.config.approvalRatio ? 'accepted' : 'rejected';

    this.eventBus.emit('proposal:resolved', { proposal });
    this.activeProposals.delete(proposal.id);
    this.votes.delete(proposal.id);
  }

  getActiveProposalCount(): number {
    return this.activeProposals.size;
  }

  getProposal(id: string): Proposal | undefined {
    return this.activeProposals.get(id);
  }

  getVotes(proposalId: string): Vote[] {
    return this.votes.get(proposalId) ?? [];
  }
}
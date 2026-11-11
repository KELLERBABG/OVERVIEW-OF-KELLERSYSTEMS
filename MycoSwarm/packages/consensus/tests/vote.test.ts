import { describe, it, expect, vi } from 'vitest';
import { VotingEngine, type NodeRegistry } from '../src/vote.js';
import { EventBus } from '@fungal/core';

function createRegistry(scores: Record<string, number>): NodeRegistry {
  let total = Object.values(scores).reduce((a, b) => a + b, 0);
  return {
    getScore: (id: string) => scores[id] ?? 1,
    getActiveNodeCount: () => Object.keys(scores).length,
    getTotalScore: () => total,
  };
}

describe('VotingEngine', () => {
  it('should accept proposal with >66% approval', () => {
    const bus = new EventBus();
    const registry = createRegistry({ a: 100, b: 100 });
    const engine = new VotingEngine(registry, bus);

    const proposal = engine.createProposal(
      'PRIORITY_CHANGE' as any,
      'test',
      {},
      'a',
    );
    engine.castVote(proposal.id, 'a', 'approve');
    engine.castVote(proposal.id, 'b', 'approve');
    expect(proposal.status).toBe('accepted');
  });

  it('should reject proposal with insufficient approval', () => {
    const bus = new EventBus();
    const registry = createRegistry({ a: 100, b: 100 });
    const engine = new VotingEngine(registry, bus);

    const proposal = engine.createProposal(
      'PRIORITY_CHANGE' as any,
      'test',
      {},
      'a',
    );
    engine.castVote(proposal.id, 'a', 'approve');
    engine.castVote(proposal.id, 'b', 'reject');
    expect(proposal.status).toBe('rejected');
  });

  it('should not count abstain votes in approval ratio', () => {
    const bus = new EventBus();
    // 4 nodes: a=approve(100), b=approve(100), c=abstain(100), d=reject(100)
    // If abstain counted as reject → 200/400=50% < 66% → rejected
    // If abstain excluded → 200/300=66.7% ≥ 66% → accepted
    const registry = createRegistry({ a: 100, b: 100, c: 100, d: 100 });
    const engine = new VotingEngine(registry, bus);

    const proposal = engine.createProposal(
      'PRIORITY_CHANGE' as any,
      'test',
      {},
      'a',
    );
    engine.castVote(proposal.id, 'a', 'approve');
    engine.castVote(proposal.id, 'b', 'approve');
    engine.castVote(proposal.id, 'c', 'abstain');
    engine.castVote(proposal.id, 'd', 'reject');
    // 4/4 voted (400/400=100% quorum)
    // Non-abstain: approve=200, reject=100 → 200/300=66.7% ≥ 66% → accepted
    // If abstain counted as reject: 200/400=50% → rejected. Hence abstain NOT counted.
    expect(proposal.status).toBe('accepted');
  });

  it('should prevent duplicate votes', () => {
    const bus = new EventBus();
    const registry = createRegistry({ a: 100, b: 100 });
    const engine = new VotingEngine(registry, bus);

    const proposal = engine.createProposal(
      'PRIORITY_CHANGE' as any,
      'test',
      {},
      'a',
    );
    engine.castVote(proposal.id, 'a', 'approve');
    engine.castVote(proposal.id, 'a', 'approve'); // duplicate
    expect(engine.getVotes(proposal.id)).toHaveLength(1);
  });

  it('should emit event on proposal resolution', () => {
    const bus = new EventBus();
    const handler = vi.fn();
    bus.on('proposal:resolved', handler);
    const registry = createRegistry({ a: 100, b: 100 });
    const engine = new VotingEngine(registry, bus);

    const proposal = engine.createProposal(
      'PRIORITY_CHANGE' as any,
      'test',
      {},
      'a',
    );
    engine.castVote(proposal.id, 'a', 'approve');
    engine.castVote(proposal.id, 'b', 'approve');
    expect(handler).toHaveBeenCalled();
  });
});
import { describe, it, expect } from 'vitest';
import { SybilResistance } from '../src/hardening/sybil-resistance.js';

describe('SybilResistance', () => {
  it('should return high score for unique IP with stake', async () => {
    const sr = new SybilResistance();
    const score = await sr.evaluateNode('node1', '1.2.3.4', BigInt(1000));
    expect(score).toBeGreaterThan(0.5);
  });

  it('should penalize nodes from same subnet', async () => {
    const sr = new SybilResistance();
    sr.registerNode('node1', '1.2.3.4');
    sr.registerNode('node2', '1.2.3.5');
    sr.registerNode('node3', '1.2.3.6');
    sr.registerNode('node4', '1.2.3.7');
    sr.registerNode('node5', '1.2.3.8');
    const score = await sr.evaluateNode('node6', '1.2.3.9', 0n);
    expect(score).toBe(0);
  });

  it('should track node removal', () => {
    const sr = new SybilResistance();
    sr.registerNode('node1', '1.2.3.4');
    sr.registerNode('node2', '1.2.3.5');
    sr.removeNode('node1', '1.2.3.4');
    sr.removeNode('node2', '1.2.3.5');
  });
});
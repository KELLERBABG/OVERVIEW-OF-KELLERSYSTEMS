import { describe, it, expect } from 'vitest';
import { FullMeshTopology, SparseMeshTopology, HybridTopology } from '../src/topology.js';

describe('FullMeshTopology', () => {
  it('should connect to all peers', () => {
    const topology = new FullMeshTopology();
    expect(topology.shouldConnectTo('peer1', ['peer2', 'peer3'])).toBe(true);
    expect(topology.shouldConnectTo('peer4', ['peer5', 'peer6', 'peer7', 'peer8'])).toBe(true);
  });

  it('should have infinite max connections', () => {
    const topology = new FullMeshTopology();
    expect(topology.maxConnections).toBe(Infinity);
  });
});

describe('SparseMeshTopology', () => {
  it('should connect to peers up to maxConnections', () => {
    const topology = new SparseMeshTopology();
    expect(topology.shouldConnectTo('peer1', [])).toBe(true);
    expect(topology.shouldConnectTo('peer2', ['a', 'b', 'c', 'd', 'e'])).toBe(true);
    // Past 6, uses hash-based selection
    const result = topology.shouldConnectTo('peer3', ['a', 'b', 'c', 'd', 'e', 'f']);
    expect(typeof result).toBe('boolean');
  });
});

describe('HybridTopology', () => {
  it('should use full mesh for < 10 peers', () => {
    const topology = new HybridTopology(5);
    expect(topology.maxConnections).toBe(Infinity);
  });

  it('should use sparse mesh for >= 10 peers', () => {
    const topology = new HybridTopology(10);
    expect(topology.maxConnections).toBe(6);
  });
});
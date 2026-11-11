export interface TopologyStrategy {
  shouldConnectTo(peerId: string, currentPeers: string[]): boolean;
  onPeerDisconnected(peerId: string): void;
  maxConnections: number;
}

/**
 * Full Mesh: every node connects to every other node.
 * O(1) latency, O(n²) connections.
 */
export class FullMeshTopology implements TopologyStrategy {
  maxConnections = Infinity;
  shouldConnectTo(_peerId: string, _currentPeers: string[]): boolean {
    return true;
  }
  onPeerDisconnected(_peerId: string): void {}
}

/**
 * Sparse Mesh: each node connects to a limited subset of peers.
 * O(log n) latency, O(n) connections.
 */
export class SparseMeshTopology implements TopologyStrategy {
  maxConnections = 6;

  shouldConnectTo(peerId: string, currentPeers: string[]): boolean {
    if (currentPeers.length < this.maxConnections) return true;
    // Simple hash-based selection: connect to peers whose ID hash
    // is closest to ours in the ID space
    return this.isCloserThanFarthest(peerId, currentPeers);
  }

  onPeerDisconnected(_peerId: string): void {}

  private isCloserThanFarthest(peerId: string, currentPeers: string[]): boolean {
    // Compare using simple string hash
    const candidateHash = simpleHash(peerId);
    const farthest = currentPeers
      .map((id) => ({ id, hash: simpleHash(id) }))
      .sort((a, b) => b.hash - a.hash)[0];
    if (!farthest) return true;
    return candidateHash < farthest.hash;
  }
}

/**
 * Hybrid: full mesh for small networks (<10 nodes), sparse for larger.
 */
export class HybridTopology implements TopologyStrategy {
  private inner: TopologyStrategy;

  constructor(peerCount: number) {
    this.inner = peerCount < 10 ? new FullMeshTopology() : new SparseMeshTopology();
  }

  get maxConnections(): number {
    return this.inner.maxConnections;
  }

  shouldConnectTo(peerId: string, currentPeers: string[]): boolean {
    return this.inner.shouldConnectTo(peerId, currentPeers);
  }

  onPeerDisconnected(peerId: string): void {
    this.inner.onPeerDisconnected(peerId);
  }
}

function simpleHash(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash |= 0;
  }
  return Math.abs(hash);
}
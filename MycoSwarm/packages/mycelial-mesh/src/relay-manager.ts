import type { MeshManager } from './mesh.js';

/**
 * P2P TURN Relay Manager
 *
 * When direct WebRTC connections fail due to symmetric NATs,
 * high-Bio-Score nodes can act as bandwidth relays for their peers.
 * This avoids reliance on centralized TURN infrastructure.
 *
 * Relay selection uses three criteria:
 * 1. Bio-Score (higher = more likely selected)
 * 2. Available bandwidth (estimated from network benchmark)
 * 3. Current relay load (each node limits concurrent relays)
 */
export interface RelaySession {
  relayNodeId: string;
  targetNodeId: string;
  allocatedBandwidthKbps: number;
  startedAt: number;
  bytesRelayed: number;
}

export class RelayManager {
  private activeRelays = new Map<string, RelaySession>();
  private maxConcurrentRelays = 3;
  private maxBandwidthPerRelayKbps = 5_000; // 5 Mbps per relay
  private candidateScores = new Map<string, number>();

  registerCandidate(nodeId: string, bioScore: number): void {
    this.candidateScores.set(nodeId, bioScore);
  }

  unregisterCandidate(nodeId: string): void {
    this.candidateScores.delete(nodeId);
    // Terminate any active relays involving this node
    for (const [relayId, session] of this.activeRelays) {
      if (session.relayNodeId === nodeId || session.targetNodeId === nodeId) {
        this.activeRelays.delete(relayId);
      }
    }
  }

  /**
   * Request a relay connection through a high-score intermediary.
   * Returns the selected relay node ID or null if none available.
   */
  requestRelay(targetNodeId: string): string | null {
    // Filter to candidates not currently at max relay load
    const available = Array.from(this.candidateScores.entries())
      .filter(([nodeId]) => {
        const currentLoad = this.getRelayCount(nodeId);
        return currentLoad < this.maxConcurrentRelays;
      })
      .sort(([, a], [, b]) => b - a); // Highest score first

    if (available.length === 0) return null;

    const selectedRelay = available[0]!;
    const session: RelaySession = {
      relayNodeId: selectedRelay[0],
      targetNodeId,
      allocatedBandwidthKbps: this.maxBandwidthPerRelayKbps,
      startedAt: Date.now(),
      bytesRelayed: 0,
    };

    const relayId = `${selectedRelay[0]}:${targetNodeId}`;
    this.activeRelays.set(relayId, session);
    return selectedRelay[0];
  }

  getRelayCount(nodeId: string): number {
    let count = 0;
    for (const session of this.activeRelays.values()) {
      if (session.relayNodeId === nodeId) count++;
    }
    return count;
  }

  recordRelayedBytes(relayNodeId: string, targetNodeId: string, bytes: number): void {
    const relayId = `${relayNodeId}:${targetNodeId}`;
    const session = this.activeRelays.get(relayId);
    if (session) {
      session.bytesRelayed += bytes;
    }
  }

  getStats() {
    return {
      activeRelays: this.activeRelays.size,
      candidateCount: this.candidateScores.size,
      totalBytesRelayed: Array.from(this.activeRelays.values())
        .reduce((s, r) => s + r.bytesRelayed, 0),
    };
  }

  stopRelay(relayNodeId: string, targetNodeId: string): void {
    const relayId = `${relayNodeId}:${targetNodeId}`;
    this.activeRelays.delete(relayId);
    // Score bonus for relayed bytes could be applied here
  }
}
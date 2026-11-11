export interface NodeState {
  id: string;
  status: string;
  score: number;
  peers: string[];
  tasksCompleted: number;
  tokensEarned: bigint;
}

export interface MeshState {
  peerCount: number;
  saturation: number;
  avgScore: number;
  tasksQueued: number;
  tasksInProgress: number;
}

export interface WalletState {
  connected: boolean;
  address: string;
  balance: bigint;
  staked: bigint;
}

export interface TaskLogEntry {
  taskId: string;
  type: string;
  status: string;
  startedAt: number;
  completedAt?: number;
  result?: unknown;
  error?: string;
}

export class UIStateManager {
  private listeners = new Map<string, Set<() => void>>();

  nodeState: NodeState = {
    id: '', status: 'offline', score: 0, peers: [],
    tasksCompleted: 0, tokensEarned: 0n,
  };

  meshState: MeshState = {
    peerCount: 0, saturation: 0, avgScore: 0,
    tasksQueued: 0, tasksInProgress: 0,
  };

  walletState: WalletState = {
    connected: false, address: '', balance: 0n, staked: 0n,
  };

  taskLog: TaskLogEntry[] = [];

  subscribe(field: string, callback: () => void): void {
    const existing = this.listeners.get(field) ?? new Set();
    existing.add(callback);
    this.listeners.set(field, existing);
  }

  unsubscribe(field: string, callback: () => void): void {
    const existing = this.listeners.get(field);
    if (existing) {
      existing.delete(callback);
    }
  }

  updateNodeState(partial: Partial<NodeState>): void {
    this.nodeState = { ...this.nodeState, ...partial };
    this.notify('nodeState');
  }

  updateMeshState(partial: Partial<MeshState>): void {
    this.meshState = { ...this.meshState, ...partial };
    this.notify('meshState');
  }

  updateWalletState(partial: Partial<WalletState>): void {
    this.walletState = { ...this.walletState, ...partial };
    this.notify('walletState');
  }

  addLogEntry(entry: TaskLogEntry): void {
    this.taskLog = [entry, ...this.taskLog].slice(0, 100);
    this.notify('taskLog');
  }

  get isHealthy(): boolean {
    return this.meshState.saturation < 0.8;
  }

  private notify(field: string): void {
    const handlers = this.listeners.get(field);
    if (handlers) {
      for (const handler of handlers) {
        try { handler(); } catch { /* ignore */ }
      }
    }
  }
}
import { BrowserAdapter } from '@fungal/substrate-layer';
import { EventBus, generateNodeId } from '@fungal/core';
import { MeshManager } from '@fungal/mycelial-mesh';
import { WasmExecutor } from '@fungal/wasm-executor';
import { TaskDistributor } from '@fungal/task-distribution';
import { ResourceScorer, ScoreBroadcaster } from '@fungal/resource-scoring';
import { ConsensusEngine } from '@fungal/consensus';
import { TokenClient } from '@fungal/incentive-token';

export interface ClientConfig {
  signalingUrl: string;
  roomId?: string;
  nodeCount?: number;
  tokenAddress?: string;
  marketAddress?: string;
}

export interface ClientInstance {
  identity: { id: string; createdAt: number };
  mesh: MeshManager;
  scorer: ResourceScorer;
  distributor: TaskDistributor;
  consensus: ConsensusEngine;
  executor: WasmExecutor;
  walletClient: TokenClient | null;
  adapter: BrowserAdapter;
  eventBus: EventBus;
}

export async function bootClient(config: ClientConfig): Promise<ClientInstance> {
  const eventBus = new EventBus();

  const adapter = new BrowserAdapter();

  const identity = {
    id: generateNodeId(),
    createdAt: Date.now(),
  };

  const mesh = new MeshManager(
    {
      signalingUrl: config.signalingUrl,
      roomId: config.roomId,
      stunServers: ['stun:stun.l.google.com:19302'],
      turnServers: [],
      maxPeers: 50,
      topology: (config.nodeCount ?? 1) < 10 ? 'full' : 'sparse',
      reconnectIntervalMs: 3_000,
      maxReconnectAttempts: 5,
    },
    eventBus,
  );
  await mesh.join();

  const scorer = new ResourceScorer();
  const scoreBroadcaster = new ScoreBroadcaster(scorer, mesh);
  scoreBroadcaster.start();

  const meshLike = {
    peerId: mesh.peerId,
    broadcast: (data: Uint8Array) => mesh.broadcast(data),
    send: (peerId: string, data: Uint8Array) => mesh.send(peerId, data),
    onMessage: (handler: (peerId: string, envelope: any) => void) => mesh.onMessage(handler),
  };

  const nodeRegistry = {
    getScore: (_nodeId: string) => 50,
    getAvailableNodes: () => {
      const map = new Map<string, number>();
      for (const peer of mesh.getConnectedPeers()) {
        map.set(peer.id, 50);
      }
      return map;
    },
    getActiveNodeCount: () => mesh.getConnectedPeers().length,
    getTotalScore: () => mesh.getConnectedPeers().length * 50,
  };

  const distributorNodeRegistry = {
    getScore: nodeRegistry.getScore,
    getAvailableNodes: nodeRegistry.getAvailableNodes,
  };

  const distributor = new TaskDistributor(mesh, distributorNodeRegistry);

  const consensus = new ConsensusEngine(meshLike, nodeRegistry, eventBus);
  consensus.start();

  const executor = new WasmExecutor();

  let walletClient: TokenClient | null = null;
  if (config.tokenAddress) {
    walletClient = new TokenClient({
      chainId: '31337',
      tokenAddress: config.tokenAddress,
      marketAddress: config.marketAddress ?? '',
      rpcUrl: 'http://localhost:8545',
    });
  }

  return {
    identity,
    mesh,
    scorer,
    distributor,
    consensus,
    executor,
    walletClient,
    adapter,
    eventBus,
  };
}
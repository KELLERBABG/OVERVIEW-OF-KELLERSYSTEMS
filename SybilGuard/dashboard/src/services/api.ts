const API_BASE = '/api/v1';

async function fetchJSON<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${url}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  });
  if (!res.ok) throw new Error(`API error: ${res.status} ${res.statusText}`);
  return res.json();
}

export interface HealthStatus {
  status: string;
  service: string;
  version: string;
}

export interface GraphAnalysis {
  small_world: boolean;
  clustering_coefficient: number;
  average_path_length: number;
  node_count: number;
  edge_count: number;
  cc_statistics: { min: number; max: number; mean: number; std_dev: number };
  top_centrality: { node_id: string; centrality: number }[];
}

export interface SybilResult {
  regions_analyzed: number;
  suspected_sybil: number;
  details: {
    region: string;
    node_count: number;
    entropy: number;
    anomaly: number;
    confidence: number;
    suspected: boolean;
    mean_cc: number;
    internal_density: number;
    external_density: number;
  }[];
}

export interface ReputationScore {
  base_score: number;
  embedding_bonus: number;
  vouching_power: number;
  decay_factor: number;
}

export const api = {
  health: () => fetchJSON<HealthStatus>('/health'),
  graphAnalyze: () => fetchJSON<GraphAnalysis>('/graph/analyze', { method: 'POST' }),
  graphTopology: () => fetchJSON<any>('/graph/topology'),
  listNodes: () => fetchJSON<{ nodes: string[]; count: number }>('/nodes'),
  createNode: (did?: string) => fetchJSON<any>('/nodes', { method: 'POST', body: JSON.stringify({ did }) }),
  detectSybil: () => fetchJSON<SybilResult>('/sybil/detect', { method: 'POST' }),
  analyzeRegion: () => fetchJSON<any>('/sybil/analyze-region', { method: 'POST' }),
  getReputation: (id: string) => fetchJSON<ReputationScore>(`/reputation/${id}`),
  vouch: (voucher: string, subject: string, depth?: number) =>
    fetchJSON<any>('/wot/vouch', { method: 'POST', body: JSON.stringify({ voucher, subject, depth }) }),
  trustChain: (id: string) => fetchJSON<any>(`/wot/trust-chain/${id}`),
  createDid: (nodeId?: string) => fetchJSON<any>('/did', { method: 'POST', body: JSON.stringify({ node_id: nodeId }) }),
  resolveDid: (did: string) => fetchJSON<any>(`/did?did=${encodeURIComponent(did)}`),
  registerNullifier: (seed: string) => fetchJSON<any>('/nullifier', { method: 'POST', body: JSON.stringify({ seed }) }),
  verifyNullifier: (nullifier: string) => fetchJSON<any>('/nullifier/verify', { method: 'POST', body: JSON.stringify({ nullifier }) }),
};
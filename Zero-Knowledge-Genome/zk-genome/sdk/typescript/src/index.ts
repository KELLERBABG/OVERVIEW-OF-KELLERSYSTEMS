/**
 * Zero-Knowledge Genome TypeScript SDK
 * 
 * Programmatic access to the ZK Genome API for submitting
 * genomic queries and managing research studies.
 */

interface QueryResult {
  id: string;
  pharma: string;
  reward_per_proof: number;
  max_submissions: number;
  status: string;
  created_at: number;
  deadline: number;
}

interface AggregateStats {
  total_queries: number;
  total_proofs: number;
  total_value_locked: number;
  active_users: number;
}

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export class ZkGenomeClient {
  private baseUrl: string;
  private apiKey: string;

  constructor(apiKey: string, baseUrl = 'http://localhost:3000') {
    this.apiKey = apiKey;
    this.baseUrl = baseUrl;
  }

  private async request<T>(method: string, path: string, data?: unknown): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: data ? JSON.stringify(data) : undefined,
    });
    const json: ApiResponse<T> = await response.json();
    if (!json.success) throw new Error(json.error || 'API error');
    return json.data!;
  }

  async createQuery(params: {
    chrom: number;
    posRange: [number, number];
    rewardPerProof: number;
    maxSubmissions: number;
    deadline?: number;
  }): Promise<QueryResult> {
    return this.request<QueryResult>('POST', '/api/v1/queries', {
      pharma: 'researcher',
      encrypted_params: JSON.stringify({ chrom: params.chrom, pos_range: params.posRange }),
      reward_per_proof: params.rewardPerProof,
      max_submissions: params.maxSubmissions,
      deadline: params.deadline || Math.floor(Date.now() / 1000) + 30 * 86400,
    });
  }

  async getQuery(id: string): Promise<QueryResult> {
    return this.request<QueryResult>('GET', `/api/v1/queries/${id}`);
  }

  async submitProof(queryId: string, proofData: Uint8Array, circuitId: string): Promise<string> {
    return this.request<string>('POST', `/api/v1/queries/${queryId}/proofs`, {
      proof_data: Array.from(proofData).map(b => b.toString(16).padStart(2, '0')).join(''),
      public_inputs: '',
      circuit_id: circuitId,
    });
  }

  async getAggregateStats(): Promise<AggregateStats> {
    return this.request<AggregateStats>('GET', '/api/v1/stats/aggregate');
  }

  async healthCheck(): Promise<boolean> {
    try { await this.request('GET', '/health'); return true; }
    catch { return false; }
  }
}
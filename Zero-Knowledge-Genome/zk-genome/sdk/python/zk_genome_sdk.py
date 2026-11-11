"""
Zero-Knowledge Genome — Python SDK for Researchers

Provides programmatic access to the ZK Genome API for submitting
genomic queries, retrieving differentially private aggregate stats,
and managing research studies.

Usage:
    from zk_genome_sdk import ZkGenomeClient

    client = ZkGenomeClient(api_key="zk_...")
    
    # Submit a query
    query = client.create_query(
        chrom=1, pos_range=(100000, 200000),
        reward_per_proof=10.0, max_submissions=100
    )
    
    # Get aggregate stats
    stats = client.get_aggregate_stats()
"""

import json
import os
import time
import hashlib
from dataclasses import dataclass, field
from typing import Optional
from urllib.request import Request, urlopen
from urllib.error import HTTPError


class ZkGenomeError(Exception):
    """Base exception for ZK Genome SDK errors."""
    pass


@dataclass
class QueryResult:
    """Result of a genomic query submission."""
    id: str
    pharma: str
    reward_per_proof: float
    max_submissions: int
    status: str
    created_at: int
    deadline: int


@dataclass
class AggregateStats:
    """Differentially private aggregate statistics."""
    total_queries: int
    total_proofs: int
    total_value_locked: float
    active_users: int


class ZkGenomeClient:
    """
    Client for the Zero-Knowledge Genome API.
    
    Args:
        api_key: Researcher API key.
        base_url: API server base URL (default: http://localhost:3000).
        timeout: Request timeout in seconds.
    """
    
    def __init__(
        self,
        api_key: str,
        base_url: str = "http://localhost:3000",
        timeout: int = 30,
    ):
        self.api_key = api_key
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
    
    def _request(self, method: str, path: str, data: Optional[dict] = None) -> dict:
        """Make an HTTP request to the API."""
        url = f"{self.base_url}{path}"
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        body = json.dumps(data).encode() if data else None
        
        req = Request(url, data=body, headers=headers, method=method)
        try:
            with urlopen(req, timeout=self.timeout) as resp:
                return json.loads(resp.read().decode())
        except HTTPError as e:
            raise ZkGenomeError(f"API error {e.code}: {e.read().decode()}")
        except Exception as e:
            raise ZkGenomeError(f"Request failed: {e}")
    
    def create_query(
        self,
        chrom: int,
        pos_range: tuple,
        reward_per_proof: float,
        max_submissions: int,
        deadline: Optional[int] = None,
    ) -> QueryResult:
        """
        Submit a new genomic query.
        
        Args:
            chrom: Chromosome number (1-22, X=23, Y=24, MT=25).
            pos_range: (start, end) 1-based position range.
            reward_per_proof: ZKG tokens per valid proof.
            max_submissions: Maximum number of proofs to accept.
            deadline: Unix timestamp deadline (default: 30 days).
        
        Returns:
            QueryResult with the created query details.
        """
        payload = {
            "pharma": "researcher",
            "encrypted_params": json.dumps({
                "chrom": chrom,
                "pos_range": list(pos_range),
            }),
            "reward_per_proof": int(reward_per_proof * 1e18),  # 18 decimals
            "max_submissions": max_submissions,
            "deadline": deadline or int(time.time()) + 30 * 86400,
        }
        result = self._request("POST", "/api/v1/queries", payload)
        if not result.get("success"):
            raise ZkGenomeError(result.get("error", "Query failed"))
        data = result["data"]
        return QueryResult(**data)
    
    def get_query(self, query_id: str) -> QueryResult:
        """Get details of a specific query."""
        result = self._request("GET", f"/api/v1/queries/{query_id}")
        if not result.get("success"):
            raise ZkGenomeError(result.get("error", "Query not found"))
        return QueryResult(**result["data"])
    
    def submit_proof(self, query_id: str, proof_data: bytes, circuit_id: str) -> str:
        """
        Submit a zero-knowledge proof for a query.
        
        Args:
            query_id: Target query ID.
            proof_data: Serialized zk-SNARK proof bytes.
            circuit_id: Circuit identifier (has-variant, variant-count, etc.).
        
        Returns:
            Proof hash string.
        """
        payload = {
            "proof_data": proof_data.hex(),
            "public_inputs": "",
            "circuit_id": circuit_id,
        }
        result = self._request("POST", f"/api/v1/queries/{query_id}/proofs", payload)
        if not result.get("success"):
            raise ZkGenomeError(result.get("error", "Proof submission failed"))
        return result["data"]
    
    def get_aggregate_stats(self) -> AggregateStats:
        """Get differentially private aggregate statistics."""
        result = self._request("GET", "/api/v1/stats/aggregate")
        if not result.get("success"):
            raise ZkGenomeError(result.get("error", "Stats unavailable"))
        return AggregateStats(**result["data"])
    
    def health_check(self) -> bool:
        """Check if the API server is reachable."""
        try:
            self._request("GET", "/health")
            return True
        except ZkGenomeError:
            return False


# Convenience function for quick CLI usage
def main():
    """Example usage of the ZK Genome SDK."""
    import sys
    
    api_key = os.environ.get("ZK_GENOME_API_KEY", "zk_test_key")
    client = ZkGenomeClient(api_key)
    
    if not client.health_check():
        print("❌ ZK Genome API is not reachable")
        sys.exit(1)
    print("✅ Connected to ZK Genome API")
    
    # Create a query
    query = client.create_query(
        chrom=1,
        pos_range=(100_000, 200_000),
        reward_per_proof=0.1,
        max_submissions=50,
    )
    print(f"📝 Created query: {query.id}")
    
    # Get stats
    stats = client.get_aggregate_stats()
    print(f"📊 Active queries: {stats.total_queries}")
    print(f"🔬 Total proofs: {stats.total_proofs}")


if __name__ == "__main__":
    main()
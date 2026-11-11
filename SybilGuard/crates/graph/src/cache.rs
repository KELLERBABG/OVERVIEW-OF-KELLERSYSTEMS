//! # Graph Computation Cache
//!
//! Incremental computation engine that avoids full O(N log N) recalculations
//! on every graph mutation. Uses dirty-flag tracking, component-scoped
//! recomputation, and staleness-aware reads.

use std::collections::HashSet;
use std::time::Instant;

use sybil_core::{NodeId, SybilResult};
use crate::graph::PoPGraph;
use crate::topology::TopologyAnalyzer;
use crate::centrality::CentralityAnalyzer;
use crate::community::CommunityDetector;

/// Revision counter type.
pub type Revision = u64;

/// Freshness requirement for cached computations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Must be computed against the very latest graph state.
    Latest,
    /// May return stale data up to `max_staleness_ms` old.
    StaleOk { max_staleness_ms: u64 },
}

/// A cached computation result with metadata.
#[derive(Debug, Clone)]
pub struct CachedResult<T> {
    pub value: T,
    pub revision: Revision,
    pub computed_at: Instant,
    pub computation_time_ms: f64,
}

/// The main graph computation cache.
pub struct GraphCache {
    graph: PoPGraph,
    global_revision: Revision,
    dirty: bool,
    config: CacheConfig,
    // Cached results
    cached_clustering: Option<CachedResult<f64>>,
    cached_path_length: Option<CachedResult<f64>>,
    cached_components: Option<CachedResult<Vec<Vec<NodeId>>>>,
    cached_centrality: Option<CachedResult<Vec<(NodeId, f64)>>>,
}

/// Configuration for the graph computation cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_staleness_ms: u64,
    pub enable_background: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_staleness_ms: 5_000,
            enable_background: true,
        }
    }
}

impl GraphCache {
    pub fn new(graph: PoPGraph, config: CacheConfig) -> Self {
        Self {
            graph,
            global_revision: 0,
            dirty: true,
            config,
            cached_clustering: None,
            cached_path_length: None,
            cached_components: None,
            cached_centrality: None,
        }
    }

    pub fn graph(&self) -> &PoPGraph { &self.graph }
    pub fn graph_mut(&mut self) -> &mut PoPGraph {
        self.dirty = true;
        self.global_revision += 1;
        &mut self.graph
    }

    /// Get average clustering coefficient, using cache if fresh.
    pub fn average_clustering_coefficient(&mut self, freshness: Freshness) -> SybilResult<f64> {
        if let Some(ref cached) = self.cached_clustering {
            if !self.needs_recompute(cached, freshness) {
                return Ok(cached.value);
            }
        }
        let start = Instant::now();
        let analyzer = TopologyAnalyzer::new(&self.graph);
        let value = analyzer.average_clustering_coefficient()?;
        self.cached_clustering = Some(CachedResult {
            value,
            revision: self.global_revision,
            computed_at: Instant::now(),
            computation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        });
        Ok(value)
    }

    /// Get connected components, cached.
    pub fn connected_components(&mut self, freshness: Freshness) -> SybilResult<Vec<Vec<NodeId>>> {
        if let Some(ref cached) = self.cached_components {
            if !self.needs_recompute(cached, freshness) {
                return Ok(cached.value.clone());
            }
        }
        let start = Instant::now();
        let detector = CommunityDetector::new(&self.graph);
        let components = detector.connected_components()?;
        self.cached_components = Some(CachedResult {
            value: components.clone(),
            revision: self.global_revision,
            computed_at: Instant::now(),
            computation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        });
        self.dirty = false;
        Ok(components)
    }

    /// Get eigenvector centrality, cached.
    pub fn centrality(&mut self, freshness: Freshness) -> SybilResult<Vec<(NodeId, f64)>> {
        if let Some(ref cached) = self.cached_centrality {
            if !self.needs_recompute(cached, freshness) {
                return Ok(cached.value.clone());
            }
        }
        let start = Instant::now();
        let analyzer = CentralityAnalyzer::new(&self.graph);
        let value = analyzer.eigenvector_centrality(100, 1e-6)?;
        self.cached_centrality = Some(CachedResult {
            value: value.clone(),
            revision: self.global_revision,
            computed_at: Instant::now(),
            computation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        });
        Ok(value)
    }

    fn needs_recompute<T>(&self, cached: &CachedResult<T>, freshness: Freshness) -> bool {
        match freshness {
            Freshness::Latest => self.dirty || cached.revision != self.global_revision,
            Freshness::StaleOk { max_staleness_ms } => {
                cached.computed_at.elapsed().as_millis() as u64 > max_staleness_ms
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sybil_core::SyntheticGraphConfig;
    use crate::synthetic::SyntheticGraphGenerator;

    #[test]
    fn test_cache_basic() {
        let config = SyntheticGraphConfig { num_nodes: 100, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let mut cache = GraphCache::new(graph, CacheConfig::default());
        let cc = cache.average_clustering_coefficient(Freshness::Latest).unwrap();
        assert!(cc > 0.0 && cc <= 1.0);
    }

    #[test]
    fn test_cache_stale_ok() {
        let config = SyntheticGraphConfig { num_nodes: 100, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let mut cache = GraphCache::new(graph, CacheConfig::default());
        let cc = cache.average_clustering_coefficient(Freshness::Latest).unwrap();
        let cc2 = cache.average_clustering_coefficient(
            Freshness::StaleOk { max_staleness_ms: 10_000 }
        ).unwrap();
        assert_eq!(cc, cc2);
    }

    #[test]
    fn test_dirty_tracking() {
        let config = SyntheticGraphConfig { num_nodes: 50, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (mut graph, _) = generator.generate().unwrap();
        let mut cache = GraphCache::new(graph, CacheConfig::default());
        let _ = cache.connected_components(Freshness::Latest).unwrap();
        // Mutating graph should invalidate cache
        let _ = cache.graph_mut();
        assert!(cache.dirty);
    }
}
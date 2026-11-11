use std::path::PathBuf;

use sybil_core::{SocialGraph, SybilResult};

/// Persistence for the social graph (CSV and JSON formats).
pub struct GraphStorage {
    data_dir: PathBuf,
}

impl GraphStorage {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Save a social graph to a JSON file.
    pub fn save_json(&self, graph: &SocialGraph, filename: &str) -> SybilResult<()> {
        let path = self.data_dir.join(format!("{}.json", filename));
        let file = std::fs::File::create(&path)
            .map_err(|e| sybil_core::SybilError::Storage(format!("cannot create file: {}", e)))?;
        serde_json::to_writer_pretty(file, graph)
            .map_err(|e| sybil_core::SybilError::Serialization(e.to_string()))?;
        Ok(())
    }

    /// Load a social graph from a JSON file.
    pub fn load_json(&self, filename: &str) -> SybilResult<SocialGraph> {
        let path = self.data_dir.join(format!("{}.json", filename));
        let file = std::fs::File::open(&path)
            .map_err(|e| sybil_core::SybilError::Storage(format!("cannot open file: {}", e)))?;
        let graph: SocialGraph = serde_json::from_reader(file)
            .map_err(|e| sybil_core::SybilError::Deserialization(e.to_string()))?;
        Ok(graph)
    }

    /// Export edges to CSV (source, target, weight).
    pub fn export_edges_csv(&self, graph: &SocialGraph, filename: &str) -> SybilResult<()> {
        let path = self.data_dir.join(format!("{}.csv", filename));
        let mut wtr = csv::Writer::from_path(&path)
            .map_err(|e| sybil_core::SybilError::Storage(e.to_string()))?;
        wtr.write_record(&["source", "target", "weight"])
            .map_err(|e| sybil_core::SybilError::Storage(e.to_string()))?;
        for edge in &graph.edges {
            wtr.write_record(&[
                edge.source.to_string(),
                edge.target.to_string(),
                edge.weight.to_string(),
            ])
            .map_err(|e| sybil_core::SybilError::Storage(e.to_string()))?;
        }
        wtr.flush().map_err(|e| sybil_core::SybilError::Storage(e.to_string()))?;
        Ok(())
    }
}
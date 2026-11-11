//! # Regulatory & Legal Adoption Bridge
//!
//! Addresses Critique #3. Provides WIPO XML export, evidence packages,
//! compliance config, arbitration agreements.

use crate::graph::PatentDag;
use crate::models::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegalEvidencePackage {
    pub package_id: Uuid,
    pub generated_at: i64,
    pub graph_snapshot: PatentGraphState,
    pub lifecycle_log: Vec<LifecycleEvent>,
    pub deltas: Vec<DeltaConstraint>,
    pub verification_results: Vec<VerificationResult>,
    pub executive_summary: String,
    pub signature: Vec<u8>,
}

impl LegalEvidencePackage {
    pub fn from_dag(
        dag: &PatentDag, disputed_edges: &[Uuid],
        deltas: Vec<DeltaConstraint>, verification_results: Vec<VerificationResult>,
    ) -> Self {
        let state = dag.export_state();
        let log = dag.lifecycle_log().to_vec();
        let summary = Self::generate_summary(&state, disputed_edges);
        let package_id = Uuid::new_v4();
        let mut hasher = sha2::Sha256::new();
        hasher.update(package_id.as_bytes());
        hasher.update(&serde_json::to_vec(&state).unwrap_or_default());
        let signature = hasher.finalize().to_vec();
        Self { package_id, generated_at: chrono::Utc::now().timestamp(),
            graph_snapshot: state, lifecycle_log: log, deltas, verification_results,
            executive_summary: summary, signature }
    }
    pub fn verify_integrity(&self) -> bool {
        let mut hasher = sha2::Sha256::new();
        hasher.update(self.package_id.as_bytes());
        hasher.update(&serde_json::to_vec(&self.graph_snapshot).unwrap_or_default());
        hasher.finalize().to_vec() == self.signature
    }
    fn generate_summary(state: &PatentGraphState, disputed: &[Uuid]) -> String {
        let mut s = format!(
            "DFIP FORENSIC PACKAGE\nPatents: {}\nEdges: {}\nDisputed: {}\n\n",
            state.nodes.len(), state.edges.len(), disputed.len());
        for eid in disputed {
            if let Some(edge) = state.edges.iter().find(|e| e.id == *eid) {
                let p = state.nodes.get(&edge.parent_id).map(|n| n.title.as_str()).unwrap_or("?");
                let c = state.nodes.get(&edge.child_id).map(|n| n.title.as_str()).unwrap_or("?");
                s.push_str(&format!("{} -> {} (w={})\n", p, c, edge.contribution_weight));
            }
        }
        s
    }
}

pub struct PatentOfficeConverter;
impl PatentOfficeConverter {
    pub fn to_wipo_xml(node: &PatentNode) -> String {
        format!(
            r#"<?xml version="1.0"?><patent><title>{t}</title><authors>{a}</authors><type>{p}</type></patent>"#,
            t = node.title, a = node.authors.join(";"), p = format!("{:?}", node.patent_type))
    }
    pub fn to_regulatory_report(dag: &PatentDag) -> String {
        let state = dag.export_state();
        let mut r = format!("DFIP Report — {} patents, {} edges\n", state.nodes.len(), state.edges.len());
        for (_, n) in &state.nodes { r.push_str(&format!("  {} ({:?})\n", n.title, n.patent_type)); }
        for e in &state.edges { r.push_str(&format!("  {} -> {} (w={})\n", e.parent_id, e.child_id, e.contribution_weight)); }
        r
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataResidency { Global, EU, US, Jurisdiction(String) }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceConfig {
    pub data_residency: DataResidency, pub encrypt_pii: bool,
    pub audit_logging: bool, pub allow_forensic_export: bool, pub retention_days: u32,
}
impl Default for ComplianceConfig {
    fn default() -> Self { Self { data_residency: DataResidency::Global, encrypt_pii: true,
        audit_logging: true, allow_forensic_export: true, retention_days: 365*20 } }
}
impl ComplianceConfig {
    pub fn meets_gdpr(&self) -> bool {
        self.encrypt_pii && self.audit_logging && self.retention_days <= 365*20
            && (self.data_residency == DataResidency::EU || self.data_residency == DataResidency::Global)
    }
    pub fn meets_uspto(&self) -> bool { self.audit_logging && self.retention_days >= 365*20 }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArbitrationAgreement {
    pub id: Uuid, pub parties: Vec<String>, pub patent_node_id: Uuid,
    pub accept_delta_as_binding: bool, pub accept_royalty_as_binding: bool,
    pub governing_law: String, pub signatures: Vec<(String, Vec<u8>)>, pub active: bool,
}
impl ArbitrationAgreement {
    pub fn new(pid: Uuid, law: &str) -> Self {
        Self { id: Uuid::new_v4(), parties: vec![], patent_node_id: pid,
            accept_delta_as_binding: true, accept_royalty_as_binding: true,
            governing_law: law.into(), signatures: vec![], active: false }
    }
    pub fn add_party(&mut self, p: &str) { self.parties.push(p.into()); }
    pub fn sign(&mut self, p: &str, sig: Vec<u8>) { self.signatures.push((p.into(), sig)); }
    pub fn activate(&mut self) { self.active = true; }
    pub fn is_fully_signed(&self) -> bool {
        self.parties.iter().all(|p| self.signatures.iter().any(|(s,_)| s == p))
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::PatentDag;
    use crate::models::*;
    use uuid::Uuid;

    #[test]
    fn test_evidence_package() {
        let dag = PatentDag::new();
        let pkg = super::LegalEvidencePackage::from_dag(&dag, &[], vec![], vec![]);
        assert!(pkg.verify_integrity());
    }
    #[test]
    fn test_wipo_xml() {
        let n = PatentNode { id: Uuid::new_v4(), title: "Q".into(), patent_type: PatentType::Software,
            authors: vec!["A".into()], created_at_unix: 0, content_hash: vec![],
            invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![] };
        assert!(super::PatentOfficeConverter::to_wipo_xml(&n).contains("Q"));
    }
    #[test]
    fn test_gdpr() {
        let c = super::ComplianceConfig { data_residency: super::DataResidency::EU, ..Default::default() };
        assert!(c.meets_gdpr());
    }
    #[test]
    fn test_arbitration() {
        let mut a = super::ArbitrationAgreement::new(Uuid::new_v4(), "Swiss Law");
        a.add_party("A"); a.sign("A", vec![1]); a.activate();
        assert!(a.active);
    }
}
// =============================================================================
// Decoupled Fluid IP Network — Complete Monolith
// =============================================================================
// All functional code in a single file for easy transport.
// Build with:  rustc dfip_monolith.rs --crate-type lib -o libdfip.rlib
// Or just run tests with inline `#[test]`s.
// =============================================================================

#![allow(unused_imports, dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::time::Instant;
use uuid::Uuid;

// =============================================================================
// SECTION 1: CORE DATA MODELS  (models.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PatentNode {
    pub id: Uuid, pub title: String, pub patent_type: PatentType,
    pub authors: Vec<String>, pub created_at_unix: i64,
    pub content_hash: Vec<u8>, pub invariant: Option<SymbolicExpression>,
    pub tags: Vec<String>, pub license: License, pub meta_commitment: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PatentType { Software, Chemical, Mechanical }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum License { Open, Protected, Secret }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: Uuid, pub parent_id: Uuid, pub child_id: Uuid,
    pub kind: EdgeKind, pub delta: Option<DeltaConstraint>,
    pub contribution_weight: f64, pub signature: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EdgeKind { Derivative, Improvement, Fork, Merge }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SymbolicExpression {
    pub format: ExpressionFormat, pub encoded_ir: Vec<u8>,
    pub free_variables: Vec<String>, pub assumptions: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExpressionFormat { SsaTrace, Molecular, Geometric, SmtLib2 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeltaConstraint {
    pub original_node_id: Uuid, pub derivative_node_id: Uuid,
    pub original_invariant_hash: Vec<u8>, pub formula_smt_lib2: Vec<u8>,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VerificationResult {
    pub edge_id: Uuid, pub sat: SatResult, pub model: Vec<u8>,
    pub unsat_core: Vec<Vec<u8>>, pub severity_score: f64,
    pub solver_time_ms: f64, pub proof_commitment: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SatResult { Sat, Unsat, Unknown }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RoyaltyInvariant { pub patent_node_id: Uuid,
    pub shares: Vec<ShareEntry>, pub upstream_nodes: Vec<Uuid> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ShareEntry { pub node_id: Uuid, pub weight: f64 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaintLabel { pub id: Uuid, pub origin_license: License,
    pub derivation_chain: Vec<Uuid> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaintLatticeJoin { Meet, Join, LeastUpperBound }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EncryptedShare { pub ciphertext: Vec<u8>,
    pub scheme: String, pub public_key_hash: Vec<u8> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContributionProof { pub proof: Vec<u8>, pub proof_type: String }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IPSpecification { pub node_id: Uuid, pub preconditions: Vec<String>,
    pub postconditions: Vec<String>, pub invariants: Vec<String>,
    pub resource_bounds: Vec<String>, pub spec_language: String,
    pub spec_bytes: Vec<u8> }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PatentGraphState { pub nodes: HashMap<Uuid, PatentNode>,
    pub edges: Vec<Edge> }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LifecycleEvent {
    pub id: Uuid, pub event_type: LifecycleEventType, pub node_id: Uuid,
    pub timestamp_unix: i64, pub signature: Vec<u8>,
    pub parent_events: Vec<Uuid>, pub merkle_root: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LifecycleEventType { NodeCreated, NodeModified, NodeBranched, EdgeAdded, EdgeRemoved }

// =============================================================================
// SECTION 2: ERROR TYPES  (error.rs)
// =============================================================================

use std::fmt;
#[derive(Debug)]
pub enum CoreError {
    NodeNotFound(String), EdgeNotFound(String),
    CycleDetected { parent: String, child: String },
    InvalidContributionWeight(f64), DuplicateNode(String), DuplicateEdge(String),
    SmtSolverError(String), VerificationTimeout(u64), InvalidFormula(String),
    RoyaltySumViolation { sum: f64 }, RoyaltyCycle(String),
    UnlicensedFragment(String), TaintPropagationError(String),
    CryptoError(String), ProofVerificationFailed(String),
    SerializationError(String), DeserializationError(String),
    InvalidArgument(String), Internal(String),
}
impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::NodeNotFound(s) => write!(f, "node not found: {s}"),
            CoreError::EdgeNotFound(s) => write!(f, "edge not found: {s}"),
            CoreError::CycleDetected { parent, child } => write!(f, "cycle: {parent} -> {child}"),
            CoreError::InvalidContributionWeight(w) => write!(f, "invalid weight: {w}"),
            CoreError::DuplicateNode(s) => write!(f, "dup node: {s}"),
            CoreError::DuplicateEdge(s) => write!(f, "dup edge: {s}"),
            CoreError::SmtSolverError(s) => write!(f, "SMT error: {s}"),
            CoreError::VerificationTimeout(t) => write!(f, "timeout: {t}ms"),
            CoreError::InvalidFormula(s) => write!(f, "bad formula: {s}"),
            CoreError::RoyaltySumViolation { sum } => write!(f, "royalty sum: {sum}"),
            CoreError::RoyaltyCycle(s) => write!(f, "royalty cycle: {s}"),
            CoreError::UnlicensedFragment(s) => write!(f, "unlicensed: {s}"),
            CoreError::TaintPropagationError(s) => write!(f, "taint: {s}"),
            CoreError::CryptoError(s) => write!(f, "crypto: {s}"),
            CoreError::ProofVerificationFailed(s) => write!(f, "proof: {s}"),
            CoreError::SerializationError(s) => write!(f, "ser: {s}"),
            CoreError::DeserializationError(s) => write!(f, "de: {s}"),
            CoreError::InvalidArgument(s) => write!(f, "invalid arg: {s}"),
            CoreError::Internal(s) => write!(f, "internal: {s}"),
        }
    }
}

// =============================================================================
// SECTION 3: EVOLUTIONARY GRAPH ENGINE  (graph.rs)
// =============================================================================

use sha2::{Digest, Sha256};
use petgraph::stable_graph::StableDiGraph;
use petgraph::graph::{DefaultIx, NodeIndex, EdgeIndex};
use petgraph::algo::{has_path_connecting, is_cyclic_directed, toposort};
use petgraph::visit::{Bfs, IntoEdgeReferences, IntoNodeReferences};
use petgraph::Direction;

#[derive(Clone, Debug)]
pub struct PatentDag {
    graph: StableDiGraph<PatentNode, Edge>,
    node_index: HashMap<Uuid, NodeIndex>, edge_index: HashMap<Uuid, EdgeIndex>,
    lifecycle_log: Vec<LifecycleEvent>, last_event_hash: Vec<u8>,
}
impl PatentDag {
    pub fn new() -> Self { Self { graph: StableDiGraph::new(), node_index: HashMap::new(),
        edge_index: HashMap::new(), lifecycle_log: Vec::new(),
        last_event_hash: Sha256::digest(b"genesis").to_vec() } }
    pub fn add_node(&mut self, node: PatentNode) -> Result<(), CoreError> {
        let id = node.id; if self.node_index.contains_key(&id) { return Err(CoreError::DuplicateNode(id.to_string())); }
        let idx = self.graph.add_node(node); self.node_index.insert(id, idx);
        self.push_lifecycle_event(id, LifecycleEventType::NodeCreated); Ok(())
    }
    pub fn get_node(&self, id: &Uuid) -> Result<&PatentNode, CoreError> {
        let idx = self.node_index.get(id).ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        Ok(&self.graph[*idx])
    }
    pub fn add_edge(&mut self, p: &Uuid, c: &Uuid, edge: Edge) -> Result<(), CoreError> {
        let pi = self.node_index.get(p).ok_or_else(|| CoreError::NodeNotFound(p.to_string()))?;
        let ci = self.node_index.get(c).ok_or_else(|| CoreError::NodeNotFound(c.to_string()))?;
        if !(0.0..=1.0).contains(&edge.contribution_weight) { return Err(CoreError::InvalidContributionWeight(edge.contribution_weight)); }
        let ei = self.graph.add_edge(*pi, *ci, edge);
        if is_cyclic_directed(&self.graph) { self.graph.remove_edge(ei); return Err(CoreError::CycleDetected { parent: p.to_string(), child: c.to_string() }); }
        let eid = self.graph[ei].id; self.edge_index.insert(eid, ei);
        self.push_lifecycle_event(eid, LifecycleEventType::EdgeAdded); Ok(())
    }
    pub fn ancestor_ids(&self, id: &Uuid) -> Result<HashSet<Uuid>, CoreError> {
        let idx = self.node_index.get(id).ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut v = HashSet::new(); let mut s = vec![*idx];
        while let Some(c) = s.pop() { if v.insert(self.graph[c].id) { for n in self.graph.neighbors_directed(c, Direction::Incoming) { s.push(n); } } }
        Ok(v)
    }
    pub fn descendant_ids(&self, id: &Uuid) -> Result<HashSet<Uuid>, CoreError> {
        let idx = self.node_index.get(id).ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut bfs = Bfs::new(&self.graph, *idx); let mut r = HashSet::new();
        while let Some(nx) = bfs.next(&self.graph) { r.insert(self.graph[nx].id); } Ok(r)
    }
    pub fn topological_order_indices(&self) -> Result<Vec<Uuid>, CoreError> {
        let order = toposort(&self.graph, None).map_err(|_| CoreError::Internal("cycle".into()))?;
        Ok(order.iter().map(|&idx| self.graph[idx].id).collect())
    }
    pub fn distance(&self, from: &Uuid, to: &Uuid) -> Result<Option<usize>, CoreError> {
        let fi = self.node_index.get(from).ok_or_else(|| CoreError::NodeNotFound(from.to_string()))?;
        let ti = self.node_index.get(to).ok_or_else(|| CoreError::NodeNotFound(to.to_string()))?;
        if !has_path_connecting(&self.graph, *fi, *ti, None) { return Ok(None); }
        let mut q = VecDeque::new(); let mut v = HashSet::new(); q.push_back((*fi, 0usize)); v.insert(*fi);
        while let Some((c, d)) = q.pop_front() { if c == *ti { return Ok(Some(d)); } for nb in self.graph.neighbors(c) { if v.insert(nb) { q.push_back((nb, d+1)); } } }
        Ok(None)
    }
    pub fn export_state(&self) -> PatentGraphState {
        let nodes: HashMap<Uuid, PatentNode> = self.graph.node_references().map(|(_, n)| (n.id, n.clone())).collect();
        let edges: Vec<Edge> = self.graph.edge_references().map(|e| e.weight().clone()).collect();
        PatentGraphState { nodes, edges }
    }
    pub fn lifecycle_log(&self) -> &[LifecycleEvent] { &self.lifecycle_log }
    pub fn last_event_hash(&self) -> &[u8] { &self.last_event_hash }
    fn push_lifecycle_event(&mut self, nid: Uuid, etype: LifecycleEventType) {
        let eid = Uuid::new_v4();
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        let mut h = Sha256::new(); h.update(&self.last_event_hash); h.update(eid.as_bytes());
        h.update(nid.as_bytes()); h.update(ts.to_le_bytes()); let root = h.finalize().to_vec();
        self.lifecycle_log.push(LifecycleEvent { id: eid, event_type: etype, node_id: nid,
            timestamp_unix: ts, signature: vec![],
            parent_events: vec![Uuid::from_slice(&self.last_event_hash[..16]).unwrap_or_default()],
            merkle_root: root.clone() });
        self.last_event_hash = root;
    }
    pub fn contains_node(&self, id: &Uuid) -> bool { self.node_index.contains_key(id) }
    pub fn node_count(&self) -> usize { self.graph.node_count() }
    pub fn edge_count(&self) -> usize { self.graph.edge_count() }
    pub fn has_path(&self, f: &Uuid, t: &Uuid) -> Result<bool, CoreError> {
        let fi = self.node_index.get(f).ok_or_else(|| CoreError::NodeNotFound(f.to_string()))?;
        let ti = self.node_index.get(t).ok_or_else(|| CoreError::NodeNotFound(t.to_string()))?;
        Ok(has_path_connecting(&self.graph, *fi, *ti, None))
    }
}
impl Default for PatentDag { fn default() -> Self { Self::new() } }

// =============================================================================
// SECTION 4: SYMBOLIC IR  (ir.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrValue { Int(i64), Float(f64), Bool(bool), BitVector(Vec<u8>),
    Variable(String), Term { function: String, args: Vec<Box<IrValue>> } }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SsaOp { Add, Sub, Mul, Div, And, Or, Xor, Shl, Lshr,
    Eq, Ne, Lt, Le, Gt, Ge, Load { address: Box<IrExpr> },
    Store { address: Box<IrExpr>, value: Box<IrExpr> }, Phi { incoming: Vec<IrExpr> }, Custom(String) }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SsaInstruction { pub result: Option<String>, pub op: SsaOp, pub operands: Vec<IrExpr> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BasicBlock { pub label: String, pub instructions: Vec<SsaInstruction>, pub terminator: Terminator }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Terminator { Branch(String), BranchCond { condition: IrExpr, true_target: String, false_target: String }, Return(Option<IrExpr>) }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SsaTrace { pub entry_block: String, pub blocks: HashMap<String, BasicBlock>,
    pub inputs: Vec<(String, IrType)>, pub outputs: Vec<(String, IrType)> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrType { Int(u32), Float(u32), Bool, BitVector(u32), Array(Box<IrType>, Box<IrType>), Struct(Vec<(String, IrType)>) }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrExpr { Value(IrValue), Ref(String), Unary { op: UnaryOp, expr: Box<IrExpr> },
    Binary { op: BinaryOp, left: Box<IrExpr>, right: Box<IrExpr> },
    Ite { cond: Box<IrExpr>, then_expr: Box<IrExpr>, else_expr: Box<IrExpr> } }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UnaryOp { Neg, Not, BitNot }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BinaryOp { Add, Sub, Mul, Div, Mod, And, Or, Xor, Shl, Lshr, Eq, Ne, Lt, Le, Gt, Ge }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BondType { Single, Double, Triple, Aromatic, Hydrogen }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Atom { pub symbol: String, pub formal_charge: i32, pub isotope: Option<u32>, pub stereochemistry: Option<String> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bond { pub atom_a: usize, pub atom_b: usize, pub bond_type: BondType }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MolecularGraph { pub atoms: Vec<Atom>, pub bonds: Vec<Bond>, pub constraints: Vec<ChemicalConstraint> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ChemicalConstraint { Chirality { atom_index: usize, configuration: String },
    BondAngle { atom_a: usize, atom_b: usize, atom_c: usize, min_degrees: f64, max_degrees: f64 },
    Distance { atom_a: usize, atom_b: usize, min_angstrom: f64, max_angstrom: f64 },
    ChargeRange { atom_index: usize, min: i32, max: i32 }, ReactionInvariant { description: String } }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point3 { pub x: f64, pub y: f64, pub z: f64 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GeometricConstraint { Coincident { entities: Vec<usize> }, Parallel { entities: Vec<usize> },
    Perpendicular { entities: Vec<usize> }, Concentric { entities: Vec<usize> },
    Distance { entity_a: usize, entity_b: usize, min: f64, max: f64 },
    Angle { entity_a: usize, entity_b: usize, min_degrees: f64, max_degrees: f64 },
    Tangent { entities: Vec<usize> }, Symmetry { entity_a: usize, entity_b: usize, about: usize },
    Fixed { entity: usize, position: Point3 }, Tolerance { entity: usize, tolerance: f64 } }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GeometricEntity { Point(Point3), Line { point: Point3, direction: Point3 },
    Plane { point: Point3, normal: Point3 }, Circle { center: Point3, normal: Point3, radius: f64 },
    Cylinder { axis_point: Point3, axis_direction: Point3, radius: f64 }, Sphere { center: Point3, radius: f64 } }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MechanicalModel { pub entities: Vec<GeometricEntity>,
    pub constraints: Vec<GeometricConstraint>, pub metadata: HashMap<String, String> }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrKind { Software(SsaTrace), Chemical(MolecularGraph), Mechanical(MechanicalModel) }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InventionIr { pub kind: IrKind, pub preconditions: Vec<String>,
    pub postconditions: Vec<String>, pub invariants: Vec<String> }
pub fn extract_invariants(ir: &InventionIr) -> Vec<String> {
    let mut v = Vec::new(); v.extend(ir.preconditions.clone());
    v.extend(ir.postconditions.clone()); v.extend(ir.invariants.clone()); v
}

// =============================================================================
// SECTION 5: DELTA CONSTRAINT BUILDER  (delta.rs)
// =============================================================================

pub fn build_delta(oid: Uuid, did: Uuid, oir: &InventionIr, dir: &InventionIr, _as: &[String]) -> Result<DeltaConstraint, CoreError> {
    let oi = extract_invariants(oir); let di = extract_invariants(dir);
    let mut h = Sha256::new(); for i in &oi { h.update(i.as_bytes()); }
    let hash = h.finalize().to_vec();
    let f = format!("(set-logic ALL)\n(assert (not (and\n  {}\n)))\n(assert (and\n  {}\n))\n(check-sat)\n",
        oi.iter().map(|s| format!("(assert {s})")).collect::<Vec<_>>().join("\n  "),
        di.iter().map(|s| format!("(assert {s})")).collect::<Vec<_>>().join("\n  "));
    Ok(DeltaConstraint { original_node_id: oid, derivative_node_id: did,
        original_invariant_hash: hash, formula_smt_lib2: f.into_bytes(), assumptions: _as.to_vec() })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuditEntry { pub id: Uuid, pub delta_constraint: DeltaConstraint,
    pub timestamp_unix: i64, pub previous_hash: Vec<u8>, pub merkle_root: Vec<u8> }
pub struct AuditTrail { entries: Vec<AuditEntry>, last_hash: Vec<u8> }
impl AuditTrail {
    pub fn new() -> Self { Self { entries: vec![], last_hash: Sha256::digest(b"audit-genesis").to_vec() } }
    pub fn push(&mut self, delta: DeltaConstraint) -> Uuid {
        let id = Uuid::new_v4(); let mut h = Sha256::new();
        h.update(&self.last_hash); h.update(&delta.formula_smt_lib2); h.update(id.as_bytes());
        let root = h.finalize().to_vec();
        self.entries.push(AuditEntry { id, delta_constraint: delta,
            timestamp_unix: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
            previous_hash: self.last_hash.clone(), merkle_root: root.clone() });
        self.last_hash = root; id }
    pub fn verify(&self) -> bool {
        let mut cur = Sha256::digest(b"audit-genesis").to_vec();
        for e in &self.entries { let mut h = Sha256::new(); h.update(&cur);
            h.update(&e.delta_constraint.formula_smt_lib2); h.update(e.id.as_bytes());
            if h.finalize().to_vec() != e.merkle_root { return false; } cur = e.merkle_root.clone(); }
        cur == self.last_hash }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}
impl Default for AuditTrail { fn default() -> Self { Self::new() } }

// =============================================================================
// SECTION 6: SMT SOLVER  (solver.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SolverBackend { Z3, Bitwuzla, Cvc5, BuiltIn }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverConfig { pub backend: SolverBackend, pub timeout_ms: u64, pub solver_path: Option<String> }
impl Default for SolverConfig { fn default() -> Self { Self { backend: SolverBackend::BuiltIn, timeout_ms: 5000, solver_path: None } } }
pub trait SmtSolver: Send + Sync { fn check_sat(&self, f: &str, c: &SolverConfig) -> Result<SolverResponse, CoreError>; }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverResponse { pub sat: SatResult, pub model: HashMap<String, String>,
    pub unsat_core: Vec<String>, pub solver_time_ms: f64 }

pub struct BuiltInSolver;
impl SmtSolver for BuiltInSolver {
    fn check_sat(&self, formula: &str, config: &SolverConfig) -> Result<SolverResponse, CoreError> {
        let start = Instant::now();
        let mut a = HashMap::new();
        for line in formula.lines() {
            let line = line.trim();
            if let Some(b) = line.strip_prefix("(assert ").and_then(|s| s.strip_suffix(')')) {
                let b = b.trim();
                if let Some(inner) = b.strip_prefix("(= ").and_then(|s| s.strip_suffix(')')) {
                    let parts: Vec<&str> = inner.split_whitespace().collect();
                    if parts.len() == 2 { if let Ok(v) = parts[1].parse::<i64>() {
                        if let Some(&e) = a.get(parts[0]) { if e != v { return Ok(SolverResponse { sat: SatResult::Unsat, model: HashMap::new(), unsat_core: vec![], solver_time_ms: start.elapsed().as_secs_f64()*1000.0 }); } }
                        a.insert(parts[0].to_string(), v);
                    }}
                }
                if b.starts_with("(not ") && b.ends_with(')') {
                    let inner = &b[5..b.len()-1].trim();
                    if let Some(eq) = inner.strip_prefix("(= ").and_then(|s| s.strip_suffix(')')) {
                        let parts: Vec<&str> = eq.split_whitespace().collect();
                        if parts.len() == 2 { if let Ok(v) = parts[1].parse::<i64>() {
                            if let Some(&e) = a.get(parts[0]) { if e == v { return Ok(SolverResponse { sat: SatResult::Unsat, model: HashMap::new(), unsat_core: vec![], solver_time_ms: start.elapsed().as_secs_f64()*1000.0 }); } }
                        }}
                    }
                }
            }
        }
        let elapsed = start.elapsed().as_secs_f64()*1000.0;
        if elapsed > config.timeout_ms as f64 { return Ok(SolverResponse { sat: SatResult::Unknown, model: HashMap::new(), unsat_core: vec![], solver_time_ms: elapsed }); }
        Ok(SolverResponse { sat: SatResult::Sat, model: HashMap::new(), unsat_core: vec![], solver_time_ms: elapsed })
    }
}

pub fn compute_severity(r: &SolverResponse) -> f64 { match r.sat {
    SatResult::Sat => 0.5 + (r.model.len() as f64 / 20.0).min(0.5),
    SatResult::Unsat => (r.unsat_core.len() as f64 / 10.0).min(0.5), _ => 0.5 } }
pub fn run_verification(eid: Uuid, smt: &[u8], solver: &dyn SmtSolver, cfg: &SolverConfig) -> Result<VerificationResult, CoreError> {
    let f = String::from_utf8(smt.to_vec()).map_err(|_| CoreError::InvalidFormula("bad UTF-8".into()))?;
    let r = solver.check_sat(&f, cfg)?; let s = compute_severity(&r);
    Ok(VerificationResult { edge_id: eid, sat: r.sat, model: serde_json::to_vec(&r.model).unwrap_or_default(),
        unsat_core: r.unsat_core.iter().map(|s| s.as_bytes().to_vec()).collect(),
        severity_score: s, solver_time_ms: r.solver_time_ms, proof_commitment: vec![] })
}

pub struct ExternalSolver { name: String }
impl ExternalSolver { pub fn new(b: &SolverBackend) -> Self {
    Self { name: match b { SolverBackend::Z3 => "z3", SolverBackend::Bitwuzla => "bitwuzla", SolverBackend::Cvc5 => "cvc5", _ => "" }.into() } } }
impl SmtSolver for ExternalSolver {
    fn check_sat(&self, formula: &str, config: &SolverConfig) -> Result<SolverResponse, CoreError> {
        let bin = config.solver_path.clone().unwrap_or_else(|| self.name.clone());
        let start = Instant::now();
        let child = std::process::Command::new(&bin).arg("-in").arg(format!("-T:{}", config.timeout_ms/1000))
            .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn();
        let output = match child { Ok(mut c) => { use std::io::Write;
                if let Some(ref mut s) = c.stdin { let _ = s.write_all(formula.as_bytes()); }
                c.wait_with_output().map_err(|e| CoreError::SmtSolverError(format!("{e}")))?
            } Err(e) => return Err(CoreError::SmtSolverError(format!("no {bin}: {e}"))),
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let sat = if stdout.contains("sat") { SatResult::Sat } else if stdout.contains("unsat") { SatResult::Unsat } else { SatResult::Unknown };
        let mut m = HashMap::new();
        if sat == SatResult::Sat { for l in stdout.lines() { if let Some(d) = l.trim().strip_prefix("(define-fun ") {
            let p: Vec<&str> = d.splitn(2, ' ').collect(); if p.len() == 2 { if let Some(vs) = p[1].find(' ') {
                m.insert(p[0].to_string(), p[1][vs..].trim().trim_end_matches(')').to_string()); } } } } }
        Ok(SolverResponse { sat, model: m, unsat_core: vec![], solver_time_ms: start.elapsed().as_secs_f64()*1000.0 })
    }
}
pub fn create_solver(c: &SolverConfig) -> Box<dyn SmtSolver> {
    match c.backend { SolverBackend::BuiltIn => Box::new(BuiltInSolver), _ => Box::new(ExternalSolver::new(&c.backend)) }
}

// =============================================================================
// SECTION 7: ROYALTY INVARIANTS  (royalties.rs)
// =============================================================================

pub struct RoyaltyVerifier { e: f64 }
impl RoyaltyVerifier { pub fn new(e: f64) -> Self { Self { e } }
    pub fn verify_all(&self, dag: &PatentDag) -> Result<(), CoreError> {
        for nid in dag.export_state().nodes.keys() {
            let state = dag.export_state();
            let inc: Vec<&Edge> = state.edges.iter().filter(|e| e.child_id == *nid).collect();
            if inc.is_empty() { continue; }
            let s: f64 = inc.iter().map(|e| e.contribution_weight).sum();
            if (s - 1.0).abs() > self.e { return Err(CoreError::RoyaltySumViolation { sum: s }); }
        } Ok(())
    }
}
impl Default for RoyaltyVerifier { fn default() -> Self { Self::new(1e-9) } }

pub fn compute_royalty_shares(dag: &PatentDag, nid: &Uuid) -> Result<HashMap<Uuid, f64>, CoreError> {
    let _ = dag.get_node(nid)?;
    let an = dag.ancestor_ids(nid)?;
    let topo = dag.topological_order_indices()?;
    let state = dag.export_state();
    let mut shares: HashMap<Uuid, f64> = HashMap::new();
    shares.insert(*nid, 1.0);
    for cur in topo.iter().rev() {
        let cs = *shares.get(cur).unwrap_or(&0.0);
        if cs == 0.0 { continue; }
        for e in &state.edges { if e.child_id == **cur { *shares.entry(e.parent_id).or_insert(0.0) += cs * e.contribution_weight; } }
    }
    Ok(shares)
}

// =============================================================================
// SECTION 8: TAINT TRACKING  (taint.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TaintValue { Unlicensed, Licensed(Uuid), Multiple(HashSet<Uuid>) }
impl TaintValue {
    pub fn join(&self, o: &Self) -> Self { match (self, o) {
        (TaintValue::Unlicensed, x) | (x, TaintValue::Unlicensed) => x.clone(),
        (TaintValue::Licensed(a), TaintValue::Licensed(b)) if a == b => TaintValue::Licensed(*a),
        (TaintValue::Licensed(a), TaintValue::Licensed(b)) => { let mut s=HashSet::new(); s.insert(*a); s.insert(*b); TaintValue::Multiple(s) }
        (TaintValue::Licensed(a), TaintValue::Multiple(bs)) => { let mut s=bs.clone(); s.insert(*a); TaintValue::Multiple(s) }
        (TaintValue::Multiple(as_), TaintValue::Licensed(b)) => { let mut s=as_.clone(); s.insert(*b); TaintValue::Multiple(s) }
        (TaintValue::Multiple(as_), TaintValue::Multiple(bs)) => { let mut s=as_.clone(); s.extend(bs); TaintValue::Multiple(s) }
        _ => TaintValue::Multiple({ let mut s=HashSet::new(); if let TaintValue::Licensed(a)=self{s.insert(*a);} if let TaintValue::Licensed(b)=o{s.insert(*b);} s }) } }
    pub fn meet(&self, o: &Self) -> Self { match (self, o) {
        (TaintValue::Unlicensed,_)|(_,TaintValue::Unlicensed) => TaintValue::Unlicensed,
        (TaintValue::Licensed(a),TaintValue::Licensed(b)) if a==b => TaintValue::Licensed(*a),
        (TaintValue::Licensed(a),TaintValue::Multiple(bs)) if bs.contains(a) => TaintValue::Licensed(*a),
        (TaintValue::Multiple(as_),TaintValue::Licensed(b)) if as_.contains(b) => TaintValue::Licensed(*b),
        (TaintValue::Multiple(as_),TaintValue::Multiple(bs)) => { let i:HashSet<Uuid>=as_.intersection(bs).copied().collect();
            if i.len()==1{return TaintValue::Licensed(*i.iter().next().unwrap());} if i.is_empty(){return TaintValue::Unlicensed;} TaintValue::Multiple(i) }
        _ => TaintValue::Unlicensed } }
}

// =============================================================================
// SECTION 9: IP SPECIFICATION  (spec.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IpSpec { pub node_id: Uuid, pub preconditions: Vec<String>,
    pub postconditions: Vec<String>, pub invariants: Vec<String>,
    pub resource_bounds: Vec<String>, pub spec_language: String, pub spec_bytes: Vec<u8> }
pub struct SpecSynthesizer;
impl SpecSynthesizer {
    pub fn synthesize(ir: &InventionIr, nid: Uuid) -> IpSpec {
        let mut c = Vec::new();
        match &ir.kind { IrKind::Software(t) => { for (l, b) in &t.blocks {
            if b.instructions.iter().any(|i| matches!(i.op, SsaOp::Phi{..})) { c.push(format!("loop_at_{l}: converge")); } }
            for (n, typ) in &t.inputs { if let IrType::Int(w) = typ { c.push(format!("in_range({n}, 0, 2^{w})")); } } }
            _ => {}
        }
        IpSpec { node_id: nid, preconditions: ir.preconditions.clone(), postconditions: ir.postconditions.clone(),
            invariants: c, resource_bounds: vec![], spec_language: "dfip-ir".into(),
            spec_bytes: format!("pre: {:?}\npost: {:?}\ninv: {:?}", ir.preconditions, ir.postconditions, c).into_bytes() }
    }
}

// =============================================================================
// SECTION 10: HOMOMORPHIC ENCRYPTION  (he.rs + zk.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeConfig { pub security_level: String, pub precision_bits: u32, pub modulus_bits: u32 }
impl Default for HeConfig { fn default() -> Self { Self { security_level: "standard".into(), precision_bits: 16, modulus_bits: 64 } } }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HePublicKey { pub id: Vec<u8>, pub config: HeConfig }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HePrivateKey { pub id: Vec<u8>, pub config: HeConfig }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeKeypair { pub public_key: HePublicKey, pub private_key: HePrivateKey }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeCiphertext { pub data: Vec<u8>, pub config: HeConfig }

pub struct HeEngine { config: HeConfig }
impl HeEngine {
    pub fn new(c: HeConfig) -> Self { Self { config: c } }
    pub fn generate_keypair(&self) -> HeKeypair {
        let id = Sha256::digest(format!("he-key-{}", rand_seed()).as_bytes()).to_vec();
        HeKeypair { public_key: HePublicKey { id: id.clone(), config: self.config.clone() },
            private_key: HePrivateKey { id, config: self.config.clone() } } }
    pub fn encrypt(&self, value: f64, pk: &HePublicKey) -> HeCiphertext {
        let scale = 1u64 << self.config.precision_bits;
        let fixed = (value.clamp(0.0, 1.0) * scale as f64) as u64;
        let ks = self.keystream(pk);
        HeCiphertext { data: fixed.to_le_bytes().iter().zip(ks.iter()).map(|(a,b)|a^b).collect(), config: self.config.clone() } }
    pub fn decrypt(&self, ct: &HeCiphertext, sk: &HePrivateKey) -> Result<f64, String> {
        let ks = self.keystream_priv(sk); let mut p = [0u8; 8];
        for i in 0..8.min(ct.data.len()) { p[i] = ct.data[i] ^ ks[i]; }
        Ok(u64::from_le_bytes(p) as f64 / (1u64 << self.config.precision_bits) as f64) }
    fn keystream(&self, pk: &HePublicKey) -> Vec<u8> {
        let mut h = Sha256::new(); h.update(b"dfip-ks"); h.update(&pk.id); h.update(self.config.modulus_bits.to_le_bytes());
        h.finalize()[..8].to_vec() }
    fn keystream_priv(&self, sk: &HePrivateKey) -> Vec<u8> {
        let mut h = Sha256::new(); h.update(b"dfip-ks"); h.update(&sk.id); h.update(self.config.modulus_bits.to_le_bytes());
        h.finalize()[..8].to_vec() }
}
pub fn rand_seed() -> String { let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    format!("{}-{}", d.as_secs(), d.subsec_nanos()) }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ZkProofType { Bulletproof, Groth16, Plonk, Simulated }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkProof { pub proof_type: ZkProofType, pub proof_bytes: Vec<u8>, pub public_inputs: Vec<u8> }
pub struct ZkProver;
impl ZkProver {
    pub fn prove_range(v: f64, blind: &[u8]) -> ZkProof {
        let mut h = Sha256::new(); h.update(b"dfip-zk"); h.update(v.to_le_bytes()); h.update(blind);
        ZkProof { proof_type: ZkProofType::Simulated, proof_bytes: h.finalize().to_vec(), public_inputs: vec![] } }
    pub fn verify_range(p: &ZkProof, _c: &[u8]) -> Result<(), String> {
        if p.proof_bytes.is_empty() { return Err("empty proof".into()); } Ok(()) }
}

// =============================================================================
// SECTION 11: IP STATE EXPLORER  (explorer.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilarityResult { pub source_id: Uuid, pub target_id: Uuid,
    pub sat: SatResult, pub severity: f64, pub distance: Option<usize> }
pub fn compare_nodes(dag: &PatentDag, src: &Uuid, tgt: &Uuid, si: &InventionIr, ti: &InventionIr,
    solver: &dyn SmtSolver, cfg: &SolverConfig) -> Result<SimilarityResult, CoreError> {
    let d = build_delta(*src, *tgt, si, ti, &[])?;
    let f = String::from_utf8(d.formula_smt_lib2).map_err(|_| CoreError::InvalidFormula("".into()))?;
    let r = solver.check_sat(&f, cfg)?; let s = compute_severity(&r);
    Ok(SimilarityResult { source_id: *src, target_id: *tgt, sat: r.sat, severity: s, distance: Ok(None)?
        // dag.distance(src, tgt).ok().flatten()
    })
}

// =============================================================================
// SECTION 12: CONTRACTS — REGISTRY, SPLITTER, ORACLE, GATE
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredPatent { pub id: Uuid, pub title: String, pub registered_at: i64,
    pub registrant: String, pub commitment: Vec<u8>, pub invariant_hash: Vec<u8>, pub license: License }
pub struct PatentRegistry { patents: HashMap<Uuid, RegisteredPatent> }
impl PatentRegistry {
    pub fn new() -> Self { Self { patents: HashMap::new() } }
    pub fn register(&mut self, node: &PatentNode, reg: &str) -> RegisteredPatent {
        let data = serde_json::to_vec(node).unwrap_or_default();
        let c = Sha256::digest(&data).to_vec();
        let ih = node.invariant.as_ref().map(|i| Sha256::digest(&i.encoded_ir).to_vec())
            .unwrap_or_else(|| Sha256::digest(b"no-invariant").to_vec());
        let r = RegisteredPatent { id: node.id, title: node.title.clone(),
            registered_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
            registrant: reg.into(), commitment: c, invariant_hash: ih, license: node.license.clone() };
        self.patents.insert(node.id, r.clone()); r }
    pub fn verify_commitment(&self, node: &PatentNode) -> bool {
        let data = serde_json::to_vec(node).unwrap_or_default();
        self.patents.get(&node.id).map(|r| Sha256::digest(&data).to_vec() == r.commitment).unwrap_or(false) }
}
impl Default for PatentRegistry { fn default() -> Self { Self::new() } }

pub struct ExecutionGate { requests: Vec<(Uuid, String, String)> }
impl ExecutionGate {
    pub fn new() -> Self { Self { requests: vec![] } }
    pub fn request_release(&mut self, pid: Uuid, req: &str, bph: &str) { self.requests.push((pid, req.into(), bph.into())); }
    pub fn is_released(&self, pid: &Uuid, req: &str) -> bool { self.requests.iter().any(|(p,r,_)| p==pid && r==req) }
}
impl Default for ExecutionGate { fn default() -> Self { Self::new() } }

// =============================================================================
// SECTION 13: P2P NETWORK LAYER  (p2p.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkMessage { AnnounceNode(PatentNode), AnnounceEdge(Edge),
    SyncRequest { requester_id: Uuid, last_known_hash: Vec<u8> },
    SyncResponse { nodes: Vec<PatentNode>, edges: Vec<Edge>, merkle_root: Vec<u8> },
    Ping { node_id: Uuid, timestamp: i64 } }
pub struct NetworkNode { pub id: Uuid, pub dag: PatentDag, peers: HashMap<Uuid, String> }
impl NetworkNode {
    pub fn new(id: Uuid) -> Self { Self { id, dag: PatentDag::new(), peers: HashMap::new() } }
    pub fn add_peer(&mut self, pid: Uuid, addr: &str) { self.peers.insert(pid, addr.into()); }
    pub fn receive(&mut self, _from: Uuid, msg: NetworkMessage) -> Option<NetworkMessage> {
        match &msg { NetworkMessage::AnnounceNode(n) => { let _ = self.dag.add_node(n.clone()); Some(msg) }
            NetworkMessage::AnnounceEdge(e) => { let _ = self.dag.add_edge(&e.parent_id, &e.child_id, e.clone()); Some(msg) }
            NetworkMessage::SyncRequest{..} => { let s = self.dag.export_state(); Some(NetworkMessage::SyncResponse {
                nodes: s.nodes.into_values().collect(), edges: s.edges, merkle_root: self.dag.last_event_hash().to_vec() }) }
            _ => None } }
}

// =============================================================================
// SECTION 14: STORAGE LAYER  (storage.rs)
// =============================================================================

pub struct StorageNode { blueprints: HashMap<String, Vec<u8>> }
impl StorageNode {
    pub fn new() -> Self { Self { blueprints: HashMap::new() } }
    pub fn store(&mut self, data: &[u8]) -> String {
        let cid = format!("Qm{}", hex::encode(Sha256::digest(data)));
        self.blueprints.insert(cid.clone(), data.to_vec()); cid }
    pub fn get(&self, cid: &str) -> Option<&[u8]> { self.blueprints.get(cid).map(|v| v.as_slice()) }
    pub fn verify(&self, cid: &str, data: &[u8]) -> bool {
        format!("Qm{}", hex::encode(Sha256::digest(data))) == cid && self.blueprints.contains_key(cid) }
}

// =============================================================================
// SECTION 15: TRANSLATION FIDELITY  (fidelity.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InvariantProvenance { StaticAnalysis, UserProvided, Heuristic, MachineLearned, Default }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AbstractionGap { LinearizedNonlinear { original_domain: String, over_approx: String },
    DiscretizedContinuous { step_size: f64, error_bound: f64 },
    ProbabilisticToWorstCase { distribution: String, bound: String },
    UninterpretedFunction { name: String, assumptions: Vec<String> },
    DeliberateSimplification { reason: String }, Exact }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnotatedInvariant { pub predicate: String, pub provenance: InvariantProvenance,
    pub confidence: f64, pub abstraction_gap: AbstractionGap, pub justification: String }
impl AnnotatedInvariant {
    pub fn exact(predicate: &str, just: &str) -> Self {
        Self { predicate: predicate.into(), provenance: InvariantProvenance::StaticAnalysis,
            confidence: 1.0, abstraction_gap: AbstractionGap::Exact, justification: just.into() } }
    pub fn heuristic(predicate: &str, gap: AbstractionGap, just: &str) -> Self {
        let c = match &gap { AbstractionGap::Exact => 0.9, AbstractionGap::LinearizedNonlinear{..} => 0.6,
            AbstractionGap::DiscretizedContinuous{..} => 0.5, AbstractionGap::ProbabilisticToWorstCase{..} => 0.4,
            AbstractionGap::UninterpretedFunction{..} => 0.3, AbstractionGap::DeliberateSimplification{..} => 0.2 };
        Self { predicate: predicate.into(), provenance: InvariantProvenance::Heuristic,
            confidence: c, abstraction_gap: gap, justification: just.into() } }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SoundnessIssue { DirectContradiction { a: String, b: String, explanation: String },
    VacuousInvariant { predicate: String }, UnboundedVariable { variable: String },
    LowConfidence { predicate: String, confidence: f64 } }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundnessReport { pub sound: bool, pub issues: Vec<SoundnessIssue>,
    pub overall_confidence: f64, pub can_use_for_infringement: bool }
pub struct IRVerifier { pub min_confidence: f64 }
impl IRVerifier {
    pub fn new(mc: f64) -> Self { Self { min_confidence: mc } }
    pub fn verify(&self, invs: &[AnnotatedInvariant]) -> SoundnessReport {
        let mut issues = Vec::new(); let mut seen: Vec<&str> = Vec::new();
        for inv in invs {
            if inv.predicate.trim() == "true" || inv.predicate.trim().is_empty() { issues.push(SoundnessIssue::VacuousInvariant { predicate: inv.predicate.clone() }); }
            if inv.confidence < self.min_confidence { issues.push(SoundnessIssue::LowConfidence { predicate: inv.predicate.clone(), confidence: inv.confidence }); }
            for p in &seen { if let Some(issue) = check_contradiction(p, &inv.predicate) {
                if !issues.iter().any(|i| matches!(i, SoundnessIssue::DirectContradiction{..})) { issues.push(issue); } } }
            seen.push(&inv.predicate);
        }
        let oc: f64 = invs.iter().map(|i| i.confidence).fold(1.0, |a, b| a.min(b));
        let can = issues.iter().all(|i| matches!(i, SoundnessIssue::LowConfidence{..})) && oc >= self.min_confidence;
        SoundnessReport { sound: issues.is_empty(), issues, overall_confidence: oc, can_use_for_infringement: can }
    }
}
impl Default for IRVerifier { fn default() -> Self { Self { min_confidence: 0.7 } } }
fn check_contradiction(a: &str, b: &str) -> Option<SoundnessIssue> {
    let ap: Vec<&str> = a.split_whitespace().collect(); let bp: Vec<&str> = b.split_whitespace().collect();
    if ap.len() != 3 || bp.len() != 3 || ap[0] != bp[0] { return None; }
    let va: f64 = ap[2].parse().ok()?; let vb: f64 = bp[2].parse().ok()?;
    let contr = match (ap[1], bp[1]) { (">", "<")|(">=","<")|(">","<=") if va >= vb => true,
        (">=", "<=") if va > vb => true, ("==", "!=")|("!=", "==") => true, _ => false };
    if contr { Some(SoundnessIssue::DirectContradiction { a: a.into(), b: b.into(),
        explanation: format!("{a} and {b} conflict") }) } else { None }
}

// =============================================================================
// SECTION 16: SOLVER STRATEGIES  (solver_strategy.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SolverStrategyKind { FullSmt, IncrementalSmt, AbstractInterpretation, Triage, BuiltIn }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyResult2 { pub strategy_used: SolverStrategyKind,
    pub strategy_chain: Vec<(SolverStrategyKind, String, f64)>, pub is_conclusive: bool }
pub struct AbstractInterpreter2;
impl AbstractInterpreter2 {
    pub fn check_sat(constraints: &[String]) -> Option<bool> {
        let mut intervals: HashMap<String, (f64, f64)> = HashMap::new();
        for c in constraints {
            let p: Vec<&str> = c.split_whitespace().collect(); if p.len() != 3 { continue; }
            let v = p[0].to_string(); let val: f64 = p[2].parse().ok()?;
            let (lo, hi) = match p[1] { ">"|">=" => (val, f64::INFINITY), "<"|"<=" => (f64::NEG_INFINITY, val),
                "==" => (val, val), _ => continue };
            match intervals.get(&v) { Some(&(l, h)) => {
                    let nl = f64::max(l, lo); let nh = f64::min(h, hi);
                    if nl <= nh { intervals.insert(v, (nl, nh)); } else { return Some(false); } }
                None => { intervals.insert(v, (lo, hi)); } }
        }
        Some(true)
    }
}

// =============================================================================
// SECTION 17: REGULATION & COMPLIANCE  (regulation.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegalEvidencePackage { pub package_id: Uuid, pub generated_at: i64,
    pub executive_summary: String, pub signature: Vec<u8> }
impl LegalEvidencePackage {
    pub fn new(summary: &str) -> Self {
        let id = Uuid::new_v4(); let mut h = Sha256::new(); h.update(id.as_bytes()); h.update(summary.as_bytes());
        Self { package_id: id, generated_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
            executive_summary: summary.into(), signature: h.finalize().to_vec() } }
    pub fn verify(&self) -> bool {
        let mut h = Sha256::new(); h.update(self.package_id.as_bytes()); h.update(self.executive_summary.as_bytes());
        h.finalize().to_vec() == self.signature }
}
pub struct PatentOfficeConverter2;
impl PatentOfficeConverter2 { pub fn to_wipo_xml(node: &PatentNode) -> String {
        format!("<?xml version=\"1.0\"?><patent><title>{t}</title><authors>{a}</authors></patent>", t=node.title, a=node.authors.join(";")) } }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataResidency2 { Global, EU, US, Jurisdiction(String) }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceConfig2 { pub data_residency: DataResidency2,
    pub encrypt_pii: bool, pub audit_logging: bool, pub retention_days: u32 }
impl ComplianceConfig2 { pub fn meets_gdpr(&self) -> bool {
    self.encrypt_pii && self.audit_logging && self.retention_days <= 365*20
        && (self.data_residency == DataResidency2::EU || self.data_residency == DataResidency2::Global) } }

// =============================================================================
// SECTION 18: CONTRIBUTION WEIGHT GOVERNANCE  (contribution.rs)
// =============================================================================

pub struct ContributionGovernance {
    pub parent_id: Uuid, pub child_id: Uuid,
    pub requested_weight: f64, pub max_allowed_weight: f64,
    pub final_weight: f64, pub justification: String,
}

pub struct ContributionGovernor;
impl ContributionGovernor {
    pub fn govern(parent: &InventionIr, child: &InventionIr, requested: f64) -> ContributionGovernance {
        let ast_r = Self::ast_delta(parent, child);
        let inv_r = Self::inv_novelty(parent, child);
        let comp_r = Self::complex_ratio(parent, child);
        let raw = ast_r.max(inv_r).max(comp_r);
        let max_allowed = raw.min(0.5); // cap at 0.5 — no single derivative claims majority
        ContributionGovernance { parent_id: Uuid::new_v4(), child_id: Uuid::new_v4(),
            requested_weight: requested, max_allowed_weight: max_allowed,
            final_weight: requested.min(max_allowed),
            justification: format!("AST:{:.3} Inv:{:.3} Comp:{:.3} -> max:{:.3}", ast_r, inv_r, comp_r, max_allowed) }
    }
    fn ast_delta(p: &InventionIr, c: &InventionIr) -> f64 {
        match (&p.kind, &c.kind) {
            (IrKind::Software(p), IrKind::Software(c)) => {
                let ps: HashSet<&str> = p.blocks.keys().map(|s| s.as_str()).collect();
                let cs: HashSet<&str> = c.blocks.keys().map(|s| s.as_str()).collect();
                cs.difference(&ps).count() as f64 / cs.len().max(1) as f64
            }
            (IrKind::Chemical(p), IrKind::Chemical(c)) =>
                c.atoms.len().saturating_sub(p.atoms.len()) as f64 / c.atoms.len().max(1) as f64,
            (IrKind::Mechanical(p), IrKind::Mechanical(c)) =>
                c.entities.len().saturating_sub(p.entities.len()) as f64 / c.entities.len().max(1) as f64,
            _ => 0.5,
        }
    }
    fn inv_novelty(p: &InventionIr, c: &InventionIr) -> f64 {
        let pv = extract_invariants(p); let cv = extract_invariants(c);
        let ps: HashSet<&str> = pv.iter().map(|s| s.as_str()).collect();
        let cs: HashSet<&str> = cv.iter().map(|s| s.as_str()).collect();
        cs.difference(&ps).count() as f64 / cs.len().max(1) as f64
    }
    fn complex_ratio(p: &InventionIr, c: &InventionIr) -> f64 {
        let ps = size(p); let cs = size(c);
        if cs <= ps || ps == 0 { 0.0 } else { (cs - ps) as f64 / cs as f64 }
    }
}
fn size(ir: &InventionIr) -> usize {
    let mut s = ir.preconditions.iter().map(|s| s.len()).sum::<usize>();
    s += ir.postconditions.iter().map(|s| s.len()).sum::<usize>();
    s += ir.invariants.iter().map(|s| s.len()).sum::<usize>();
    match &ir.kind { IrKind::Software(t) => s += t.blocks.len() * 10, _ => {} } s
}

// =============================================================================
// TESTS  (all inline)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: Uuid) -> PatentNode { PatentNode { id, title: "T".into(), patent_type: PatentType::Software,
        authors: vec!["A".into()], created_at_unix: 0, content_hash: vec![],
        invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![] } }
    fn make_edge(p: Uuid, c: Uuid, w: f64) -> Edge { Edge { id: Uuid::new_v4(), parent_id: p, child_id: c,
        kind: EdgeKind::Derivative, delta: None, contribution_weight: w, signature: vec![] } }

    #[test] fn test_dag_add() {
        let mut dag = PatentDag::new(); dag.add_node(make_node(Uuid::new_v4())).unwrap(); assert_eq!(dag.node_count(), 1); }
    #[test] fn test_dag_cycle() {
        let mut dag = PatentDag::new(); let a=Uuid::new_v4(); let b=Uuid::new_v4(); let c=Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap(); dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a,b,0.5)).unwrap(); dag.add_edge(&b, &c, make_edge(b,c,0.5)).unwrap();
        assert!(dag.add_edge(&c, &a, make_edge(c,a,0.5)).is_err()); }
    #[test] fn test_dag_topo() {
        let mut dag = PatentDag::new(); let a=Uuid::new_v4(); let b=Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a,b,1.0)).unwrap();
        let o = dag.topological_order_indices().unwrap();
        assert!(o.iter().position(|&x|x==a) < o.iter().position(|&x|x==b)); }
    #[test] fn test_delta() {
        let o = InventionIr { kind: IrKind::Software(SsaTrace{entry_block:"e".into(),blocks:HashMap::new(),inputs:vec![],outputs:vec![]}),
            preconditions: vec!["(= x 0)".into()], postconditions: vec!["(= y 1)".into()], invariants: vec![] };
        let d = InventionIr { kind: IrKind::Software(SsaTrace{entry_block:"e".into(),blocks:HashMap::new(),inputs:vec![],outputs:vec![]}),
            preconditions: vec!["(= x 0)".into()], postconditions: vec!["(= y 2)".into()], invariants: vec![] };
        assert!(!build_delta(Uuid::new_v4(),Uuid::new_v4(),&o,&d,&[]).unwrap().formula_smt_lib2.is_empty()); }
    #[test] fn test_solver() {
        let solver = BuiltInSolver;
        assert_eq!(solver.check_sat("(assert (= x 5))\n(assert (= x 10))\n", &SolverConfig::default()).unwrap().sat, SatResult::Unsat); }
    #[test] fn test_royalty() {
        let mut dag = PatentDag::new(); let a=Uuid::new_v4(); let b=Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a,b,1.0)).unwrap();
        assert!(RoyaltyVerifier::default().verify_all(&dag).is_ok()); }
    #[test] fn test_shares() {
        let mut dag = PatentDag::new(); let a=Uuid::new_v4(); let b=Uuid::new_v4(); let c=Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap(); dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a,b,0.5)).unwrap(); dag.add_edge(&b, &c, make_edge(b,c,0.5)).unwrap();
        let s = compute_royalty_shares(&dag, &c).unwrap();
        assert!((s[&a] - 0.25).abs() < 1e-9); assert!((s[&b] - 0.5).abs() < 1e-9); }
    #[test] fn test_taint() {
        let a=Uuid::new_v4();let b=Uuid::new_v4();
        assert_eq!(TaintValue::Unlicensed.join(&TaintValue::Licensed(a)), TaintValue::Licensed(a));
        assert_eq!(TaintValue::Licensed(a).meet(&TaintValue::Licensed(b)), TaintValue::Unlicensed); }
    #[test] fn test_he() {
        let e = HeEngine::new(HeConfig::default()); let kp = e.generate_keypair();
        let ct = e.encrypt(0.42, &kp.public_key); assert!((0.42 - e.decrypt(&ct, &kp.private_key).unwrap()).abs() < 0.001); }
    #[test] fn test_registry() {
        let mut r = PatentRegistry::new(); let n = make_node(Uuid::new_v4());
        r.register(&n, "0xAlice"); assert!(r.verify_commitment(&n));
        let mut t = n.clone(); t.title = "X".into(); assert!(!r.verify_commitment(&t)); }
    #[test] fn test_gate() {
        let mut g = ExecutionGate::new(); let pid = Uuid::new_v4();
        g.request_release(pid, "0xF", "Qm"); assert!(!g.is_released(&pid, "0xF")); }
    #[test] fn test_p2p() {
        let mut a = NetworkNode::new(Uuid::new_v4()); let mut b = NetworkNode::new(Uuid::new_v4());
        a.add_peer(b.id, "127.0.0.1:9001");
        let msg = NetworkMessage::AnnounceNode(make_node(Uuid::new_v4()));
        assert!(b.receive(a.id, msg).is_some()); }
    #[test] fn test_storage() {
        let mut s = StorageNode::new(); let data = b"hello";
        let cid = s.store(data); assert_eq!(s.get(&cid), Some(&data[..])); assert!(s.verify(&cid, data)); }
    #[test] fn test_spec() {
        let ir = InventionIr { kind: IrKind::Software(SsaTrace{entry_block:"e".into(),blocks:HashMap::new(),inputs:vec![("x".into(),IrType::Int(32))],outputs:vec![]}),
            preconditions: vec!["x > 0".into()], postconditions: vec!["y == x + 1".into()], invariants: vec![] };
        assert!(!SpecSynthesizer::synthesize(&ir, Uuid::new_v4()).spec_bytes.is_empty()); }
    #[test] fn test_fidelity_contradiction() {
        let issue = check_contradiction("x > 5", "x < 3");
        assert!(issue.is_some()); }
    #[test] fn test_fidelity_no_false() {
        assert!(check_contradiction("x > 5", "x > 3").is_none()); }
    #[test] fn test_fidelity_vacuous() {
        let v = IRVerifier::default();
        let r = v.verify(&[AnnotatedInvariant::exact("true", "trivial")]);
        assert!(!r.sound); }
    #[test] fn test_fidelity_low_conf() {
        let v = IRVerifier::new(0.8);
        let r = v.verify(&[AnnotatedInvariant::heuristic("x > 0", AbstractionGap::DeliberateSimplification{reason:"test".into()},"low")]);
        assert!(!r.can_use_for_infringement); }
    #[test] fn test_abstract_interval_unsat() {
        assert_eq!(AbstractInterpreter2::check_sat(&vec!["x > 5".into(), "x < 3".into()]), Some(false)); }
    #[test] fn test_abstract_interval_sat() {
        assert_eq!(AbstractInterpreter2::check_sat(&vec!["x > 5".into(), "x < 10".into()]), Some(true)); }
    #[test] fn test_evidence_package() {
        let p = LegalEvidencePackage::new("test summary");
        assert!(p.verify()); }
    #[test] fn test_wipo() {
        let n = make_node(Uuid::new_v4());
        assert!(PatentOfficeConverter2::to_wipo_xml(&n).contains("T")); }
    #[test] fn test_compliance() {
        let c = ComplianceConfig2 { data_residency: DataResidency2::EU, encrypt_pii: true, audit_logging: true, retention_days: 365*20 };
        assert!(c.meets_gdpr()); }
    // Contribution governance tests
    fn make_test_ir(blocks: usize, pre: &[&str], post: &[&str]) -> InventionIr {
        let mut b = std::collections::HashMap::new();
        for i in 0..blocks { b.insert(format!("b{i}"), BasicBlock { label: format!("b{i}"), instructions: vec![], terminator: Terminator::Return(None) }); }
        InventionIr { kind: IrKind::Software(SsaTrace { entry_block: "b0".into(), blocks: b, inputs: vec![], outputs: vec![] }),
            preconditions: pre.iter().map(|s| s.to_string()).collect(),
            postconditions: post.iter().map(|s| s.to_string()).collect(), invariants: vec![] }
    }
    #[test] fn test_governor_reduces_weight() {
        let p = make_test_ir(2, &["x > 0"], &["y = x"]);
        let c = make_test_ir(2, &["x > 0"], &["y = x"]); // identical
        assert!(ContributionGovernor::govern(&p, &c, 0.9).final_weight < 0.9); }
    #[test] fn test_governor_allows_weight() {
        let p = make_test_ir(1, &["x > 0"], &["y = x"]);
        let c = make_test_ir(5, &["x > 0"], &["y = x", "z = y+1"]); // 4 new blocks, 1 new invariant
        let g = ContributionGovernor::govern(&p, &c, 0.3);
        assert!(g.final_weight > 0.0 && g.final_weight <= 0.5); }
}
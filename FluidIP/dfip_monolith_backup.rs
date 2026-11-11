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
    pub id: Uuid,
    pub title: String,
    pub patent_type: PatentType,
    pub authors: Vec<String>,
    pub created_at_unix: i64,
    pub content_hash: Vec<u8>,
    pub invariant: Option<SymbolicExpression>,
    pub tags: Vec<String>,
    pub license: License,
    pub meta_commitment: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PatentType { Software, Chemical, Mechanical }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum License { Open, Protected, Secret }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: Uuid,
    pub parent_id: Uuid,
    pub child_id: Uuid,
    pub kind: EdgeKind,
    pub delta: Option<DeltaConstraint>,
    pub contribution_weight: f64,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EdgeKind { Derivative, Improvement, Fork, Merge }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SymbolicExpression {
    pub format: ExpressionFormat,
    pub encoded_ir: Vec<u8>,
    pub free_variables: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExpressionFormat { SsaTrace, Molecular, Geometric, SmtLib2 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeltaConstraint {
    pub original_node_id: Uuid,
    pub derivative_node_id: Uuid,
    pub original_invariant_hash: Vec<u8>,
    pub formula_smt_lib2: Vec<u8>,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VerificationResult {
    pub edge_id: Uuid,
    pub sat: SatResult,
    pub model: Vec<u8>,
    pub unsat_core: Vec<Vec<u8>>,
    pub severity_score: f64,
    pub solver_time_ms: f64,
    pub proof_commitment: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SatResult { Sat, Unsat, Unknown }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RoyaltyInvariant {
    pub patent_node_id: Uuid,
    pub shares: Vec<ShareEntry>,
    pub upstream_nodes: Vec<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ShareEntry { pub node_id: Uuid, pub weight: f64 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaintLabel {
    pub id: Uuid,
    pub origin_license: License,
    pub derivation_chain: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaintLatticeJoin { Meet, Join, LeastUpperBound }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaintPropagationRule {
    pub operator: String,
    pub join_semantics: TaintLatticeJoin,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EncryptedShare {
    pub ciphertext: Vec<u8>,
    pub scheme: String,
    pub public_key_hash: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContributionProof {
    pub proof: Vec<u8>,
    pub proof_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IPSpecification {
    pub node_id: Uuid,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub resource_bounds: Vec<String>,
    pub spec_language: String,
    pub spec_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PatentGraphState {
    pub nodes: HashMap<Uuid, PatentNode>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LifecycleEvent {
    pub id: Uuid,
    pub event_type: LifecycleEventType,
    pub node_id: Uuid,
    pub timestamp_unix: i64,
    pub signature: Vec<u8>,
    pub parent_events: Vec<Uuid>,
    pub merkle_root: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LifecycleEventType { NodeCreated, NodeModified, NodeBranched, EdgeAdded, EdgeRemoved }

// =============================================================================
// SECTION 2: ERROR TYPES  (error.rs)
// =============================================================================

use std::fmt;

#[derive(Debug)]
pub enum CoreError {
    NodeNotFound(String),
    EdgeNotFound(String),
    CycleDetected { parent: String, child: String },
    InvalidContributionWeight(f64),
    DuplicateNode(String),
    DuplicateEdge(String),
    SmtSolverError(String),
    VerificationTimeout(u64),
    InvalidFormula(String),
    RoyaltySumViolation { sum: f64 },
    RoyaltyCycle(String),
    UnlicensedFragment(String),
    TaintPropagationError(String),
    CryptoError(String),
    ProofVerificationFailed(String),
    SerializationError(String),
    DeserializationError(String),
    InvalidArgument(String),
    Internal(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::NodeNotFound(s) => write!(f, "node not found: {s}"),
            CoreError::EdgeNotFound(s) => write!(f, "edge not found: {s}"),
            CoreError::CycleDetected { parent, child } => {
                write!(f, "cycle detected: {parent} -> {child}")
            }
            CoreError::InvalidContributionWeight(w) => {
                write!(f, "invalid contribution weight: {w}")
            }
            CoreError::DuplicateNode(s) => write!(f, "duplicate node: {s}"),
            CoreError::DuplicateEdge(s) => write!(f, "duplicate edge: {s}"),
            CoreError::SmtSolverError(s) => write!(f, "SMT solver error: {s}"),
            CoreError::VerificationTimeout(t) => write!(f, "verification timeout: {t}ms"),
            CoreError::InvalidFormula(s) => write!(f, "invalid formula: {s}"),
            CoreError::RoyaltySumViolation { sum } => {
                write!(f, "royalty sum violation: {sum}")
            }
            CoreError::RoyaltyCycle(s) => write!(f, "royalty cycle: {s}"),
            CoreError::UnlicensedFragment(s) => write!(f, "unlicensed fragment: {s}"),
            CoreError::TaintPropagationError(s) => write!(f, "taint error: {s}"),
            CoreError::CryptoError(s) => write!(f, "crypto error: {s}"),
            CoreError::ProofVerificationFailed(s) => write!(f, "proof failed: {s}"),
            CoreError::SerializationError(s) => write!(f, "ser error: {s}"),
            CoreError::DeserializationError(s) => write!(f, "de error: {s}"),
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
    node_index: HashMap<Uuid, NodeIndex>,
    edge_index: HashMap<Uuid, EdgeIndex>,
    lifecycle_log: Vec<LifecycleEvent>,
    last_event_hash: Vec<u8>,
}

impl PatentDag {
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::new(),
            node_index: HashMap::new(),
            edge_index: HashMap::new(),
            lifecycle_log: Vec::new(),
            last_event_hash: Sha256::digest(b"genesis").to_vec(),
        }
    }

    pub fn add_node(&mut self, node: PatentNode) -> Result<(), CoreError> {
        let id = node.id;
        if self.node_index.contains_key(&id) {
            return Err(CoreError::DuplicateNode(id.to_string()));
        }
        let idx = self.graph.add_node(node);
        self.node_index.insert(id, idx);
        self.push_lifecycle_event(id, LifecycleEventType::NodeCreated);
        Ok(())
    }

    pub fn get_node(&self, id: &Uuid) -> Result<&PatentNode, CoreError> {
        let idx = self.node_index.get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        Ok(&self.graph[*idx])
    }

    pub fn get_node_mut(&mut self, id: &Uuid) -> Result<&mut PatentNode, CoreError> {
        let idx = self.node_index.get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        Ok(&mut self.graph[*idx])
    }

    pub fn update_node<F>(&mut self, id: &Uuid, updater: F) -> Result<(), CoreError>
    where F: FnOnce(&mut PatentNode) {
        let idx = self.node_index.get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        updater(&mut self.graph[*idx]);
        self.push_lifecycle_event(*id, LifecycleEventType::NodeModified);
        Ok(())
    }

    pub fn remove_node(&mut self, id: &Uuid) -> Result<PatentNode, CoreError> {
        let idx = self.node_index.remove(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let edges: Vec<Uuid> = self.edge_index.iter()
            .filter(|(_, &ei)| {
                let (a, b) = self.graph.edge_endpoints(ei).unwrap();
                a == idx || b == idx
            }).map(|(uid, _)| *uid).collect();
        for eid in &edges { self.edge_index.remove(eid); }
        let node = self.graph.remove_node(idx).unwrap();
        self.push_lifecycle_event(*id, LifecycleEventType::NodeCreated);
        Ok(node)
    }

    pub fn contains_node(&self, id: &Uuid) -> bool { self.node_index.contains_key(id) }
    pub fn node_count(&self) -> usize { self.graph.node_count() }

    pub fn add_edge(&mut self, parent_id: &Uuid, child_id: &Uuid, edge: Edge) -> Result<(), CoreError> {
        let parent_idx = self.node_index.get(parent_id)
            .ok_or_else(|| CoreError::NodeNotFound(parent_id.to_string()))?;
        let child_idx = self.node_index.get(child_id)
            .ok_or_else(|| CoreError::NodeNotFound(child_id.to_string()))?;
        if !(0.0..=1.0).contains(&edge.contribution_weight) {
            return Err(CoreError::InvalidContributionWeight(edge.contribution_weight));
        }
        let edge_idx = self.graph.add_edge(*parent_idx, *child_idx, edge);
        if is_cyclic_directed(&self.graph) {
            self.graph.remove_edge(edge_idx);
            return Err(CoreError::CycleDetected {
                parent: parent_id.to_string(), child: child_id.to_string()
            });
        }
        let eid = self.graph[edge_idx].id;
        self.edge_index.insert(eid, edge_idx);
        self.push_lifecycle_event(eid, LifecycleEventType::EdgeAdded);
        Ok(())
    }

    pub fn get_edge(&self, id: &Uuid) -> Result<&Edge, CoreError> {
        let idx = self.edge_index.get(id)
            .ok_or_else(|| CoreError::EdgeNotFound(id.to_string()))?;
        Ok(&self.graph[*idx])
    }

    pub fn remove_edge(&mut self, id: &Uuid) -> Result<Edge, CoreError> {
        let idx = self.edge_index.remove(id)
            .ok_or_else(|| CoreError::EdgeNotFound(id.to_string()))?;
        let edge = self.graph.remove_edge(idx).unwrap();
        self.push_lifecycle_event(*id, LifecycleEventType::EdgeRemoved);
        Ok(edge)
    }

    pub fn edge_count(&self) -> usize { self.graph.edge_count() }

    pub fn topological_order(&self) -> Result<Vec<&PatentNode>, CoreError> {
        let order = toposort(&self.graph, None)
            .map_err(|_| CoreError::Internal("cycle in DAG".into()))?;
        Ok(order.iter().map(|&idx| &self.graph[idx]).collect())
    }

    pub fn topological_order_indices(&self) -> Result<Vec<Uuid>, CoreError> {
        let order = toposort(&self.graph, None)
            .map_err(|_| CoreError::Internal("cycle in DAG".into()))?;
        Ok(order.iter().map(|&idx| self.graph[idx].id).collect())
    }

    pub fn ancestors(&self, id: &Uuid) -> Result<Vec<&PatentNode>, CoreError> {
        let idx = self.node_index.get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut visited = HashSet::new();
        let mut stack = vec![*idx];
        let mut result = Vec::new();
        while let Some(c) = stack.pop() {
            if visited.insert(c) {
                result.push(&self.graph[c]);
                for n in self.graph.neighbors_directed(c, Direction::Incoming) {
                    if !visited.contains(&n) { stack.push(n); }
                }
            }
        }
        Ok(result)
    }

    pub fn descendants(&self, id: &Uuid) -> Result<Vec<&PatentNode>, CoreError> {
        let idx = self.node_index.get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut bfs = Bfs::new(&self.graph, *idx);
        let mut result = Vec::new();
        while let Some(nx) = bfs.next(&self.graph) { result.push(&self.graph[nx]); }
        Ok(result)
    }

    pub fn ancestor_ids(&self, id: &Uuid) -> Result<HashSet<Uuid>, CoreError> {
        let idx = self.node_index.get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut visited = HashSet::new();
        let mut stack = vec![*idx];
        while let Some(c) = stack.pop() {
            if visited.insert(self.graph[c].id) {
                for n in self.graph.neighbors_directed(c, Direction::Incoming) { stack.push(n); }
            }
        }
        Ok(visited)
    }

    pub fn descendant_ids(&self, id: &Uuid) -> Result<HashSet<Uuid>, CoreError> {
        let idx = self.node_index.get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut bfs = Bfs::new(&self.graph, *idx);
        let mut result = HashSet::new();
        while let Some(nx) = bfs.next(&self.graph) { result.insert(self.graph[nx].id); }
        Ok(result)
    }

    pub fn has_path(&self, from: &Uuid, to: &Uuid) -> Result<bool, CoreError> {
        let fi = self.node_index.get(from)
            .ok_or_else(|| CoreError::NodeNotFound(from.to_string()))?;
        let ti = self.node_index.get(to)
            .ok_or_else(|| CoreError::NodeNotFound(to.to_string()))?;
        Ok(has_path_connecting(&self.graph, *fi, *ti, None))
    }

    pub fn lowest_common_ancestors(&self, a: &Uuid, b: &Uuid) -> Result<Vec<&PatentNode>, CoreError> {
        let aa = self.ancestor_ids(a)?;
        let ba = self.ancestor_ids(b)?;
        let common: HashSet<Uuid> = aa.intersection(&ba).copied().collect();
        if common.is_empty() { return Ok(vec![]); }
        let topo = self.topological_order_indices()?;
        let mut lcas = Vec::new();
        for nid in &topo {
            if common.contains(nid) {
                let desc = self.descendant_ids(nid)?;
                let has_child = desc.iter().any(|d| d != nid && common.contains(d));
                if !has_child { lcas.push(*nid); }
            }
        }
        Ok(lcas.iter().map(|id| &self.graph[self.node_index[id]]).collect())
    }

    pub fn find_paths(&self, from: &Uuid, to: &Uuid, max: usize) -> Result<Vec<Vec<Uuid>>, CoreError> {
        let fi = self.node_index.get(from)
            .ok_or_else(|| CoreError::NodeNotFound(from.to_string()))?;
        let ti = self.node_index.get(to)
            .ok_or_else(|| CoreError::NodeNotFound(to.to_string()))?;
        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(vec![*fi]);
        while let Some(path) = queue.pop_front() {
            if results.len() >= max { break; }
            let last = *path.last().unwrap();
            if last == *ti { results.push(path.iter().map(|&n| self.graph[n].id).collect()); continue; }
            for nb in self.graph.neighbors(last) {
                if !path.contains(&nb) {
                    let mut p = path.clone(); p.push(nb); queue.push_back(p);
                }
            }
        }
        Ok(results)
    }

    pub fn distance(&self, from: &Uuid, to: &Uuid) -> Result<Option<usize>, CoreError> {
        let fi = self.node_index.get(from)
            .ok_or_else(|| CoreError::NodeNotFound(from.to_string()))?;
        let ti = self.node_index.get(to)
            .ok_or_else(|| CoreError::NodeNotFound(to.to_string()))?;
        if !has_path_connecting(&self.graph, *fi, *ti, None) { return Ok(None); }
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        queue.push_back((*fi, 0usize));
        visited.insert(*fi);
        while let Some((c, d)) = queue.pop_front() {
            if c == *ti { return Ok(Some(d)); }
            for nb in self.graph.neighbors(c) { if visited.insert(nb) { queue.push_back((nb, d+1)); } }
        }
        Ok(None)
    }

    pub fn export_state(&self) -> PatentGraphState {
        let nodes: HashMap<Uuid, PatentNode> = self.graph.node_references()
            .map(|(_, n)| (n.id, n.clone())).collect();
        let edges: Vec<Edge> = self.graph.edge_references()
            .map(|e| e.weight().clone()).collect();
        PatentGraphState { nodes, edges }
    }

    pub fn lifecycle_log(&self) -> &[LifecycleEvent] { &self.lifecycle_log }
    pub fn last_event_hash(&self) -> &[u8] { &self.last_event_hash }

    fn push_lifecycle_event(&mut self, nid: Uuid, etype: LifecycleEventType) {
        let eid = Uuid::new_v4();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        let mut h = Sha256::new();
        h.update(&self.last_event_hash); h.update(eid.as_bytes());
        h.update(nid.as_bytes()); h.update(ts.to_le_bytes());
        let root = h.finalize().to_vec();
        self.lifecycle_log.push(LifecycleEvent {
            id: eid, event_type: etype, node_id: nid, timestamp_unix: ts,
            signature: vec![],
            parent_events: vec![Uuid::from_slice(&self.last_event_hash[..16]).unwrap_or_default()],
            merkle_root: root.clone(),
        });
        self.last_event_hash = root;
    }
}

impl Default for PatentDag { fn default() -> Self { Self::new() } }

// =============================================================================
// SECTION 4: SYMBOLIC IR  (ir.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrValue {
    Int(i64), Float(f64), Bool(bool), BitVector(Vec<u8>),
    Variable(String), Term { function: String, args: Vec<Box<IrValue>> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SsaOp {
    Add, Sub, Mul, Div, And, Or, Xor, Shl, Lshr,
    Eq, Ne, Lt, Le, Gt, Ge,
    Load { address: Box<IrExpr> }, Store { address: Box<IrExpr>, value: Box<IrExpr> },
    Phi { incoming: Vec<IrExpr> }, Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SsaInstruction {
    pub result: Option<String>, pub op: SsaOp, pub operands: Vec<IrExpr>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BasicBlock {
    pub label: String, pub instructions: Vec<SsaInstruction>, pub terminator: Terminator,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Terminator {
    Branch(String),
    BranchCond { condition: IrExpr, true_target: String, false_target: String },
    Return(Option<IrExpr>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SsaTrace {
    pub entry_block: String, pub blocks: HashMap<String, BasicBlock>,
    pub inputs: Vec<(String, IrType)>, pub outputs: Vec<(String, IrType)>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrType {
    Int(u32), Float(u32), Bool, BitVector(u32),
    Array(Box<IrType>, Box<IrType>), Struct(Vec<(String, IrType)>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrExpr {
    Value(IrValue), Ref(String),
    Unary { op: UnaryOp, expr: Box<IrExpr> },
    Binary { op: BinaryOp, left: Box<IrExpr>, right: Box<IrExpr> },
    Ite { cond: Box<IrExpr>, then_expr: Box<IrExpr>, else_expr: Box<IrExpr> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UnaryOp { Neg, Not, BitNot }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BinaryOp { Add, Sub, Mul, Div, Mod, And, Or, Xor, Shl, Lshr, Eq, Ne, Lt, Le, Gt, Ge }

// Chemical
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BondType { Single, Double, Triple, Aromatic, Hydrogen }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Atom { pub symbol: String, pub formal_charge: i32, pub isotope: Option<u32>,
    pub stereochemistry: Option<String> }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bond { pub atom_a: usize, pub atom_b: usize, pub bond_type: BondType }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MolecularGraph {
    pub atoms: Vec<Atom>, pub bonds: Vec<Bond>, pub constraints: Vec<ChemicalConstraint>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ChemicalConstraint {
    Chirality { atom_index: usize, configuration: String },
    BondAngle { atom_a: usize, atom_b: usize, atom_c: usize, min_degrees: f64, max_degrees: f64 },
    Distance { atom_a: usize, atom_b: usize, min_angstrom: f64, max_angstrom: f64 },
    ChargeRange { atom_index: usize, min: i32, max: i32 },
    ReactionInvariant { description: String },
}

// Mechanical
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point3 { pub x: f64, pub y: f64, pub z: f64 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GeometricConstraint {
    Coincident { entities: Vec<usize> }, Parallel { entities: Vec<usize> },
    Perpendicular { entities: Vec<usize> }, Concentric { entities: Vec<usize> },
    Distance { entity_a: usize, entity_b: usize, min: f64, max: f64 },
    Angle { entity_a: usize, entity_b: usize, min_degrees: f64, max_degrees: f64 },
    Tangent { entities: Vec<usize> },
    Symmetry { entity_a: usize, entity_b: usize, about: usize },
    Fixed { entity: usize, position: Point3 }, Tolerance { entity: usize, tolerance: f64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GeometricEntity {
    Point(Point3), Line { point: Point3, direction: Point3 },
    Plane { point: Point3, normal: Point3 }, Circle { center: Point3, normal: Point3, radius: f64 },
    Cylinder { axis_point: Point3, axis_direction: Point3, radius: f64 },
    Sphere { center: Point3, radius: f64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MechanicalModel {
    pub entities: Vec<GeometricEntity>, pub constraints: Vec<GeometricConstraint>,
    pub metadata: HashMap<String, String>,
}

// Top-level IR
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrKind { Software(SsaTrace), Chemical(MolecularGraph), Mechanical(MechanicalModel) }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InventionIr {
    pub kind: IrKind,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
}

pub fn extract_invariants(ir: &InventionIr) -> Vec<String> {
    let mut v = Vec::new();
    v.extend(ir.preconditions.clone()); v.extend(ir.postconditions.clone()); v.extend(ir.invariants.clone());
    v
}

pub fn describe_ir(ir: &InventionIr) -> String {
    match &ir.kind {
        IrKind::Software(t) => format!("Software IR: {} blocks, {} ins, {} outs",
            t.blocks.len(), t.inputs.len(), t.outputs.len()),
        IrKind::Chemical(m) => format!("Chemical IR: {} atoms, {} bonds", m.atoms.len(), m.bonds.len()),
        IrKind::Mechanical(m) => format!("Mechanical IR: {} entities, {} constraints",
            m.entities.len(), m.constraints.len()),
    }
}

// =============================================================================
// SECTION 5: DELTA CONSTRAINT BUILDER  (delta.rs)
// =============================================================================

pub fn build_delta(
    original_id: Uuid, derivative_id: Uuid,
    original_ir: &InventionIr, derivative_ir: &InventionIr,
    assumptions: &[String],
) -> Result<DeltaConstraint, CoreError> {
    let o_invs = extract_invariants(original_ir);
    let d_invs = extract_invariants(derivative_ir);
    let mut h = Sha256::new();
    for inv in &o_invs { h.update(inv.as_bytes()); }
    let hash = h.finalize().to_vec();
    let formula = build_smt_lib2(&o_invs, &d_invs);
    Ok(DeltaConstraint {
        original_node_id: original_id, derivative_node_id: derivative_id,
        original_invariant_hash: hash, formula_smt_lib2: formula.into_bytes(),
        assumptions: assumptions.to_vec(),
    })
}

fn build_smt_lib2(original: &[String], derivative: &[String]) -> String {
    let neg = if original.is_empty() { "true".into() } else {
        let c = original.iter().map(|s| format!("(assert {s})")).collect::<Vec<_>>().join("\n  ");
        format!("(not (and\n  {c}\n))")
    };
    let d = if derivative.is_empty() { "true".into() } else {
        derivative.iter().map(|s| format!("(assert {s})")).collect::<Vec<_>>().join("\n  ")
    };
    format!(
        "(set-logic ALL)\n; Δ = ¬Original ∧ Derivative\n(declare-fun delta-query () Bool)\n\
         (assert\n  (and\n    {neg}\n    {d}\n  )\n)\n(check-sat)\n(get-model)\n"
    )
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuditEntry {
    pub id: Uuid, pub delta_constraint: DeltaConstraint,
    pub timestamp_unix: i64, pub previous_hash: Vec<u8>, pub merkle_root: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditTrail {
    entries: Vec<AuditEntry>, last_hash: Vec<u8>,
}

impl AuditTrail {
    pub fn new() -> Self { Self { entries: vec![], last_hash: Sha256::digest(b"audit-genesis").to_vec() } }
    pub fn push(&mut self, delta: DeltaConstraint) -> Uuid {
        let id = Uuid::new_v4();
        let mut h = Sha256::new();
        h.update(&self.last_hash); h.update(&delta.formula_smt_lib2); h.update(id.as_bytes());
        let root = h.finalize().to_vec();
        self.entries.push(AuditEntry {
            id, delta_constraint: delta,
            timestamp_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
            previous_hash: self.last_hash.clone(), merkle_root: root.clone(),
        });
        self.last_hash = root; id
    }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn verify(&self) -> bool {
        let mut cur = Sha256::digest(b"audit-genesis").to_vec();
        for e in &self.entries {
            let mut h = Sha256::new();
            h.update(&cur); h.update(&e.delta_constraint.formula_smt_lib2); h.update(e.id.as_bytes());
            if h.finalize().to_vec() != e.merkle_root { return false; }
            cur = e.merkle_root.clone();
        }
        cur == self.last_hash
    }
}
impl Default for AuditTrail { fn default() -> Self { Self::new() } }

// =============================================================================
// SECTION 6: SMT SOLVER  (solver.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SolverBackend { Z3, Bitwuzla, Cvc5, BuiltIn }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverConfig {
    pub backend: SolverBackend, pub timeout_ms: u64, pub solver_path: Option<String>,
}
impl Default for SolverConfig {
    fn default() -> Self { Self { backend: SolverBackend::BuiltIn, timeout_ms: 5000, solver_path: None } }
}

pub trait SmtSolver: Send + Sync {
    fn check_sat(&self, formula: &str, config: &SolverConfig) -> Result<SolverResponse, CoreError>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverResponse {
    pub sat: SatResult, pub model: HashMap<String, String>,
    pub unsat_core: Vec<String>, pub solver_time_ms: f64,
}

// Built-in naive solver
pub struct BuiltInSolver;
impl BuiltInSolver {
    fn naive_check(formula: &str) -> SatResult {
        let mut assignments: HashMap<String, i64> = HashMap::new();
        for line in formula.lines() {
            let line = line.trim();
            if let Some(body) = line.strip_prefix("(assert ").and_then(|s| s.strip_suffix(')')) {
                // (= VAR VAL)
                if let Some((var, val_str)) = parse_eq(body) {
                    if let Ok(val) = val_str.parse::<i64>() {
                        if let Some(&ex) = assignments.get(&var) { if ex != val { return SatResult::Unsat; } }
                        assignments.insert(var, val);
                    }
                }
                // (not (= VAR VAL))
                if let Some((var, val_str)) = parse_not_eq(body) {
                    if let Ok(val) = val_str.parse::<i64>() {
                        if let Some(&ex) = assignments.get(&var) { if ex == val { return SatResult::Unsat; } }
                    }
                }
            }
        }
        SatResult::Sat
    }
}

fn parse_eq(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if s.starts_with("(= ") && s.ends_with(')') {
        let inner = &s[3..s.len()-1];
        let parts: Vec<&str> = inner.split_whitespace().collect();
        if parts.len() == 2 { return Some((parts[0].to_string(), parts[1].to_string())); }
    }
    None
}

fn parse_not_eq(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if s.starts_with("(not ") && s.ends_with(')') {
        return parse_eq(&s[5..s.len()-1].trim());
    }
    None
}

impl SmtSolver for BuiltInSolver {
    fn check_sat(&self, formula: &str, config: &SolverConfig) -> Result<SolverResponse, CoreError> {
        let start = Instant::now();
        let sat = Self::naive_check(formula);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        if elapsed > config.timeout_ms as f64 {
            return Ok(SolverResponse { sat: SatResult::Unknown, model: HashMap::new(),
                unsat_core: vec![], solver_time_ms: elapsed });
        }
        Ok(SolverResponse { sat, model: HashMap::new(), unsat_core: vec![], solver_time_ms: elapsed })
    }
}

pub struct ExternalSolver { binary_name: String }
impl ExternalSolver {
    pub fn new(backend: &SolverBackend) -> Self {
        let name = match backend {
            SolverBackend::Z3 => "z3", SolverBackend::Bitwuzla => "bitwuzla",
            SolverBackend::Cvc5 => "cvc5", _ => unreachable!(),
        };
        Self { binary_name: name.to_string() }
    }
}

impl SmtSolver for ExternalSolver {
    fn check_sat(&self, formula: &str, config: &SolverConfig) -> Result<SolverResponse, CoreError> {
        let binary = config.solver_path.clone().unwrap_or_else(|| self.binary_name.clone());
        let start = Instant::now();
        let output = std::process::Command::new(&binary)
            .arg("-in").arg(format!("-T:{}", config.timeout_ms / 1000))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())
            .spawn();
        let output = match output {
            Ok(mut child) => {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin { let _ = stdin.write_all(formula.as_bytes()); }
                child.wait_with_output()
                    .map_err(|e| CoreError::SmtSolverError(format!("solver error: {e}")))?
            }
            Err(e) => return Err(CoreError::SmtSolverError(
                format!("failed to launch '{binary}': {e}. Is it installed?"))),
        };
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let sat = if stdout.contains("sat") { SatResult::Sat }
            else if stdout.contains("unsat") { SatResult::Unsat } else { SatResult::Unknown };
        let mut model = HashMap::new();
        if sat == SatResult::Sat {
            for line in stdout.lines() {
                let line = line.trim();
                if let Some(def) = line.strip_prefix("(define-fun ") {
                    let parts: Vec<&str> = def.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let var = parts[0].to_string();
                        if let Some(vs) = parts[1].find(' ') {
                            let val = parts[1][vs..].trim().trim_end_matches(')').to_string();
                            model.insert(var, val);
                        }
                    }
                }
            }
        }
        Ok(SolverResponse { sat, model, unsat_core: vec![], solver_time_ms: elapsed })
    }
}

pub fn create_solver(config: &SolverConfig) -> Box<dyn SmtSolver> {
    match config.backend { SolverBackend::BuiltIn => Box::new(BuiltInSolver), _ => Box::new(ExternalSolver::new(&config.backend)) }
}

pub fn run_verification(edge_id: Uuid, smt: &[u8], solver: &dyn SmtSolver, config: &SolverConfig) -> Result<VerificationResult, CoreError> {
    let formula = String::from_utf8(smt.to_vec())
        .map_err(|_| CoreError::InvalidFormula("non-UTF-8 SMT-LIB2".into()))?;
    let response = solver.check_sat(&formula, config)?;
    let severity = compute_severity(&response);
    Ok(VerificationResult {
        edge_id, sat: response.sat,
        model: serde_json::to_vec(&response.model).unwrap_or_default(),
        unsat_core: response.unsat_core.iter().map(|s| s.as_bytes().to_vec()).collect(),
        severity_score: severity, solver_time_ms: response.solver_time_ms, proof_commitment: vec![],
    })
}

pub fn compute_severity(response: &SolverResponse) -> f64 {
    match response.sat {
        SatResult::Sat => 0.5 + (response.model.len() as f64 / 20.0).min(0.5),
        SatResult::Unsat => (response.unsat_core.len() as f64 / 10.0).min(0.5),
        SatResult::Unknown => 0.5,
    }
}

// =============================================================================
// SECTION 7: ROYALTY INVARIANTS  (royalties.rs)
// =============================================================================

pub struct RoyaltyVerifier { epsilon: f64 }
impl RoyaltyVerifier {
    pub fn new(epsilon: f64) -> Self { Self { epsilon } }
    pub fn verify_node(&self, dag: &PatentDag, nid: &Uuid) -> Result<(), CoreError> {
        let _ = dag.get_node(nid)?;
        let state = dag.export_state();
        let incoming: Vec<&Edge> = state.edges.iter().filter(|e| e.child_id == *nid).collect();
        if incoming.is_empty() { return Ok(()); }
        let sum: f64 = incoming.iter().map(|e| e.contribution_weight).sum();
        if (sum - 1.0).abs() > self.epsilon { return Err(CoreError::RoyaltySumViolation { sum }); }
        Ok(())
    }
    pub fn verify_all(&self, dag: &PatentDag) -> Result<(), CoreError> {
        for nid in dag.export_state().nodes.keys() { self.verify_node(dag, nid)?; }
        Ok(())
    }
}
impl Default for RoyaltyVerifier { fn default() -> Self { Self::new(1e-9) } }

pub fn compute_royalty_shares(dag: &PatentDag, nid: &Uuid) -> Result<HashMap<Uuid, f64>, CoreError> {
    let _ = dag.get_node(nid)?;
    let ancestors = dag.ancestor_ids(nid)?;
    let topo_all = dag.topological_order_indices()?;
    let topo: Vec<&Uuid> = topo_all.iter().filter(|id| ancestors.contains(id)).collect();
    let state = dag.export_state();
    let mut shares: HashMap<Uuid, f64> = HashMap::new();
    shares.insert(*nid, 1.0);
    for cur in topo.iter().rev() {
        let cs = *shares.get(cur).unwrap_or(&0.0);
        if cs == 0.0 { continue; }
        for e in &state.edges {
            if e.child_id == **cur { *shares.entry(e.parent_id).or_insert(0.0) += cs * e.contribution_weight; }
        }
    }
    Ok(shares)
}

pub fn compute_royalty_payments(dag: &PatentDag, nid: &Uuid, base: f64) -> Result<HashMap<Uuid, f64>, CoreError> {
    Ok(compute_royalty_shares(dag, nid)?.into_iter().map(|(id, s)| (id, s * base)).collect())
}

// =============================================================================
// SECTION 8: TAINT TRACKING  (taint.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TaintValue {
    Unlicensed, Licensed(Uuid), Multiple(HashSet<Uuid>),
}

impl TaintValue {
    pub fn bottom() -> Self { TaintValue::Unlicensed }
    pub fn join(&self, other: &Self) -> Self {
        match (self, other) {
            (TaintValue::Unlicensed, x) | (x, TaintValue::Unlicensed) => x.clone(),
            (TaintValue::Licensed(a), TaintValue::Licensed(b)) if a == b => TaintValue::Licensed(*a),
            (TaintValue::Licensed(a), TaintValue::Licensed(b)) => {
                let mut s = HashSet::new(); s.insert(*a); s.insert(*b); TaintValue::Multiple(s)
            }
            (TaintValue::Licensed(a), TaintValue::Multiple(bs)) => {
                let mut s = bs.clone(); s.insert(*a); TaintValue::Multiple(s)
            }
            (TaintValue::Multiple(as_), TaintValue::Licensed(b)) => {
                let mut s = as_.clone(); s.insert(*b); TaintValue::Multiple(s)
            }
            (TaintValue::Multiple(as_), TaintValue::Multiple(bs)) => {
                let mut s = as_.clone(); s.extend(bs); TaintValue::Multiple(s)
            }
            _ => TaintValue::Multiple({
                let mut s = HashSet::new();
                if let TaintValue::Licensed(a) = self { s.insert(*a); }
                if let TaintValue::Licensed(b) = other { s.insert(*b); }
                s
            }),
        }
    }
    pub fn meet(&self, other: &Self) -> Self {
        match (self, other) {
            (TaintValue::Unlicensed, _) | (_, TaintValue::Unlicensed) => TaintValue::Unlicensed,
            (TaintValue::Licensed(a), TaintValue::Licensed(b)) if a == b => TaintValue::Licensed(*a),
            (TaintValue::Licensed(a), TaintValue::Multiple(bs)) if bs.contains(a) => TaintValue::Licensed(*a),
            (TaintValue::Multiple(as_), TaintValue::Licensed(b)) if as_.contains(b) => TaintValue::Licensed(*b),
            (TaintValue::Multiple(as_), TaintValue::Multiple(bs)) => {
                let i: HashSet<Uuid> = as_.intersection(bs).copied().collect();
                if i.len() == 1 { return TaintValue::Licensed(*i.iter().next().unwrap()); }
                if i.is_empty() { return TaintValue::Unlicensed; }
                TaintValue::Multiple(i)
            }
            _ => TaintValue::Unlicensed,
        }
    }
}

pub struct TaintAnalyzer { rules: HashMap<String, TaintLatticeJoin> }
impl TaintAnalyzer {
    pub fn new() -> Self {
        let mut r = HashMap::new();
        for op in &["Add", "Sub", "Mul", "Div", "And", "Or", "Xor"] { r.insert(op.to_string(), TaintLatticeJoin::Join); }
        r.insert("Eq".into(), TaintLatticeJoin::Meet);
        r.insert("Ne".into(), TaintLatticeJoin::Meet);
        r.insert("Load".into(), TaintLatticeJoin::Join);
        r.insert("Store".into(), TaintLatticeJoin::Join);
        r.insert("Phi".into(), TaintLatticeJoin::Join);
        Self { rules: r }
    }
    pub fn analyze_trace(&self, trace: &SsaTrace, initial: &HashMap<String, TaintValue>) -> Result<HashMap<String, TaintValue>, CoreError> {
        let mut env = initial.clone();
        let mut block_order = vec![];
        let mut visited = HashSet::new();
        let mut stack = vec![trace.entry_block.clone()];
        while let Some(label) = stack.pop() {
            if visited.insert(label.clone()) {
                block_order.push(label.clone());
                if let Some(block) = trace.blocks.get(&label) {
                    match &block.terminator {
                        Terminator::Branch(t) => stack.push(t.clone()),
                        Terminator::BranchCond { true_target, false_target, .. } => {
                            stack.push(true_target.clone()); stack.push(false_target.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
        for label in &block_order {
            if let Some(block) = trace.blocks.get(label) {
                for inst in &block.instructions {
                    let result = inst.result.as_ref()
                        .ok_or_else(|| CoreError::TaintPropagationError("no result var".into()))?;
                    let op_name = format!("{:?}", inst.op);
                    let rule = self.rules.get(&op_name).unwrap_or(&TaintLatticeJoin::Join);
                    let taint = self.compute_operand_taint(&inst.operands, rule, &env);
                    env.insert(result.clone(), taint);
                }
            }
        }
        Ok(env)
    }
    fn compute_operand_taint(&self, ops: &[IrExpr], rule: &TaintLatticeJoin, env: &HashMap<String, TaintValue>) -> TaintValue {
        let taints: Vec<TaintValue> = ops.iter().map(|op| self.taint_of_expr(op, env)).collect();
        if taints.is_empty() { return TaintValue::Unlicensed; }
        match rule {
            TaintLatticeJoin::Join | TaintLatticeJoin::LeastUpperBound =>
                taints.iter().fold(TaintValue::Unlicensed, |a, t| a.join(t)),
            TaintLatticeJoin::Meet =>
                taints.iter().skip(1).fold(taints[0].clone(), |a, t| a.meet(t)),
        }
    }
    fn taint_of_expr(&self, expr: &IrExpr, env: &HashMap<String, TaintValue>) -> TaintValue {
        match expr {
            IrExpr::Value(_) => TaintValue::Unlicensed,
            IrExpr::Ref(name) => env.get(name).cloned().unwrap_or(TaintValue::Unlicensed),
            IrExpr::Unary { expr: e, .. } => self.taint_of_expr(e, env),
            IrExpr::Binary { left, right, .. } => self.taint_of_expr(left, env).join(&self.taint_of_expr(right, env)),
            IrExpr::Ite { cond, then_expr, else_expr, .. } =>
                self.taint_of_expr(cond, env).join(&self.taint_of_expr(then_expr, env)).join(&self.taint_of_expr(else_expr, env)),
        }
    }
}
impl Default for TaintAnalyzer { fn default() -> Self { Self::new() } }

// =============================================================================
// SECTION 9: IP SPECIFICATION  (spec.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IpSpec {
    pub node_id: Uuid, pub preconditions: Vec<String>, pub postconditions: Vec<String>,
    pub invariants: Vec<String>, pub resource_bounds: Vec<String>,
    pub spec_language: String, pub spec_bytes: Vec<u8>,
}

pub struct SpecSynthesizer;
impl SpecSynthesizer {
    pub fn synthesize(ir: &InventionIr, node_id: Uuid) -> IpSpec {
        let mut candidates = Vec::new();
        match &ir.kind {
            IrKind::Software(trace) => {
                for (label, block) in &trace.blocks {
                    if block.instructions.iter().any(|i| matches!(i.op, SsaOp::Phi { .. })) {
                        candidates.push(format!("loop_at_{label}: all_paths_converge"));
                    }
                }
                for (name, typ) in &trace.inputs {
                    if let IrType::Int(w) = typ {
                        candidates.push(format!("in_range({name}, 0, 2^{w})"));
                    }
                }
            }
            IrKind::Chemical(mol) => {
                candidates.push(format!("num_atoms == {}", mol.atoms.len()));
                candidates.push(format!("num_bonds == {}", mol.bonds.len()));
                for c in &mol.constraints {
                    if let ChemicalConstraint::Distance { atom_a, atom_b, min_angstrom, max_angstrom } = c {
                        candidates.push(format!("distance(a{atom_a}, a{atom_b}) ∈ [{min_angstrom}, {max_angstrom}]"));
                    }
                }
            }
            IrKind::Mechanical(model) => {
                candidates.push(format!("num_entities == {}", model.entities.len()));
                for c in &model.constraints {
                    if let GeometricConstraint::Distance { entity_a, entity_b, min, max } = c {
                        candidates.push(format!("distance(e{entity_a}, e{entity_b}) ∈ [{min}, {max}]"));
                    }
                }
            }
        }
        let lang = match &ir.kind {
            IrKind::Software(_) => "dfip-ir-ssa",
            IrKind::Chemical(_) => "dfip-ir-molecular",
            IrKind::Mechanical(_) => "dfip-ir-geometric",
        };
        let content = format!(
            "// dfip-ipspec v1\n// Auto-generated\n\n\
             // Pre-conditions:\n{}\n// Post-conditions:\n{}\n// Invariants:\n{}\n",
            ir.preconditions.iter().map(|p| format!("requires {p}")).collect::<Vec<_>>().join("\n"),
            ir.postconditions.iter().map(|p| format!("ensures {p}")).collect::<Vec<_>>().join("\n"),
            candidates.join("\n"),
        );
        IpSpec {
            node_id, preconditions: ir.preconditions.clone(),
            postconditions: ir.postconditions.clone(),
            invariants: candidates, resource_bounds: vec![],
            spec_language: lang.into(), spec_bytes: content.into_bytes(),
        }
    }
}

// =============================================================================
// SECTION 10: HOMOMORPHIC ENCRYPTION  (he.rs + zk.rs simplified)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeConfig {
    pub security_level: String, pub precision_bits: u32, pub modulus_bits: u32,
}
impl Default for HeConfig {
    fn default() -> Self { Self { security_level: "standard".into(), precision_bits: 16, modulus_bits: 64 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HePublicKey { pub id: Vec<u8>, pub config: HeConfig }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HePrivateKey { pub id: Vec<u8>, pub config: HeConfig }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeKeypair { pub public_key: HePublicKey, pub private_key: HePrivateKey }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeCiphertext { pub data: Vec<u8>, pub config: HeConfig }

pub fn rand_seed() -> String {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    format!("{}-{}", d.as_secs(), d.subsec_nanos())
}

pub struct HeEngine { config: HeConfig }
impl HeEngine {
    pub fn new(config: HeConfig) -> Self { Self { config } }
    pub fn generate_keypair(&self) -> HeKeypair {
        let id = Sha256::digest(format!("he-key-{}", rand_seed()).as_bytes()).to_vec();
        HeKeypair {
            public_key: HePublicKey { id: id.clone(), config: self.config.clone() },
            private_key: HePrivateKey { id, config: self.config.clone() },
        }
    }
    pub fn encrypt(&self, value: f64, pk: &HePublicKey) -> HeCiphertext {
        let scale = 1u64 << self.config.precision_bits;
        let fixed = (value.clamp(0.0, 1.0) * scale as f64) as u64;
        let keystream = self.derive_keystream(pk);
        let enc: Vec<u8> = fixed.to_le_bytes().iter().zip(keystream.iter()).map(|(a, b)| a ^ b).collect();
        HeCiphertext { data: enc, config: self.config.clone() }
    }
    pub fn decrypt(&self, ct: &HeCiphertext, sk: &HePrivateKey) -> Result<f64, String> {
        let keystream = self.derive_keystream_priv(sk);
        let mut plain = [0u8; 8];
        for i in 0..8.min(ct.data.len()) { plain[i] = ct.data[i] ^ keystream[i]; }
        let scale = 1u64 << self.config.precision_bits;
        Ok(u64::from_le_bytes(plain) as f64 / scale as f64)
    }
    pub fn sum(&self, cts: &[HeCiphertext], pk: &HePublicKey) -> Result<HeCiphertext, String> {
        let mut sum = 0.0;
        for ct in cts {
            let kp = self.generate_keypair();
            sum += self.decrypt(ct, &kp.private_key).unwrap_or(0.0);
        }
        Ok(self.encrypt(sum, pk))
    }
    fn derive_keystream(&self, pk: &HePublicKey) -> Vec<u8> {
        let mut h = Sha256::new();
        h.update(b"dfip-he-ks"); h.update(&pk.id); h.update(self.config.modulus_bits.to_le_bytes());
        h.finalize()[..8].to_vec()
    }
    fn derive_keystream_priv(&self, sk: &HePrivateKey) -> Vec<u8> {
        let mut h = Sha256::new();
        h.update(b"dfip-he-ks"); h.update(&sk.id); h.update(self.config.modulus_bits.to_le_bytes());
        h.finalize()[..8].to_vec()
    }
}

// ZK
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ZkProofType { Bulletproof, Groth16, Plonk, Simulated }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkProof {
    pub proof_type: ZkProofType, pub proof_bytes: Vec<u8>, pub public_inputs: Vec<u8>,
}

pub struct ZkProver;
impl ZkProver {
    pub fn prove_range(value: f64, blinding: &[u8]) -> ZkProof {
        let mut h = Sha256::new();
        h.update(b"dfip-zk-range"); h.update(value.to_le_bytes()); h.update(blinding);
        ZkProof { proof_type: ZkProofType::Simulated, proof_bytes: h.finalize().to_vec(), public_inputs: vec![] }
    }
    pub fn verify_range(proof: &ZkProof, _commitment: &[u8]) -> Result<(), String> {
        if proof.proof_bytes.is_empty() { return Err("empty proof".into()); }
        Ok(())
    }
}

// =============================================================================
// SECTION 11: IP STATE EXPLORER  (explorer.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilarityResult {
    pub source_id: Uuid, pub target_id: Uuid,
    pub sat: SatResult, pub severity: f64, pub distance: Option<usize>,
}

pub fn compare_nodes(
    dag: &PatentDag, src: &Uuid, tgt: &Uuid,
    src_ir: &InventionIr, tgt_ir: &InventionIr,
    solver: &dyn SmtSolver, config: &SolverConfig,
) -> Result<SimilarityResult, CoreError> {
    let delta = build_delta(*src, *tgt, src_ir, tgt_ir, &[])?;
    let formula = String::from_utf8(delta.formula_smt_lib2)
        .map_err(|_| CoreError::InvalidFormula("bad UTF-8".into()))?;
    let resp = solver.check_sat(&formula, config)?;
    let severity = compute_severity(&resp);
    let dist = dag.distance(src, tgt).ok().flatten();
    Ok(SimilarityResult { source_id: *src, target_id: *tgt, sat: resp.sat, severity, distance: dist })
}

pub fn cluster_by_invariants(patents: &HashMap<Uuid, InventionIr>) -> Vec<(Uuid, Vec<Uuid>, String)> {
    let mut clusters: HashMap<String, Vec<Uuid>> = HashMap::new();
    for (id, ir) in patents {
        let mut invs = extract_invariants(ir);
        invs.sort();
        clusters.entry(invs.join("|")).or_default().push(*id);
    }
    clusters.into_iter().map(|(k, v)| (Uuid::new_v4(), v, k.split('|').next().unwrap_or("").to_string())).collect()
}

pub fn compute_novelty_score(
    dag: &PatentDag, cid: &Uuid, cir: &InventionIr,
    patents: &HashMap<Uuid, InventionIr>,
    solver: &dyn SmtSolver, config: &SolverConfig,
) -> Result<f64, CoreError> {
    let mut max_sev = 0.0;
    for (pid, pir) in patents {
        if *pid == *cid { continue; }
        let d = build_delta(*pid, *cid, pir, cir, &[])?;
        let f = String::from_utf8(d.formula_smt_lib2).map_err(|_| CoreError::InvalidFormula("".into()))?;
        let r = solver.check_sat(&f, config)?;
        let s = compute_severity(&r);
        if s > max_sev { max_sev = s; }
    }
    Ok(max_sev)
}

// =============================================================================
// SECTION 12: CONTRACTS — REGISTRY, SPLITTER, ORACLE, GATE
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredPatent {
    pub id: Uuid, pub title: String, pub registered_at: i64,
    pub registrant: String, pub commitment: Vec<u8>,
    pub invariant_hash: Vec<u8>, pub license: License,
}

pub struct PatentRegistry { patents: HashMap<Uuid, RegisteredPatent> }
impl PatentRegistry {
    pub fn new() -> Self { Self { patents: HashMap::new() } }
    pub fn register(&mut self, node: &PatentNode, registrant: &str) -> RegisteredPatent {
        let data = serde_json::to_vec(node).unwrap_or_default();
        let commitment = Sha256::digest(&data).to_vec();
        let ih = node.invariant.as_ref()
            .map(|i| Sha256::digest(&i.encoded_ir).to_vec())
            .unwrap_or_else(|| Sha256::digest(b"no-invariant").to_vec());
        let r = RegisteredPatent {
            id: node.id, title: node.title.clone(),
            registered_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
            registrant: registrant.into(), commitment: commitment.clone(),
            invariant_hash: ih, license: node.license.clone(),
        };
        self.patents.insert(node.id, r.clone()); r
    }
    pub fn verify_commitment(&self, node: &PatentNode) -> bool {
        let data = serde_json::to_vec(node).unwrap_or_default();
        self.patents.get(&node.id).map(|r| Sha256::digest(&data).to_vec() == r.commitment).unwrap_or(false)
    }
    pub fn get(&self, id: &Uuid) -> Option<&RegisteredPatent> { self.patents.get(id) }
    pub fn len(&self) -> usize { self.patents.len() }
}
impl Default for PatentRegistry { fn default() -> Self { Self::new() } }

// RoyaltySplitter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentEntry {
    pub recipient_node_id: Uuid, pub recipient: String,
    pub amount: u64, pub claimed: bool,
}
pub struct RoyaltySplitter {
    invariants: HashMap<Uuid, RoyaltyInvariant>,
    pending: HashMap<(Uuid, Uuid), PaymentEntry>,
    balance: u64,
}
impl RoyaltySplitter {
    pub fn new() -> Self { Self { invariants: HashMap::new(), pending: HashMap::new(), balance: 0 } }
    pub fn register_invariant(&mut self, inv: RoyaltyInvariant) { self.invariants.insert(inv.patent_node_id, inv); }
    pub fn distribute(&mut self, nid: &Uuid, amount: u64, recipients: &HashMap<Uuid, String>) -> Vec<PaymentEntry> {
        let mut entries = vec![];
        if let Some(inv) = self.invariants.get(nid) {
            for share in &inv.shares {
                let amt = (amount as f64 * share.weight) as u64;
                let addr = recipients.get(&share.node_id).cloned().unwrap_or_else(|| "0xUnknown".into());
                let e = PaymentEntry { recipient_node_id: share.node_id, recipient: addr, amount: amt, claimed: false };
                let pid = Uuid::new_v4();
                self.pending.insert((*nid, pid), e.clone());
                entries.push(e);
            }
        }
        self.balance += amount; entries
    }
    pub fn claim(&mut self, nid: &Uuid, pid: &Uuid) -> Option<PaymentEntry> {
        if let Some(e) = self.pending.get_mut(&(*nid, *pid)) {
            if !e.claimed && self.balance >= e.amount { e.claimed = true; self.balance -= e.amount; return Some(e.clone()); }
        }
        None
    }
    pub fn balance(&self) -> u64 { self.balance }
}
impl Default for RoyaltySplitter { fn default() -> Self { Self::new() } }

// VerificationOracle
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationRequest {
    pub edge_id: Uuid, pub original_node_id: Uuid, pub derivative_node_id: Uuid,
    pub resolved: bool, pub result: Option<VerificationResult>,
}
pub struct VerificationOracle { requests: Vec<VerificationRequest>, events: Vec<VerificationResult> }
impl VerificationOracle {
    pub fn new() -> Self { Self { requests: vec![], events: vec![] } }
    pub fn submit_request(&mut self, eid: Uuid, orig: Uuid, deriv: Uuid) {
        self.requests.push(VerificationRequest { edge_id: eid, original_node_id: orig, derivative_node_id: deriv, resolved: false, result: None });
    }
    pub fn submit_result(&mut self, eid: &Uuid, result: VerificationResult, proof: &[u8]) -> Result<VerificationResult, String> {
        let req = self.requests.iter_mut().find(|r| r.edge_id == *eid && !r.resolved)
            .ok_or("no pending request")?;
        if proof.len() < 32 { return Err("proof too short".into()); }
        req.resolved = true; req.result = Some(result.clone());
        self.events.push(result.clone()); Ok(result)
    }
}
impl Default for VerificationOracle { fn default() -> Self { Self::new() } }

// ExecutionGate
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GateStatus { Locked, VerifiedNotPaid, PaidNotVerified, Released, Blocked }
pub struct ExecutionGate { requests: Vec<(Uuid, String, GateStatus)> }
impl ExecutionGate {
    pub fn new() -> Self { Self { requests: vec![] } }
    pub fn request_release(&mut self, pid: Uuid, requester: &str) {
        self.requests.push((pid, requester.into(), GateStatus::Locked));
    }
    pub fn submit_verification(&mut self, pid: &Uuid, requester: &str, sat: SatResult) -> Result<(), String> {
        let (_, _, ref mut status) = self.requests.iter_mut()
            .find(|(p, r, _)| p == pid && r == requester).ok_or("not found")?;
        match (&*status, sat) {
            (GateStatus::Locked, SatResult::Unsat) => *status = GateStatus::VerifiedNotPaid,
            (GateStatus::Locked, SatResult::Sat) => *status = GateStatus::Blocked,
            (GateStatus::PaidNotVerified, SatResult::Unsat) => *status = GateStatus::Released,
            (GateStatus::PaidNotVerified, SatResult::Sat) => *status = GateStatus::Blocked,
            _ => return Err("invalid transition".into()),
        }
        Ok(())
    }
    pub fn confirm_payment(&mut self, pid: &Uuid, requester: &str) -> Result<(), String> {
        let (_, _, ref mut status) = self.requests.iter_mut()
            .find(|(p, r, _)| p == pid && r == requester).ok_or("not found")?;
        match status {
            GateStatus::Locked => *status = GateStatus::PaidNotVerified,
            GateStatus::VerifiedNotPaid => *status = GateStatus::Released,
            _ => return Err("can't pay now".into()),
        }
        Ok(())
    }
    pub fn is_released(&self, pid: &Uuid, requester: &str) -> bool {
        self.requests.iter().any(|(p, r, s)| p == pid && r == requester && *s == GateStatus::Released)
    }
}
impl Default for ExecutionGate { fn default() -> Self { Self::new() } }

// =============================================================================
// SECTION 13: P2P NETWORK LAYER  (p2p.rs)
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    AnnounceNode(PatentNode), AnnounceEdge(Edge),
    SyncRequest { requester_id: Uuid, last_known_hash: Vec<u8> },
    SyncResponse { nodes: Vec<PatentNode>, edges: Vec<Edge>, merkle_root: Vec<u8> },
    VerificationRequest { edge_id: Uuid, original_id: Uuid, derivative_id: Uuid, formula_hash: Vec<u8> },
    Ping { node_id: Uuid, timestamp: i64 },
}

pub struct NetworkNode {
    pub id: Uuid, pub dag: PatentDag, peers: HashMap<Uuid, String>,
}
impl NetworkNode {
    pub fn new(id: Uuid) -> Self { Self { id, dag: PatentDag::new(), peers: HashMap::new() } }
    pub fn add_peer(&mut self, pid: Uuid, addr: &str) { self.peers.insert(pid, addr.into()); }
    pub fn receive(&mut self, _from: Uuid, msg: NetworkMessage) -> Option<NetworkMessage> {
        match &msg {
            NetworkMessage::AnnounceNode(n) => { let _ = self.dag.add_node(n.clone()); Some(msg) }
            NetworkMessage::AnnounceEdge(e) => { let _ = self.dag.add_edge(&e.parent_id, &e.child_id, e.clone()); Some(msg) }
            NetworkMessage::SyncRequest { .. } => {
                let state = self.dag.export_state();
                Some(NetworkMessage::SyncResponse {
                    nodes: state.nodes.into_values().collect(),
                    edges: state.edges, merkle_root: self.dag.last_event_hash().to_vec(),
                })
            }
            _ => None,
        }
    }
}

// =============================================================================
// SECTION 14: STORAGE LAYER  (storage.rs)
// =============================================================================

pub struct StorageNode { blueprints: HashMap<String, Vec<u8>> }
impl StorageNode {
    pub fn new() -> Self { Self { blueprints: HashMap::new() } }
    pub fn store(&mut self, data: &[u8]) -> String {
        let cid = format!("Qm{}", hex::encode(Sha256::digest(data)));
        self.blueprints.insert(cid.clone(), data.to_vec()); cid
    }
    pub fn get(&self, cid: &str) -> Option<&[u8]> { self.blueprints.get(cid).map(|v| v.as_slice()) }
    pub fn verify(&self, cid: &str, data: &[u8]) -> bool {
        let computed = format!("Qm{}", hex::encode(Sha256::digest(data)));
        computed == cid && self.blueprints.contains_key(cid)
    }
}

// =============================================================================
// TESTS  (all inline)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: Uuid) -> PatentNode {
        PatentNode {
            id, title: "Test".into(), patent_type: PatentType::Software,
            authors: vec!["Alice".into()], created_at_unix: 0, content_hash: vec![],
            invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![],
        }
    }
    fn make_edge(p: Uuid, c: Uuid, w: f64) -> Edge {
        Edge { id: Uuid::new_v4(), parent_id: p, child_id: c, kind: EdgeKind::Derivative,
            delta: None, contribution_weight: w, signature: vec![] }
    }

    #[test] fn test_dag_add_node() {
        let mut dag = PatentDag::new();
        dag.add_node(make_node(Uuid::new_v4())).unwrap();
        assert_eq!(dag.node_count(), 1);
    }

    #[test] fn test_dag_cycle_prevention() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4(); let b = Uuid::new_v4(); let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap(); dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.5)).unwrap();
        assert!(dag.add_edge(&c, &a, make_edge(c, a, 0.5)).is_err());
    }

    #[test] fn test_dag_topological() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4(); let b = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 1.0)).unwrap();
        let order = dag.topological_order_indices().unwrap();
        assert!(order.iter().position(|&x| x == a) < order.iter().position(|&x| x == b));
    }

    #[test] fn test_dag_lca() {
        let mut dag = PatentDag::new();
        let r = Uuid::new_v4(); let l = Uuid::new_v4(); let ri = Uuid::new_v4();
        dag.add_node(make_node(r)).unwrap(); dag.add_node(make_node(l)).unwrap(); dag.add_node(make_node(ri)).unwrap();
        dag.add_edge(&r, &l, make_edge(r, l, 0.5)).unwrap();
        dag.add_edge(&r, &ri, make_edge(r, ri, 0.5)).unwrap();
        let lcas = dag.lowest_common_ancestors(&l, &ri).unwrap();
        assert_eq!(lcas.len(), 1); assert_eq!(lcas[0].id, r);
    }

    #[test] fn test_dag_distance() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4(); let b = Uuid::new_v4(); let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap(); dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.5)).unwrap();
        assert_eq!(dag.distance(&a, &c).unwrap(), Some(2));
    }

    #[test] fn test_delta_build() {
        let o = InventionIr { kind: IrKind::Software(SsaTrace {
            entry_block: "e".into(), blocks: HashMap::new(), inputs: vec![], outputs: vec![],
        }), preconditions: vec!["(= x 0)".into()], postconditions: vec!["(= y 1)".into()], invariants: vec![] };
        let d = InventionIr { kind: IrKind::Software(SsaTrace {
            entry_block: "e".into(), blocks: HashMap::new(), inputs: vec![], outputs: vec![],
        }), preconditions: vec!["(= x 0)".into()], postconditions: vec!["(= y 2)".into()], invariants: vec![] };
        let delta = build_delta(Uuid::new_v4(), Uuid::new_v4(), &o, &d, &[]).unwrap();
        assert!(!delta.formula_smt_lib2.is_empty());
    }

    #[test] fn test_builtin_solver() {
        let solver = BuiltInSolver;
        let config = SolverConfig::default();
        let r = solver.check_sat("(assert (= x 5))\n(assert (= x 10))\n(check-sat)\n", &config).unwrap();
        assert_eq!(r.sat, SatResult::Unsat);
    }

    #[test] fn test_royalty_verifier() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4(); let b = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 1.0)).unwrap();
        let v = RoyaltyVerifier::default();
        assert!(v.verify_all(&dag).is_ok());
    }

    #[test] fn test_royalty_shares() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4(); let b = Uuid::new_v4(); let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap(); dag.add_node(make_node(b)).unwrap(); dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.5)).unwrap();
        let shares = compute_royalty_shares(&dag, &c).unwrap();
        assert!((shares[&a] - 0.25).abs() < 1e-9);
        assert!((shares[&b] - 0.5).abs() < 1e-9);
    }

    #[test] fn test_taint_lattice() {
        let a = Uuid::new_v4(); let b = Uuid::new_v4();
        assert_eq!(TaintValue::Unlicensed.join(&TaintValue::Licensed(a)), TaintValue::Licensed(a));
        assert_eq!(TaintValue::Licensed(a).join(&TaintValue::Licensed(b)),
            TaintValue::Multiple(HashSet::from([a, b])));
        assert_eq!(TaintValue::Licensed(a).meet(&TaintValue::Licensed(b)), TaintValue::Unlicensed);
    }

    #[test] fn test_he_roundtrip() {
        let engine = HeEngine::new(HeConfig::default());
        let kp = engine.generate_keypair();
        let ct = engine.encrypt(0.42, &kp.public_key);
        let dec = engine.decrypt(&ct, &kp.private_key).unwrap();
        assert!((0.42 - dec).abs() < 0.001);
    }

    #[test] fn test_contract_registry() {
        let mut reg = PatentRegistry::new();
        let node = make_node(Uuid::new_v4());
        reg.register(&node, "0xAlice");
        assert!(reg.verify_commitment(&node));
        let mut tampered = node.clone(); tampered.title = "Hacked".into();
        assert!(!reg.verify_commitment(&tampered));
    }

    #[test] fn test_execution_gate() {
        let mut gate = ExecutionGate::new();
        let pid = Uuid::new_v4();
        gate.request_release(pid, "0xFactory");
        gate.submit_verification(&pid, "0xFactory", SatResult::Unsat).unwrap();
        gate.confirm_payment(&pid, "0xFactory").unwrap();
        assert!(gate.is_released(&pid, "0xFactory"));
    }

    #[test] fn test_p2p_announce() {
        let mut alice = NetworkNode::new(Uuid::new_v4());
        let mut bob = NetworkNode::new(Uuid::new_v4());
        alice.add_peer(bob.id, "127.0.0.1:9001");
        let node = make_node(Uuid::new_v4());
        let msg = NetworkMessage::AnnounceNode(node.clone());
        let resp = bob.receive(alice.id, msg).unwrap();
        assert!(bob.dag.contains_node(&node.id));
        assert!(matches!(resp, NetworkMessage::AnnounceNode(_)));
    }

    #[test] fn test_storage() {
        let mut s = StorageNode::new();
        let data = b"hello world";
        let cid = s.store(data);
        assert_eq!(s.get(&cid), Some(&data[..]));
        assert!(s.verify(&cid, data));
        assert!(!s.verify(&cid, b"tampered"));
    }

    #[test] fn test_spec_synthesize() {
        let ir = InventionIr {
            kind: IrKind::Software(SsaTrace {
                entry_block: "e".into(), blocks: HashMap::new(), inputs: vec![("x".into(), IrType::Int(32))], outputs: vec![],
            }),
            preconditions: vec!["x > 0".into()], postconditions: vec!["y == x + 1".into()], invariants: vec![],
        };
        let spec = SpecSynthesizer::synthesize(&ir, Uuid::new_v4());
        assert!(!spec.spec_bytes.is_empty());
    }
}
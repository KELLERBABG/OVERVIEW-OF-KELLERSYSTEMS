//! # Symbolic Patent-Logic Intermediate Representation (IR)
//!
//! Defines an intermediate representation for translating invention artifacts
//! into formal mathematical constraints. Supports three domains:
//! - **SSA trace** for software (static single assignment form)
//! - **Molecular graph** for chemical structures
//! - **Geometric constraint system** for mechanical blueprints
//!
//! The IR is the bridge between invention artifacts and SMT-LIB2 formulas
//! used in the verification engine (Phase 4).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A typed value in the IR.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrValue {
    /// Integer constant (unbounded, bit-width specified by context).
    Int(i64),
    /// Floating-point constant.
    Float(f64),
    /// Boolean constant.
    Bool(bool),
    /// Bit-vector constant (byte array).
    BitVector(Vec<u8>),
    /// Symbolic variable (named).
    Variable(String),
    /// Compound term (e.g., function application).
    Term {
        function: String,
        args: Vec<Box<IrValue>>,
    },
}

/// An SSA-style operation (for software patents).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SsaOp {
    /// Binary arithmetic.
    Add,
    Sub,
    Mul,
    Div,
    /// Bitwise.
    And,
    Or,
    Xor,
    Shl,
    Lshr,
    /// Comparison (returns Bool).
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// Memory operations.
    Load { address: Box<IrExpr> },
    Store { address: Box<IrExpr>, value: Box<IrExpr> },
    /// Control-flow phi node.
    Phi { incoming: Vec<IrExpr> },
    /// User-defined (for domain-specific operations).
    Custom(String),
}

/// A single SSA instruction (for software patents).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SsaInstruction {
    /// The result variable name (None for void operations like Store).
    pub result: Option<String>,
    /// The operation.
    pub op: SsaOp,
    /// The operands.
    pub operands: Vec<IrExpr>,
}

/// A basic block in an SSA trace.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BasicBlock {
    pub label: String,
    pub instructions: Vec<SsaInstruction>,
    pub terminator: Terminator,
}

/// Block terminator (control flow).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Terminator {
    /// Unconditional branch.
    Branch(String),
    /// Conditional branch.
    BranchCond { condition: IrExpr, true_target: String, false_target: String },
    /// Return with optional value.
    Return(Option<IrExpr>),
}

/// A full SSA trace — a control-flow graph of basic blocks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SsaTrace {
    pub entry_block: String,
    pub blocks: HashMap<String, BasicBlock>,
    /// Function/module-level input parameters (pre-conditions).
    pub inputs: Vec<(String, IrType)>,
    /// Function/module-level output (post-condition variable).
    pub outputs: Vec<(String, IrType)>,
}

/// IR type for SSA traces.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrType {
    Int(u32),       // bit width
    Float(u32),     // bit width (32 or 64)
    Bool,
    BitVector(u32), // bit width
    Array(Box<IrType>, Box<IrType>),
    Struct(Vec<(String, IrType)>),
}

/// An IR expression.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrExpr {
    Value(IrValue),
    /// Variable reference.
    Ref(String),
    /// Unary operation.
    Unary { op: UnaryOp, expr: Box<IrExpr> },
    /// Binary operation.
    Binary { op: BinaryOp, left: Box<IrExpr>, right: Box<IrExpr> },
    /// If-then-else (conditional expression).
    Ite { cond: Box<IrExpr>, then_expr: Box<IrExpr>, else_expr: Box<IrExpr> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Mod,
    And, Or, Xor, Shl, Lshr,
    Eq, Ne, Lt, Le, Gt, Ge,
}

// ─── Molecular IR (Chemical) ────────────────────────────────────────────────

/// A chemical bond type.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BondType {
    Single,
    Double,
    Triple,
    Aromatic,
    Hydrogen,
}

/// A chemical element / atom in a molecular graph.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Atom {
    pub symbol: String,        // e.g. "C", "H", "O", "N"
    pub formal_charge: i32,
    pub isotope: Option<u32>,
    pub stereochemistry: Option<String>,
}

/// A bond between two atoms in a molecular graph.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bond {
    pub atom_a: usize,  // index into atoms array
    pub atom_b: usize,
    pub bond_type: BondType,
}

/// A molecular graph representation for chemical patents.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MolecularGraph {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    /// Additional constraints: e.g., chirality, pH range, temperature range.
    pub constraints: Vec<ChemicalConstraint>,
}

/// A chemical constraint on the molecular graph.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ChemicalConstraint {
    Chirality { atom_index: usize, configuration: String },
    BondAngle { atom_a: usize, atom_b: usize, atom_c: usize, min_degrees: f64, max_degrees: f64 },
    Distance { atom_a: usize, atom_b: usize, min_angstrom: f64, max_angstrom: f64 },
    ChargeRange { atom_index: usize, min: i32, max: i32 },
    ReactionInvariant { description: String },
}

// ─── Geometric IR (Mechanical / CAD) ─────────────────────────────────────────

/// A 3D point or vector.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A geometric constraint type for CAD models.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GeometricConstraint {
    Coincident { entities: Vec<usize> },
    Parallel { entities: Vec<usize> },
    Perpendicular { entities: Vec<usize> },
    Concentric { entities: Vec<usize> },
    Distance { entity_a: usize, entity_b: usize, min: f64, max: f64 },
    Angle { entity_a: usize, entity_b: usize, min_degrees: f64, max_degrees: f64 },
    Tangent { entities: Vec<usize> },
    Symmetry { entity_a: usize, entity_b: usize, about: usize },
    Fixed { entity: usize, position: Point3 },
    Tolerance { entity: usize, tolerance: f64 },
}

/// A geometric entity (point, line, plane, circle, etc.).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GeometricEntity {
    Point(Point3),
    Line { point: Point3, direction: Point3 },
    Plane { point: Point3, normal: Point3 },
    Circle { center: Point3, normal: Point3, radius: f64 },
    Cylinder { axis_point: Point3, axis_direction: Point3, radius: f64 },
    Sphere { center: Point3, radius: f64 },
}

/// A full mechanical/CAD model represented as a constraint system.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MechanicalModel {
    pub entities: Vec<GeometricEntity>,
    pub constraints: Vec<GeometricConstraint>,
    /// Tolerances and material properties.
    pub metadata: HashMap<String, String>,
}

// ─── Top-level IR ────────────────────────────────────────────────────────────

/// The kind of invention artifact being represented.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IrKind {
    Software(SsaTrace),
    Chemical(MolecularGraph),
    Mechanical(MechanicalModel),
}

/// The complete intermediate representation of an invention.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InventionIr {
    pub kind: IrKind,
    /// Pre-conditions (inputs must satisfy these).
    pub preconditions: Vec<String>,
    /// Post-conditions (outputs must satisfy these).
    pub postconditions: Vec<String>,
    /// Loop/structural invariants (properties that hold at all points).
    pub invariants: Vec<String>,
}

// ─── Invariant Extraction ────────────────────────────────────────────────────

/// Extracts formal invariants from a given IR representation.
/// This is the starting point for symbolic patent-logic matching.
pub fn extract_invariants(ir: &InventionIr) -> Vec<String> {
    let mut invariants = Vec::new();
    invariants.extend(ir.preconditions.iter().cloned());
    invariants.extend(ir.postconditions.iter().cloned());
    invariants.extend(ir.invariants.iter().cloned());
    invariants
}

/// Generate a human-readable description of the IR kind.
pub fn describe_ir(ir: &InventionIr) -> String {
    match &ir.kind {
        IrKind::Software(trace) => {
            format!(
                "Software IR: {} blocks, {} inputs, {} outputs",
                trace.blocks.len(),
                trace.inputs.len(),
                trace.outputs.len()
            )
        }
        IrKind::Chemical(mol) => {
            format!(
                "Chemical IR: {} atoms, {} bonds",
                mol.atoms.len(),
                mol.bonds.len()
            )
        }
        IrKind::Mechanical(model) => {
            format!(
                "Mechanical IR: {} entities, {} constraints",
                model.entities.len(),
                model.constraints.len()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_ir_creation() {
        let mut blocks = HashMap::new();
        blocks.insert(
            "entry".to_string(),
            BasicBlock {
                label: "entry".into(),
                instructions: vec![
                    SsaInstruction {
                        result: Some("x".into()),
                        op: SsaOp::Add,
                        operands: vec![
                            IrExpr::Value(IrValue::Int(1)),
                            IrExpr::Value(IrValue::Int(2)),
                        ],
                    },
                ],
                terminator: Terminator::Return(Some(IrExpr::Ref("x".into()))),
            },
        );
        let trace = SsaTrace {
            entry_block: "entry".into(),
            blocks,
            inputs: vec![("a".into(), IrType::Int(32))],
            outputs: vec![("result".into(), IrType::Int(32))],
        };
        let ir = InventionIr {
            kind: IrKind::Software(trace),
            preconditions: vec!["a > 0".into()],
            postconditions: vec!["result == a + 3".into()],
            invariants: vec![],
        };
        let extracted = extract_invariants(&ir);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&"a > 0".to_string()));
        assert!(extracted.contains(&"result == a + 3".to_string()));
    }

    #[test]
    fn test_chemical_ir_creation() {
        let mol = MolecularGraph {
            atoms: vec![
                Atom { symbol: "C".into(), formal_charge: 0, isotope: None, stereochemistry: None },
                Atom { symbol: "O".into(), formal_charge: 0, isotope: None, stereochemistry: None },
            ],
            bonds: vec![
                Bond { atom_a: 0, atom_b: 1, bond_type: BondType::Double },
            ],
            constraints: vec![
                ChemicalConstraint::Distance { atom_a: 0, atom_b: 1, min_angstrom: 1.1, max_angstrom: 1.3 },
            ],
        };
        let ir = InventionIr {
            kind: IrKind::Chemical(mol),
            preconditions: vec![],
            postconditions: vec![],
            invariants: vec!["bond_length(C,O) ∈ [1.1, 1.3]".into()],
        };
        let extracted = extract_invariants(&ir);
        assert_eq!(extracted.len(), 1);
    }

    #[test]
    fn test_mechanical_ir_creation() {
        let model = MechanicalModel {
            entities: vec![
                GeometricEntity::Point(Point3 { x: 0.0, y: 0.0, z: 0.0 }),
                GeometricEntity::Point(Point3 { x: 1.0, y: 0.0, z: 0.0 }),
            ],
            constraints: vec![
                GeometricConstraint::Distance { entity_a: 0, entity_b: 1, min: 0.9, max: 1.1 },
            ],
            metadata: HashMap::new(),
        };
        let ir = InventionIr {
            kind: IrKind::Mechanical(model),
            preconditions: vec![],
            postconditions: vec![],
            invariants: vec!["distance(p0, p1) ∈ [0.9, 1.1]".into()],
        };
        let extracted = extract_invariants(&ir);
        assert_eq!(extracted.len(), 1);
    }

    #[test]
    fn test_describe_ir() {
        let trace = SsaTrace {
            entry_block: "entry".into(),
            blocks: HashMap::new(),
            inputs: vec![],
            outputs: vec![],
        };
        let ir = InventionIr {
            kind: IrKind::Software(trace),
            preconditions: vec![],
            postconditions: vec![],
            invariants: vec![],
        };
        let desc = describe_ir(&ir);
        assert!(desc.starts_with("Software IR:"));
    }
}
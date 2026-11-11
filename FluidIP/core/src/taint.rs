//! # Taint-Coded Derivative Tracking
//!
//! Continuously tracks data and logic flow from the original base invention
//! to complex derivatives, ensuring no code or formula fragment can be used
//! without proper licensing.
//!
//! The taint system uses a lattice of labels:
//! - `Unlicensed` — no license (bottom of lattice)
//! - `Licensed(id)` — specific license origin
//! - `Multiple(ids)` — composition of multiple licenses (join)
//!
//! Static taint analysis propagates labels through IR operations. Dynamic
//! taint verification checks at execution time that no unlicensed fragment
//! is used.

use crate::error::CoreError;
use crate::ir::*;
use crate::models::{License, TaintLabel, TaintPropagationRule, TaintLatticeJoin};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// ─── Taint Lattice ───────────────────────────────────────────────────────────

/// An element of the taint lattice.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TaintValue {
    /// No license information (bottom of lattice).
    Unlicensed,
    /// A single license origin.
    Licensed(Uuid),
    /// Multiple license origins (join of multiple labels).
    Multiple(HashSet<Uuid>),
}

impl TaintValue {
    /// The bottom of the lattice — no restrictions.
    pub fn bottom() -> Self {
        TaintValue::Unlicensed
    }

    /// The top of the lattice — all licenses (maximally restrictive).
    pub fn top() -> Self {
        TaintValue::Multiple(HashSet::new()) // empty set = all origins possible
    }

    /// Join (least upper bound) of two taint values.
    /// The result is the most restrictive label combination.
    pub fn join(&self, other: &Self) -> Self {
        match (self, other) {
            (TaintValue::Unlicensed, x) => x.clone(),
            (x, TaintValue::Unlicensed) => x.clone(),
            (TaintValue::Licensed(a), TaintValue::Licensed(b)) => {
                if a == b {
                    TaintValue::Licensed(*a)
                } else {
                    let mut set = HashSet::new();
                    set.insert(*a);
                    set.insert(*b);
                    TaintValue::Multiple(set)
                }
            }
            (TaintValue::Licensed(a), TaintValue::Multiple(bs)) => {
                let mut set = bs.clone();
                set.insert(*a);
                TaintValue::Multiple(set)
            }
            (TaintValue::Multiple(as_), TaintValue::Licensed(b)) => {
                let mut set = as_.clone();
                set.insert(*b);
                TaintValue::Multiple(set)
            }
            (TaintValue::Multiple(as_), TaintValue::Multiple(bs)) => {
                let mut set = as_.clone();
                set.extend(bs);
                TaintValue::Multiple(set)
            }
        }
    }

    /// Meet (greatest lower bound) of two taint values.
    /// The result is the least restrictive common label.
    pub fn meet(&self, other: &Self) -> Self {
        match (self, other) {
            (TaintValue::Unlicensed, _) => TaintValue::Unlicensed,
            (_, TaintValue::Unlicensed) => TaintValue::Unlicensed,
            (TaintValue::Licensed(a), TaintValue::Licensed(b)) => {
                if a == b {
                    TaintValue::Licensed(*a)
                } else {
                    TaintValue::Unlicensed // incompatible → bottom
                }
            }
            (TaintValue::Licensed(a), TaintValue::Multiple(bs)) => {
                if bs.contains(a) {
                    TaintValue::Licensed(*a)
                } else {
                    TaintValue::Unlicensed
                }
            }
            (TaintValue::Multiple(as_), TaintValue::Licensed(b)) => {
                if as_.contains(b) {
                    TaintValue::Licensed(*b)
                } else {
                    TaintValue::Unlicensed
                }
            }
            (TaintValue::Multiple(as_), TaintValue::Multiple(bs)) => {
                let intersection: HashSet<Uuid> = as_.intersection(bs).copied().collect();
                if intersection.is_empty() {
                    TaintValue::Unlicensed
                } else if intersection.len() == 1 {
                    TaintValue::Licensed(*intersection.iter().next().unwrap())
                } else {
                    TaintValue::Multiple(intersection)
                }
            }
        }
    }
}

// ─── Taint Label on IR Values ───────────────────────────────────────────────

/// A taint-annotated IR expression value.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaintedExpr {
    /// The IR expression.
    pub expr: IrExpr,
    /// The taint label attached to this expression.
    pub taint: TaintValue,
}

// ─── Static Taint Analyzer ──────────────────────────────────────────────────

/// A static taint analyzer that propagates labels through an SSA trace.
pub struct TaintAnalyzer {
    /// Propagation rules for different operators.
    rules: HashMap<String, TaintLatticeJoin>,
}

impl TaintAnalyzer {
    /// Create a new taint analyzer with default propagation rules.
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        // Arithmetic: union of operand taints.
        rules.insert("Add".to_string(), TaintLatticeJoin::Join);
        rules.insert("Sub".to_string(), TaintLatticeJoin::Join);
        rules.insert("Mul".to_string(), TaintLatticeJoin::Join);
        rules.insert("Div".to_string(), TaintLatticeJoin::Join);
        // Bitwise: union of operand taints.
        rules.insert("And".to_string(), TaintLatticeJoin::Join);
        rules.insert("Or".to_string(), TaintLatticeJoin::Join);
        rules.insert("Xor".to_string(), TaintLatticeJoin::Join);
        // Comparison: more restrictive (all operands must be compared).
        rules.insert("Eq".to_string(), TaintLatticeJoin::Meet);
        rules.insert("Ne".to_string(), TaintLatticeJoin::Meet);
        // Memory: union of address and value taints.
        rules.insert("Load".to_string(), TaintLatticeJoin::Join);
        rules.insert("Store".to_string(), TaintLatticeJoin::Join);
        // Phi: least upper bound of all incoming values.
        rules.insert("Phi".to_string(), TaintLatticeJoin::Join);
        Self { rules }
    }

    /// Add a custom taint propagation rule.
    pub fn add_rule(&mut self, operator: &str, semantic: TaintLatticeJoin) {
        self.rules.insert(operator.to_string(), semantic);
    }

    /// Analyze an SSA trace and annotate every expression with taint.
    pub fn analyze_trace<'a>(
        &self,
        trace: &'a SsaTrace,
        initial_taints: &HashMap<String, TaintValue>,
    ) -> Result<HashMap<String, TaintValue>, CoreError> {
        let mut env: HashMap<String, TaintValue> = initial_taints.clone();

        // Process blocks in topological order (entry first).
        let blocks = &trace.blocks;
        let mut block_order = vec![];
        let mut visited = HashSet::new();
        let mut stack = vec![trace.entry_block.clone()];
        while let Some(label) = stack.pop() {
            if visited.insert(label.clone()) {
                block_order.push(label.clone());
                if let Some(block) = blocks.get(&label) {
                    match &block.terminator {
                        Terminator::Branch(target) => stack.push(target.clone()),
                        Terminator::BranchCond { true_target, false_target, .. } => {
                            stack.push(true_target.clone());
                            stack.push(false_target.clone());
                        }
                        Terminator::Return(_) => {}
                    }
                }
            }
        }

        // Annotate each block's instructions.
        for label in &block_order {
            if let Some(block) = blocks.get(label) {
                for inst in &block.instructions {
                    let result = inst.result.as_ref().ok_or_else(|| {
                        CoreError::TaintPropagationError(
                            "instruction without result variable".to_string(),
                        )
                    })?;

                    let op_name = format!("{:?}", inst.op);
                    let rule = self.rules.get(&op_name).unwrap_or(&TaintLatticeJoin::Join);

                    let taint = self.compute_operand_taint(&inst.operands, rule, &env);
                    env.insert(result.clone(), taint);
                }
            }
        }

        Ok(env)
    }

    /// Compute the taint for a set of operands based on the rule.
    fn compute_operand_taint(
        &self,
        operands: &[IrExpr],
        rule: &TaintLatticeJoin,
        env: &HashMap<String, TaintValue>,
    ) -> TaintValue {
        let taints: Vec<TaintValue> = operands
            .iter()
            .map(|op| self.taint_of_expr(op, env))
            .collect();

        if taints.is_empty() {
            return TaintValue::Unlicensed;
        }

        match rule {
            TaintLatticeJoin::Join | TaintLatticeJoin::LeastUpperBound => {
                taints
                    .iter()
                    .fold(TaintValue::Unlicensed, |acc, t| acc.join(t))
            }
            TaintLatticeJoin::Meet => {
                let first = taints[0].clone();
                taints.iter().skip(1).fold(first, |acc, t| acc.meet(t))
            }
        }
    }

    /// Get the taint of an expression from the environment.
    fn taint_of_expr(
        &self,
        expr: &IrExpr,
        env: &HashMap<String, TaintValue>,
    ) -> TaintValue {
        match expr {
            IrExpr::Value(_) => TaintValue::Unlicensed, // constants have no taint
            IrExpr::Ref(name) => env.get(name).cloned().unwrap_or(TaintValue::Unlicensed),
            IrExpr::Unary { expr: e, .. } => self.taint_of_expr(e, env),
            IrExpr::Binary { left, right, .. } => {
                self.taint_of_expr(left, env).join(&self.taint_of_expr(right, env))
            }
            IrExpr::Ite { cond, then_expr, else_expr, .. } => {
                self.taint_of_expr(cond, env)
                    .join(&self.taint_of_expr(then_expr, env))
                    .join(&self.taint_of_expr(else_expr, env))
            }
        }
    }
}

impl Default for TaintAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Taint Specification ─────────────────────────────────────────────────────

/// A specification of what taint annotations are expected.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaintSpec {
    /// Variables that must have a specific taint value.
    pub required_taints: HashMap<String, TaintValue>,
    /// Variables that must not be unlicensed.
    pub must_be_licensed: Vec<String>,
}

impl TaintSpec {
    /// Verify that a taint environment satisfies this specification.
    pub fn verify(&self, env: &HashMap<String, TaintValue>) -> Result<(), CoreError> {
        for (var, required) in &self.required_taints {
            let actual = env.get(var).unwrap_or(&TaintValue::Unlicensed);
            // Check that actual is at least as restrictive as required.
            // If actual meets required = actual, then actual >= required in the lattice.
            let meets = actual.meet(required);
            if meets != *actual {
                return Err(CoreError::TaintPropagationError(format!(
                    "variable '{var}': expected taint {required:?}, got {actual:?}"
                )));
            }
        }
        for var in &self.must_be_licensed {
            let actual = env.get(var).unwrap_or(&TaintValue::Unlicensed);
            if *actual == TaintValue::Unlicensed {
                return Err(CoreError::UnlicensedFragment(var.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;
    use std::collections::HashMap;

    #[test]
    fn test_taint_lattice_join() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        assert_eq!(
            TaintValue::Unlicensed.join(&TaintValue::Licensed(a)),
            TaintValue::Licensed(a)
        );
        assert_eq!(
            TaintValue::Licensed(a).join(&TaintValue::Licensed(b)),
            TaintValue::Multiple(HashSet::from([a, b]))
        );
        assert_eq!(
            TaintValue::Licensed(a).join(&TaintValue::Licensed(a)),
            TaintValue::Licensed(a)
        );
    }

    #[test]
    fn test_taint_lattice_meet() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        assert_eq!(
            TaintValue::Unlicensed.meet(&TaintValue::Licensed(a)),
            TaintValue::Unlicensed
        );
        assert_eq!(
            TaintValue::Licensed(a).meet(&TaintValue::Licensed(a)),
            TaintValue::Licensed(a)
        );
        assert_eq!(
            TaintValue::Licensed(a).meet(&TaintValue::Licensed(b)),
            TaintValue::Unlicensed
        );
    }

    #[test]
    fn test_taint_analyzer_simple() {
        let analyzer = TaintAnalyzer::new();

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
                terminator: Terminator::Return(None),
            },
        );

        let trace = SsaTrace {
            entry_block: "entry".into(),
            blocks,
            inputs: vec![],
            outputs: vec![],
        };

        let mut initial_taints = HashMap::new();
        let license_id = Uuid::new_v4();
        initial_taints.insert("input_a".to_string(), TaintValue::Licensed(license_id));

        let env = analyzer.analyze_trace(&trace, &initial_taints).unwrap();
        // x is computed from constants (unlicensed), so it should be unlicensed.
        assert_eq!(env.get("x"), Some(&TaintValue::Unlicensed));
    }

    #[test]
    fn test_taint_propagation_through_arithmetic() {
        let analyzer = TaintAnalyzer::new();

        let mut blocks = HashMap::new();
        blocks.insert(
            "entry".to_string(),
            BasicBlock {
                label: "entry".into(),
                instructions: vec![
                    SsaInstruction {
                        result: Some("z".into()),
                        op: SsaOp::Add,
                        operands: vec![
                            IrExpr::Ref("a".into()),
                            IrExpr::Ref("b".into()),
                        ],
                    },
                ],
                terminator: Terminator::Return(None),
            },
        );

        let trace = SsaTrace {
            entry_block: "entry".into(),
            blocks,
            inputs: vec![("a".into(), IrType::Int(32)), ("b".into(), IrType::Int(32))],
            outputs: vec![("z".into(), IrType::Int(32))],
        };

        let license_a = Uuid::new_v4();
        let license_b = Uuid::new_v4();
        let mut initial_taints = HashMap::new();
        initial_taints.insert("a".to_string(), TaintValue::Licensed(license_a));
        initial_taints.insert("b".to_string(), TaintValue::Licensed(license_b));

        let env = analyzer.analyze_trace(&trace, &initial_taints).unwrap();
        // z should have both licenses (join = Multiple({a, b}))
        match &env["z"] {
            TaintValue::Multiple(ids) => {
                assert!(ids.contains(&license_a));
                assert!(ids.contains(&license_b));
            }
            _ => panic!("expected Multiple taint, got {:?}", env["z"]),
        }
    }

    #[test]
    fn test_taint_spec_verification() {
        let mut env = HashMap::new();
        let lic = Uuid::new_v4();
        env.insert("output".to_string(), TaintValue::Licensed(lic));

        let spec = TaintSpec {
            required_taints: HashMap::new(),
            must_be_licensed: vec!["output".to_string()],
        };
        assert!(spec.verify(&env).is_ok());

        // Unlicensed variable should fail.
        env.insert("output".to_string(), TaintValue::Unlicensed);
        let result = spec.verify(&env);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::UnlicensedFragment(_)));
    }
}
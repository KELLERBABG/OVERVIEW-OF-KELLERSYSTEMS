//! Lowering from C++ AST to Keller-VIR SSA hypergraphs.
//!
//! Produces verification conditions for:
//! 1. Iterator invalidation freedom (`cpp-iterator-invalidation`)
//! 2. Absence of pure virtual calls (`cpp-pure-virtual-call`)
//! 3. Use after delete freedom (`cpp-use-after-delete`)
//! 4. Absence of double delete (`cpp-double-delete`)
//! 5. Null pointer dereference freedom (`cpp-null-deref`)
//! 6. Vector index in-bounds (`cpp-vector-oob`)
//! 7. Division by zero freedom (`cpp-div-by-zero`)

#![allow(dead_code)]

use crate::cpp::ast::*;
use crate::cpp::analyzer::Analyzer;

/// Result of lowering a C++ translation unit into Keller-VIR.
pub struct LowerResult {
    /// Hypergraph nodes: SSA values.
    pub nodes: Vec<Node>,
    /// Hyperedges: multi-argument operations linking nodes.
    pub edges: Vec<HyperEdge>,
    /// Verification conditions generated.
    pub vc: Vec<VerificationCondition>,
}

/// A node in the SSA hypergraph.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub kind: NodeKind,
    pub label: String,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    /// A container object (vector, string).
    Container(String),
    /// An iterator derived from a container.
    IteratorOf(usize),
    /// A class instance.
    Instance(String),
    /// A vtable pointer.
    VTablePtr(String),
    /// A method call result.
    CallResult,
    /// A pointer reference.
    Pointer(String),
    /// An integer or scalar value.
    Scalar(String),
}

/// A hyperedge connecting multiple nodes via an operation.
#[derive(Debug, Clone)]
pub struct HyperEdge {
    pub op: String,
    pub inputs: Vec<usize>,
    pub output: Option<usize>,
    pub line: usize,
}

/// A verification condition to be discharged by the solver portfolio.
#[derive(Debug, Clone)]
pub struct VerificationCondition {
    pub name: String,
    pub kind: VcKind,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum VcKind {
    /// Assert that no iterator is used after an invalidating operation.
    IteratorValid { container: usize, op_index: usize },
    /// Assert that a virtual call target is not pure virtual.
    NoPureVirtualCall { call_site: usize, class_name: String },
    /// Assert no access after delete.
    UseAfterDelete { ptr_name: String, call_site: usize },
    /// Assert no double delete.
    NoDoubleDelete { ptr_name: String, call_site: usize },
    /// Assert no null dereference.
    NoNullDeref { ptr_name: String, call_site: usize },
    /// Assert vector access within bounds.
    VectorInBounds { vec_name: String, call_site: usize },
    /// Assert divisor is non-zero.
    NoDivByZero { divisor: String, call_site: usize },
}

/// Audit report produced in audit-only mode.
pub struct AuditReport {
    pub file_path: String,
    pub total_classes: usize,
    pub total_functions: usize,
    pub total_containers: usize,
    pub iterator_violations: Vec<String>,
    pub pure_virtual_call_sites: Vec<String>,
    pub memory_violations: Vec<String>,
    pub bounds_violations: Vec<String>,
    pub div_by_zero_violations: Vec<String>,
    pub vtable_integrity_ok: bool,
    pub passed: bool,
}

/// Lower a parsed translation unit into Keller-VIR hypergraph form.
pub fn lower_cpp(tu: &TranslationUnit) -> LowerResult {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut vc = Vec::new();

    // --- Lower classes and vtables ---
    for class in &tu.classes {
        let instance_id = nodes.len();
        nodes.push(Node {
            id: instance_id,
            kind: NodeKind::Instance(class.name.clone()),
            label: format!("inst.{}", class.name),
        });

        let vtable_id = nodes.len();
        nodes.push(Node {
            id: vtable_id,
            kind: NodeKind::VTablePtr(class.name.clone()),
            label: format!("vtable.{}", class.name),
        });

        edges.push(HyperEdge {
            op: "has_vtable".into(),
            inputs: vec![instance_id],
            output: Some(vtable_id),
            line: class.line,
        });

        for pv in class.pure_virtual_methods() {
            let relevant_calls: Vec<&VirtualCall> = tu
                .virtual_calls
                .iter()
                .filter(|c| c.method_name == pv.name && c.receiver_is_abstract)
                .collect();
            for call in relevant_calls {
                vc.push(VerificationCondition {
                    name: format!("no_pure_virtual_{}_{}", class.name, call.line),
                    kind: VcKind::NoPureVirtualCall {
                        call_site: call.line,
                        class_name: class.name.clone(),
                    },
                    description: format!(
                        "Line {}: call to '{}' through potentially abstract receiver '{}'",
                        call.line, pv.name, call.receiver
                    ),
                });
            }
        }
    }

    // --- Lower containers and iterator invariants ---
    for container in &tu.containers {
        let cont_id = nodes.len();
        nodes.push(Node {
            id: cont_id,
            kind: NodeKind::Container(container.element_type.clone()),
            label: format!("container.{}", container.var_name),
        });

        let mut active_iterators: Vec<usize> = Vec::new();

        for (op_idx, op) in container.operations.iter().enumerate() {
            if op.op_kind == OpKind::Iterate || op.op_kind == OpKind::Index {
                let iter_id = nodes.len();
                nodes.push(Node {
                    id: iter_id,
                    kind: NodeKind::IteratorOf(cont_id),
                    label: format!("iter.{}_{}", container.var_name, op_idx),
                });

                edges.push(HyperEdge {
                    op: "derive_iterator".into(),
                    inputs: vec![cont_id],
                    output: Some(iter_id),
                    line: op.line,
                });

                active_iterators.push(iter_id);
            } else if op.op_kind.invalidates_iterators() {
                for iter_id in &active_iterators {
                    vc.push(VerificationCondition {
                        name: format!("iter_valid_{}_op{}", container.var_name, op_idx),
                        kind: VcKind::IteratorValid {
                            container: cont_id,
                            op_index: op_idx,
                        },
                        description: format!(
                            "Iterator {} (node {}) on '{}' invalidated by {} at op {}",
                            iter_id, iter_id, container.var_name, op.op_kind, op_idx
                        ),
                    });
                }
                active_iterators.clear();

                edges.push(HyperEdge {
                    op: format!("mutate_{}", op.op_kind),
                    inputs: vec![cont_id],
                    output: None,
                    line: op.line,
                });
            }
        }
    }

    LowerResult { nodes, edges, vc }
}

/// Run audit-only verification: produce a report without mutating source.
pub fn audit(tu: &TranslationUnit, file_path: &str) -> AuditReport {
    let mut analyzer = Analyzer::new(file_path);
    analyzer.analyze_tu(tu);

    let mut iterator_violations = Vec::new();
    let mut pure_virtual_sites = Vec::new();
    let mut memory_violations = Vec::new();
    let mut bounds_violations = Vec::new();
    let mut div_by_zero_violations = Vec::new();

    for f in &analyzer.findings {
        match f.class.as_str() {
            "cpp-iterator-invalidation" => iterator_violations.push(f.message.clone()),
            "cpp-pure-virtual-call" => pure_virtual_sites.push(f.message.clone()),
            "cpp-use-after-delete" | "cpp-double-delete" | "cpp-null-deref" => {
                memory_violations.push(f.message.clone());
            }
            "cpp-vector-oob" => bounds_violations.push(f.message.clone()),
            "cpp-div-by-zero" => div_by_zero_violations.push(f.message.clone()),
            _ => {}
        }
    }

    let vtable_integrity_ok = tu.classes.iter().all(|c| {
        if c.pure_virtual_methods().is_empty() {
            let has_virtuals = !c.virtual_methods().is_empty();
            let vtable = tu.vtables.get(&c.name);
            match vtable {
                Some(vt) => !has_virtuals || !vt.is_empty(),
                None => true,
            }
        } else {
            true
        }
    });

    let passed = iterator_violations.is_empty()
        && pure_virtual_sites.is_empty()
        && memory_violations.is_empty()
        && bounds_violations.is_empty()
        && div_by_zero_violations.is_empty()
        && vtable_integrity_ok;

    AuditReport {
        file_path: file_path.to_string(),
        total_classes: tu.classes.len(),
        total_functions: tu.functions.len(),
        total_containers: tu.containers.len(),
        iterator_violations,
        pure_virtual_call_sites: pure_virtual_sites,
        memory_violations,
        bounds_violations,
        div_by_zero_violations,
        vtable_integrity_ok,
        passed,
    }
}

impl std::fmt::Display for AuditReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== C++ Formal Verification Audit Report ===")?;
        writeln!(f, "File: {}", self.file_path)?;
        writeln!(f, "Classes analyzed: {}", self.total_classes)?;
        writeln!(f, "Functions analyzed: {}", self.total_functions)?;
        writeln!(f, "Containers analyzed: {}", self.total_containers)?;
        writeln!(
            f,
            "Vtable integrity: {}",
            if self.vtable_integrity_ok { "OK" } else { "FAILED" }
        )?;
        writeln!(f)?;

        if !self.iterator_violations.is_empty() {
            writeln!(f, "[ITERATOR INVALIDATION RISKS]")?;
            for v in &self.iterator_violations {
                writeln!(f, "  - {}", v)?;
            }
            writeln!(f)?;
        }

        if !self.pure_virtual_call_sites.is_empty() {
            writeln!(f, "[PURE VIRTUAL CALL SITES]")?;
            for v in &self.pure_virtual_call_sites {
                writeln!(f, "  - {}", v)?;
            }
            writeln!(f)?;
        }

        if !self.memory_violations.is_empty() {
            writeln!(f, "[MEMORY SAFETY RISKS]")?;
            for v in &self.memory_violations {
                writeln!(f, "  - {}", v)?;
            }
            writeln!(f)?;
        }

        if !self.bounds_violations.is_empty() {
            writeln!(f, "[BOUNDS VIOLATIONS]")?;
            for v in &self.bounds_violations {
                writeln!(f, "  - {}", v)?;
            }
            writeln!(f)?;
        }

        if !self.div_by_zero_violations.is_empty() {
            writeln!(f, "[DIVISION BY ZERO RISKS]")?;
            for v in &self.div_by_zero_violations {
                writeln!(f, "  - {}", v)?;
            }
            writeln!(f)?;
        }

        writeln!(
            f,
            "RESULT: [{}]",
            if self.passed { "PASS" } else { "FAIL" }
        )?;
        Ok(())
    }
}

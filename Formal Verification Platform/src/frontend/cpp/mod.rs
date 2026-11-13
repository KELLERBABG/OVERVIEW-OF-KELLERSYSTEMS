//! C++ Frontend for Keller-VIR Formal Verification Platform
//!
//! Lowers C++ classes, vtables, and standard library collections into
//! Keller-VIR SSA Hypergraphs. Supports audit-only mode for benchmark
//! verification without source mutation.

pub mod ast;
pub mod lower;
pub mod vtable;

use crate::core::hypergraph::{HyperGraph, NodeId};
use crate::core::ssa::{SsaBuilder, SsaValue};
use std::collections::HashMap;

/// Result of lowering a C++ translation unit into Keller-VIR.
pub struct CppLoweringResult {
    /// The resulting SSA hypergraph.
    pub graph: HyperGraph,
    /// Mapping from C++ class names to their vtable node IDs.
    pub vtable_map: HashMap<String, NodeId>,
    /// All detected pure virtual functions (for absence checking).
    pub pure_virtuals: Vec<PureVirtualInfo>,
    /// STL container instantiations detected in the TU.
    pub stl_containers: Vec<StlContainerInstance>,
}

/// Metadata about a pure virtual function declaration.
#[derive(Debug, Clone)]
pub struct PureVirtualInfo {
    /// Fully qualified name (e.g., "MyClass::virtualMethod").
    pub qualified_name: String,
    /// The class that declares this pure virtual.
    pub declaring_class: String,
    /// Source location (file, line, column).
    pub location: SourceLocation,
}

/// A detected STL container instantiation.
#[derive(Debug, Clone)]
pub struct StlContainerInstance {
    /// Container kind (e.g., "std::vector", "std::string").
    pub kind: String,
    /// Element type (for vector) or char type (for string).
    pub element_type: String,
    /// Variable name if it's a named instance.
    pub var_name: Option<String>,
    /// Source location.
    pub location: SourceLocation,
}

/// Source location in the original C++ file.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// Configuration for the C++ frontend lowering pass.
#[derive(Debug, Clone)]
pub struct CppFrontendConfig {
    /// If true, only analyze and report; never mutate source.
    pub audit_only: bool,
    /// Enable STL container invariant checking.
    pub check_stl_invariants: bool,
    /// Enable iterator invalidation analysis.
    pub check_iterator_invalidation: bool,
    /// Enable pure virtual call detection.
    pub check_pure_virtual_calls: bool,
    /// Maximum class hierarchy depth to analyze.
    pub max_hierarchy_depth: usize,
}

impl Default for CppFrontendConfig {
    fn default() -> Self {
        Self {
            audit_only: true,
            check_stl_invariants: true,
            check_iterator_invalidation: true,
            check_pure_virtual_calls: true,
            max_hierarchy_depth: 32,
        }
    }
}

/// The C++ frontend driver.
pub struct CppFrontend {
    config: CppFrontendConfig,
}

impl CppFrontend {
    /// Create a new C++ frontend with the given configuration.
    pub fn new(config: CppFrontendConfig) -> Self {
        Self { config }
    }

    /// Lower a parsed C++ AST into a Keller-VIR SSA hypergraph.
    pub fn lower(&self, ast: &ast::TranslationUnit) -> Result<CppLoweringResult, CppFrontendError> {
        let mut ssa = SsaBuilder::new();
        let mut vtable_map = HashMap::new();
        let mut pure_virtuals = Vec::new();

        // Phase 1: Lower class declarations and build vtables.
        for class in &ast.classes {
            let vt_node = vtable::lower_vtable(&mut ssa, class)?;
            vtable_map.insert(class.name.clone(), vt_node);

            for method in &class.methods {
                if method.is_pure_virtual {
                    pure_virtuals.push(PureVirtualInfo {
                        qualified_name: format!("{}::{}", class.name, method.name),
                        declaring_class: class.name.clone(),
                        location: method.location.clone(),
                    });
                }
            }
        }

        // Phase 2: Lower function bodies into SSA.
        for func in &ast.functions {
            lower::lower_function(&mut ssa, func)?;
        }

        // Phase 3: Detect STL container instantiations.
        let stl_containers = detect_stl_containers(ast);

        let graph = ssa.build();

        Ok(CppLoweringResult {
            graph,
            vtable_map,
            pure_virtuals,
            stl_containers,
        })
    }

    /// Verify that no pure virtual functions are called at runtime.
    /// In audit-only mode, this only reports violations without healing.
    pub fn verify_no_pure_virtual_calls(
        &self,
        result: &CppLoweringResult,
    ) -> Vec<PureVirtualCallViolation> {
        let mut violations = Vec::new();

        for pv in &result.pure_virtuals {
            // Check if any call site targets this pure virtual.
            // In a full implementation, we'd trace the vtable slot and check
            // that all concrete subclasses override it before any dispatch.
            let called = self.is_pure_virtual_called(result, pv);
            if called {
                violations.push(PureVirtualCallViolation {
                    pure_virtual: pv.clone(),
                    call_sites: vec![], // Populated by dataflow analysis
                    severity: ViolationSeverity::Error,
                });
            }
        }

        violations
    }

    /// Verify STL container invariants hold throughout the program.
    pub fn verify_stl_invariants(
        &self,
        result: &CppLoweringResult,
    ) -> Vec<StlInvariantViolation> {
        let mut violations = Vec::new();

        for container in &result.stl_containers {
            match container.kind.as_str() {
                "std::vector" => {
                    // Check: capacity >= size always holds.
                    // Check: no iterator invalidation from push_back during iteration.
                    if self.config.check_iterator_invalidation {
                        let inv = check_vector_iterator_invalidation(&result.graph, container);
                        violations.extend(inv);
                    }
                }
                "std::string" => {
                    // Check: null-termination invariant (s[n] == '\0').
                    // Check: no use-after-free of c_str() after mutation.
                    let inv = check_string_invariants(&result.graph, container);
                    violations.extend(inv);
                }
                _ => {}
            }
        }

        violations
    }

    fn is_pure_virtual_called(
        &self,
        result: &CppLoweringResult,
        pv: &PureVirtualInfo,
    ) -> bool {
        // Walk the hypergraph looking for virtual dispatch nodes
        // that target a slot corresponding to this pure virtual
        // in a context where no concrete override is guaranteed.
        let mut called = false;
        for node_id in result.graph.nodes() {
            if let Some(node) = result.graph.get_node(node_id) {
                if node.is_virtual_dispatch() {
                    // Check if this dispatch could reach the pure virtual slot
                    // without a concrete override being installed.
                    if self.dispatch_targets_pure_virtual(&result.graph, node_id, pv) {
                        called = true;
                        break;
                    }
                }
            }
        }
        called
    }

    fn dispatch_targets_pure_virtual(
        &self,
        graph: &HyperGraph,
        dispatch_node: NodeId,
        pv: &PureVirtualInfo,
    ) -> bool {
        // A pure virtual is "called" if there exists a path from the dispatch
        // node to the pure virtual's vtable slot where no concrete class
        // in the hierarchy overrides it.
        let _ = (graph, dispatch_node, pv);
        // Conservative: flag as potential violation if the declaring class
        // is not abstract at the call site (i.e., could be instantiated).
        true
    }
}

/// A violation of the no-pure-virtual-call invariant.
#[derive(Debug, Clone)]
pub struct PureVirtualCallViolation {
    pub pure_virtual: PureVirtualInfo,
    pub call_sites: Vec<SourceLocation>,
    pub severity: ViolationSeverity,
}

/// Severity levels for verification violations.
#[derive(Debug, Clone, PartialEq)]
pub enum ViolationSeverity {
    /// Hard error: program is UB or guaranteed crash.
    Error,
    /// Warning: potential issue under certain conditions.
    Warning,
    /// Informational: style or best-practice note.
    Info,
}

/// An STL invariant violation.
#[derive(Debug, Clone)]
pub struct StlInvariantViolation {
    pub container: StlContainerInstance,
    pub invariant_name: String,
    pub description: String,
    pub location: SourceLocation,
    pub severity: ViolationSeverity,
}

/// Errors that can occur during C++ frontend lowering.
#[derive(Debug)]
pub enum CppFrontendError {
    /// The AST contains a construct the frontend cannot lower.
    UnsupportedConstruct { name: String, location: SourceLocation },
    /// A vtable slot conflict was detected.
    VtableSlotConflict { class: String, slot: usize },
    /// Internal builder error propagated from the SSA core.
    SsaError(String),
}

impl std::fmt::Display for CppFrontendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedConstruct { name, location } => {
                write!(
                    f,
                    "Unsupported C++ construct '{}' at {}:{}:{}",
                    name, location.file, location.line, location.column
                )
            }
            Self::VtableSlotConflict { class, slot } => {
                write!(f, "VTable slot conflict in class '{}' at slot {}", class, slot)
            }
            Self::SsaError(msg) => write!(f, "SSA builder error: {}", msg),
        }
    }
}

impl std::fmt::Debug for CppFrontendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt(f)
    }
}

/// Detect STL container instantiations in a translation unit.
fn detect_stl_containers(tu: &ast::TranslationUnit) -> Vec<StlContainerInstance> {
    let mut containers = Vec::new();

    for var in &tu.variables {
        if let Some(container) = classify_stl_container(&var.type_name, &var.name, &var.location) {
            containers.push(container);
        }
    }

    // Also check function parameters and local declarations.
    for func in &tu.functions {
        for param in &func.params {
            if let Some(container) = classify_stl_container(&param.type_name, None, &param.location) {
                containers.push(container);
            }
        }
    }

    containers
}

/// Classify a type as an STL container if it matches known patterns.
fn classify_stl_container(
    type_name: &str,
    var_name: Option<&String>,
    location: &SourceLocation,
) -> Option<StlContainerInstance> {
    // Match std::vector<T>
    if let Some(inner) = type_name.strip_prefix("std::vector<").and_then(|s| s.strip_suffix('>')) {
        return Some(StlContainerInstance {
            kind: "std::vector".to_string(),
            element_type: inner.to_string(),
            var_name: var_name.map(|s| s.clone()),
            location: location.clone(),
        });
    }

    // Match std::string or std::basic_string<char>
    if type_name == "std::string" || type_name == "std::basic_string<char>" {
        return Some(StlContainerInstance {
            kind: "std::string".to_string(),
            element_type: "char".to_string(),
            var_name: var_name.map(|s| s.clone()),
            location: location.clone(),
        });
    }

    None
}

/// Check for vector iterator invalidation patterns.
fn check_vector_iterator_invalidation(
    graph: &HyperGraph,
    container: &StlContainerInstance,
) -> Vec<StlInvariantViolation> {
    let mut violations = Vec::new();

    // Pattern: range-for or iterator loop over a vector with push_back/insert/erase
    // inside the loop body.
    for node_id in graph.nodes() {
        if let Some(node) = graph.get_node(node_id) {
            if node.is_loop_header() && node.iterates_over_container(&container.kind) {
                // Check loop body for mutating operations.
                let body_nodes: Vec<NodeId> = graph.successors(node_id).collect();
                for &body_node in &body_nodes {
                    if let Some(bn) = graph.get_node(body_node) {
                        if bn.is_call() && bn.calls_mutating_vector_method() {
                            violations.push(StlInvariantViolation {
                                container: container.clone(),
                                invariant_name: "iterator_invalidation".to_string(),
                                description: format!(
                                    "Call to mutating method invalidates iterators \
                                     for '{}' declared at {}:{}",
                                    container.var_name.as_deref().unwrap_or("<unnamed>"),
                                    container.location.file,
                                    container.location.line,
                                ),
                                location: bn.location(),
                                severity: ViolationSeverity::Error,
                            });
                        }
                    }
                }
            }
        }
    }

    violations
}

/// Check for string invariant violations.
fn check_string_invariants(
    graph: &HyperGraph,
    container: &StlContainerInstance,
) -> Vec<StlInvariantViolation> {
    let mut violations = Vec::new();

    // Pattern: c_str() result used after a mutation (push_back, append, operator+=).
    for node_id in graph.nodes() {
        if let Some(node) = graph.get_node(node_id) {
            if node.is_call() && node.calls_method("c_str") {
                // Check if there's a subsequent mutation before the pointer is used.
                let successors: Vec<NodeId> = graph.successors(node_id).collect();
                for &succ in &successors {
                    if let Some(sn) = graph.get_node(succ) {
                        if sn.is_call() && sn.calls_mutating_string_method() {
                            violations.push(StlInvariantViolation {
                                container: container.clone(),
                                invariant_name: "c_str_lifetime".to_string(),
                                description: format!(
                                    "Pointer from c_str() on '{}' may be invalidated \
                                     by subsequent mutation",
                                    container.var_name.as_deref().unwrap_or("<unnamed>"),
                                ),
                                location: sn.location(),
                                severity: ViolationSeverity::Error,
                            });
                        }
                    }
                }
            }
        }
    }

    violations
}

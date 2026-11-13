//! VTable analysis and lowering for C++ virtual dispatch.
//!
//! Builds vtable structures from class declarations and verifies
//! that no pure virtual functions are dispatched at runtime.

use crate::core::hypergraph::NodeId;
use crate::core::ssa::SsaBuilder;
use super::ast::{ClassDecl, MethodDecl};
use super::CppFrontendError;

/// Lower a class's vtable into the SSA hypergraph.
///
/// Returns the NodeId of the vtable root node in the graph.
pub fn lower_vtable(
    ssa: &mut SsaBuilder,
    class: &ClassDecl,
) -> Result<NodeId, CppFrontendError> {
    // Assign vtable slots to virtual methods (non-pure-virtual first,
    // then pure virtuals get sentinel values).
    let mut slot = 0usize;
    for method in &class.methods {
        if method.is_virtual && !method.is_pure_virtual {
            // Concrete virtual: assign a slot.
            ssa.define_vtable_slot(
                &class.name,
                slot,
                &method.name,
                &method.return_type,
            );
            slot += 1;
        } else if method.is_pure_virtual {
            // Pure virtual: assign a slot with a poison/sentinel value.
            ssa.define_vtable_slot(
                &class.name,
                slot,
                &method.name,
                "__PURE_VIRTUAL_SENTINEL__",
            );
            slot += 1;
        }
    }

    // Create the vtable root node in the hypergraph.
    let vt_node = ssa.emit_vtable_root(&class.name, slot);

    // For each base class, merge vtable entries (simplified: just record
    // the inheritance relationship).
    for base in &class.bases {
        ssa.record_inheritance(&class.name, &base.name, base.is_virtual);
    }

    Ok(vt_node)
}

/// Verify that all concrete (non-abstract) classes override every
/// pure virtual from their bases.
pub fn verify_vtable_completeness(
    classes: &[ClassDecl],
) -> Vec<VtableCompletenessError> {
    let mut errors = Vec::new();

    for class in classes {
        if !class.is_abstract {
            // This class must override all pure virtuals from its bases.
            for base in &class.bases {
                if let Some(base_class) = classes.iter().find(|c| c.name == base.name) {
                    for method in &base_class.methods {
                        if method.is_pure_virtual {
                            // Check if `class` provides an override.
                            let overridden = class.methods.iter().any(
                                |m| m.name == method.name && m.is_virtual && !m.is_pure_virtual,
                            );
                            if !overridden {
                                errors.push(VtableCompletenessError {
                                    derived_class: class.name.clone(),
                                    base_class: base.name.clone(),
                                    missing_override: method.name.clone(),
                                    message: format!(
                                        "Class '{}' does not override pure virtual \
                                         '{}::{}' from base '{}'",
                                        class.name, base.name, method.name, base.name,
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    errors
}

/// A vtable completeness error: a concrete class fails to override a pure virtual.
#[derive(Debug, Clone)]
pub struct VtableCompletenessError {
    pub derived_class: String,
    pub base_class: String,
    pub missing_override: String,
    pub message: String,
}

/// Compute the vtable layout for a class hierarchy.
///
/// Returns a map from class name to its vtable entries (slot -> method name).
pub fn compute_vtable_layout(classes: &[ClassDecl]) -> std::collections::HashMap<String, Vec<(usize, String)>> {
    let mut layout = std::collections::HashMap::new();

    for class in classes {
        let mut entries = Vec::new();
        let mut slot = 0;

        // First, inherit slots from bases (simplified: just the first base).
        if let Some(base) = &class.bases.first() {
            if let Some(base_entries) = layout.get(&base.name) {
                for (s, name) in base_entries {
                    entries.push((*s, name.clone()));
                }
                slot = base_entries.len();
            }
        }

        // Then, add this class's own virtual methods.
        for method in &class.methods {
            if method.is_virtual {
                entries.push((slot, method.name.clone()));
                slot += 1;
            }
        }

        layout.insert(class.name.clone(), entries);
    }

    layout
}

/// Check that no vtable slot contains a pure virtual sentinel at a
/// concrete dispatch site. This is the core "absence of pure virtual calls" proof.
pub fn check_no_pure_virtual_dispatch(
    classes: &[ClassDecl],
) -> Vec<PureVirtualDispatchError> {
    let mut errors = Vec::new();

    for class in classes {
        if !class.is_abstract {
            // For each pure virtual in the hierarchy, verify it's overridden.
            for method in &class.methods {
                if method.is_pure_virtual {
                    // This shouldn't happen in a concrete class by definition,
                    // but guard against malformed input.
                    errors.push(PureVirtualDispatchError {
                        class_name: class.name.clone(),
                        method_name: method.name.clone(),
                        reason: "Pure virtual declared in non-abstract class".to_string(),
                    });
                }
            }

            // Check inherited pure virtuals.
            for base in &class.bases {
                if let Some(base_class) = classes.iter().find(|c| c.name == base.name) {
                    for bm in &base_class.methods {
                        if bm.is_pure_virtual {
                            let overridden = class.methods.iter().any(
                                |m| m.name == bm.name && !m.is_pure_virtual,
                            );
                            if !overridden {
                                errors.push(PureVirtualDispatchError {
                                    class_name: class.name.clone(),
                                    method_name: bm.name.clone(),
                                    reason: format!(
                                        "Pure virtual '{}' from '{}' not overridden in concrete class '{}'",
                                        bm.name, base.name, class.name,
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    errors
}

/// An error indicating a potential pure virtual dispatch.
#[derive(Debug, Clone)]
pub struct PureVirtualDispatchError {
    pub class_name: String,
    pub method_name: String,
    pub reason: String,
}

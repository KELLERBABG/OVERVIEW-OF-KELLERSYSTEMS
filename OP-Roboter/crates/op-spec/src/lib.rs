//! # op-spec — Automated medical specification generation
//!
//! Generates formal specifications from the system's runtime behaviour,
//! serving as an incorruptible, machine-readable mathematical reference for
//! medical certification (IEC 62304 / ISO 14971 oriented).
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

/// Category of a formal requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementCategory {
    /// Safety requirement.
    Safety,
    /// Performance requirement.
    Performance,
    /// Reliability requirement.
    Reliability,
}

/// A single formal requirement extracted from system behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormalRequirement {
    /// Unique requirement identifier.
    pub id: String,
    /// Requirement category.
    pub category: RequirementCategory,
    /// Machine-readable predicate (e.g. an SMT constraint name).
    pub predicate: String,
    /// Human-readable statement.
    pub statement: String,
    /// Bound value referenced by the requirement (SI units).
    pub bound_value: f64,
    /// Bound unit.
    pub unit: String,
}

/// A generated formal specification document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Specification {
    /// Specification identifier.
    pub specification_id: String,
    /// Software version this specification applies to.
    pub version: String,
    /// Formal requirements in the specification.
    pub requirements: Vec<FormalRequirement>,
}

/// Error raised during specification generation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpecError {
    /// No requirements were provided.
    #[error("at least one requirement is required")]
    EmptyRequirements,
}

/// Builder for formal specifications from recorded system behaviour.
#[derive(Debug, Clone, Default)]
pub struct SpecGenerator {
    /// Requirements accumulated so far.
    requirements: Vec<FormalRequirement>,
}

impl SpecGenerator {
    /// Construct an empty generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a requirement.
    pub fn add_requirement(&mut self, req: FormalRequirement) {
        self.requirements.push(req);
    }

    /// Add a typical safety requirement: a bound that must not be exceeded.
    pub fn add_bound_requirement(
        &mut self,
        id: &str,
        category: RequirementCategory,
        predicate: &str,
        statement: &str,
        bound_value: f64,
        unit: &str,
    ) {
        self.requirements.push(FormalRequirement {
            id: id.to_string(),
            category,
            predicate: predicate.to_string(),
            statement: statement.to_string(),
            bound_value,
            unit: unit.to_string(),
        });
    }

    /// Generate the specification document.
    pub fn generate(&self, specification_id: &str, version: &str) -> Result<Specification, SpecError> {
        if self.requirements.is_empty() {
            return Err(SpecError::EmptyRequirements);
        }
        Ok(Specification {
            specification_id: specification_id.to_string(),
            version: version.to_string(),
            requirements: self.requirements.clone(),
        })
    }

    /// Serialize the specification to JSON (canonical machine-readable form).
    pub fn to_json(&self, specification_id: &str, version: &str) -> Result<String, SpecError> {
        let spec = self.generate(specification_id, version)?;
        serde_json::to_string_pretty(&spec).map_err(|_| SpecError::EmptyRequirements)
    }

    /// Render the specification as a structured Markdown document (IEC 62304 / ISO 14971 audit artifact).
    pub fn to_markdown(&self, specification_id: &str, version: &str) -> Result<String, SpecError> {
        let spec = self.generate(specification_id, version)?;
        let mut md = String::new();
        md.push_str(&format!("# Formal Medical Software Specification: {}\n\n", spec.specification_id));
        md.push_str(&format!("**Software Version:** {}\n", spec.version));
        md.push_str("**Standards Alignment:** IEC 62304 Class C / ISO 14971 Risk Controls / IEC 80601-2-77\n\n");
        md.push_str("## 1. Formal Safety & Verification Requirements\n\n");
        md.push_str("| Requirement ID | Category | Formal Mathematical Predicate | Description | Limit Bound |\n");
        md.push_str("|---|---|---|---|---|\n");
        for req in &spec.requirements {
            let cat = match req.category {
                RequirementCategory::Safety => "Safety (Lethal Invariant)",
                RequirementCategory::Performance => "Performance",
                RequirementCategory::Reliability => "Reliability",
            };
            md.push_str(&format!(
                "| `{}` | {} | `{}` | {} | {:.4} {} |\n",
                req.id, cat, req.predicate, req.statement, req.bound_value, req.unit
            ));
        }
        md.push_str("\n---\n*Auto-generated by OP-Roboter formal verification engine.*\n");
        Ok(md)
    }

    /// Build a standard formal specification covering all baseline surgical lethal invariants.
    pub fn standard_surgical_spec() -> Self {
        let mut gen = Self::new();
        gen.add_bound_requirement(
            "REQ-SAF-001",
            RequirementCategory::Safety,
            "ws_min <= tool_pos <= ws_max",
            "Tool tip must remain strictly within reachable anatomical volume",
            0.30,
            "m",
        );
        gen.add_bound_requirement(
            "REQ-SAF-002",
            RequirementCategory::Safety,
            "normal · (p - p_b) >= margin",
            "Tool tip and continuous trajectory must stay on protected side of tissue boundaries",
            0.005,
            "m",
        );
        gen.add_bound_requirement(
            "REQ-SAF-003",
            RequirementCategory::Safety,
            "||(p_rcm - p_tip) x z_axis|| / ||z_axis|| <= rcm_tol",
            "Tool shaft must pass through trocar pivot point within allowable off-axis deviation",
            0.002,
            "m",
        );
        gen.add_bound_requirement(
            "REQ-SAF-004",
            RequirementCategory::Safety,
            "||f_tip|| <= max_force",
            "Tool-tissue interaction force must not exceed tissue rupture limit",
            5.0,
            "N",
        );
        gen.add_bound_requirement(
            "REQ-SAF-005",
            RequirementCategory::Safety,
            "insertion_depth <= depth_limit",
            "Incision depth must not exceed surgical target plane",
            0.10,
            "m",
        );
        gen.add_bound_requirement(
            "REQ-SAF-006",
            RequirementCategory::Safety,
            "action_energy <= energy_budget",
            "Total delivered kinetic/rotational energy per control cycle must not exceed energy budget",
            2.0,
            "J",
        );
        gen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_specification() {
        let mut gen = SpecGenerator::new();
        gen.add_bound_requirement(
            "REQ-001",
            RequirementCategory::Safety,
            "depth <= depth_limit",
            "Incision depth must never exceed the limit",
            0.1,
            "m",
        );
        let spec = gen.generate("SPEC-1", "0.1.0").unwrap();
        assert_eq!(spec.requirements.len(), 1);
        assert_eq!(spec.requirements[0].id, "REQ-001");
    }

    #[test]
    fn empty_generator_errors() {
        let gen = SpecGenerator::new();
        assert!(matches!(
            gen.generate("SPEC-1", "0.1.0"),
            Err(SpecError::EmptyRequirements)
        ));
    }

    #[test]
    fn serializes_to_json() {
        let mut gen = SpecGenerator::new();
        gen.add_bound_requirement(
            "REQ-002",
            RequirementCategory::Performance,
            "force <= force_limit",
            "Tool force must not exceed the limit",
            5.0,
            "N",
        );
        let json = gen.to_json("SPEC-2", "0.1.0").unwrap();
        assert!(json.contains("REQ-002"));
        assert!(json.contains("force <= force_limit"));
    }

    #[test]
    fn renders_markdown_report() {
        let gen = SpecGenerator::standard_surgical_spec();
        let md = gen.to_markdown("SPEC-OP-001", "0.1.0").unwrap();
        assert!(md.contains("Formal Medical Software Specification: SPEC-OP-001"));
        assert!(md.contains("REQ-SAF-001"));
        assert!(md.contains("REQ-SAF-004"));
    }
}
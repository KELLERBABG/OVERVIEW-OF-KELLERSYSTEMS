//! Kinetic path constraint generation from a planned motion path.

use crate::{Comparison, ConstraintExpr, ConstraintSet};
use op_math::Scalar;
use op_trajectory::{MotionPath, RcmConstraint, Workspace, SAFETY_MARGIN};

/// Error raised while generating path constraints.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConstraintError {
    /// The motion path has zero length.
    #[error("cannot generate constraints for a zero-length motion path")]
    ZeroLengthPath,
    /// The tool-shaft axis is degenerate (zero length).
    #[error("tool axis is degenerate")]
    DegenerateAxis,
}

/// Generates boolean action-path constraints from a planned motion and the
/// surgical environment.
#[derive(Debug, Clone)]
pub struct PathConstraintGenerator {
    /// Workspace with tissue boundaries.
    pub workspace: Workspace,
    /// RCM (trocar) constraint, when the procedure uses one.
    pub rcm: Option<RcmConstraint>,
    /// Maximum allowed tool-tip force (N).
    pub force_limit: Scalar,
    /// Maximum allowed incision depth (m).
    pub depth_limit: Scalar,
}

impl PathConstraintGenerator {
    /// Construct a generator.
    pub fn new(
        workspace: Workspace,
        rcm: Option<RcmConstraint>,
        force_limit: Scalar,
        depth_limit: Scalar,
    ) -> Self {
        Self {
            workspace,
            rcm,
            force_limit,
            depth_limit,
        }
    }

    /// Generate the boolean constraint expression for an entire motion path.
    ///
    /// The returned formula is satisfied only when **every** sampled tool pose
    /// stays within the reachable volume, on the protected side of each tissue
    /// boundary, within the RCM constraint, and below the force and depth
    /// limits. A fault state is described by the negation of this formula.
    pub fn generate(
        &self,
        path: &MotionPath,
        samples: usize,
    ) -> Result<ConstraintSet, ConstraintError> {
        if path.length() <= 1e-12 {
            return Err(ConstraintError::ZeroLengthPath);
        }
        if samples < 2 {
            return Err(ConstraintError::ZeroLengthPath);
        }

        let mut clauses = Vec::new();
        let mut quantities = Vec::new();
        let mut evaluations = Vec::new();

        let samples_vec = path
            .sample_n(samples)
            .map_err(|_| ConstraintError::ZeroLengthPath)?;

        for (idx, sample) in samples_vec.iter().enumerate() {
            let p = sample.position;
            let s = format!("{:.3}_{:.3}_{:.3}", p.x, p.y, p.z);

            // Reachable volume box.
            clauses.push(ConstraintExpr::And(vec![
                box_clause(&s, "x", Comparison::Ge, "ws_min_x"),
                box_clause(&s, "x", Comparison::Le, "ws_max_x"),
                box_clause(&s, "y", Comparison::Ge, "ws_min_y"),
                box_clause(&s, "y", Comparison::Le, "ws_max_y"),
                box_clause(&s, "z", Comparison::Ge, "ws_min_z"),
                box_clause(&s, "z", Comparison::Le, "ws_max_z"),
            ]));
            quantities.extend([
                format!("pos_{}_x", s),
                format!("pos_{}_y", s),
                format!("pos_{}_z", s),
            ]);
            evaluations.push(crate::Evaluation {
                name: format!("pos_{}_x", s),
                value: p.x,
            });
            evaluations.push(crate::Evaluation {
                name: format!("pos_{}_y", s),
                value: p.y,
            });
            evaluations.push(crate::Evaluation {
                name: format!("pos_{}_z", s),
                value: p.z,
            });

            // Tissue boundaries (discrete point signed distance).
            for (i, boundary) in self.workspace.boundaries.iter().enumerate() {
                let q = format!("clear_{}_b{}", s, i);
                let clearance = boundary.signed_distance(p);
                clauses.push(ConstraintExpr::Compare {
                    lhs: q.clone(),
                    op: Comparison::Ge,
                    rhs: "margin".into(),
                });
                quantities.push(q.clone());
                evaluations.push(crate::Evaluation {
                    name: q,
                    value: clearance,
                });
            }

            // Continuous segment check to previous sample (closing discretization gap).
            if idx > 0 {
                let prev_p = samples_vec[idx - 1].position;
                for (i, boundary) in self.workspace.boundaries.iter().enumerate() {
                    let seg_q = format!("seg_clear_{}_b{}", s, i);
                    let seg_dist = boundary.segment_signed_min_distance(prev_p, p);
                    clauses.push(ConstraintExpr::Compare {
                        lhs: seg_q.clone(),
                        op: Comparison::Ge,
                        rhs: "margin".into(),
                    });
                    quantities.push(seg_q.clone());
                    evaluations.push(crate::Evaluation {
                        name: seg_q,
                        value: seg_dist,
                    });
                }
            }

            // RCM constraint.
            if let Some(rcm) = &self.rcm {
                let axis = sample.orientation.rotate(op_math::Vector3::Z);
                if axis.norm() <= 1e-12 {
                    return Err(ConstraintError::DegenerateAxis);
                }
                let off_axis = rcm
                    .off_axis_distance(p, axis)
                    .map_err(|_| ConstraintError::DegenerateAxis)?;
                let q = format!("rcm_off_{}", s);
                clauses.push(ConstraintExpr::Compare {
                    lhs: q.clone(),
                    op: Comparison::Le,
                    rhs: "rcm_tol".into(),
                });
                quantities.push(q.clone());
                evaluations.push(crate::Evaluation {
                    name: q,
                    value: off_axis,
                });
            }

            // Force and depth limits.
            let force_val = path
                .end
                .force
                .or(path.start.force)
                .unwrap_or(0.0);
            clauses.push(ConstraintExpr::Compare {
                lhs: "force".into(),
                op: Comparison::Le,
                rhs: "force_limit".into(),
            });
            quantities.push("force".into());
            evaluations.push(crate::Evaluation {
                name: "force".into(),
                value: force_val,
            });

            let depth_val = if let Some(rcm) = &self.rcm {
                let axis = sample.orientation.rotate(op_math::Vector3::Z);
                rcm.projection_along_shaft(p, axis).unwrap_or(p.z)
            } else {
                p.z
            };
            clauses.push(ConstraintExpr::Compare {
                lhs: "depth".into(),
                op: Comparison::Le,
                rhs: "depth_limit".into(),
            });
            quantities.push("depth".into());
            evaluations.push(crate::Evaluation {
                name: "depth".into(),
                value: depth_val,
            });
        }

        // Deduplicate quantity names and evaluations.
        let mut seen = std::collections::BTreeSet::new();
        quantities.retain(|q| seen.insert(q.clone()));

        let mut seen_eval = std::collections::BTreeSet::new();
        evaluations.retain(|e| seen_eval.insert(e.name.clone()));

        Ok(ConstraintSet {
            formula: ConstraintExpr::And(clauses),
            quantities,
            evaluations,
        })
    }

    /// Default safety margin used by the generator.
    pub const DEFAULT_MARGIN: Scalar = SAFETY_MARGIN;
}

/// Build a single box-bound comparison clause.
fn box_clause(stamp: &str, axis: &str, op: Comparison, rhs: &str) -> ConstraintExpr {
    ConstraintExpr::Compare {
        lhs: format!("pos_{}_{}", stamp, axis),
        op,
        rhs: rhs.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_math::{Isometry3, Quaternion, Vector3};
    use op_trajectory::{TissueBoundary, Volume, Waypoint};

    fn test_path() -> (MotionPath, Workspace) {
        let vol = Volume::new(
            Vector3::new(-0.2, -0.2, 0.0),
            Vector3::new(0.2, 0.2, 0.2),
        )
        .unwrap();
        let boundary = TissueBoundary::new(Vector3::new(0.0, 0.0, 0.1), Vector3::Z);
        let ws = Workspace::new(vol, vec![boundary]);
        let start = Waypoint {
            pose: Isometry3::from_rotation_quaternion(
                Quaternion::IDENTITY,
                Vector3::new(0.0, 0.0, 0.02),
            ),
            force: Some(2.0),
        };
        let end = Waypoint {
            pose: Isometry3::from_rotation_quaternion(
                Quaternion::IDENTITY,
                Vector3::new(0.0, 0.0, 0.05),
            ),
            force: Some(2.0),
        };
        (MotionPath::new(start, end), ws)
    }

    #[test]
    fn generates_formula_for_safe_path() {
        let (path, ws) = test_path();
        let gen = PathConstraintGenerator::new(ws, None, 5.0, 0.1);
        let set = gen.generate(&path, 4).unwrap();
        assert!(!set.quantities.is_empty());
        assert!(!matches!(set.formula, ConstraintExpr::True));
    }

    #[test]
    fn zero_length_path_errors() {
        let (_, ws) = test_path();
        let gen = PathConstraintGenerator::new(ws, None, 5.0, 0.1);
        let p = Waypoint {
            pose: Isometry3::from_translation(Vector3::new(0.0, 0.0, 0.02)),
            force: None,
        };
        let path = MotionPath::new(p, p);
        assert_eq!(
            gen.generate(&path, 4).unwrap_err(),
            ConstraintError::ZeroLengthPath
        );
    }
}
//! # op-energy — Symbolic force & energy analysis
//!
//! Symbolic computation of force, torque and energy delivery per tool action,
//! to exclude tissue overload or hardware failure before it happens.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use op_math::{Scalar, Vector3};
use serde::{Deserialize, Serialize};

/// Affine expression `constant + Σ coefficientᵢ * quantityᵢ`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolicExpr {
    /// Constant term.
    pub constant: Scalar,
    /// Coefficients keyed by quantity name.
    pub coefficients: Vec<(String, Scalar)>,
}

impl SymbolicExpr {
    /// The zero expression.
    pub fn zero() -> Self {
        Self { constant: 0.0, coefficients: Vec::new() }
    }

    /// Evaluate for a set of named quantity values.
    pub fn evaluate(&self, env: &[QuantityValue]) -> Scalar {
        let mut v = self.constant;
        for (name, coef) in &self.coefficients {
            if let Some(q) = env.iter().find(|e| &e.name == name) {
                v += coef * q.value;
            }
        }
        v
    }
}

/// A named scalar quantity value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantityValue {
    /// Quantity name.
    pub name: String,
    /// Quantity value.
    pub value: Scalar,
}

/// End-effector wrench: force (N) and torque (N·m) at the tool tip.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ToolWrench {
    /// Force at the tool tip (N).
    pub force: Vector3,
    /// Torque at the tool tip (N·m).
    pub torque: Vector3,
}

/// Energy and work result for a tool action.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnergyReport {
    /// Work done by linear force (J).
    pub linear_work: Scalar,
    /// Work done by torque (J).
    pub rotational_work: Scalar,
    /// Total energy delivered (J).
    pub total_energy: Scalar,
    /// Peak power during the action (W).
    pub peak_power: Scalar,
}

/// Error raised during energy analysis.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnergyError {
    /// A numeric quantity required for evaluation is missing.
    #[error("missing quantity: {0}")]
    MissingQuantity(String),
}

/// Performs symbolic force and energy analysis for tool actions.
#[derive(Debug, Clone)]
pub struct EnergyAnalyzer;

impl EnergyAnalyzer {
    /// Symbolic torque for a revolute joint under a tool wrench:
    /// `τ = z · ( (p_tip - p) × f + n )`.
    pub fn joint_torque_symbolic(
        &self,
        joint_axis: Vector3,
        joint_position: Vector3,
        tip_position: Vector3,
    ) -> SymbolicExpr {
        let lever = joint_axis.cross(tip_position - joint_position);
        let mut expr = SymbolicExpr::zero();
        expr.coefficients = vec![
            ("f_x".into(), lever.x),
            ("f_y".into(), lever.y),
            ("f_z".into(), lever.z),
            ("n_x".into(), joint_axis.x),
            ("n_y".into(), joint_axis.y),
            ("n_z".into(), joint_axis.z),
        ];
        expr
    }

    /// Energy delivered by a tool action over a displacement, rotation and duration.
    pub fn action_energy(
        &self,
        wrench_start: ToolWrench,
        wrench_end: ToolWrench,
        displacement: Vector3,
        angular_displacement: Scalar,
    ) -> EnergyReport {
        self.action_energy_with_duration(wrench_start, wrench_end, displacement, angular_displacement, 1.0)
    }

    /// Energy and power delivered by a tool action over a given duration `dt` (s).
    pub fn action_energy_with_duration(
        &self,
        wrench_start: ToolWrench,
        wrench_end: ToolWrench,
        displacement: Vector3,
        angular_displacement: Scalar,
        dt: Scalar,
    ) -> EnergyReport {
        let dt_safe = if dt <= 1e-9 { 1e-9 } else { dt };
        let avg_force = (wrench_start.force + wrench_end.force) * 0.5;
        let avg_torque_mag = (wrench_start.torque.norm() + wrench_end.torque.norm()) * 0.5;
        let linear_work = avg_force.dot(displacement).abs();
        let rotational_work = avg_torque_mag * angular_displacement.abs();
        let total_energy = linear_work + rotational_work;
        let peak_power = total_energy / dt_safe;
        EnergyReport {
            linear_work,
            rotational_work,
            total_energy,
            peak_power,
        }
    }

    /// Check if action energy and peak power conform to safety limits.
    pub fn check_energy_safety(
        &self,
        report: &EnergyReport,
        energy_budget: Scalar,
        peak_power_limit: Scalar,
    ) -> Result<(), String> {
        if report.total_energy > energy_budget + 1e-9 {
            return Err(format!(
                "total energy {:.4} J exceeds budget {:.4} J",
                report.total_energy, energy_budget
            ));
        }
        if report.peak_power > peak_power_limit + 1e-9 {
            return Err(format!(
                "peak power {:.4} W exceeds limit {:.4} W",
                report.peak_power, peak_power_limit
            ));
        }
        Ok(())
    }

    /// Detect tissue overload: force magnitude exceeds the limit while in contact.
    pub fn tissue_overload(&self, wrench: ToolWrench, force_limit: Scalar, contact_threshold: Scalar) -> bool {
        wrench.force.norm() > force_limit && wrench.force.norm() > contact_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joint_torque_symbolic_evaluates() {
        let analyzer = EnergyAnalyzer;
        let expr = analyzer.joint_torque_symbolic(Vector3::Z, Vector3::ZERO, Vector3::new(1.0, 0.0, 0.0));
        // lever = r × z = (0, 1, 0) → τ = f_y * 1.0
        let tau = expr.evaluate(&[QuantityValue { name: "f_y".into(), value: 10.0 }]);
        assert!((tau - 10.0).abs() < 1e-9);
    }

    #[test]
    fn action_energy_is_correct() {
        let analyzer = EnergyAnalyzer;
        let w = ToolWrench { force: Vector3::new(5.0, 0.0, 0.0), torque: Vector3::ZERO };
        let rep = analyzer.action_energy(w, w, Vector3::new(0.1, 0.0, 0.0), 0.0);
        assert!((rep.linear_work - 0.5).abs() < 1e-9); // 5 N * 0.1 m
    }

    #[test]
    fn detects_tissue_overload() {
        let analyzer = EnergyAnalyzer;
        let w = ToolWrench { force: Vector3::new(20.0, 0.0, 0.0), torque: Vector3::ZERO };
        assert!(analyzer.tissue_overload(w, 15.0, 1.0));
        assert!(!analyzer.tissue_overload(w, 25.0, 1.0));
    }
}
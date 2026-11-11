//! Jacobian analysis and singularity detection.

use crate::{Chain, DhParamError, JointState};
use op_math::{Scalar, Vector3};
use serde::{Deserialize, Serialize};

/// Sign of the Jacobian determinant loss in a configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SingularityKind {
    /// Determinant (or minimum singular value) is near zero.
    RankDeficient,
    /// Wrist axes are aligned (last three joint axes coplanar).
    Wrist,
    /// Elbow is fully extended or folded (shoulder–elbow–wrist align).
    Elbow,
    /// Shoulder of the RCM arm is at the workspace boundary.
    Shoulder,
    /// Undetermined but near-singular.
    Unknown,
}

/// Result of a singularity analysis at a configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingularityReport {
    /// True when the configuration is (near-)singular.
    pub is_singular: bool,
    /// Kind of singularity, when identified.
    pub kind: Option<SingularityKind>,
    /// Smallest singular value of the Jacobian (a proxy for manipulability).
    pub min_singular_value: Scalar,
    /// Estimated condition number of the Jacobian.
    pub condition_number: Scalar,
}

/// Geometric Jacobian (6×n) of a serial chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Jacobian {
    /// Linear part (3 rows), one column per joint.
    pub linear: Vec<Vector3>,
    /// Angular part (3 rows), one column per joint.
    pub angular: Vec<Vector3>,
}

impl Jacobian {
    /// Number of joints (columns).
    pub fn dof(&self) -> usize {
        self.linear.len()
    }

    /// Multiply the Jacobian by a joint velocity vector.
    pub fn apply(&self, qd: &[Scalar]) -> Option<(Vector3, Vector3)> {
        if qd.len() != self.dof() {
            return None;
        }
        let mut lin = Vector3::ZERO;
        let mut ang = Vector3::ZERO;
        for (i, &v) in qd.iter().enumerate() {
            lin += self.linear[i] * v;
            ang += self.angular[i] * v;
        }
        Some((lin, ang))
    }

    /// Compute the geometric Jacobian at a given joint state.
    pub fn compute(chain: &Chain, q: &JointState) -> Result<Self, DhParamError> {
        if chain.links.is_empty() {
            return Err(DhParamError::EmptyChain);
        }
        if q.len() != chain.dof() {
            return Err(DhParamError::EmptyChain);
        }
        use crate::ForwardKinematics;

        let n = chain.dof();
        let tip = chain.forward(q)?.position;
        let mut acc = op_math::Isometry3::IDENTITY;
        let mut linear = Vec::with_capacity(n);
        let mut angular = Vec::with_capacity(n);

        for (link, &qi) in chain.links.iter().zip(q.iter()) {
            let axis = acc.transform_vector(Vector3::Z);
            let p = acc.translation;
            match link.joint {
                crate::JointType::Revolute => {
                    linear.push(axis.cross(tip - p));
                    angular.push(axis);
                }
                crate::JointType::Prismatic => {
                    linear.push(axis);
                    angular.push(Vector3::ZERO);
                }
            }
            acc = acc.then(link.transform(qi));
        }

        Ok(Self { linear, angular })
    }

    /// Largest and smallest singular values of the linear part.
    ///
    /// Uses the PSD Gram matrix `G = J Jᵀ` (3×3). Its eigenvalues are the
    /// squared singular values. `λ_max` comes from power iteration; `λ_min`
    /// is recovered from `λ₂ + λ₃ = tr G - λ_max` and
    /// `λ₂ λ₃ = det G / λ_max` via the quadratic formula. Robust for PSD.
    fn gram_singular(&self) -> (Scalar, Scalar) {
        let mut g = [[0.0; 3]; 3];
        for c in &self.linear {
            let v = [c.x, c.y, c.z];
            for i in 0..3 {
                for j in 0..3 {
                    g[i][j] += v[i] * v[j];
                }
            }
        }

        let trace = g[0][0] + g[1][1] + g[2][2];
        let det = g[0][0] * (g[1][1] * g[2][2] - g[1][2] * g[2][1])
            - g[0][1] * (g[1][0] * g[2][2] - g[1][2] * g[2][0])
            + g[0][2] * (g[1][0] * g[2][1] - g[1][1] * g[2][0]);

        if trace <= 0.0 {
            return (0.0, 0.0);
        }

        // Power iteration for the largest eigenvalue.
        let mut v = [1.0, 0.0, 0.0];
        let mut lam_max = 1.0;
        for _ in 0..40 {
            let mut w = [0.0; 3];
            for i in 0..3 {
                for j in 0..3 {
                    w[i] += g[i][j] * v[j];
                }
            }
            let norm = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
            if norm < 1e-300 {
                lam_max = 0.0;
                break;
            }
            lam_max = w[0] * v[0] + w[1] * v[1] + w[2] * v[2];
            for i in 0..3 {
                v[i] = w[i] / norm;
            }
        }
        let lam_max = lam_max.max(0.0);

        // Remaining two eigenvalues sum to S, product to P.
        let s = (trace - lam_max).max(0.0);
        let p = if lam_max > 1e-300 { det / lam_max } else { 0.0 };
        let disc = (s * s - 4.0 * p).max(0.0);
        let lam_min = ((s - disc.sqrt()) * 0.5).max(0.0);

        (lam_max.sqrt(), lam_min.sqrt())
    }

    /// Singularity analysis at a joint state.
    pub fn analyze(chain: &Chain, q: &JointState, tol: Scalar) -> Result<SingularityReport, DhParamError> {
        let jac = Self::compute(chain, q)?;
        let (s_max, s_min) = jac.gram_singular();
        let condition = if s_max > 1e-12 { s_max / s_min.max(1e-12) } else { f64::INFINITY };
        let is_singular = s_min <= tol || condition.is_infinite();

        let kind = if !is_singular {
            None
        } else if jac.dof() >= 3 && s_min <= tol {
            let n = jac.dof();
            if n >= 3 {
                let a = jac.angular[n - 3];
                let b = jac.angular[n - 2];
                let _c = jac.angular[n - 1];
                if a.cross(b).norm() < tol {
                    Some(SingularityKind::Wrist)
                } else {
                    Some(SingularityKind::RankDeficient)
                }
            } else {
                Some(SingularityKind::RankDeficient)
            }
        } else {
            Some(SingularityKind::Unknown)
        };

        Ok(SingularityReport {
            is_singular,
            kind,
            min_singular_value: s_min,
            condition_number: condition,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Chain, DhLink};

    fn spatial_chain() -> Chain {
        Chain {
            links: vec![
                DhLink::revolute(0.0, 0.0, 0.0, std::f64::consts::FRAC_PI_2),
                DhLink::revolute(0.0, 0.0, 1.0, 0.0),
                DhLink::revolute(0.0, 0.0, 1.0, 0.0),
            ],
        }
    }

    #[test]
    fn jacobian_applies_to_velocity() {
        let chain = spatial_chain();
        let q = vec![0.0, 0.0, 0.0];
        let j = Jacobian::compute(&chain, &q).unwrap();
        let (lin, ang) = j.apply(&[1.0, 0.0, 0.0]).unwrap();
        assert!(lin.norm() > 0.5);
        assert!(ang.norm() > 0.5);
    }

    #[test]
    fn full_extension_is_singular() {
        let chain = spatial_chain();
        // Fully extended: the two 1 m links are collinear → rank deficient.
        let rep = Jacobian::analyze(&chain, &vec![0.0, 0.0, 0.0], 1e-6).unwrap();
        assert!(rep.is_singular);
    }

    #[test]
    fn non_singular_configuration() {
        let chain = spatial_chain();
        // Generic pose with bent elbow and perpendicular yaw → full rank.
        let q = vec![0.7, 1.1, 0.5];
        let j = Jacobian::compute(&chain, &q).unwrap();

        let mut g = [[0.0; 3]; 3];
        for c in &j.linear {
            let v = [c.x, c.y, c.z];
            for i in 0..3 {
                for k in 0..3 {
                    g[i][k] += v[i] * v[k];
                }
            }
        }
        let det = g[0][0] * (g[1][1] * g[2][2] - g[1][2] * g[2][1])
            - g[0][1] * (g[1][0] * g[2][2] - g[1][2] * g[2][0])
            + g[0][2] * (g[1][0] * g[2][1] - g[1][1] * g[2][0]);
        assert!(det > 1e-9, "Gram determinant too small: {det:e}");

        let rep = Jacobian::analyze(&chain, &q, 1e-6).unwrap();
        assert!(
            !rep.is_singular,
            "min singular value: {}",
            rep.min_singular_value
        );
    }
}
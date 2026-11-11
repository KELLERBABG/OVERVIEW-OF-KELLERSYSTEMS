//! Numerical inverse kinematics via damped least squares.

use crate::{Chain, DhParamError, ForwardKinematics, Jacobian, JointState, ToolPose};
use op_math::{Scalar, Vector3};

/// Result of an inverse kinematics solve.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IkSolution {
    /// Joint configuration found.
    pub q: JointState,
    /// Position error (m) at convergence.
    pub position_error: Scalar,
    /// Orientation error (rad) at convergence.
    pub orientation_error: Scalar,
    /// Number of iterations used.
    pub iterations: u32,
    /// True when the solve converged within tolerance.
    pub converged: bool,
}

/// Numerical inverse kinematics using damped least squares on the Jacobian.
pub trait InverseKinematics {
    /// Solve for a joint state reaching `target`.
    ///
    /// `q0` is the initial guess; `tol` is the position tolerance in metres.
    fn inverse(
        &self,
        target: &ToolPose,
        q0: &JointState,
        tol: Scalar,
    ) -> Result<IkSolution, DhParamError>;
}

impl InverseKinematics for Chain {
    fn inverse(
        &self,
        target: &ToolPose,
        q0: &JointState,
        tol: Scalar,
    ) -> Result<IkSolution, DhParamError> {
        if self.links.is_empty() {
            return Err(DhParamError::EmptyChain);
        }
        if q0.len() != self.dof() {
            return Err(DhParamError::EmptyChain);
        }

        let max_iter = 100;
        let mut q = q0.clone();
        let damping: Scalar = 1e-3;

        for iter in 0..max_iter {
            let pose = self.forward(&q)?;

            // Position error in base frame.
            let dp = target.position - pose.position;

            // Orientation error in base frame (axis-angle from relative rotation).
            let rel = target
                .orientation
                .rotation
                .mul_matrix(pose.orientation.rotation.transpose());
            let angle = rel.determinant().clamp(-1.0, 1.0).acos();
            // Robust axis extraction from skew-symmetric part.
            let mut axis = Vector3::new(
                rel.at(2, 1) - rel.at(1, 2),
                rel.at(0, 2) - rel.at(2, 0),
                rel.at(1, 0) - rel.at(0, 1),
            );
            let w = axis.norm();
            let orientation_error = if w > 1e-12 {
                axis /= w;
                angle
            } else {
                0.0
            };

            if dp.norm() <= tol && orientation_error.abs() <= tol {
                return Ok(IkSolution {
                    q,
                    position_error: dp.norm(),
                    orientation_error,
                    iterations: iter,
                    converged: true,
                });
            }

            // Damped least squares: dq = Jᵀ (J Jᵀ + λ²I)⁻¹ e
            let jac = Jacobian::compute(self, &q)?;
            let nn = jac.dof();

            let jlin: Vec<Vector3> = jac.linear.clone();
            let jang: Vec<Vector3> = jac.angular.clone();

            // Solve for dq using the 3×n linear system with damping.
            let g = gram(&jlin, damping);
            let mut dq = vec![0.0; nn];
            if let Some(inv) = invert3x3(&g) {
                let rhs = mat_vec(&inv, dp);
                for k in 0..nn {
                    dq[k] = jlin[k].dot(rhs);
                }
            }
            // Orientation refinement with the angular Jacobian block.
            let g2 = gram(&jang, damping);
            if let Some(inv2) = invert3x3(&g2) {
                let d_omega = Vector3::new(
                    axis.x * orientation_error,
                    axis.y * orientation_error,
                    axis.z * orientation_error,
                );
                let rhs2 = mat_vec(&inv2, d_omega);
                for k in 0..nn {
                    dq[k] += jang[k].dot(rhs2);
                }
            }

            for (qi, dqi) in q.iter_mut().zip(dq.iter()) {
                *qi += *dqi;
            }
        }

        let pose = self.forward(&q)?;
        let dp = target.position - pose.position;
        Ok(IkSolution {
            q,
            position_error: dp.norm(),
            orientation_error: 0.0,
            iterations: max_iter,
            converged: false,
        })
    }
}

/// Gram matrix `J Jᵀ + λ²I` for a 3×n Jacobian block.
fn gram(cols: &[Vector3], lambda: Scalar) -> [[Scalar; 3]; 3] {
    let mut g = [[0.0; 3]; 3];
    for c in cols {
        g[0][0] += c.x * c.x;
        g[0][1] += c.x * c.y;
        g[0][2] += c.x * c.z;
        g[1][0] += c.y * c.x;
        g[1][1] += c.y * c.y;
        g[1][2] += c.y * c.z;
        g[2][0] += c.z * c.x;
        g[2][1] += c.z * c.y;
        g[2][2] += c.z * c.z;
    }
    for (i, row) in g.iter_mut().enumerate() {
        row[i] += lambda * lambda;
    }
    g
}

/// Inverse of a 3×3 matrix, or `None` when singular.
fn invert3x3(m: &[[Scalar; 3]; 3]) -> Option<[[Scalar; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
        ],
    ])
}

/// Multiply a 3×3 matrix by a vector.
fn mat_vec(m: &[[Scalar; 3]; 3], v: Vector3) -> Vector3 {
    Vector3::new(
        m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
        m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
        m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
    )
}

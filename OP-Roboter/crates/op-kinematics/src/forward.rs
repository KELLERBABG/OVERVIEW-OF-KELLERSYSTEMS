//! Forward kinematics for serial chains.

use crate::{Chain, DhParamError, JointState, ToolPose};
use op_math::Isometry3;

/// Forward kinematics for a serial kinematic chain.
pub trait ForwardKinematics {
    /// Compute the tool pose for a given joint state.
    ///
    /// Returns `Err(DhParamError::EmptyChain)` when the chain has no links.
    fn forward(&self, q: &JointState) -> Result<ToolPose, DhParamError>;

    /// Compute the cumulative base-to-tip transform for a joint state.
    fn forward_transform(&self, q: &JointState) -> Result<Isometry3, DhParamError>;

    /// Compute the transform of a specific link frame (index into `links`).
    fn link_transform(&self, q: &JointState, link_index: usize) -> Result<Isometry3, DhParamError>;
}

impl ForwardKinematics for Chain {
    fn forward(&self, q: &JointState) -> Result<ToolPose, DhParamError> {
        if self.links.is_empty() {
            return Err(DhParamError::EmptyChain);
        }
        let t = self.forward_transform(q)?;
        Ok(ToolPose {
            position: t.translation,
            orientation: t,
        })
    }

    fn forward_transform(&self, q: &JointState) -> Result<Isometry3, DhParamError> {
        if self.links.is_empty() {
            return Err(DhParamError::EmptyChain);
        }
        if q.len() != self.dof() {
            return Err(DhParamError::EmptyChain); // length mismatch surfaced as invalid chain
        }
        let mut acc = Isometry3::IDENTITY;
        for (link, &qi) in self.links.iter().zip(q.iter()) {
            acc = acc.then(link.transform(qi));
        }
        Ok(acc)
    }

    fn link_transform(&self, q: &JointState, link_index: usize) -> Result<Isometry3, DhParamError> {
        if self.links.is_empty() {
            return Err(DhParamError::EmptyChain);
        }
        if link_index >= self.links.len() || q.len() != self.dof() {
            return Err(DhParamError::EmptyChain);
        }
        let mut acc = Isometry3::IDENTITY;
        for (link, &qi) in self.links.iter().zip(q.iter()).take(link_index + 1) {
            acc = acc.then(link.transform(qi));
        }
        Ok(acc)
    }
}
//! 3D workspace and tissue boundary representation.

use op_math::{Scalar, Vector3, EPSILON};
use serde::{Deserialize, Serialize};

/// Error raised for invalid workspace definitions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkspaceError {
    /// A volume has non-positive extent.
    #[error("volume extent must be positive")]
    NonPositiveExtent,
    /// A tissue boundary is invalid (empty or inconsistent).
    #[error("tissue boundary is invalid")]
    InvalidTissueBoundary,
}

/// An axis-aligned box volume.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Volume {
    /// Minimum corner.
    pub min: Vector3,
    /// Maximum corner.
    pub max: Vector3,
}

impl Volume {
    /// Construct a volume; returns an error when the extent is non-positive.
    pub fn new(min: Vector3, max: Vector3) -> Result<Self, WorkspaceError> {
        let extent = max - min;
        if extent.x <= 0.0 || extent.y <= 0.0 || extent.z <= 0.0 {
            return Err(WorkspaceError::NonPositiveExtent);
        }
        Ok(Self { min, max })
    }

    /// True when `p` lies within the volume (inclusive bounds).
    #[inline]
    pub fn contains(&self, p: Vector3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// True when the entire line segment `[p0, p1]` lies inside the volume.
    ///
    /// An axis-aligned box is a **convex** set, so a segment is contained iff
    /// both endpoints are contained. This is the *continuous* containment
    /// test: it closes the sampling-discretization gap for the workspace
    /// envelope between two control-step samples.
    #[inline]
    pub fn contains_segment(&self, p0: Vector3, p1: Vector3) -> bool {
        self.contains(p0) && self.contains(p1)
    }

    /// Distance from `p` to the volume surface (0 inside).
    #[inline]
    pub fn distance_to(&self, p: Vector3) -> Scalar {
        let dx = (self.min.x - p.x).max(0.0).max(p.x - self.max.x);
        let dy = (self.min.y - p.y).max(0.0).max(p.y - self.max.y);
        let dz = (self.min.z - p.z).max(0.0).max(p.z - self.max.z);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// A planar tissue boundary with a safety offset, described by a point and a
/// unit normal. The *protected* side is `normal · (p - point) >= 0`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TissueBoundary {
    /// A point on the plane (m).
    pub point: Vector3,
    /// Unit normal pointing toward the protected tissue (dimensionless).
    pub normal: Vector3,
}

impl TissueBoundary {
    /// Construct a boundary, normalizing the normal vector.
    pub fn new(point: Vector3, normal: Vector3) -> Self {
        Self {
            point,
            normal: normal.normalized(),
        }
    }

    /// Signed distance of `p` from the plane (positive on protected side).
    #[inline]
    pub fn signed_distance(&self, p: Vector3) -> Scalar {
        self.normal.dot(p - self.point)
    }

    /// True when `p` is on the protected (allowed) side by at least `margin`.
    #[inline]
    pub fn is_protected(&self, p: Vector3, margin: Scalar) -> bool {
        self.signed_distance(p) >= margin - EPSILON
    }

    /// Return a boundary shifted `distance` metres farther into the protected
    /// half-space (the plane is translated by `+normal`).
    ///
    /// In a dynamic surgical field the tissue may shift by up to `distance`
    /// between sensor updates; building this *expanded* (swept) boundary and
    /// checking the path against it yields a conservative, sensor-updated
    /// barrier-function check with no need to re-sample the whole path.
    #[inline]
    pub fn expanded(self, distance: Scalar) -> Self {
        Self::new(self.point + self.normal * distance, self.normal)
    }

    /// Exact minimum signed distance of the moving tool tip to the plane as it
    /// travels **linearly** from `p0` to `p1`.
    ///
    /// The signed distance along the interpolation `s(t) = p0 + t(p1 - p0)`
    /// is itself linear in `t`, so its minimum over the whole segment is
    /// simply the smaller endpoint value. This is exact — no value between the
    /// samples can dip below `min(d0, d1)`, closing the discretization gap for
    /// half-space (tissue-plane) constraints.
    #[inline]
    pub fn segment_signed_min_distance(&self, p0: Vector3, p1: Vector3) -> Scalar {
        self.signed_distance(p0).min(self.signed_distance(p1))
    }

    /// True when **every** point of the segment `[p0, p1]` remains at least
    /// `margin` on the protected side. Exact for piecewise-linear tool motion.
    #[inline]
    pub fn segment_protected_with_margin(&self, p0: Vector3, p1: Vector3, margin: Scalar) -> bool {
        self.segment_signed_min_distance(p0, p1) >= margin - EPSILON
    }
}

/// The surgical workspace: a reachable volume plus tissue boundaries that
/// the tool must never cross.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    /// Reachable axis-aligned volume of the arm.
    pub reachable: Volume,
    /// Protected tissue planes the tool tip must remain on the safe side of.
    pub boundaries: Vec<TissueBoundary>,
}

impl Workspace {
    /// Construct a workspace.
    pub fn new(reachable: Volume, boundaries: Vec<TissueBoundary>) -> Self {
        Self { reachable, boundaries }
    }

    /// True when a tool tip position is inside the reachable volume and on
    /// the protected side of every boundary (with the given margin).
    pub fn contains_with_margin(&self, p: Vector3, margin: Scalar) -> bool {
        if !self.reachable.contains(p) {
            return false;
        }
        self.boundaries.iter().all(|b| b.is_protected(p, margin))
    }

    /// Continuous per-step check: true when the **whole** segment `[p0, p1]`
    /// stays inside the envelope and protected by every tissue boundary.
    ///
    /// - Envelope: exact (convex box).
    /// - Tissue boundaries: exact linear half-space distance over the segment.
    ///
    /// Recommended over `contains_with_margin` for each control-loop step so
    /// a motion between samples can never slip through a tissue plane.
    pub fn contains_segment_with_margin(&self, p0: Vector3, p1: Vector3, margin: Scalar) -> bool {
        if !self.reachable.contains_segment(p0, p1) {
            return false;
        }
        self.boundaries
            .iter()
            .all(|b| b.segment_protected_with_margin(p0, p1, margin))
    }

    /// Minimum signed clearance of the whole segment `[p0, p1]` to any
    /// boundary (negative ⇒ the continuous path violates a boundary).
    pub fn segment_clearance_min(&self, p0: Vector3, p1: Vector3) -> Scalar {
        self.boundaries
            .iter()
            .map(|b| b.segment_signed_min_distance(p0, p1))
            .fold(f64::INFINITY, Scalar::min)
    }

    /// Minimum signed clearance of `p` to any boundary (negative when violated).
    pub fn clearance(&self, p: Vector3) -> Scalar {
        self.boundaries
            .iter()
            .map(|b| b.signed_distance(p))
            .fold(f64::INFINITY, Scalar::min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_segment_containment_is_exact_for_convex_box() {
        let vol = Volume::new(Vector3::new(-1.0, -1.0, -1.0), Vector3::new(1.0, 1.0, 1.0)).unwrap();
        // Both endpoints inside → entire segment inside (convex set).
        assert!(vol.contains_segment(Vector3::new(-0.5, 0.0, 0.0), Vector3::new(0.5, 0.0, 0.0)));
        // One endpoint outside → segment fails.
        assert!(!vol.contains_segment(Vector3::new(-0.5, 0.0, 0.0), Vector3::new(1.5, 0.0, 0.0)));
    }

    #[test]
    fn segment_min_distance_detects_crossing_between_samples() {
        let b = TissueBoundary::new(Vector3::new(0.0, 0.0, 0.10), Vector3::Z);
        // Discrete endpoints straddle the plane: the minimum over the segment
        // is the smaller endpoint (-0.05) — the crossing is detected exactly.
        let min = b.segment_signed_min_distance(Vector3::new(0.0, 0.0, 0.05), Vector3::new(0.0, 0.0, 0.15));
        assert!((min + 0.05).abs() < 1e-9);
        assert!(!b.segment_protected_with_margin(
            Vector3::new(0.0, 0.0, 0.05),
            Vector3::new(0.0, 0.0, 0.15),
            0.0,
        ));
    }

    #[test]
    fn expanded_boundary_catches_dynamic_tissue_shift() {
        let b = TissueBoundary::new(Vector3::new(0.0, 0.0, 0.10), Vector3::Z);
        // Both sampled endpoints are on the nominal protected side...
        let p0 = Vector3::new(0.0, 0.0, 0.12);
        let p1 = Vector3::new(0.0, 0.0, 0.20);
        assert!(b.segment_protected_with_margin(p0, p1, 0.0));
        // ...but the tissue may have shifted +0.06 m since the last update.
        // The swept (expanded) boundary is now at z=0.16; the segment clearly
        // violates it even though both discrete samples would have passed.
        let swept = b.expanded(0.06);
        assert!(!swept.segment_protected_with_margin(p0, p1, 0.0));
        assert!(swept.segment_signed_min_distance(p0, p1) < 0.0);
    }

    #[test]
    fn workspace_segment_check_holds_for_safe_step() {
        // Boundary at z = 0.1 with the protected side above (z ≥ 0.1).
        let ws = Workspace::new(
            Volume::new(Vector3::new(-1.0, -1.0, -1.0), Vector3::new(1.0, 1.0, 1.0)).unwrap(),
            vec![TissueBoundary::new(Vector3::new(0.0, 0.0, 0.1), Vector3::Z)],
        );
        // Entire segment on the protected side of the plane: safe.
        assert!(ws.contains_segment_with_margin(
            Vector3::new(0.0, 0.0, 0.2),
            Vector3::new(0.0, 0.0, 0.4),
            0.0,
        ));
        // Segment crosses the plane: the continuous check catches it even
        // though a sparse sampler might miss the crossing point.
        assert!(!ws.contains_segment_with_margin(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 0.2),
            0.0,
        ));
    }
}
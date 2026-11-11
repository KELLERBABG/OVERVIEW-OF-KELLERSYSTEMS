"""
Simulation subpackage: Synthetic scene generation, forward Monte Carlo ray-tracing,
and SiPM detector response modeling.
"""

from .synthetic_scene import (
    VoxelScene,
    create_void_phantom,
    create_heavy_inclusion_phantom,
    create_rebar_phantom,
)
from .ray_tracer import (
    MonteCarloRayTracer,
    MuonTrackEvent,
)
# NOTE: detector_sim is deprecated and deliberately not imported here. It is a
# second detector model that contradicts detector.py (direct position smearing
# rather than strip charge sharing, no geometry file). Import it explicitly from
# muonscope.simulation.detector_sim if you need the old behaviour.
from .geometry import (
    DetectorGeometry,
    Panel,
    StripPlane,
    load_detector_geometry,
    parse_detector_geometry,
    validate_detector_geometry,
)
from .detector import (
    DetectorResponse,
    HodoscopeSimulator,
    LayerHit,
    MeasuredTrack,
)

__all__ = [
    "VoxelScene",
    "create_void_phantom",
    "create_heavy_inclusion_phantom",
    "create_rebar_phantom",
    "MonteCarloRayTracer",
    "MuonTrackEvent",
    "DetectorGeometry",
    "Panel",
    "StripPlane",
    "load_detector_geometry",
    "parse_detector_geometry",
    "validate_detector_geometry",
    "DetectorResponse",
    "HodoscopeSimulator",
    "LayerHit",
    "MeasuredTrack",
]

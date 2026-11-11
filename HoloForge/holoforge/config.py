"""Configuration dataclasses: optics, grid, resin, print job."""
from __future__ import annotations

import json
from dataclasses import dataclass, field


@dataclass(frozen=True)
class OpticsConfig:
    """Physical setup of the HoloForge optical head.

    The relay images the SLM into the resin vat with magnification M
    (M < 1 = demagnification), so the effective pixel pitch inside the
    vat is p_slm * M. Default M = 0.25 (e.g. 8 um SLM through a 4x
    objective) -> 2 um vat pixels over a ~256 um field of view at
    n_xy = 128: a physically consistent mm-scale print envelope.
    """

    wavelength_nm: float = 405.0        # vacuum wavelength of the laser
    slm_pixels: int = 256               # square SLM active area (pixels)
    slm_pitch_um: float = 8.0           # SLM pixel pitch (micrometers)
    relay_magnification: float = 0.25   # SLM->vat magnification
    vat_index: float = 1.46             # refractive index of the resin

    @property
    def pitch_um(self) -> float:
        """Effective transverse pixel pitch inside the vat (um)."""
        return self.slm_pitch_um * self.relay_magnification

    @property
    def wavelength_vat_um(self) -> float:
        return self.wavelength_nm / 1000.0 / self.vat_index


@dataclass(frozen=True)
class GridConfig:
    """Simulation voxel grid. Voxel pitch == effective vat pixel pitch."""

    n_xy: int = 128                     # transverse samples per axis
    n_z: int = 96                       # axial slices
    slice_step_um: float | None = None  # dz per propagation step; default = 2 * pitch

    def __post_init__(self) -> None:
        if self.slice_step_um is None:
            object.__setattr__(self, "slice_step_um", 2.0)

    def extent_um(self, optics: OpticsConfig) -> tuple[float, float, float]:
        return (
            self.n_xy * optics.pitch_um,
            self.n_xy * optics.pitch_um,
            self.n_z * self.slice_step_um,
        )


@dataclass(frozen=True)
class ResinModel:
    """Minimal photo-resin model at the working wavelength."""

    name: str
    gel_dose_mJ_cm2: float   # dose at which a voxel gels
    absorption_per_mm: float # intensity attenuation coefficient alpha (1/mm)
    scatter_length_um: float # forward-scatter transport length (um); 0 disables
    index: float             # refractive index (overrides optics.vat_index when set)

    @property
    def amplitude_absorption_per_mm(self) -> float:
        # amplitude decays at half the intensity rate
        return 0.5 * self.absorption_per_mm


RESINS: dict[str, ResinModel] = {
    "hf-standard-405": ResinModel(
        name="hf-standard-405",
        gel_dose_mJ_cm2=12.0,
        absorption_per_mm=0.35,
        scatter_length_um=900.0,
        index=1.46,
    ),
    "hf-clear-405": ResinModel(
        name="hf-clear-405",           # low-scatter microfluidics formulation
        gel_dose_mJ_cm2=10.0,
        absorption_per_mm=0.20,
        scatter_length_um=4000.0,
        index=1.47,
    ),
    "hf-dental-405": ResinModel(
        name="hf-dental-405",          # filled ceramic-capable formulation
        gel_dose_mJ_cm2=18.0,
        absorption_per_mm=0.60,
        scatter_length_um=350.0,
        index=1.52,
    ),
    "hf-pegda-405": ResinModel(
        name="hf-pegda-405",           # PEGDA + 1.0 wt% TPO-L formulation (Stage 2)
        gel_dose_mJ_cm2=11.0,
        absorption_per_mm=0.28,
        scatter_length_um=3200.0,
        index=1.465,
    ),
    "hf-hdda-405": ResinModel(
        name="hf-hdda-405",            # HDDA + 1.0 wt% TPO-L high-density crosslink formulation (Stage 2)
        gel_dose_mJ_cm2=14.0,
        absorption_per_mm=0.32,
        scatter_length_um=2800.0,
        index=1.456,
    ),
}


@dataclass
class PrintJob:
    """A complete printable result: phases + metadata + predicted cure."""

    optics: OpticsConfig
    grid: GridConfig
    resin: ResinModel
    target_mask: object                 # (nz, ny, nx) bool on CPU numpy
    phases_rad: object                  # (K, ny, nx) float64, radians
    pattern_weights: list[float]          # relative exposure time per pattern
    exposure_s: float = 1.0               # total exposure seconds for all patterns
    rotation_angles_deg: list[float] | None = None  # None for static vat, list of angles for tomographic
    metrics: dict = field(default_factory=dict)
    meta: dict = field(default_factory=dict)

    def summary(self) -> str:
        m = self.metrics
        mode_str = f"tomographic ({len(self.rotation_angles_deg)} angles)" if self.rotation_angles_deg else "static_axial"
        lines = [
            f"HoloForge job: {self.meta.get('name', 'unnamed')}",
            f"  mode             : {mode_str}",
            f"  resin            : {self.resin.name} (gel {self.resin.gel_dose_mJ_cm2} mJ/cm2)",
            f"  patterns         : {len(self.pattern_weights)}",
            f"  exposure         : {self.exposure_s:.2f} s",
            f"  uniformity err   : {m.get('uniformity_error', float('nan')):.3f}",
            f"  bleed ratio      : {m.get('bleed_ratio', float('nan')):.4f}",
            f"  cure IoU         : {m.get('cure_iou', float('nan')):.3f}",
            f"  peak/mean inside : {m.get('peak_to_mean', float('nan')):.2f}",
        ]
        return "\n".join(lines)

    def to_json(self) -> str:
        return json.dumps(
            {
                "optics": vars(self.optics),
                "grid": {
                    **vars(self.grid),
                },
                "resin": vars(self.resin),
                "pattern_weights": self.pattern_weights,
                "exposure_s": self.exposure_s,
                "rotation_angles_deg": self.rotation_angles_deg,
                "metrics": _jsonable(self.metrics),
                "meta": _jsonable(self.meta),
            },
            indent=2,
        )


def _jsonable(obj):
    import numpy as np

    if isinstance(obj, dict):
        return {k: _jsonable(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [_jsonable(v) for v in obj]
    if isinstance(obj, (np.floating, np.integer)):
        return obj.item()
    if isinstance(obj, np.ndarray):
        return obj.tolist()
    return obj

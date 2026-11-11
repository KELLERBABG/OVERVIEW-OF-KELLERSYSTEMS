"""Forward simulation of a print job + cure prediction."""
from __future__ import annotations

import numpy as np

from .config import GridConfig, OpticsConfig, PrintJob, ResinModel
from .optics import VatPropagator
from .volume import dilate


def simulate_dose(phases_rad: np.ndarray, weights: list[float],
                  optics: OpticsConfig, grid: GridConfig,
                  resin: ResinModel,
                  angles_deg: list[float] | None = None) -> np.ndarray:
    """Total accumulated dose volume in simulation units (per unit exposure).

    If angles_deg is provided, simulates tomographic rotary accumulation around the y-axis.
    Otherwise, simulates static axial beam propagation.
    """
    prop = VatPropagator(optics, grid)
    w = np.asarray(weights, dtype=np.float64)
    w_norm = w / max(float(w.sum()), 1e-12)

    if angles_deg is not None:
        tomo_dose = prop.forward_tomographic(phases_rad, angles_deg, resin)
        return np.tensordot(w_norm, tomo_dose, axes=(0, 0))
    else:
        dose, _ = prop.forward(phases_rad, resin, keep_fields=False)
        return np.tensordot(w_norm, dose, axes=(0, 0))


def predict_cure(dose: np.ndarray, gel_level_sim: float) -> np.ndarray:
    return dose >= gel_level_sim


def gel_level_simulation_units(resin: ResinModel, optics: OpticsConfig,
                               grid: GridConfig, exposure_s: float,
                               ref_power_mW: float = 120.0, eta: float = 0.55) -> float:
    """Convert resin gel dose [mJ/cm^2] into simulation dose units.

    Physics mapping: an exposure of T seconds delivers energy density
    E = eta * P * T / area  [J/cm^2] across the vat cross-section.
    A voxel with simulation dose d receives physical dose ~ E * d/unit,
    where unit = mean(|U|^2) of the unshaped pupil (=1 inside pupil on
    average). Gel threshold therefore maps to:

        gel_level_sim = unit * D_gel / E
    """
    prop = VatPropagator(optics, grid)
    unit = float((prop.aperture() ** 2).mean())
    beam_area_mm2 = (grid.n_xy * optics.pitch_um * 1e-3) ** 2
    area_cm2 = max(beam_area_mm2 / 100.0, 1e-12)
    energy_density_J_cm2 = ref_power_mW * 1e-3 * eta * max(exposure_s, 1e-12) / area_cm2
    D_gel_J_cm2 = resin.gel_dose_mJ_cm2 * 1e-3
    return float(unit * D_gel_J_cm2 / energy_density_J_cm2)


def evaluate_job(job: PrintJob) -> dict:
    """Recompute dose for saved phases and derive all quality metrics."""
    dose = simulate_dose(job.phases_rad, job.pattern_weights,
                         job.optics, job.grid, job.resin,
                         angles_deg=job.rotation_angles_deg)
    gel = gel_level_simulation_units(job.resin, job.optics, job.grid,
                                     job.exposure_s)
    target = np.asarray(job.target_mask, dtype=bool)
    cured = predict_cure(dose, gel)

    inside = dose[target]
    mean_in = float(inside.mean()) if inside.size else 0.0
    shell = dilate(target, 2) & ~dilate(target, 1)
    bleed = float(dose[shell].mean()) if shell.any() else 0.0
    inter = float(np.logical_and(cured, target).sum())
    union = float(np.logical_or(cured, target).sum())

    far_out = ~dilate(target, 3)
    overcure = float(np.logical_and(cured, far_out).sum()) / max(float(far_out.sum()), 1.0)

    metrics = {
        "mean_inside": mean_in,
        "peak_to_mean": float(inside.max() / max(mean_in, 1e-12)) if inside.size else 0.0,
        "uniformity_error": float(inside.std() / max(mean_in, 1e-12)) if inside.size else 0.0,
        "bleed_ratio": bleed / max(mean_in, 1e-12),
        "cure_iou": inter / union if union else 0.0,
        "outside_cure_fraction": overcure,
        "gel_level_sim": gel,
    }
    return {"dose": dose, "cured": cured, "metrics": metrics}

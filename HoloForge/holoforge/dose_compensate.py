"""Iterative dose compensation for depth-dependent attenuation & scatter.

After the initial Gerchberg-Saxton optimization produces SLM phase patterns,
the achieved dose inside the target is typically non-uniform: deeper slices
receive less dose due to Beer-Lambert absorption and scatter broadening.  The
compensation loop re-weights the per-voxel target amplitudes so the optimizer
pre-emphasizes under-dosed regions, converging toward spatially uniform gel-dose
delivery.

Algorithm
---------
1. Run the base GS optimizer → initial phases + dose.
2. Compute per-voxel dose error:  err = (target_dose − achieved_dose) / target_dose.
3. Build a 3-D correction map:  correction *= (1 + gain * err), clamped.
4. Feed the corrected target amplitudes back into GS.
5. Repeat until uniformity_error < threshold or max iterations reached.

The Beer-Lambert attenuation map and Monte-Carlo-lite scatter model are
exposed as standalone utilities for analysis and visualization.

This module is the core IP described in PLAN.md Phase 0, item 2.
"""
from __future__ import annotations

import time
from dataclasses import dataclass

import numpy as np

from .config import GridConfig, OpticsConfig, ResinModel
from .optics import VatPropagator
from .optimize import MultiPlaneGS, OptimizeResult, _dose_metrics
from .simulate import simulate_dose


@dataclass
class CompensationResult:
    """Result of the iterative dose compensation procedure."""

    final_result: OptimizeResult          # the last GS optimization result
    compensation_map: np.ndarray          # (nz, ny, nx) final correction weights
    uniformity_history: list[float]       # uniformity_error per compensation iter
    iou_history: list[float]             # cure_iou per compensation iter
    comp_iterations: int                  # how many compensation loops ran
    total_elapsed_s: float                # wall time for the full procedure


def beer_lambert_attenuation_map(
    resin: ResinModel,
    grid: GridConfig,
) -> np.ndarray:
    """Depth-dependent intensity attenuation from Beer-Lambert absorption.

    Returns a (nz,) array of relative intensity at each slice depth,
    normalized so slice 0 = 1.0.  This is the expected attenuation for a
    collimated beam propagating along z through the resin.

    Physics:  I(z) = I_0 * exp(-alpha * z)
    where alpha = absorption_per_mm and z is in mm.
    """
    alpha = resin.absorption_per_mm  # 1/mm
    dz_mm = grid.slice_step_um * 1e-3
    z_mm = np.arange(grid.n_z) * dz_mm
    return np.exp(-alpha * z_mm)


def monte_carlo_lite_scatter(
    resin: ResinModel,
    optics: OpticsConfig,
    grid: GridConfig,
) -> np.ndarray:
    """Analytical forward-scatter PSF broadening per slice.

    Returns a (nz,) array of effective scatter-broadening sigma in pixels at
    each depth.  This models cumulative small-angle scattering as a Gaussian
    whose width grows with sqrt(depth), following the diffusion approximation.

    Model:  sigma(z) = sqrt(z * p / L) * (dz / p)
    where L = scatter transport length, p = pixel pitch, dz = slice step.
    The result is in pixel units for direct use as a Gaussian filter width.
    """
    L_um = resin.scatter_length_um
    if L_um <= 0:
        return np.zeros(grid.n_z)

    p_um = optics.pitch_um
    dz_um = grid.slice_step_um

    # cumulative angular diffusion: theta_rms(z) = sqrt(z_um / L_um)
    # lateral broadening: sigma_um(z) = theta_rms(z) * z_um
    # in pixels: sigma_px(z) = sigma_um(z) / p_um
    z_um = (np.arange(grid.n_z) + 1) * dz_um
    theta_rms = np.sqrt(z_um / L_um)
    sigma_um = theta_rms * dz_um  # broadening per step, accumulated
    sigma_px = sigma_um / p_um
    return sigma_px


def _build_correction_map(
    dose: np.ndarray,
    target: np.ndarray,
    gain: float,
    floor: float = 0.3,
    ceiling: float = 4.0,
) -> np.ndarray:
    """Build a 3-D per-voxel correction multiplier from dose error.

    For voxels inside the target:
        error = (mean_target_dose - local_dose) / mean_target_dose
        correction *= (1 + gain * error)

    The correction is clamped to [floor, ceiling] for stability.
    Voxels outside the target keep correction = 1.0.
    """
    correction = np.ones_like(dose)
    inside = target.astype(bool)

    if not inside.any():
        return correction

    dose_inside = dose[inside]
    mean_dose = float(dose_inside.mean())
    if mean_dose < 1e-15:
        return correction

    error = (mean_dose - dose[inside]) / mean_dose
    correction[inside] = 1.0 + gain * error
    np.clip(correction, floor, ceiling, out=correction)

    return correction


class DoseCompensator:
    """Iterative dose compensation wrapping the multi-plane GS optimizer.

    Usage::

        comp = DoseCompensator(optics, grid, resin, target_mask)
        result = comp.run(gs_iterations=40, comp_iterations=5)
        # result.final_result has the compensated phases
    """

    def __init__(
        self,
        optics: OpticsConfig,
        grid: GridConfig,
        resin: ResinModel,
        target_mask: np.ndarray,
        n_patterns: int = 8,
        seed: int = 7,
    ):
        self.optics = optics
        self.grid = grid
        self.resin = resin
        self.target = target_mask.astype(bool)
        self.n_patterns = n_patterns
        self.seed = seed

    def run(
        self,
        gs_iterations: int = 40,
        comp_iterations: int = 5,
        beta: float = 0.65,
        gain: float = 0.5,
        uniformity_threshold: float = 0.10,
        verbose: bool = True,
    ) -> CompensationResult:
        """Run the full compensation loop.

        Parameters
        ----------
        gs_iterations : int
            GS iterations per compensation round.
        comp_iterations : int
            Maximum number of dose-compensation outer loops.
        beta : float
            GS relaxation parameter.
        gain : float
            Correction gain (higher = more aggressive, risk of oscillation).
        uniformity_threshold : float
            Stop when uniformity_error drops below this.
        verbose : bool
            Print progress.

        Returns
        -------
        CompensationResult
            Contains the final optimized phases with compensation applied.
        """
        t0 = time.time()

        # Initialize correction map as uniform
        correction = np.ones(
            (self.grid.n_z, self.grid.n_xy, self.grid.n_xy),
            dtype=np.float64,
        )

        uniformity_history: list[float] = []
        iou_history: list[float] = []
        best_result: OptimizeResult | None = None

        for ci in range(comp_iterations):
            if verbose:
                print(f"\n── compensation iter {ci}/{comp_iterations} ──")

            # Build corrected target amplitudes
            target_amplitudes = self.target.astype(np.float64) * correction

            # Run GS optimizer with corrected targets
            gs = MultiPlaneGS(
                self.optics, self.grid, self.resin, self.target,
                n_patterns=self.n_patterns, seed=self.seed,
                target_amplitudes=target_amplitudes,
            )

            # Warm-start from previous result if available
            if best_result is not None:
                gs.phases = best_result.phases_rad.copy()

            result = gs.optimize(
                iterations=gs_iterations,
                beta=beta,
                verbose=verbose,
                log_every=10,
            )

            # Simulate achieved dose
            dose = simulate_dose(
                result.phases_rad, result.pattern_weights,
                self.optics, self.grid, self.resin,
            )

            # Compute gel level and metrics
            prop = VatPropagator(self.optics, self.grid)
            gel_norm = float((prop.aperture() ** 2).mean())
            metrics = _dose_metrics(dose, self.target, gel_level=gel_norm)
            ue = metrics["uniformity_error"]
            iou = metrics["cure_iou"]

            uniformity_history.append(ue)
            iou_history.append(iou)
            best_result = result

            if verbose:
                print(f"  comp iter {ci}: uniformity_error={ue:.4f}, "
                      f"cure_iou={iou:.4f}, bleed={metrics['bleed_ratio']:.4f}")

            # Check convergence
            if ue < uniformity_threshold:
                if verbose:
                    print(f"  ✓ uniformity target {uniformity_threshold} "
                          f"reached at comp iter {ci}")
                break

            # Build correction for next round
            correction = correction * _build_correction_map(
                dose, self.target, gain=gain,
            )
            # Re-normalize correction so mean inside target = 1
            inside_vals = correction[self.target]
            if inside_vals.mean() > 1e-12:
                correction /= inside_vals.mean()

        total_time = time.time() - t0

        return CompensationResult(
            final_result=best_result,
            compensation_map=correction,
            uniformity_history=uniformity_history,
            iou_history=iou_history,
            comp_iterations=len(uniformity_history),
            total_elapsed_s=total_time,
        )

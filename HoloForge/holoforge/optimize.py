"""Multi-plane Gerchberg-Saxton hologram optimizer for volumetric printing.

Time-multiplexed phase-only SLM patterns are optimized so the accumulated
dose in the resin matches the target: high and uniform inside the object,
below gel threshold everywhere else.

The volume is divided into K axial bands (one per pattern, each about one
Rayleigh range deep - the axial resolving power of the optical head). Each
pattern owns one band. Per iteration, for every pattern:

  1. propagate SLM -> resin through the pattern's band (+guard slices),
  2. REPLACE the field amplitude at in-band slices with sqrt(desired local
     dose), keeping the phase (classic GS amplitude constraint),
  3. drive guard slices just outside the band toward darkness,
  4. adjoint-walk the corrected fields back to the SLM,
  5. blend the correction into the SLM field (relaxation beta) and reapply
     the phase-only constraint.

Total-dose metrics (IoU vs target, uniformity, bleed) are tracked every
iteration; optimization stops on plateau.
"""
from __future__ import annotations

import time
from dataclasses import dataclass

import numpy as np

from .config import GridConfig, OpticsConfig, ResinModel
from .optics import VatPropagator


@dataclass
class OptimizeResult:
    phases_rad: np.ndarray            # (K, ny, nx)
    pattern_weights: list[float]      # relative exposure per pattern
    exposure_s: float                 # total exposure seconds at reference power
    metrics_history: list[dict]
    final_metrics: dict
    elapsed_s: float
    iterations: int


def _dose_metrics(dose_norm: np.ndarray, target: np.ndarray,
                  gel_level: float) -> dict:
    """Quality metrics of a normalized dose volume against a boolean target.

    ``dose_norm`` is in units where one unit of total exposure deposits
    |U|^2 <= 1 per voxel (weights sum to 1). Exposure time is a free
    parameter, so metrics are evaluated at the optimal global scale alpha
    that brings the object interior to margin*gel. Shape fidelity (IoU,
    bleed, uniformity) is exposure-independent; alpha is reported as
    'relative_exposure'.
    """
    from .volume import dilate

    inside = dose_norm[target]
    mean_in = float(inside.mean()) if inside.size else 0.0

    if inside.size == 0:
        return {
            "mean_inside": 0.0,
            "peak_to_mean": 0.0,
            "uniformity_error": 0.0,
            "bleed_ratio": 0.0,
            "cure_iou": 0.0,
            "outside_cure_fraction": 0.0,
            "relative_exposure": 0.0,
        }

    # operating point: interior mean sits at 1.3 x gel
    alpha = (1.3 * gel_level) / max(mean_in, 1e-12)
    scaled = dose_norm * alpha

    std_in = float(scaled[target].std())
    peak_in = float(scaled[target].max())

    shell = dilate(target, iterations=2) & ~dilate(target, iterations=1)
    bleed = float(scaled[shell].mean()) if shell.any() else 0.0

    cured = scaled >= gel_level
    inter = float(np.logical_and(cured, target).sum())
    union = float(np.logical_or(cured, target).sum())

    far_out = ~dilate(target, iterations=3)
    overcure = float(np.logical_and(cured, far_out).sum()) / max(float(far_out.sum()), 1.0)

    return {
        "mean_inside": float(scaled[target].mean()),
        "peak_to_mean": peak_in / max(float(scaled[target].mean()), 1e-12),
        "uniformity_error": std_in / max(float(scaled[target].mean()), 1e-12),
        "bleed_ratio": bleed / max(float(scaled[target].mean()), 1e-12),
        "cure_iou": inter / union if union else 0.0,
        "outside_cure_fraction": overcure,
        "relative_exposure": float(alpha),
    }


class MultiPlaneGS:
    """Band-decomposed multi-plane Gerchberg-Saxton for the HoloForge vat."""

    def __init__(self, optics: OpticsConfig, grid: GridConfig, resin: ResinModel,
                 target_mask: np.ndarray, n_patterns: int = 8, seed: int = 7,
                 guard_slices: int = 3, target_amplitudes: np.ndarray | None = None):
        self.optics = optics
        self.grid = grid
        self.resin = resin
        self.prop = VatPropagator(optics, grid)
        self.target = target_mask.astype(bool)
        self.K = n_patterns
        self.rng = np.random.default_rng(seed)
        nz = grid.n_z
        if self.target.shape[0] != nz:
            raise ValueError(f"target has {self.target.shape[0]} slices, grid expects {nz}")

        # --- bands ------------------------------------------------------
        edges = np.linspace(0, nz, self.K + 1).astype(int)
        self.band_edges = list(zip(edges[:-1], edges[1:]))
        occ = [int(self.target[z].sum()) for z in range(nz)]
        mass = np.array([sum(occ[a:b]) for a, b in self.band_edges], dtype=np.float64)
        if mass.sum() <= 0:
            raise ValueError("empty target")
        self.weights = mass / mass.sum()
        self.guard = guard_slices

        # --- per-voxel target amplitudes (dose compensation support) ------
        # When target_amplitudes is provided by the dose compensator, it is a
        # (nz, ny, nx) float array with per-voxel desired relative dose levels
        # (higher values = optimizer works harder to deliver dose there).
        # When None, fall back to the binary target mask.
        if target_amplitudes is not None:
            self.target_amps = target_amplitudes.astype(np.float64)
        else:
            self.target_amps = self.target.astype(np.float64)

        # --- desired levels ---------------------------------------------
        self._gel_norm_cache: float | None = None
        self.d_star = 1.30 * self._gel_norm()   # drive slightly above gel

        # --- state -------------------------------------------------------
        self.phases = self._init_phases()

    def _init_phases(self) -> np.ndarray:
        n = self.grid.n_xy
        ph = self.rng.random((self.K, n, n)) * 2 * np.pi - np.pi
        fh = np.fft.fft2(ph, axes=(1, 2))
        fx = np.fft.fftfreq(n)
        KY, KX = np.meshgrid(fx, fx, indexing="ij")
        lp = np.exp(-6.0 * (KX ** 2 + KY ** 2))
        return np.angle(np.fft.ifft2(fh * lp[None], axes=(1, 2)))

    def aperture_safe(self) -> np.ndarray:
        return self.prop.aperture()[None]

    def _gel_norm(self) -> float:
        """Simulation |U|^2 level corresponding to an unshaped pupil beam."""
        if self._gel_norm_cache is None:
            self._gel_norm_cache = float((self.prop.aperture() ** 2).mean())
        return self._gel_norm_cache

    def total_dose(self) -> np.ndarray:
        dose, _ = self.prop.forward(self.phases, self.resin, keep_fields=False)
        return np.tensordot(self.weights, dose, axes=(0, 0))

    # ------------------------------------------------------------------
    def step(self, beta: float = 0.65) -> dict:
        """One MP-GS sweep over all patterns; returns total-dose metrics.

        Amplitude replacement uses scale-free per-slice projection: the
        replaced profile is the *shape* sqrt(target share), scaled by the
        least-squares factor of the achieved field, so the energy budget
        never fights the phase-only constraint.
        """
        tgt_unit = self.target_amps              # compensated amplitude profile

        for k in range(self.K):
            z_lo, z_hi = self.band_edges[k]
            g_lo = max(z_lo - self.guard, 0)
            g_hi = min(z_hi + self.guard, self.grid.n_z)

            fields = self.prop.forward_band(self.phases[k], self.resin, g_lo, g_hi)

            wk = max(float(self.weights[k]), 1e-6)
            des_shape = np.sqrt(wk * tgt_unit)      # desired amplitude profile
            for zi in range(g_lo, g_hi):
                U = fields[zi]
                if z_lo <= zi < z_hi:
                    des = des_shape[zi]
                    norm2 = float(np.vdot(des, des).real)
                    if norm2 > 0:
                        s = float(np.vdot(des, U).real) / norm2  # LS scale
                        new_amp = abs(s) * des
                    else:
                        new_amp = np.abs(U)
                    phase = np.angle(U)
                    fields[zi] = new_amp * np.exp(1j * phase)
                else:
                    # guard slices: suppress leakage toward darkness
                    fields[zi] = np.abs(U) * 0.25 * np.exp(1j * np.angle(U))

            corrected = self.prop.backward_band(fields, g_lo, g_hi, self.resin)

            # relaxed blend + phase-only constraint
            cur = np.exp(1j * self.phases[k]) * self.prop.aperture()
            blended = (1.0 - beta) * cur + beta * (corrected + 1e-9)
            self.phases[k] = np.angle(blended)

        dose = self.total_dose()
        return _dose_metrics(dose, self.target, gel_level=self._gel_norm())

    # ------------------------------------------------------------------
    def optimize(self, iterations: int = 40, beta: float = 0.65,
                 verbose: bool = True, log_every: int = 5) -> OptimizeResult:
        t0 = time.time()
        hist: list[dict] = []
        best_iou, stagnant, it_done = -1.0, 0, 0
        for it in range(iterations):
            m = self.step(beta)
            hist.append(m)
            it_done = it + 1
            if verbose and (it % log_every == 0 or it == iterations - 1):
                print(f"  iter {it:3d}  IoU {m['cure_iou']:.3f}  "
                      f"unif {m['uniformity_error']:.3f}  "
                      f"bleed {m['bleed_ratio']:.4f}  "
                      f"peak/mean {m['peak_to_mean']:.2f}")
            if m["cure_iou"] > best_iou + 5e-4:
                best_iou, stagnant = m["cure_iou"], 0
            else:
                stagnant += 1
                if stagnant >= 15:
                    if verbose:
                        print(f"  early stop at iter {it} (plateau)")
                    break

        dose = self.total_dose()
        final = _dose_metrics(dose, self.target, gel_level=self._gel_norm())
        exposure_s = self._estimate_exposure_s(final["relative_exposure"])

        return OptimizeResult(
            phases_rad=self.phases.copy(),
            pattern_weights=self.weights.tolist(),
            exposure_s=float(exposure_s),
            metrics_history=hist,
            final_metrics=final,
            elapsed_s=time.time() - t0,
            iterations=it_done,
        )

    def _estimate_exposure_s(self, mean_inside_sim: float) -> float:
        """Exposure seconds so the mean in-object dose equals the gel point."""
        D_gel_cgs = self.resin.gel_dose_mJ_cm2 * 1e-3          # J/cm^2
        ref_power_mW, eta = 120.0, 0.55                          # MVP defaults
        area_mm2 = (self.grid.n_xy * self.optics.pitch_um * 1e-3) ** 2
        area_cm2 = max(area_mm2 / 100.0, 1e-12)
        rate_J_cm2_s = (ref_power_mW * 1e-3 * eta) / area_cm2   # full-pupil rate
        frac_inside = mean_inside_sim / max(self._gel_norm(), 1e-12)
        t = D_gel_cgs / max(rate_J_cm2_s * frac_inside, 1e-12)
        return float(np.clip(t, 0.05, 900.0))


@dataclass(frozen=True)
class OptimizerConfig:
    n_patterns: int = 8
    iterations: int = 40
    beta: float = 0.65
    seed: int = 7


class TomographicHolographicOptimizer:
    """Holographic Tomographic Volumetric Additive Manufacturing (Tomo-H-VAM) Optimizer.

    Combines multi-angle rotary projection around the vertical cylinder (y) axis with
    coherent double-phase SLM wavefront shaping.
    For K projection angles theta_k in [0, 360 deg):
      1. Rotates target volume by theta_k and extracts line-integral / visual hull projection profile.
      2. Synthesizes phase hologram phi_k(y, x) using double-phase spatial carrier encoding.
      3. Propagates through the 4f telecentric relay and vat via forward_tomographic().
      4. Iteratively refines projection profiles using closed-loop dose error feedback.
    """

    def __init__(self, optics: OpticsConfig, grid: GridConfig, resin: ResinModel,
                 target_mask: np.ndarray, n_patterns: int = 16,
                 angles_deg: list[float] | None = None, seed: int = 7,
                 target_amplitudes: np.ndarray | None = None,
                 vat_radius_mm: float | None = None,
                 device: str = "auto"):
        self.optics = optics
        self.grid = grid
        self.resin = resin
        self.device = device
        self.prop = VatPropagator(optics, grid, device=device)
        self.target = target_mask.astype(bool)
        self.K = n_patterns
        self.rng = np.random.default_rng(seed)
        self.vat_radius_mm = vat_radius_mm

        if vat_radius_mm is not None and vat_radius_mm > 0.0:
            from .optics import cylindrical_vat_phase_correction
            self.aberration_comp = cylindrical_vat_phase_correction(
                self.optics, self.grid, vat_radius_mm=vat_radius_mm, vat_index=self.resin.index
            )
        else:
            self.aberration_comp = np.zeros((grid.n_xy, grid.n_xy), dtype=np.float64)

        if angles_deg is not None:
            self.angles_deg = [float(a) for a in angles_deg]
            self.K = len(self.angles_deg)
        else:
            self.angles_deg = [float(a) for a in np.linspace(0.0, 360.0, self.K, endpoint=False)]

        self.weights = np.ones(self.K, dtype=np.float64) / max(self.K, 1)
        self.phases = np.zeros((self.K, grid.n_xy, grid.n_xy), dtype=np.float64)
        self.projections = [None] * self.K

        self._init_tomographic_phases()

    def _init_tomographic_phases(self) -> None:
        """Extract visual-hull / line-integral projection and encode into SLM phase."""
        n = self.grid.n_xy
        checker = (((np.arange(n)[:, None] + np.arange(n)[None, :]) % 2) == 0)
        target_f = self.target.astype(np.float64)

        for k, deg in enumerate(self.angles_deg):
            v_rot = self.prop.backend.rotate_volume_y(target_f, deg, order=1)
            # Thresholded line integral projection (silhouette visual hull)
            proj = (v_rot > 0.25).any(axis=0).astype(np.float64)
            self.projections[k] = proj
            A = np.clip(proj, 0.0, 1.0)
            theta = np.arccos(A)
            base_phase = np.where(checker, theta, -theta)
            self.phases[k] = np.angle(np.exp(1j * (base_phase + self.aberration_comp)))

    def total_dose(self) -> np.ndarray:
        tomo_dose = self.prop.forward_tomographic(self.phases, self.angles_deg, self.resin)
        return np.tensordot(self.weights, tomo_dose, axes=(0, 0))

    def step(self, beta: float = 0.3) -> dict:
        """One iteration of tomographic closed-loop dose error correction."""
        dose = self.total_dose()
        n = self.grid.n_xy
        checker = (((np.arange(n)[:, None] + np.arange(n)[None, :]) % 2) == 0)

        # Operating point & error calculation
        inside = dose[self.target]
        mean_in = float(inside.mean()) if inside.size else 1.0
        gel = self._gel_norm()

        alpha = (1.3 * gel) / max(mean_in, 1e-12)
        scaled_dose = dose * alpha
        target_val = 1.3 * gel

        err = np.zeros_like(dose)
        err[self.target] = np.clip(target_val - scaled_dose[self.target], -target_val, target_val)
        err[~self.target] = np.clip(0.0 - scaled_dose[~self.target], -target_val, 0.0)

        for k, deg in enumerate(self.angles_deg):
            err_rot = self.prop.backend.rotate_volume_y(err, deg, order=1)
            err_proj = err_rot.mean(axis=0) / max(target_val, 1e-12)
            self.projections[k] = np.clip(self.projections[k] + beta * err_proj, 0.0, 1.0)
            theta = np.arccos(self.projections[k])
            base_phase = np.where(checker, theta, -theta)
            self.phases[k] = np.angle(np.exp(1j * (base_phase + self.aberration_comp)))

        dose = self.total_dose()
        return _dose_metrics(dose, self.target, gel_level=self._gel_norm())

    def _gel_norm(self) -> float:
        return float((self.prop.aperture() ** 2).mean())

    def optimize(self, iterations: int = 10, beta: float = 0.3,
                 verbose: bool = True, log_every: int = 2) -> OptimizeResult:
        t0 = time.time()
        hist: list[dict] = []
        best_iou, stagnant, it_done = -1.0, 0, 0

        dose = self.total_dose()
        m0 = _dose_metrics(dose, self.target, gel_level=self._gel_norm())
        hist.append(m0)
        best_iou = m0["cure_iou"]
        best_phases = self.phases.copy()
        best_metrics = m0
        if verbose:
            print(f"  init iter   0  IoU {m0['cure_iou']:.3f}  "
                  f"unif {m0['uniformity_error']:.3f}  "
                  f"bleed {m0['bleed_ratio']:.4f}  "
                  f"peak/mean {m0['peak_to_mean']:.2f}")

        for it in range(iterations):
            m = self.step(beta)
            hist.append(m)
            it_done = it + 1
            if m["cure_iou"] > best_iou:
                best_iou = m["cure_iou"]
                best_phases = self.phases.copy()
                best_metrics = m
                stagnant = 0
            else:
                stagnant += 1

            if verbose and (it % log_every == 0 or it == iterations - 1):
                print(f"  iter {it+1:3d}  IoU {m['cure_iou']:.3f}  "
                      f"unif {m['uniformity_error']:.3f}  "
                      f"bleed {m['bleed_ratio']:.4f}  "
                      f"peak/mean {m['peak_to_mean']:.2f}")

            if stagnant >= 3:
                if verbose:
                    print(f"  early stop at iter {it+1} (plateau)")
                break

        self.phases = best_phases.copy()
        final = best_metrics
        exposure_s = self._estimate_exposure_s(final["relative_exposure"])

        return OptimizeResult(
            phases_rad=best_phases,
            pattern_weights=self.weights.tolist(),
            exposure_s=float(exposure_s),
            metrics_history=hist,
            final_metrics=final,
            elapsed_s=time.time() - t0,
            iterations=it_done,
        )

    def _estimate_exposure_s(self, relative_exposure: float) -> float:
        D_gel_cgs = self.resin.gel_dose_mJ_cm2 * 1e-3
        ref_power_mW, eta = 120.0, 0.55
        area_mm2 = (self.grid.n_xy * self.optics.pitch_um * 1e-3) ** 2
        area_cm2 = max(area_mm2 / 100.0, 1e-12)
        rate_J_cm2_s = (ref_power_mW * 1e-3 * eta) / area_cm2
        t = float(relative_exposure * (D_gel_cgs / max(rate_J_cm2_s, 1e-12)))
        return float(np.clip(t, 1e-6, 900.0))


# backwards-compatible alias
WavefrontMatchedCGH = MultiPlaneGS

from dataclasses import dataclass
from typing import List, Optional
import numpy as np

from .synthetic_scene import VoxelScene
from ..physics.scattering import sample_moliere_scattering
from ..physics.energy_loss import bethe_bloch_stopping_power

@dataclass
class MuonTrackEvent:
    event_id: int
    energy_initial_gev: float
    charge: int
    p_in: np.ndarray
    d_in: np.ndarray
    p_out: np.ndarray
    d_out: np.ndarray
    scattering_angle_rad: float
    absorbed: bool = False
    stopped_in_scene: bool = False

class MonteCarloRayTracer:
    def __init__(
        self,
        scene: VoxelScene,
        top_panel_z_mm: float = 300.0,
        bottom_panel_z_mm: float = -300.0,
        detector_half_width_mm: float = 250.0,
        step_size_mm: float = 5.0,
    ):
        self.scene = scene
        self.top_z = top_panel_z_mm
        self.bottom_z = bottom_panel_z_mm
        self.half_width = detector_half_width_mm
        self.step_size_mm = step_size_mm

    def trace_muon(
        self,
        event_id: int,
        p0_mm: np.ndarray,
        d0: np.ndarray,
        energy_gev: float,
        charge: int = 1,
        rng: np.random.Generator = None,
    ) -> Optional[MuonTrackEvent]:
        events = self.trace_batch(1, np.array([energy_gev]), d0[np.newaxis, :], np.array([charge]), rng=rng, p0_fixed=p0_mm[np.newaxis, :])
        return events[0] if len(events) > 0 else None

    def trace_batch(
        self,
        num_events: int,
        energies_gev: np.ndarray,
        directions: np.ndarray,
        charges: np.ndarray,
        rng: np.random.Generator = None,
        p0_fixed: Optional[np.ndarray] = None,
    ) -> List[MuonTrackEvent]:
        if rng is None:
            rng = np.random.default_rng()

        N = num_events
        if p0_fixed is not None:
            pos = p0_fixed.copy().astype(np.float64)
        else:
            pos = np.zeros((N, 3), dtype=np.float64)
            pos[:, 0] = rng.uniform(-self.half_width, self.half_width, size=N)
            pos[:, 1] = rng.uniform(-self.half_width, self.half_width, size=N)
            pos[:, 2] = self.top_z

        dir = directions.copy().astype(np.float64)
        dir_norm = np.linalg.norm(dir, axis=1, keepdims=True)
        dir = dir / np.maximum(dir_norm, 1e-9)

        # Force downward component
        dir[dir[:, 2] >= 0.0, 2] = -0.01
        dir = dir / np.linalg.norm(dir, axis=1, keepdims=True)

        p_in = pos.copy()
        d_in = dir.copy()
        energy_mev = (energies_gev * 1000.0).copy().astype(np.float64)

        b_min = np.array(self.scene.bounds_min_mm, dtype=np.float64)
        b_max = np.array(self.scene.bounds_max_mm, dtype=np.float64)
        grid_size = np.array(self.scene.grid_size, dtype=np.int32)
        voxel_size = np.array(self.scene.voxel_size_mm, dtype=np.float64)

        active = np.ones(N, dtype=bool)
        step_cm = self.step_size_mm / 10.0

        max_steps = int(np.ceil((self.top_z - self.bottom_z) / self.step_size_mm) * 2)

        for _ in range(max_steps):
            active_indices = np.where(active)[0]
            if len(active_indices) == 0:
                break

            curr_pos = pos[active_indices]
            curr_dir = dir[active_indices]
            curr_energy = energy_mev[active_indices]

            # In bounds check
            in_b = (
                (curr_pos[:, 0] >= b_min[0]) & (curr_pos[:, 0] <= b_max[0]) &
                (curr_pos[:, 1] >= b_min[1]) & (curr_pos[:, 1] <= b_max[1]) &
                (curr_pos[:, 2] >= b_min[2]) & (curr_pos[:, 2] <= b_max[2])
            )

            if np.any(in_b):
                in_b_idx = active_indices[in_b]
                v_idx = np.floor((pos[in_b_idx] - b_min) / voxel_size).astype(int)
                v_idx[:, 0] = np.clip(v_idx[:, 0], 0, grid_size[0] - 1)
                v_idx[:, 1] = np.clip(v_idx[:, 1], 0, grid_size[1] - 1)
                v_idx[:, 2] = np.clip(v_idx[:, 2], 0, grid_size[2] - 1)

                rad_len_cm = self.scene.rad_length_grid_cm[v_idx[:, 0], v_idx[:, 1], v_idx[:, 2]]
                density = self.scene.density_grid_g_cm3[v_idx[:, 0], v_idx[:, 1], v_idx[:, 2]]
                z_mat = self.scene.z_eff_grid[v_idx[:, 0], v_idx[:, 1], v_idx[:, 2]]

                p_mev = np.sqrt(np.maximum(energy_mev[in_b_idx]**2 - 105.66**2, 1.0))
                beta = p_mev / np.maximum(energy_mev[in_b_idx], 1.0)
                x_over_x0 = step_cm / np.maximum(rad_len_cm, 1e-4)

                theta0 = (13.6 / np.maximum(p_mev * beta, 1e-3)) * np.sqrt(x_over_x0) * (1.0 + 0.038 * np.log(np.maximum(x_over_x0, 1e-6)))
                theta0 = np.clip(theta0, 0.0, 0.5)

                tx = rng.normal(0.0, theta0)
                ty = rng.normal(0.0, theta0)

                dir[in_b_idx, 0] += tx
                dir[in_b_idx, 1] += ty
                dir[in_b_idx] = dir[in_b_idx] / np.linalg.norm(dir[in_b_idx], axis=1, keepdims=True)

                # Bethe-Bloch loss
                gamma = energy_mev[in_b_idx] / 105.66
                beta_sq = np.maximum(1.0 - 1.0 / (gamma**2), 1e-6)
                t_max = (2.0 * 0.511 * beta_sq * gamma**2) / (1.0 + 2.0 * gamma * (0.511 / 105.66) + (0.511 / 105.66)**2)
                i_pot = 16.0 * (z_mat**0.9) * 1e-6
                ln_term = np.log(np.maximum((2.0 * 0.511 * beta_sq * gamma**2 * t_max) / (i_pot**2), 1.0))
                dedx = 0.307075 * (z_mat / (z_mat * 2.0)) * (1.0 / beta_sq) * (0.5 * ln_term - beta_sq) * density
                dedx = np.maximum(dedx, 0.0)

                energy_mev[in_b_idx] -= dedx * step_cm

            # Step propagation
            pos[active_indices] += dir[active_indices] * self.step_size_mm

            # Stop conditions
            stopped = energy_mev[active_indices] <= 10.0
            past_bottom = pos[active_indices, 2] <= self.bottom_z

            active[active_indices[stopped | past_bottom]] = False

        # Calculate exit points at bottom detector plane
        t_bottom = (self.bottom_z - pos[:, 2]) / dir[:, 2]
        p_out = pos + t_bottom[:, np.newaxis] * dir
        d_out = dir.copy()

        # Geometric acceptance filter
        valid_mask = (
            (energy_mev > 10.0) &
            (np.abs(p_out[:, 0]) <= self.half_width) &
            (np.abs(p_out[:, 1]) <= self.half_width)
        )

        valid_events = []
        valid_indices = np.where(valid_mask)[0]

        for idx in valid_indices:
            cos_th = np.clip(np.dot(d_in[idx], d_out[idx]) / (np.linalg.norm(d_in[idx]) * np.linalg.norm(d_out[idx])), -1.0, 1.0)
            theta = float(np.arccos(cos_th))
            valid_events.append(MuonTrackEvent(
                event_id=int(idx),
                energy_initial_gev=float(energies_gev[idx]),
                charge=int(charges[idx]),
                p_in=p_in[idx],
                d_in=d_in[idx],
                p_out=p_out[idx],
                d_out=d_out[idx],
                scattering_angle_rad=theta,
                stopped_in_scene=False
            ))

        return valid_events

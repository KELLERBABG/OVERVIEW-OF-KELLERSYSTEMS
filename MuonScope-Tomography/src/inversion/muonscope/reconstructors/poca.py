from typing import Tuple, Optional, List
import numpy as np

def theta2_to_lambda(theta2_rad2: float, momentum_mev: float, path_cm: float, beta: float = 1.0) -> float:
    """First-order inversion of the Highland formula.

        theta0 = (13.6 MeV / (beta p)) * sqrt(x / X0) * (1 + 0.038 ln(x / X0))

    Squaring and solving for lambda = 1 / X0, neglecting the logarithmic term
    (which is under 5% for x / X0 of order one, the regime one voxel sits in):

        lambda ~= theta^2 * (beta p / 13.6)^2 / x

    Dominant remaining error: PoCA attributes a muon's whole-path scattering to a
    single voxel rather than the path length actually inside it, so a small
    inclusion is biased low and the background high. That is what the Siddon
    matrix and MAP-EM exist to correct.
    """
    if path_cm <= 0 or momentum_mev <= 0:
        return 0.0
    k = (beta * momentum_mev / 13.6) ** 2
    return float(theta2_rad2 * k / path_cm)


def compute_poca_point(
    p_in: np.ndarray,
    d_in: np.ndarray,
    p_out: np.ndarray,
    d_out: np.ndarray,
) -> Tuple[np.ndarray, float, float]:
    w0 = p_in - p_out
    a = float(np.dot(d_in, d_in))
    b = float(np.dot(d_in, d_out))
    c = float(np.dot(d_out, d_out))
    d = float(np.dot(d_in, w0))
    e = float(np.dot(d_out, w0))

    denom = a * c - (b ** 2)
    if abs(denom) < 1e-9:
        poca_point = 0.5 * (p_in + p_out)
        miss_dist = float(np.linalg.norm(w0 - np.dot(w0, d_in) * d_in))
        cos_th = np.clip(np.dot(d_in, d_out) / (np.linalg.norm(d_in) * np.linalg.norm(d_out)), -1.0, 1.0)
        return poca_point, miss_dist, float(np.arccos(cos_th))

    t_min = (b * e - c * d) / denom
    s_min = (a * e - b * d) / denom

    point_in = p_in + t_min * d_in
    point_out = p_out + s_min * d_out

    poca_point = 0.5 * (point_in + point_out)
    miss_dist = float(np.linalg.norm(point_in - point_out))

    cos_th = np.clip(np.dot(d_in, d_out) / (np.linalg.norm(d_in) * np.linalg.norm(d_out)), -1.0, 1.0)
    theta = float(np.arccos(cos_th))

    return poca_point, miss_dist, theta

class PoCAReconstructor:
    def __init__(
        self,
        grid_size: Tuple[int, int, int] = (64, 64, 64),
        bounds_min_mm: Tuple[float, float, float] = (-250.0, -250.0, -200.0),
        bounds_max_mm: Tuple[float, float, float] = (250.0, 250.0, 200.0),
        min_angle_mrad: float = 1.0,
        max_miss_dist_mm: float = 30.0,
        default_momentum_mev: float = 3000.0,
    ):
        self.nx, self.ny, self.nz = grid_size
        self.xmin, self.ymin, self.zmin = bounds_min_mm
        self.xmax, self.ymax, self.zmax = bounds_max_mm
        self.min_angle_rad = min_angle_mrad * 1e-3
        self.max_miss_dist = max_miss_dist_mm
        self.default_momentum_mev = float(default_momentum_mev)

        # mean path length a muon spends inside one voxel, in cm - the x in Highland
        dx = (self.xmax - self.xmin) / max(self.nx, 1)
        dy = (self.ymax - self.ymin) / max(self.ny, 1)
        dz = (self.zmax - self.zmin) / max(self.nz, 1)
        self.voxel_path_cm = float((dx + dy + dz) / 3.0 / 10.0)

        self.density_grid = np.zeros((self.nx, self.ny, self.nz), dtype=np.float32)
        self.hit_count_grid = np.zeros((self.nx, self.ny, self.nz), dtype=np.int32)
        self.theta2_grid = np.zeros((self.nx, self.ny, self.nz), dtype=np.float64)
        self.momentum_grid = np.zeros((self.nx, self.ny, self.nz), dtype=np.float64)

    def calculate_poca_intersection(
        self,
        p_in: np.ndarray,
        d_in: np.ndarray,
        p_out: np.ndarray,
        d_out: np.ndarray,
    ) -> Tuple[np.ndarray, float, float]:
        return compute_poca_point(p_in, d_in, p_out, d_out)

    def process_event(
        self,
        p_in: np.ndarray,
        d_in: np.ndarray,
        p_out: np.ndarray,
        d_out: np.ndarray,
        momentum_mev: Optional[float] = None,
    ) -> bool:
        poca, miss_dist, theta = compute_poca_point(p_in, d_in, p_out, d_out)

        if miss_dist > self.max_miss_dist or theta < self.min_angle_rad:
            return False

        if not (self.xmin <= poca[0] <= self.xmax and self.ymin <= poca[1] <= self.ymax and self.zmin <= poca[2] <= self.zmax):
            return False

        gx = (poca[0] - self.xmin) / max(self.xmax - self.xmin, 1e-6) * self.nx
        gy = (poca[1] - self.ymin) / max(self.ymax - self.ymin, 1e-6) * self.ny
        gz = (poca[2] - self.zmin) / max(self.zmax - self.zmin, 1e-6) * self.nz

        ix = min(int(gx), self.nx - 1)
        iy = min(int(gy), self.ny - 1)
        iz = min(int(gz), self.nz - 1)

        # unweighted mean squared angle, plus the mean momentum, so the conversion
        # to scattering density has something physical to work with
        p_used = momentum_mev if momentum_mev is not None else self.default_momentum_mev
        self.theta2_grid[ix, iy, iz] += theta ** 2
        self.momentum_grid[ix, iy, iz] += p_used
        self.hit_count_grid[ix, iy, iz] += 1

        hits = max(self.hit_count_grid[ix, iy, iz], 1)
        theta2_mean = self.theta2_grid[ix, iy, iz] / hits
        p_mean = self.momentum_grid[ix, iy, iz] / hits
        self.density_grid[ix, iy, iz] = theta2_to_lambda(theta2_mean, p_mean, self.voxel_path_cm)
        return True

    def reconstruct(self, events: list) -> np.ndarray:
        for e in events:
            self.process_event(e.p_in, e.d_in, e.p_out, e.d_out)
        return self.get_reconstructed_density()

    def process_batch(
        self,
        p_in_arr: np.ndarray,
        d_in_arr: np.ndarray,
        p_out_arr: np.ndarray,
        d_out_arr: np.ndarray,
    ) -> int:
        n_events = len(p_in_arr)
        accepted = 0
        for i in range(n_events):
            if self.process_event(p_in_arr[i], d_in_arr[i], p_out_arr[i], d_out_arr[i]):
                accepted += 1
        return accepted

    def get_reconstructed_density(self) -> np.ndarray:
        """Reconstructed scattering density lambda in cm^-1 (was mean rad^2)."""
        return self.density_grid.copy()

    def get_scattering_angle_grid(self) -> np.ndarray:
        """Mean squared scattering angle per voxel, rad^2, before conversion."""
        hits = np.maximum(self.hit_count_grid, 1)
        return (self.theta2_grid / hits).astype(np.float32)

from typing import Tuple, Optional
import numpy as np
import scipy.sparse as sp

GRAVITATIONAL_CONSTANT = 6.67430e-11

def compute_gravitational_sensitivity_matrix(
    sensor_positions_m: np.ndarray,
    voxel_centers_m: np.ndarray,
    voxel_volumes_m3: np.ndarray,
) -> np.ndarray:
    n_sensors = len(sensor_positions_m)
    n_voxels = len(voxel_centers_m)
    g_mat = np.zeros((n_sensors, n_voxels), dtype=np.float32)

    for i in range(n_sensors):
        diff = sensor_positions_m[i] - voxel_centers_m
        r = np.maximum(np.linalg.norm(diff, axis=1), 0.05)
        g_mat[i, :] = GRAVITATIONAL_CONSTANT * voxel_volumes_m3 * (diff[:, 2] / (r**3)) * 1e5

    return g_mat

class JointMuonGravimetryInverter:
    def __init__(
        self,
        grid_size: Tuple[int, int, int] = (16, 16, 16),
        muon_weight_alpha: float = 1.0,
        grav_weight_beta: float = 0.2,
        tv_weight_gamma: float = 0.05,
        max_iterations: int = 50,
        alpha_grav: Optional[float] = None,
    ):
        self.grid_size = grid_size
        self.alpha = muon_weight_alpha
        self.beta = alpha_grav if alpha_grav is not None else grav_weight_beta
        self.gamma = tv_weight_gamma
        self.max_iterations = max_iterations

    def solve(
        self,
        muon_matrix,
        muon_data: np.ndarray,
        grav_data: Optional[np.ndarray] = None,
        grav_matrix: Optional[np.ndarray] = None,
        num_iters: Optional[int] = None,
        learning_rate: float = 1e-3,
    ) -> np.ndarray:
        nx, ny, nz = self.grid_size
        n_voxels = nx * ny * nz

        if not sp.issparse(muon_matrix):
            muon_matrix = sp.csr_matrix(muon_matrix)

        rho = np.full(n_voxels, 2.35, dtype=np.float32)
        iters = num_iters if num_iters is not None else self.max_iterations

        if grav_matrix is None or grav_data is None:
            grav_matrix = np.zeros((1, n_voxels), dtype=np.float32)
            grav_data = np.zeros(1, dtype=np.float32)

        for _ in range(iters):
            grad_mu = muon_matrix.T.dot(muon_matrix.dot(rho) - muon_data)
            grad_grav = grav_matrix.T.dot(grav_matrix.dot(rho) - grav_data)

            total_grad = self.alpha * grad_mu + self.beta * grad_grav
            rho = np.maximum(rho - learning_rate * total_grad, 0.001)

        return rho.reshape(self.grid_size)

JointMuonGravSolver = JointMuonGravimetryInverter

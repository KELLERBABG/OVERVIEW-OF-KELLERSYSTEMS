from typing import Tuple, Optional
import numpy as np
import scipy.sparse as sp

try:
    import torch
    TORCH_AVAILABLE = True
except ImportError:
    TORCH_AVAILABLE = False

def huber_regularization_gradient_np(image: np.ndarray, delta: float = 0.01) -> np.ndarray:
    grad = np.zeros_like(image)
    for dim in [0, 1, 2]:
        diff_fwd = np.roll(image, shift=-1, axis=dim) - image
        diff_bwd = image - np.roll(image, shift=1, axis=dim)
        huber_fwd = np.where(np.abs(diff_fwd) <= delta, diff_fwd, np.sign(diff_fwd))
        huber_bwd = np.where(np.abs(diff_bwd) <= delta, diff_bwd, np.sign(diff_bwd))
        grad += (huber_fwd - huber_bwd)
    return grad

huber_regularization_gradient = huber_regularization_gradient_np

def huber_regularization_gradient_torch(image, delta: float = 0.01):
    grad = torch.zeros_like(image)
    for dim in [0, 1, 2]:
        diff_fwd = torch.roll(image, shifts=-1, dims=dim) - image
        diff_bwd = image - torch.roll(image, shifts=1, dims=dim)
        huber_fwd = torch.where(torch.abs(diff_fwd) <= delta, diff_fwd, torch.sign(diff_fwd))
        huber_bwd = torch.where(torch.abs(diff_bwd) <= delta, diff_bwd, torch.sign(diff_bwd))
        grad = grad + (huber_fwd - huber_bwd)
    return grad

class MAPEMReconstructor:
    def __init__(
        self,
        grid_size: Tuple[int, int, int] = (64, 64, 64),
        beta: float = 0.05,
        huber_delta: float = 0.01,
        max_iterations: int = 50,
        device: Optional[str] = None,
        use_torch: bool = True,
    ):
        self.grid_size = grid_size
        self.beta = beta
        self.huber_delta = huber_delta
        self.max_iterations = max_iterations
        self.use_torch = use_torch and TORCH_AVAILABLE
        self.device = device

    def reconstruct(
        self,
        system_matrix_csr: sp.csr_matrix,
        squared_angles_rad2: np.ndarray,
        initial_density: Optional[np.ndarray] = None,
    ) -> np.ndarray:
        """Maximum a posteriori EM reconstruction of scattering density.

        The forward model is  W lambda = theta^2 / K  with K = (13.6 / (beta p))^2,
        so ``squared_angles_rad2`` must be the **momentum-normalised** quantity
        y_i = theta_i^2 / K_i, not the raw squared angle. Raw theta^2 is larger by
        K, which is 1.4e-4 at 1.2 GeV - feeding it in raw scales the whole
        reconstruction by 1/K. Voxel units come out in cm^-1 when W is in cm.

        The prior enters the denominator, which is the one-step-late (Green 1990)
        form for a MAP-EM, not an error. beta = 0 reduces this to plain MLEM.
        """
        if self.use_torch:
            return self._reconstruct_torch(system_matrix_csr, squared_angles_rad2, initial_density)
        return self._reconstruct_numpy(system_matrix_csr, squared_angles_rad2, initial_density)

    def _reconstruct_numpy(
        self,
        system_matrix_csr: sp.csr_matrix,
        squared_angles_rad2: np.ndarray,
        initial_density: Optional[np.ndarray],
    ) -> np.ndarray:
        n_rays, n_voxels = system_matrix_csr.shape
        nx, ny, nz = self.grid_size

        if initial_density is None:
            lam = np.full(n_voxels, 0.0865, dtype=np.float32)
        else:
            lam = initial_density.flatten().astype(np.float32)

        w_trans = system_matrix_csr.T
        # sum_i W[i,v]: the MLEM denominator. The old form weighted this
        # by 1/pred, which is not the EM step here and diverged ~25x per iteration.
        sensitivity = np.asarray(
            w_trans.dot(np.ones(system_matrix_csr.shape[0], dtype=np.float32))).ravel()

        for _ in range(self.max_iterations):
            pred_var = np.maximum(system_matrix_csr.dot(lam), 1e-6)
            ratio = squared_angles_rad2 / pred_var
            backproj = w_trans.dot(ratio)

            lam_3d = lam.reshape(nx, ny, nz)
            prior_grad = huber_regularization_gradient_np(lam_3d, delta=self.huber_delta).flatten()

            denom = np.maximum(sensitivity - self.beta * prior_grad,
                                   np.maximum(0.1 * sensitivity, 1e-8))
            lam = np.maximum(lam * (backproj / denom), 1e-6)

        return lam.reshape(nx, ny, nz)

    def _reconstruct_torch(
        self,
        system_matrix_csr: sp.csr_matrix,
        squared_angles_rad2: np.ndarray,
        initial_density: Optional[np.ndarray],
    ) -> np.ndarray:
        dev = torch.device(self.device if self.device else ("cuda" if torch.cuda.is_available() else "cpu"))
        nx, ny, nz = self.grid_size
        n_voxels = nx * ny * nz

        coo = system_matrix_csr.tocoo()
        indices = torch.tensor(np.vstack((coo.row, coo.col)), dtype=torch.long, device=dev)
        values = torch.tensor(coo.data, dtype=torch.float32, device=dev)
        w_torch = torch.sparse_coo_tensor(indices, values, torch.Size(coo.shape), device=dev)

        y_meas = torch.tensor(squared_angles_rad2, dtype=torch.float32, device=dev)

        if initial_density is None:
            lam = torch.full((n_voxels, 1), 0.0865, dtype=torch.float32, device=dev)
        else:
            lam = torch.tensor(initial_density.reshape(-1, 1), dtype=torch.float32, device=dev)

        w_trans = w_torch.t()
        sens = torch.sparse.mm(
            w_trans, torch.ones((coo.shape[0], 1), dtype=torch.float32, device=dev))

        for _ in range(self.max_iterations):
            pred_var = torch.clamp(torch.sparse.mm(w_torch, lam), min=1e-6)
            ratio = (y_meas.view(-1, 1)) / pred_var
            backproj = torch.sparse.mm(w_trans, ratio)

            lam_3d = lam.view(nx, ny, nz)
            prior_grad = huber_regularization_gradient_torch(lam_3d, delta=self.huber_delta).view(n_voxels, 1)

            denom = torch.maximum(sens - self.beta * prior_grad,
                              torch.clamp(0.1 * sens, min=1e-8))
            lam = torch.clamp(lam * (backproj / denom), min=1e-6)

        return lam.view(nx, ny, nz).detach().cpu().numpy()

from typing import Tuple, List
import numpy as np
import scipy.sparse as sp

def compute_siddon_ray_voxel_intersections(
    p1_mm: np.ndarray,
    p2_mm: np.ndarray,
    grid_size: Tuple[int, int, int],
    bounds_min_mm: Tuple[float, float, float],
    bounds_max_mm: Tuple[float, float, float],
) -> Tuple[List[int], List[float]]:
    nx, ny, nz = grid_size
    xmin, ymin, zmin = bounds_min_mm
    xmax, ymax, zmax = bounds_max_mm

    dx = (xmax - xmin) / nx
    dy = (ymax - ymin) / ny
    dz = (zmax - zmin) / nz

    total_dist = float(np.linalg.norm(p2_mm - p1_mm))
    if total_dist <= 1e-6:
        return [], []

    d = p2_mm - p1_mm
    alpha_min = 0.0
    alpha_max = 1.0

    for k in range(3):
        p1_k = p1_mm[k]
        d_k = d[k]
        b_min_k = bounds_min_mm[k]
        b_max_k = bounds_max_mm[k]

        if abs(d_k) > 1e-9:
            a0 = (b_min_k - p1_k) / d_k
            a1 = (b_max_k - p1_k) / d_k
            alpha_min = max(alpha_min, min(a0, a1))
            alpha_max = min(alpha_max, max(a0, a1))
        else:
            if p1_k < b_min_k or p1_k > b_max_k:
                return [], []

    if alpha_min >= alpha_max:
        return [], []

    alphas = [alpha_min, alpha_max]
    for k, (n_k, b_min_k, delta_k) in enumerate([(nx, xmin, dx), (ny, ymin, dy), (nz, zmin, dz)]):
        if abs(d[k]) > 1e-9:
            p_enter = p1_mm[k] + alpha_min * d[k]
            p_exit = p1_mm[k] + alpha_max * d[k]
            idx_start = int(np.floor((min(p_enter, p_exit) - b_min_k) / delta_k)) + 1
            idx_end = int(np.ceil((max(p_enter, p_exit) - b_min_k) / delta_k))
            
            for plane_idx in range(max(1, idx_start), min(n_k, idx_end + 1)):
                plane_pos = b_min_k + plane_idx * delta_k
                alpha_plane = (plane_pos - p1_mm[k]) / d[k]
                if alpha_min < alpha_plane < alpha_max:
                    alphas.append(alpha_plane)

    alphas = sorted(set(alphas))
    voxel_indices = []
    path_lengths = []

    for i in range(len(alphas) - 1):
        a_mid = 0.5 * (alphas[i] + alphas[i + 1])
        mid_point = p1_mm + a_mid * d

        ix = int(np.floor((mid_point[0] - xmin) / dx))
        iy = int(np.floor((mid_point[1] - ymin) / dy))
        iz = int(np.floor((mid_point[2] - zmin) / dz))

        if 0 <= ix < nx and 0 <= iy < ny and 0 <= iz < nz:
            seg_len_cm = ((alphas[i + 1] - alphas[i]) * total_dist) / 10.0
            flat_idx = ix * (ny * nz) + iy * nz + iz
            voxel_indices.append(flat_idx)
            path_lengths.append(seg_len_cm)

    return voxel_indices, path_lengths

def build_sparse_system_matrix(
    p1_arr: np.ndarray,
    p2_arr: np.ndarray,
    grid_size: Tuple[int, int, int],
    bounds_min_mm: Tuple[float, float, float],
    bounds_max_mm: Tuple[float, float, float],
) -> sp.csr_matrix:
    n_rays = len(p1_arr)
    n_voxels = grid_size[0] * grid_size[1] * grid_size[2]

    rows = []
    cols = []
    data = []

    for r_idx in range(n_rays):
        v_idx_list, lengths = compute_siddon_ray_voxel_intersections(
            p1_arr[r_idx],
            p2_arr[r_idx],
            grid_size,
            bounds_min_mm,
            bounds_max_mm,
        )
        for c_idx, val in zip(v_idx_list, lengths):
            rows.append(r_idx)
            cols.append(c_idx)
            data.append(val)

    return sp.csr_matrix((data, (rows, cols)), shape=(n_rays, n_voxels), dtype=np.float32)

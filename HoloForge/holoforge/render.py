"""PNG report renderer - visual QA of a print job.

Layout (one PNG):
  row 1: target | dose | cured-vs-target overlay   (max intensity projections)
  row 2: center slices z/y/x of the dose field
  row 3: phase maps of the first patterns + metrics banner
"""
from __future__ import annotations

import numpy as np
from PIL import Image, ImageDraw

from .config import PrintJob


# inferno-inspired colormap LUT (256 entries)
def _inferno_lut() -> np.ndarray:
    stops = np.array([
        [0, 0, 4], [27, 12, 65], [74, 12, 107], [120, 28, 109],
        [165, 44, 96], [207, 68, 70], [237, 105, 37], [251, 155, 6],
        [247, 209, 61], [252, 255, 164], [255, 255, 255],
    ], dtype=np.float64)
    n = len(stops) - 1
    lut = np.zeros((256, 3))
    for i in range(256):
        t = i / 255.0 * n
        j = int(min(t, n - 1))
        f = t - j
        lut[i] = stops[j] * (1 - f) + stops[j + 1] * f
    return lut.astype(np.uint8)


_LUT = _inferno_lut()


def _to_rgb(norm_map: np.ndarray, vmin=0.0, vmax=None) -> np.ndarray:
    m = norm_map.astype(np.float64)
    vmax = vmax if vmax is not None and vmax > 0 else max(m.max(), 1e-12)
    idx = np.clip((m - vmin) / max(vmax - vmin, 1e-12), 0, 1)
    return _LUT[(idx * 255).astype(np.uint8)]


def _label(draw: ImageDraw.ImageDraw, xy: tuple[int, int], text: str,
           fg=(235, 235, 235), bg=(20, 20, 26)) -> None:
    draw.rectangle([xy[0], xy[1], xy[0] + 8 * len(text) + 8, xy[1] + 18], fill=bg)
    draw.text((xy[0] + 4, xy[1] + 3), text, fill=fg)


def render_report(job: PrintJob, dose: np.ndarray, cured: np.ndarray,
                  path: str, tile: int = 192) -> str:
    nz, ny, nx = dose.shape
    target = np.asarray(job.target_mask, dtype=bool)

    W = tile * 3 + 40
    H = tile * 3 + 130
    img = Image.new("RGB", (W, H), (16, 16, 22))
    d = ImageDraw.Draw(img)

    vmax = float(dose[target].mean() * 2.2) if target.any() else None

    def paste(arr_rgb, ox, oy):
        im = Image.fromarray(np.ascontiguousarray(arr_rgb)).resize(
            (tile, tile), Image.NEAREST)
        img.paste(im, (ox, oy))

    # --- row 1: MIPs ---
    mip_t = _to_rgb(target.any(axis=1).astype(float))
    mip_d = _to_rgb(dose.max(axis=1), vmax=vmax)
    overlay = np.zeros((nz, nx, 3), dtype=np.uint8)
    tgt_mip = target.any(axis=1)
    cur_mip = cured.any(axis=1)
    overlay[tgt_mip & ~cur_mip] = (60, 60, 230)      # missed: blue
    overlay[cur_mip & ~tgt_mip] = (230, 60, 60)      # overcure bleed: red
    overlay[tgt_mip & cur_mip] = (240, 240, 240)     # correct cure: white
    paste(mip_t, 10, 30)
    paste(mip_d, 10 + tile + 10, 30)
    paste(overlay, 10 + 2 * tile + 20, 30)
    _label(d, (10, 8), "TARGET (MIP)")
    _label(d, (10 + tile + 10, 8), "DOSE (MIP, log-scaled below)")
    _label(d, (10 + 2 * tile + 20, 8), "CURE OVERLAY white=ok red=bleed blue=miss")

    # --- row 2: center slices ---
    midz, midy, midx = nz // 2, ny // 2, nx // 2
    paste(_to_rgb(dose[midz], vmax=vmax), 10, 40 + tile)
    paste(_to_rgb(dose[:, midy, :], vmax=vmax), 10 + tile + 10, 40 + tile)
    paste(_to_rgb(dose[:, :, midx], vmax=vmax), 10 + 2 * tile + 20, 40 + tile)
    m = job.metrics or {}
    banner = (f"IoU {m.get('cure_iou', float('nan')):.3f}   "
              f"unif {m.get('uniformity_error', float('nan')):.3f}   "
              f"bleed {m.get('bleed_ratio', float('nan')):.4f}   "
              f"exp {job.exposure_s:.1f}s")
    _label(d, (10, 36 + tile), f"DOSE SLICES z/mid y/mid x/mid      {banner}")

    # --- row 3: phase maps ---
    for k in range(min(3, job.phases_rad.shape[0])):
        ph = job.phases_rad[k]
        rgb = (((ph + np.pi) / (2 * np.pi)) * 255).astype(np.int32).clip(0, 255)
        hsl = np.stack([rgb, 255 - rgb, (rgb * 7) % 256], axis=-1).astype(np.uint8)
        paste(hsl, 10 + k * (tile // 2 + 5), 46 + 2 * tile)
    _label(d, (10, 42 + 2 * tile),
           f"SLM PHASES (first {min(3, job.phases_rad.shape[0])} of "
           f"{job.phases_rad.shape[0]}, {job.resin.name})")

    img.save(path)
    return path

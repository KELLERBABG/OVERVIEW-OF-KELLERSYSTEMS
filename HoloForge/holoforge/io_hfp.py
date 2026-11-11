"""Save/load HoloForge print jobs (.hfp = npz container + sidecar .json)."""
from __future__ import annotations

import json

import numpy as np

from .config import RESINS, GridConfig, OpticsConfig, PrintJob, ResinModel


def save_job(job: PrintJob, path: str) -> None:
    """Write job to `path` (npz payload) plus `path + '.json'` metadata."""
    with open(path, "wb") as f:
        np.savez_compressed(
            f,
            target_mask=job.target_mask.astype(np.uint8),
            phases_rad=job.phases_rad,
        )
    with open(path + ".json", "w", encoding="utf-8") as f:
        f.write(job.to_json())


def load_job(path: str) -> PrintJob:
    data = np.load(path)
    target_mask = data["target_mask"].astype(bool)
    phases_rad = data["phases_rad"].copy()
    data.close()
    with open(path + ".json", encoding="utf-8") as f:
        meta = json.load(f)
    optics = OpticsConfig(**meta["optics"])
    grid = GridConfig(**meta["grid"])
    resin = RESINS.get(meta["resin"]["name"])
    if resin is None:
        resin = ResinModel(**meta["resin"])
    return PrintJob(
        optics=optics,
        grid=grid,
        resin=resin,
        target_mask=target_mask,
        phases_rad=phases_rad,
        pattern_weights=meta["pattern_weights"],
        exposure_s=meta["exposure_s"],
        rotation_angles_deg=meta.get("rotation_angles_deg"),
        metrics=meta.get("metrics", {}),
        meta=meta.get("meta", {}),
    )

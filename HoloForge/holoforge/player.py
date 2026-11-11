"""Hardware SLM Player and Sequence Timing Engine.

Orchestrates time-multiplexed phase projection across multiple patterns
with exact millisecond exposure intervals corresponding to the optimized weights:
  t_k = T_total * w_k
"""
from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass

import numpy as np
from PIL import Image

from .calibration import DEFAULT_LUT, SLMCalibrationLUT
from .config import PrintJob


@dataclass
class PatternFrame:
    index: int
    duration_ms: float
    weight: float
    image_path: str | None = None


@dataclass
class PlaybackManifest:
    job_name: str
    total_exposure_s: float
    num_patterns: int
    resin_name: str
    frames: list[PatternFrame]

    def to_json(self) -> str:
        return json.dumps({
            "job_name": self.job_name,
            "total_exposure_s": self.total_exposure_s,
            "num_patterns": self.num_patterns,
            "resin_name": self.resin_name,
            "frames": [
                {
                    "index": f.index,
                    "duration_ms": round(f.duration_ms, 2),
                    "weight": round(f.weight, 4),
                    "image_path": f.image_path,
                }
                for f in self.frames
            ]
        }, indent=2)


def prepare_hardware_bundle(
    job: PrintJob,
    out_dir: str,
    lut: SLMCalibrationLUT | None = None,
    image_format: str = "png",
) -> PlaybackManifest:
    """Export all phase patterns as calibrated 8-bit bitmaps along with timing manifest.
    
    Parameters
    ----------
    job : PrintJob
        The optimized HoloForge job.
    out_dir : str
        Target folder for images and manifest.
    lut : SLMCalibrationLUT, optional
        Phase Look-Up Table.
    image_format : str
        'png' or 'bmp'.
    """
    os.makedirs(out_dir, exist_ok=True)
    lut = lut or DEFAULT_LUT
    
    phases = job.phases_rad
    weights = np.asarray(job.pattern_weights, dtype=np.float64)
    weights /= weights.sum()
    
    total_ms = job.exposure_s * 1000.0
    frames = []
    
    for k in range(len(weights)):
        duration_ms = float(total_ms * weights[k])
        
        # Convert to 8-bit SLM drive levels
        drive_img = lut.phase_to_drive(phases[k])
        img = Image.fromarray(drive_img, mode="L")
        
        filename = f"frame_{k:04d}.{image_format.lower()}"
        filepath = os.path.join(out_dir, filename)
        img.save(filepath)
        
        frames.append(PatternFrame(
            index=k,
            duration_ms=duration_ms,
            weight=float(weights[k]),
            image_path=filename,
        ))
        
    manifest = PlaybackManifest(
        job_name=job.meta.get("name", "unnamed_job"),
        total_exposure_s=job.exposure_s,
        num_patterns=len(weights),
        resin_name=job.resin.name,
        frames=frames,
    )
    
    manifest_path = os.path.join(out_dir, "manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as f:
        f.write(manifest.to_json())
        
    return manifest


def play_sequence_simulated(
    manifest: PlaybackManifest,
    speed_multiplier: float = 1.0,
    verbose: bool = True,
) -> None:
    """Simulate hardware playback with precise console logging."""
    if verbose:
        print(f"\n[SLM Hardware Player] Starting playback for '{manifest.job_name}'")
        print(f"  Total Duration: {manifest.total_exposure_s:.2f} s | Patterns: {manifest.num_patterns}")
        print(f"  Resin: {manifest.resin_name}\n")
        
    t_start = time.time()
    for frame in manifest.frames:
        sim_ms = frame.duration_ms / speed_multiplier
        if verbose:
            print(f"  -> Projecting Pattern #{frame.index:02d} ({frame.image_path}) "
                  f"for {frame.duration_ms:.1f} ms [weight: {frame.weight:.3f}]")
        # Sleep for simulated timing if multiplier > 0
        if speed_multiplier > 0:
            time.sleep(min(sim_ms / 1000.0, 0.5)) # capped for interactive responsiveness
            
    if verbose:
        elapsed = time.time() - t_start
        print(f"\n[SLM Hardware Player] [OK] Sequence complete in {elapsed:.2f} s wall time.")

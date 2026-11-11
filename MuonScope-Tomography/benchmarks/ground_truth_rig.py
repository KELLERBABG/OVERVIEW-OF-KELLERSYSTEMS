import os
import sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "src", "inversion")))

import numpy as np
from muonscope.simulation.synthetic_scene import VoxelScene
from muonscope.simulation.ray_tracer import MonteCarloRayTracer
from muonscope.physics.gaisser import sample_cosmic_muons
from muonscope.reconstructors.poca import PoCAReconstructor

def build_iso_astm_calibration_phantom(grid_size=(64, 64, 64)) -> VoxelScene:
    nx, ny, nz = grid_size

    density = np.full((nx, ny, nz), 2.35, dtype=np.float32)
    rad_len = np.full((nx, ny, nz), 11.55, dtype=np.float32)
    z_eff = np.full((nx, ny, nz), 11.5, dtype=np.float32)

    xs = np.linspace(-250.0, 250.0, nx)
    ys = np.linspace(-250.0, 250.0, ny)
    zs = np.linspace(-250.0, 250.0, nz)
    xg, yg, zg = np.meshgrid(xs, ys, zs, indexing="ij")

    # Air void (r = 20 mm)
    air_mask = np.sqrt((xg + 125.0)**2 + (yg + 125.0)**2) <= 20.0
    density[air_mask] = 0.0012
    rad_len[air_mask] = 30420.0
    z_eff[air_mask] = 7.2

    # Water channel (r = 25 mm)
    water_mask = np.sqrt((xg - 125.0)**2 + (yg + 125.0)**2) <= 25.0
    density[water_mask] = 1.0
    rad_len[water_mask] = 36.08
    z_eff[water_mask] = 7.42

    # Iron cylinder (r = 15 mm)
    iron_mask = np.sqrt((xg + 125.0)**2 + (yg - 125.0)**2) <= 15.0
    density[iron_mask] = 7.87
    rad_len[iron_mask] = 1.76
    z_eff[iron_mask] = 26.0

    # Lead cylinder (r = 12.5 mm)
    lead_mask = np.sqrt((xg - 125.0)**2 + (yg - 125.0)**2) <= 12.5
    density[lead_mask] = 11.35
    rad_len[lead_mask] = 0.56
    z_eff[lead_mask] = 82.0

    return VoxelScene(
        grid_size=grid_size,
        bounds_min_mm=(-250.0, -250.0, -250.0),
        bounds_max_mm=(250.0, 250.0, 250.0),
        density_grid_g_cm3=density,
        rad_length_grid_cm=rad_len,
        z_eff_grid=z_eff,
    )

def run_benchmark(num_events: int = 1500) -> dict:
    scene = build_iso_astm_calibration_phantom()
    tracer = MonteCarloRayTracer(scene, top_panel_z_mm=350.0, bottom_panel_z_mm=-350.0)

    energies, dirs, charges = sample_cosmic_muons(num_events)
    events = tracer.trace_batch(num_events, energies, dirs, charges)

    recon = PoCAReconstructor(
        grid_size=(64, 64, 64),
        bounds_min_mm=(-250.0, -250.0, -250.0),
        bounds_max_mm=(250.0, 250.0, 250.0),
    )
    for ev in events:
        recon.process_event(ev.p_in, ev.d_in, ev.p_out, ev.d_out)

    recon_grid = recon.get_reconstructed_density()

    return {
        "simulated_events": num_events,
        "accepted_events": len(events),
        "acceptance_ratio": len(events) / num_events,
        "max_reconstructed_density": float(np.max(recon_grid)),
        "mean_reconstructed_density": float(np.mean(recon_grid)),
    }

if __name__ == "__main__":
    results = run_benchmark(1000)
    print("=" * 60)
    print("  ISO/ASTM E1441 CALIBRATION RIG BENCHMARK RESULTS")
    print("=" * 60)
    for k, v in results.items():
        print(f"  {k}: {v}")
    print("=" * 60)

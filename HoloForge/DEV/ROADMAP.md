# HoloForge — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Python`
> **Summary:** A Python Volumetric Additive Manufacturing suite (`holoforge/optics.py`, `optimize.py`, `kinetics.py`, `metrology.py`, `dose_compensate.py`, `volume.py`, `hardware/stage.py`, `laser.py`, `display.py`, `runner.py`, `web/server.py`) achieving 92.3% simulated Cure IoU and <0.02% overcure via Multi-Plane Gerchberg-Saxton and coupled free-radical O2-inhibition kinetics.

## Task Queue

- [ ] **TASK-01: Enforce pre-flight volumetric dose invariant gate in hardware print runner**
  - **Target Files:** `holoforge/hardware/runner.py`, `tests/test_safety.py`
  - **Context Files:** `holoforge/optimize.py`, `holoforge/hardware/laser.py`
  - **Verify:** `pytest tests/test_safety.py tests/test_hardware.py -q`
  - **Spec:** Enhance `holoforge/hardware/runner.py` so `PrintRunner.execute()` runs a mandatory pre-flight dose invariant check before opening the 405 nm laser shutter.

1. Reject execution with `DoseInvariantError` if the hologram set's predicted `overcure_fraction > max_overcure` (default `0.0005`), if any angular frame contains NaN/Inf phase values, or if rotary stage angular step synchronization mismatches the hologram count.
2. Add unit tests in `tests/test_safety.py` verifying that out-of-spec hologram stacks never open the laser shutter.

- [ ] **TASK-02: Implement Lorentz-Lorenz refractive index Δn endpoint trigger in Schlieren metrology**
  - **Target Files:** `holoforge/metrology.py`, `holoforge/hardware/camera.py`, `tests/test_optics.py`
  - **Context Files:** `holoforge/kinetics.py`
  - **Verify:** `pytest tests/test_optics.py -q`
  - **Spec:** Extend `holoforge/metrology.py` and `holoforge/hardware/camera.py` to compute the Lorentz-Lorenz refractive index change $\Delta n(r)$ as monomer conversion $\alpha(r)$ crosses the gelation point $\alpha_{\text{gel}}$.

1. Provide `detect_gelation_endpoint(schlieren_frames) -> EndpointVerdict` that triggers an automatic shutter close signal within 1 angular frame of reaching target conversion.
2. Add unit tests in `tests/test_optics.py`.

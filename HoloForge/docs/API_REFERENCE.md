# HoloForge Complete API Reference

This reference covers the public classes, dataclasses, and functions across the entire HoloForge Python library.

---

## 1. `holoforge.config`

### `OpticsConfig`
```python
@dataclass(frozen=True)
class OpticsConfig:
    wavelength_nm: float = 405.0        # Laser vacuum wavelength
    slm_pixels: int = 256               # Square SLM pixel count
    slm_pitch_um: float = 8.0           # Physical SLM pixel pitch (um)
    relay_magnification: float = 0.25   # SLM to vat demagnification (M < 1)
    vat_index: float = 1.46             # Refractive index of the resin
```
**Properties**:
- `pitch_um`: Effective transverse pixel pitch inside the vat (`slm_pitch_um * relay_magnification`).
- `wavelength_vat_um`: Wavelength inside the resin medium (`wavelength_nm / 1000.0 / vat_index`).

### `GridConfig`
```python
@dataclass(frozen=True)
class GridConfig:
    n_xy: int = 128                     # Transverse grid samples
    n_z: int = 96                       # Axial slices
    slice_step_um: float | None = None  # Axial slice step dz (defaults to 2.0 um)
```
**Methods**:
- `extent_um(optics: OpticsConfig) -> tuple[float, float, float]`: Total spatial domain size in micrometers.

### `ResinModel`
```python
@dataclass(frozen=True)
class ResinModel:
    name: str
    gel_dose_mJ_cm2: float   # Gel point threshold dose
    absorption_per_mm: float # Linear attenuation coefficient alpha
    scatter_length_um: float # Forward-scatter transport length
    index: float             # Refractive index at working wavelength
```
**Properties**:
- `amplitude_absorption_per_mm`: Half the intensity absorption rate (`0.5 * absorption_per_mm`).

**Built-In Presets in `RESINS`**:
- `"hf-standard-405"`: General prototyping ($D_{\text{gel}}=12.0\,\text{mJ/cm}^2$).
- `"hf-clear-405"`: Low-scatter microfluidics ($D_{\text{gel}}=10.0\,\text{mJ/cm}^2$).
- `"hf-dental-405"`: High-modulus filled crowns ($D_{\text{gel}}=18.0\,\text{mJ/cm}^2$).
- `"hf-pegda-405"`: PEGDA + 1.0 wt% TPO-L ($D_{\text{gel}}=11.0\,\text{mJ/cm}^2$).
- `"hf-hdda-405"`: HDDA + 1.0 wt% TPO-L ($D_{\text{gel}}=14.0\,\text{mJ/cm}^2$).

### `PrintJob`
```python
@dataclass
class PrintJob:
    optics: OpticsConfig
    grid: GridConfig
    resin: ResinModel
    target_mask: np.ndarray             # (nz, ny, nx) bool
    phases_rad: np.ndarray              # (K, ny, nx) float64
    pattern_weights: list[float]        # Relative weights summing to 1.0
    exposure_s: float = 1.0             # Total print duration (seconds)
    metrics: dict = field(default_factory=dict)
    meta: dict = field(default_factory=dict)
```
**Methods**:
- `summary() -> str`: Multi-line formatted quality report.
- `to_json() -> str`: JSON serialization of job parameters and metadata.

---

## 2. `holoforge.optics`

### `VatPropagator`
```python
class VatPropagator(optics: OpticsConfig, grid: GridConfig)
```
- `forward(phases_rad: np.ndarray, resin: ResinModel, keep_fields: bool = False) -> tuple[np.ndarray, list[np.ndarray]]`: Forward split-step ASM walk through the resin.
- `forward_band(phase_rad: np.ndarray, resin: ResinModel, z_lo: int, z_hi: int) -> dict[int, np.ndarray]`: Propagate a single pattern through its assigned axial band.
- `backward_band(fields_by_slice: dict[int, np.ndarray], z_lo: int, z_hi: int, resin: ResinModel) -> np.ndarray`: Adjoint backward walk from band exit back to the SLM.
- `backward(sources: list[np.ndarray], resin: ResinModel) -> np.ndarray`: Full-volume adjoint backward walk.
- `aperture() -> np.ndarray`: Circular pupil aperture transmission function.
- `field_from_phases(phases_rad: np.ndarray) -> np.ndarray`: Complex field at the SLM plane.

---

## 3. `holoforge.optimize`

### `MultiPlaneGS` (alias `WavefrontMatchedCGH`)
```python
class MultiPlaneGS(
    optics: OpticsConfig,
    grid: GridConfig,
    resin: ResinModel,
    target_mask: np.ndarray,
    n_patterns: int = 8,
    seed: int = 7,
    guard_slices: int = 3,
    target_amplitudes: np.ndarray | None = None,
)
```
- `step(beta: float = 0.65) -> dict`: Executes one Multi-Plane Gerchberg-Saxton sweep over all $K$ patterns.
- `optimize(iterations: int = 40, beta: float = 0.65, verbose: bool = True, log_every: int = 5) -> OptimizeResult`: Runs iterative optimization with plateau early stopping.

---

## 4. `holoforge.dose_compensate`

### `DoseCompensator`
```python
class DoseCompensator(
    optics: OpticsConfig,
    grid: GridConfig,
    resin: ResinModel,
    target_mask: np.ndarray,
    n_patterns: int = 8,
    seed: int = 7,
)
```
- `run(gs_iterations: int = 40, comp_iterations: int = 5, beta: float = 0.65, gain: float = 0.5, uniformity_threshold: float = 0.10, verbose: bool = True) -> CompensationResult`: Executes the outer dose compensation loop.

**Standalone Physical Maps**:
- `beer_lambert_attenuation_map(resin: ResinModel, grid: GridConfig) -> np.ndarray`: Depth-dependent intensity attenuation profile $I(z) = I_0 e^{-\alpha z}$.
- `monte_carlo_lite_scatter(resin: ResinModel, optics: OpticsConfig, grid: GridConfig) -> np.ndarray`: Forward-scattering Gaussian beam broadening per slice depth.

---

## 5. `holoforge.calibration` (Stage 1)

### `SLMCalibrationLUT`
```python
@dataclass
class SLMCalibrationLUT:
    name: str = "linear-405nm"
    drive_levels: np.ndarray = None        # (256,) uint8 [0..255]
    phase_response_rad: np.ndarray = None  # (256,) float [0..2pi]
```
- `phase_to_drive(phase_rad: np.ndarray) -> np.ndarray[uint8]`: Maps continuous phase in radians to 8-bit digital drive values.
- `drive_to_phase(drive_uint8: np.ndarray) -> np.ndarray[float64]`: Inverts 8-bit drive levels back to phase retardation.
- `make_scurve_lut(gamma: float = 1.6) -> SLMCalibrationLUT`: Creates non-linear liquid crystal response profile.

**Alignment & Export Functions**:
- `flat_field_correction(measured_beam: np.ndarray, target_profile: str = "uniform") -> np.ndarray`: Compensates for non-uniform Gaussian laser profiles.
- `generate_dye_cell_target(target_type: str = "spot_grid", optics=None, grid=None, spots_xy=(3, 3), focal_depth_mm=0.5) -> np.ndarray`: Generates test target phase maps (`"spot_grid"`, `"bessel_beam"`, `"crosshair"`).
- `export_slm_images(phases_rad: np.ndarray, out_dir: str, prefix: str = "slm_pattern", lut=None, image_format: str = "png") -> list[str]`: Saves 8-bit uncompressed grayscale SLM bitmaps.

---

## 6. `holoforge.kinetics` (Stage 2)

### `KineticParameters`
```python
@dataclass
class KineticParameters:
    initial_oxygen_mM: float = 1.4
    critical_oxygen_mM: float = 0.05
    initial_monomer_M: float = 5.2
    initial_pi_wt_percent: float = 1.0
    quantum_yield: float = 0.55
    k_inhibition: float = 1.5e6
    k_propagation: float = 8.5e2
    k_termination: float = 2.0e4
    gel_conversion: float = 0.08
```
- `simulate_kinetics(intensity_mW_cm2: float, params: KineticParameters = None, total_time_s: float = 15.0, n_steps: int = 1500) -> KineticSimulationResult`: Solves transient differential equations for dissolved $O_2$ consumption and radical crosslinking conversion.

---

## 7. `holoforge.metrology` (Stage 2)

- `extract_topography_height_map(cured_mask: np.ndarray, optics: OpticsConfig, grid: GridConfig, axis: int = 0, direction: int = 1) -> tuple[np.ndarray, np.ndarray]`: Extracts 2D surface elevation from 3D voxel arrays.
- `calculate_roughness(height_map_um: np.ndarray, valid_mask: np.ndarray = None, pitch_um: float = 2.0, remove_tilt: bool = True) -> RoughnessReport`: Calculates ISO 4287 roughness parameters ($R_a$, $R_q$, $R_z$) with tilt plane de-trending.
- `analyze_channel_patency(cured_mask: np.ndarray, channel_void_target: np.ndarray, optics: OpticsConfig) -> ChannelPatencyReport`: Verifies internal microfluidic void clearance and hydraulic diameter.

---

## 8. `holoforge.verticals` (Stage 3)

- `dental_crown(shape=(96, 128, 128), cusp_height=0.12, margin_radius=0.28) -> VoxelVolume`: Anatomical dental molar crown with realistic occlusal cusps and cervical preparation margin.
- `microfluidic_chip(shape=(96, 128, 128), channel_radius=0.035, chip_span=(0.28, 0.40, 0.40)) -> tuple[VoxelVolume, VoxelVolume]`: Lab-on-a-chip solid block enclosing leak-free serpentine micro-channels.
- `aspheric_lens(shape=(96, 128, 128), radius=0.36, curvature_c=1.8, conic_k=-0.65, alpha4=0.25, center_thickness=0.30) -> VoxelVolume`: Even-asphere precision optical lens profile.

---

## 9. `holoforge.player`

- `prepare_hardware_bundle(job: PrintJob, out_dir: str, lut: SLMCalibrationLUT = None, image_format: str = "png") -> PlaybackManifest`: Exports calibrated 8-bit bitmap frames alongside timing manifest (`manifest.json`).
- `play_sequence_simulated(manifest: PlaybackManifest, speed_multiplier: float = 1.0, verbose: bool = True) -> None`: Simulates hardware playback timing.

---

## 10. `holoforge.simulate`, `render`, `io_hfp`, `benchmark`

- `simulate_dose(phases_rad, weights, optics, grid, resin) -> np.ndarray`: Computes 3D accumulated energy field.
- `predict_cure(dose, gel_level_sim) -> np.ndarray`: Applies binary gel threshold.
- `gel_level_simulation_units(resin, optics, grid, exposure_s) -> float`: Maps physical energy density ($\text{J/cm}^2$) to simulation units.
- `render_report(job, dose, cured, path, tile=192) -> str`: Generates multi-panel PNG visual QA report.
- `save_job(job: PrintJob, path: str) -> None`: Compresses job into `.hfp` archive + `.json` sidecar.
- `load_job(path: str) -> PrintJob`: Loads `.hfp` archive and restores metadata.
- `run_benchmark(...) -> BenchmarkResult`: Executes standardized benchmark suite across test shapes.

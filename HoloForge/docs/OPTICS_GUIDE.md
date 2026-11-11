# HoloForge Optics Guide

The optical system is the physical backbone of the HoloForge printer. It transforms computationally derived phase maps into volumetric 3D dose distributions.

## Optical Train Overview

The standard optical path consists of:
1. **Laser**: A 405 nm diode laser (e.g., 100-500 mW).
2. **Beam Expander**: Collimates and expands the beam to fill the SLM aperture.
3. **Phase-Only SLM**: A Spatial Light Modulator (e.g., Holoeye Pluto or Jasper Display) that applies the programmed phase shift to the wavefront.
4. **4f Relay System**: Imaging lenses that relay the modulated wavefront into the center of the resin vat.
5. **Resin Vat**: A container with FEP or quartz windows holding the photopolymer.

## SLM Configuration

The SLM configuration dictates the resolution of the printing volume:
- `slm_pixels`: The active square area (e.g., 256x256).
- `slm_pitch_um`: The physical size of the SLM pixels (e.g., 8.0 µm).
- **Phase Response LUT**: You must calibrate the SLM's phase response (Look-Up Table) so that 0-255 grayscale values map linearly to a $0 - 2\pi$ phase delay at 405 nm.

## Relay Magnification & Pixel Pitch

The 4f relay system defines how the SLM pixels are projected into the resin vat.
If the magnification is $M$ (e.g., $M = 0.25$ using a 4x objective for demagnification), the effective transverse pixel pitch inside the vat becomes:
$$ p_{\text{vat}} = p_{\text{slm}} \times M $$
For an 8 µm SLM, a 0.25x relay yields 2 µm vat pixels.

## Angular Spectrum Propagation Physics

The HoloForge simulation engine models the volumetric light field using the **Split-Step Angular Spectrum Method**. The field walks slice-by-slice through the vat in steps of $dz$.

For each slice, the algorithm applies:
1. **Half-Step Fresnel Drift**: Propagation in Fourier space using $H_{\text{half}} = \exp(i k_z dz / 2)$.
2. **Forward-Scatter Blur**: A small-angle scatter convolution applied in the spatial domain.
3. **Absorption**: Intensity attenuation via $e^{-\alpha dz / 2}$ (Beer-Lambert).
4. **Half-Step Fresnel Drift**: Final spatial propagation to the next slice.

## Nyquist, Sampling, and Bands

The optimizer uses a **Band Decomposition Strategy**. The axial resolving power of the optical head dictates how far a single phase hologram can control the light field. This depth is roughly one Rayleigh range.

The target volume is divided into $K$ bands, one for each pattern. During optimization, the field is only constrained strictly within its assigned band, while guard slices suppress stray light.

## Alignment & Calibration Procedures

To execute Phase 1 successfully, follow these calibration routines:
- **Flat-Field Correction**: Ensure the laser intensity is uniform across the SLM aperture.
- **Phase Characterization**: Measure the phase delay curve and apply a corrective LUT.
- **First Light**: Project a single 2D focusing hologram into a vat filled with a fluorescent UV indicator dye to verify contrast and dose distribution before attempting 3D optimization.

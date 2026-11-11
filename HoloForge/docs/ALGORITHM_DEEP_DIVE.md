# Algorithm Deep Dive

HoloForge leverages a sophisticated optical solver to convert 3D volumetric targets into a time-multiplexed sequence of 2D phase holograms.

## The Gerchberg-Saxton Algorithm

The foundation is the Gerchberg-Saxton (GS) phase-retrieval algorithm. It alternates between constraints in two domains:
1. **Source plane (SLM)**: The field must have uniform amplitude (since it's a phase-only SLM).
2. **Target plane**: The field must match the desired amplitude profile.

HoloForge uses a **Multi-Plane** extension. The volume is decomposed into $K$ bands. Each of the $K$ patterns is responsible for delivering dose to one band.

## Amplitude Replacement & Least-Squares Scaling

During the forward sweep through a band's slices, the complex field $U$ is forced to match the desired target amplitude while preserving the phase. To prevent the optimizer from fighting the fixed total energy budget of the SLM, we use a least-squares scaled projection.

For a target shape $D$, the replacement amplitude is:
$$ \text{new\_amp} = |s| \times D $$
Where $s$ is the least-squares scaling factor:
$$ s = \frac{\langle D, U \rangle}{\langle D, D \rangle} $$

## Guard Slices

To prevent stray light from bleeding into regions outside the active band, **guard slices** are defined just above and below the band. The field in these slices is attenuated (multiplied by 0.25) during the GS sweep, strongly penalizing out-of-band energy delivery.

## Iterative Dose Compensation

Dose delivered at depth suffers from two degradation effects:
1. **Beer-Lambert Attenuation**: $I(z) = I_0 e^{-\alpha z}$
2. **Forward Scattering**: Diffuses sharp features. Modeled as angular diffusion $\theta_{\text{rms}} = \sqrt{z / L}$, leading to a Gaussian blur that broadens with depth.

The **Dose Compensator** runs an outer loop around the GS optimizer. After optimization, it evaluates the accumulated 3D dose and computes an error map:
$$ \text{error} = \frac{\text{mean\_target\_dose} - \text{local\_dose}}{\text{mean\_target\_dose}} $$

A correction map is updated multiplicatively:
$$ C \leftarrow C \times (1 + \text{gain} \times \text{error}) $$

This pre-emphasizes under-dosed (deeper or highly scattered) voxels, feeding back into the GS algorithm as a modified target amplitude.

## Quality Metrics

- **IoU (Intersection over Union)**: Overlap of the predicted cured volume vs. the target mask.
- **Uniformity Error**: $\sigma / \mu$ of the dose inside the target mask. Lower is better (<10% is ideal).
- **Bleed Ratio**: Mean dose in the 1-voxel dilation shell relative to the interior mean dose.
- **Peak to Mean**: Maximum internal dose divided by mean internal dose. (Measures hotspots).

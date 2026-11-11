# Frequently Asked Questions (FAQ)

### What makes HoloForge fundamentally different from SLA and DLP 3D printing?
Standard Stereolithography (SLA) and Digital Light Processing (DLP) build objects layer-by-layer ($25\text{–}100\,\mu\text{m}$ slices). This creates mechanical anisotropy (weak $Z$-axis tensile strength), visible stair-stepping striations, and requires sacrificial support structures that mar surface quality. HoloForge is a **volumetric** additive manufacturing system. It shapes a 3D coherent light field inside an oxygen-inhibited resin vat, solidifying the entire monolithic part simultaneously in **10 to 30 seconds** (simulated design target) with zero layers, zero supports, and glass-smooth surfaces ($R_a < 100\text{ nm}$, simulated).

### Why use a phase-only SLM instead of amplitude modulation (DLP/CAL)?
Tomographic Computed Axial Lithography (CAL) uses amplitude projectors that block light to create darkness, discarding $>50\text{–}80\%$ of optical energy. A **phase-only Spatial Light Modulator (SLM)** dynamically steers and shapes the optical wavefront with $>90\%$ transmission efficiency. It enables true 3D volumetric focusing and holographic multiplexing, allowing light to be concentrated precisely where needed at multiple depths within the vat.

### What are the 3 execution stages in the roadmap?
1. **Stage 1 (Optical Alignment Bench)**: Sourcing Thorlabs cage system, 405 nm single-mode laser, 4f relay optics, and a lab-grade phase LCoS SLM (Holoeye/Jasper) to validate 3D focusing in dye-doped liquid cells (€8k–€10k budget).
2. **Stage 2 (First Volumetric Solidification)**: Formulating PEGDA/HDDA + 1.0 wt% TPO-L resin and solidifying a 10 mm sphere and lattice cube in $\le 15\text{ seconds}$, validated via optical profilometry ($R_a < 100\text{ nm}$).
3. **Stage 3 (Niche High-Margin Commercial Focus)**: Targeting high-value B2B specialty markets: Digital Dental Labs (15s crowns/inlays), Microfluidic Lab-on-a-Chip prototyping (leak-free channels), and Custom Aspheric Lens manufacturing.

### Why bypass the consumer desktop 3D printer market?
Consumer/maker desktop 3D printing is a low-margin red ocean dominated by commodity FDM and resin printers. HoloForge’s unique value propositions (15-second monolithic print speed, zero support post-processing, optically smooth sub-micron surfaces, and leak-free internal micro-channels) solve mission-critical pain points in high-margin B2B sectors where customers bill €50–€500+ per part and operate under strict turnaround deadlines.

### How does HoloForge achieve 15-second solidification in Stage 2?
We formulate a low-viscosity acrylate system: **PEGDA** ($M_n \approx 250\text{–}575$) or **HDDA** combined with **1.0 wt% TPO-L** photoinitiator. Dissolved ambient molecular oxygen acts as a radical scavenger ($k_{\text{inh}} \approx 10^9\,\text{M}^{-1}\text{s}^{-1}$), suppressing crosslinking until an induction dose $D_{\text{gel}}$ is reached. Once oxygen is locally depleted, free-radical polymerization crosslinks the network in $<1\text{ second}$.

### How does HoloForge interface with SLM hardware?
HoloForge includes a dedicated hardware pattern exporter and playback engine (`holoforge.player`). The `holoforge export-slm` command converts any `.hfp` job file into an uncompressed sequence of 8-bit calibrated grayscale bitmaps (mapping $[0, 2\pi]$ phase shift through calibrated Look-Up Tables) and generates a `manifest.json` file specifying exact millisecond exposure intervals per pattern ($t_k = T_{\text{total}} \cdot w_k$).

### What optical resolution and surface roughness does HoloForge deliver?
Effective resolution is governed by the physical SLM pixel pitch ($8.0\,\mu\text{m}$) and the telecentric $4f$ relay demagnification ($M = 0.25\times$), yielding an effective in-vat transverse voxel pitch of:
$$p_{\text{vat}} = 8.0\,\mu\text{m} \times 0.25 = 2.0\,\mu\text{m}$$
At the evidence-harness benchmark resolution (64×64×32 grid, 2 µm voxel pitch, pinned seed 7 — regenerate with `python -m holoforge.cli verify`), the virtual white-light interferometric profilometer in the Digital Twin measures an average roughness $R_a \approx 3.4\,\mu\text{m}$ on a cured sphere surface; at this resolution roughness is dominated by voxel stair-stepping. $R_a < 100\text{ nm}$ remains a design target for finer-grid and future hardware configurations, not a reproduced result.

### What file formats are supported?
- Input: Any standard binary or ASCII `.stl` 3D mesh model (auto-fitted and voxelized via ray-parity along the $Z$-axis) or parametric analytical primitives.
- Output: `.hfp` (HoloForge Print) compressed archives with JSON metadata sidecars, 8-bit SLM drive bitmaps, and PNG visual QA reports.

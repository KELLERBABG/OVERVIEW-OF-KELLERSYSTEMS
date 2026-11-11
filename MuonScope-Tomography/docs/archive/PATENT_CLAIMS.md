# MuonScope™ Defensive Patent Claims & Intellectual Property Portfolio

---

## TITLE OF THE INVENTION
**PORTABLE MULTI-STATION COSMIC-RAY MUON TOMOGRAPHY SYSTEM WITH BARYCENTRIC SUB-MILLIMETER TRACKING, PASSIVE MOMENTUM SPECTROMETRY, AND DISTRIBUTED TIME-SYNCHRONIZED RECONSTRUCTION**

---

## FIELD OF THE INVENTION
The present invention relates generally to non-destructive testing (NDT), radiological particle tracking, and geophysical subsurface imaging, and more particularly to passive cosmic-ray muon scattering and absorption tomography systems for structural health monitoring, subterranean cavity detection, and nuclear security contraband screening.

---

## PATENT CLAIMS DRAFT (INDEPENDENT & DEPENDENT CLAIMS)

### **Claim 1 (Independent — Triangular Barycentric Scintillator Geometry):**
A particle detection apparatus for tracking ionizing charged particles, comprising:
1. A plurality of elongated plastic scintillator bars arranged in a mutually interlocking, planar array, wherein each scintillator bar has a substantially triangular cross-section comprising a base and an opposing apex, arranged such that adjacent bars alternate in apex orientation;
2. A wavelength-shifting (WLS) optical fiber embedded along a longitudinal center-of-gravity groove of each triangular scintillator bar;
3. A silicon photomultiplier (SiPM) optically coupled to an end termination of each WLS optical fiber; and
4. An electronic processing unit configured to:
   - detect coincident scintillation light pulses in at least two mutually adjacent interlocking triangular scintillator bars traversed by an incident charged particle;
   - measure the analog charge amplitudes ($Q_1, Q_2$) deposited in said adjacent bars; and
   - calculate a continuous transverse particle hit position ($x$) via charge-weighted barycentric interpolation:
     $$x = \frac{Q_1 \cdot x_1 + Q_2 \cdot x_2}{Q_1 + Q_2}$$
     where $x_1$ and $x_2$ represent the spatial center coordinates of said adjacent bars, whereby spatial hit resolution $\sigma_{\text{pos}} \le 0.8\text{ mm}$ is achieved with a bar base pitch of $20\text{ mm}$.

### **Claim 2 (Dependent on Claim 1 — Optical Labyrinth & Thermal Coupling):**
The apparatus of Claim 1, further comprising:
- an enclosure featuring a double-labyrinth light-tight seal providing $>120\text{ dB}$ optical attenuation of ambient sunlight; and
- a spring-loaded optical interface exerting a continuous contact force of $2.5\text{ N} \pm 0.3\text{ N}$ between each WLS fiber termination and corresponding SiPM active surface through an index-matching optical silicone elastomer having a refractive index $\eta = 1.465 \pm 0.015$.

### **Claim 3 (Independent — Passive Lead Momentum Spectrometry):**
A method for eliminating particle momentum ambiguity in cosmic-ray muon scattering tomography, comprising:
1. Providing a dual-panel muon hodoscope comprising an upper tracking panel, a lower tracking panel, and a passive high-$Z$ lead absorber plate of known thickness ($t_{\text{Pb}} = 5.0\text{ mm}$) interposed between inner detector planes;
2. Tracking the ingress trajectory vector $\vec{d}_{\text{in}}$ of a through-going atmospheric muon prior to entering said lead plate;
3. Tracking the egress trajectory vector $\vec{d}_{\text{out}}$ of said muon after exiting said lead plate;
4. Measuring an angular deflection angle $\theta_{\text{Pb}} = \arccos(\vec{d}_{\text{in}} \cdot \vec{d}_{\text{out}})$;
5. Estimating the individual particle momentum ($p$) via the Highland Multiple Coulomb Scattering inversion relation:
   $$p \approx \frac{13.6\text{ MeV}}{\beta c \cdot \theta_{\text{Pb}}} \sqrt{\frac{t_{\text{Pb}}}{X_0}} \left[1 + 0.038 \ln\left(\frac{t_{\text{Pb}}}{X_0}\right)\right]$$
6. Weighting the 3D tomographic scattering variance score in a Point of Closest Approach (PoCA) or Maximum A Posteriori (MAP-EM) voxel grid proportionally to $(p / p_{\text{ref}})^2$, thereby eliminating systematic false-positive artifacts caused by low-energy muon scattering anomalies.

### **Claim 4 (Independent — Distributed SAM-T Mesh Synchronization):**
A distributed multi-station tomographic tracking system comprising:
1. A plurality of autonomous, physically uncoupled muon detection stations deployed at spatially separated locations around a target structure;
2. Each detection station comprising an RTK-GNSS receiver, a 9-DoF Inertial Measurement Unit (IMU), an FPGA-based tapped delay line time-to-digital converter (TDC) having a timing resolution $\Delta t \le 75\text{ ps}$, and a wireless mesh transceiver;
3. Wherein each station time-stamps detected single-track segments with nanosecond-precision UTC timestamps derived from GNSS Pulse-Per-Second (PPS) signals; and
4. A central edge inversion engine configured to:
   - cross-correlate ingress track segments from a first station with egress track segments from a second station within a coincidence time window $\Delta t_{\text{mesh}} \le 10\text{ ns}$;
   - transform track coordinates from local station frames to a unified global UTM reference frame via real-time IMU quaternion attitude matrices; and
   - perform 3D tomographic density reconstruction of the volume enclosed between said stations.

### **Claim 5 (Independent — Automated BIM / IFC 4.3 Feature Translation Engine):**
A non-destructive testing system for structural infrastructure, comprising:
1. A volumetric reconstruction processor generating a 3D scattering density grid $\lambda(x,y,z)$ of a concrete structure from cosmic-ray muon trajectories;
2. A defect segmentation module configured to identify connected voxel regions having $\lambda < 0.02\text{ cm}^{-1}$ as internal air voids and connected voxel regions having $\lambda > 0.25\text{ cm}^{-1}$ as structural steel reinforcing bars; and
3. A Building Information Modeling (BIM) export pipeline configured to translate said segmented voxel regions into standardized Industry Foundation Classes (IFC 4.3) geometric entities, tagging said air voids as `IfcVoidingFeature` and said reinforcing bars as `IfcReinforcingBar`, and exporting a fully compliant `.ifc` structural model for direct ingestion into CAD/BIM software suites.

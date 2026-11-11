# MuonScope™ Field Operations & Deployment Manual

---

## 1. Quick-Start Deployment (15-Minute Setup)

```
Step 1: Unpack & Mount Tripod (0–3 min)
  └── Level the heavy-duty carbon fiber tripod using the integrated bubble level.
  └── Secure the MuonScope bottom panel to the tripod base plate with the 3/8"-16 quick-release clamp.

Step 2: Position Upper Panel & Spacers (3–6 min)
  └── Insert the precision CNC spacer brackets (default Δz = 150 mm, or 450 mm for deep mode).
  └── Fasten upper detector panel and engage the locking safety pins.

Step 3: Power On & Self-Test (6–9 min)
  └── Connect the internal 153.6 Wh LiFePO4 battery or external DC supply (10–36 V).
  └── Press and hold the power button for 2 seconds. The LED ring indicates:
      - BLUE (pulsing): Booting & FPGA initialization
      - GREEN (solid): All 128 SiPM channels equalized, ToF trigger locked, RTK-GNSS fixed
      - YELLOW: Operating in GNSS-denied mode (UWB / IMU active)
      - RED: Fault detected (refer to Section 5 Troubleshooting)

Step 4: Georeference & Initiate Survey (9–15 min)
  └── Connect tablet or laptop via Wi-Fi 6 (SSID: `MuonScope_Station_XX`) or USB-C tether.
  └── Open Web UI at `http://192.168.4.1:8080`.
  └── Name the project, select target material preset (Concrete, Masonry, Tunnel), and click "Start Survey".
```

---

## 2. Operating in GNSS-Denied & Subterranean Environments

When deploying inside tunnels, mines, basements, or deep urban canyons where satellite GNSS signals are unavailable:

1. **UWB Mesh Anchor Setup:**
   - Place 3x battery-powered Ultra-Wideband (UWB) anchor beacons around the survey perimeter ($10\text{--}50\text{ m}$ baseline).
   - The MuonScope station automatically synchronizes coordinates with the local UWB grid ($\sigma_{\text{xyz}} \le 20\text{ mm}$).
2. **Total Station / Laser Fiducial Alignment:**
   - Mount an optical prism reflector to the top panel fiducial mounting hole.
   - Sight the prism with a standard surveying total station and enter the resulting global UTM coordinates into the MuonScope Field GUI.
3. **Manual Coordinate Entry:**
   - In the Field Web GUI, navigate to `Settings -> Georeferencing -> Manual Mode` and input $(X, Y, Z, \text{Yaw}, \text{Pitch}, \text{Roll})$.

---

## 3. Operational Calibration Procedures & Schedule

```
┌───────────────────────────┬───────────────────────────────┬───────────────────────────────────────────┐
│ Calibration Type          │ Frequency                     │ Procedure & Acceptance Criteria           │
├───────────────────────────┼───────────────────────────────┼───────────────────────────────────────────┤
│ SiPM Bias Gain Trim       │ Automated on every boot       │ Closed-loop DAC scan to align 1 p.e. dark  │
│                           │                               │ count peak across all channels (±10 mV).  │
├───────────────────────────┼───────────────────────────────┼───────────────────────────────────────────┤
│ TDC Channel Delay Deskew  │ Weekly or after major shock   │ 10-minute isotropic cosmic shower test.   │
│                           │                               │ Residual skew must satisfy Δt_skew < 30 ps│
├───────────────────────────┼───────────────────────────────┼───────────────────────────────────────────┤
│ ASTM Phantom Audit        │ Monthly / Pre-Contract Audit  │ 60-minute scan on reference cube.         │
│                           │                               │ Must achieve CNR ≥ 4.5 on lead cylinder.  │
├───────────────────────────┼───────────────────────────────┼───────────────────────────────────────────┤
│ Optical Transmission Check│ Annual Maintenance            │ Dark current and single-MIP peak ratio    │
│                           │                               │ must remain within 85% of baseline.       │
└───────────────────────────┴───────────────────────────────┴───────────────────────────────────────────┘
```

---

## 4. Preventive Maintenance & Cleaning

- **Optical Window & Connectors:** Keep all external M12 connectors and optical fiducials free of mud and sand. Clean using isopropyl alcohol (IPA) wipes.
- **Nitrogen Purge Check:** Inspect the chassis pressure relief valve annually. If internal humidity exceeds $30\%$, re-purge with dry nitrogen gas via the Schrader service port.
- **Battery Storage:** For storage exceeding 30 days, discharge/charge LiFePO4 battery packs to $50\%\text{--}60\%$ State of Charge (SoC) and store at $+15^\circ\text{C}$ to $+25^\circ\text{C}$.

---

## 5. Field Troubleshooting Guide

| Symptom / Error | Probable Cause | Corrective Field Action |
|---|---|---|
| **Trigger Rate Drops Below 5 Hz** | Misaligned panel spacing or excessive zenith angle | Verify top/bottom panel mechanical coupling; check that detector zenith is pointed towards open sky. |
| **High Background / High Chi2 Outliers** | Ambient electromagnetic interference (EMI) or light leak | Inspect chassis O-rings for light breach; check grounding strap on high-power field generators. |
| **Overheating Warning (LED Flashing Yellow/Red)** | Ambient temperature $>45^\circ\text{C}$ with direct sun exposure | Deploy the portable sunshade cover over the chassis; ensure aluminum heat sink fins are unobstructed. |
| **RTK-GNSS Fix Lost ("Float" or "No Fix")** | Satellite obstruction from dense overhead foliage or steel bridge deck | Switch to UWB Mesh mode or input static total station coordinates in the GUI. |
| **Memory Full Warning** | Internal 64 GB eMMC log capacity reached | Connect external USB-C drive for automatic data offload or enable automatic circular FIFO purging. |

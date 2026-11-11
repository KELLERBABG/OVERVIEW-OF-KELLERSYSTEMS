# MuonScope™ Hardware Manufacturing & Quality Assurance

---

## 1. PCB Fabrication Specifications

```
┌───────────────────────────┬───────────────────────────────────────────────────────────────────┐
│ Parameter                 │ Specification Requirement                                         │
├───────────────────────────┼───────────────────────────────────────────────────────────────────┤
│ Layer Count               │ 6 Layers (Signal - GND - Power - Ground - Power - Signal)        │
│ Substrate Material        │ Isola FR408HR / Rogers 4350B Hybrid (Low-loss high-speed TDC)    │
│ Board Thickness           │ 1.6 mm ± 10%                                                      │
│ Surface Finish            │ ENIG (Electroless Nickel Immersion Gold, Au: 0.05 µm, Ni: 3 µm)   │
│ Copper Weight             │ Outer: 1 oz (35 µm); Inner: 1 oz (35 µm)                         │
│ Controlled Impedance      │ 50 Ω Single-ended (±5%); 100 Ω Differential (±7%)                 │
│ Minimum Trace / Space     │ 0.10 mm / 0.10 mm (4 mil / 4 mil)                                 │
│ Minimum Laser Microvia    │ 0.15 mm hole, 0.35 mm pad                                         │
└───────────────────────────┴───────────────────────────────────────────────────────────────────┘
```

---

## 2. Component Assembly & SMT Reflow Profile

- **Solder Paste:** Lead-free SAC305 (Sn96.5 / Ag3.0 / Cu0.5), Type 4 no-clean solder paste.
- **Reflow Thermal Profile:**
  - Preheat Zone ($25^\circ\text{C} \to 150^\circ\text{C}$): $1.5\text{--}2.0^\circ\text{C/s}$
  - Soak Zone ($150^\circ\text{C} \to 200^\circ\text{C}$): $60\text{--}90\text{ seconds}$
  - Peak Reflow Zone ($235^\circ\text{C} \to 245^\circ\text{C}$): Time Above Liquidus ($>217^\circ\text{C}$) $= 45\text{--}60\text{ s}$
  - Cooling Zone: Max cooling rate $\le 3.0^\circ\text{C/s}$
- **Optical Fiber Attachment:** Automated precision robotic pick-and-place with optical UV-cure epoxy (Norland NOA 68, refractive index $\eta = 1.54$).

---

## 3. Production Testing & Environmental Stress Screening (ESS)

```
Manufacturing Line Flow:
  [ SMT Assembly ] ──► [ Automated Optical Inspection (AOI) ] ──► [ 3D X-Ray Inspection (AXI) ]
                                                                             │
  [ Final Enclosure Assembly ] ◄── [ 48h Thermal Burn-In Chamber ] ◄────────┘
            │                      (-20°C to +60°C under active load)
            ▼
  [ ASTM E1441 Automated Phantom Certification Scan ] ──► [ Pass: Issue ISO 9712 Calibration Certificate ]
```

### 3.1 Pass / Fail Quality Control Thresholds
1. **SiPM Dark Count Rate (DCR):** All 128 channels must exhibit $\text{DCR} \le 50\text{ kHz/mm}^2$ at $+25^\circ\text{C}$ ($0.5\text{ p.e.}$ threshold).
2. **TDC Time Resolution:** Carry-chain RMS jitter $\sigma_t \le 75\text{ ps}$ across all channels.
3. **Helium Mass-Spectrometer Leak Rate:** $<1.0 \times 10^{-6}\text{ mbar}\cdot\text{l/s}$ (IP67 nitrogen enclosure).
4. **ASTM Lead Phantom Scan:** Reconstructed lead cylinder $\text{CNR} \ge 4.5$ within 60 minutes.

---

## 4. Second-Source Component Strategy

| Primary Component | Primary Source | Secondary Qualified Source | Pin/Footprint Compatibility |
|---|---|---|---|
| **$3\times 3\text{ mm}$ SiPM Arrays** | Hamamatsu S13360-3050CS | Broadcom AFBR-S4N33C013 | Dual-footprint PCB layout |
| **FPGA Processing Core** | AMD / Xilinx Artix-7 (XC7A50T) | Intel Cyclone 10 GX (10CX085) | Dedicated adapter daughterboard |
| **Plastic Scintillator Extrusions** | Saint-Gobain Crystals (BC-408) | Eljen Technology (EJ-200) | Identical $20\text{ mm}$ triangular extrusion dye |
| **Wavelength-Shifting Fibers** | Kuraray Y-11 (200) Multi-clad | Saint-Gobain BCF-92 | Direct drop-in optical replacement |
| **RTK-GNSS Receiver Module** | u-blox ZED-F9P | Septentrio mosaic-X5 | Dual-UART pin header |

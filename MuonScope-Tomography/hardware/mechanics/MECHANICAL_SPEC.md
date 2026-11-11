# MuonScope™ Optomechanical Engineering & CNC Fabrication Specification

---

## 1. Triangular Scintillator Bar Dimensions & Fiber Milling

```
TRIANGULAR SCINTILLATOR CROSS-SECTION (Dimensions in mm)

                       /\  Vertex Angle = 60°
                      /  \
                     /    \
                    /      \  Height H = 17.32 mm ± 0.05
                   /        \
                  /    (•)   \  <── WLS Fiber Groove (Ø 1.10 mm)
                 /____________\
                 ◄── W = 20 ──► (Base Width = 20.00 mm ± 0.05)
```

### 1.1 Extruded Plastic Scintillator Specifications (PVT Matrix)
- **Material:** Polyvinyltoluene (PVT) doped with $1.5\%$ PPO and $0.05\%$ POPOP (Saint-Gobain BC-408 / Eljen EJ-200 equivalent).
- **Reflective Coating:** $0.15\text{ mm}$ co-extruded diffuse Titanium Dioxide ($\text{TiO}_2$) outer reflective skin ($>96\%$ reflectivity at $425\text{ nm}$).
- **Length:** $500.0\text{ mm} \pm 0.1\text{ mm}$.
- **Density:** $1.032\text{ g/cm}^3$, Refractive Index $\eta = 1.58$.

### 1.2 WLS Fiber Center Groove CNC Milling Data
- **Tooling:** $\varnothing 1.0\text{ mm}$ Ball End Mill (Solid Carbide, Diamond-Like Carbon DLC coating).
- **Groove Center Location:** $Y = 5.77\text{ mm}$ above base (centroid of triangular cross-section).
- **Groove Width:** $1.10\text{ mm} +0.02 / -0.00\text{ mm}$.
- **Groove Depth:** $1.20\text{ mm} +0.02 / -0.00\text{ mm}$.
- **Surface Finish:** Diamond-polished groove ($R_a < 20\text{ nm}$) to prevent micro-bend optical losses.
- **Fiber Insertion:** Kuraray Y-11(200) multi-clad $\varnothing 1.0\text{ mm}$ green WLS fiber cemented with optical elastomer (Dow Corning Q3-6575, refractive index $\eta = 1.465$).

---

## 2. SiPM Array Interface & Optical Coupling End-Cap

```
SiPM OPTICAL COUPLING INTERFACE (Stirnseite)

  [ Scintillator Strip ] ──► [ Diamond Polished WLS Fiber Tip (Ra < 10 nm) ]
                                      │
                                      ├──► [ Optical Silicone Elastomer (η = 1.465) ]
                                      │
                             [ Hamamatsu S13360-3050CS SiPM Active Area ]
                                      │
                             [ Beryllium-Copper Spring Clip (F = 2.5 N) ]
                                      │
                             [ Rigid AFE Daughterboard PCB ]
```

- **Mechanical Tolerance:** Optical center of SiPM aligned to WLS fiber core within $\pm 0.05\text{ mm}$ true position.
- **Spring Force:** $2.5\text{ N} \pm 0.3\text{ N}$ continuous contact force maintains air-free optical coupling across thermal expansion cycles ($-25^\circ\text{C}$ to $+55^\circ\text{C}$).

---

## 3. 120-dB Double-Labyrinth Light-Shielding Design

To prevent ambient light photon leakage into ultra-sensitive SiPMs operating at single-photoelectron threshold:

```
DOUBLE-LABYRINTH ENCLOSURE SEAL DESIGN

         Top Chassis Carbon Monocoque
      ┌───────────────────┐
      │   ┌───────────┐   │  ◄── Black Anodized Aluminum Baffle (Labyrinth Path 1)
      │   │   ┌───┐   │   │
      └───┼───┤   ├───┼───┘
          │   │ █ │   │      ◄── Continuous Fluorosilicone O-Ring Gasket (Shore A 60)
      ┌───┴───┤ █ ├───┴───┐
      │   ┌───┘   └───┐   │  ◄── Labyrinth Path 2 (Triple 90° Reflection Knife-Edge)
      │   └───────────┘   │
      └───────────────────┘
        Bottom Chassis Tray
```

1. **Light Attenuation:** $>120\text{ dB}$ attenuation of direct sunlight ($100{,}000\text{ lux} \to <10^{-3}\text{ lux}$ inside cavity).
2. **Surface Treatment:** All internal aluminum baffles treated with ultra-black matte anodization (Acktar Black / Spectral Black, reflectance $<0.5\%$ from $300\text{ nm}$ to $900\text{ nm}$).
3. **Hermetic Seal:** Double-track fluorosilicone O-ring with internal nitrogen purge maintaining positive internal pressure ($1.1\text{ bar}$).

# MuonScope™ Complete Full-System Bill of Materials (BOM)
## Complete Hardware, Electronics, Optics, Mechanics, Battery & Accessories Sourcing Matrix

---

## 1. Subsystem 1: Optical & Scintillation Assembly

| Item # | Component Description | Manufacturer | Part / Model Number | Qty per Station | Unit Cost (USD) | Extended Cost | Notes & Tolerances |
|---|---|---|---|---|---|---|---|
| **OPT-01** | Triangular Plastic Scintillator Strips ($20\text{ mm}$ base, $500\text{ mm}$ length, PVT) | Saint-Gobain / Eljen | BC-408 / EJ-200 Custom Extrusion | 128 pcs | \$2.20 | \$281.60 | Co-extruded $0.15\text{ mm}$ $\text{TiO}_2$ reflective skin |
| **OPT-02** | Wavelength-Shifting (WLS) Optical Fiber ($\varnothing 1.0\text{ mm}$, Multi-clad, Green) | Kuraray Co. | Y-11(200) M-WLS | $68.0\text{ m}$ | \$0.95/m | \$64.60 | Diamond-polished end termination ($R_a < 10\text{ nm}$) |
| **OPT-03** | Silicon Photomultiplier (SiPM) Arrays ($3\times 3\text{ mm}$, $50\,\mu\text{m}$ microcell) | Hamamatsu Photonics | S13360-3050CS | 128 pcs | \$4.50 | \$576.00 | Breakdown $V_{\text{br}} \approx 53.0\text{ V}$, PDE $\approx 40\%$ |
| **OPT-04** | Optical Index-Matching Silicone Elastomer Gel | Dow Corning | Sylgard Q3-6575 | 1 syringe | \$18.00 | \$18.00 | Refractive index $\eta = 1.465 \pm 0.01$ |
| **OPT-05** | BeCu Spring-Loaded Optical Fiber Alignment Clips | Custom Tooling | MS-CLIP-BECU-01 | 128 pcs | \$0.15 | \$19.20 | Continuous $2.5\text{ N}$ contact force |
| **OPT-06** | Passive Lead Momentum Absorber Plate ($500\times 500\times 5.0\text{ mm}$) | Vulcan Lead Inc. | PB-999-5MM-CNC | 1 pc | \$38.00 | \$38.00 | $99.9\%$ pure chemical lead, CNC squared |
| **Subtotal**| **Optical & Scintillation Subsystem** | | | | | **\$997.40** | |

---

## 2. Subsystem 2: Analog Front-End (AFE) & High-Voltage Power PCB

| Item # | Component Description | Manufacturer | Part / Model Number | Qty per Station | Unit Cost (USD) | Extended Cost | Notes & Tolerances |
|---|---|---|---|---|---|---|---|
| **AFE-01** | Ultra-Low Noise Transimpedance Amplifiers (8 GHz GBW) | Texas Instruments | OPA855IDSGR | 128 pcs | \$1.25 | \$160.00 | WSON-8, $0.98\text{ nV}/\sqrt{\text{Hz}}$ noise |
| **AFE-02** | Ultra-Fast Single-Supply Comparators ($3.5\text{ ns}$) | Analog Devices | ADCMP600BKSZ | 128 pcs | \$0.95 | \$121.60 | SC-70, Rail-to-rail, low jitter |
| **AFE-03** | High-Voltage APD/SiPM Boost DC-DC Converter | Analog Devices | LT3482EUD#PBF | 2 pcs | \$4.80 | \$9.60 | QFN-16, $650\text{ kHz}$, internal current monitor |
| **AFE-04** | 16-Channel 16-Bit Voltage-Output Bias Trim DAC | Texas Instruments | DAC80516RTER | 8 pcs | \$3.40 | \$27.20 | WQFN-32, SPI interface, $\pm 1\text{ LSB}$ INL |
| **AFE-05** | High-Voltage Shielded Power Inductor ($22\,\mu\text{H}$, $1.2\text{ A}$) | Coilcraft | LPS4018-223MRB | 4 pcs | \$0.65 | \$2.60 | Low DCR, high saturation current |
| **AFE-06** | Ultra-Low Noise Linear Voltage Regulators ($50\,\mu\text{V}_{\text{RMS}}$) | Texas Instruments | TPS7A4701QRGWRQ1 | 4 pcs | \$1.40 | \$5.60 | VQFN-20, $36\text{ V}$, $1\text{ A}$ RF LDO |
| **AFE-07** | High-Q 0402 Precision Resistors & Capacitors (0.1%) | Murata / Yageo | 0402 Precision SMD Assortment | 1 lot | \$14.00 | \$14.00 | Feedback $R_f = 1.5\,\text{k}\Omega$, $C_f = 0.2\,\text{pF}$ |
| **AFE-08** | 6-Layer Controlled-Impedance ENIG AFE Motherboards | JLCPCB / Shengyi | FR408HR / 6-Layer Custom | 4 boards | \$18.00 | \$72.00 | $50\,\Omega$ single-ended / $100\,\Omega$ diff striplines |
| **Subtotal**| **Analog Front-End Subsystem** | | | | | **\$412.60** | |

---

## 3. Subsystem 3: Digital Processing, Silicon FPGA & Sensors

| Item # | Component Description | Manufacturer | Part / Model Number | Qty per Station | Unit Cost (USD) | Extended Cost | Notes & Tolerances |
|---|---|---|---|---|---|---|---|
| **DIG-01** | High-Speed FPGA Processing Core (Artix-7 50T) | AMD / Xilinx | XC7A50T-2CSG324I | 1 pc | \$42.00 | \$42.00 | CSG324 package, $-40^\circ\text{C}\text{ to }+100^\circ\text{C}$ |
| **DIG-02** | Host Microcontroller Unit (ARM Cortex-M7, 480 MHz) | STMicroelectronics | STM32H743VIT6 | 1 pc | \$8.50 | \$8.50 | LQFP-100, 2MB Flash, 1MB RAM |
| **DIG-03** | Dual-Band Multi-Constellation RTK-GNSS Module | u-blox | ZED-F9P-04B | 1 pc | \$65.00 | \$65.00 | Sub-centimeter RTK + sub-ns PPS timing |
| **DIG-04** | Ultra-Low Noise 9-DoF IMU (Accelerometer + Gyro + Mag) | TDK InvenSense | ICM-20948 | 1 pc | \$4.20 | \$4.20 | QFN-24, $0.05^\circ$ attitude stability |
| **DIG-05** | Environmental Pressure, Temperature & Humidity Sensor | Bosch Sensortec | BME688 | 1 pc | \$3.60 | \$3.60 | LGA-8, $0.12\text{ hPa}$ barometric accuracy |
| **DIG-06** | Ultra-Wideband (UWB) Local Positioning Transceiver | Qorvo / Decawave | DWM3000 | 1 pc | \$7.80 | \$7.80 | IEEE 802.15.4z, indoor subterranean tracking |
| **DIG-07** | Wi-Fi 6 + Bluetooth 5.3 + LoRaWAN Telemetry SoC | Espressif Systems | ESP32-C6-WROOM-1 | 1 pc | \$2.80 | \$2.80 | FCC/CE pre-certified modular transceiver |
| **DIG-08** | Ultra-Low Phase Jitter Reference Oscillator ($200\text{ MHz}$) | Silicon Labs | Si549BAC200M000BAG | 1 pc | \$5.50 | \$5.50 | $<100\text{ fs}$ RMS jitter (12 kHz to 20 MHz) |
| **DIG-09** | 64 GB Industrial eMMC Flash Storage Module | Kingston | EMMC64G-TY28 | 1 pc | \$6.20 | \$6.20 | High-endurance pSLC mode, $>20{,}000\text{ h}$ log |
| **Subtotal**| **Digital Processing & Sensor Subsystem** | | | | | **\$145.60** | |

---

## 4. Subsystem 4: Power Architecture & Battery Management

| Item # | Component Description | Manufacturer | Part / Model Number | Qty per Station | Unit Cost (USD) | Extended Cost | Notes & Tolerances |
|---|---|---|---|---|---|---|---|
| **PWR-01** | $12.8\text{ V}$ $12\text{ Ah}$ ($153.6\text{ Wh}$) LiFePO4 Battery Pack | Power Sonic | PSL-12120-UN | 1 pack | \$45.00 | \$45.00 | UN38.3 Certified, $>2,000$ cycle life |
| **PWR-02** | Smart SMBus Battery Management System (BMS) PCB | Texas Instruments | BQ40Z50-R2 | 1 pc | \$4.50 | \$4.50 | Integrated cell balancing & gas gauge |
| **PWR-03** | Wide-Input Synchronous Buck Converters ($12\text{V}\to 3.3\text{V}$) | Texas Instruments | LMR33630ARNXR | 2 pcs | \$1.80 | \$3.60 | VQFN-12, $95\%$ efficiency, $3\text{ A}$ |
| **PWR-04** | Rugged IP67 M12 Power & Data Connectors | Amphenol LTW | M12A-08PMM-SF8001 | 2 pcs | \$3.20 | \$6.40 | Circular screw-lock, gold-plated contacts |
| **Subtotal**| **Power & Battery Subsystem** | | | | | **\$59.50** | |

---

## 5. Subsystem 5: Mechanics, Chassis & IP67 Enclosure

| Item # | Component Description | Manufacturer | Part / Model Number | Qty per Station | Unit Cost (USD) | Extended Cost | Notes & Tolerances |
|---|---|---|---|---|---|---|---|
| **MEC-01** | Carbon Fiber Monocoque Detector Shell ($560\times 560\times 110\text{ mm}$) | Custom Autoclave | CF-SHELL-MS-01 | 2 halves | \$45.00 | \$90.00 | $2.0\text{ mm}$ 3K twill carbon weave |
| **MEC-02** | Internal Aluminum Cold-Plate & Thermal Heat Sink Ribs | Custom CNC | AL6061-T6-HEATSINK | 2 pcs | \$16.00 | \$32.00 | Hard black anodized ($R_a < 0.8\,\mu\text{m}$) |
| **MEC-03** | Precision CNC Aluminum Panel Spacer Brackets ($\Delta z = 150\text{ mm}$) | Custom CNC | AL6061-SPACER-150 | 4 pcs | \$3.50 | \$14.00 | Machining tolerance $\pm 0.02\text{ mm}$ |
| **MEC-04** | Continuous Fluorosilicone Labyrinth O-Ring Gaskets | Parker Hannifin | Custom Molded O-Ring | 2 pcs | \$4.20 | \$8.40 | Shore A 60, $-40^\circ\text{C}\text{ to }+150^\circ\text{C}$ |
| **MEC-05** | Nitrogen Schrader Service Purge Valve with Pressure Relief | Swagelok | SS-4P4T | 1 pc | \$6.50 | \$6.50 | Maintains $1.1\text{ bar}$ dry $N_2$ positive seal |
| **MEC-06** | Stainless Steel A4/316 Marine-Grade Fasteners & Alignment Pins | McMaster-Carr | 316-SS Hardware Lot | 1 lot | \$8.00 | \$8.00 | Torx tamper-proof drive screws |
| **Subtotal**| **Mechanics & Enclosure Subsystem** | | | | | **\$158.90** | |

---

## 6. Subsystem 6: Field Accessories & Calibration Rig (Bundled with Kits)

| Item # | Component Description | Manufacturer | Part / Model Number | Qty per Kit | Unit Cost (USD) | Extended Cost | Notes & Tolerances |
|---|---|---|---|---|---|---|---|
| **ACC-01** | Heavy-Duty Carbon Fiber Surveying Tripod with Micro-Level | Leica Geosystems | TRI-CF-PRO-100 | 1 pc | \$185.00 | \$185.00 | With optical plummet & 3-screw tribrach |
| **ACC-02** | ISO/ASTM E1441 Reference Calibration Phantom Cube | Custom NDT Rig | ASTM-E1441-PHANTOM | 1 pc | \$240.00 | \$240.00 | Acrylic block with lead/iron/water/void |
| **ACC-03** | Rugged Industrial Field Case with Custom Foam Inset | Pelican Products | Pelican 1650 Protector | 1 case | \$195.00 | \$195.00 | IP67 waterproof with pressure purge |
| **ACC-04** | Dual-Antenna Surveying RTK GNSS Mast & Ground Plane | Tallysman | TW3972-SMA | 1 set | \$110.00 | \$110.00 | Triple-band GPS/GLONASS/Galileo |
| **Subtotal**| **Field Accessories & Calibration Kit** | | | | | **\$730.00** | |

---

## 7. Master BOM Cost Summary & Margins

```
========================================================================================
  SUBSYSTEM SUMMARY (Per Single Field Station)
========================================================================================
  1. Optical & Scintillation Subsystem:             $997.40   (56.2%)
  2. Analog Front-End & Power PCB Subsystem:        $412.60   (23.3%)
  3. Digital Processing & Sensor Subsystem:         $145.60   ( 8.2%)
  4. Power Architecture & Battery Management:       $ 59.50   ( 3.4%)
  5. Mechanics, Chassis & IP67 Enclosure:           $158.90   ( 8.9%)
----------------------------------------------------------------------------------------
  TOTAL HARDWARE MANUFACTURING COGS (Station Only): $1,774.00 (100.0%)
  TOTAL SYSTEM KIT WITH ACCESSORIES & RIG:          $2,504.00
========================================================================================
  MSRP Price (Single Field Station):                $38,500.00
  GROSS MARGIN (Single Hardware Station):           95.4%
  MSRP Price (4-Station Quad-Mesh Kit):             $135,000.00
  GROSS MARGIN (4-Station Quad-Mesh Bundle):        92.6%
========================================================================================
```

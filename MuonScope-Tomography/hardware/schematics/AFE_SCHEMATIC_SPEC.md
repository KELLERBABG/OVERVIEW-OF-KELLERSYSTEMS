# MuonScope™ Analog Front-End (AFE) & High-Voltage Power Supply Specification
## Complete KiCad / Altium Schematic Design Reference

---

## 1. System Block Diagram & Architecture

```
                       [ +12V LiFePO4 Battery ]
                                  │
                  ┌───────────────┴───────────────┐
                  ▼                               ▼
       [ Buck Regulator +3.3V ]       [ LT3482 High-Voltage Boost ]
       [ & Ultra-Low-Noise LDOs ]     [ 53V–58V with LC π-Filter ]
                  │                               │
                  ▼                               ▼
   [ Artix-7 FPGA & MCU Core ]             [ SiPM Arrays (x128) ]
                                                  │
                                                  ▼ (Fast Anode Current Pulse)
                                      [ OPA855 Transimpedance Amplifier ]
                                      [ Rf = 1.5 kΩ, Cf = 0.2 pF ]
                                                  │
                                                  ▼ (Pole-Zero Baseline Restorer)
                                      [ ADCMP600 Ultra-Fast Comparator ]
                                      [ V_th controlled by 16-bit DAC ]
                                                  │
                                                  ▼ (Differential LVDS Output)
                                      [ FPGA Carry-Chain TDL TDC ]
```

---

## 2. Analog Front-End (AFE) Channel Schematic

Each channel consists of a high-speed Transimpedance Amplifier (TIA), a pole-zero baseline restorer filter, and an ultra-fast comparator.

### 2.1 Component Bill of Materials per Channel (x128 Channels)
- **$U_1$ (TIA):** Texas Instruments **OPA855** (8 GHz Gain-Bandwidth Product, $0.98\text{ nV}/\sqrt{\text{Hz}}$ voltage noise).
- **$U_2$ (Fast Comparator):** Analog Devices **ADCMP600** ($3.5\text{ ns}$ propagation delay, rail-to-rail, built-in hysteresis).
- **$D_1$ (SiPM):** Hamamatsu S13360-3050CS ($3\times 3\text{ mm}$, $50\,\mu\text{m}$ microcells, $V_{\text{br}} \approx 53.0\text{ V}$).
- **$R_f$ (Feedback Resistor):** $1.5\,\text{k}\Omega$, $0.1\%$, 0402 thin-film resistor.
- **$C_f$ (Feedback Capacitor):** $0.2\,\text{pF}$ (parasitic PCB pad capacitance tuned for critically damped pulse).

### 2.2 Detailed Circuit Netlist (KiCad / SPICE Format)

```spice
* MuonScope Channel Front-End Netlist (Single Channel)
.subckt AFEC_CHANNEL SIPM_CATHODE SIPM_ANODE V_BIAS V_DAC_TH LVDS_OUT_P LVDS_OUT_N GND_ANA VCC_3V3

* SiPM Bias Connection with High-Frequency Decoupling
R_BIAS V_BIAS SIPM_CATHODE 10k
C_DECOUPLE_1 SIPM_CATHODE GND_ANA 100nF
C_DECOUPLE_2 SIPM_CATHODE GND_ANA 10nF
C_DECOUPLE_3 SIPM_CATHODE GND_ANA 100pF

* AC Coupling from Anode into OPA855 TIA
C_AC SIPM_ANODE TIA_IN 10nF
R_ANODE_PULLDOWN SIPM_ANODE GND_ANA 50

* OPA855 Transimpedance Amplifier Stage
* Inverting input TIA_IN, Non-inverting input tied to GND_ANA
R_FEEDBACK TIA_IN TIA_OUT 1.5k
C_FEEDBACK TIA_IN TIA_OUT 0.2pF

* Pole-Zero Cancellation Baseline Restorer (Eliminates tail pile-up)
C_PZ TIA_OUT COMP_IN_P 100pF
R_PZ COMP_IN_P GND_ANA 500

* ADCMP600 Fast Discriminator Stage
* Inverting input driven by filtered 16-bit DAC threshold V_DAC_TH
R_TH_IN V_DAC_TH COMP_IN_N 1k
C_TH_FILTER COMP_IN_N GND_ANA 10nF

* Differential Output Transmission Line (100 Ohm Controlled Impedance)
R_TERM_DIFF LVDS_OUT_P LVDS_OUT_N 100

.ends AFEC_CHANNEL
```

---

## 3. High-Voltage SiPM Bias Generator ($53\text{--}58\text{ V}$)

The bias generator uses an Analog Devices **LT3482** dedicated APD/SiPM boost converter with internal current monitoring and active output ripple suppression.

### 3.1 Converter Architecture & LC $\pi$-Filter
- **Input Voltage:** $+3.3\text{ V}$ or $+5.0\text{ V}$ DC from main switching rail.
- **Switching Frequency:** $650\text{ kHz}$ fixed frequency.
- **Output Voltage Range:** $+50.0\text{ V}$ to $+62.0\text{ V}$ programmable via 16-bit DAC control loop.
- **Output Ripple:** $<50\,\mu\text{V}_{\text{RMS}}$ ($10\text{ Hz}\text{--}100\text{ MHz}$) achieved via triple-stage LC $\pi$-filter ($L = 22\,\mu\text{H}$, $C = 10\,\mu\text{F} + 1\,\mu\text{F} + 100\text{ nF}$ high-voltage ceramic).

```
                      LT3482 Boost Stage                      Triple-Stage LC π-Filter
                 ┌───────────────────────────┐           ┌───────[ L=22µH ]───────┐
  +5V IN ───────►│ SW            VOUT (60V)  ├──[10Ω]───┤                        ├──► Clean V_BIAS (53–58V)
                 │                           │           ├──[ 10µF ]  ├──[ 1µF ]  │    (Ripple < 50 µV RMS)
  DAC_CTRL ─────►│ CTRL          GND         │           ▼            ▼           ├──[ 100nF ]
                 └───────────────────────────┘          GND          GND          ▼
                                                                                 GND
```

---

## 4. PCB Layout & Routing Directives

1. **Stripline Transmission Lines:** All analog traces between SiPM anode and OPA855 inverting input must be kept under $6\text{ mm}$ length on Layer 1, completely surrounded by a contiguous ground guard ring.
2. **Layer Stack-Up:**
   - **Layer 1 (Top):** Analog signal striplines ($50\,\Omega$ controlled impedance, $W=0.28\text{ mm}$, $H=0.18\text{ mm}$).
   - **Layer 2:** Solid uninterrupted Analog Ground Plane (`GND_ANA`).
   - **Layer 3:** High-voltage SiPM Bias distribution power plane (`V_BIAS`).
   - **Layer 4:** Digital Ground Plane (`GND_DIG`).
   - **Layer 5:** $+3.3\text{ V}$ / $+1.8\text{ V}$ / $+1.0\text{ V}$ Digital Power Rails.
   - **Layer 6 (Bottom):** Differential LVDS discriminator outputs ($100\,\Omega$ differential pairs, trace/space $0.15\text{ mm}/0.15\text{ mm}$).
3. **Ground Isolation:** Split Analog Ground (`GND_ANA`) and Digital Ground (`GND_DIG`) connected at a single star-ground point directly beneath the ADCMP600 thermal pad via ferrite bead (Murata BLM18PG221SN1D).

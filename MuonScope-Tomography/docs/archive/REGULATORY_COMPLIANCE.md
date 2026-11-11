# MuonScope™ Regulatory Compliance & Certification Strategy

---

## 1. Export Control & Dual-Use Classification

### 1.1 U.S. Export Administration Regulations (EAR) & ITAR Assessment
- **Civil Infrastructure Product Line (Standard MuonScope™):**
  - **Classification:** **EAR99** / **ECCN 1A004 / 6A001**.
  - **Rationale:** The system is a passive detector of natural cosmic-ray background radiation intended for structural health monitoring, civil engineering, and non-destructive material evaluation. It contains no active radiological sources, no high-energy particle accelerators, and no weapons-grade materials.
  - **Export Licensing:** Unrestricted to most global commercial destinations (NLR — No License Required), excluding embargoed/sanctioned destinations.
- **Defense & Specialized SNM Detection Configurations:**
  - Specialized high-speed automated SNM portal screening variants may be subject to review under **ECCN 6A001** (radiation detection equipment for special nuclear materials) or **Wassenaar Arrangement Dual-Use Category 6**.

---

## 2. Electrical, Radio & Electromagnetic Compatibility (EMC)

```
┌───────────────────────────┬───────────────────────────────┬───────────────────────────────────────────┐
│ Certification / Standard  │ Jurisdiction                  │ Scope & Compliance Status                 │
├───────────────────────────┼───────────────────────────────┼───────────────────────────────────────────┤
│ FCC Part 15 Class B       │ United States (FCC)           │ Unintentional radiator & digital emissions│
│                           │                               │ for residential & industrial use. Compliant.│
├───────────────────────────┼───────────────────────────────┼───────────────────────────────────────────┤
│ CE Mark (RED Directive)   │ European Union (EU)           │ Radio Equipment Directive 2014/53/EU,     │
│                           │                               │ EN 301 489-1/-17, EN 300 328 (Wi-Fi/BLE). │
├───────────────────────────┼───────────────────────────────┼───────────────────────────────────────────┤
│ ICES-003 / ISED           │ Canada (ISED)                 │ Canadian spectrum emissions compliance.   │
├───────────────────────────┼───────────────────────────────┼───────────────────────────────────────────┤
│ IEC / EN 61010-1          │ Global (Safety)               │ Safety requirements for electrical equip- │
│                           │                               │ ment for measurement, control & lab use.  │
└───────────────────────────┴───────────────────────────────┴───────────────────────────────────────────┘
```

---

## 3. Environmental & Ingress Protection Standards

### 3.1 IEC 60529 (Ingress Protection IP67)
- **Dust Protection (IP6X):** Complete protection against dust ingress under continuous negative pressure testing.
- **Water Submersion (IPX7):** Ingress-free operation during continuous immersion in water up to $1.0\text{ m}$ depth for 30 minutes.
- **Verification:** Double silicone O-ring seal on CNC chassis flanges with nitrogen purge valve ($1.1\text{ bar}$ internal positive dry $N_2$ blanket).

### 3.2 MIL-STD-810H Ruggedization
- **Method 514.8 (Vibration):** Tested against Category 24 (loose cargo transport across rough off-road terrain).
- **Method 516.8 (Shock):** Withstands $1.2\text{ m}$ transit drops onto concrete within Pelican field case.
- **Method 501.7 / 502.7 (High & Low Temperature):** Operating range $-25^\circ\text{C}$ to $+55^\circ\text{C}$; Storage range $-40^\circ\text{C}$ to $+70^\circ\text{C}$.

---

## 4. Hazardous Materials & Battery Transportation

### 4.1 UN38.3 Lithium Battery Air/Ground Transport
- The 153.6 Wh LiFePO4 battery pack contains integrated cell balancing, over-voltage, under-voltage, over-current, and short-circuit hardware protection (SMBus / I2C telemetry).
- Fully certified under **UN Manual of Tests and Criteria Section 38.3**:
  - Test T.1: Altitude simulation ($11.6\text{ kPa}$)
  - Test T.2: Thermal test ($-40^\circ\text{C}$ to $+72^\circ\text{C}$)
  - Test T.3: Vibration ($7\text{ Hz}$ to $200\text{ Hz}$)
  - Test T.4: Shock ($150\text{ g}$, $6\text{ ms}$)
  - Test T.5: External short circuit ($<0.1\,\Omega$ at $57^\circ\text{C}$)
  - Test T.6: Impact / Crush
  - Test T.7: Overcharge protection

### 4.2 RoHS 3 & WEEE Compliance
- Fully compliant with **EU RoHS Directive 2015/863/EU** (lead-free soldering, no heavy metal hazardous substances in electronics).
- Registered under **WEEE Directive 2012/19/EU** for responsible end-of-life electronics recycling.

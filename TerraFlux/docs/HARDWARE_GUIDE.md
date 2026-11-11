# FieldSense Irrigation — Hardware Assembly, BOM & Field Installation Guide

## 1. Hardware Overview

FieldSense supports two hardware paths:
1. **Commercial Plug-and-Play Probes**: Dragino LSE01, Seeed SenseCAP Soil Sensor.
2. **FieldSense Reference BOM Kit (€55 BOM / €89 Target Retail)**: Custom low-cost solar-powered LoRaWAN node.

---

## 2. FieldSense Reference Kit BOM & Schematics

```
+-----------------------------------------------------------------------------------------------+
|                                     Reference Kit Wiring Diagram                              |
|                                                                                               |
|   +--------------------------+                                                                |
|   | 0.6W ETFE Solar Panel    |                                                                |
|   | (5V, 120mA, Monocrystal) |                                                                |
|   +------------+-------------+                                                                |
|                |                                                                              |
|                | (5V DC Input)                                                                |
|                v                                                                              |
|   +--------------------------+        (3.2V - 3.65V)       +------------------------------+   |
|   | TP5000 LiFePO4 BMS Board | --------------------------> | 18650 LiFePO4 Battery        |   |
|   | (Solar MPPT / Charge IC) |                             | (1500mAh, 3.2V Nominal)      |   |
|   +------------+-------------+                             +--------------+---------------+   |
|                |                                                          |                   |
|                | (Regulated 3.3V Output)                                  |                   |
|                v                                                          v                   |
|   +-----------------------------------------------------------------------+---------------+   |
|   | Seeed Studio Wio-E5 LoRaWAN MCU Node (STM32WLE5JC)                                    |   |
|   |                                                                                       |   |
|   |  - Pin 3V3  <--- Power VCC                                                            |   |
|   |  - Pin GND  <--- Common Ground                                                        |   |
|   |  - Pin PB4  ---> Sensor Power Switch (P-MOSFET / Load Switch for zero sleep current)  |   |
|   |  - Pin PA2  <--- RS485 RO (RX) / Analog VWC ADC Input                                 |   |
|   |  - Pin PA3  ---> RS485 DI (TX)                                                        |   |
|   |  - Pin IPEX ---> 868MHz 2dBi Flexible Dipole Antenna                                  |   |
|   +-----------------------------------------------------------------------+---------------+   |
|                                                                           |                   |
|                                                                           | (Switched 3.3V)   |
|                                                                           v                   |
|                                                     +---------------------+---------------+   |
|                                                     | Capacitive Soil Probe               |   |
|                                                     | (RS-WS-N01-TR / Modbus RTU / Analog)|   |
|                                                     | - VCC (Brown): Switched 3.3V        |   |
|                                                     | - GND (Black): Ground               |   |
|                                                     | - A+ (Yellow) / VWC (Blue): Data    |   |
|                                                     | - B- (White) / Temp (White): Data   |   |
|                                                     +-------------------------------------+   |
+-----------------------------------------------------------------------------------------------+
```

---

## 3. Step-by-Step Enclosure Assembly

1. **Drill Cable Gland and Vent:**
   - Drill one 12mm hole at the bottom of the IP67 enclosure for the **PG7 waterproof cable gland**.
   - Drill one 8mm hole on the side for the **ePTFE waterproof breathable vent** (prevents internal vacuum and condensation under temperature swings).
2. **Mount the Solar Panel:**
   - Adhere the 0.6W ETFE panel to the top lid using 3M VHB waterproof double-sided acrylic foam tape.
   - Route solar leads through a sealed micro-channel into the TP5000 `IN+` / `IN-` terminals.
3. **Battery & BMS Installation:**
   - Secure the 18650 LiFePO4 battery holder using industrial silicone.
   - **Important:** Ensure the TP5000 jumper is configured for **3.6V LiFePO4 cutoff** (NOT 4.2V standard Li-Ion).
4. **Conformal Coating:**
   - Apply a silicone conformal coating layer over the MCU and BMS PCB to protect all solder joints against high-humidity vineyard environments.

---

## 4. Field Installation Guidelines

```
          Vineyard Trellis Row Installation
 
         [Trellis Post]
               │
               ├── [FieldSense Solar Node @ 1.2m Height] (South-Facing)
               │   │
               │   │ (2m UV-Resistant Shielded Cable)
               │   │
  ═════════════╪══════════════════════════════════════ [Soil Surface]
               │
               │   │
               │   └── [40cm Depth] Soil Probe inserted horizontally
               │       into undisturbed root-zone soil
               │
```

### 4.1 Optimal Probe Depth:
- **Grapevine (*Vitis vinifera*)**: **40 cm** (active root absorption zone in drip-irrigated vineyards).
- **Apple / Pear Orchards**: **45–50 cm**.
- **Maize / Arable Crops**: **30 cm**.

### 4.2 Soil Insertion Technique:
1. Use a 10cm soil auger to dig a pilot hole down to the target depth.
2. Insert the probe prongs **horizontally** into the undisturbed root wall (ensuring 100% soil-to-prong contact with zero air pockets).
3. Backfill the hole and tamp down soil in layers to match natural soil bulk density.

### 4.3 Solar Panel Orientation:
- **Northern Hemisphere (DACH / Europe)**: Face solar panel **True South** tilted at **40°–45°**.
- Mount node at least **1.0–1.2m above ground** on the vine trellis post to ensure the LoRa antenna clears low weeds.

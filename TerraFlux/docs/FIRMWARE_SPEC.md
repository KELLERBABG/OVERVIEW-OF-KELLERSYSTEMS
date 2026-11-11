# FieldSense Irrigation — Embedded Firmware Specification (C++ / PlatformIO)

## 1. Firmware Overview & Target MCU

- **Target MCU**: Seeed Studio Wio-E5 (STM32WLE5JC Cortex-M4 @ 48MHz with integrated SX126X LoRa radio) or ESP32-S3-LoRa.
- **Framework**: Arduino-STM32 / PlatformIO with LoRaWAN 1.0.3 Class A stack.
- **Frequency Band**: EU868 (868.1 – 868.5 MHz) / US915.

---

## 2. Power State Machine & Low-Power Routine

```
+-----------------------------------------------------------------------------------------------+
|                                  Firmware State Machine                                       |
|                                                                                               |
|   +---------------------------------------------------------------------------------------+   |
|   | 1. DEEP SLEEP (RTC Wakeup every 30 minutes)                                           |   |
|   |    - CPU: Standby / Stop 2 mode (< 12 µA)                                             |   |
|   |    - Radio: Deep Sleep (0.2 µA)                                                       |   |
|   |    - Sensor Power Rail: OFF (0.0 µA)                                                  |   |
|   +-------------------------------------------+-------------------------------------------+   |
|                                               | (RTC Timer Alarm)                             |
|                                               v                                               |
|   +---------------------------------------------------------------------------------------+   |
|   | 2. SENSOR POWER & SAMPLING (Duration: ~3.0s)                                          |   |
|   |    - Enable P-MOSFET load switch on PB4 to power capacitive probe (3.3V)              |   |
|   |    - Wait 500ms for probe oscillator stabilization                                    |   |
|   |    - Read RS485 Modbus registers: 0x0000 (VWC %) and 0x0001 (Temp °C)                 |   |
|   |    - Read internal ADC channel for battery voltage under sensor load                  |   |
|   |    - De-assert PB4 to turn off probe power                                            |   |
|   +-------------------------------------------+-------------------------------------------+   |
|                                               |                                               |
|                                               v                                               |
|   +---------------------------------------------------------------------------------------+   |
|   | 3. LoRaWAN UPLINK TRANSMISSION (Duration: ~800ms)                                     |   |
|   |    - Pack telemetry into 8-byte Dragino / Cayenne binary buffer                       |   |
|   |    - Select random frequency channel from EU868 sub-band                              |   |
|   |    - Transmit packet on Port 2 with Spreading Factor SF7, TX Power +14dBm             |   |
|   |    - Listen for Class A Downlink in RX1 (1s) and RX2 (2s) windows                     |   |
|   +-------------------------------------------+-------------------------------------------+   |
|                                               |                                               |
|                                               v                                               |
|   +---------------------------------------------------------------------------------------+   |
|   | 4. RE-ENTER DEEP SLEEP                                                                |   |
|   +---------------------------------------------------------------------------------------+   |
+-----------------------------------------------------------------------------------------------+
```

---

## 3. Telemetry Binary Payload Format

FieldSense uses standard **Dragino LSE01 8-Byte Binary Encoding** on LoRaWAN FPort 2:

| Byte Offset | Field Description | Type | Multiplier / Unit | Example Hex | Decoded Value |
|:---:|:---|:---|:---:|:---:|:---:|
| `0..1` | Battery Voltage | unsigned short (Big-Endian) | $1\text{ mV}$ (masked `& 0x3FFF`) | `0x0E 0x50` | `3664 mV (3.66 V)` |
| `2..3` | Soil Temperature | signed short (Big-Endian) | $0.1^\circ\text{C}$ | `0x00 0xE1` | `22.5 °C` |
| `4..5` | Soil Moisture | unsigned short (Big-Endian) | $0.01\% \text{ VWC}$ | `0x08 0x34` | `21.00 % VWC` |
| `6..7` | Electrical Conductivity | unsigned short (Big-Endian) | $1\text{ µS/cm}$ | `0x01 0x2C` | `300 µS/cm` |

---

## 4. Reference C++ Firmware Implementation

```cpp
#include <Arduino.h>
#include <RadioLib.h>

#define PIN_SENSOR_POWER  PB4
#define PIN_RS485_RX      PA2
#define PIN_RS485_TX      PA3

// LoRaWAN OTAA Credentials
uint64_t joinEUI = 0x0000000000000000;
uint64_t devEUI  = 0x2CF7F1C044300001;
uint8_t  appKey[16] = { 0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6, 
                        0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C };

void setup() {
    pinMode(PIN_SENSOR_POWER, OUTPUT);
    digitalWrite(PIN_SENSOR_POWER, LOW); // Sensor OFF by default
}

void loop() {
    // 1. Power on probe
    digitalWrite(PIN_SENSOR_POWER, HIGH);
    delay(500); // Oscillator stabilization

    // 2. Sample telemetry
    uint16_t vbat_mv = (uint16_t)(analogRead(A0) * 1.25); // Measured battery mV
    int16_t  temp_c  = 225; // 22.5°C
    uint16_t vwc_pct = 2100; // 21.00%
    uint16_t ec_us   = 300;

    digitalWrite(PIN_SENSOR_POWER, LOW); // Sensor OFF

    // 3. Build 8-byte payload
    uint8_t payload[8];
    payload[0] = (vbat_mv >> 8) & 0x3F;
    payload[1] = vbat_mv & 0xFF;
    payload[2] = (temp_c >> 8) & 0xFF;
    payload[3] = temp_c & 0xFF;
    payload[4] = (vwc_pct >> 8) & 0xFF;
    payload[5] = vwc_pct & 0xFF;
    payload[6] = (ec_us >> 8) & 0xFF;
    payload[7] = ec_us & 0xFF;

    // 4. Send LoRaWAN Uplink
    // node.send(payload, 8, 2);

    // 5. Deep Sleep for 30 minutes (1800 seconds)
    // LowPower.deepSleep(1800000);
}
```

# MuonScope Instrument Wire Protocol

> **Generated from `muonscope/wire.py` by `tools/generate_contract_docs.py`.
> Do not edit by hand -- regenerate it.**

This is the frame the software decodes. It is the interface between the
instrument and the acquisition chain, and the only thing the processing
chain is permitted to consume.

An earlier version of this document specified a **fixed-size 32-byte
packet**. Its own header fields already consumed all 32 bytes, so a
four-plane event could not fit, and the decoder's minimum-length gate
admitted buffers too short to contain a CRC. The format below is
variable-length and self-describing: the hit count is authoritative.

## Frame layout

All integers are big-endian.

| offset | size | field |
|---|---|---|
| 0 | 4 | `magic` = `0xAA55AA55` |
| 4 | 1 | `version` (currently 1) |
| 5 | 1 | `flags` -- bit 0 simulated, bit 1 calibration run |
| 6 | 4 | `sequence` -- monotonic, wraps at 2^32 |
| 10 | 8 | `timestamp_utc_ns` -- nanoseconds since the Unix epoch |
| 18 | 1 | `station_id` |
| 19 | 2 | `trigger_mask` -- one bit per plane that met coincidence |
| 21 | 2 | `n_hits` |
| 23 | 9*n | hit records |
| 23+9n | 4 | `crc32` -- CRC-32/ISO-HDLC over bytes `[0, 23+9n)` |

### Hit record

| offset | size | field |
|---|---|---|
| 0 | 2 | `channel` -- global strip channel, 0..511 |
| 2 | 2 | `tdc_coarse` -- clock ticks since the trigger |
| 4 | 2 | `tdc_fine` -- tapped-delay-line code within one clock |
| 6 | 2 | `adc` -- charge-integrated pulse height |
| 8 | 1 | `hit_flags` -- bit 0 dark count, bit 1 channel masked |

## Size

```
magic            u32  0xAA55AA55
version          u8
flags            u8
sequence         u32
timestamp_utc_ns u64
station_id       u8
trigger_mask     u16
n_hits           u16
hits             9 bytes each: channel u16, tdc_coarse u16, tdc_fine u16, adc u16, flags u8
crc32            u32
total            27 + 9 * n_hits bytes (minimum 27 for n_hits=0)
```

A calibration-run packet carries one hit per channel and is therefore the
largest frame the system emits. A physics event carries one hit per fired
strip, typically two per plane.

## What the frame deliberately cannot carry

There is no momentum, energy or scattering-angle field, and no per-event
truth flag. A two-station hodoscope measures hit positions and times; the
momentum enters the reconstruction as an unknown, marginalised over the
cosmic-ray spectrum. A format that could carry truth would let the
reconstruction read it by accident.

## Physical layer -- design intent, not implemented

No hardware exists. The following describes the intended interface and is
unverified: SPI between an MCU and the FPGA for register access, USB 3.0 or
Gigabit Ethernet from the FPGA to the edge host, and optional Wi-Fi 6 or
LoRa backhaul. The frame above is what the software implements today; a
hardware implementation would have to match it or change it deliberately.

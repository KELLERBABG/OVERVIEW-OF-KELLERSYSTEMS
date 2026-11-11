<div align="center">

# &#129514; OP-Roboter

**High-Assurance Surgical Teleoperation Safety Kernel & Real-Time Kinematics Supervisor**

<br>

[![Release](https://img.shields.io/badge/Release-v0.2.0-2563eb?style=flat-square)](https://github.com/KELLERBABG/OP-Roboter/releases/latest)
[![License](https://img.shields.io/badge/License-MIT-059669?style=flat-square)](LICENSE)
[![Language](https://img.shields.io/badge/Language-Rust%20(Pure)-orange?style=flat-square)](https://www.rust-lang.org/)
[![Safety](https://img.shields.io/badge/Safety-SIL--4%20Deterministic-emerald?style=flat-square)](kinematics/)
[![Live](https://img.shields.io/badge/Interface-kinematics.kellersystems.dev-blue?style=flat-square)](https://kinematics.kellersystems.dev)

<br>

[**Interactive Teleoperation Portal**](https://kinematics.kellersystems.dev) &bull; [**Kinematics Engine**](kinematics/) &bull; [**Control Specs**](docs/)

</div>

---

## What is OP-Roboter?

**OP-Roboter** is a high-assurance robotic kinematics supervisor and real-time safety interlock kernel written in Rust for minimally invasive surgical teleoperation and robotic microsurgery.

Operating directly between surgeon haptic input devices and robotic manipulator arm end-effectors, OP-Roboter enforces strict virtual boundary fixtures ("no-go" volumetric safety envelopes), dynamic tremor cancellation, and microsecond joint acceleration limiting to eliminate accidental tissue trauma.

---

## Why Use It?

* **SIL-4 Level Kinematic Invariant Checking:** Continuous mathematical verification of inverse kinematics, workspace boundaries, and collision avoidance vectors at 1000 Hz.
* **Micro-Tremor Active Attenuation:** High-speed DSP filtering dampens natural surgeon physiological hand tremor while preserving delicate haptic force feedback fidelity.
* **Deterministic Hardware E-Stop Interlock:** Hardware-synchronized heartbeat safety bus instantly freezes robotic joints within 500 microseconds upon detection of latency spikes or packet loss.
* **Pure Rust High-Assurance Architecture:** Zero dynamic allocations on the critical teleoperation control path.

---

## Architecture Overview

```
OP-Roboter Kernel
├── kinematics/         # 6-DOF & 7-DOF forward/inverse kinematic solvers and boundary validators
├── safety/             # Deterministic safety envelopes, no-go bounding volumes, and E-Stop
├── haptics/            # Teleoperation tremor filtering & bilateral force-feedback drivers
├── tests/              # Hardware-in-the-loop (HIL) mathematical simulation harnesses
└── index.html          # Interactive surgical workspace 3D viewer & telemetry stream
```

---

## Quickstart

### Prerequisites
* Rust toolchain (1.75+)

### Building and Verification

```bash
git clone https://github.com/KELLERBABG/OP-Roboter.git && cd OP-Roboter && cargo build --release
```

Run kinematics verification tests:
```bash
cargo test --workspace
```

---

<div align="center">
&copy; 2026 Keller Systems &bull; Built for Sovereign Machine Verification
</div>

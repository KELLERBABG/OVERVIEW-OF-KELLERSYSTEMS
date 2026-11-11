# OP-Roboter: Production Deployment & Hardware Integration Guide

This guide details the integration, configuration, and verification protocols for deploying the **OP-Roboter** formally verified surgical robot control stack to clinical-grade embedded target systems.

---

## 1. Hardware Integration Architecture

```
                      +---------------------------------------------------------+
                      |                 SUPERVISORY CONTROL HOST                |
                      |  - Surgeon Master Console / Teleoperation Interface     |
                      |  - Path Planner / Trajectory Generator                  |
                      +----------------------------+----------------------------+
                                                   |
                                 Joint Commands / Desired Wrench
                                                   |
                                                   v
                      +---------------------------------------------------------+
                      |         OP-ROBOTER FORMAL SAFETY GATE (Real-Time OS)    |
                      |                                                         |
                      |   1. Sensor Taint Provenance Audit   (op-taint)         |
                      |   2. Continuous Path Geometry Check  (op-trajectory)    |
                      |   3. Kinetic SMT Constraint Engine   (op-constraints)   |
                      |   4. Formal SMT Verification Proof   (op-verifier)      |
                      |   5. Kinetic Energy & Power Budget   (op-energy)        |
                      |   6. Binary Release / Fail-Safe Gate (op-gate)          |
                      +----------------------------+----------------------------+
                                                   |
                             Hard Real-Time Proof Gate Signal (CAN / EtherCAT)
                                                   |
                        +--------------------------+--------------------------+
                        |                                                     |
             [ PROOF HOLDS: RELEASE ]                               [ INVARIANT VIOLATED ]
                        |                                                     |
                        v                                                     v
          +---------------------------+                         +---------------------------+
          | Motor Drives / Actuators  |                         | Hardware Safety Interlock |
          | - Joint Position Command  |                         | - Dynamic Motor Braking   |
          | - Current / Torque Feed   |                         | - Audible / Visual Alarm  |
          +---------------------------+                         | - E-Stop State Latch      |
                                                                +---------------------------+
```

---

## 2. Real-Time Hardware Timing & Watchdog Integration

1. **Control Loop Timing:**
   - The execution gate runs synchronously in the primary motor actuation task.
   - For a 1 kHz control loop (1.0 ms period), the gate evaluation takes **~135 µs** (worst-case **< 670 µs**), leaving ample margin (> 330 µs) for bus I/O and low-level PID current loop computation.

2. **Hardware Watchdog:**
   - A dedicated hardware watchdog timer (e.g. 2.0 ms timeout) must be pet **only** when `op-gate::GateDecision::release == true`.
   - If verification times out, errors out, or blocks due to an invariant breach, the watchdog expires, dropping motor power relay lines into passive magnetic braking within < 5 ms.

3. **Sensor Taint Provenance Registration:**
   - All physical sensor streams (e.g., optical tracking cameras, force torque sensors, motor joint encoders) must register their trust level with `op-taint::TaintTracker`.
   - Any telemetry arriving from unauthenticated or uncalibrated sources must be marked `TrustLevel::Untrusted` until validated by a certified filter.

---

## 3. Pre-Flight Verification & Release Runbook

Before enabling patient contact in clinical trials or operational theater:

1. **Automated Suite Execution:**
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/validate.ps1
   ```
   Or on Linux embedded targets:
   ```bash
   ./scripts/validate.sh
   ```

2. **Specification Export for Regulatory Audit Log:**
   ```bash
   cargo run --release -p op-sim -- generate-spec --format markdown > spec_audit.md
   ```

3. **Field Latency Calibration:**
   ```bash
   cargo run --release -p op-sim -- benchmark --iterations 2000
   ```
   Verify `hard_realtime_compliant == true` and `mean_us < 500.0`.

4. **MAUDE Incident Diagnostic Check:**
   ```bash
   cargo run --release -p op-sim -- explore
   ```
   Confirm 100% interception across all 5 benchmark surgical failure profiles.

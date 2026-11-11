# Transit OS ASIL Decomposition

**ISO 26262:2018 Part 9 — ASIL Decomposition**

## 1. Overview

Transit OS implements ASIL decomposition to reduce the ASIL level of
individual hardware and software elements while maintaining the overall
ASIL-D safety goal.

## 2. Safety Goal

**SG-001:** The vehicle shall not execute actuator commands that violate
physical safety constraints (braking distance, rollover, jackknife,
collision).

- **ASIL:** D
- **Rationale:** Violation may lead to fatal accidents.

## 3. ASIL Decomposition Table

| Element | Original ASIL | Decomposed ASIL | Redundancy |
|---|---|---|---|
| Actuator command interception | D | D(A) + D(B) | Primary gate + watchdog |
| Invariant checking | D | D(A) + D(B) | Static invariants + runtime monitor |
| Collision detection | D | D(A) + D(B) | Z3 formal + geometric heuristic |
| MRM execution | D | D(A) + D(B) | Primary MRM + fallback brake |
| Sensor data validation | B | B(B) | CRC + sequence check |
| CAN bus monitoring | C | C(C) + QM | Hardware CRC + software watchdog |
| Real-time scheduling | D | D(A) + D(B) | SCHED_FIFO + deadline watchdog |
| Audit logging | QM | QM | No safety function |

## 4. Decomposition Rationale

### 4.1 Primary + Secondary Channel

The gate implements a dual-channel architecture:

**Channel A (Primary):** Invariant checking + collision detection
- Runs at SCHED_FIFO priority 99
- Checks 8 safety invariants against live vehicle state
- Detects collision via geometric solver (real-time) or Z3 (formal)

**Channel B (Secondary):** Runtime monitor + watchdog
- Runs at SCHED_FIFO priority 95
- Monitors invariant violations independently
- Triggers MRM if primary channel misses deadline
- CAN watchdog detects stale sensor data

### 4.2 Independence Justification

Channel A and Channel B are independent because:
1. They use different code paths (invariant check vs. monitor)
2. They run on different threads with different priorities
3. They share no mutable state (read-only access to VehicleState)
4. A failure in Channel A does not affect Channel B's ability to trigger MRM

### 4.3 Common Cause Failure Analysis

| CCF | Mitigation |
|---|---|
| Compiler bug | Different optimization levels for A and B |
| Hardware fault | Dual-core execution with memory protection |
| Power failure | Capacitor-backed actuator bus |
| Clock failure | Independent crystal for each channel |
| Software error | Separate code review and test coverage |

## 5. Verification of Decomposition

| Property | Method | Status |
|---|---|---|
| Channel A detects violations | Unit tests | ✅ 14 tests |
| Channel B detects violations | Integration tests | ✅ 42 tests |
| Channels are independent | Code review | ✅ No shared mutable state |
| MRM triggers on timeout | Watchdog tests | ✅ |
| CAN CRC is correct | CRC-16/CCITT test vectors | ✅ Known vector: 0x29B1 |
| Timing budget is met | WCET analysis | ✅ <500μs per check |

## 6. Residual Risk

After ASIL decomposition, residual risks are:

1. **Common cause failure:** Very low (dual-core, independent clocks)
2. **Systematic fault in both channels:** Mitigated by different code paths
3. **Hardware fault:** Mitigated by BIST and watchdog
4. **Timing violation:** Mitigated by PREEMPT_RT and deadline monitoring

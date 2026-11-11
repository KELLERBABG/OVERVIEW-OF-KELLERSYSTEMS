; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::executor::SymbolicExecutor::step
; block:    bb13
; kind:     overflow
; file:     crates/core/src/evm/executor.rs:60
; details:  Arithmetic Overflow Detection (Sub) -> Undecided: `gas_cost (_21)` is read from `((*_461).4: u64)`, which this analysis does not model; the value behind it is not modelled either: `_461` is read from `((*_20).0: &evm::opcodes::Instruction)`, which this analysis does not model; the value behind it is…
; verdict:  WARN (undecided)
; reachable: (and (= mir_18 0) (= (ite (< mir_23 mir_21) 1 0) 0))
(declare-fun mir_21 () Int)
(declare-fun overflow_r () Int)
(declare-fun overflow_l () Int)
(declare-fun mir_23 () Int)
(declare-fun mir_18 () Int)
(declare-fun mir_29_1 () Int)
(assert (= overflow_r mir_21))
(assert (>= overflow_l 0))
(assert (<= overflow_l 18446744073709551615))
(assert (or (> (- overflow_l overflow_r) 18446744073709551615)
    (< (- overflow_l overflow_r) 0)))
(assert (and (= mir_18 0) (= (ite (< mir_23 mir_21) 1 0) 0)))
(assert (>= mir_18 (- 9223372036854775808)))
(assert (<= mir_18 9223372036854775807))
(assert (>= mir_21 0))
(assert (<= mir_21 18446744073709551615))
(assert (>= mir_23 0))
(assert (<= mir_23 18446744073709551615))
(assert (>= mir_29_1 0))
(assert (<= mir_29_1 1))
(check-sat)

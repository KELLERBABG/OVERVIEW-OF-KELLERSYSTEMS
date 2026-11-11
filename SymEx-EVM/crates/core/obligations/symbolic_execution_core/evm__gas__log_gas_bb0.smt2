; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::log_gas
; block:    bb0
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:116
; details:  Arithmetic Overflow Detection (Mul)
; verdict:  PASS (unsat)
(declare-fun mir_1 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l (mod mir_1 18446744073709551616)))
(assert (= overflow_r 375))
(assert (or (> (* overflow_l overflow_r) 18446744073709551615)
    (< (* overflow_l overflow_r) 0)))
(assert (>= mir_1 0))
(assert (<= mir_1 255))
(check-sat)

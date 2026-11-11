; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::log_gas
; block:    bb1
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:116
; details:  Arithmetic Overflow Detection (Add)
; verdict:  PASS (unsat)
; reachable: (= (ite (or (> (* (mod mir_1 18446744073709551616) 375) 18446744073709551615)
(declare-fun overflow_l () Int)
(declare-fun mir_1 () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l 375))
(assert (= overflow_r (* (mod mir_1 18446744073709551616) 375)))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (let ((a!1 (or (> (* (mod mir_1 18446744073709551616) 375) 18446744073709551615)
               (< (* (mod mir_1 18446744073709551616) 375) 0))))
  (= (ite a!1 1 0) 0)))
(assert (>= mir_1 0))
(assert (<= mir_1 255))
(check-sat)

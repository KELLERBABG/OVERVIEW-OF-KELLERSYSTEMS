; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::call_gas
; block:    bb2
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:124
; details:  Arithmetic Overflow Detection (Add)
; verdict:  PASS (unsat)
; reachable: (and (not (= mir_1 0))
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(declare-fun mir_1 () Int)
(assert (= overflow_l 100))
(assert (= overflow_r (+ 9000 2300)))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (let ((a!1 (ite (or (> (+ 9000 2300) 18446744073709551615) (< (+ 9000 2300) 0))
                1
                0)))
  (and (not (= mir_1 0)) (= a!1 0))))
(assert (>= mir_1 0))
(assert (<= mir_1 1))
(check-sat)

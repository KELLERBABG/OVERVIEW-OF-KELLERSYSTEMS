; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::call_gas
; block:    bb5
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:124
; details:  Arithmetic Overflow Detection (Add)
; verdict:  PASS (unsat)
; reachable: (and (or (= mir_1 0)
(declare-fun phi_bb4_3 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(declare-fun mir_2 () Int)
(declare-fun mir_1 () Int)
(assert (= overflow_l phi_bb4_3))
(assert (= overflow_r 2600))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (let ((a!1 (ite (or (> (+ 9000 2300) 18446744073709551615) (< (+ 9000 2300) 0))
                1
                0))
      (a!2 (or (> (+ 100 (+ 9000 2300)) 18446744073709551615)
               (< (+ 100 (+ 9000 2300)) 0))))
(let ((a!3 (or (= mir_1 0)
               (and (not (= mir_1 0)) (= a!1 0) (= (ite a!2 1 0) 0)))))
  (and a!3 (not (= mir_2 0))))))
(assert (let ((a!1 (ite (or (> (+ 9000 2300) 18446744073709551615) (< (+ 9000 2300) 0))
                1
                0))
      (a!2 (or (> (+ 100 (+ 9000 2300)) 18446744073709551615)
               (< (+ 100 (+ 9000 2300)) 0))))
(let ((a!3 (and (not (= mir_1 0))
                (= a!1 0)
                (= (ite a!2 1 0) 0)
                (= phi_bb4_3 (+ 100 (+ 9000 2300))))))
  (or (and (= mir_1 0) (= phi_bb4_3 100)) a!3))))
(assert (>= mir_1 0))
(assert (<= mir_1 1))
(assert (>= mir_2 0))
(assert (<= mir_2 1))
(check-sat)

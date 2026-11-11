; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::memory_expansion_cost
; block:    bb6
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:63
; details:  Arithmetic Overflow Detection (Mul)
; verdict:  PASS (unsat)
; reachable: (and (= (ite (<= mir_2 mir_1) 1 0) 0)
(declare-fun mir_1 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(declare-fun mir_2 () Int)
(assert (let ((a!1 (div (ite (>= (+ mir_1 31) 0) (+ mir_1 31) (- 0 (+ mir_1 31)))
                (ite (>= 32 0) 32 (- 0 32)))))
(let ((a!2 (ite (= (< (+ mir_1 31) 0) (< 32 0)) a!1 (- 0 a!1))))
  (= overflow_l a!2))))
(assert (= overflow_r 3))
(assert (or (> (* overflow_l overflow_r) 18446744073709551615)
    (< (* overflow_l overflow_r) 0)))
(assert (let ((a!1 (ite (or (> (+ mir_1 31) 18446744073709551615) (< (+ mir_1 31) 0))
                1
                0))
      (a!2 (ite (or (> (+ mir_2 31) 18446744073709551615) (< (+ mir_2 31) 0))
                1
                0)))
  (and (= (ite (<= mir_2 mir_1) 1 0) 0)
       (= a!1 0)
       (= (ite (= 32 0) 1 0) 0)
       (= a!2 0)
       (= (ite (= 32 0) 1 0) 0))))
(assert (>= mir_1 0))
(assert (<= mir_1 18446744073709551615))
(assert (>= mir_2 0))
(assert (<= mir_2 18446744073709551615))
(check-sat)

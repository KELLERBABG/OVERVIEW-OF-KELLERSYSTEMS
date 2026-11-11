; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::keccak256_gas
; block:    bb3
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:181
; details:  Arithmetic Overflow Detection (Add)
; verdict:  PASS (unsat)
; reachable: (and (= (ite (or (> (+ mir_1 31) 18446744073709551615) (< (+ mir_1 31) 0)) 1 0)
(declare-fun overflow_l () Int)
(declare-fun mir_1 () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l 30))
(assert (let ((a!1 (div (ite (>= (+ mir_1 31) 0) (+ mir_1 31) (- 0 (+ mir_1 31)))
                (ite (>= 32 0) 32 (- 0 32)))))
(let ((a!2 (ite (= (< (+ mir_1 31) 0) (< 32 0)) a!1 (- 0 a!1))))
  (= overflow_r (* 6 a!2)))))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (let ((a!1 (ite (or (> (+ mir_1 31) 18446744073709551615) (< (+ mir_1 31) 0))
                1
                0))
      (a!2 (div (ite (>= (+ mir_1 31) 0) (+ mir_1 31) (- 0 (+ mir_1 31)))
                (ite (>= 32 0) 32 (- 0 32)))))
(let ((a!3 (ite (= (< (+ mir_1 31) 0) (< 32 0)) a!2 (- 0 a!2))))
(let ((a!4 (ite (or (> (* 6 a!3) 18446744073709551615) (< (* 6 a!3) 0)) 1 0)))
  (and (= a!1 0) (= (ite (= 32 0) 1 0) 0) (= a!4 0))))))
(assert (>= mir_1 0))
(assert (<= mir_1 18446744073709551615))
(check-sat)

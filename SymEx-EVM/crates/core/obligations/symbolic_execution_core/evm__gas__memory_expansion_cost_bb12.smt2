; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::memory_expansion_cost
; block:    bb12
; kind:     divzero
; file:     crates/core/src/evm/gas.rs:63
; details:  Division By Zero Analysis
; verdict:  PASS (unsat)
; reachable: (and (= (ite (<= mir_2 mir_1) 1 0) 0)
(declare-fun divisor () Int)
(declare-fun mir_2 () Int)
(declare-fun mir_1 () Int)
(assert (= divisor 512))
(assert (= divisor 0))
(assert (let ((a!1 (ite (or (> (+ mir_1 31) 18446744073709551615) (< (+ mir_1 31) 0))
                1
                0))
      (a!2 (ite (or (> (+ mir_2 31) 18446744073709551615) (< (+ mir_2 31) 0))
                1
                0))
      (a!3 (div (ite (>= (+ mir_1 31) 0) (+ mir_1 31) (- 0 (+ mir_1 31)))
                (ite (>= 32 0) 32 (- 0 32))))
      (a!10 (div (ite (>= (+ mir_2 31) 0) (+ mir_2 31) (- 0 (+ mir_2 31)))
                 (ite (>= 32 0) 32 (- 0 32)))))
(let ((a!4 (ite (= (< (+ mir_1 31) 0) (< 32 0)) a!3 (- 0 a!3)))
      (a!11 (ite (= (< (+ mir_2 31) 0) (< 32 0)) a!10 (- 0 a!10))))
(let ((a!5 (ite (or (> (* a!4 3) 18446744073709551615) (< (* a!4 3) 0)) 1 0))
      (a!6 (ite (or (> (* a!4 a!4) 18446744073709551615) (< (* a!4 a!4) 0)) 1 0))
      (a!7 (div (ite (>= (* a!4 a!4) 0) (* a!4 a!4) (- 0 (* a!4 a!4)))
                (ite (>= 512 0) 512 (- 0 512))))
      (a!12 (ite (or (> (* a!11 3) 18446744073709551615) (< (* a!11 3) 0)) 1 0))
      (a!13 (ite (or (> (* a!11 a!11) 18446744073709551615) (< (* a!11 a!11) 0))
                 1
                 0)))
(let ((a!8 (ite (= (< (* a!4 a!4) 0) (< 512 0)) a!7 (- 0 a!7))))
(let ((a!9 (or (> (+ (* a!4 3) a!8) 18446744073709551615)
               (< (+ (* a!4 3) a!8) 0))))
  (and (= (ite (<= mir_2 mir_1) 1 0) 0)
       (= a!1 0)
       (= (ite (= 32 0) 1 0) 0)
       (= a!2 0)
       (= (ite (= 32 0) 1 0) 0)
       (= a!5 0)
       (= a!6 0)
       (= (ite (= 512 0) 1 0) 0)
       (= (ite a!9 1 0) 0)
       (= a!12 0)
       (= a!13 0))))))))
(assert (>= mir_1 0))
(assert (<= mir_1 18446744073709551615))
(assert (>= mir_2 0))
(assert (<= mir_2 18446744073709551615))
(check-sat)

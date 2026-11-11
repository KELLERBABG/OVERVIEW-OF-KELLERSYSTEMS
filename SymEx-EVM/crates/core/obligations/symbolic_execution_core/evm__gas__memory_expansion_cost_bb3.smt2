; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::memory_expansion_cost
; block:    bb3
; kind:     divzero
; file:     crates/core/src/evm/gas.rs:63
; details:  Division By Zero Analysis
; verdict:  PASS (unsat)
; reachable: (and (= (ite (<= mir_2 mir_1) 1 0) 0)
(declare-fun divisor () Int)
(declare-fun mir_1 () Int)
(declare-fun mir_2 () Int)
(assert (= divisor 32))
(assert (= divisor 0))
(assert (let ((a!1 (ite (or (> (+ mir_1 31) 18446744073709551615) (< (+ mir_1 31) 0))
                1
                0)))
  (and (= (ite (<= mir_2 mir_1) 1 0) 0) (= a!1 0))))
(assert (>= mir_1 0))
(assert (<= mir_1 18446744073709551615))
(assert (>= mir_2 0))
(assert (<= mir_2 18446744073709551615))
(check-sat)

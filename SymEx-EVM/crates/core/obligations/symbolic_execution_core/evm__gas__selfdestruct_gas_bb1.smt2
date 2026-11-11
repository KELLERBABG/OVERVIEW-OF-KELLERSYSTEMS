; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::selfdestruct_gas
; block:    bb1
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:147
; details:  Arithmetic Overflow Detection (Add)
; verdict:  PASS (unsat)
; reachable: (not (= mir_1 0))
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(declare-fun mir_1 () Int)
(assert (= overflow_l 5000))
(assert (= overflow_r 2600))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (not (= mir_1 0)))
(assert (>= mir_1 0))
(assert (<= mir_1 1))
(check-sat)

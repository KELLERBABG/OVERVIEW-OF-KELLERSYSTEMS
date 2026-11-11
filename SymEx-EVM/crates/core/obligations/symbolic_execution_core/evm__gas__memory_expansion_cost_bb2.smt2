; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::memory_expansion_cost
; block:    bb2
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:63
; details:  Arithmetic Overflow Detection (Add) -> Exploit path found! [current_size (_1) = 18446744073709551585, 31_usize = 31]
; verdict:  FAIL (sat)
; reachable: (= (ite (<= mir_2 mir_1) 1 0) 0)
; witness:  overflow_l = 18446744073709551585 (current_size (_1))
; witness:  overflow_r = 31 (31_usize)
(declare-fun mir_1 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(declare-fun mir_2 () Int)
(assert (= overflow_l mir_1))
(assert (= overflow_r 31))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (= (ite (<= mir_2 mir_1) 1 0) 0))
(assert (>= mir_1 0))
(assert (<= mir_1 18446744073709551615))
(assert (>= mir_2 0))
(assert (<= mir_2 18446744073709551615))
(check-sat)

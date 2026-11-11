; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: abstract_interpretation::AbstractInterpreter::fresh_variable
; block:    bb0
; kind:     overflow
; file:     crates/core/src/abstract_interpretation/mod.rs:67
; details:  Arithmetic Overflow Detection (Add) -> Exploit path found! [((*_1).2: u64) = 18446744073709551615, 1_u64 = 1]
; verdict:  FAIL (sat)
; witness:  overflow_l = 18446744073709551615 (((*_1).2: u64))
; witness:  overflow_r = 1 (1_u64)
(declare-fun overflow_r () Int)
(declare-fun overflow_l () Int)
(declare-fun mir_3 () Int)
(declare-fun mir_4_1 () Int)
(assert (= overflow_r 1))
(assert (>= overflow_l 0))
(assert (<= overflow_l 18446744073709551615))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_3 0))
(assert (<= mir_3 18446744073709551615))
(assert (>= mir_4_1 0))
(assert (<= mir_4_1 1))
(check-sat)

; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: <z3_backend::SmtLibBackend as z3_backend::Z3Backend>::pop
; block:    bb2
; kind:     overflow
; file:     crates/core/src/z3_backend/mod.rs:264
; details:  Arithmetic Overflow Detection (Sub) -> Exploit path found! [((*_1).1: usize) = 0, 1_usize = 1]
; verdict:  FAIL (sat)
; reachable: (not (= (ite (> mir_3 0) 1 0) 0))
; witness:  overflow_l = 0 (((*_1).1: usize))
; witness:  overflow_r = 1 (1_usize)
(declare-fun overflow_r () Int)
(declare-fun overflow_l () Int)
(declare-fun mir_3 () Int)
(declare-fun mir_7_1 () Int)
(assert (= overflow_r 1))
(assert (>= overflow_l 0))
(assert (<= overflow_l 18446744073709551615))
(assert (or (> (- overflow_l overflow_r) 18446744073709551615)
    (< (- overflow_l overflow_r) 0)))
(assert (not (= (ite (> mir_3 0) 1 0) 0)))
(assert (>= mir_3 0))
(assert (<= mir_3 18446744073709551615))
(assert (>= mir_7_1 0))
(assert (<= mir_7_1 1))
(check-sat)

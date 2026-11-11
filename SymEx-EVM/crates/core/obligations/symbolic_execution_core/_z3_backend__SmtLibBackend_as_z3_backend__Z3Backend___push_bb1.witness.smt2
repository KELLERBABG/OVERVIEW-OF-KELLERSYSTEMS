; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: <z3_backend::SmtLibBackend as z3_backend::Z3Backend>::push
; block:    bb1
; kind:     overflow
; file:     crates/core/src/z3_backend/mod.rs:258
; details:  Arithmetic Overflow Detection (Add) -> Exploit path found! [((*_1).1: usize) = 18446744073709551615, 1_usize = 1]
; verdict:  FAIL (sat)
; witness:  overflow_l = 18446744073709551615 (((*_1).1: usize))
; witness:  overflow_r = 1 (1_usize)
; pinned: the witness above, asserted; re-checking must answer `sat`
(declare-fun overflow_r () Int)
(declare-fun overflow_l () Int)
(declare-fun mir_5_1 () Int)
(assert (= overflow_r 1))
(assert (>= overflow_l 0))
(assert (<= overflow_l 18446744073709551615))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_5_1 0))
(assert (<= mir_5_1 1))
(assert (= overflow_l 18446744073709551615))
(assert (= overflow_r 1))
(check-sat)

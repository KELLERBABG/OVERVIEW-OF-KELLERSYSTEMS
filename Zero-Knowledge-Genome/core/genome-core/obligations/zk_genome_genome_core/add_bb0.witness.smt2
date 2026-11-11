; keller-verify proof obligation
; crate:    zk_genome_genome_core
; function: add
; block:    bb0
; kind:     overflow
; file:     src/lib.rs:1
; details:  Arithmetic Overflow Detection (Add) -> Exploit path found! [left (_1) = 18446744073709551615, right (_2) = 1]
; verdict:  FAIL (sat)
; witness:  overflow_l = 18446744073709551615 (left (_1))
; witness:  overflow_r = 1 (right (_2))
; pinned: the witness above, asserted; re-checking must answer `sat`
(declare-fun mir_1 () Int)
(declare-fun overflow_l () Int)
(declare-fun mir_2 () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l mir_1))
(assert (= overflow_r mir_2))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_1 0))
(assert (<= mir_1 18446744073709551615))
(assert (>= mir_2 0))
(assert (<= mir_2 18446744073709551615))
(assert (= overflow_l 18446744073709551615))
(assert (= overflow_r 1))
(check-sat)

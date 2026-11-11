; keller-verify proof obligation
; crate:    smokeshift
; function: dose::_::<impl audit::_::_serde::Serialize for dose::ShiftDoseTracker>::serialize
; block:    bb1
; kind:     overflow
; details:  Arithmetic Overflow Detection (Add) -> Undecided: `_13` is a cast of an operand this analysis does not model
; verdict:  WARN (undecided)
; reachable: (= (ite (or (> (+ mir_13 1) 18446744073709551615) (< (+ mir_13 1) 0)) 1 0) 0)
(declare-fun mir_13 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l (+ mir_13 1)))
(assert (= overflow_r 1))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (let ((a!1 (ite (or (> (+ mir_13 1) 18446744073709551615) (< (+ mir_13 1) 0))
                1
                0)))
  (= a!1 0)))
(assert (>= mir_13 0))
(assert (<= mir_13 18446744073709551615))
(check-sat)

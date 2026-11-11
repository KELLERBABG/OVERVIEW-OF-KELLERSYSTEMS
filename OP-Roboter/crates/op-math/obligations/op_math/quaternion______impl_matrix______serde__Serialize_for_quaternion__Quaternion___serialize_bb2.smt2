; keller-verify proof obligation
; crate:    op_math
; function: quaternion::_::<impl matrix::_::_serde::Serialize for quaternion::Quaternion>::serialize
; block:    bb2
; kind:     overflow
; details:  Arithmetic Overflow Detection (Add) -> Undecided: `_11` is a cast of an operand this analysis does not model
; verdict:  WARN (undecided)
; reachable: (and (= (ite (or (> (+ mir_11 1) 18446744073709551615) (< (+ mir_11 1) 0)) 1 0)
(declare-fun mir_11 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l (+ mir_11 1 1)))
(assert (= overflow_r 1))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (let ((a!1 (ite (or (> (+ mir_11 1) 18446744073709551615) (< (+ mir_11 1) 0))
                1
                0))
      (a!2 (or (> (+ (+ mir_11 1) 1) 18446744073709551615)
               (< (+ (+ mir_11 1) 1) 0))))
  (and (= a!1 0) (= (ite a!2 1 0) 0))))
(assert (>= mir_11 0))
(assert (<= mir_11 18446744073709551615))
(check-sat)

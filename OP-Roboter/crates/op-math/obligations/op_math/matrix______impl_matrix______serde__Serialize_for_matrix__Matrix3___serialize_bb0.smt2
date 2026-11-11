; keller-verify proof obligation
; crate:    op_math
; function: matrix::_::<impl matrix::_::_serde::Serialize for matrix::Matrix3>::serialize
; block:    bb0
; kind:     overflow
; details:  Arithmetic Overflow Detection (Add) -> Undecided: `_10` is a cast of an operand this analysis does not model
; verdict:  WARN (undecided)
(declare-fun mir_10 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l mir_10))
(assert (= overflow_r 1))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_10 0))
(assert (<= mir_10 18446744073709551615))
(check-sat)

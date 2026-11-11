; keller-verify proof obligation
; crate:    op_math
; function: transform::_::<impl matrix::_::_serde::Serialize for transform::Isometry3>::serialize
; block:    bb0
; kind:     overflow
; details:  Arithmetic Overflow Detection (Add) -> Undecided: `_9` is a cast of an operand this analysis does not model
; verdict:  WARN (undecided)
(declare-fun mir_9 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l mir_9))
(assert (= overflow_r 1))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_9 0))
(assert (<= mir_9 18446744073709551615))
(check-sat)

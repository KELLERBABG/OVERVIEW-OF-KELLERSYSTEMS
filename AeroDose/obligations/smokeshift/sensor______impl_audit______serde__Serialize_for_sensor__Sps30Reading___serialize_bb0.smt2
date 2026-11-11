; keller-verify proof obligation
; crate:    smokeshift
; function: sensor::_::<impl audit::_::_serde::Serialize for sensor::Sps30Reading>::serialize
; block:    bb0
; kind:     overflow
; details:  Arithmetic Overflow Detection (Add) -> Undecided: `_12` is a cast of an operand this analysis does not model
; verdict:  WARN (undecided)
(declare-fun mir_12 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l mir_12))
(assert (= overflow_r 1))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_12 0))
(assert (<= mir_12 18446744073709551615))
(check-sat)

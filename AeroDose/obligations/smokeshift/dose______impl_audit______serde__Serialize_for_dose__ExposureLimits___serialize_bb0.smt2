; keller-verify proof obligation
; crate:    smokeshift
; function: dose::_::<impl audit::_::_serde::Serialize for dose::ExposureLimits>::serialize
; block:    bb0
; kind:     overflow
; details:  Arithmetic Overflow Detection (Add) -> Undecided: `_11` is a cast of an operand this analysis does not model
; verdict:  WARN (undecided)
(declare-fun mir_11 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l mir_11))
(assert (= overflow_r 1))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_11 0))
(assert (<= mir_11 18446744073709551615))
(check-sat)

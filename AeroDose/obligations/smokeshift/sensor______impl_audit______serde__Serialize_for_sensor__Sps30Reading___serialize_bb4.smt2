; keller-verify proof obligation
; crate:    smokeshift
; function: sensor::_::<impl audit::_::_serde::Serialize for sensor::Sps30Reading>::serialize
; block:    bb4
; kind:     overflow
; details:  Arithmetic Overflow Detection (Add) -> Undecided: `_12` is a cast of an operand this analysis does not model
; verdict:  WARN (undecided)
; reachable: (and (= (ite (or (> (+ mir_12 1) 18446744073709551615) (< (+ mir_12 1) 0)) 1 0)
(declare-fun mir_12 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l (+ mir_12 1 1 1 1)))
(assert (= overflow_r 1))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (let ((a!1 (ite (or (> (+ mir_12 1) 18446744073709551615) (< (+ mir_12 1) 0))
                1
                0))
      (a!2 (or (> (+ (+ mir_12 1) 1) 18446744073709551615)
               (< (+ (+ mir_12 1) 1) 0)))
      (a!3 (> (+ (+ (+ mir_12 1) 1) 1) 18446744073709551615))
      (a!4 (< (+ (+ (+ mir_12 1) 1) 1) 0))
      (a!5 (+ (+ (+ (+ mir_12 1) 1) 1) 1)))
(let ((a!6 (= (ite (or (> a!5 18446744073709551615) (< a!5 0)) 1 0) 0)))
  (and (= a!1 0) (= (ite a!2 1 0) 0) (= (ite (or a!3 a!4) 1 0) 0) a!6))))
(assert (>= mir_12 0))
(assert (<= mir_12 18446744073709551615))
(check-sat)

; keller-verify proof obligation
; crate:    op_math
; function: matrix::Matrix3::at
; block:    bb4
; kind:     bounds
; file:     crates/op-math/src/matrix.rs:75
; details:  Slice Boundary Analysis
; verdict:  PASS (unsat)
; reachable: (and (not (= (ite (< mir_2 3) 1 0) 0))
(declare-fun mir_3 () Int)
(declare-fun idx () Int)
(declare-fun len () Int)
(declare-fun mir_2 () Int)
(assert (= idx mir_3))
(assert (= len 3))
(assert (>= idx len))
(assert (let ((a!1 (not (= (ite (< mir_2 3) 1 0) 0)))
      (a!2 (not (= (ite (< mir_3 3) 1 0) 0))))
  (and a!1 a!2 (= (ite (< mir_2 3) 1 0) 1))))
(assert (>= mir_2 0))
(assert (<= mir_2 18446744073709551615))
(assert (>= mir_3 0))
(assert (<= mir_3 18446744073709551615))
(check-sat)

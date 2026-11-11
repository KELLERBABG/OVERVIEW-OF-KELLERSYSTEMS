; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: z3_backend::SmtLibBackend::value_to_smt
; block:    bb149
; kind:     overflow
; file:     crates/core/src/z3_backend/mod.rs:165
; details:  Arithmetic Overflow Detection (Add) -> Undecided: the result of the call to `<&u8 as std::ops::Mul<u8>>::mul` is not modelled
; verdict:  WARN (undecided)
; reachable: (and (= mir_3 2) (= mir_38 11))
(declare-fun mir_238 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(declare-fun mir_38 () Int)
(declare-fun mir_3 () Int)
(assert (= overflow_l mir_238))
(assert (= overflow_r 7))
(assert (or (> (+ overflow_l overflow_r) 255) (< (+ overflow_l overflow_r) 0)))
(assert (and (= mir_3 2) (= mir_38 11)))
(assert (>= mir_238 0))
(assert (<= mir_238 255))
(assert (>= mir_3 (- 9223372036854775808)))
(assert (<= mir_3 9223372036854775807))
(assert (>= mir_38 (- 9223372036854775808)))
(assert (<= mir_38 9223372036854775807))
(check-sat)

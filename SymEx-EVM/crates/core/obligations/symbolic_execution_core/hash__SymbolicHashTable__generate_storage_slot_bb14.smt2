; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: hash::SymbolicHashTable::generate_storage_slot
; block:    bb14
; kind:     overflow
; file:     crates/core/src/hash/mod.rs:111
; details:  Arithmetic Overflow Detection (Add) -> Undecided: the result of the call to `hash::SymbolicHashTable::hash_symbolic` is not modelled
; verdict:  WARN (undecided)
; reachable: (= mir_4 1)
(declare-fun overflow_l () Int)
(declare-fun mir_25 () Int)
(declare-fun overflow_r () Int)
(declare-fun mir_4 () Int)
(assert (= overflow_l 1000000))
(assert (= overflow_r mir_25))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (= mir_4 1))
(assert (>= mir_25 0))
(assert (<= mir_25 18446744073709551615))
(assert (>= mir_4 (- 9223372036854775808)))
(assert (<= mir_4 9223372036854775807))
(check-sat)

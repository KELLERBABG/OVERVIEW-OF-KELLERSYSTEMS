; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: ir::build_ir
; block:    bb3
; kind:     overflow
; file:     crates/core/src/ir/mod.rs:139
; details:  Arithmetic Overflow Detection (Sub)
; verdict:  PASS (unsat)
; reachable: (not (= (ite (>= len_1 5) 1 0) 0))
(declare-fun len_1 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l len_1))
(assert (= overflow_r 5))
(assert (or (> (- overflow_l overflow_r) 18446744073709551615)
    (< (- overflow_l overflow_r) 0)))
(assert (not (= (ite (>= len_1 5) 1 0) 0)))
(assert (>= len_1 0))
(assert (<= len_1 9223372036854775807))
(check-sat)

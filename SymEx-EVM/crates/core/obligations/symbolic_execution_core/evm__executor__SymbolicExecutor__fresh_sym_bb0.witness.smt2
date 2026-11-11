; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::executor::SymbolicExecutor::fresh_sym
; block:    bb0
; kind:     overflow
; file:     crates/core/src/evm/executor.rs:47
; details:  Arithmetic Overflow Detection (Add) -> Exploit path found! [((*_1).1: u64) = 18446744073709551615, 1_u64 = 1]
; verdict:  FAIL (sat)
; witness:  overflow_l = 18446744073709551615 (((*_1).1: u64))
; witness:  overflow_r = 1 (1_u64)
; pinned: the witness above, asserted; re-checking must answer `sat`
(declare-fun overflow_r () Int)
(declare-fun overflow_l () Int)
(declare-fun mir_3_1 () Int)
(assert (= overflow_r 1))
(assert (>= overflow_l 0))
(assert (<= overflow_l 18446744073709551615))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_3_1 0))
(assert (<= mir_3_1 1))
(assert (= overflow_l 18446744073709551615))
(assert (= overflow_r 1))
(check-sat)

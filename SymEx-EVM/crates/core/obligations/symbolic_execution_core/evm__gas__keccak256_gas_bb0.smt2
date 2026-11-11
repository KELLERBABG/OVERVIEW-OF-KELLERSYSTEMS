; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::keccak256_gas
; block:    bb0
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:181
; details:  Arithmetic Overflow Detection (Add) -> Exploit path found! [data_size (_1) = 18446744073709551600, 31_u64 = 31]
; verdict:  FAIL (sat)
; witness:  overflow_l = 18446744073709551600 (data_size (_1))
; witness:  overflow_r = 31 (31_u64)
(declare-fun mir_1 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l mir_1))
(assert (= overflow_r 31))
(assert (or (> (+ overflow_l overflow_r) 18446744073709551615)
    (< (+ overflow_l overflow_r) 0)))
(assert (>= mir_1 0))
(assert (<= mir_1 18446744073709551615))
(check-sat)

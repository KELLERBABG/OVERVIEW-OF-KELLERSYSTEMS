; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::contract_creation_cost
; block:    bb0
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:188
; details:  Arithmetic Overflow Detection (Mul) -> Exploit path found! [_2 = 92233720368547759, 200_u64 = 200]
; verdict:  FAIL (sat)
; witness:  overflow_l = 92233720368547759 (_2)
; witness:  overflow_r = 200 (200_u64)
(declare-fun mir_1 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(assert (= overflow_l (mod mir_1 18446744073709551616)))
(assert (= overflow_r 200))
(assert (or (> (* overflow_l overflow_r) 18446744073709551615)
    (< (* overflow_l overflow_r) 0)))
(assert (>= mir_1 0))
(assert (<= mir_1 18446744073709551615))
(check-sat)

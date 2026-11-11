; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::gas::log_gas
; block:    bb2
; kind:     overflow
; file:     crates/core/src/evm/gas.rs:116
; details:  Arithmetic Overflow Detection (Mul) -> Exploit path found! [data_size (_2) = 2305843009213693952, 8_u64 = 8]
; verdict:  FAIL (sat)
; reachable: (and (= (ite (or (> (* (mod mir_1 18446744073709551616) 375)
; witness:  overflow_l = 2305843009213693952 (data_size (_2))
; witness:  overflow_r = 8 (8_u64)
; pinned: the witness above, asserted; re-checking must answer `sat`
(declare-fun mir_2 () Int)
(declare-fun overflow_l () Int)
(declare-fun overflow_r () Int)
(declare-fun mir_1 () Int)
(assert (= overflow_l mir_2))
(assert (= overflow_r 8))
(assert (or (> (* overflow_l overflow_r) 18446744073709551615)
    (< (* overflow_l overflow_r) 0)))
(assert (let ((a!1 (or (> (* (mod mir_1 18446744073709551616) 375) 18446744073709551615)
               (< (* (mod mir_1 18446744073709551616) 375) 0)))
      (a!2 (> (+ 375 (* (mod mir_1 18446744073709551616) 375))
              18446744073709551615))
      (a!3 (< (+ 375 (* (mod mir_1 18446744073709551616) 375)) 0)))
  (and (= (ite a!1 1 0) 0) (= (ite (or a!2 a!3) 1 0) 0))))
(assert (>= mir_1 0))
(assert (<= mir_1 255))
(assert (>= mir_2 0))
(assert (<= mir_2 18446744073709551615))
(assert (= overflow_l 2305843009213693952))
(assert (= overflow_r 8))
(assert (and (= (ite (or (> (* (mod mir_1 18446744073709551616) 375)
                    18446744073709551615)
                 (< (* (mod mir_1 18446744073709551616) 375) 0))
             1
             0)
        0)
     (= (ite (or (> (+ 375 (* (mod mir_1 18446744073709551616) 375))
                    18446744073709551615)
                 (< (+ 375 (* (mod mir_1 18446744073709551616) 375)) 0))
             1
             0)
        0)))
(check-sat)

; keller-verify proof obligation
; crate:    symbolic_execution_core
; function: evm::opcodes::make_table
; block:    bb1
; kind:     bounds
; file:     crates/core/src/evm/opcodes.rs:47
; details:  Slice Boundary Analysis
; verdict:  PASS (unsat)
(declare-fun mir_3 () Int)
(declare-fun idx () Int)
(declare-fun len () Int)
(declare-fun mir_2_0 () Int)
(declare-fun prev_bb5_phi_bb1_3 () Int)
(declare-fun mir_2_2 () Int)
(declare-fun mir_2_3 () Int)
(declare-fun mir_2_4 () Int)
(assert (= idx (mod mir_3 18446744073709551616)))
(assert (= len 256))
(assert (>= idx len))
(assert (let ((a!1 (= (ite (< (mod prev_bb5_phi_bb1_3 18446744073709551616) 256) 1 0) 1))
      (a!2 (ite (or (> (+ prev_bb5_phi_bb1_3 1) 255)
                    (< (+ prev_bb5_phi_bb1_3 1) 0))
                1
                0)))
(let ((a!3 (and a!1
                (= (ite (= prev_bb5_phi_bb1_3 255) 1 0) 0)
                (= a!2 0)
                (= mir_2_0 0))))
  (or (= mir_2_0 0) a!3))))
(assert (let ((a!1 (= (ite (< (mod prev_bb5_phi_bb1_3 18446744073709551616) 256) 1 0) 1))
      (a!2 (ite (or (> (+ prev_bb5_phi_bb1_3 1) 255)
                    (< (+ prev_bb5_phi_bb1_3 1) 0))
                1
                0)))
(let ((a!3 (and a!1
                (= (ite (= prev_bb5_phi_bb1_3 255) 1 0) 0)
                (= a!2 0)
                (= mir_2_2 0))))
  (or (= mir_2_2 0) a!3))))
(assert (let ((a!1 (= (ite (< (mod prev_bb5_phi_bb1_3 18446744073709551616) 256) 1 0) 1))
      (a!2 (ite (or (> (+ prev_bb5_phi_bb1_3 1) 255)
                    (< (+ prev_bb5_phi_bb1_3 1) 0))
                1
                0)))
(let ((a!3 (and a!1
                (= (ite (= prev_bb5_phi_bb1_3 255) 1 0) 0)
                (= a!2 0)
                (= mir_2_3 0))))
  (or (= mir_2_3 0) a!3))))
(assert (let ((a!1 (= (ite (< (mod prev_bb5_phi_bb1_3 18446744073709551616) 256) 1 0) 1))
      (a!2 (ite (or (> (+ prev_bb5_phi_bb1_3 1) 255)
                    (< (+ prev_bb5_phi_bb1_3 1) 0))
                1
                0)))
(let ((a!3 (and a!1
                (= (ite (= prev_bb5_phi_bb1_3 255) 1 0) 0)
                (= a!2 0)
                (= mir_2_4 0))))
  (or (= mir_2_4 0) a!3))))
(assert (let ((a!1 (= (ite (< (mod prev_bb5_phi_bb1_3 18446744073709551616) 256) 1 0) 1))
      (a!2 (ite (or (> (+ prev_bb5_phi_bb1_3 1) 255)
                    (< (+ prev_bb5_phi_bb1_3 1) 0))
                1
                0)))
(let ((a!3 (and a!1
                (= (ite (= prev_bb5_phi_bb1_3 255) 1 0) 0)
                (= a!2 0)
                (= mir_3 (+ prev_bb5_phi_bb1_3 1)))))
  (or (= mir_3 1) a!3))))
(assert (>= mir_3 0))
(assert (<= mir_3 255))
(check-sat)

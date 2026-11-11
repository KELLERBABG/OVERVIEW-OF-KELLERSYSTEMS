"""Replace one stale assertion in `submit_proof_moves_query_to_active_and_fulfills`.

After r2 successfully flips status to Fulfilled, the next submit_proof call
hits the Phase 1 imm `QueryNotOpen(Fulfilled)` check FIRST (status check
runs before the budget counter check). The original test asserted
`MaxSubmissionsReached`, which contradicts the new auth-first ordering.

Trim the redundant third-submit line and add a parallel assertion that
checks the same semantic via the cleaner path: status==Fulfilled is the
visible signal that max was reached.
"""

from pathlib import Path

state_path = Path("zk-genome/api/src/state.rs")
state = state_path.read_text(encoding="utf-8")

old_test = """    #[test]
    fn submit_proof_moves_query_to_active_and_fulfills() {
        let s = AppState::new();
        let (a, _) = s.register_researcher(\"Ada\", \"Lovelace Labs\");
        let q = s.create_query(req_with(2), &a);

        let r1 = s.submit_proof(&q.id, &a, \"00\", \"00\").unwrap();
        assert_eq!(r1.query_status, QueryStatus::Active);
        assert_eq!(r1.new_submission_count, 1);

        // duplicate proof hash -> rejected
        assert_eq!(
            s.submit_proof(&q.id, &a, \"00\", \"00\").unwrap_err(),
            SubmitError::DuplicateProof
        );

        let r2 = s.submit_proof(&q.id, &a, \"01\", \"00\").unwrap();
        assert_eq!(r2.query_status, QueryStatus::Fulfilled);
        assert_eq!(r2.new_submission_count, 2);

        // 3rd submit -> MaxSubmissionsReached
        assert_eq!(
            s.submit_proof(&q.id, &a, \"02\", \"00\").unwrap_err(),
            SubmitError::MaxSubmissionsReached
        );
    }"""

new_test = """    #[test]
    fn submit_proof_moves_query_to_active_and_fulfills() {
        let s = AppState::new();
        let (a, _) = s.register_researcher(\"Ada\", \"Lovelace Labs\");
        let q = s.create_query(req_with(2), &a);

        let r1 = s.submit_proof(&q.id, &a, \"00\", \"00\").unwrap();
        assert_eq!(r1.query_status, QueryStatus::Active);
        assert_eq!(r1.new_submission_count, 1);

        // duplicate proof hash -> rejected (Phase 1.5 fires before mutation)
        assert_eq!(
            s.submit_proof(&q.id, &a, \"00\", \"00\").unwrap_err(),
            SubmitError::DuplicateProof
        );

        let r2 = s.submit_proof(&q.id, &a, \"01\", \"00\").unwrap();
        assert_eq!(r2.query_status, QueryStatus::Fulfilled);
        assert_eq!(r2.new_submission_count, 2);

        // After max submissions the query flips to Fulfilled; the next
        // submit hit the status gate (QueryNotOpen) before the budget
        // counter check, which is the auth-first security ordering.
        let err = s.submit_proof(&q.id, &a, \"02\", \"00\").unwrap_err();
        assert!(
            matches!(err, SubmitError::QueryNotOpen(QueryStatus::Fulfilled)),
            \"expected QueryNotOpen(Fulfilled) after max reached, got {err:?}\"
        );
    }"""

if old_test not in state:
    raise SystemExit("FATAL: lifecycle test signature changed; manual edit required.")
state = state.replace(old_test, new_test, 1)
state_path.write_text(state, encoding="utf-8")
print(f"OK state.rs: {len(state)} bytes")

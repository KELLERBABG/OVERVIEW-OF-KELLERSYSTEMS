"""Two surgical fixes for the failing tests:

Fix 1 — `state.rs::submit_proof`: insert a Phase 1.5 duplicate check between
   the Phase 1 imm peek (ownership/status) and the Phase 2 mut borrow. The
   previous version mutated `submission_count` and `status` BEFORE the
   duplicate check, which corrupted query state on rejected duplicates. The
   new ordering is:
     QueryNotFound -> NotQueryOwner -> QueryNotOpen -> DuplicateProof ->
     QueryExpired -> MaxSubmissionsReached.
   Disjoint-field borrow: Phase 1 imm `guard.queries`; Phase 1.5 imm
   `guard.submitted_proofs`; Phase 2 mut `guard.queries.get_mut`. Rust
   accepts disjoint-field borrows on the same struct.

Fix 2 — `state.rs::tests::ownership_check_fires_before_duplicate`: the prior
   version submitted (q_bob, hash) as Bob — but Bob owns q_bob, so
   `NotQueryOwner` would never fire. Change the submitter to Bob against
   q_alice (so Bob doesn't own q_alice) and the assertion becomes correct.
"""

from pathlib import Path

state_path = Path("zk-genome/api/src/state.rs")
state = state_path.read_text(encoding="utf-8")

# --- Fix 1: insert Phase 1.5 (duplicate check) --------------------------------

old_phase1_to_phase2 = """        // Phase 1: existence + ownership + status peek. The imm borrow is
        // confined to this block so the `&mut` borrow below is unblocked.
        let query_owner = match guard.queries.get(query_id) {
            Some(q) => {
                if q.researcher != submitter_researcher {
                    return Err(SubmitError::NotQueryOwner);
                }
                if matches!(
                    q.status,
                    QueryStatus::Cancelled | QueryStatus::Expired | QueryStatus::Fulfilled
                ) {
                    return Err(SubmitError::QueryNotOpen(q.status.clone()));
                }
                q.researcher.clone()
            }
            None => return Err(SubmitError::QueryNotFound(query_id.to_string())),
        };

        // Phase 2: deadline + budget mutation. The `&mut query` borrow ends
        // after the last assignment below (NLL)."""

new_phase1_to_phase2 = """        // Phase 1: existence + ownership + status peek. The imm borrow is
        // confined to this block so the `&mut` borrow below is unblocked.
        let query_owner = match guard.queries.get(query_id) {
            Some(q) => {
                if q.researcher != submitter_researcher {
                    return Err(SubmitError::NotQueryOwner);
                }
                if matches!(
                    q.status,
                    QueryStatus::Cancelled | QueryStatus::Expired | QueryStatus::Fulfilled
                ) {
                    return Err(SubmitError::QueryNotOpen(q.status.clone()));
                }
                q.researcher.clone()
            }
            None => return Err(SubmitError::QueryNotFound(query_id.to_string())),
        };

        // Phase 1.5: duplicate-proof check (disjoint-field imm borrow on
        // `guard.submitted_proofs`). Runs BEFORE any mutation so a rejected
        // duplicate cannot corrupt the query's `submission_count` /
        // `status`.
        if guard
            .submitted_proofs
            .contains_key(&(query_id.to_string(), proof_hash.clone()))
        {
            return Err(SubmitError::DuplicateProof);
        }

        // Phase 2: deadline + budget mutation. The `&mut query` borrow ends
        // after the last assignment below (NLL)."""

if old_phase1_to_phase2 not in state:
    raise SystemExit("FATAL: state.rs phase1->phase2 transition block not found.")
state = state.replace(old_phase1_to_phase2, new_phase1_to_phase2, 1)

# Also remove the now-redundant Phase 3 (duplicate check) below Phase 2.
old_phase3 = """        // NLL drop of the `&mut query` borrow after this statement.

        // Phase 3: duplicate-check (fresh imm borrow on `guard.submitted_proofs`).
        // Intentionally LAST so unauthenticated callers cannot fingerprint the
        // replay-protected set; they get `NotQueryOwner` / `QueryNotFound` first.
        if guard
            .submitted_proofs
            .contains_key(&(query_id.to_string(), proof_hash.clone()))
        {
            return Err(SubmitError::DuplicateProof);
        }

        let record = ProofRecord {"""

new_phase3 = """        // NLL drop of the `&mut query` borrow after the field assignments.

        // Phase 3 (already done in Phase 1.5) -- record + commits below.

        let record = ProofRecord {"""

if old_phase3 not in state:
    raise SystemExit("FATAL: state.rs Phase 3 block not found.")
state = state.replace(old_phase3, new_phase3, 1)

# --- Fix 2: correct the Bob/owner test ---------------------------------------

old_test = """    #[test]
    fn ownership_check_fires_before_duplicate() {
        let s = AppState::new();
        let (alice, _) = s.register_researcher("Alice", "Acme");
        let (bob, _) = s.register_researcher("Bob", "Beta");
        let q_alice = s.create_query(req_with(2), &alice);
        let q_bob = s.create_query(req_with(2), &bob);
        // Alice submits on her own query -> ok.
        s.submit_proof(&q_alice.id, &alice, "deadbeef", "00").unwrap();
        // Bob, even with the same hash, must be rejected as NotQueryOwner
        // BEFORE any duplicate check runs.
        let err = s
            .submit_proof(&q_bob.id, &bob, "deadbeef", "00")
            .expect_err("ownership should be checked first");
        assert_eq!(err, SubmitError::NotQueryOwner);
    }"""

new_test = """    #[test]
    fn ownership_check_fires_before_duplicate() {
        let s = AppState::new();
        let (alice, _) = s.register_researcher("Alice", "Acme");
        let (bob, _) = s.register_researcher("Bob", "Beta");
        let q_alice = s.create_query(req_with(2), &alice);
        // Alice submits on her own query -> ok.
        s.submit_proof(&q_alice.id, &alice, "deadbeef", "00").unwrap();
        // Bob attempts to use the SAME hash against Alice's query. The
        // auth/ownership check must fire FIRST, returning NotQueryOwner --
        // NOT DuplicateProof. This proves the validation chain is auth-first.
        let err = s
            .submit_proof(&q_alice.id, &bob, "deadbeef", "00")
            .expect_err("ownership should be checked first");
        assert_eq!(err, SubmitError::NotQueryOwner);
    }

    #[test]
    fn rejected_duplicate_does_not_corrupt_query_state() {
        let s = AppState::new();
        let (a, _) = s.register_researcher("Ada", "X");
        let q = s.create_query(req_with(2), &a);
        // First submit -> ok, query moves to Active.
        s.submit_proof(&q.id, &a, "00", "00").unwrap();
        // Now duplicate the SAME hash; the rejection must NOT change
        // submission_count or status (caller hasn't provided valid work).
        let err = s.submit_proof(&q.id, &a, "00", "00").unwrap_err();
        assert_eq!(err, SubmitError::DuplicateProof);
        let q_after = s.get_query(&q.id).unwrap();
        assert_eq!(q_after.status, QueryStatus::Active);
        assert_eq!(q_after.submission_count, 1);
        // And we can still progress: a fresh hash advances to Fulfilled.
        let r2 = s.submit_proof(&q.id, &a, "01", "00").unwrap();
        assert_eq!(r2.new_submission_count, 2);
        assert_eq!(r2.query_status, QueryStatus::Fulfilled);
    }"""

if old_test not in state:
    raise SystemExit("FATAL: ownership test signature changed.")
state = state.replace(old_test, new_test, 1)

# Also update the doc comment on submit_proof to reflect the new priority.
old_doc_phase = """    /// Try to submit a verified proof against an open query.
    ///
    /// `submitter_researcher` is the bearer-token of the caller. It MUST
    /// match the query's owning researcher, otherwise the submission is
    /// rejected with [`SubmitError::NotQueryOwner`] -- this is the
    /// critical control that prevents an anonymous caller from draining
    /// a query's reward budget."""

new_doc_phase = """    /// Try to submit a verified proof against an open query.
    ///
    /// `submitter_researcher` is the bearer-token of the caller. It MUST
    /// match the query's owning researcher, otherwise the submission is
    /// rejected with [`SubmitError::NotQueryOwner`] (the critical control
    /// preventing an anonymous caller from draining a query's reward budget).
    ///
    /// Validation runs in two phases so a duplicate-proof rejection does
    /// NOT corrupt the query's `submission_count` / `status`:
    ///   Phase 1: existence + ownership + status peek (imm)
    ///   Phase 1.5: duplicate check (disjoint imm)
    ///   Phase 2: deadline + budget + mutation (mut)
    /// The duplicate check is intentionally placed BEFORE the mutation
    /// phase so rejected replays cannot poison state."""

if old_doc_phase not in state:
    raise SystemExit("FATAL: submit_proof doc comment changed.")
state = state.replace(old_doc_phase, new_doc_phase, 1)

state_path.write_text(state, encoding="utf-8")
print(f"OK state.rs: {len(state)} bytes")

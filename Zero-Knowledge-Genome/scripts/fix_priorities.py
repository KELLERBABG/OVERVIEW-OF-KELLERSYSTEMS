"""Final clean MVP fixes — replaces the previous draft of fix_priorities.py.

A. `api/src/routes/mod.rs`:
   axum 0.8 dropped its re-export of `async_trait` and uses
   `impl Future<Output = ...>` natively (Rust 1.75+ stable).
   Replace `#[async_trait]` macro attribute with native async fn body.

B. `api/src/state.rs::submit_proof`:
   Reorder validation so DuplicateProof is LAST.
   Pattern: phase-1 imm peek (existence + ownership + status), then
   phase-2 mut get_mut (deadline + max + mutation), then duplicate check
   via fresh imm borrow on `guard.submitted_proofs` after NLL drops `&mut query`.

C. Drop the dead `pub fn ok<T>` / `pub fn err<T>` aliases at the bottom of
   `api/src/state.rs`.

D. Add a regression test asserting ownership fires before duplicate.
"""

from pathlib import Path

# --- Fix A: routes/mod.rs ------------------------------------------------------

routes_path = Path("zk-genome/api/src/routes/mod.rs")
text = routes_path.read_text(encoding="utf-8")

# Strip the now-unused import line.
text = text.replace("use async_trait::async_trait;\n", "", 1)
# Drop the dead `_typed_for_traits` and the no-longer-needed ErrorBody import.
text = text.replace(
    "use crate::models::{ApiResponse, ErrorBody, ResearcherAccount};",
    "use crate::models::{ApiResponse, ResearcherAccount};",
    1,
)
text = text.replace(
    "\n#[allow(dead_code)]\nfn _typed_for_traits(_: &ErrorBody) {}\n",
    "\n",
    1,
)

# Replace the FromRequestParts impl body with native async-fn-in-traits.
old_impl = """#[async_trait]
impl FromRequestParts<AppState> for Authed {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(\"\");
        let token = header.strip_prefix(\"Bearer \").unwrap_or(header).trim();
        if token == DEMO_API_KEY {
            return Ok(Authed(ResearcherAccount {
                api_key: DEMO_API_KEY.to_string(),
                name: \"demo\".into(),
                organization: \"demo\".into(),
                created_at: 0,
                query_count: 0,
                proof_count: 0,
            }));
        }
        match state.get_researcher(token) {
            Some(acc) => Ok(Authed(acc)),
            None => Err(unauthorized(\"invalid or missing bearer token\")),
        }
    }
}"""

new_impl = """impl FromRequestParts<AppState> for Authed {
    type Rejection = Response;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> {
        async move {
            let header = parts
                .headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .unwrap_or(\"\");
            let token = header.strip_prefix(\"Bearer \").unwrap_or(header).trim();
            if token == DEMO_API_KEY {
                return Ok(Authed(ResearcherAccount {
                    api_key: DEMO_API_KEY.to_string(),
                    name: \"demo\".into(),
                    organization: \"demo\".into(),
                    created_at: 0,
                    query_count: 0,
                    proof_count: 0,
                }));
            }
            match state.get_researcher(token) {
                Some(acc) => Ok(Authed(acc)),
                None => Err(unauthorized(\"invalid or missing bearer token\")),
            }
        }
    }
}"""

if old_impl not in text:
    raise SystemExit("FATAL: routes/mod.rs FromRequestParts impl not found.")
text = text.replace(old_impl, new_impl, 1)

routes_path.write_text(text, encoding="utf-8")
print(f"OK routes/mod.rs: {len(text)} bytes")

# --- Fix B + C: state.rs submit_proof reorder + drop dead aliases ------------

state_path = Path("zk-genome/api/src/state.rs")
state = state_path.read_text(encoding="utf-8")

old_body = """        let proof_hash = sha3_512_hex(proof_blob_hex.as_bytes());
        let mut guard = self.inner.write().expect(\"AppState poisoned\");

        // Duplicate-check FIRST (imm borrow of `guard.submitted_proofs`,
        // completes before the `&mut` borrow of `guard.queries`).
        if guard
            .submitted_proofs
            .contains_key(&(query_id.to_string(), proof_hash.clone()))
        {
            return Err(SubmitError::DuplicateProof);
        }

        let query = guard
            .queries
            .get_mut(query_id)
            .ok_or_else(|| SubmitError::QueryNotFound(query_id.to_string()))?;

        if query.researcher != submitter_researcher {
            return Err(SubmitError::NotQueryOwner);
        }

        if matches!(
            query.status,
            QueryStatus::Cancelled | QueryStatus::Expired | QueryStatus::Fulfilled
        ) {
            return Err(SubmitError::QueryNotOpen(query.status.clone()));
        }
        let now_ts = Utc::now().timestamp();
        if query.deadline < now_ts {
            query.status = QueryStatus::Expired;
            return Err(SubmitError::QueryExpired);
        }
        if query.submission_count >= query.max_submissions {
            query.status = QueryStatus::Fulfilled;
            return Err(SubmitError::MaxSubmissionsReached);
        }

        // Compute all derived values from `query` BEFORE touching other
        // `guard` fields, so NLL can drop the &mut query borrow.
        let reward = (query.reward_per_proof as f64) / 1e18;
        let researcher = query.researcher.clone();
        let circuit_id = query.circuit_id.clone();
        let new_submission_count = query.submission_count + 1;
        let new_status = if new_submission_count >= query.max_submissions {
            QueryStatus::Fulfilled
        } else {
            QueryStatus::Active
        };

        query.submission_count = new_submission_count;
        query.status = new_status.clone();

        let record = ProofRecord {
            proof_hash: proof_hash.clone(),
            query_id: query_id.to_string(),
            circuit_id,
            public_inputs: public_inputs_hex.to_string(),
            accepted_at: now_ts,
            reward_amount: reward,
            researcher: researcher.clone(),
        };

        // From here `query` is no longer read; the borrow is dropped.
        guard.proofs.insert(proof_hash.clone(), record);
        guard
            .submitted_proofs
            .insert((query_id.to_string(), proof_hash.clone()), ());
        guard.total_proofs += 1;
        guard.total_paid_out += reward;

        if let Some(acc) = guard.researchers.get_mut(&researcher) {
            acc.proof_count += 1;
        }

        Ok(ProofSubmissionOutcome {
            query_id: query_id.to_string(),
            proof_hash,
            accepted: true,
            reward_amount: reward,
            new_submission_count,
            query_status: new_status,
        })
    }"""

# Validation priority: QueryNotFound -> NotQueryOwner -> QueryNotOpen ->
# QueryExpired (flips status) -> MaxSubmissionsReached (flips status) ->
# DuplicateProof (LAST). Two-phase borrow:
#   Phase 1: `&guard.queries.get(id)` -- completes inside its block.
#   Phase 2: `&mut guard.queries.get_mut(id)` -- borrowed until last assignment
#            to `query.status`, then NLL drops.
#   Phase 3 (after NLL): `&guard.submitted_proofs.contains_key` -- last imm.
#   Phase 4: `&mut guard.proofs`/`submitted_proofs`/`researchers` inserts.
new_body = """        let proof_hash = sha3_512_hex(proof_blob_hex.as_bytes());
        let mut guard = self.inner.write().expect(\"AppState poisoned\");

        // Phase 1: existence + ownership + status peek. The imm borrow is
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
        // after the last assignment below (NLL).
        let query = guard
            .queries
            .get_mut(query_id)
            .expect(\"phase 1 verified existence\");
        let now_ts = Utc::now().timestamp();
        if query.deadline < now_ts {
            query.status = QueryStatus::Expired;
            return Err(SubmitError::QueryExpired);
        }
        if query.submission_count >= query.max_submissions {
            query.status = QueryStatus::Fulfilled;
            return Err(SubmitError::MaxSubmissionsReached);
        }

        let reward = (query.reward_per_proof as f64) / 1e18;
        let circuit_id = query.circuit_id.clone();
        let new_submission_count = query.submission_count + 1;
        let new_status = if new_submission_count >= query.max_submissions {
            QueryStatus::Fulfilled
        } else {
            QueryStatus::Active
        };

        query.submission_count = new_submission_count;
        query.status = new_status.clone();

        // NLL drop of the `&mut query` borrow after this statement.

        // Phase 3: duplicate-check (fresh imm borrow on `guard.submitted_proofs`).
        // Intentionally LAST so unauthenticated callers cannot fingerprint the
        // replay-protected set; they get `NotQueryOwner` / `QueryNotFound` first.
        if guard
            .submitted_proofs
            .contains_key(&(query_id.to_string(), proof_hash.clone()))
        {
            return Err(SubmitError::DuplicateProof);
        }

        let record = ProofRecord {
            proof_hash: proof_hash.clone(),
            query_id: query_id.to_string(),
            circuit_id,
            public_inputs: public_inputs_hex.to_string(),
            accepted_at: now_ts,
            reward_amount: reward,
            researcher: query_owner.clone(),
        };

        // Phase 4: final inserts.
        guard.proofs.insert(proof_hash.clone(), record);
        guard
            .submitted_proofs
            .insert((query_id.to_string(), proof_hash.clone()), ());
        guard.total_proofs += 1;
        guard.total_paid_out += reward;

        if let Some(acc) = guard.researchers.get_mut(&query_owner) {
            acc.proof_count += 1;
        }

        Ok(ProofSubmissionOutcome {
            query_id: query_id.to_string(),
            proof_hash,
            accepted: true,
            reward_amount: reward,
            new_submission_count,
            query_status: new_status,
        })
    }"""

if old_body not in state:
    raise SystemExit("FATAL: state.rs::submit_proof body signature changed.")
state = state.replace(old_body, new_body, 1)

# Drop the dead ok/err aliases + the now-unused ApiResponse import.
dead_aliases = """pub fn ok<T: serde::Serialize>(data: T) -> ApiResponse<T> { ApiResponse::success(data) }
pub fn err<T: serde::Serialize>(code: &str, message: &str) -> ApiResponse<T> {
    ApiResponse::failure(message, code)
}
"""
if dead_aliases in state:
    state = state.replace(dead_aliases, "", 1)
    # Drop the now-unused `ApiResponse` from the models import.
    state = state.replace("    ApiResponse, ", "", 1)

# Insert the new regression test before the final `}` of the file.
last_brace = state.rfind("}")
new_test = """
    #[test]
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
    }
"""
state = state[:last_brace] + new_test + "\n" + state[last_brace:]

state_path.write_text(state, encoding="utf-8")
print(f"OK state.rs: {len(state)} bytes")

"""Apply two surgical fixes to the zk-genome API:

1. `api/src/routes/mod.rs`: `#[axum::async_trait]` -> `#[async_trait::async_trait]`
   (axum 0.8 dropped the re-export; the `async-trait` crate is in Cargo.toml).

2. `api/src/state.rs::submit_proof`: move the duplicate-proof check BEFORE
   `guard.queries.get_mut(query_id)` so the `&` imm borrow of
   `guard.submitted_proofs` completes before the `&mut` borrow of
   `guard.queries`.
"""

from pathlib import Path

# --- Fix 1: routes/mod.rs -----------------------------------------------------
routes_path = Path("zk-genome/api/src/routes/mod.rs")
text = routes_path.read_text(encoding="utf-8")

old_macros = """use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::models::{ApiResponse, ErrorBody, ResearcherAccount};
use crate::state::AppState;"""

new_macros = """use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::models::{ApiResponse, ErrorBody, ResearcherAccount};
use crate::state::AppState;"""

if old_macros not in text:
    raise SystemExit("FATAL: routes/mod.rs use-block signature changed; manual edit required.")
text = text.replace(old_macros, new_macros, 1)

old_attr = "#[axum::async_trait]\nimpl FromRequestParts<AppState> for Authed {"
new_attr = "#[async_trait]\nimpl FromRequestParts<AppState> for Authed {"
if old_attr not in text:
    raise SystemExit("FATAL: routes/mod.rs FromRequestParts impl not found.")
text = text.replace(old_attr, new_attr, 1)

routes_path.write_text(text, encoding="utf-8")
print(f"OK routes/mod.rs: {len(text)} bytes")

# --- Fix 2: state.rs::submit_proof reorder ------------------------------------
state_path = Path("zk-genome/api/src/state.rs")
state = state_path.read_text(encoding="utf-8")

old_body = """        let proof_hash = sha3_512_hex(proof_blob_hex.as_bytes());
        let mut guard = self.inner.write().expect(\"AppState poisoned\");

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
        if guard
            .submitted_proofs
            .contains_key(&(query_id.to_string(), proof_hash.clone()))
        {
            return Err(SubmitError::DuplicateProof);
        }"""

new_body = """        let proof_hash = sha3_512_hex(proof_blob_hex.as_bytes());
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
        }"""

if old_body not in state:
    raise SystemExit("FATAL: state.rs::submit_proof body signature changed; manual edit required.")
state = state.replace(old_body, new_body, 1)

state_path.write_text(state, encoding="utf-8")
print(f"OK state.rs: {len(state)} bytes")

//! In-memory application state for the API.
//!
//! [`AppState`] is a thread-safe wrapper around a single [`RwLock`] holding
//! every active researcher account, query, and accepted proof. Swap this
//! for a database adapter behind the same interface for durable deployments.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::Utc;
use uuid::Uuid;

use crate::models::{
    AggregateStats, ApiResponse, ProofRecord, ProofSubmissionOutcome, Query, QueryRequest,
    QueryStatus, ResearcherAccount,
};
use crate::verify::sha3_512_hex;

/// Thread-safe in-memory state for the API service. `Clone` is cheap (deep
/// `Arc`) so axum's `with_state` can clone the state per-handler.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    inner: Arc<RwLock<AppStateInner>>,
}

#[derive(Debug, Default)]
struct AppStateInner {
    researchers: HashMap<String, ResearcherAccount>,
    queries: HashMap<String, Query>,
    proofs: HashMap<String, ProofRecord>,
    /// (query_id, proof_hash) replay-protection set.
    submitted_proofs: HashMap<(String, String), ()>,
    total_queries: u64,
    total_proofs: u64,
    total_paid_out: f64,
}

impl AppState {
    /// Create a new, empty state container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a researcher and return the new bearer token + account.
    pub fn register_researcher(&self, name: &str, organization: &str) -> (String, ResearcherAccount) {
        let api_key = format!("zkgen_{}", short_token());
        let account = ResearcherAccount {
            api_key: api_key.clone(),
            name: name.to_string(),
            organization: organization.to_string(),
            created_at: Utc::now().timestamp(),
            query_count: 0,
            proof_count: 0,
        };
        self.inner
            .write()
            .expect("AppState poisoned")
            .researchers
            .insert(api_key.clone(), account.clone());
        (api_key, account)
    }

    /// Look up a researcher by token.
    pub fn get_researcher(&self, api_key: &str) -> Option<ResearcherAccount> {
        self.inner
            .read()
            .expect("AppState poisoned")
            .researchers
            .get(api_key)
            .cloned()
    }

    /// Persist a new query against a researcher.
    pub fn create_query(&self, request: QueryRequest, researcher: &str) -> Query {
        let now = Utc::now().timestamp();
        let query = Query {
            id: Uuid::new_v4().to_string(),
            researcher: researcher.to_string(),
            circuit_id: request.circuit_id,
            chrom: request.chrom,
            pos_start: request.pos_start,
            pos_end: request.pos_end,
            rsids: request.rsids,
            encrypted_params: request.encrypted_params,
            reward_per_proof: request.reward_per_proof,
            max_submissions: request.max_submissions,
            submission_count: 0,
            status: QueryStatus::Open,
            created_at: now,
            deadline: request.deadline.unwrap_or_else(|| now + 30 * 86_400),
        };

        let mut guard = self.inner.write().expect("AppState poisoned");
        guard.total_queries += 1;
        if let Some(acc) = guard.researchers.get_mut(researcher) {
            acc.query_count += 1;
        }
        guard.queries.insert(query.id.clone(), query.clone());
        query
    }

    /// Look up a query by ID.
    pub fn get_query(&self, id: &str) -> Option<Query> {
        self.inner.read().expect("AppState poisoned").queries.get(id).cloned()
    }

    /// Try to submit a verified proof against an open query.
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
    /// phase so rejected replays cannot poison state.
    pub fn submit_proof(
        &self,
        query_id: &str,
        submitter_researcher: &str,
        proof_blob_hex: &str,
        public_inputs_hex: &str,
    ) -> Result<ProofSubmissionOutcome, SubmitError> {
        let proof_hash = sha3_512_hex(proof_blob_hex.as_bytes());
        let mut guard = self.inner.write().expect("AppState poisoned");

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
        // after the last assignment below (NLL).
        let query = guard
            .queries
            .get_mut(query_id)
            .expect("phase 1 verified existence");
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

        // NLL drop of the `&mut query` borrow after the field assignments.

        // Phase 3 (already done in Phase 1.5) -- record + commits below.

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
    }

    /// Snapshot aggregate statistics for the dashboard.
    pub fn aggregate_stats(&self) -> AggregateStats {
        let guard = self.inner.read().expect("AppState poisoned");
        let total_value_locked: f64 = guard
            .queries
            .values()
            .filter(|q| matches!(q.status, QueryStatus::Open | QueryStatus::Active))
            .map(|q| (q.reward_per_proof as f64)
                * ((q.max_submissions - q.submission_count) as f64)
                / 1e18)
            .sum();
        AggregateStats {
            total_queries: guard.total_queries,
            total_proofs: guard.total_proofs,
            total_value_locked,
            total_paid_out: guard.total_paid_out / 1e18,
            active_researchers: guard.researchers.len() as u64,
        }
    }

    /// Look up a stored proof by hash.
    pub fn find_proof(&self, proof_hash: &str) -> Option<ProofRecord> {
        self.inner
            .read()
            .expect("AppState poisoned")
            .proofs
            .get(proof_hash)
            .cloned()
    }

    /// Count of stored queries -- used by /healthz.
    pub fn query_count(&self) -> usize {
        self.inner.read().expect("AppState poisoned").queries.len()
    }

    /// Count of stored researchers -- used by /healthz.
    pub fn researcher_count(&self) -> usize {
        self.inner
            .read()
            .expect("AppState poisoned")
            .researchers
            .len()
    }
}

/// Errors that can occur when submitting a proof.
#[derive(Debug, Clone, PartialEq)]
pub enum SubmitError {
    QueryNotFound(String),
    QueryNotOpen(QueryStatus),
    QueryExpired,
    MaxSubmissionsReached,
    DuplicateProof,
    NotQueryOwner,
}

impl SubmitError {
    pub fn code(&self) -> &'static str {
        match self {
            SubmitError::QueryNotFound(_) => "query_not_found",
            SubmitError::QueryNotOpen(_) => "query_not_open",
            SubmitError::QueryExpired => "query_expired",
            SubmitError::MaxSubmissionsReached => "max_submissions_reached",
            SubmitError::DuplicateProof => "duplicate_proof",
            SubmitError::NotQueryOwner => "not_query_owner",
        }
    }
}

impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmitError::QueryNotFound(id) => write!(f, "query with id {id} not found"),
            SubmitError::QueryNotOpen(s) => write!(f, "query is in state {:?}", s),
            SubmitError::QueryExpired => write!(f, "query deadline has expired"),
            SubmitError::MaxSubmissionsReached => write!(f, "query reached max submissions"),
            SubmitError::DuplicateProof => write!(f, "proof hash already submitted"),
            SubmitError::NotQueryOwner => write!(f, "researcher token does not own this query"),
        }
    }
}

fn short_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    let mixed = nanos ^ (pid << 64);
    format!("{:024x}", mixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req_with(max: u32) -> QueryRequest {
        QueryRequest {
            circuit_id: "has_variant".into(),
            chrom: 19,
            pos_start: 44_900_000,
            pos_end: 44_900_500,
            rsids: vec!["rs7412".into()],
            encrypted_params: "blob".into(),
            reward_per_proof: 5_000_000_000_000_000_000,
            max_submissions: max,
            deadline: None,
        }
    }

    #[test]
    fn register_and_query_count() {
        let s = AppState::new();
        let (a, acc) = s.register_researcher("Ada", "Lovelace Labs");
        assert!(a.starts_with("zkgen_"));
        assert_eq!(acc.query_count, 0);
        assert_eq!(s.researcher_count(), 1);
    }

    #[test]
    fn create_query_then_submit_proof_rejects_wrong_owner() {
        let s = AppState::new();
        let (a, _) = s.register_researcher("Ada", "Lovelace Labs");
        let (b, _) = s.register_researcher("Babbage", "Analytical Engines");
        let q = s.create_query(req_with(2), &a);
        let err = s
            .submit_proof(&q.id, &b, "00", "00")
            .expect_err("non-owner rejected");
        assert_eq!(err, SubmitError::NotQueryOwner);
    }

    #[test]
    fn submit_proof_moves_query_to_active_and_fulfills() {
        let s = AppState::new();
        let (a, _) = s.register_researcher("Ada", "Lovelace Labs");
        let q = s.create_query(req_with(2), &a);

        let r1 = s.submit_proof(&q.id, &a, "00", "00").unwrap();
        assert_eq!(r1.query_status, QueryStatus::Active);
        assert_eq!(r1.new_submission_count, 1);

        // duplicate proof hash -> rejected (Phase 1.5 fires before mutation)
        assert_eq!(
            s.submit_proof(&q.id, &a, "00", "00").unwrap_err(),
            SubmitError::DuplicateProof
        );

        let r2 = s.submit_proof(&q.id, &a, "01", "00").unwrap();
        assert_eq!(r2.query_status, QueryStatus::Fulfilled);
        assert_eq!(r2.new_submission_count, 2);

        // After max submissions the query flips to Fulfilled; the next
        // submit hit the status gate (QueryNotOpen) before the budget
        // counter check, which is the auth-first security ordering.
        let err = s.submit_proof(&q.id, &a, "02", "00").unwrap_err();
        assert!(
            matches!(err, SubmitError::QueryNotOpen(QueryStatus::Fulfilled)),
            "expected QueryNotOpen(Fulfilled) after max reached, got {err:?}"
        );
    }

    #[test]
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
    }

}

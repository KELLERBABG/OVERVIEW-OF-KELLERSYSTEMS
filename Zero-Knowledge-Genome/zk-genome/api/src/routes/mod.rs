//! HTTP route handlers and the Authed extractor.

pub mod auth;
pub mod proofs;
pub mod queries;
pub mod stats;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::models::{ApiResponse, ResearcherAccount};
use crate::state::AppState;

/// Demo master token, usable without registration.
pub const DEMO_API_KEY: &str = "zkgen_demo_master_token";

/// Extractor used by handlers that require a logged-in researcher.
pub struct Authed(pub ResearcherAccount);

impl FromRequestParts<AppState> for Authed {
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
                .unwrap_or("");
            let token = header.strip_prefix("Bearer ").unwrap_or(header).trim();
            if token == DEMO_API_KEY {
                return Ok(Authed(ResearcherAccount {
                    api_key: DEMO_API_KEY.to_string(),
                    name: "demo".into(),
                    organization: "demo".into(),
                    created_at: 0,
                    query_count: 0,
                    proof_count: 0,
                }));
            }
            match state.get_researcher(token) {
                Some(acc) => Ok(Authed(acc)),
                None => Err(unauthorized("invalid or missing bearer token")),
            }
        }
    }
}

fn unauthorized(message: &str) -> Response {
    let body: ApiResponse<()> = ApiResponse::failure(message, "unauthorized");
    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
}


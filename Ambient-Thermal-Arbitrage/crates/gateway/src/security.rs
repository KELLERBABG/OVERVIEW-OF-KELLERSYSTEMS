//! Security: HMAC-SHA256 bearer token authentication for API routes.
//!
//! Tokens are derived as:
//!   token = base64url( HMAC-SHA256( api_secret, route_scope ) )
//!
//! This gives per-route capability tokens derived from a single secret.
//! In production, set `ATA_SECRET` environment variable to a random value.

use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Derive a bearer token for a given scope from the api secret.
pub fn derive_token(secret: &str, scope: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts keys of any size");
    mac.update(scope.as_bytes());
    let digest = mac.finalize().into_bytes();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// Middleware that checks a bearer token against a scope.
pub async fn require_auth(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AuthState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !state.auth_enabled {
        return Ok(next.run(request).await);
    }

    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let scope = request.uri().path().to_string();
    let expected = derive_token(&state.secret, &scope);

    if constant_time_eq(token.as_bytes(), expected.as_bytes()) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Constant-time comparison to avoid timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Shared auth state for middleware.
#[derive(Clone)]
pub struct AuthState {
    pub secret: String,
    pub auth_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_derivation_is_deterministic() {
        let t1 = derive_token("secret", "/price");
        let t2 = derive_token("secret", "/price");
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_token_differs_by_scope() {
        let t1 = derive_token("secret", "/price");
        let t2 = derive_token("secret", "/config");
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_token_differs_by_secret() {
        let t1 = derive_token("secret-a", "/price");
        let t2 = derive_token("secret-b", "/price");
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_token_is_url_safe_no_padding() {
        let t = derive_token("secret", "/price");
        assert!(!t.contains('+'));
        assert!(!t.contains('/'));
        assert!(!t.contains('='));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
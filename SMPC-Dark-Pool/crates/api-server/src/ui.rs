//! Trading console UI (DEVELOPMENT_PLAN Phase 3.6).
//!
//! Serves a self-contained single-page trading console embedded at compile
//! time. The page requires no build step and no external assets: inline CSS
//! and JavaScript talk to the REST + WebSocket API described in
//! `docs/api-reference.md`.

/// Embedded trading console HTML.
pub const INDEX_HTML: &str = include_str!("../ui/index.html");

/// HTTP response for the trading console.
pub async fn index_response() -> axum::response::Response {
    use axum::response::IntoResponse;
    axum::response::Html(INDEX_HTML).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_html_is_valid() {
        assert!(INDEX_HTML.contains("SMPC Dark Pool"));
        assert!(INDEX_HTML.contains("/api/orders"));
        assert!(INDEX_HTML.contains("/api/trades"));
        assert!(INDEX_HTML.contains("/api/book"));
        assert!(INDEX_HTML.contains("/api/ws"));
    }

    #[tokio::test]
    async fn test_index_response_ok() {
        let resp = index_response().await;
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }
}

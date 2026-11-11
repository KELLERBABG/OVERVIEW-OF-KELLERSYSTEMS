/// Middleware for the Sybil API server.
use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
};

/// Logging middleware that traces incoming requests.
pub async fn request_logger(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    tracing::info!("→ {} {}", method, uri);
    let response = next.run(req).await;
    tracing::info!("← {} {} → {}", method, uri, response.status());
    response
}
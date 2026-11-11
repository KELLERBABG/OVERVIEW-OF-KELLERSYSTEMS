use crate::state::AppState;
use axum::{
    Json,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};

/// Auth middleware using axum's from_fn pattern (works with route_layer).
pub async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    req: Request,
    next: axum::middleware::Next,
) -> Response {
    if let Some(ref key) = state.api_key {
        let header = req.headers().get("X-API-Key").and_then(|v| v.to_str().ok());
        if header != Some(key.as_str()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    }
    next.run(req).await
}

// ---------------------------------------------------------------------------
// Rate limiter as a proper Tower Layer + Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct RateLimitLayer {
    state: Arc<AppState>,
}

impl RateLimitLayer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    state: Arc<AppState>,
}

impl<S> Service<Request> for RateLimitService<S>
where
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Response, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let client_ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        let now = Instant::now();
        let mut limiter = self.state.rate_limiter.requests.lock().unwrap();
        let timestamps = limiter.entry(client_ip).or_default();
        timestamps
            .retain(|t| now.duration_since(*t).as_secs() < self.state.rate_limiter.window_secs);
        if timestamps.len() >= self.state.rate_limiter.max_requests {
            return Box::pin(async move {
                Ok((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(serde_json::json!({"error": "rate limit exceeded"})),
                )
                    .into_response())
            });
        }
        timestamps.push(now);
        drop(limiter);

        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}

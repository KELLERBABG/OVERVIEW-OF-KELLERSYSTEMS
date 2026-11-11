use api_server::{middleware as mw, routes, state::AppState, ws};
use axum::{
    Router, middleware,
    routing::{get, post},
};
use clap::Parser;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[derive(Parser)]
struct Args {
    #[arg(long, env = "TLS_CERT_PATH")]
    tls_cert: Option<String>,
    #[arg(long, env = "TLS_KEY_PATH")]
    tls_key: Option<String>,
    #[arg(long, default_value = "0.0.0.0:8080")]
    addr: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let state: Arc<AppState> = AppState::shared();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let protected = Router::new()
        .route("/api/orders", post(routes::submit_order))
        .route("/api/orders/{id}", get(routes::get_order))
        .route("/api/book", get(routes::get_book))
        .route("/api/trades", get(routes::get_trades))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            mw::auth_middleware,
        ));

    let app = Router::new()
        .route("/api/health", get(routes::health))
        .route("/api/metrics", get(routes::metrics))
        .route("/api/ws", get(ws::ws_handler))
        .merge(protected)
        .layer(mw::RateLimitLayer::new(state.clone()))
        .layer(cors)
        .with_state(state);

    match (args.tls_cert, args.tls_key) {
        (Some(cert), Some(key)) => {
            let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
            tracing::info!("API server listening on {} with TLS", args.addr);
            axum_server::bind_rustls(args.addr.parse()?, config)
                .serve(app.into_make_service())
                .await?;
        }
        _ => {
            tracing::info!("API server listening on {} (no TLS)", args.addr);
            axum_server::bind(args.addr.parse()?)
                .serve(app.into_make_service())
                .await?;
        }
    }
    Ok(())
}

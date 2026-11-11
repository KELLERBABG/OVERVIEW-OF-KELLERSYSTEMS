//! Embedded FHE-DB server inside the phone shell.
//!
//! Spawns the real [`fhe_server::build_router`] axum stack on a private
//! loopback port and records matched-route request/response metadata for the
//! Monitor app's live wire view. The wire tap wraps the real router, so each
//! entry crossed the HTTP boundary. It records method/path/status only, not
//! bodies or sizes, so it cannot by itself prove what the server handled.

use std::net::SocketAddr;

use axum::extract::{Request, State as AxumState};
use axum::middleware::Next;
use axum::response::Response;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::wire::{WireRecord, WireRing};

/// A running embedded server handle.
pub struct EmbeddedServer {
    /// The bound loopback address.
    pub addr: SocketAddr,
    /// Shutdown signal to stop the server.
    shutdown: tokio::sync::watch::Sender<bool>,
    /// The server's tokio runtime.
    ///
    /// This field exists to keep the runtime alive: dropping a tokio
    /// `Runtime` cancels every task spawned on it, which would kill the
    /// axum listener (and the wire-tap forwarder) the moment `start`
    /// returned. Holding it here ties the server's life to this handle.
    /// It is intentionally never read — its `Drop` is the whole point.
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,
}

impl EmbeddedServer {
    /// Start the embedded FHE-DB server on an ephemeral loopback port.
    ///
    /// `sink` is the shared wire ring buffer; every HTTP request and response
    /// that crosses the loopback boundary is appended to it so the Monitor
    /// app can render the live zero-trust feed.
    pub fn start(sink: WireRing) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let state = fhe_server::AppState::new();

        // The real FHE-DB router, wrapped with the wire tap. The tap forwards
        // into an mpsc channel; a forwarder task drains it into the shared
        // ring so the Monitor reads records without blocking the server.
        let (tap_tx, tap_rx) = mpsc::channel::<WireRecord>(1024);
        runtime.spawn(forward_to_sink(tap_rx, sink));

        let router = fhe_server::build_router(state)
            .route_layer(axum::middleware::from_fn_with_state(
                WireTapState { tx: tap_tx },
                wire_tap,
            ))
            .layer(tower_http::cors::CorsLayer::permissive());

        let addr: SocketAddr = "127.0.0.1:0".parse()?;
        let listener = runtime.block_on(TcpListener::bind(addr))?;
        let bound_addr = listener.local_addr()?;

        runtime.spawn(async move {
            // `into_make_service_with_connect_info` lets the server's rate
            // limiter see the real peer address of every request.
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.changed().await;
            })
            .await
            .expect("embedded fhe-server exited unexpectedly");
        });

        Ok(Self {
            addr: bound_addr,
            shutdown: shutdown_tx,
            // Keep the runtime alive for the server's whole lifetime (see
            // the field docs — dropping it cancels the axum serve task).
            runtime,
        })
    }

    /// The base URL for the [`fhe_client::FheClient`].
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Stop the server and wait for it to gracefully shut down.
    pub fn shutdown(self) {
        let _ = self.shutdown.send(true);
        // Give the axum task a moment to unwind.
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
}

/// Drains wire records from the tap channel into the shared ring.
async fn forward_to_sink(mut tap_rx: mpsc::Receiver<WireRecord>, sink: WireRing) {
    while let Some(record) = tap_rx.recv().await {
        sink.push(record);
    }
}

/// State for the wire-tap middleware.
#[derive(Clone)]
struct WireTapState {
    /// Sender for wire records captured by the middleware.
    tx: mpsc::Sender<WireRecord>,
}

/// axum middleware capturing the method, path, and status of every request
/// and response crossing the wire boundary.
async fn wire_tap(
    AxumState(state): AxumState<WireTapState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let _ = state.tx.try_send(WireRecord::request(&method, &path));

    let response = next.run(request).await;
    let status = response.status().as_u16();
    let _ = state.tx.try_send(WireRecord::response(status));

    response
}

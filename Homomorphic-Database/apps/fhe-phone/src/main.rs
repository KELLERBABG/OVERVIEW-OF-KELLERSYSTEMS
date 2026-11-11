//! FHE-DB as a phone: launch the embedded server, then open the shell.
//!
//! Run with `cargo run --release -p fhe-phone`.

use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use fhe_phone::{Engine, WireRing};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Shared wire ring: every HTTP request/response on the loopback boundary
    // is captured here for the Monitor app.
    let wire = WireRing::new();

    // Boot the real FHE-DB stack: embedded axum server + real HTTP client.
    let engine = Engine::start(wire).context("failed to start embedded FHE server")?;
    let base = engine.server.base_url();
    tracing::info!("embedded fhe-server listening on {base}");
    tracing::info!("real fhe-client SDK bound; opening the FHE phone …");

    let shared: fhe_phone::SharedEngine = Arc::new(Mutex::new(engine));

    // Run the shell; closing the window exits the process.
    fhe_phone::shell::run(shared)
}

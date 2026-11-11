//! # encrypted_sum — End-to-End Demo
//!
//! Demonstrates the complete zero-trust flow of FHE-DB:
//!
//! 1. Boot an in-process [`fhe_server`] on an ephemeral port.
//! 2. Client generates `(ClientKey, ServerKey)`.
//! 3. Client uploads **only** the public server key.
//! 4. Client encrypts plaintext salaries locally and pushes the ciphertexts.
//! 5. Server computes SUM / AVG / COUNT homomorphically — it never sees
//!    value plaintext.
//! 6. Client decrypts the results and verifies the arithmetic.
//!
//! Run with `cargo run --release -p encrypted_sum`. The release profile is
//! strongly recommended: TFHE is much faster optimized.

use std::net::{SocketAddr, TcpListener};

use anyhow::Context as _;
use fhe_client::FheClient;
use fhe_core::{ClientKey, ScalarWidth};
use fhe_server::{app, AppState};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // ------------------------------------------------------------------
    // 1. Boot an in-process server on an ephemeral port.
    // ------------------------------------------------------------------
    let listener = TcpListener::bind("127.0.0.1:0").context("bind ephemeral port")?;
    let addr: SocketAddr = listener.local_addr().context("read ephemeral port")?;
    // Drop the std listener; axum will bind the same addr.
    drop(listener);

    let state = AppState::new();
    let router = app(state);
    let server = axum::serve(
        tokio::net::TcpListener::bind(addr)
            .await
            .context("axum bind")?,
        router,
    );
    let handle = tokio::spawn(async move {
        if let Err(e) = server.await {
            warn!("server error: {e}");
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let base_url = format!("http://{addr}");
    info!("server listening at {base_url}");
    let client = FheClient::new(base_url.clone())?;
    assert!(client.is_healthy().await, "server health check failed");
    info!("health check passed");

    // ------------------------------------------------------------------
    // 2. Client key generation. The client key never leaves this process.
    // ------------------------------------------------------------------
    let (client_key, server_key) = ClientKey::new();
    info!("generated client key (secret, stays local) + server key");

    // ------------------------------------------------------------------
    // 3. Upload only the public server key.
    // ------------------------------------------------------------------
    client.upload_server_key(&server_key).await?;
    info!("uploaded public server key to compute server");

    // ------------------------------------------------------------------
    // 4. Define schema and create the table.
    // ------------------------------------------------------------------
    let schema = fhe_client::TableSchemaJson {
        name: "salaries".to_string(),
        columns: vec![fhe_client::ColumnDefJson {
            name: "amount".to_string(),
            column_type: fhe_client::ColumnTypeJson::U32,
        }],
    };
    client.create_table(&schema).await?;
    info!("created table `salaries(amount U32)`");

    // ------------------------------------------------------------------
    // 4b. Encrypt and insert salary values. Plaintext lives only here.
    // ------------------------------------------------------------------
    let salaries: [u64; 5] = [50_000, 65_000, 72_000, 48_000, 90_000];
    let expected_sum: u64 = salaries.iter().sum();
    let expected_avg: u64 = expected_sum / salaries.len() as u64;

    let inserted = client
        .insert_plaintext(
            &client_key,
            "salaries",
            "amount",
            ScalarWidth::U32,
            &salaries,
        )
        .await?;
    assert_eq!(inserted, salaries.len());
    info!("encrypted and inserted {inserted} salary rows");

    // ------------------------------------------------------------------
    // 5. Server-side homomorphic aggregations (blind compute).
    // ------------------------------------------------------------------
    let (encrypted_sum, count) = client.sum("salaries", "amount").await?;
    let (encrypted_avg_total, avg_count) = client.avg("salaries", "amount").await?;
    let row_count = client.count("salaries", "amount").await?;

    // ------------------------------------------------------------------
    // 6. Client-side decryption + verification.
    // ------------------------------------------------------------------
    let decrypted_sum = encrypted_sum.decrypt(&client_key).as_u64();
    let decrypted_avg = encrypted_avg_total.decrypt(&client_key).as_u64();

    info!("--- results (decrypted client-side) ---");
    info!("plaintext salaries            : {salaries:?}");
    info!("SUM  server computed -> client : {decrypted_sum}");
    info!("SUM  expected                  : {expected_sum}");
    info!(
        "AVG  server computed -> client : {}",
        decrypted_avg / avg_count
    );
    info!("AVG  expected (floor)         : {expected_avg}");
    info!("count (public)                : {count} / {row_count}");

    assert_eq!(count, salaries.len() as u64, "count mismatch");
    assert_eq!(row_count, salaries.len() as u64, "count mismatch");

    // Note: FHE integer addition is exact (no rounding) up to the modulus,
    // so the SUM must match exactly. The AVG divides client-side by the
    // public count, so it matches the floor integer average.
    assert_eq!(decrypted_sum, expected_sum, "SUM mismatch");
    assert_eq!(decrypted_avg / avg_count, expected_avg, "AVG mismatch");

    info!("[PASS] all assertions passed - encrypted SUM/AVG/COUNT verified");

    // Shut the server down cleanly.
    handle.abort();
    Ok(())
}

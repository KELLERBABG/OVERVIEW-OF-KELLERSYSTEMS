//! FHE-DB standalone server binary.
//!
//! Runs the [`fhe_server`] REST API on a TCP socket with optional on-disk
//! durability. When `--data-dir` is given, the database is recovered from the
//! write-ahead log at `<data-dir>/fhe-db.log`; a legacy pre-WAL `database.bin`
//! snapshot is migrated into it exactly once. Every mutation is journaled
//! before applying (write-ahead ordering); the log is checkpointed (compacted)
//! on a periodic interval and on graceful shutdown.
//!
//! Security options: `--auth-token` (or `FHE_DB_AUTH_TOKEN`) requires
//! `Authorization: Bearer <token>` on every request except `GET /health`;
//! `--tls-cert`/`--tls-key` serve HTTPS instead of HTTP.

use std::io::IsTerminal;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use fhe_db::Wal;
use fhe_server::AppState;
use tokio::net::TcpListener;
use tokio::time::interval;

/// Name of the write-ahead log inside `--data-dir` (the durability store).
const LOG_FILE: &str = "fhe-db.log";
/// Legacy snapshot name (pre-WAL persistence). Migrated once on first boot.
const LEGACY_DATABASE_FILE: &str = "database.bin";

#[derive(Parser, Debug)]
#[command(
    name = "fhe-server",
    version,
    about = "FHE-DB encrypted database REST server (zero-trust: the server never sees value plaintext)"
)]
struct Cli {
    /// Address to bind.
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Port to listen on.
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Directory for the persistent write-ahead log. Recovered on boot and
    /// checkpointed on shutdown/interval. Omit for in-memory-only.
    #[arg(long, value_name = "DIR")]
    data_dir: Option<PathBuf>,

    /// Seconds between periodic WAL checkpoints (only with --data-dir).
    #[arg(long, default_value_t = 60)]
    save_interval_secs: u64,

    /// Require `Authorization: Bearer <token>` on every request except
    /// `GET /health`. Also read from the `FHE_DB_AUTH_TOKEN` environment
    /// variable. Omit for an open server (loopback-only default posture).
    #[arg(long, value_name = "TOKEN", env = "FHE_DB_AUTH_TOKEN")]
    auth_token: Option<String>,

    /// PEM certificate chain for HTTPS. Requires `--tls-key`; both must be
    /// given to serve TLS instead of plain HTTP.
    ///
    /// Only present when `fhe-server` is built with the `tls` cargo feature
    /// (see the crate manifest): the rustls `aws-lc-rs` provider needs a C
    /// compiler, so the feature is opt-in and exercised by CI.
    #[cfg(feature = "tls")]
    #[arg(long, value_name = "PATH", requires = "tls_key")]
    tls_cert: Option<PathBuf>,

    /// PEM private key (PKCS#8 or PKCS#1) for `--tls-cert`.
    #[cfg(feature = "tls")]
    #[arg(long, value_name = "PATH", requires = "tls_cert")]
    tls_key: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Disable ANSI colouring when stdout is redirected: structured events are
    // grepped by operators and durability tests, and a colour reset between a
    // field name and its `=` breaks `field=value` matching.
    let ansi = std::io::stdout().is_terminal();
    tracing_subscriber::fmt()
        .with_ansi(ansi)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,fhe_server=debug".into()),
        )
        .init();

    let cli = Cli::parse();

    // Recover the WAL (creating it if absent), migrating a legacy pre-WAL
    // snapshot exactly once.
    let log_path = cli.data_dir.as_ref().map(|d| d.join(LOG_FILE));
    // The boot report seeds the WAL metrics below; it is `None` for the
    // in-memory mode, where there is no log to report on.
    let mut recovery: Option<fhe_db::RecoveryReport> = None;
    let (db, wal) = if let Some(path) = &log_path {
        let legacy = cli.data_dir.as_ref().map(|d| d.join(LEGACY_DATABASE_FILE));
        if !path.exists() && legacy.as_ref().is_some_and(|l| l.exists()) {
            let legacy = legacy
                .as_ref()
                .expect("the legacy branch is only taken with a legacy path");
            tracing::info!(
                target: "fhe_server::wal",
                event = "wal.migrate",
                outcome = "start",
                legacy = %legacy.display(),
                "migrating a legacy snapshot into the write-ahead log"
            );
            let db = fhe_db::EncryptedDatabase::load(legacy)
                .with_context(|| format!("failed to load legacy snapshot {}", legacy.display()))?;
            let mut wal = Wal::create(path)
                .with_context(|| format!("failed to create WAL {}", path.display()))?;
            let checkpoint = wal
                .checkpoint_with_report(&db)
                .with_context(|| format!("failed to checkpoint migrated WAL {}", path.display()))?;
            tracing::info!(
                target: "fhe_server::wal",
                event = "wal.migrate",
                outcome = "ok",
                legacy = %legacy.display(),
                snapshot_bytes = checkpoint.snapshot_bytes,
                log_bytes = checkpoint.log_bytes_after,
                "legacy snapshot migrated"
            );
            recovery = Some(fhe_db::RecoveryReport {
                snapshot_bytes: checkpoint.snapshot_bytes,
                records_replayed: 0,
                record_bytes_replayed: 0,
                torn_tail_bytes_truncated: 0,
                log_bytes: checkpoint.log_bytes_after,
                created: true,
            });
            (db, Some(wal))
        } else {
            let (db, wal, report) = Wal::recover_with_report(path)
                .map_err(|e| {
                    tracing::error!(
                        target: "fhe_server::wal",
                        event = "wal.recover",
                        outcome = "error",
                        path = %path.display(),
                        error = %e,
                        "write-ahead log recovery failed"
                    );
                    e
                })
                .with_context(|| {
                    format!("failed to recover database from WAL {}", path.display())
                })?;
            // Exactly one structured line per boot: whether the log was
            // created, what replayed, and whether a torn tail was healed.
            tracing::info!(
                target: "fhe_server::wal",
                event = "wal.recover",
                outcome = if report.created { "created" } else { "recovered" },
                path = %path.display(),
                snapshot_bytes = report.snapshot_bytes,
                records_replayed = report.records_replayed,
                record_bytes_replayed = report.record_bytes_replayed,
                torn_tail_bytes_truncated = report.torn_tail_bytes_truncated,
                log_bytes = report.log_bytes,
                "write-ahead log recovered"
            );
            recovery = Some(report);
            (db, Some(wal))
        }
    } else {
        tracing::info!("running with an in-memory database (no --data-dir)");
        (fhe_db::EncryptedDatabase::new(), None)
    };

    let state = match wal {
        Some(wal) => {
            AppState::with_limits_and_db_and_wal(fhe_server::routes::Limits::default(), db, wal)
        }
        None => AppState::with_db(db),
    };
    let state = match &cli.auth_token {
        Some(token) => state.with_auth(token.clone()),
        None => state,
    };
    if let Some(report) = &recovery {
        state.metrics.record_recovery(report);
    }

    // Periodic WAL checkpoint (compaction) so the log cannot grow without
    // bound. `checkpoint_wal` owns the db-before-wal lock order shared with
    // the mutation handlers; do not inline the lock acquisitions here.
    let save_state = state.clone();
    if log_path.is_some() {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(cli.save_interval_secs));
            loop {
                ticker.tick().await;
                // `checkpoint_wal` emits the structured `wal.checkpoint`
                // event (including a failing one); the task must stay alive
                // either way so the next interval retries.
                let _ = save_state.checkpoint_wal().await;
            }
        });
    }

    let addr: SocketAddr = format!("{}:{}", cli.bind, cli.port).parse()?;
    let app = fhe_server::app(state.clone()).into_make_service_with_connect_info::<SocketAddr>();

    // Serve HTTPS when a key pair is provided, plain HTTP otherwise. The TLS
    // branch is only compiled with the `tls` cargo feature (aws-lc-rs needs a
    // C compiler; this repo's Windows-GNU host builds without it — CI covers
    // the TLS build).
    #[cfg(feature = "tls")]
    if let (Some(cert), Some(key)) = (&cli.tls_cert, &cli.tls_key) {
        let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key)
            .await
            .with_context(|| {
                format!(
                    "failed to load TLS key pair (cert: {}, key: {})",
                    cert.display(),
                    key.display()
                )
            })?;
        // axum-server 0.7 serves until its handle is shut down; the signal
        // watcher triggers the graceful drain. `serve` then returns and the
        // WAL checkpoint below runs exactly as in the plain-HTTP path.
        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();
        tokio::spawn(async move {
            shutdown_signal().await;
            shutdown_handle.graceful_shutdown(None);
        });
        tracing::info!("FHE-DB server listening on https://{addr}");
        axum_server::bind_rustls(addr, config)
            .handle(handle)
            .serve(app)
            .await?;
    } else {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("failed to bind {addr}"))?;
        tracing::info!("FHE-DB server listening on http://{addr}");
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    }

    #[cfg(not(feature = "tls"))]
    {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("failed to bind {addr}"))?;
        tracing::info!("FHE-DB server listening on http://{addr}");
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    }

    // Checkpoint on graceful shutdown so recovery starts from a compact log.
    if let Some(path) = &log_path {
        state
            .checkpoint_wal()
            .await
            .with_context(|| format!("failed to checkpoint WAL on shutdown: {}", path.display()))?;
    }
    tracing::info!(
        target: "fhe_server::server",
        event = "server.shutdown",
        "server stopped"
    );

    Ok(())
}

/// Resolve on Ctrl-C (all platforms), SIGTERM (unix), or Ctrl-Break (Windows).
///
/// Ctrl-Break matters on Windows: it is what `GenerateConsoleCtrlEvent` and
/// console hosts deliver, and an unhandled Ctrl-Break terminates the process
/// before the shutdown checkpoint runs. Registering the handler makes both
/// console events drain and checkpoint instead of aborting.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    #[cfg(windows)]
    let ctrl_break = async {
        tokio::signal::windows::ctrl_break()
            .expect("failed to install Ctrl-Break handler")
            .recv()
            .await;
    };

    #[cfg(not(windows))]
    let ctrl_break = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = ctrl_break => {},
    }
    tracing::info!("shutdown signal received; draining");
}

//! Process-level durability tests for `fhe-server`.
//!
//! These tests spawn the **real** server binary against a real `--data-dir`
//! write-ahead log, mutate it over real HTTP, and then either stop it with the
//! platform's graceful console signal or kill the process outright. They are
//! the end-to-end half of the durability contract:
//!
//! - graceful stop → drain, checkpoint (log compacted to header + snapshot),
//!   exit 0, and a restart serves exactly what the client committed;
//! - hard kill → every mutation the client received `200` for survives a
//!   restart, a rejected mutation leaves no record behind, and the recovered
//!   log is still appendable;
//! - a torn tail (crash mid-append) heals on boot and keeps the committed
//!   prefix instead of refusing to start.
//!
//! The deterministic per-boundary matrix (crash before append, after append
//! but before the in-memory apply, mid-checkpoint with a stale `.tmp`) lives
//! in `fhe_db::wal`'s unit tests, where each on-disk state can be constructed
//! exactly rather than raced against a timer.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use base64::Engine;
use fhe_core::{Ciphertext, ClientKey, Encryptor, ScalarWidth};

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Unique temp directory per test (tests run in parallel in one process).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "fhe_server_durability_{tag}_{}_{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Ask the OS for an unused loopback port, then release it for the child.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

/// The real server binary built for this test run.
fn server_binary() -> &'static str {
    env!("CARGO_BIN_EXE_fhe-server")
}

/// A running `fhe-server` child process plus the plumbing to stop it.
struct ServerProc {
    child: Child,
    base: String,
    data_dir: PathBuf,
    captured: PathBuf,
    /// Whether the child has been reaped. Never signal an already-reaped pid:
    /// on unix the OS may have recycled it for an unrelated process.
    exited: bool,
}

impl ServerProc {
    /// Spawn the server against `data_dir` and wait until it answers, retrying
    /// on a fresh port if the probe port was claimed by a concurrent test in
    /// the window between `free_port()` and the child's `bind`.
    async fn launch(data_dir: &Path) -> Self {
        let mut last = String::new();
        for _ in 0..4 {
            let mut proc = Self::start(data_dir);
            match proc.wait_ready(Duration::from_secs(60)).await {
                Ok(()) => return proc,
                Err(e) => {
                    // Only an early exit is retryable (a port claimed between
                    // the probe and the bind). A hang is reported at once.
                    if !proc.exited {
                        panic!("server did not become ready: {e}");
                    }
                    last = e;
                }
            }
        }
        panic!("server did not start after 4 attempts: {last}");
    }

    /// Spawn the server against `data_dir` on a fresh loopback port.
    fn start(data_dir: &Path) -> Self {
        let port = free_port();
        let base = format!("http://127.0.0.1:{port}");
        let captured = data_dir.join("server-stdio.log");
        let out = std::fs::File::create(&captured).expect("create stdio log");
        let err = out.try_clone().expect("clone stdio log");

        let mut cmd = Command::new(server_binary());
        cmd.args([
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--data-dir",
        ])
        .arg(data_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        // Never inherit an auth token from the developer/CI environment: these
        // tests speak to an open loopback server.
        .env_remove("FHE_DB_AUTH_TOKEN");
        configure_process_group(&mut cmd);

        let child = cmd.spawn().expect("spawn fhe-server");
        Self {
            child,
            base,
            data_dir: data_dir.to_path_buf(),
            captured,
            exited: false,
        }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn log_path(&self) -> PathBuf {
        self.data_dir.join("fhe-db.log")
    }

    fn captured_output(&self) -> String {
        std::fs::read_to_string(&self.captured).unwrap_or_default()
    }

    /// Poll `GET /health` until the server answers or `timeout` elapses.
    async fn wait_ready(&mut self, timeout: Duration) -> Result<(), String> {
        let http = reqwest::Client::new();
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().expect("poll child") {
                return Err(format!(
                    "server exited before becoming ready ({status:?}); output:\n{}",
                    self.captured_output()
                ));
            }
            let attempt = http
                .get(format!("{}/health", self.base))
                .timeout(Duration::from_millis(500))
                .send()
                .await;
            if let Ok(response) = attempt {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "server did not become ready within {timeout:?}; output:\n{}",
                    self.captured_output()
                ));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Deliver the platform's graceful console stop and wait for exit.
    async fn graceful_stop(&mut self, timeout: Duration) -> ExitStatus {
        deliver_graceful_stop(self.pid());
        self.wait_for_exit(timeout).await
    }

    /// Kill without any shutdown handling, mirroring a power loss or SIGKILL.
    async fn hard_kill(&mut self, timeout: Duration) -> ExitStatus {
        // `Child::kill` is TerminateProcess on Windows and SIGKILL on unix:
        // neither runs the graceful shutdown path or flushes anything.
        if !self.exited {
            self.child.kill().expect("kill child");
        }
        self.wait_for_exit(timeout).await
    }

    async fn wait_for_exit(&mut self, timeout: Duration) -> ExitStatus {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().expect("poll child") {
                self.exited = true;
                return status;
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                self.exited = true;
                panic!(
                    "server did not exit within {timeout:?}; output:\n{}",
                    self.captured_output()
                );
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    async fn http(&self) -> reqwest::Client {
        reqwest::Client::new()
    }

    async fn create_table(&self, name: &str, column: &str, column_type: &str) -> reqwest::Response {
        self.http()
            .await
            .post(format!("{}/tables", self.base))
            .json(&serde_json::json!({
                "name": name,
                "columns": [{ "name": column, "column_type": column_type }],
            }))
            .send()
            .await
            .expect("POST /tables")
    }

    async fn insert(&self, table: &str, column: &str, cts: &[Ciphertext]) -> reqwest::Response {
        let encoded: Vec<String> = cts
            .iter()
            .map(|ct| B64.encode(ct.to_envelope_bytes().expect("envelope bytes")))
            .collect();
        self.http()
            .await
            .post(format!("{}/tables/{table}/rows", self.base))
            .json(&serde_json::json!({ "column": column, "ciphertexts": encoded }))
            .send()
            .await
            .expect("POST rows")
    }

    /// Read a column back and decrypt every envelope locally.
    async fn read_values(&self, table: &str, column: &str, key: &ClientKey) -> Vec<u64> {
        let response = self
            .http()
            .await
            .get(format!("{}/tables/{table}/ciphertexts/{column}", self.base))
            .send()
            .await
            .expect("GET ciphertexts");
        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "read-back failed: {}",
            response.text().await.unwrap_or_default()
        );
        let body: serde_json::Value = response.json().await.expect("read-back JSON");
        let envelopes = body["ciphertexts"]
            .as_array()
            .expect("ciphertexts array")
            .clone();
        assert_eq!(
            body["count"].as_u64().expect("count"),
            envelopes.len() as u64,
            "count matches envelope list"
        );
        envelopes
            .iter()
            .map(|encoded| {
                let bytes = B64
                    .decode(encoded.as_str().expect("envelope string"))
                    .expect("base64 envelope");
                let ct = Ciphertext::from_envelope_bytes(&bytes).expect("parse envelope");
                ct.decrypt(key).as_u64()
            })
            .collect()
    }
}

impl Drop for ServerProc {
    fn drop(&mut self) {
        // A failed assertion must not leak a server process (or its port).
        // Skip an already-reaped child: signalling a recycled pid could kill
        // an unrelated process.
        if !self.exited {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Put the child in its own process group so a console control event can be
/// addressed to it alone (CTRL_BREAK to the whole console would also stop the
/// test runner).
#[cfg(windows)]
fn configure_process_group(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

#[cfg(unix)]
fn configure_process_group(_cmd: &mut Command) {}

/// Deliver the graceful stop the OS would send on Ctrl-C / Ctrl-Break / SIGTERM.
#[cfg(windows)]
fn deliver_graceful_stop(pid: u32) {
    const CTRL_BREAK_EVENT: u32 = 1;
    extern "system" {
        fn GenerateConsoleCtrlEvent(dw_ctrl_event: u32, dw_process_group_id: u32) -> i32;
    }
    // CTRL_BREAK is the only console event that can be addressed to a process
    // group created with CREATE_NEW_PROCESS_GROUP, and the server installs a
    // Ctrl-Break handler (see `shutdown_signal` in main.rs).
    let ok = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) };
    assert!(
        ok != 0,
        "GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, {pid}) failed: {}",
        std::io::Error::last_os_error()
    );
}

#[cfg(unix)]
fn deliver_graceful_stop(pid: u32) {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .expect("run kill -TERM");
    assert!(status.success(), "kill -TERM {pid} failed: {status:?}");
}

/// The log's on-disk size and the byte length of its snapshot region.
fn log_layout(path: &Path) -> (u64, u64) {
    let bytes = std::fs::read(path).expect("read WAL");
    assert!(bytes.len() >= 24, "WAL shorter than its fixed header");
    let snapshot_len = u64::from_le_bytes(bytes[16..24].try_into().expect("8-byte field"));
    (bytes.len() as u64, snapshot_len)
}

/// Encrypt `values` at `width` with a fresh keypair.
fn encrypt(key: &ClientKey, values: &[u64], width: ScalarWidth) -> Vec<Ciphertext> {
    let encryptor = Encryptor::new(key);
    values
        .iter()
        .map(|v| encryptor.encrypt_u64(*v, width).expect("encrypt"))
        .collect()
}

/// A1: a graceful stop drains, checkpoints, exits 0, and the restart serves
/// exactly the committed rows.
#[tokio::test]
async fn graceful_stop_compacts_the_wal_and_restart_recovers() {
    let dir = temp_dir("graceful");
    let (client_key, _server_key) = ClientKey::new();
    let mut server = ServerProc::launch(&dir).await;

    let create = server.create_table("ledger", "amount", "U32").await;
    assert_eq!(create.status(), reqwest::StatusCode::OK, "create table");

    let rows = encrypt(&client_key, &[11, 22, 33], ScalarWidth::U32);
    let insert = server.insert("ledger", "amount", &rows).await;
    assert_eq!(insert.status(), reqwest::StatusCode::OK, "insert rows");
    let body: serde_json::Value = insert.json().await.expect("insert JSON");
    assert_eq!(body["inserted"], 3);

    // The log has records beyond the snapshot: this is the state a crash would
    // have to replay, and a graceful stop must compact away.
    let (before_len, before_snapshot) = log_layout(&server.log_path());
    assert!(
        before_len > 24 + before_snapshot,
        "expected appended records before shutdown ({before_len} bytes, snapshot {before_snapshot})"
    );

    let status = server.graceful_stop(Duration::from_secs(60)).await;
    assert!(
        status.success(),
        "graceful stop must exit 0, got {status:?}; output:\n{}",
        server.captured_output()
    );

    // B4: the stop must report the compaction it performed, not just do it.
    let output = server.captured_output();
    assert!(
        output.contains("wal.checkpoint"),
        "graceful stop must emit a structured checkpoint event; output:\n{output}"
    );
    assert!(
        output.contains("records_compacted"),
        "the checkpoint event must report compacted records; output:\n{output}"
    );

    // Compaction on shutdown: header + snapshot, no records left to replay.
    let (after_len, after_snapshot) = log_layout(&server.log_path());
    assert!(after_snapshot > 0, "checkpointed snapshot is non-empty");
    assert_eq!(
        after_len,
        24 + after_snapshot,
        "shutdown checkpoint must leave header + snapshot only"
    );
    assert!(
        after_len < before_len,
        "checkpoint compacts the log ({before_len} -> {after_len})"
    );

    let mut restarted = ServerProc::launch(&dir).await;
    let values = restarted.read_values("ledger", "amount", &client_key).await;
    assert_eq!(values, vec![11, 22, 33], "committed rows recovered");

    // Reads are not mutations: the recovered log still holds no records.
    let (len_after_reads, snap_after_reads) = log_layout(&restarted.log_path());
    assert_eq!(len_after_reads, 24 + snap_after_reads);

    let status = restarted.graceful_stop(Duration::from_secs(60)).await;
    assert!(status.success(), "second graceful stop exits 0");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A2: every acknowledged mutation survives a hard kill, a rejected mutation
/// leaves no record that blocks startup, and the recovered log is appendable.
#[tokio::test]
async fn hard_kill_loses_nothing_acknowledged_and_stays_appendable() {
    let dir = temp_dir("hardkill");
    let (client_key, _server_key) = ClientKey::new();
    let mut server = ServerProc::launch(&dir).await;

    let create = server.create_table("payments", "amount", "U32").await;
    assert_eq!(create.status(), reqwest::StatusCode::OK);

    let committed = encrypt(&client_key, &[100, 250], ScalarWidth::U32);
    let insert = server.insert("payments", "amount", &committed).await;
    assert_eq!(
        insert.status(),
        reqwest::StatusCode::OK,
        "acknowledged insert"
    );

    // A rejected mutation (U16 envelope into a U32 column) must not leave a
    // WAL record: the F2 invariant, here through the real process boundary.
    let wrong = encrypt(&client_key, &[5], ScalarWidth::U16);
    let rejected = server.insert("payments", "amount", &wrong).await;
    assert_eq!(
        rejected.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "width mismatch is rejected"
    );
    let body: serde_json::Value = rejected.json().await.expect("error JSON");
    assert_eq!(body["code"], "WIDTH_MISMATCH");

    let (len_before_kill, _) = log_layout(&server.log_path());
    server.hard_kill(Duration::from_secs(30)).await;

    // Nothing the client was told "ok" about may be lost, and the rejected
    // batch must not have grown the log.
    let mut restarted = ServerProc::launch(&dir).await;
    // B4: boot recovery is observable: one structured event with the replay
    // counts, emitted before the server serves.
    let recovered_output = restarted.captured_output();
    assert!(
        recovered_output.contains("wal.recover"),
        "boot must emit a structured recovery event; output:\n{recovered_output}"
    );
    assert!(
        recovered_output.contains("records_replayed"),
        "the recovery event must report replayed records; output:\n{recovered_output}"
    );
    assert_eq!(
        restarted
            .read_values("payments", "amount", &client_key)
            .await,
        vec![100, 250],
        "acknowledged rows survive a hard kill"
    );

    // The recovered log must still accept new writes (a poisoned or truncated
    // log would fail here or on the next restart).
    let more = encrypt(&client_key, &[7], ScalarWidth::U32);
    let insert = restarted.insert("payments", "amount", &more).await;
    assert_eq!(
        insert.status(),
        reqwest::StatusCode::OK,
        "post-recovery insert"
    );

    // Second crash round: the row written after the first recovery is durable
    // too, and the rejected batch still left no record behind.
    restarted.hard_kill(Duration::from_secs(30)).await;
    let (len_after_rounds, _) = log_layout(&restarted.log_path());
    assert!(
        len_after_rounds > len_before_kill,
        "the post-recovery insert was journaled ({len_before_kill} -> {len_after_rounds})"
    );

    let mut third = ServerProc::launch(&dir).await;
    assert_eq!(
        third.read_values("payments", "amount", &client_key).await,
        vec![100, 250, 7],
        "rows committed across two crash/recovery cycles survive"
    );
    let status = third.graceful_stop(Duration::from_secs(60)).await;
    assert!(status.success());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A2: a torn tail (crash mid-append) heals on boot and keeps the committed
/// prefix instead of refusing to start.
#[tokio::test]
async fn torn_tail_heals_on_boot_and_keeps_the_committed_prefix() {
    let dir = temp_dir("torntail");
    let (client_key, _server_key) = ClientKey::new();
    let mut server = ServerProc::launch(&dir).await;

    let create = server.create_table("events", "value", "U32").await;
    assert_eq!(create.status(), reqwest::StatusCode::OK);

    // Two separate requests → two records, so truncating the tail costs
    // exactly the last one.
    let first = encrypt(&client_key, &[7], ScalarWidth::U32);
    let response = server.insert("events", "value", &first).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let second = encrypt(&client_key, &[8], ScalarWidth::U32);
    let response = server.insert("events", "value", &second).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    server.hard_kill(Duration::from_secs(30)).await;

    // Simulate a crash part-way through the trailing record's write: chop the
    // last 5 bytes, which always lands inside the final record's payload/CRC.
    let log = server.log_path();
    let full = std::fs::read(&log).expect("read WAL");
    let truncated_len = full.len() - 5;
    let mut file = std::fs::File::create(&log).expect("truncate WAL");
    file.write_all(&full[..truncated_len]).expect("write WAL");
    file.sync_all().expect("sync WAL");
    drop(file);

    // Boot must self-heal (not exit 1) and serve the committed prefix.
    let mut healed = ServerProc::launch(&dir).await;
    // B4: a healed torn tail is reported with the number of bytes dropped,
    // so an operator does not have to infer it from a smaller file.
    let healed_output = healed.captured_output();
    // The whole trailing record is unreadable, so the reported region is its
    // full frame (not the 5 chopped bytes): assert it is present and non-zero.
    assert!(
        healed_output.contains("torn_tail_bytes_truncated="),
        "boot must report the healed torn tail; output:\n{healed_output}"
    );
    assert!(
        !healed_output.contains("torn_tail_bytes_truncated=0 "),
        "a healed tail must not report zero dropped bytes; output:\n{healed_output}"
    );
    assert_eq!(
        healed.read_values("events", "value", &client_key).await,
        vec![7],
        "only the torn trailing record is dropped"
    );
    let (healed_len, healed_snapshot) = log_layout(&healed.log_path());
    assert!(
        healed_len < truncated_len as u64,
        "boot truncated the torn tail ({truncated_len} -> {healed_len})"
    );
    assert!(healed_len >= 24 + healed_snapshot);

    let status = healed.graceful_stop(Duration::from_secs(60)).await;
    assert!(
        status.success(),
        "healed server exits 0; output:\n{}",
        healed.captured_output()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

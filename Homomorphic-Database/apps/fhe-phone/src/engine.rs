//! The FHE engine: a real `fhe-client` session against the embedded server.
//!
//! Every app on the phone drives the **actual** FHE-DB stack through the
//! [`fhe_client::FheClient`] HTTP SDK. Values are encrypted on-device with
//! [`fhe_core::Encryptor`], pushed to the embedded [`fhe_server`] as base64
//! envelopes, aggregated homomorphically, and decrypted only back on the
//! phone. Nothing here is simulated.
//!
//! Heavy operations (key generation, encryption batches, inference) run on
//! dedicated worker threads so the webview event loop never blocks; results
//! are pushed back to the shell over IPC.

use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context as _};
use fhe_client::{ColumnDefJson, ColumnTypeJson, FheClient, TableSchemaJson};
use fhe_core::nn::FixedPoint;
use fhe_core::{ClientKey, Encryptor, ScalarWidth};
use serde_json::{json, Value};
use zeroize::Zeroizing;

use crate::keystore::{KeyStore, PinCheck, Unlock};
use crate::server::EmbeddedServer;
use crate::wire::WireRing;

/// The secret-key format note shown in the Vault app.
const VAULT_NOTE: &str = "The secret ClientKey is never sent to the server. \
It stays on this phone, encrypted at rest, and can be exported inside a \
PIN-protected backup. The server holds only the public ServerKey.";

/// Encrypted-column note shown in the Database app.
const DB_NOTE: &str = "Every value is encrypted at the application layer as a \
TFHE ciphertext envelope. The server sees ciphertexts and metadata, never \
value plaintext.";

/// Shares the engine across worker threads and the IPC handler.
pub type SharedEngine = Arc<Mutex<Engine>>;

/// The phone's single FHE session.
pub struct Engine {
    /// Real HTTP client SDK bound to the embedded server.
    pub client: FheClient,
    /// Embedded fhe-server handle.
    pub server: EmbeddedServer,
    /// Secret key, present once [`Engine::keygen`] has run or the phone has
    /// been unlocked with a stored key.
    pub client_key: Option<Arc<ClientKey>>,
    /// Whether the encrypted XNOR model is installed server-side.
    pub model_installed: bool,
    /// Wire ring for the Monitor app.
    pub wire: WireRing,
    /// Encrypted-at-rest key store (PIN-gated; see [`KeyStore`]).
    pub store: KeyStore,
    /// The verified PIN, held in memory only while the phone is unlocked so
    /// `keygen` can persist a fresh key pair without re-asking. Wrapped in
    /// [`Zeroizing`] so locking wipes the buffer instead of leaving the PIN
    /// in freed heap memory.
    pub current_pin: Option<Zeroizing<String>>,
}

impl Engine {
    /// Start the embedded server and bind the real client SDK.
    pub fn start(wire: WireRing) -> anyhow::Result<Self> {
        Self::start_with_store(wire, None)
    }

    /// Like [`Engine::start`], but root the encrypted key store at an
    /// explicit directory. Tests use this to keep real user data untouched;
    /// production uses the platform data directory.
    pub fn start_with_store(
        wire: WireRing,
        store_dir: Option<std::path::PathBuf>,
    ) -> anyhow::Result<Self> {
        let server = EmbeddedServer::start(wire.clone())?;
        let client = FheClient::new(format!("http://{}", server.addr))
            .map_err(|e| anyhow!("failed to create FHE client: {e}"))?;
        let store = match store_dir {
            Some(dir) => KeyStore::open_at(&dir)
                .map_err(|e| anyhow!("failed to open the test key store: {e}"))?,
            None => KeyStore::open()
                .map_err(|e| anyhow!("failed to open the encrypted key store: {e}"))?,
        };
        Ok(Self {
            client,
            server,
            client_key: None,
            model_installed: false,
            wire,
            store,
            current_pin: None,
        })
    }

    /// The secret client key, or an error telling the user to run keygen.
    fn key(&self) -> anyhow::Result<Arc<ClientKey>> {
        self.client_key.clone().ok_or_else(|| {
            anyhow!("No keys generated yet. Open the Vault app and tap Generate Keys.")
        })
    }

    /// Run a fallible engine op on a fresh worker runtime.
    fn run_blocking<F, T>(f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&tokio::runtime::Runtime) -> anyhow::Result<T>,
    {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        f(&rt)
    }
}

/// Dispatch one IPC op and return a JSON envelope.
pub fn dispatch(engine: &SharedEngine, req: &Value) -> Value {
    let op = req
        .get("op")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);

    let outcome = match op.as_str() {
        "status" => status(engine),
        "keygen" => keygen(engine),
        "lock.status" => lock_status(engine),
        "lock.setpin" => lock_setpin(engine, req),
        "lock.changepin" => lock_changepin(engine, req),
        "lock.unlock" => lock_unlock(engine, req),
        "lock.lock" => lock_lock(engine),
        "backup.export" => backup_export(engine, req),
        "backup.import" => backup_import(engine, req),
        "vault.destroy" => vault_destroy(engine, req),
        "vault.info" => vault_info(engine),
        "vault.reveal" => vault_reveal(engine, req),
        "tables.list" => tables_list(engine),
        "wallet.init" => wallet_init(engine),
        "wallet.balance" => wallet_balance(engine),
        "wallet.deposit" => wallet_deposit(engine, req),
        "wallet.withdraw" => wallet_withdraw(engine, req),
        "health.init" => health_init(engine, req),
        "health.add" => health_add(engine, req),
        "health.stats" => health_stats(engine),
        "messages.send" => messages_send(engine, req),
        "messages.list" => messages_list(engine),
        "neural.install" => neural_install(engine),
        "neural.infer" => neural_infer(engine, req),
        "monitor.snapshot" => monitor_snapshot(engine),
        _ => Err(anyhow!("unknown op `{op}`")),
    };

    match outcome {
        Ok(data) => json!({ "id": id, "ok": true, "data": data }),
        Err(err) => json!({ "id": id, "ok": false, "error": err.to_string() }),
    }
}

// ---------------------------------------------------------------------------
// Status / keygen / vault
// ---------------------------------------------------------------------------

/// `status` — server health, key state, model state, and table list.
fn status(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    let healthy = Engine::run_blocking(|rt| Ok(rt.block_on(me.client.is_healthy())))?;
    let tables = Engine::run_blocking(|rt| {
        Ok(rt
            .block_on(me.client.list_tables())
            .map(|t| t.len())
            .unwrap_or(0))
    })?;
    Ok(json!({
        "healthy": healthy,
        "port": me.server.addr.port(),
        "keys": me.client_key.is_some(),
        "locked": me.client_key.is_none(),
        "pinSet": me.store.has_pin(),
        "keyStored": me.store.has_keys(),
        "model": me.model_installed,
        "tables": tables,
    }))
}

/// `keygen` — generate `(ClientKey, ServerKey)`, upload the public server
/// key to the embedded server, and persist the key pair encrypted at rest
/// under the configured PIN. The secret key stays on the phone.
fn keygen(engine: &SharedEngine) -> anyhow::Result<Value> {
    // Fail fast if keys already exist.
    {
        let me = engine.lock().expect("engine lock");
        if me.client_key.is_some() {
            return Ok(json!({ "already": true, "note": VAULT_NOTE }));
        }
        if me.store.has_keys() {
            return Err(anyhow!(
                "Keys already exist on this device. Unlock to use them; generating a new pair would overwrite the stored key."
            ));
        }
        if !me.store.has_pin() {
            return Err(anyhow!(
                "Set a PIN first: Settings → Security → Set PIN. The keys are stored encrypted at rest under your PIN."
            ));
        }
    }

    // Key generation is CPU-heavy; run on this worker thread, then upload the
    // public server key over HTTP.
    let (client_key, server_key) = ClientKey::new();
    let encrypted_weights_note = "ServerKey uploaded (public, cannot decrypt).";

    let ck_arc = Arc::new(client_key);
    {
        let mut me = engine.lock().expect("engine lock");
        let pin = me
            .current_pin
            .clone()
            .ok_or_else(|| anyhow!("Unlock the phone before generating keys."))?;
        me.store
            .save_keys(ck_arc.as_ref(), &server_key, pin.as_str())?;
        me.client_key = Some(ck_arc.clone());
    }
    let client = {
        let me = engine.lock().expect("engine lock");
        me.client.clone()
    };
    Engine::run_blocking(|rt| {
        rt.block_on(client.upload_server_key(&server_key))
            .context("uploading server key")?;
        Ok(())
    })?;

    Ok(json!({
        "already": false,
        "note": VAULT_NOTE,
        "detail": encrypted_weights_note,
        "security": "TFHE-rs 2M128 parameter set (unaudited) · server sees ciphertexts and metadata",
        "stored": "encrypted at rest under your PIN (AES-256-GCM, Argon2id)",
    }))
}

// ---------------------------------------------------------------------------
// Lock screen & encrypted-at-rest key storage
// ---------------------------------------------------------------------------

/// `lock.status` — PIN / stored-key / locked state for the lock screen.
fn lock_status(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    Ok(json!({
        "pinSet": me.store.has_pin(),
        "keyStored": me.store.has_keys(),
        "locked": me.client_key.is_none(),
        "keys": me.client_key.is_some(),
        "storeDir": me.store.dir().display().to_string(),
    }))
}

/// `lock.setpin {pin}` — configure the lock PIN (refuses to replace an
/// existing PIN; use `lock.changepin`).
fn lock_setpin(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let pin = required_str(req, "pin")?;
    let me = engine.lock().expect("engine lock");
    me.store.set_pin(&pin)?;
    Ok(json!({ "ok": true }))
}

/// `lock.changepin {old, new}` — verify the current PIN, then re-encrypt the
/// stored keys (if any) under the new PIN in a **single atomic vault write**.
///
/// The old flow wrote the new PIN hash and then re-encrypted the key file as
/// two separate commits; a failure in between locked the user out. The
/// keystore now performs both in one commit, so there is no mixed state.
fn lock_changepin(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let old = Zeroizing::new(required_str(req, "old")?);
    let new_pin = Zeroizing::new(required_str(req, "new")?);
    let mut me = engine.lock().expect("engine lock");
    me.store.change_pin(old.as_str(), new_pin.as_str())?;
    // Keep the in-memory PIN in sync when the phone is unlocked.
    me.current_pin = Some(new_pin);
    Ok(json!({ "ok": true }))
}

/// `lock.unlock {pin}` — verify the PIN (with the persisted lockout), load
/// the stored key pair (if any), and upload the public server key.
fn lock_unlock(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let pin = Zeroizing::new(required_str(req, "pin")?);
    {
        let me = engine.lock().expect("engine lock");
        if me.client_key.is_some() {
            return Ok(json!({ "ok": true, "already": true, "keysLoaded": true }));
        }
        if !me.store.has_pin() {
            return Err(anyhow!(
                "No PIN configured. Open Settings → Security → Set PIN first."
            ));
        }
    }

    let store = engine.lock().expect("engine lock").store.clone();
    let (client_key, keys_loaded) = match store.unlock(pin.as_str())? {
        Unlock::Unlocked(Some(keys)) => {
            let client = engine.lock().expect("engine lock").client.clone();
            Engine::run_blocking(|rt| {
                rt.block_on(client.upload_server_key(&keys.server_key))
                    .context("uploading server key after unlock")?;
                Ok(())
            })?;
            (Some(Arc::new(keys.client_key)), true)
        }
        Unlock::Unlocked(None) => (None, false),
        Unlock::Wrong { attempts_left } => {
            return Err(anyhow!(
                "Wrong PIN. {attempts_left} attempt(s) left before a temporary lockout."
            ))
        }
        Unlock::Locked { retry_after_secs } => {
            return Err(anyhow!(
                "Too many failed attempts. Try again in {retry_after_secs} s."
            ))
        }
        Unlock::NotConfigured => {
            return Err(anyhow!(
                "No PIN configured. Open Settings → Security → Set PIN first."
            ))
        }
    };

    {
        let mut me = engine.lock().expect("engine lock");
        me.client_key = client_key;
        me.current_pin = Some(pin);
    }
    Ok(json!({ "ok": true, "keysLoaded": keys_loaded }))
}

/// `lock.lock` — drop the in-memory key material and return to the lock
/// screen. The encrypted store stays on disk.
fn lock_lock(engine: &SharedEngine) -> anyhow::Result<Value> {
    let mut me = engine.lock().expect("engine lock");
    me.client_key = None;
    me.current_pin = None;
    Ok(json!({ "locked": true }))
}

/// `backup.export {path?}` — write a PIN-protected backup. The key material
/// stays encrypted, but the file is copyable and the PIN hash is inside it,
/// so a short PIN is guessable offline. Defaults to `backup.json` in the
/// store directory.
fn backup_export(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    let path = req
        .get("path")
        .and_then(Value::as_str)
        .filter(|p| !p.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| me.store.dir().join("backup.json"));
    me.store.export_backup(&path)?;
    Ok(json!({ "path": path.display().to_string() }))
}

/// `backup.import {path}` — restore a backup, replacing the current PIN and
/// keys, then lock so the user unlocks with the imported PIN.
fn backup_import(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let path = required_str(req, "path")?;
    let mut me = engine.lock().expect("engine lock");
    me.store.import_backup(&path)?;
    // Drop any in-memory state: the imported PIN now governs the phone.
    me.client_key = None;
    me.current_pin = None;
    Ok(json!({ "ok": true, "path": path }))
}

/// `vault.destroy {pin}` — authenticate, then erase the local key store.
///
/// This is crypto-shredding of the local vault: without the key pair, the
/// ciphertexts on the embedded server are undecryptable. Exported backups and
/// OS copies are not removed and the UI says so.
fn vault_destroy(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let pin = Zeroizing::new(required_str(req, "pin")?);
    let mut me = engine.lock().expect("engine lock");
    match me.store.check_pin(pin.as_str())? {
        PinCheck::Ok => {}
        PinCheck::Wrong { attempts_left } => {
            return Err(anyhow!(
                "Wrong PIN. {attempts_left} attempt(s) left before a temporary lockout."
            ))
        }
        PinCheck::Locked { retry_after_secs } => {
            return Err(anyhow!(
                "Too many failed attempts. Try again in {retry_after_secs} s."
            ))
        }
        PinCheck::NotConfigured => {
            return Err(anyhow!(
                "No PIN is configured; there is nothing to destroy."
            ))
        }
    }
    me.store.destroy()?;
    me.client_key = None;
    me.current_pin = None;
    me.model_installed = false;
    Ok(json!({
        "ok": true,
        "note": "Local key store erased. Existing ciphertexts can no longer be decrypted; exported backups are not deleted."
    }))
}

/// `vault.info` — key state plus the security invariant note.
fn vault_info(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    Ok(json!({
        "keys": me.client_key.is_some(),
        "note": VAULT_NOTE,
    }))
}

/// `vault.reveal` — decrypt a column's ciphertexts on-device via the
/// read-back route (`GET /tables/{t}/ciphertexts/{c}`) and show that the
/// server returned only encrypted envelopes.
fn vault_reveal(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let table = required_str(req, "table")?;
    let column = required_str(req, "column")?;
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let ciphertexts = rt
            .block_on(client.get_ciphertexts(&table, &column))
            .with_context(|| format!("read-back of `{table}.{column}`"))?;
        let values: Vec<u64> = ciphertexts
            .iter()
            .map(|ct| ct.decrypt(&ck).as_u64())
            .collect();
        // Serialize each ciphertext to preview its envelope size. Errors here
        // are corrupt-database conditions, not wire input; still propagate
        // them instead of panicking.
        let mut envelope_previews = Vec::with_capacity(ciphertexts.len());
        for ct in &ciphertexts {
            let bytes = ct.to_envelope_bytes()?;
            envelope_previews.push(format!(
                "{} bytes · width {}-bit",
                bytes.len(),
                ct.width().bits()
            ));
        }
        Ok(json!({
            "table": table,
            "column": column,
            "count": values.len(),
            "values": values,
            "envelopes": envelope_previews,
        }))
    })
}

/// `tables.list` — list real table schemas on the embedded server.
fn tables_list(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let tables = rt.block_on(client.list_tables())?;
        Ok(json!({ "tables": tables, "note": DB_NOTE }))
    })
}

// ---------------------------------------------------------------------------
// Wallet — homomorphic ledger over U16
// ---------------------------------------------------------------------------

const WALLET_TABLE: &str = "wallet";
const WALLET_COL: &str = "balance";
const WALLET_WIDTH: ScalarWidth = ScalarWidth::U16;

/// `wallet.init` — create the encrypted ledger (idempotent).
fn wallet_init(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let schema = TableSchemaJson {
            name: WALLET_TABLE.into(),
            columns: vec![ColumnDefJson {
                name: WALLET_COL.into(),
                column_type: ColumnTypeJson::U16,
            }],
        };
        if rt
            .block_on(client.list_tables())?
            .iter()
            .any(|t| t.name == WALLET_TABLE)
        {
            return Ok(json!({ "status": "exists" }));
        }
        rt.block_on(client.create_table(&schema))?;
        // Seed the ledger with a private encrypted balance: 1000.
        let encryptor = Encryptor::new(ck.as_ref());
        let ct = encryptor.encrypt_u64(1000, WALLET_WIDTH)?;
        let inserted =
            rt.block_on(client.insert_ciphertexts(WALLET_TABLE, WALLET_COL, vec![ct]))?;
        Ok(json!({ "status": "created", "inserted": inserted }))
    })
}

/// Decrypt the homomorphic sum of the ledger (interpreted as signed i16).
fn wallet_balance(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let (sum_ct, count) = rt.block_on(client.sum(WALLET_TABLE, WALLET_COL))?;
        let raw = sum_ct.decrypt(&ck).as_u64() as u16;
        let signed = raw as i16;
        Ok(json!({
            "balance": signed,
            "raw": raw,
            "rows": count,
        }))
    })
}

/// `wallet.deposit {amount}` — encrypt and insert a positive amount.
fn wallet_deposit(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let amount = required_i64(req, "amount")?;
    if amount <= 0 {
        return Err(anyhow!("deposit must be positive"));
    }
    insert_wallet_amount(engine, amount as u64, "deposit")
}

/// `wallet.withdraw {amount}` — insert the two's-complement negation so the
/// homomorphic sum wraps correctly (U16 ring arithmetic).
fn wallet_withdraw(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let amount = required_i64(req, "amount")?;
    if amount <= 0 {
        return Err(anyhow!("withdraw must be positive"));
    }
    let encoded = (0u16.wrapping_sub(amount as u16)) as u64;
    insert_wallet_amount(engine, encoded, "withdraw")
}

/// Shared insert for the wallet ledger with a friendly action label.
fn insert_wallet_amount(
    engine: &SharedEngine,
    encoded: u64,
    action: &str,
) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let encryptor = Encryptor::new(ck.as_ref());
        let ct = encryptor.encrypt_u64(encoded, WALLET_WIDTH)?;
        let inserted =
            rt.block_on(client.insert_ciphertexts(WALLET_TABLE, WALLET_COL, vec![ct]))?;
        let (sum_ct, count) = rt.block_on(client.sum(WALLET_TABLE, WALLET_COL))?;
        let raw = sum_ct.decrypt(&ck).as_u64() as u16;
        let signed = raw as i16;
        Ok(json!({
            "action": action,
            "inserted": inserted,
            "balance_after": signed,
            "rows": count,
        }))
    })
}

// ---------------------------------------------------------------------------
// Health — encrypted vitals over U16
// ---------------------------------------------------------------------------

const HEALTH_TABLE: &str = "health";
const HEALTH_COL: &str = "bpm";
const HEALTH_WIDTH: ScalarWidth = ScalarWidth::U16;

/// `health.init {bpm}` — create the vitals table and record the first
/// encrypted heartbeat.
fn health_init(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let bpm = req.get("bpm").and_then(Value::as_u64).unwrap_or(72);
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let schema = TableSchemaJson {
            name: HEALTH_TABLE.into(),
            columns: vec![ColumnDefJson {
                name: HEALTH_COL.into(),
                column_type: ColumnTypeJson::U16,
            }],
        };
        let tables = rt.block_on(client.list_tables())?;
        if !tables.iter().any(|t| t.name == HEALTH_TABLE) {
            rt.block_on(client.create_table(&schema))?;
        }
        let encryptor = Encryptor::new(ck.as_ref());
        let ct = encryptor.encrypt_u64(bpm, HEALTH_WIDTH)?;
        let inserted =
            rt.block_on(client.insert_ciphertexts(HEALTH_TABLE, HEALTH_COL, vec![ct]))?;
        let count = rt.block_on(client.count(HEALTH_TABLE, HEALTH_COL))?;
        Ok(json!({ "inserted": inserted, "count": count }))
    })
}

/// `health.add {bpm}` — encrypt and record one heartbeat.
fn health_add(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let bpm = req.get("bpm").and_then(Value::as_u64).unwrap_or(72);
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let encryptor = Encryptor::new(ck.as_ref());
        let ct = encryptor.encrypt_u64(bpm, HEALTH_WIDTH)?;
        let inserted =
            rt.block_on(client.insert_ciphertexts(HEALTH_TABLE, HEALTH_COL, vec![ct]))?;
        Ok(json!({ "inserted": inserted, "bpm": bpm }))
    })
}

/// `health.stats` — homomorphic AVG + COUNT over the encrypted vitals.
fn health_stats(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let count = rt.block_on(client.count(HEALTH_TABLE, HEALTH_COL))?;
        let (sum_ct, _) = rt.block_on(client.avg(HEALTH_TABLE, HEALTH_COL))?;
        let sum = sum_ct.decrypt(&ck).as_u64();
        let avg = if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        };
        // Read back the raw ciphertexts too (the Database truth view).
        let ciphertexts = rt.block_on(client.get_ciphertexts(HEALTH_TABLE, HEALTH_COL))?;
        let values: Vec<u64> = ciphertexts
            .iter()
            .map(|ct| ct.decrypt(&ck).as_u64())
            .collect();
        Ok(json!({
            "count": count,
            "sum": sum,
            "avg": (avg * 10.0).round() / 10.0,
            "values": values,
        }))
    })
}

// ---------------------------------------------------------------------------
// Messages — encrypted notes (each UTF-8 char is one encrypted row)
// ---------------------------------------------------------------------------

const MESSAGES_TABLE: &str = "messages";
const MESSAGES_COL: &str = "payload";
const MESSAGES_WIDTH: ScalarWidth = ScalarWidth::U16;

/// `messages.send {text}` — encrypt each UTF-16 char code as a U16 row,
/// followed by a NUL separator row so `messages.list` can split the stream
/// back into individual messages.
fn messages_send(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let text = required_str(req, "text")?;
    if text.is_empty() {
        return Err(anyhow!("message must not be empty"));
    }
    let chars: Vec<u16> = text.encode_utf16().collect();
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let schema = TableSchemaJson {
            name: MESSAGES_TABLE.into(),
            columns: vec![ColumnDefJson {
                name: MESSAGES_COL.into(),
                column_type: ColumnTypeJson::U16,
            }],
        };
        let tables = rt.block_on(client.list_tables())?;
        if !tables.iter().any(|t| t.name == MESSAGES_TABLE) {
            rt.block_on(client.create_table(&schema))?;
        }
        let encryptor = Encryptor::new(ck.as_ref());
        // One row per char code plus a NUL terminator that delimits the
        // message boundary for the split in `messages_list`.
        let mut rows = Vec::with_capacity(chars.len() + 1);
        for code in chars {
            rows.push(encryptor.encrypt_u64(code as u64, MESSAGES_WIDTH)?);
        }
        rows.push(encryptor.encrypt_u64(0, MESSAGES_WIDTH)?);
        let inserted =
            rt.block_on(client.insert_ciphertexts(MESSAGES_TABLE, MESSAGES_COL, rows))?;
        Ok(json!({ "inserted": inserted, "chars": text.chars().count() }))
    })
}

/// `messages.list` — read back the raw ciphertexts and decrypt locally,
/// splitting on NUL separators into individual messages.
fn messages_list(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    // Even without a key, allow listing the envelope metadata.
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        let tables = rt.block_on(client.list_tables())?;
        if !tables.iter().any(|t| t.name == MESSAGES_TABLE) {
            return Ok(json!({ "messages": [], "count": 0 }));
        }
        let ciphertexts = rt.block_on(client.get_ciphertexts(MESSAGES_TABLE, MESSAGES_COL))?;
        let messages: Vec<String> = match me.key() {
            Ok(ck) => {
                let decoded: String = ciphertexts
                    .iter()
                    .map(|ct| {
                        let raw = ct.decrypt(&ck).as_u64() as u16;
                        char::from_u32(raw as u32).unwrap_or('\u{FFFD}')
                    })
                    .collect();
                decoded
                    .split('\u{0}')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            }
            Err(_) => vec![],
        };
        Ok(json!({
            "messages": messages,
            "count": ciphertexts.len(),
            "envelope_bytes": ciphertexts.iter().map(|ct| ct.to_envelope_bytes().map(|b| b.len()).unwrap_or(0)).collect::<Vec<_>>(),
        }))
    })
}

// ---------------------------------------------------------------------------
// Neural — encrypted XNOR inference over HTTP
// ---------------------------------------------------------------------------

const XNOR_FIXED: FixedPoint = FixedPoint {
    width: ScalarWidth::U16,
    frac_bits: 0,
};

/// `neural.install` — encrypt the XNOR weights (w0=1, w1=1, bias=-1), upload,
/// and verify the full truth table through the real server.
fn neural_install(engine: &SharedEngine) -> anyhow::Result<Value> {
    // Fast path: if the model is already installed, don't re-upload it.
    let (ck, client, already_installed) = {
        let me = engine.lock().expect("engine lock");
        (me.key()?, me.client.clone(), me.model_installed)
    };

    Engine::run_blocking(|rt| {
        if !already_installed {
            let encryptor = Encryptor::new(ck.as_ref());
            let w0 = encryptor.encrypt_u64(XNOR_FIXED.encode(1.0)?, XNOR_FIXED.width)?;
            let w1 = encryptor.encrypt_u64(XNOR_FIXED.encode(1.0)?, XNOR_FIXED.width)?;
            rt.block_on(client.install_xnor_model(vec![w0, w1], vec![-1]))?;
            // Mark installed (single lock, no nesting).
            {
                let mut me = engine.lock().expect("engine lock");
                me.model_installed = true;
            }
        }
        // Verify the full truth table through the real HTTP inference route.
        let table = run_xnor_truth_table(&ck, &client, rt)?;
        Ok(json!({
            "installed": true,
            "note": "Weights w0=1, w1=1 are ciphertexts on the server; the bias -1 is a public \
                     constant. The server computed h=(x0+x1-1)^2 over ciphertexts only.",
            "truth_table": table,
        }))
    })
}

/// `neural.infer {x0, x1}` — encrypted inference for one input pair.
fn neural_infer(engine: &SharedEngine, req: &Value) -> anyhow::Result<Value> {
    let x0 = req.get("x0").and_then(Value::as_f64).unwrap_or(0.0);
    let x1 = req.get("x1").and_then(Value::as_f64).unwrap_or(0.0);
    let me = engine.lock().expect("engine lock");
    let ck = me.key()?;
    let client = me.client.clone();
    Engine::run_blocking(|rt| {
        if !me.model_installed {
            return Err(anyhow!(
                "Model not installed. Run Neural > Install Model first."
            ));
        }
        let encryptor = Encryptor::new(ck.as_ref());
        let ct0 = encryptor.encrypt_u64(XNOR_FIXED.encode(x0)?, XNOR_FIXED.width)?;
        let ct1 = encryptor.encrypt_u64(XNOR_FIXED.encode(x1)?, XNOR_FIXED.width)?;
        let out = rt.block_on(client.infer_xnor((ct0, ct1)))?;
        let decoded = XNOR_FIXED.decode(out.decrypt(ck.as_ref()).as_u64());
        let predicted = (decoded + 0.5).floor() as i64;
        Ok(json!({
            "x0": x0,
            "x1": x1,
            "h": (decoded * 1000.0).round() / 1000.0,
            "predicted": predicted,
        }))
    })
}

/// Run the four XNOR truth-table rows through the real HTTP inference route.
fn run_xnor_truth_table(
    ck: &Arc<ClientKey>,
    client: &FheClient,
    rt: &tokio::runtime::Runtime,
) -> anyhow::Result<Vec<Value>> {
    let encryptor = Encryptor::new(ck.as_ref());
    let mut rows = Vec::new();
    for (x0, x1) in [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)] {
        let ct0 = encryptor.encrypt_u64(XNOR_FIXED.encode(x0)?, XNOR_FIXED.width)?;
        let ct1 = encryptor.encrypt_u64(XNOR_FIXED.encode(x1)?, XNOR_FIXED.width)?;
        let out = rt.block_on(client.infer_xnor((ct0, ct1)))?;
        let decoded = XNOR_FIXED.decode(out.decrypt(ck.as_ref()).as_u64());
        let predicted = (decoded + 0.5).floor() as i64;
        rows.push(json!({
            "x0": x0 as i64,
            "x1": x1 as i64,
            "h": (decoded * 1000.0).round() / 1000.0,
            "predicted": predicted,
        }));
    }
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Monitor
// ---------------------------------------------------------------------------

/// `monitor.snapshot` — the live wire feed captured by the server middleware.
fn monitor_snapshot(engine: &SharedEngine) -> anyhow::Result<Value> {
    let me = engine.lock().expect("engine lock");
    Ok(json!({
        "records": me.wire.snapshot(),
        "count": me.wire.len(),
        "note": "Every entry is a real HTTP request/response the embedded fhe-server \
                 observed. The tap records method, path, and status only — not bodies — \
                 so this is a traffic view, not proof about payload contents.",
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn required_str(req: &Value, key: &str) -> anyhow::Result<String> {
    req.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("missing string parameter `{key}`"))
}

fn required_i64(req: &Value, key: &str) -> anyhow::Result<i64> {
    req.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("missing number parameter `{key}`"))
}

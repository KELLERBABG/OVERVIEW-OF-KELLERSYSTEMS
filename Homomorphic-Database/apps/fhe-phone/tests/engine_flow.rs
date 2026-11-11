//! Headless end-to-end test of the phone's FHE engine.
//!
//! Boots the real embedded fhe-server, binds the real fhe-client SDK, and
//! drives every phone app through the same `dispatch` IPC entrypoint the
//! webview uses — no GUI required. This pins the full stack: keygen,
//! server-key upload, encrypted inserts, homomorphic SUM/AVG/COUNT,
//! encrypted read-back, and Phase 2 encrypted XNOR inference, all over the
//! loopback HTTP boundary with the wire tap recording every request.

use std::sync::{Arc, Mutex};

use fhe_phone::{dispatch, Engine, SharedEngine, WireRing};
use serde_json::{json, Value};

/// Call one engine op exactly as the UI would (`{op, id, ...params}`).
fn call(engine: &SharedEngine, op: &str, params: Value) -> Value {
    let mut req = json!({ "op": op, "id": 1 });
    if let (Value::Object(req_map), Value::Object(params)) = (&mut req, params) {
        req_map.extend(params);
    }
    dispatch(engine, &req)
}

fn ok(r: &Value) -> bool {
    r.get("ok").and_then(Value::as_bool).unwrap_or(false)
}

fn err(r: &Value) -> String {
    r.get("error")
        .and_then(Value::as_str)
        .unwrap_or("<no error field>")
        .to_string()
}

fn data(r: &Value) -> &Value {
    r.get("data").expect("response has data")
}

#[test]
fn full_phone_flow_over_embedded_server() {
    // The encrypted key store must live in a throwaway temp dir so the test
    // never touches (or collides with) real user data in %APPDATA%.
    let store_dir = std::env::temp_dir().join(format!("fhe_phone_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store_dir);
    let engine: SharedEngine = Arc::new(Mutex::new(
        Engine::start_with_store(WireRing::new(), Some(store_dir.clone()))
            .expect("embedded server must start"),
    ));

    // -- boot status: server healthy, no keys yet ---------------------------
    let r = call(&engine, "status", json!({}));
    assert!(ok(&r), "status failed: {}", err(&r));
    assert_eq!(
        data(&r)["healthy"],
        true,
        "embedded server must be reachable"
    );
    assert_eq!(data(&r)["keys"], false);

    // -- lock: configure the PIN, then keygen persists keys at rest --------
    let r = call(&engine, "lock.setpin", json!({ "pin": "123456" }));
    assert!(ok(&r), "lock.setpin failed: {}", err(&r));
    let r = call(&engine, "lock.status", json!({}));
    assert_eq!(data(&r)["pinSet"], true);
    // The phone is on the lock screen; unlock (no keys yet) to enter.
    let r = call(&engine, "lock.unlock", json!({ "pin": "123456" }));
    assert!(ok(&r), "lock.unlock failed: {}", err(&r));
    assert_eq!(data(&r)["keysLoaded"], false, "no keys stored yet");

    // -- vault: keygen + upload server key ----------------------------------
    let r = call(&engine, "keygen", json!({}));
    assert!(ok(&r), "keygen failed: {}", err(&r));
    assert_eq!(data(&r)["already"], false);
    assert_eq!(
        data(&r)["stored"],
        "encrypted at rest under your PIN (AES-256-GCM, Argon2id)"
    );
    let r = call(&engine, "vault.info", json!({}));
    assert_eq!(data(&r)["keys"], true);
    let r = call(&engine, "status", json!({}));
    assert_eq!(data(&r)["keys"], true);

    // -- lock/unlock roundtrip: keys survive from the encrypted store ------
    let r = call(&engine, "lock.lock", json!({}));
    assert!(ok(&r), "lock.lock failed: {}", err(&r));
    let r = call(&engine, "status", json!({}));
    assert_eq!(data(&r)["keys"], false, "locked drops in-memory keys");
    let r = call(&engine, "lock.unlock", json!({ "pin": "123456" }));
    assert!(ok(&r), "lock.unlock failed: {}", err(&r));
    assert_eq!(
        data(&r)["keysLoaded"],
        true,
        "stored keys restored on unlock"
    );
    let r = call(&engine, "status", json!({}));
    assert_eq!(data(&r)["keys"], true);
    // Wrong PIN is rejected.
    let r = call(&engine, "lock.lock", json!({}));
    assert!(ok(&r));
    let r = call(&engine, "lock.unlock", json!({ "pin": "9999" }));
    assert!(!ok(&r), "wrong PIN must be rejected");
    // Back to the correct PIN so the rest of the flow has keys.
    let r = call(&engine, "lock.unlock", json!({ "pin": "123456" }));
    assert!(ok(&r), "re-unlock failed: {}", err(&r));

    // -- wallet: homomorphic ledger -----------------------------------------
    let r = call(&engine, "wallet.init", json!({}));
    assert!(ok(&r), "wallet.init failed: {}", err(&r));
    let r = call(&engine, "wallet.balance", json!({}));
    assert!(ok(&r), "wallet.balance failed: {}", err(&r));
    assert_eq!(data(&r)["balance"], 1000, "seeded balance is 1000");
    assert_eq!(data(&r)["rows"], 1);

    let r = call(&engine, "wallet.deposit", json!({ "amount": 25 }));
    assert!(ok(&r), "deposit failed: {}", err(&r));
    assert_eq!(data(&r)["balance_after"], 1025);

    let r = call(&engine, "wallet.withdraw", json!({ "amount": 10 }));
    assert!(ok(&r), "withdraw failed: {}", err(&r));
    assert_eq!(data(&r)["balance_after"], 1015);

    let r = call(&engine, "wallet.balance", json!({}));
    assert_eq!(data(&r)["balance"], 1015);
    assert_eq!(data(&r)["rows"], 3, "seed + deposit + withdraw");

    // -- health: homomorphic AVG over encrypted vitals ----------------------
    let r = call(&engine, "health.init", json!({ "bpm": 72 }));
    assert!(ok(&r), "health.init failed: {}", err(&r));
    let r = call(&engine, "health.add", json!({ "bpm": 80 }));
    assert!(ok(&r), "health.add failed: {}", err(&r));
    let r = call(&engine, "health.stats", json!({}));
    assert!(ok(&r), "health.stats failed: {}", err(&r));
    assert_eq!(data(&r)["count"], 2);
    assert_eq!(data(&r)["sum"], 152);
    let avg = data(&r)["avg"].as_f64().expect("avg is a number");
    assert!((avg - 76.0).abs() < 1e-9, "avg = {avg}");

    // -- messages: encrypted notes ------------------------------------------
    let r = call(&engine, "messages.send", json!({ "text": "hello" }));
    assert!(ok(&r), "messages.send failed: {}", err(&r));
    assert_eq!(data(&r)["inserted"], 6, "5 chars + 1 NUL separator");
    let r = call(&engine, "messages.send", json!({ "text": "world" }));
    assert!(ok(&r), "messages.send #2 failed: {}", err(&r));
    let r = call(&engine, "messages.list", json!({}));
    assert!(ok(&r), "messages.list failed: {}", err(&r));
    assert_eq!(data(&r)["messages"], json!(["hello", "world"]));

    // -- vault.reveal: read-back + on-device decryption ----------------------
    let r = call(
        &engine,
        "vault.reveal",
        json!({ "table": "health", "column": "bpm" }),
    );
    assert!(ok(&r), "vault.reveal failed: {}", err(&r));
    assert_eq!(data(&r)["values"], json!([72, 80]));

    // -- neural: encrypted XNOR model over HTTP -----------------------------
    let r = call(&engine, "neural.install", json!({}));
    assert!(ok(&r), "neural.install failed: {}", err(&r));
    let truth = data(&r)["truth_table"]
        .as_array()
        .expect("truth table is an array");
    let predicted: Vec<i64> = truth
        .iter()
        .map(|row| row["predicted"].as_i64().expect("predicted is an int"))
        .collect();
    assert_eq!(predicted, vec![1, 0, 0, 1], "XNOR truth table");

    let r = call(&engine, "neural.infer", json!({ "x0": 1.0, "x1": 1.0 }));
    assert!(ok(&r), "neural.infer failed: {}", err(&r));
    assert_eq!(data(&r)["predicted"], 1);
    let r = call(&engine, "neural.infer", json!({ "x0": 1.0, "x1": 0.0 }));
    assert_eq!(data(&r)["predicted"], 0);

    // -- tables.list --------------------------------------------------------
    let r = call(&engine, "tables.list", json!({}));
    assert!(ok(&r), "tables.list failed: {}", err(&r));
    let names: Vec<String> = data(&r)["tables"]
        .as_array()
        .expect("tables is an array")
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default().to_string())
        .collect();
    for expected in ["wallet", "health", "messages"] {
        assert!(
            names.iter().any(|n| n == expected),
            "table {expected} present in {names:?}"
        );
    }

    // -- monitor: the wire tap saw real HTTP traffic ------------------------
    let r = call(&engine, "monitor.snapshot", json!({}));
    assert!(ok(&r), "monitor.snapshot failed: {}", err(&r));
    let count = data(&r)["count"].as_u64().expect("count is a number");
    assert!(
        count > 0,
        "wire ring recorded HTTP traffic ({count} records)"
    );
    let records = data(&r)["records"].as_array().expect("records array");
    let has_req = records
        .iter()
        .any(|rec| rec["direction"].as_str() == Some("req"));
    let has_res = records
        .iter()
        .any(|rec| rec["direction"].as_str() == Some("res"));
    assert!(
        has_req && has_res,
        "wire feed has both requests and responses"
    );

    // -- PIN change and destruction over the real IPC boundary --------------
    let r = call(
        &engine,
        "lock.changepin",
        json!({ "old": "123456", "new": "654321" }),
    );
    assert!(ok(&r), "lock.changepin failed: {}", err(&r));
    let r = call(&engine, "lock.lock", json!({}));
    assert!(ok(&r), "lock.lock failed: {}", err(&r));
    let r = call(&engine, "lock.unlock", json!({ "pin": "123456" }));
    assert!(!ok(&r), "the old PIN must not unlock after a change");
    let r = call(&engine, "lock.unlock", json!({ "pin": "654321" }));
    assert!(ok(&r), "new PIN must unlock: {}", err(&r));

    // Wrong PIN cannot destroy; the correct one erases the local store.
    let r = call(&engine, "vault.destroy", json!({ "pin": "000000" }));
    assert!(!ok(&r), "destroy must require the PIN");
    let r = call(&engine, "vault.destroy", json!({ "pin": "654321" }));
    assert!(ok(&r), "vault.destroy failed: {}", err(&r));
    let r = call(&engine, "lock.status", json!({}));
    assert_eq!(data(&r)["pinSet"], false, "destroy removes the PIN");
    assert_eq!(data(&r)["keyStored"], false, "destroy removes the keys");

    let _ = std::fs::remove_dir_all(&store_dir);
}

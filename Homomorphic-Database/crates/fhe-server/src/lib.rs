//! # fhe-server — Encrypted Database REST API
//!
//! An axum 0.8 HTTP server that exposes the [`fhe_db`] engine as a zero-trust
//! REST API. Clients upload a TFHE [`ServerKey`], push encrypted columns as
//! base64 [`CiphertextEnvelope`]s, and run homomorphic SUM / AVG / COUNT
//! aggregations. The server never sees value plaintext; schema metadata and
//! the XNOR bias are outside that guarantee.
//!
//! ## Layout
//!
//! - [`routes`] — all HTTP handlers and shared [`routes::AppState`].
//! - [`error`] — [`ServerError`], [`ServerResult`]: typed HTTP errors mapped
//!   to JSON responses.
//!
//! ## Security Notes
//!
//! - The server key is installed once via `POST /keys/server` and pinned per
//!   request with [`fhe_core::ServerKeyGuard`].
//! - Handlers never unwrap on wire data; every envelope is validated with
//!   [`fhe_core::Ciphertext::from_envelope_bytes`].

pub mod error;
pub mod routes;

pub use error::{ServerError, ServerResult};
pub use routes::{build_router, AppState};

/// Build the fully-wired axum router: CORS, request tracing, and all routes.
///
/// Installs tower-http request tracing and permissive CORS (the database is a
/// local/trusted-network service; tighten for public deployments). Tracing's
/// default events are `DEBUG`; the CLI's default `info,fhe_server=debug` filter
/// does not emit per-request logs, so enable `tower_http=debug` when needed.
pub fn app(state: AppState) -> axum::Router {
    use tower_http::cors::CorsLayer;
    use tower_http::trace::TraceLayer;

    build_router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Method, Request, StatusCode};
    use base64::Engine;
    use fhe_core::{ClientKey, Encryptor, ScalarWidth};
    use std::net::SocketAddr;
    use tower::ServiceExt;

    const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;
    const JSON_TYPE: &str = "application/json";
    const OCTET_TYPE: &str = "application/octet-stream";

    /// Send a request with an explicit `Content-Type`. axum 0.8 extractors
    /// reject mismatched/missing content types (415), so tests must mirror a
    /// real HTTP client, which sets `Content-Type` automatically.
    async fn request(
        router: axum::Router,
        method: Method,
        uri: &str,
        content_type: &str,
        body: Body,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(CONTENT_TYPE, content_type)
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn json_request(
        router: axum::Router,
        method: Method,
        uri: &str,
        body: &str,
    ) -> axum::response::Response {
        request(router, method, uri, JSON_TYPE, Body::from(body.to_string())).await
    }

    /// Send a request carrying an explicit peer address, mirroring the
    /// `ConnectInfo` extension a real listener installs for every connection.
    async fn request_from_peer(
        router: axum::Router,
        uri: &str,
        peer: SocketAddr,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(uri)
                    .header(CONTENT_TYPE, JSON_TYPE)
                    .extension(ConnectInfo(peer))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Send a request with `Authorization: Bearer <token>`.
    async fn authenticated_request(
        router: axum::Router,
        method: Method,
        uri: &str,
        token: &str,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(CONTENT_TYPE, JSON_TYPE)
                    .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let router = build_router(AppState::new());
        let response = request(router, Method::GET, "/health", JSON_TYPE, Body::empty()).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_and_list_tables() {
        let router = build_router(AppState::new());

        // Create a table.
        let create_body =
            r#"{"name":"payments","columns":[{"name":"amount","column_type":"U32"}]}"#;
        let response = json_request(router.clone(), Method::POST, "/tables", create_body).await;
        assert_eq!(response.status(), StatusCode::OK);

        // List tables.
        let response = request(router, Method::GET, "/tables", JSON_TYPE, Body::empty()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["name"], "payments");
    }

    #[tokio::test]
    async fn aggregation_without_server_key_conflicts() {
        let router = build_router(AppState::new());
        let body = r#"{"name":"t","columns":[]}"#;
        let response = json_request(router.clone(), Method::POST, "/tables", body).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = json_request(
            router,
            Method::POST,
            "/tables/t/aggregations/sum",
            r#"{"column":"nonexistent"}"#,
        )
        .await;
        // No server key installed yet → conflict takes precedence.
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn full_flow_sum_decrypts_to_expected_value() {
        let (client_key, server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        let state = AppState::new();

        // Install the server key.
        let server_key_bytes = server_key.to_bytes().unwrap();
        let key_response = request(
            build_router(state.clone()),
            Method::POST,
            "/keys/server",
            OCTET_TYPE,
            Body::from(server_key_bytes),
        )
        .await;
        assert_eq!(key_response.status(), StatusCode::NO_CONTENT);

        let router = build_router(state.clone());

        // Create a table.
        let create_body =
            r#"{"name":"salaries","columns":[{"name":"amount","column_type":"U32"}]}"#;
        let response = json_request(router.clone(), Method::POST, "/tables", create_body).await;
        assert_eq!(response.status(), StatusCode::OK);

        // Insert three encrypted rows.
        let cts: Vec<String> = [100u64, 200, 300]
            .iter()
            .map(|v| {
                let ct = encryptor.encrypt_u64(*v, ScalarWidth::U32).unwrap();
                B64.encode(ct.to_envelope_bytes().unwrap())
            })
            .collect();
        let insert_body = serde_json::json!({
            "column": "amount",
            "ciphertexts": cts,
        })
        .to_string();
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/salaries/rows",
            &insert_body,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        // SUM.
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/salaries/aggregations/sum",
            r#"{"column":"amount"}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["count"], 3);
        let sum_bytes = B64.decode(json["sum"].as_str().unwrap()).unwrap();
        let sum_ct = fhe_core::Ciphertext::from_envelope_bytes(&sum_bytes).unwrap();
        assert_eq!(sum_ct.decrypt(&client_key).as_u64(), 600);

        // AVG (client divides by public count).
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/salaries/aggregations/avg",
            r#"{"column":"amount"}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let avg_bytes = B64.decode(json["sum"].as_str().unwrap()).unwrap();
        let avg_ct = fhe_core::Ciphertext::from_envelope_bytes(&avg_bytes).unwrap();
        let total = avg_ct.decrypt(&client_key).as_u64();
        assert_eq!(total / json["count"].as_u64().unwrap(), 200);

        // COUNT.
        let response = json_request(
            router,
            Method::POST,
            "/tables/salaries/aggregations/count",
            r#"{"column":"amount"}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["count"], 3);
    }

    #[tokio::test]
    async fn encrypted_xnor_inference_over_http() {
        use fhe_core::nn::FixedPoint;
        let (client_key, server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        let state = AppState::new();

        // Install the server key.
        let key_bytes = server_key.to_bytes().unwrap();
        let _ = request(
            build_router(state.clone()),
            Method::POST,
            "/keys/server",
            OCTET_TYPE,
            Body::from(key_bytes),
        )
        .await;

        let router = build_router(state.clone());

        // Fixed-point format: Q15.0 over 16-bit storage (matches the server's
        // XNOR_FIXED).
        let fixed = FixedPoint::new(ScalarWidth::U16, 0).unwrap();
        let encrypt = |v: f64| {
            let encoded = fixed.encode(v).unwrap();
            encryptor.encrypt_u64(encoded, ScalarWidth::U16).unwrap()
        };

        // Install the encrypted model: w0=1, w1=1, bias=-1.
        let w0 = encrypt(1.0);
        let w1 = encrypt(1.0);
        let install_body = serde_json::json!({
            "weights": [
                B64.encode(w0.to_envelope_bytes().unwrap()),
                B64.encode(w1.to_envelope_bytes().unwrap()),
            ],
            "bias": [-1i64],
        })
        .to_string();
        let response =
            json_request(router.clone(), Method::POST, "/models/xnor", &install_body).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Run XNOR over the full truth table.
        let cases: [(f64, f64, f64); 4] = [
            (0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0),
            (1.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
        ];
        for (x0, x1, expected) in cases {
            let infer_body = serde_json::json!({
                "inputs": [
                    B64.encode(encrypt(x0).to_envelope_bytes().unwrap()),
                    B64.encode(encrypt(x1).to_envelope_bytes().unwrap()),
                ],
            })
            .to_string();
            let response = json_request(
                router.clone(),
                Method::POST,
                "/models/xnor/infer",
                &infer_body,
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let out_bytes = B64.decode(json["output"].as_str().unwrap()).unwrap();
            let out_ct = fhe_core::Ciphertext::from_envelope_bytes(&out_bytes).unwrap();
            let decoded = fixed.decode(out_ct.decrypt(&client_key).as_u64());
            let predicted = (decoded + 0.5).floor();
            assert!(
                (predicted - expected).abs() < 1e-6,
                "XNOR({x0},{x1}) -> {decoded} (predicted {predicted}, expected {expected})"
            );
        }
    }

    #[tokio::test]
    async fn ciphertext_readback_decrypts_to_original_values() {
        let (client_key, server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        let state = AppState::new();

        // Install the server key (not needed for read-back, but exercises
        // the full zero-trust flow).
        let key_bytes = server_key.to_bytes().unwrap();
        let _ = request(
            build_router(state.clone()),
            Method::POST,
            "/keys/server",
            OCTET_TYPE,
            Body::from(key_bytes),
        )
        .await;

        let router = build_router(state.clone());

        // Create a table and insert encrypted rows.
        let _ = json_request(
            router.clone(),
            Method::POST,
            "/tables",
            r#"{"name":"messages","columns":[{"name":"payload","column_type":"U32"}]}"#,
        )
        .await;
        let cts: Vec<String> = [42u64, 1337, 9001]
            .iter()
            .map(|v| {
                let ct = encryptor.encrypt_u64(*v, ScalarWidth::U32).unwrap();
                B64.encode(ct.to_envelope_bytes().unwrap())
            })
            .collect();
        let _ = json_request(
            router.clone(),
            Method::POST,
            "/tables/messages/rows",
            &serde_json::json!({"column":"payload","ciphertexts":cts}).to_string(),
        )
        .await;

        // Read the ciphertexts back over the new read-back route.
        let response = request(
            router,
            Method::GET,
            "/tables/messages/ciphertexts/payload",
            JSON_TYPE,
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["count"], 3);
        let envelopes = json["ciphertexts"].as_array().unwrap();
        let values: Vec<u64> = envelopes
            .iter()
            .map(|e| {
                let bytes = B64.decode(e.as_str().unwrap()).unwrap();
                fhe_core::Ciphertext::from_envelope_bytes(&bytes)
                    .unwrap()
                    .decrypt(&client_key)
                    .as_u64()
            })
            .collect();
        assert_eq!(values, vec![42, 1337, 9001]);
    }

    #[tokio::test]
    async fn malformed_envelope_rejected_with_400() {
        let (_client_key, server_key) = ClientKey::new();
        let state = AppState::new();

        let key_bytes = server_key.to_bytes().unwrap();
        let _ = request(
            build_router(state.clone()),
            Method::POST,
            "/keys/server",
            OCTET_TYPE,
            Body::from(key_bytes),
        )
        .await;

        let router = build_router(state);
        let _ = json_request(
            router.clone(),
            Method::POST,
            "/tables",
            r#"{"name":"t","columns":[{"name":"c","column_type":"U32"}]}"#,
        )
        .await;

        // Corrupt base64 → 400.
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/t/rows",
            &serde_json::json!({"column":"c","ciphertexts":["!!not-base64!!"]}).to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Valid base64 of garbage bytes → 400.
        let garbage = B64.encode(b"this is not an envelope");
        let response = json_request(
            router,
            Method::POST,
            "/tables/t/rows",
            &serde_json::json!({"column":"c","ciphertexts":[garbage]}).to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Decode a JSON error body into its `code` field.
    async fn error_code(response: axum::response::Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["code"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn auth_gates_everything_except_health() {
        let router = build_router(AppState::new().with_auth("s3cret-token"));

        // /health stays open for liveness probes (no secret needed).
        let response = request(
            router.clone(),
            Method::GET,
            "/health",
            JSON_TYPE,
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        // No token → 401 UNAUTHORIZED.
        let response = request(
            router.clone(),
            Method::GET,
            "/tables",
            JSON_TYPE,
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(error_code(response).await, "UNAUTHORIZED");

        // Wrong token → 401.
        let response = authenticated_request(router.clone(), Method::GET, "/tables", "wrong").await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(error_code(response).await, "UNAUTHORIZED");

        // Correct token → 200.
        let response =
            authenticated_request(router.clone(), Method::GET, "/tables", "s3cret-token").await;
        assert_eq!(response.status(), StatusCode::OK);

        // /metrics is gated too (Prometheus scrapers must hold the token).
        let response = request(
            router.clone(),
            Method::GET,
            "/metrics",
            JSON_TYPE,
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // POST bodies are gated the same way.
        let response = json_request(
            router,
            Method::POST,
            "/tables",
            r#"{"name":"t","columns":[]}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_is_optional_by_default() {
        // Backward compatibility: without a configured token every route is
        // open (the loopback-only posture).
        let router = build_router(AppState::new());
        let response = request(router, Method::GET, "/tables", JSON_TYPE, Body::empty()).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[cfg(feature = "tls")]
    async fn tls_serves_https_with_auth() {
        // End-to-end over a real rustls listener using the committed
        // test-only self-signed cert (see tests/tls/README.md). The client
        // accepts the self-signed cert explicitly — this is a test fixture,
        // never a production trust decision.
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let cert = manifest.join("tests/tls/localhost-cert.pem");
        let key = manifest.join("tests/tls/localhost-key.pem");
        let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key)
            .await
            .expect("load test TLS key pair");

        // Bind an ephemeral port so parallel test runs never collide.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let router = build_router(AppState::new().with_auth("tls-token"));
        let handle = axum_server::Handle::new();
        let server_handle = handle.clone();
        tokio::spawn(async move {
            axum_server::from_tcp_rustls(listener, config)
                .handle(server_handle)
                .serve(router)
                .await
                .expect("TLS server exited unexpectedly");
        });

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();

        // Liveness probe over TLS stays open without a token.
        let response = client
            .get(format!("https://{addr}/health"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Data routes require the bearer token over TLS too.
        let response = client
            .get(format!("https://{addr}/tables"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let response = client
            .get(format!("https://{addr}/tables"))
            .header(axum::http::header::AUTHORIZATION, "Bearer tls-token")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        handle.graceful_shutdown(None);
    }

    #[tokio::test]
    async fn error_bodies_carry_stable_codes() {
        let state = AppState::new();
        let router = build_router(state);

        // Unknown table → 404 UNKNOWN_TABLE (through DbError mapping).
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/nope/aggregations/count",
            r#"{"column":"x"}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(error_code(response).await, "UNKNOWN_TABLE");

        // Duplicate table → 409 TABLE_EXISTS.
        let body = r#"{"name":"t","columns":[{"name":"c","column_type":"U32"}]}"#;
        let _ = json_request(router.clone(), Method::POST, "/tables", body).await;
        let response = json_request(router.clone(), Method::POST, "/tables", body).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(error_code(response).await, "TABLE_EXISTS");

        // Duplicate column → 409 COLUMN_EXISTS.
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/t/columns",
            r#"{"name":"c","column_type":"U32"}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(error_code(response).await, "COLUMN_EXISTS");

        // Missing server key on an aggregation → 409 NO_SERVER_KEY.
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/t/aggregations/sum",
            r#"{"column":"c"}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(error_code(response).await, "NO_SERVER_KEY");

        // Corrupt envelope → 400 INVALID_ENVELOPE.
        let garbage = B64.encode(b"not an envelope");
        let response = json_request(
            router,
            Method::POST,
            "/tables/t/rows",
            &serde_json::json!({"column":"c","ciphertexts":[garbage]}).to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(response).await, "INVALID_ENVELOPE");
    }

    #[tokio::test]
    async fn insert_batch_cap_rejects_oversized_batches() {
        use crate::routes::Limits;
        let limits = Limits {
            max_insert_envelopes: 2,
            ..Limits::default()
        };
        let router = build_router(AppState::with_limits(limits));
        let _ = json_request(
            router.clone(),
            Method::POST,
            "/tables",
            r#"{"name":"t","columns":[{"name":"c","column_type":"U32"}]}"#,
        )
        .await;

        let response = json_request(
            router,
            Method::POST,
            "/tables/t/rows",
            &serde_json::json!({"column":"c","ciphertexts":["a","b","c"]}).to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(error_code(response).await, "PAYLOAD_TOO_LARGE");
    }

    #[tokio::test]
    async fn aggregation_row_cap_rejects_large_columns() {
        use crate::routes::Limits;
        let (client_key, _server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        let state = AppState::with_limits(Limits {
            max_aggregation_rows: 2,
            ..Limits::default()
        });
        let router = build_router(state);
        let _ = json_request(
            router.clone(),
            Method::POST,
            "/tables",
            r#"{"name":"t","columns":[{"name":"c","column_type":"U32"}]}"#,
        )
        .await;
        let cts: Vec<String> = (0..3)
            .map(|_| {
                let ct = encryptor.encrypt_u64(1, ScalarWidth::U32).unwrap();
                B64.encode(ct.to_envelope_bytes().unwrap())
            })
            .collect();
        let _ = json_request(
            router.clone(),
            Method::POST,
            "/tables/t/rows",
            &serde_json::json!({"column":"c","ciphertexts":cts}).to_string(),
        )
        .await;

        // COUNT over 3 rows with a cap of 2 → 413 (no server key needed).
        let response = json_request(
            router,
            Method::POST,
            "/tables/t/aggregations/count",
            r#"{"column":"c"}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(error_code(response).await, "PAYLOAD_TOO_LARGE");
    }

    #[tokio::test]
    async fn schema_caps_reject_oversized_schemas() {
        use crate::routes::Limits;
        let limits = Limits {
            max_table_name_len: 4,
            max_columns_per_table: 1,
            ..Limits::default()
        };
        let router = build_router(AppState::with_limits(limits));

        // Over-long table name → 400 SCHEMA_INVALID.
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables",
            r#"{"name":"toolong","columns":[]}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(response).await, "SCHEMA_INVALID");

        // Too many columns → 400 SCHEMA_INVALID.
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables",
            r#"{"name":"t","columns":[{"name":"a","column_type":"U32"},{"name":"b","column_type":"U32"}]}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(response).await, "SCHEMA_INVALID");
    }

    #[tokio::test]
    async fn oversized_json_body_rejected_with_413() {
        let router = build_router(AppState::new());
        // 3 MiB of padding — the JSON body cap is 2 MiB.
        let big = format!("{{\"name\":\"{}\"}}", "x".repeat(3 * 1024 * 1024));
        let response = json_request(router, Method::POST, "/tables", &big).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn oversized_server_key_reaches_handler_not_body_limit() {
        // /keys/server has its own 256 MiB cap; a 3 MiB body must reach the
        // handler (and be rejected as invalid key material, 400) rather than
        // tripping the 2 MiB JSON limit (413).
        let router = build_router(AppState::new());
        let junk = vec![0x42u8; 3 * 1024 * 1024];
        let response = request(
            router,
            Method::POST,
            "/keys/server",
            OCTET_TYPE,
            Body::from(junk),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rate_limiter_trips_after_quota() {
        use crate::routes::Limits;
        let limits = Limits {
            rate_quota: 3,
            ..Limits::default()
        };
        let router = build_router(AppState::with_limits(limits));

        for _ in 0..3 {
            let response = request(
                router.clone(),
                Method::GET,
                "/health",
                JSON_TYPE,
                Body::empty(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
        }
        let response = request(router, Method::GET, "/health", JSON_TYPE, Body::empty()).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(error_code(response).await, "RATE_LIMITED");
    }

    #[tokio::test]
    async fn rate_limiter_shares_one_bucket_per_ip_across_connections() {
        use crate::routes::Limits;
        let limits = Limits {
            rate_quota: 2,
            ..Limits::default()
        };
        let router = build_router(AppState::with_limits(limits));

        // Two separate TCP connections from the same IP: the second must see
        // the first request's count, even though its ephemeral port differs.
        for port in [40_001, 40_002] {
            let response = request_from_peer(
                router.clone(),
                "/health",
                SocketAddr::from(([127, 0, 0, 1], port)),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
        }
        let response = request_from_peer(
            router.clone(),
            "/health",
            SocketAddr::from(([127, 0, 0, 1], 40_003)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(error_code(response).await, "RATE_LIMITED");

        // The quota is per IP, not global: another peer still gets its own.
        let response = request_from_peer(
            router,
            "/health",
            SocketAddr::from(([127, 0, 0, 2], 40_004)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejected_mutations_do_not_poison_the_wal() {
        use crate::routes::Limits;

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("fhe_wal_server_{}_{}", std::process::id(), unique));
        std::fs::create_dir_all(&dir).unwrap();
        let log_path = dir.join("fhe-db.log");

        let wal = fhe_db::Wal::create(&log_path).unwrap();
        let state = AppState::with_limits_and_db_and_wal(
            Limits::default(),
            fhe_db::EncryptedDatabase::new(),
            wal,
        );
        let router = build_router(state.clone());
        let log_len = || std::fs::metadata(&log_path).unwrap().len();

        // Commit a table and a row so recovery has state to preserve.
        let create = r#"{"name":"t","columns":[{"name":"c","column_type":"U16"}]}"#;
        assert_eq!(
            json_request(router.clone(), Method::POST, "/tables", create)
                .await
                .status(),
            StatusCode::OK
        );
        let (client_key, _server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        let valid = B64.encode(
            encryptor
                .encrypt_u64(7, ScalarWidth::U16)
                .unwrap()
                .to_envelope_bytes()
                .unwrap(),
        );
        let insert = serde_json::json!({ "column": "c", "ciphertexts": [valid] }).to_string();
        assert_eq!(
            json_request(router.clone(), Method::POST, "/tables/t/rows", &insert)
                .await
                .status(),
            StatusCode::OK
        );

        // Rejected duplicate table.
        let committed_len = log_len();
        let response = json_request(router.clone(), Method::POST, "/tables", create).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(error_code(response).await, "TABLE_EXISTS");
        assert_eq!(log_len(), committed_len);

        // Rejected duplicate column.
        let duplicate_column = r#"{"name":"c","column_type":"U16"}"#;
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/t/columns",
            duplicate_column,
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(error_code(response).await, "COLUMN_EXISTS");
        assert_eq!(log_len(), committed_len);

        // Rejected width-mismatched insert.
        let wrong_width = B64.encode(
            encryptor
                .encrypt_u64(9, ScalarWidth::U32)
                .unwrap()
                .to_envelope_bytes()
                .unwrap(),
        );
        let bad_insert =
            serde_json::json!({ "column": "c", "ciphertexts": [wrong_width] }).to_string();
        let response =
            json_request(router.clone(), Method::POST, "/tables/t/rows", &bad_insert).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(response).await, "WIDTH_MISMATCH");
        assert_eq!(log_len(), committed_len);

        // Restart: every committed mutation survives and replay does not
        // choke on a record for a mutation the database rejected.
        drop(router);
        drop(state);
        let (db, recovered) = fhe_db::Wal::recover(&log_path).unwrap();
        assert_eq!(db.table_schema("t").unwrap().columns.len(), 1);
        let column = db.column("t", "c").unwrap();
        assert_eq!(column.len(), 1);
        assert_eq!(
            column.iter().next().unwrap().decrypt(&client_key).as_u64(),
            7
        );
        drop(recovered);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn checkpoint_acquires_database_before_wal() {
        use crate::routes::Limits;

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "fhe_checkpoint_lock_order_{}_{}",
            std::process::id(),
            unique
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let log_path = dir.join("fhe-db.log");
        let wal = fhe_db::Wal::create(&log_path).unwrap();
        let state = AppState::with_limits_and_db_and_wal(
            Limits::default(),
            fhe_db::EncryptedDatabase::new(),
            wal,
        );

        // Hold the database write lock, as a mutation handler does, so the
        // checkpoint cannot complete.
        let db_guard = state.db.write().await;
        let checkpoint = {
            let state = state.clone();
            tokio::spawn(async move { state.checkpoint_wal().await })
        };
        // Give the checkpoint task time to reach its first lock acquisition.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // With the db-before-wal invariant the checkpoint is blocked on
        // `db.read()` and the WAL is still free. Under the old wal-before-db
        // order it would hold `wal.write()` while waiting for the database,
        // which is the deadlock this test pins.
        let wal_probe = state.wal.try_write();
        assert!(
            wal_probe.is_ok(),
            "checkpoint held the WAL while waiting for the database: lock order inverted"
        );
        drop(wal_probe);
        drop(db_guard);

        checkpoint.await.unwrap().unwrap();
        drop(state);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn wal_recovery_and_checkpoint_metrics_are_exposed() {
        use crate::routes::Limits;

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("fhe_wal_metrics_{}_{}", std::process::id(), unique));
        std::fs::create_dir_all(&dir).unwrap();
        let log_path = dir.join("fhe-db.log");
        let wal = fhe_db::Wal::create(&log_path).unwrap();
        let state = AppState::with_limits_and_db_and_wal(
            Limits::default(),
            fhe_db::EncryptedDatabase::new(),
            wal,
        );

        // Journal one mutation, then compact it through the same helper the
        // periodic task and graceful shutdown use.
        let schema = fhe_db::TableSchema::new("t").with_column(fhe_db::ColumnDef {
            name: "c".into(),
            column_type: fhe_db::ColumnType::U32,
        });
        state
            .wal
            .write()
            .await
            .as_mut()
            .unwrap()
            .append(&fhe_db::WalRecord::CreateTable { schema })
            .unwrap();
        state.checkpoint_wal().await.unwrap();

        // The CLI folds the boot report into the same counters right after
        // constructing the state.
        state.metrics.record_recovery(&fhe_db::RecoveryReport {
            snapshot_bytes: 4096,
            records_replayed: 2,
            record_bytes_replayed: 512,
            torn_tail_bytes_truncated: 7,
            log_bytes: 4608,
            created: false,
        });
        state.metrics.record_recovery_failure();

        let response = request(
            build_router(state.clone()),
            Method::GET,
            "/metrics",
            JSON_TYPE,
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("fhe_wal_checkpoints_total 1"), "{text}");
        assert!(
            text.contains("fhe_wal_checkpoint_records_compacted_total 1"),
            "{text}"
        );
        assert!(
            text.contains("fhe_wal_checkpoint_failures_total 0"),
            "{text}"
        );
        assert!(text.contains("fhe_wal_recoveries_total 1"), "{text}");
        assert!(
            text.contains("fhe_wal_recovery_records_replayed_total 2"),
            "{text}"
        );
        assert!(
            text.contains("fhe_wal_recovery_record_bytes_total 512"),
            "{text}"
        );
        assert!(
            text.contains("fhe_wal_torn_tail_bytes_truncated_total 7"),
            "{text}"
        );
        assert!(text.contains("fhe_wal_recovery_failures_total 1"), "{text}");
        // The recovery report seeds the gauge; a fresh boot that reports
        // nothing leaves it at the checkpointed log size.
        assert!(text.contains("fhe_wal_log_bytes 4608"), "{text}");
        assert!(
            text.contains("fhe_wal_checkpoint_duration_seconds_total "),
            "{text}"
        );

        drop(state);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn metrics_expose_counters() {
        let router = build_router(AppState::new());
        let _ = request(
            router.clone(),
            Method::GET,
            "/health",
            JSON_TYPE,
            Body::empty(),
        )
        .await;
        let _ = request(
            router.clone(),
            Method::GET,
            "/health",
            JSON_TYPE,
            Body::empty(),
        )
        .await;
        // One 404 to bump the 4xx class.
        let _ = request(
            router.clone(),
            Method::GET,
            "/tables/ghost",
            JSON_TYPE,
            Body::empty(),
        )
        .await;

        let response = request(router, Method::GET, "/metrics", JSON_TYPE, Body::empty()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("# TYPE fhe_requests_total counter"), "{text}");
        assert!(text.contains("fhe_requests_total 3"), "{text}");
        assert!(
            text.contains("fhe_responses_total{class=\"2xx\"} 2"),
            "{text}"
        );
        assert!(
            text.contains("fhe_responses_total{class=\"4xx\"} 1"),
            "{text}"
        );
        assert!(text.contains("fhe_rate_limited_total 0"), "{text}");
    }

    #[tokio::test]
    async fn concurrent_inserts_are_serialized_and_counted() {
        use crate::routes::Limits;
        // Generous rate quota so the concurrency test is not throttled.
        let limits = Limits {
            rate_quota: 10_000,
            rate_quota_expensive: 10_000,
            ..Limits::default()
        };
        let (client_key, _server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        let state = AppState::with_limits(limits);
        let router = build_router(state.clone());

        let _ = json_request(
            router.clone(),
            Method::POST,
            "/tables",
            r#"{"name":"t","columns":[{"name":"c","column_type":"U32"}]}"#,
        )
        .await;

        // 8 writers × 25 inserts of a single encrypted row each.
        let mut handles = Vec::new();
        for _ in 0..8 {
            let router = router.clone();
            let encryptor = encryptor.clone();
            handles.push(tokio::spawn(async move {
                let mut inserted = 0;
                for _ in 0..25 {
                    let ct = encryptor.encrypt_u64(1, ScalarWidth::U32).unwrap();
                    let encoded = B64.encode(ct.to_envelope_bytes().unwrap());
                    let response = json_request(
                        router.clone(),
                        Method::POST,
                        "/tables/t/rows",
                        &serde_json::json!({"column":"c","ciphertexts":[encoded]}).to_string(),
                    )
                    .await;
                    assert_eq!(response.status(), StatusCode::OK);
                    inserted += 1;
                }
                inserted
            }));
        }
        let mut total = 0;
        for handle in handles {
            total += handle.await.unwrap();
        }
        assert_eq!(total, 200);

        let db = state.db.read().await;
        let column = db.column("t", "c").unwrap();
        assert_eq!(column.len(), 200);
    }

    #[tokio::test]
    async fn partial_batch_leaves_column_untouched() {
        let (_client_key, server_key) = ClientKey::new();
        let state = AppState::new();
        let _ = request(
            build_router(state.clone()),
            Method::POST,
            "/keys/server",
            OCTET_TYPE,
            Body::from(server_key.to_bytes().unwrap()),
        )
        .await;
        let router = build_router(state.clone());
        let _ = json_request(
            router.clone(),
            Method::POST,
            "/tables",
            r#"{"name":"t","columns":[{"name":"c","column_type":"U32"}]}"#,
        )
        .await;

        let encryptor = Encryptor::new(&ClientKey::new().0);
        let good = B64.encode(
            encryptor
                .encrypt_u64(1, ScalarWidth::U32)
                .unwrap()
                .to_envelope_bytes()
                .unwrap(),
        );
        let bad = B64.encode(b"corrupt payload");
        let response = json_request(
            router.clone(),
            Method::POST,
            "/tables/t/rows",
            &serde_json::json!({"column":"c","ciphertexts":[good.clone(), bad, good.clone()]})
                .to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(response).await, "INVALID_ENVELOPE");

        // All-or-nothing: the two valid envelopes were not inserted.
        let db = state.db.read().await;
        assert_eq!(db.column("t", "c").unwrap().len(), 0);
    }

    #[tokio::test]
    async fn restored_database_is_served() {
        // Backup/restore roundtrip through the server layer: persist a
        // database with `EncryptedDatabase::save`, restore it into
        // `AppState::with_db`, and confirm the schema is served.
        let mut db = fhe_db::EncryptedDatabase::new();
        let schema = fhe_db::TableSchema::new("restored").with_column(fhe_db::ColumnDef {
            name: "amount".into(),
            column_type: fhe_db::ColumnType::U32,
        });
        db.create_table(schema).unwrap();

        let dir = std::env::temp_dir().join(format!("fhe_db_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.bin");
        db.save(&path).unwrap();

        let restored = fhe_db::EncryptedDatabase::load(&path).unwrap();
        let router = build_router(AppState::with_db(restored));
        let response = request(router, Method::GET, "/tables", JSON_TYPE, Body::empty()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["name"], "restored");

        std::fs::remove_dir_all(&dir).ok();
    }
}

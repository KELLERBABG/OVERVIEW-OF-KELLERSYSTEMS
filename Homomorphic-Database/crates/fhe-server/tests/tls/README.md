# Test-only TLS certificates

`localhost-cert.pem` / `localhost-key.pem` are a **self-signed, test-only**
certificate pair generated for the `fhe-server` TLS integration test
(`tls_serves_health_over_https` in `crates/fhe-server/src/lib.rs`).

- Subject: `CN=localhost`
- SAN: `DNS:localhost, IP:127.0.0.1`
- Validity: 2026-08-05 → 2036-08-02 (10 years)
- Key: RSA-2048, PKCS#8, **unencrypted**

**The private key is intentionally public.** This pair exists so the test can
spin up a real rustls listener and connect to it without depending on a
certificate-authority at test time. It must **never** be used to serve a real
deployment — any production certificate must come from a proper CA (or an
internal PKI) and be supplied at runtime via `fhe-server --tls-cert/--tls-key`.

Regenerate with (OpenSSL ≥ 3):

```bash
MSYS_NO_PATHCONV=1 openssl req -x509 -newkey rsa:2048 \
  -keyout localhost-key.pem -out localhost-cert.pem -days 3650 -nodes \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
```

(On Git-Bash/Windows, `MSYS_NO_PATHCONV=1` prevents the leading `/` of
`-subj` from being rewritten to a Windows path.)

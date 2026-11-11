# Key Lifecycle Review

**Scope:** `ClientKey`/`ServerKey` generation, custody, backup, restore,
rotation, and destruction in the workspace and the `fhe-phone` shell.
**Status:** internal review, first filed 2026-11-10; **F4 and F8 fixed
2026-11-11** and re-verified against the single-file vault. Residual risks are
listed in §8 and are not claimed as fixed.

## 1. Lifecycle at a glance

| Stage | What the code does | Evidence |
|---|---|---|
| Generation | `ClientKey::new()` builds a fresh pair from TFHE-rs's default config; the phone's `keygen` runs it in a worker, then saves and uploads the public key. It refuses to overwrite an existing stored pair. | `crates/fhe-core/src/keys.rs`; `apps/fhe-phone/src/engine.rs` `keygen` |
| Custody | Both keys are persisted in one `vault.json` (format v2) under `%APPDATA%/FHE-DB` (or `~/.fhe-db-phone`) as AES-256-GCM ciphertext under a pinned Argon2id PIN-derived key. | `apps/fhe-phone/src/keystore.rs` |
| Unlock | Check the PIN against the PHC hash with a persisted failure lockout, decrypt the key pair, hold the `ClientKey` in memory, re-upload the `ServerKey` to the embedded server. | `KeyStore::check_pin_at`, `KeyStore::unlock`; `engine.rs` `lock_unlock` |
| Backup | Export a v2 JSON object containing the whole vault (PIN hash + encrypted key pair); v1 backups remain importable. | `KeyStore::export_backup`, `import_backup` |
| Restore | Validate the backup, reset failure state, write the new vault in one atomic commit, drop in-memory state, require unlock with the imported PIN. | `KeyStore::import_backup`; `engine.rs` `backup_import` |
| Rotation | No key-pair rotation. PIN change re-encrypts the same pair in one atomic commit; auth-token rotation is operational; server-key replacement has no versioning. | `KeyStore::change_pin`; `crates/fhe-server/src/routes.rs` `upload_server_key` |
| Destruction | `vault.destroy {pin}` authenticates, removes the vault (and legacy files) atomically from the app's perspective, and clears the in-memory key, PIN, and model flag. Exported backups and OS copies are not erased. | `KeyStore::destroy`; `engine.rs` `vault_destroy`; Settings → Security → Destroy keys |

## 2. Generation

- `ClientKey::new` uses `tfhe::ConfigBuilder::default().build()`, so the
  parameter set is whatever TFHE-rs 1.7.0 resolves for the high-level integer
  API (see [`PARAMETERS.md`](PARAMETERS.md)).
- Generation happens in the client process/phone worker. There is no HSM,
  secure enclave, entropy health check, or key ceremony.
- The server receives only `ServerKey` (`POST /keys/server`) and keeps it in
  `AppState.server_key`. Uploading a new key replaces the old one; existing
  ciphertexts are not re-encrypted or versioned, so evaluation after a
  replacement is not meaningful for old ciphertexts.
- `keygen` now returns an error if the vault already stores keys but the phone
  is locked, instead of generating a new pair and overwriting the old one.
  Saving happens before the server-key upload by design: a failed upload leaves
  the secret key safely stored, and the next unlock re-uploads the public key.
  No key material is lost.

## 3. Storage (format v2)

The phone store is one file, `vault.json`:

```json
{
  "version": 2,
  "kdf": { "m_cost": 19456, "t_cost": 2, "p_cost": 1, "output_len": 32 },
  "pin_hash": "$argon2id$v=19$m=19456,t=2,p=1$...",
  "payload_format": "binary_v2",
  "keys": { "salt": "...", "nonce": "...", "ciphertext": "..." },
  "failed_attempts": 0,
  "locked_until_unix": 0
}
```

- **One atomic commit.** `atomic_write` creates `vault.json.tmp`, writes and
  `sync_all`s it, then renames it over `vault.json`. A crash or I/O error
  leaves either the complete old vault or the complete new one. On Unix the
  directory is also synced; Windows does not expose directory fsync, so
  power-loss durability of the rename itself is not proven (see §8).
- **Pinned KDF.** The Argon2id parameters are a deliberate constant
  (`KdfParams::PINNED`: 19 MiB, t=2, p=1, 32-byte output, v0x13) and are
  recorded in the vault. A future `argon2` default change cannot strand
  existing keys. Parameters are bounds-checked before use, so a hand-edited or
  imported vault cannot request a multi-terabyte allocation.
- **PIN policy.** 6–12 ASCII digits, matching what the lock-screen keypad can
  enter. The lock screen still accepts a legacy 4-digit PIN to unlock an
  existing store; new and changed PINs must meet the floor.
- **Persisted lockout.** Five free attempts, then 30 s doubling to a 1 h cap,
  stored in the vault and checked before the KDF runs. This bounds online
  guessing at the lock screen and survives a restart; it does not stop offline
  guessing from a copied file.
- **Zeroization.** The derived AES key, the decrypted payload, the encoded key
  buffers, and the in-memory PIN use `zeroize::Zeroizing`, so they are wiped on
  drop. The `ClientKey`/`ServerKey` types come from TFHE-rs and expose no
  zeroization API; they are dropped but not explicitly wiped.
- **Payload formats.** New writes use a length-prefixed binary payload
  (`binary_v2`). A migrated v1 file keeps the old JSON payload layout
  (`json_v1`) so its ciphertext stays decryptable; the next PIN change or save
  rewrites it in the v2 layout.
- **Migration.** On open, a v1 `pin.json` + `keys.json` pair is combined into
  `vault.json` (atomic write) and then removed. If a crash happens after the
  rename, the v2 vault is authoritative and the legacy files are cleaned up on
  the next open.
- **Permissions.** The store relies on the platform data directory and the OS
  user account; it does not set or verify restrictive ACLs.

## 4. Backup and restore

`export_backup` writes:

```json
{ "version": 2, "vault": { ...the complete VaultFile... } }
```

The backup is copyable but **not independently authenticated**. Its
confidentiality depends on the encrypted key pair and the PIN hash, so a stolen
backup is an offline guessing target. The backup contains the Argon2id PHC
hash, which is what makes verification possible after restore.

`import_backup`:

1. parses and version-checks the backup (v2 object, or v1 `pin`/`keys` strings);
2. validates the KDF bounds and the PHC string, rejecting absurd parameters;
3. resets the failure counter and lockout;
4. writes the new vault in **one atomic commit**;
5. drops the in-memory key and PIN in `engine.rs`.

A v1 backup is converted to the v2 vault with the recorded legacy parameters;
its JSON payload layout is preserved. A valid empty v1 backup (no PIN, no keys)
clears the local store. An imported backup is a new root of trust; the user is
expected to unlock with its PIN.

## 5. Rotation and destruction

- **PIN change** (`lock.changepin`): verify the old PIN under the lockout
  policy → decrypt the key pair → hash the new PIN → re-encrypt with a fresh
  salt/nonce → write the new hash and ciphertext in **one atomic commit**. A
  failure before the rename leaves the old vault intact, which the
  `pin_change_is_atomic_and_preserves_keys` test proves by making the commit
  fail and then unlocking with the old PIN.
- **Key-pair rotation:** absent. There is no API to generate a new pair while
  keeping old ciphertexts decryptable, no ciphertext re-encryption, and no
  key-version field in the envelope/DB. Rotating means starting a new database.
- **Auth-token rotation:** supported operationally by restarting with a new
  `--auth-token`/`FHE_DB_AUTH_TOKEN`; there is no online rotation or key ID.
- **Server-key rotation:** `POST /keys/server` can replace the key, but
  ciphertexts are not bound to a key ID; old data is effectively orphaned for
  homomorphic evaluation.
- **Destruction:** `vault.destroy {pin}` requires the PIN (through the same
  lockout), erases the local vault and legacy files, and clears the in-memory
  key, PIN, and model state. This is crypto-shredding: ciphertexts already on
  the server remain but cannot be decrypted. Exported backups, OS
  file-system copies, and the server's ciphertext store are not erased.

## 6. Documented vs implemented guarantees

| Document claim | Actual behavior |
|---|---|
| "Key derivation uses Argon2id with pinned parameters." | True: `KdfParams::PINNED` is recorded in the vault and used for every new derivation. A missing/corrupt parameter block is rejected, not defaulted. |
| "A stolen `vault.json` is resistant to fast offline guessing." | Qualified: Argon2id (19 MiB, t=2) raises the per-guess cost, but a 6-digit PIN is still only 10^6 candidates. The backup contains the hash. |
| "The PIN cannot be entered but not set." | Fixed: the policy is 6–12 digits and both the keypad and Settings agree. |
| "PIN change cannot lock the user out." | True for the commit: hash and ciphertext change in one atomic write. Test injection covers a failed commit. |
| "Only the PIN holder can use the backup." | Anyone who guesses the PIN can; the backup is not authenticated. |
| "The secret ClientKey never leaves your phone." | It is never sent to the server, but it is persisted encrypted and can be exported in a PIN-protected backup. |
| "Destruction removes the keys." | Local vault and in-memory state are erased; backups and OS copies are explicitly out of scope. |

## 7. Tests and limits

`cargo test -p fhe-phone --release` (13 tests: 12 unit + 1 headless engine
flow) covers:

- `pin_roundtrip_and_wrong_pin_rejected` — set/verify, wrong PIN, no-PIN state.
- `pin_policy_requires_six_to_twelve_digits` — length and digits-only policy.
- `kdf_parameters_are_pinned_and_recorded` — on-disk v2 fields.
- `legacy_kdf_defaults_match_the_pinned_migration_record` — v1 migration
  parameters equal `Argon2::default()`.
- `keys_save_unlock_roundtrip_and_wrong_pin` — real key material survives and
  decrypts a value encrypted by the original key.
- `pin_change_is_atomic_and_preserves_keys` — injected commit failure leaves the
  old vault decryptable; a successful change commits hash + ciphertext.
- `failures_are_persisted_and_lock_out_then_recover` — five failures, lockout
  persisted across instances, correct PIN after the wait, counter reset.
- `destroy_removes_local_key_material` — authenticated destruction.
- `export_import_roundtrip_and_v1_backup_import` — v2 roundtrip, v1 backup
  import, tampered KDF parameters rejected without touching the live vault.
- `legacy_two_file_store_is_migrated` — v1 pair becomes one vault, legacy files
  removed, migrated ciphertext decrypts.
- `full_phone_flow_over_embedded_server` — keygen, lock/unlock, wrong PIN,
  wallet/health/messages, encrypted read-back, encrypted XNOR inference, wire
  tap — all through the real embedded server and client SDK.

Not exercised: a real GUI launch, disk-full writes, actual power loss, OS-level
file permissions/ACLs, concurrent processes opening the same store, or a
malicious backup crafted to exploit parser edge cases beyond the KDF bounds.

## 8. Residual risks and open recommendations

1. **Offline PIN guessing.** The 6-digit floor and Argon2id cost reduce the
   risk but do not eliminate it; a 6-digit space is 10^6. A longer
   alphanumeric passphrase would be the real fix, at a UX cost. The backup is
   an offline target by design.
2. **No memory locking.** Keys and PIN copies can be swapped or read by a
   same-user debugger. `Zeroizing` wipes only the buffers this crate owns.
3. **TFHE-rs key zeroization.** `ClientKey`/`ServerKey` do not expose a wipe
   API; dropping them does not guarantee the heap is cleared.
4. **Windows rename durability.** File contents are flushed before the rename;
   the directory entry is not fsynced (unsupported on Windows). A power loss
   immediately after the rename is not covered by a test.
5. **Key-pair rotation is absent.** Replacing the server key orphans old
   ciphertexts; there is no key ID in the envelope or database.
6. **File permissions are not enforced.** The store depends on the OS account
   and the default ACL of `%APPDATA%`.
7. **Backups are unauthenticated.** An attacker who can replace the backup file
   can replace the root of trust on import; this is a local-trust boundary,
   stated in the UI and `SECURITY.md`.
8. **F7 remains open.** The keystore adds no constant-time key handling or
   memory locking; TFHE-rs states that side-channel mitigation is not
   implemented.

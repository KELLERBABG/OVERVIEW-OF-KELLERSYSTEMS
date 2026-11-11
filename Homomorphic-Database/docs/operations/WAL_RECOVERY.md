# WAL Recovery Runbook

**Audience:** whoever operates an `fhe-server` instance started with
`--data-dir` (the durable mode). **Scope:** the write-ahead log
`<data-dir>/fhe-db.log` — what it guarantees, what a failed boot means, and
what to do about it.

The engine's design is documented in `crates/fhe-db/src/wal.rs`; the boot and
shutdown sequence is in `crates/fhe-server/src/main.rs`.

---

## 1. What the log guarantees

The log is a single self-contained file:

```
[ 8-byte magic "FHELOG01" ]
[ 8-byte format version = 1 ]
[ 8-byte snapshot length  ]
[ snapshot: bincode(EncryptedDatabase) at the last checkpoint ]
[ record: u32 length | payload | u32 CRC-32 ] ...
```

Each record is one mutation (`CreateTable`, `AddColumn`, `InsertRows`). The
write path is **validate → append (+fsync) → apply → respond**. Consequences:

| Guarantee | Why |
|---|---|
| A `2xx` response means the mutation is on disk. It survives `kill -9`, `TerminateProcess`, and power loss. | The response is only sent after `append()` has `fsync`ed the record and the in-memory database has applied it. |
| A mutation that was **rejected** (4xx) leaves no record. | Records are validated against the live database before the append (audit F2). |
| A crash *between* append and the in-memory apply is harmless. | Recovery replays the record; the client saw a timeout, and the write is either fully present or fully absent — never half-applied. |
| A crash *mid-append* leaves a **torn tail**, which heals. | The trailing record is shorter than its length prefix; boot truncates to the last complete record and starts normally. |
| A **complete** record that fails its CRC, or cannot be decoded, is **fatal**. | Corruption is reported (`CorruptedDatabase`, exit 1); it is never silently skipped. |
| A graceful stop (`SIGTERM`, Ctrl-C, Ctrl-Break) compacts the log. | The shutdown path checkpoints: header + snapshot only, no records to replay. |
| The log never needs a second file to be consistent. | The snapshot and records are in one file, replaced atomically by `checkpoint()` via write-temp → fsync → rename. |

### Not guaranteed

- **Live copies.** Copying `fhe-db.log` while the server is running is not a
  backup; the file is being appended to and periodically replaced.
- **Single-writer discipline.** One process must own a data directory. There
  is no file lock: a second server pointed at the same `--data-dir` will
  interleave records into the same log and corrupt it.
- **Post-IO-error writes.** If an append fails part-way (disk full, quota,
  device error), a partial record can remain in the file. The server stays up
  and a later append would land after that partial record, which recovery
  reports as corruption. **After any WAL I/O error, stop the server and follow
  §4 before writing again.**
- **Confidentiality or integrity against the server itself.** Encryption
  protects values, not the log's plaintext metadata (schema, counts); nothing
  authenticates the log against a malicious local writer.

---

## 2. First response to a failed boot

The process exits with code `1` and prints the context line, then the specific
cause indented under `Caused by:` (verified against the release binary):

```
Error: failed to recover database from WAL <path>\fhe-db.log

Caused by:
    corrupted database file: <cause>
```

1. **Do not delete the log.** It is the only copy of the committed data.
2. **Copy it aside first** (the server is not running, so this is safe):
   `cp fhe-db.log fhe-db.log.investigate` — or on Windows
   `Copy-Item fhe-db.log fhe-db.log.investigate`.
3. Read the cause in the table below. Every message is emitted by
   `Wal::open`/`Wal::replay` in `crates/fhe-db/src/wal.rs`.

| Cause (substring of the error) | Meaning | Action |
|---|---|---|
| `is not an FHE-DB log (bad magic)` | The file is not a WAL (wrong path, or another file copied over it). | Point `--data-dir` at the right directory, or restore a backup. |
| `unsupported FHE-DB log version N` | The log was written by a newer build. | Run the build that wrote it, or restore a backup and re-insert. Downgrading is not supported. |
| `truncated in its snapshot region` | The file is shorter than the snapshot the header declares (partial copy, truncated restore). | Restore a complete backup. The snapshot cannot be reconstructed. |
| `log checksum mismatch at offset N` | A **complete** record failed CRC-32: bit rot, external edit, or a partial write followed by later appends. | Restore a backup. If none, §5 is the last resort. |
| `log record at offset N failed to decode` | Passed CRC but is not a valid record. | Same as above. |
| `log replay failed at record N: <DbError>` | The record is well-formed but does not apply to the recovered state (e.g. a duplicate table written by a **pre-fix** build before audit F2). | §5: find record N's offset and deliberately truncate the log there, or restore a backup. |
| `log record length exceeds file bounds during replay` | The file changed between `open` and `replay` (another process wrote to it, or it was replaced underneath). | Ensure only one process owns the directory, then retry with a restored copy. |
| `Io(...)` / `Access is denied` / `Permission denied` | Permissions, a read-only volume, or the file is open exclusively elsewhere. | Fix ACLs / close the other handle. No data is at risk. |
| Process exits `1` with no recovery error, or fails to bind | Not a WAL problem. | Check `--port`, the bind address, and the log line "server listening on". |

A torn tail does **not** produce an error and **does not** prevent startup: the
server truncates it and reports the byte count in its structured `wal.recover`
event (§2.1). Confirm recovery with §6 as well.

### 2.1 Boot and checkpoint events (structured log lines)

With `--data-dir`, the server emits **one** structured line per boot recovery
and **one** per checkpoint (periodic and shutdown). A clean boot and a silent
heal are now distinguishable, and a failed checkpoint is visible even though
the process keeps serving.

Boot (recovery):

```text
INFO fhe_server::wal: write-ahead log recovered event="wal.recover" outcome="recovered" path="..." snapshot_bytes=169464 records_replayed=2 record_bytes_replayed=512 torn_tail_bytes_truncated=0 log_bytes=169976
```

- `outcome="created"` — the log did not exist and was created (first boot).
- `outcome="recovered"` — an existing log was replayed.
- `torn_tail_bytes_truncated` — non-zero means a crash mid-append was healed;
  the value is exactly how many trailing bytes were dropped.
- `records_replayed` / `record_bytes_replayed` — what was applied on top of the
  snapshot.
- A failed recovery logs `outcome="error"` with the `Caused by:` cause and the
  process exits 1; the line appears before the error is printed.

Checkpoint (compaction):

```text
INFO fhe_server::wal: write-ahead log checkpointed event="wal.checkpoint" outcome="ok" path="..." snapshot_bytes=169464 records_compacted=3 record_bytes_compacted=1024 log_bytes_before=170512 log_bytes_after=169488 duration_micros=12400
```

- `records_compacted` — appends dropped since the previous checkpoint; `0`
  means the log was already compact.
- `log_bytes_before` → `log_bytes_after` — the size change from the atomic
  replace.
- `duration_micros` — how long serializing, writing, and renaming took.

When stdout is a terminal the lines are coloured; when it is redirected (a
service log, `server-stdio.log`) colouring is disabled, so `field=value` stays
contiguous and grep-able.
- A failure logs `outcome="error"` with `error=...`; the server stays up and
  retries at the next interval, but see §4 before writing more data.

Every one of these values is also in `GET /metrics` (`fhe_wal_*`):
`fhe_wal_recoveries_total`, `fhe_wal_recovery_failures_total`,
`fhe_wal_recovery_records_replayed_total`, `fhe_wal_recovery_record_bytes_total`,
`fhe_wal_torn_tail_bytes_truncated_total`, `fhe_wal_log_bytes`,
`fhe_wal_checkpoints_total`, `fhe_wal_checkpoint_failures_total`,
`fhe_wal_checkpoint_records_compacted_total`,
`fhe_wal_checkpoint_snapshot_bytes_total`, and
`fhe_wal_checkpoint_duration_seconds_total`.

---

## 3. Backup and restore (supported procedure)

**Backup** — with the server stopped (graceful stop preferred, so the log is
already compacted):

```bash
# POSIX
cp <data-dir>/fhe-db.log backup/fhe-db-$(date +%Y%m%dT%H%M%S).log
```

```powershell
# Windows PowerShell
Copy-Item <data-dir>\fhe-db.log ("backup\fhe-db-" + (Get-Date -Format s) + ".log")
```

`<data-dir>/fhe-db.tmp` is a checkpoint scratch file and may be ignored (or
deleted while the server is stopped).

**Restore** — with the server stopped:

1. Move the current log aside (`fhe-db.log.bad-<timestamp>`).
2. Copy the backup to `<data-dir>/fhe-db.log`.
3. Start the server and run the §6 verification.

**Legacy snapshot migration.** A pre-WAL `database.bin` is imported **only**
when `fhe-db.log` is absent. If both exist, the log wins and `database.bin` is
ignored. Never delete the log to "force" a migration over live data.

---

## 4. Disk-full or I/O error during operation

1. Stop the server (do not keep serving; see §1 "Not guaranteed").
2. Free space / fix the device. Do not start a new server against the same
   directory before step 3.
3. Try to start it.
   - Boot succeeds → the tail was torn and healed. Verify with §6, then
     re-issue the mutations that failed (a failed mutation was never
     acknowledged).
   - Boot fails with `checksum mismatch` / `failed to decode` → the partial
     record was followed by more appends. Restore the most recent backup; the
     data after that backup must be re-inserted.
4. If the volume cannot be trusted, replace it before restoring.

---

## 5. Last resort: deliberately truncating a poisoned log

Use this only when replay fails at a specific record and no backup exists.
**Everything after the truncation point is lost.** Prefer a backup whenever one
exists.

First list the record boundaries. Save this as `wal_records.py` next to the log:

```python
"""List FHE-DB WAL record boundaries and checksums (read-only)."""
import binascii, struct, sys

MAGIC = b"FHELOG01"
path = sys.argv[1]
data = open(path, "rb").read()
if len(data) < 24 or data[:8] != MAGIC:
    sys.exit("not an FHE-DB log (bad magic)")
version, snap = struct.unpack_from("<QQ", data, 8)
if version != 1:
    sys.exit(f"unsupported log version {version}")
print(f"version={version} snapshot_len={snap} file_len={len(data)}")
off = 24 + snap
i = 0
while off < len(data):
    if off + 4 > len(data):
        print(f"record {i}: torn tail at offset {off}")
        break
    (ln,) = struct.unpack_from("<I", data, off)
    end = off + 4 + ln + 4
    if end > len(data):
        print(f"record {i}: torn tail at offset {off} (declares {ln} bytes)")
        break
    payload = data[off + 4 : off + 4 + ln]
    (stored,) = struct.unpack_from("<I", data, off + 4 + ln)
    actual = binascii.crc32(payload) & 0xFFFFFFFF
    if actual != stored:
        print(f"record {i}: offset={off} CRC MISMATCH "
              f"(stored {stored:08x}, computed {actual:08x})")
        break
    print(f"record {i}: offset={off} payload={ln} end={end} crc ok")
    off = end
    i += 1
print(f"{i} complete record(s)")
```

Run it against the copy you made in §2:

```bash
python wal_records.py fhe-db.log.investigate
```

Take the `offset=` of the record named in the boot error (`record N`, counting
from 0) and truncate the **live** log to that offset — after making sure the
server is stopped:

```bash
# POSIX
truncate -s <offset> <data-dir>/fhe-db.log
```

```powershell
# Windows PowerShell
$f = [System.IO.File]::Open("<data-dir>\fhe-db.log", 'Open', 'Write')
$f.SetLength(<offset>)
$f.Close()
```

Then start the server and verify (§6). Because the truncated record was a
mutation the old build failed to apply, the resulting state is the state as of
just before that mutation.

---

## 6. Verify a recovery

```bash
curl -s http://127.0.0.1:8080/health          # {"status":"ok"}
curl -s http://127.0.0.1:8080/tables          # schemas survived?
curl -s http://127.0.0.1:8080/tables/<t>/ciphertexts/<c> | head -c 200
```

The `count` in the read-back response is public metadata; compare it with the
number of rows you expect. Decrypting the returned envelopes with the client
key is the only complete check — the server cannot do it for you. Cross-check
the boot line (§2.1): `torn_tail_bytes_truncated=0` on a clean restore, and a
`records_replayed` value that matches how far behind the last checkpoint the
backup was. Then stop the server gracefully and confirm the log compacted:

```bash
head -c 24 <data-dir>/fhe-db.log | od -An -tu8
# field 3 is the snapshot length; a cleanly stopped server has
# file size == 24 + snapshot length
```

---

## 7. What pins these claims

| Claim | Evidence |
|---|---|
| Graceful stop exits 0 and compacts the log | `crates/fhe-server/tests/durability.rs::graceful_stop_compacts_the_wal_and_restart_recovers` (SIGTERM on unix, Ctrl-Break on Windows) |
| Acknowledged rows survive a hard kill; a rejected mutation leaves no record | `durability.rs::hard_kill_loses_nothing_acknowledged_and_stays_appendable` |
| A torn tail heals on boot and keeps the committed prefix | `durability.rs::torn_tail_heals_on_boot_and_keeps_the_committed_prefix`, `fhe_db::wal::tests::torn_tail_is_truncated` |
| Crash before append / after append before apply / mid-checkpoint | `fhe_db::wal::tests::crash_before_append_keeps_only_committed_rows`, `crash_after_append_before_apply_replays_the_record`, `checkpoint_ignores_a_stale_partial_tmp_file` |
| Complete-record corruption is fatal, never silent | `fhe_db::wal::tests::corrupt_complete_record_is_rejected` |
| Checkpoint lock order cannot deadlock against mutations | `fhe_server::tests::checkpoint_acquires_database_before_wal` |
| Boot and checkpoint emit one structured line with real counts | `crates/fhe-server/tests/durability.rs` asserts `wal.recover`, `wal.checkpoint`, `records_replayed`, `records_compacted`, and `torn_tail_bytes_truncated=5` in the child's output |
| The recovery/checkpoint counters are exposed | `fhe_server::tests::wal_recovery_and_checkpoint_metrics_are_exposed`; `fhe_db::wal::tests::recovery_report_counts_records_snapshot_and_torn_tail`, `checkpoint_report_counts_only_appends_since_the_previous_checkpoint` |

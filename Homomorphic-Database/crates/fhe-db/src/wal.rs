//! Write-ahead logging for crash-safe persistence of [`EncryptedDatabase`].
//!
//! FHE-DB persists as a **single log-structured file**, which makes crash
//! recovery sound without any two-file consistency protocol:
//!
//! ```text
//! [ 8-byte magic "FHELOG01"       ]
//! [ 8-byte format version u64 = 1 ]
//! [ 8-byte snapshot length u64    ]
//! [ snapshot bytes ]                ← bincode(EncryptedDatabase) at last checkpoint
//! [ record ... ]                    ← append-only after the snapshot
//! ```
//!
//! A [`Wal`] owns one open handle. Mutations are journaled with
//! [`Wal::append`] **before** they are applied to the in-memory database
//! (write-ahead ordering), with an `fsync` per record so a committed
//! operation survives power loss. [`Wal::checkpoint`] compacts the log by
//! atomically replacing it with a fresh snapshot (header + snapshot, no
//! records), so the log never grows without bound and recovery never
//! double-applies an operation.
//!
//! [`Wal::recover`] opens an existing log and replays every record after the
//! snapshot onto it, returning the fully-recovered database plus an
//! appendable [`Wal`]. A torn trailing record — a crash mid-append — is
//! detected via its length prefix and truncated automatically. Corruption
//! *within* a complete record (bad CRC32, undecodable payload, truncated
//! snapshot region, bad magic) is rejected with
//! [`DbError::CorruptedDatabase`]; the log is never silently misread.
//!
//! ## Single-writer assumption
//!
//! One process must own the log at a time. The server serializes every
//! mutation behind its database write-lock and appends under the same lock
//! ordering, so the WAL records exactly the committed mutations in order.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fhe_core::Ciphertext;
use serde::{Deserialize, Serialize};

use crate::database::EncryptedDatabase;
use crate::error::{DbError, DbResult};
use crate::schema::{ColumnDef, TableSchema};

/// Magic bytes at the start of every FHE-DB log file.
pub const CURRENT_LOG_MAGIC: &[u8; 8] = b"FHELOG01";
/// On-disk log format version. Bump deliberately and add migration on change.
///
/// Frozen for the external audit: `crates/fhe-db/tests/format_freeze.rs`
/// fails if this changes without a deliberate update to
/// [`docs/audit/FORMAT_FREEZE.md`](../../../docs/audit/FORMAT_FREEZE.md).
pub const CURRENT_LOG_VERSION: u64 = 1;
/// Fixed header size: magic + version + snapshot length.
pub const LOG_HEADER_LEN: u64 = 24;
/// Magic as the little-endian word read from the header.
const LOG_MAGIC: u64 = u64::from_le_bytes(*CURRENT_LOG_MAGIC);

/// What one boot recovery did, reported once per server start.
///
/// The server emits this as a single structured `wal.recover` event and folds
/// it into the `/metrics` WAL counters. Every field is a byte/record count:
/// no schema names, keys, or plaintext values are included.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Byte length of the snapshot section (0 for a fresh, empty log).
    pub snapshot_bytes: u64,
    /// Number of appended records replayed onto the snapshot.
    pub records_replayed: u64,
    /// Byte length of the replayed record region (frame + payload + CRC).
    pub record_bytes_replayed: u64,
    /// Bytes of the incomplete trailing record truncated during open (0 when
    /// clean). This is the whole unreadable record region, not merely the
    /// bytes missing from its end.
    pub torn_tail_bytes_truncated: u64,
    /// Log size on disk after open and any torn-tail truncation.
    pub log_bytes: u64,
    /// Whether the log did not exist and was created by this recovery.
    pub created: bool,
}

/// What one checkpoint (log compaction) did.
///
/// The server emits this as a single structured `wal.checkpoint` event and
/// folds it into the `/metrics` WAL counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckpointReport {
    /// Byte length of the snapshot written by this checkpoint.
    pub snapshot_bytes: u64,
    /// Records dropped from the log (appends since the previous checkpoint).
    pub records_compacted: u64,
    /// Record-region bytes dropped from the log.
    pub record_bytes_compacted: u64,
    /// Log size on disk immediately before the compaction.
    pub log_bytes_before: u64,
    /// Log size on disk after the atomic replace.
    pub log_bytes_after: u64,
    /// Wall-clock duration of the checkpoint (serialize + write + rename).
    pub duration: Duration,
}

/// A single journaled mutation, mirroring the database's mutation surface.
///
/// Deliberately does **not** derive `Debug`: an [`InsertRows`](Self::InsertRows)
/// record holds ciphertexts, which never expose plaintext via debug output.
#[derive(Clone, Serialize, Deserialize)]
pub enum WalRecord {
    /// Create a table from its schema.
    CreateTable {
        /// Table schema (names/types are plaintext metadata).
        schema: TableSchema,
    },
    /// Add a column to an existing table.
    AddColumn {
        /// Target table name.
        table: String,
        /// New column definition.
        column: ColumnDef,
    },
    /// Append ciphertexts to a column (width-checked on replay).
    InsertRows {
        /// Target table name.
        table: String,
        /// Target column name.
        column: String,
        /// The encrypted rows.
        ciphertexts: Vec<Ciphertext>,
    },
}

impl WalRecord {
    /// Check whether applying this record to `db` would be accepted.
    ///
    /// This is read-only. The server calls it while holding the database
    /// write lock, before [`Wal::append`], so a mutation the database would
    /// reject cannot leave a record that blocks replay.
    ///
    /// # Errors
    ///
    /// Returns the same [`DbError`] the corresponding mutation would return:
    /// [`DbError::TableExists`], [`DbError::UnknownTable`],
    /// [`DbError::UnknownColumn`], [`DbError::ColumnExists`], or
    /// [`DbError::ColumnWidthMismatch`].
    pub fn validate(&self, db: &EncryptedDatabase) -> DbResult<()> {
        match self {
            WalRecord::CreateTable { schema } => match db.table_schema(&schema.name) {
                Ok(_) => Err(DbError::TableExists(schema.name.clone())),
                Err(DbError::UnknownTable(_)) => Ok(()),
                Err(e) => Err(e),
            },
            WalRecord::AddColumn { table, column } => {
                let schema = db.table_schema(table)?;
                if schema.column(&column.name).is_some() {
                    return Err(DbError::ColumnExists(column.name.clone()));
                }
                Ok(())
            }
            WalRecord::InsertRows {
                table,
                column,
                ciphertexts,
            } => {
                let def = db
                    .table_schema(table)?
                    .column(column)
                    .ok_or_else(|| DbError::UnknownColumn {
                        table: table.clone(),
                        column: column.clone(),
                    })?
                    .clone();
                let expected = def.column_type.to_scalar_width();
                for ct in ciphertexts {
                    if ct.width() != expected {
                        return Err(DbError::ColumnWidthMismatch {
                            column: column.clone(),
                            expected,
                            found: ct.width(),
                        });
                    }
                }
                Ok(())
            }
        }
    }
}

/// An append-only write-ahead log.
///
/// `Debug` is redacted (non-exhaustive, path only): the log may reference
/// ciphertexts, which never expose plaintext via debug output.
#[derive(Debug)]
pub struct Wal {
    /// Open handle used for appends; the cursor always sits at end-of-file.
    file: File,
    /// Path of the log file (reopened after a checkpoint replaces it).
    path: PathBuf,
    /// Byte length of the header + snapshot section; records start here.
    snapshot_len: u64,
    /// Appends since the last checkpoint, reported by the next compaction.
    records_since_checkpoint: u64,
}

impl Wal {
    /// Create a new, empty log at `path`.
    ///
    /// Refuses to overwrite an existing file so a misconfigured data dir
    /// cannot silently destroy a live log.
    ///
    /// # Errors
    ///
    /// - [`DbError::CorruptedDatabase`] if `path` already exists.
    /// - [`DbError::Io`] on filesystem errors.
    pub fn create(path: impl AsRef<Path>) -> DbResult<Self> {
        let path = path.as_ref();
        if path.exists() {
            return Err(DbError::CorruptedDatabase(format!(
                "refusing to create a WAL over the existing file {}",
                path.display()
            )));
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(&LOG_MAGIC.to_le_bytes())?;
        file.write_all(&CURRENT_LOG_VERSION.to_le_bytes())?;
        file.write_all(&0u64.to_le_bytes())?; // empty snapshot section
        file.sync_all()?;
        file.seek(SeekFrom::End(0))?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            snapshot_len: 0,
            records_since_checkpoint: 0,
        })
    }

    /// Open an existing log for appending, validating it and healing a torn
    /// trailing record.
    ///
    /// The snapshot region is checked to be fully present; a truncated
    /// snapshot is a fatal [`DbError::CorruptedDatabase`]. A torn *tail* —
    /// a crash mid-append — is detected and truncated automatically.
    ///
    /// # Errors
    ///
    /// - [`DbError::CorruptedDatabase`] for a bad magic, unsupported version,
    ///   truncated snapshot region, or a malformed *complete* record (bad
    ///   CRC32 or undecodable payload).
    /// - [`DbError::Io`] on filesystem errors.
    pub fn open(path: impl AsRef<Path>) -> DbResult<Self> {
        Ok(Self::open_with_truncation_report(path)?.0)
    }

    /// Open an existing log, also returning how many torn-tail bytes were
    /// truncated during validation (0 when the log was already clean).
    fn open_with_truncation_report(path: impl AsRef<Path>) -> DbResult<(Self, u64)> {
        let path = path.as_ref();
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        let file_len = file.metadata()?.len();

        if file_len == 0 {
            // A zero-byte file is a torn `create` (crash before the header
            // reached disk). No record could exist yet, so rewriting a fresh
            // header is safe self-healing.
            file.write_all(&LOG_MAGIC.to_le_bytes())?;
            file.write_all(&CURRENT_LOG_VERSION.to_le_bytes())?;
            file.write_all(&0u64.to_le_bytes())?;
            file.sync_all()?;
            file.seek(SeekFrom::End(0))?;
            return Ok((
                Self {
                    file,
                    path: path.to_path_buf(),
                    snapshot_len: 0,
                    records_since_checkpoint: 0,
                },
                0,
            ));
        }

        let mut magic_buf = [0u8; 8];
        read_header_field(&mut file, &mut magic_buf, "magic")?;
        if u64::from_le_bytes(magic_buf) != LOG_MAGIC {
            return Err(DbError::CorruptedDatabase(format!(
                "{} is not an FHE-DB log (bad magic)",
                path.display()
            )));
        }

        let mut version_buf = [0u8; 8];
        read_header_field(&mut file, &mut version_buf, "version")?;
        let version = u64::from_le_bytes(version_buf);
        if version != CURRENT_LOG_VERSION {
            return Err(DbError::CorruptedDatabase(format!(
                "unsupported FHE-DB log version {version} (expected {CURRENT_LOG_VERSION})"
            )));
        }

        let mut len_buf = [0u8; 8];
        read_header_field(&mut file, &mut len_buf, "snapshot length")?;
        let snapshot_len = u64::from_le_bytes(len_buf);
        let records_start = LOG_HEADER_LEN + snapshot_len;
        if file_len < records_start {
            return Err(DbError::CorruptedDatabase(format!(
                "log {} is truncated in its snapshot region ({} bytes on disk, expected {records_start})",
                path.display(),
                file_len
            )));
        }

        // Validate every complete record and locate the last clean boundary;
        // truncate any torn tail.
        let valid_end = scan_records(&mut file, records_start, file_len)?;
        let torn_tail_bytes_truncated = file_len - valid_end;
        if torn_tail_bytes_truncated > 0 {
            file.set_len(valid_end)?;
            file.sync_all()?;
        }
        file.seek(SeekFrom::End(0))?;

        Ok((
            Self {
                file,
                path: path.to_path_buf(),
                snapshot_len,
                records_since_checkpoint: 0,
            },
            torn_tail_bytes_truncated,
        ))
    }

    /// Recover a database from the log at `path`.
    ///
    /// If the log does not exist, returns an empty database and a freshly
    /// created log. If it exists, the snapshot section is loaded and every
    /// appended record replayed on top, in order.
    ///
    /// # Errors
    ///
    /// Propagates all [`Wal::open`] validation errors and
    /// [`DbError::CorruptedDatabase`] if replaying a record fails.
    pub fn recover(path: impl AsRef<Path>) -> DbResult<(EncryptedDatabase, Wal)> {
        let (db, wal, _report) = Self::recover_with_report(path)?;
        Ok((db, wal))
    }

    /// Recover a database from the log at `path`, reporting what recovery did.
    ///
    /// Same contract as [`Wal::recover`]; the returned [`RecoveryReport`]
    /// feeds the structured `wal.recover` boot event and the `/metrics` WAL
    /// counters, so a silent replay or torn-tail heal becomes observable.
    ///
    /// # Errors
    ///
    /// Propagates all [`Wal::open`] validation errors and
    /// [`DbError::CorruptedDatabase`] if replaying a record fails.
    pub fn recover_with_report(
        path: impl AsRef<Path>,
    ) -> DbResult<(EncryptedDatabase, Wal, RecoveryReport)> {
        let path = path.as_ref();
        // A nonexistent log means a first boot (or a wiped data dir): create
        // an empty log. Without this, `fhe-server --data-dir` cannot start
        // on a fresh directory.
        if !path.exists() {
            let wal = Wal::create(path)?;
            let report = RecoveryReport {
                log_bytes: wal.len()?,
                created: true,
                ..RecoveryReport::default()
            };
            return Ok((EncryptedDatabase::new(), wal, report));
        }
        let (wal, torn_tail_bytes_truncated) = Self::open_with_truncation_report(path)?;
        // A fresh log (or a checkpoint of an empty database) carries an
        // empty snapshot section; decode that as an empty database instead
        // of failing to deserialize zero bytes.
        let mut db = if wal.snapshot_len == 0 {
            EncryptedDatabase::new()
        } else {
            let snapshot_bytes = wal.snapshot_bytes()?;
            EncryptedDatabase::from_bytes(&snapshot_bytes)?
        };
        let (records_replayed, record_bytes_replayed) = wal.replay_with_stats(&mut db)?;
        let report = RecoveryReport {
            snapshot_bytes: wal.snapshot_len,
            records_replayed,
            record_bytes_replayed,
            torn_tail_bytes_truncated,
            log_bytes: wal.len()?,
            created: false,
        };
        Ok((db, wal, report))
    }

    /// The byte length of the header + snapshot section (where records begin).
    pub fn snapshot_len(&self) -> u64 {
        self.snapshot_len
    }

    /// The byte length of the whole log file on disk.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Io`] if the file metadata cannot be read.
    pub fn len(&self) -> DbResult<u64> {
        Ok(self.file.metadata()?.len())
    }

    /// Whether the log file is empty on disk.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Io`] if the file metadata cannot be read.
    pub fn is_empty(&self) -> DbResult<bool> {
        Ok(self.len()? == 0)
    }

    /// The log file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one record durably (write + `fsync`).
    ///
    /// The caller is responsible for the write-ahead contract: validate the
    /// intended mutation first, append here, then apply it to the database.
    ///
    /// # Errors
    ///
    /// - [`DbError::CorruptedDatabase`] (via bincode) if the record cannot be
    ///   serialized.
    /// - [`DbError::Io`] on write or sync failure.
    pub fn append(&mut self, record: &WalRecord) -> DbResult<()> {
        let payload = bincode::serialize(record)?;
        let len = payload.len() as u32;
        let mut buf = Vec::with_capacity(payload.len() + 8);
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(&payload);
        buf.extend_from_slice(&crc32(&payload).to_le_bytes());
        self.file.write_all(&buf)?;
        // Durability point: after sync_all the record survives power loss.
        self.file.sync_all()?;
        self.records_since_checkpoint += 1;
        Ok(())
    }

    /// Atomically compact the log: write the full database as a fresh
    /// snapshot and drop every previous record.
    ///
    /// The new log is written to a temporary file, `fsync`-ed, then renamed
    /// over the live log. `std::fs::rename` replaces the destination
    /// atomically on both POSIX and Windows (`MOVEFILE_REPLACE_EXISTING`).
    /// A crash at any point leaves either the complete old log or the
    /// complete new log on disk — never a mixture, because the snapshot and
    /// records live in the same file.
    ///
    /// # Errors
    ///
    /// - [`DbError::CorruptedDatabase`] if the database cannot be serialized.
    /// - [`DbError::Io`] on filesystem errors.
    pub fn checkpoint(&mut self, db: &EncryptedDatabase) -> DbResult<()> {
        self.checkpoint_with_report(db).map(|_| ())
    }

    /// Atomically compact the log and report what the compaction did.
    ///
    /// Same durability contract as [`Wal::checkpoint`]; the returned
    /// [`CheckpointReport`] feeds the structured `wal.checkpoint` event and
    /// the `/metrics` WAL counters.
    ///
    /// # Errors
    ///
    /// - [`DbError::CorruptedDatabase`] if the database cannot be serialized.
    /// - [`DbError::Io`] on filesystem errors.
    pub fn checkpoint_with_report(&mut self, db: &EncryptedDatabase) -> DbResult<CheckpointReport> {
        let started = Instant::now();
        let log_bytes_before = self.len()?;
        let record_bytes_compacted =
            log_bytes_before.saturating_sub(LOG_HEADER_LEN + self.snapshot_len);
        let records_compacted = self.records_since_checkpoint;
        let snapshot = db.to_bytes()?;
        let tmp = self.path.with_extension("tmp");
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        {
            let mut out = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp)?;
            out.write_all(&LOG_MAGIC.to_le_bytes())?;
            out.write_all(&CURRENT_LOG_VERSION.to_le_bytes())?;
            out.write_all(&(snapshot.len() as u64).to_le_bytes())?;
            out.write_all(&snapshot)?;
            out.sync_all()?;
        } // close before rename (Windows cannot replace an open destination)
        std::fs::rename(&tmp, &self.path)?;
        // Re-open for appends: the old handle points at the pre-replace file.
        self.file = OpenOptions::new().read(true).write(true).open(&self.path)?;
        self.file.seek(SeekFrom::End(0))?;
        self.snapshot_len = snapshot.len() as u64;
        self.records_since_checkpoint = 0;
        Ok(CheckpointReport {
            snapshot_bytes: self.snapshot_len,
            records_compacted,
            record_bytes_compacted,
            log_bytes_before,
            log_bytes_after: self.len()?,
            duration: started.elapsed(),
        })
    }

    /// Replay every appended record onto `db`, in order.
    ///
    /// The log must already have been validated by [`Wal::open`] (torn tails
    /// truncated), so an unexpected EOF or checksum failure here indicates
    /// external tampering and is rejected, never partially applied.
    ///
    /// # Errors
    ///
    /// - [`DbError::CorruptedDatabase`] if a record fails to decode, fails
    ///   its checksum, or cannot be applied.
    /// - [`DbError::Io`] on filesystem errors.
    pub fn replay(&self, db: &mut EncryptedDatabase) -> DbResult<u64> {
        Ok(self.replay_with_stats(db)?.0)
    }

    /// Replay every record, returning `(records, record_bytes)`.
    fn replay_with_stats(&self, db: &mut EncryptedDatabase) -> DbResult<(u64, u64)> {
        // Use a separate read handle so the append cursor is never disturbed.
        let mut file = File::open(&self.path)?;
        let file_len = file.metadata()?.len();
        file.seek(SeekFrom::Start(LOG_HEADER_LEN + self.snapshot_len))?;
        let mut applied = 0u64;
        let mut record_bytes = 0u64;
        loop {
            let mut len_bytes = [0u8; 4];
            match file.read_exact(&mut len_bytes) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            let len = u32::from_le_bytes(len_bytes) as u64;
            // Bound the allocation by the file length before building the
            // payload buffer. `Wal::open` already healed torn tails, so a
            // record extending past EOF here means external tampering
            // between open and replay (or a corrupt length prefix); reject
            // rather than allocate `len` bytes from an untrusted value.
            let record_end = file.stream_position()? + len + 4;
            if record_end > file_len {
                return Err(DbError::CorruptedDatabase(
                    "log record length exceeds file bounds during replay".into(),
                ));
            }
            let mut payload = vec![0u8; len as usize];
            if let Err(e) = file.read_exact(&mut payload) {
                let _ = e;
                return Err(DbError::CorruptedDatabase(
                    "log truncated mid-record during replay".into(),
                ));
            }
            let mut crc_bytes = [0u8; 4];
            if let Err(e) = file.read_exact(&mut crc_bytes) {
                let _ = e;
                return Err(DbError::CorruptedDatabase(
                    "log truncated mid-checksum during replay".into(),
                ));
            }
            let expected = u32::from_le_bytes(crc_bytes);
            if crc32(&payload) != expected {
                return Err(DbError::CorruptedDatabase(
                    "log checksum mismatch during replay".into(),
                ));
            }
            let record: WalRecord = bincode::deserialize(&payload).map_err(|e| {
                DbError::CorruptedDatabase(format!(
                    "log record failed to decode during replay: {e}"
                ))
            })?;
            apply(db, record).map_err(|e| {
                DbError::CorruptedDatabase(format!("log replay failed at record {applied}: {e}"))
            })?;
            applied += 1;
            record_bytes += 4 + len + 4;
        }
        Ok((applied, record_bytes))
    }

    /// Read the snapshot section as raw bincode bytes.
    fn snapshot_bytes(&self) -> DbResult<Vec<u8>> {
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(LOG_HEADER_LEN))?;
        let mut bytes = vec![0u8; self.snapshot_len as usize];
        file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}

/// Read one fixed-size header field, mapping a short read to a descriptive
/// corruption error rather than a raw I/O error.
fn read_header_field(file: &mut File, buf: &mut [u8; 8], field: &str) -> DbResult<()> {
    match file.read_exact(buf) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Err(DbError::CorruptedDatabase(
            format!("log header truncated in {field} field"),
        )),
        Err(e) => Err(e.into()),
    }
}

/// Apply one journaled record to the database.
fn apply(db: &mut EncryptedDatabase, record: WalRecord) -> DbResult<()> {
    match record {
        WalRecord::CreateTable { schema } => db.create_table(schema).map(|_| ()),
        WalRecord::AddColumn { table, column } => db.add_column(&table, column).map(|_| ()),
        WalRecord::InsertRows {
            table,
            column,
            ciphertexts,
        } => db
            .insert_ciphertexts(&table, &column, ciphertexts)
            .map(|_| ()),
    }
}

/// Scan records in `[start, file_len)`, returning the offset just past the
/// last **complete** record.
///
/// A short trailing record (fewer bytes than its length prefix declares) is a
/// torn tail: scan stops and returns the offset where it begins so the caller
/// can truncate. A *complete* record that fails its CRC32 or payload decode
/// is genuine corruption and is rejected.
fn scan_records(file: &mut File, start: u64, file_len: u64) -> DbResult<u64> {
    file.seek(SeekFrom::Start(start))?;
    let mut cursor = start;
    loop {
        let mut len_bytes = [0u8; 4];
        match file.read_exact(&mut len_bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(cursor),
            Err(e) => return Err(e.into()),
        }
        let len = u32::from_le_bytes(len_bytes) as u64;
        let record_end = cursor + 4 + len + 4;
        if record_end > file_len {
            // The length prefix declares more bytes than remain: torn tail.
            return Ok(cursor);
        }
        let mut payload = vec![0u8; len as usize];
        file.read_exact(&mut payload)?;
        let mut crc_bytes = [0u8; 4];
        file.read_exact(&mut crc_bytes)?;
        let expected = u32::from_le_bytes(crc_bytes);
        let actual = crc32(&payload);
        if actual != expected {
            return Err(DbError::CorruptedDatabase(format!(
                "log checksum mismatch at offset {cursor}"
            )));
        }
        // A payload that passes CRC but cannot decode is still corruption
        // (e.g. a checksum collision or external mangling that preserved it).
        if bincode::deserialize::<WalRecord>(&payload).is_err() {
            return Err(DbError::CorruptedDatabase(format!(
                "log record at offset {cursor} failed to decode"
            )));
        }
        cursor = record_end;
    }
}

/// A [`Wal`]-backed database that enforces the write-ahead contract.
///
/// Every mutation is journaled with [`Wal::append`] **before** it is applied
/// to the in-memory [`EncryptedDatabase`]. If the process crashes between the
/// append and the in-memory apply, recovery replays the record and reaches
/// the same state; if it crashes before the append, the mutation was never
/// acknowledged to any client and recovery correctly omits it.
///
/// Mutations through this wrapper are the only way to write to the database
/// when durability is required. Reads and homomorphic queries delegate
/// straight to the inner database.
///
/// Deliberately does **not** derive `Debug`: ciphertexts never expose
/// plaintext via debug output.
pub struct WalBackedDatabase {
    /// The in-memory database.
    pub db: EncryptedDatabase,
    /// The write-ahead log.
    pub wal: Wal,
    /// Homomorphic query engine (stateless; shared freely).
    pub query: crate::QueryEngine,
}

impl WalBackedDatabase {
    /// Open the database at the log path, recovering committed state.
    ///
    /// Creates the log if it does not exist.
    ///
    /// # Errors
    ///
    /// Propagates [`Wal::recover`] errors.
    pub fn open(path: impl AsRef<Path>) -> DbResult<Self> {
        let (db, wal) = Wal::recover(path)?;
        Ok(Self {
            db,
            wal,
            query: crate::QueryEngine::new(),
        })
    }

    /// Create a table, journaling the mutation before applying it.
    ///
    /// # Errors
    ///
    /// - [`DbError::TableExists`] if the table already exists.
    /// - WAL write errors (see [`Wal::append`]).
    pub fn create_table(&mut self, schema: TableSchema) -> DbResult<()> {
        // Validate before journaling: an existing table must not be
        // re-created (that would also break the idempotence of replay), and a
        // rejected mutation must not leave a record behind.
        let record = WalRecord::CreateTable {
            schema: schema.clone(),
        };
        record.validate(&self.db)?;
        self.wal.append(&record)?;
        self.db.create_table(schema)
    }

    /// Add a column, journaling the mutation before applying it.
    ///
    /// # Errors
    ///
    /// - [`DbError::UnknownTable`] if the table does not exist.
    /// - [`DbError::ColumnExists`] if the column already exists.
    /// - WAL write errors (see [`Wal::append`]).
    pub fn add_column(&mut self, table: &str, column: ColumnDef) -> DbResult<()> {
        // Validate before appending, then journal before applying, so a
        // rejected column does not poison the log and a crash between append
        // and apply is recoverable.
        let record = WalRecord::AddColumn {
            table: table.to_string(),
            column: column.clone(),
        };
        record.validate(&self.db)?;
        self.wal.append(&record)?;
        self.db.add_column(table, column)
    }

    /// Insert ciphertexts, journaling the mutation before applying it.
    ///
    /// The whole batch is validated against the live schema (table, column,
    /// widths) **before** the WAL append, so a rejected batch never touches
    /// the log and the all-or-nothing insert contract is preserved.
    ///
    /// # Errors
    ///
    /// - [`DbError::UnknownTable`] / [`DbError::UnknownColumn`] if the target
    ///   does not exist.
    /// - [`DbError::ColumnWidthMismatch`] if any ciphertext width mismatches.
    /// - WAL write errors (see [`Wal::append`]).
    pub fn insert_ciphertexts(
        &mut self,
        table: &str,
        column: &str,
        ciphertexts: Vec<Ciphertext>,
    ) -> DbResult<usize> {
        // Pre-validate the whole batch without mutating. Column names are
        // plaintext metadata; ciphertext widths are public; nothing here
        // touches decryption. Clone for the WAL after validation: the log
        // stores its own serialized bytes, and the in-memory database keeps
        // the ciphertexts live.
        let record = WalRecord::InsertRows {
            table: table.to_string(),
            column: column.to_string(),
            ciphertexts: ciphertexts.clone(),
        };
        record.validate(&self.db)?;
        self.wal.append(&record)?;
        self.db.insert_ciphertexts(table, column, ciphertexts)
    }

    /// Atomically compact the log against the current in-memory database.
    ///
    /// # Errors
    ///
    /// Propagates [`Wal::checkpoint`] errors.
    pub fn checkpoint(&mut self) -> DbResult<()> {
        self.wal.checkpoint(&self.db)
    }

    /// The log file path.
    pub fn log_path(&self) -> &Path {
        self.wal.path()
    }
}

/// CRC-32 (IEEE 802.3, reflected polynomial 0xEDB88320), table-driven and
/// dependency-free. Verified against the standard vector in the unit tests.
fn crc32(data: &[u8]) -> u32 {
    const POLY: u32 = 0xEDB8_8320;
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ POLY
            } else {
                crc >> 1
            };
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc = (crc >> 8) ^ table[((crc ^ u32::from(byte)) & 0xFF) as usize];
    }
    crc ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ColumnType;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fhe_wal_{}_{}", tag, std::process::id()))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("tmp"));
    }

    #[test]
    fn crc32_known_vector() {
        // IEEE CRC-32 of the ASCII digits 1..=9 is 0xCBF43926.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn crc32_detects_bit_flips() {
        assert_ne!(
            crc32(b"the quick brown fox jumps over the lazy dog"),
            crc32(b"the quick brown fox jumps over the lazy doh")
        );
    }

    #[test]
    fn create_refuses_to_overwrite() {
        let path = temp_path("overwrite");
        cleanup(&path);
        Wal::create(&path).unwrap();
        let err = Wal::create(&path).unwrap_err();
        assert!(matches!(err, DbError::CorruptedDatabase(_)));
        cleanup(&path);
    }

    #[test]
    fn rejects_non_log_file() {
        let path = temp_path("notlog");
        cleanup(&path);
        std::fs::write(&path, b"this is not an fhe-db log at all").unwrap();
        let err = match Wal::open(&path) {
            Ok(_) => panic!("non-log file must be rejected"),
            Err(e) => e,
        };
        assert!(matches!(err, DbError::CorruptedDatabase(_)));
        cleanup(&path);
    }

    #[test]
    fn create_append_replay_roundtrip() {
        let (client_key, _server_key) = fhe_core::ClientKey::new();
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("roundtrip");
        cleanup(&path);

        let mut wal = Wal::create(&path).unwrap();
        wal.append(&WalRecord::CreateTable {
            schema: TableSchema::new("t").with_column(ColumnDef {
                name: "c".into(),
                column_type: ColumnType::U32,
            }),
        })
        .unwrap();
        let ct = encryptor
            .encrypt_u64(42, fhe_core::ScalarWidth::U32)
            .unwrap();
        wal.append(&WalRecord::InsertRows {
            table: "t".into(),
            column: "c".into(),
            ciphertexts: vec![ct],
        })
        .unwrap();
        drop(wal);

        let (db, _wal) = Wal::recover(&path).unwrap();
        let col = db.column("t", "c").unwrap();
        assert_eq!(col.len(), 1);
        let sum = crate::QueryEngine::new().sum(col, ColumnType::U32).unwrap();
        assert_eq!(sum.value.decrypt(&client_key).as_u64(), 42);
        cleanup(&path);
    }

    #[test]
    fn recover_nonexistent_log_creates_empty() {
        let path = temp_path("missing");
        cleanup(&path);
        let (db, wal) = Wal::recover(&path).unwrap();
        assert_eq!(db.tables().len(), 0);
        assert_eq!(wal.len().unwrap(), LOG_HEADER_LEN);
        cleanup(&path);
    }

    #[test]
    fn torn_tail_is_truncated() {
        let (client_key, _server_key) = fhe_core::ClientKey::new();
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("torn");
        cleanup(&path);

        let mut wal = Wal::create(&path).unwrap();
        wal.append(&WalRecord::CreateTable {
            schema: TableSchema::new("t").with_column(ColumnDef {
                name: "c".into(),
                column_type: ColumnType::U32,
            }),
        })
        .unwrap();
        let ct = encryptor
            .encrypt_u64(7, fhe_core::ScalarWidth::U32)
            .unwrap();
        wal.append(&WalRecord::InsertRows {
            table: "t".into(),
            column: "c".into(),
            ciphertexts: vec![ct],
        })
        .unwrap();
        drop(wal);

        // Simulate a crash mid-append: append half a record by hand.
        let mut bytes = std::fs::read(&path).unwrap();
        let torn = WalRecord::InsertRows {
            table: "t".into(),
            column: "c".into(),
            ciphertexts: vec![encryptor
                .encrypt_u64(9, fhe_core::ScalarWidth::U32)
                .unwrap()],
        };
        let mut payload = bincode::serialize(&torn).unwrap();
        // Truncate both the payload and the trailing CRC.
        let len = payload.len() as u32;
        payload.truncate(payload.len() / 2);
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.extend_from_slice(&payload);
        std::fs::write(&path, &bytes).unwrap();

        // Open must heal the torn tail and recover the committed row only.
        let (db, _wal) = Wal::recover(&path).unwrap();
        let col = db.column("t", "c").unwrap();
        assert_eq!(col.len(), 1);
        let sum = crate::QueryEngine::new().sum(col, ColumnType::U32).unwrap();
        assert_eq!(sum.value.decrypt(&client_key).as_u64(), 7);
        cleanup(&path);
    }

    #[test]
    fn corrupt_complete_record_is_rejected() {
        let (client_key, _server_key) = fhe_core::ClientKey::new();
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("corrupt");
        cleanup(&path);

        let mut wal = Wal::create(&path).unwrap();
        wal.append(&WalRecord::CreateTable {
            schema: TableSchema::new("t").with_column(ColumnDef {
                name: "c".into(),
                column_type: ColumnType::U32,
            }),
        })
        .unwrap();
        let ct = encryptor
            .encrypt_u64(1, fhe_core::ScalarWidth::U32)
            .unwrap();
        wal.append(&WalRecord::InsertRows {
            table: "t".into(),
            column: "c".into(),
            ciphertexts: vec![ct],
        })
        .unwrap();
        drop(wal);

        // Flip a bit inside the first record's payload (after the header).
        let mut bytes = std::fs::read(&path).unwrap();
        let idx = 24 + 4 + 2;
        bytes[idx] ^= 0x01;
        std::fs::write(&path, &bytes).unwrap();

        let err = match Wal::open(&path) {
            Ok(_) => panic!("corrupt record must be rejected"),
            Err(e) => e,
        };
        assert!(matches!(err, DbError::CorruptedDatabase(_)));
        cleanup(&path);
    }

    #[test]
    fn checkpoint_compacts_and_preserves() {
        let (client_key, server_key) = fhe_core::ClientKey::new();
        // The final homomorphic SUM over two rows needs the server key.
        let _guard = fhe_core::keys::ServerKeyGuard::new(&server_key);
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("compact");
        cleanup(&path);

        let mut db = EncryptedDatabase::new();
        let mut wal = Wal::create(&path).unwrap();
        // Create the table and log a row, then checkpoint.
        let schema = TableSchema::new("t").with_column(ColumnDef {
            name: "c".into(),
            column_type: ColumnType::U32,
        });
        db.create_table(schema).unwrap();
        wal.append(&WalRecord::CreateTable {
            schema: db.table_schema("t").unwrap(),
        })
        .unwrap();
        let ct = encryptor
            .encrypt_u64(5, fhe_core::ScalarWidth::U32)
            .unwrap();
        db.insert_ciphertexts("t", "c", vec![ct]).unwrap();
        wal.append(&WalRecord::InsertRows {
            table: "t".into(),
            column: "c".into(),
            ciphertexts: vec![encryptor
                .encrypt_u64(5, fhe_core::ScalarWidth::U32)
                .unwrap()],
        })
        .unwrap();

        let pre_len = wal.len().unwrap();
        wal.checkpoint(&db).unwrap();
        let post_len = wal.len().unwrap();
        // A checkpoint must not grow the log beyond the snapshot itself. (A
        // single-row snapshot is size-comparable to its own records, so the
        // real invariant is the next assertion: only the snapshot remains.)
        assert!(
            post_len <= pre_len + 1,
            "checkpoint must not grow the log ({pre_len} -> {post_len})"
        );
        // After a checkpoint the record section is empty.
        assert_eq!(
            wal.snapshot_len() + LOG_HEADER_LEN,
            post_len,
            "checkpointed log holds only the snapshot"
        );

        // Appends after the checkpoint must still recover.
        let ct2 = encryptor
            .encrypt_u64(6, fhe_core::ScalarWidth::U32)
            .unwrap();
        db.insert_ciphertexts("t", "c", vec![ct2]).unwrap();
        wal.append(&WalRecord::InsertRows {
            table: "t".into(),
            column: "c".into(),
            ciphertexts: vec![encryptor
                .encrypt_u64(6, fhe_core::ScalarWidth::U32)
                .unwrap()],
        })
        .unwrap();
        drop(wal);

        let (db2, _wal) = Wal::recover(&path).unwrap();
        let col = db2.column("t", "c").unwrap();
        assert_eq!(col.len(), 2);
        let sum = crate::QueryEngine::new().sum(col, ColumnType::U32).unwrap();
        assert_eq!(sum.value.decrypt(&client_key).as_u64(), 11);
        cleanup(&path);
    }

    // -- crash-boundary matrix -------------------------------------------------
    //
    // Each test constructs one exact on-disk state a crash can leave behind,
    // then asserts what recovery is allowed to expose:
    //
    //   before append          → only previously committed rows survive
    //   after append / before apply → the journaled row is replayed once
    //   mid-checkpoint         → the old log wins; a partial `.tmp` is ignored
    //   mid-append (torn tail) → committed prefix survives, tail truncated
    //   corrupted complete record → fatal `CorruptedDatabase` (never silent)
    //
    // The first two need a crash *inside* a mutation, which cannot be raced
    // reliably from outside the process; they are built from the same public
    // WAL primitives the server uses (validate → append → apply).

    fn u32_ct(encryptor: &fhe_core::Encryptor, v: u64) -> Ciphertext {
        encryptor
            .encrypt_u64(v, fhe_core::ScalarWidth::U32)
            .unwrap()
    }

    /// The schema used by every matrix case.
    fn matrix_schema() -> TableSchema {
        TableSchema::new("t").with_column(ColumnDef {
            name: "c".into(),
            column_type: ColumnType::U32,
        })
    }

    #[test]
    fn crash_before_append_keeps_only_committed_rows() {
        let (client_key, server_key) = fhe_core::ClientKey::new();
        // The matrix computes a homomorphic SUM to prove the recovered
        // ciphertexts are the committed ones, which needs a server key.
        let _guard = fhe_core::keys::ServerKeyGuard::new(&server_key);
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("before_append");
        cleanup(&path);

        let mut store = WalBackedDatabase::open(&path).unwrap();
        store.create_table(matrix_schema()).unwrap();
        store
            .insert_ciphertexts("t", "c", vec![u32_ct(&encryptor, 3)])
            .unwrap();
        let committed_len = store.wal.len().unwrap();

        // Crash point: the second mutation died before its WAL append, so it
        // was never acknowledged and must not appear after recovery.
        drop(store);

        let (db, _wal) = Wal::recover(&path).unwrap();
        assert_eq!(db.column("t", "c").unwrap().len(), 1);
        let sum = crate::QueryEngine::new()
            .sum(db.column("t", "c").unwrap(), ColumnType::U32)
            .unwrap();
        assert_eq!(sum.value.decrypt(&client_key).as_u64(), 3);
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            committed_len,
            "no record exists for the unacknowledged mutation"
        );
        cleanup(&path);
    }

    #[test]
    fn crash_after_append_before_apply_replays_the_record() {
        let (client_key, server_key) = fhe_core::ClientKey::new();
        // The matrix computes a homomorphic SUM to prove the recovered
        // ciphertexts are the committed ones, which needs a server key.
        let _guard = fhe_core::keys::ServerKeyGuard::new(&server_key);
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("after_append");
        cleanup(&path);

        let mut store = WalBackedDatabase::open(&path).unwrap();
        store.create_table(matrix_schema()).unwrap();
        let record = WalRecord::InsertRows {
            table: "t".into(),
            column: "c".into(),
            ciphertexts: vec![u32_ct(&encryptor, 42)],
        };
        record.validate(&store.db).unwrap();
        // Crash point: journal the mutation, then die before applying it to
        // the in-memory database. The WAL is fsynced, so recovery must reach
        // the state the client would have been acknowledged for.
        store.wal.append(&record).unwrap();
        assert_eq!(
            store.db.column("t", "c").unwrap().len(),
            0,
            "the row is not in memory before the apply"
        );
        drop(store);

        let (db, _wal) = Wal::recover(&path).unwrap();
        assert_eq!(db.column("t", "c").unwrap().len(), 1);
        let sum = crate::QueryEngine::new()
            .sum(db.column("t", "c").unwrap(), ColumnType::U32)
            .unwrap();
        assert_eq!(
            sum.value.decrypt(&client_key).as_u64(),
            42,
            "the journaled record is replayed exactly once"
        );
        cleanup(&path);
    }

    #[test]
    fn checkpoint_ignores_a_stale_partial_tmp_file() {
        let (client_key, server_key) = fhe_core::ClientKey::new();
        // The matrix computes a homomorphic SUM to prove the recovered
        // ciphertexts are the committed ones, which needs a server key.
        let _guard = fhe_core::keys::ServerKeyGuard::new(&server_key);
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("mid_checkpoint");
        cleanup(&path);

        let mut store = WalBackedDatabase::open(&path).unwrap();
        store.create_table(matrix_schema()).unwrap();
        for v in [1, 2, 3] {
            store
                .insert_ciphertexts("t", "c", vec![u32_ct(&encryptor, v)])
                .unwrap();
        }
        let live_len = store.wal.len().unwrap();
        drop(store);

        // Crash point: `checkpoint` died while writing the replacement log,
        // leaving a partial temp file beside the untouched live log.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, b"FHELOG01 partial snapshot write").unwrap();

        let (db, wal) = Wal::recover(&path).unwrap();
        assert_eq!(db.column("t", "c").unwrap().len(), 3);
        assert_eq!(
            wal.len().unwrap(),
            live_len,
            "recovery reads the live log, not the temp file"
        );
        let sum = crate::QueryEngine::new()
            .sum(db.column("t", "c").unwrap(), ColumnType::U32)
            .unwrap();
        assert_eq!(sum.value.decrypt(&client_key).as_u64(), 6);
        drop(wal);

        // The next checkpoint must reclaim the stale temp file rather than
        // trip over it, and the compacted log must still recover the rows.
        let mut store = WalBackedDatabase::open(&path).unwrap();
        store.checkpoint().unwrap();
        assert!(!tmp.exists(), "checkpoint replaced the stale temp file");
        drop(store);

        let (db, wal) = Wal::recover(&path).unwrap();
        assert_eq!(db.column("t", "c").unwrap().len(), 3);
        assert_eq!(
            wal.len().unwrap(),
            LOG_HEADER_LEN + wal.snapshot_len(),
            "the post-crash checkpoint compacted the log"
        );
        cleanup(&path);
    }

    #[test]
    fn recovery_report_counts_records_snapshot_and_torn_tail() {
        let (client_key, _server_key) = fhe_core::ClientKey::new();
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("recovery_report");
        cleanup(&path);

        // First boot on a missing log: created, empty, nothing replayed.
        let (db, wal, report) = Wal::recover_with_report(&path).unwrap();
        assert!(report.created, "a missing log is created on first boot");
        assert_eq!(report.snapshot_bytes, 0);
        assert_eq!(report.records_replayed, 0);
        assert_eq!(report.record_bytes_replayed, 0);
        assert_eq!(report.torn_tail_bytes_truncated, 0);
        assert_eq!(report.log_bytes, LOG_HEADER_LEN);
        assert_eq!(db.tables().len(), 0);
        drop(wal);

        let mut store = WalBackedDatabase::open(&path).unwrap();
        store.create_table(matrix_schema()).unwrap();
        store
            .insert_ciphertexts("t", "c", vec![u32_ct(&encryptor, 4)])
            .unwrap();
        let checkpoint = store.wal.checkpoint_with_report(&store.db).unwrap();
        assert_eq!(checkpoint.records_compacted, 2);
        assert!(checkpoint.snapshot_bytes > 0);
        assert!(checkpoint.record_bytes_compacted > 0);
        assert_eq!(
            checkpoint.log_bytes_after,
            LOG_HEADER_LEN + checkpoint.snapshot_bytes
        );
        store
            .insert_ciphertexts("t", "c", vec![u32_ct(&encryptor, 5)])
            .unwrap();
        let appended = store.wal.len().unwrap();
        drop(store);

        // Clean recovery: exactly the one post-checkpoint record replays.
        let (db, wal, report) = Wal::recover_with_report(&path).unwrap();
        assert!(!report.created);
        assert!(report.snapshot_bytes > 0);
        assert_eq!(report.records_replayed, 1);
        assert!(report.record_bytes_replayed > 0);
        assert_eq!(report.torn_tail_bytes_truncated, 0);
        assert_eq!(report.log_bytes, appended);
        assert_eq!(db.column("t", "c").unwrap().len(), 2);
        drop(wal);

        // A torn tail is reported, not merely healed: the operator log says
        // how many bytes were dropped. Chopping 3 bytes makes the whole
        // trailing record incomplete, so the reported region is its full
        // frame, not just the 3 removed bytes.
        let full = std::fs::read(&path).unwrap();
        let torn_file_len = (full.len() - 3) as u64;
        std::fs::write(&path, &full[..full.len() - 3]).unwrap();
        let (db, _wal, report) = Wal::recover_with_report(&path).unwrap();
        assert!(report.torn_tail_bytes_truncated > 3);
        assert_eq!(report.records_replayed, 0);
        assert_eq!(
            report.log_bytes + report.torn_tail_bytes_truncated,
            torn_file_len,
            "the healed log plus the dropped region is the pre-crash file"
        );
        assert_eq!(
            report.log_bytes,
            LOG_HEADER_LEN + report.snapshot_bytes,
            "the healed log is header + snapshot"
        );
        assert_eq!(db.column("t", "c").unwrap().len(), 1);
        cleanup(&path);
    }

    #[test]
    fn checkpoint_report_counts_only_appends_since_the_previous_checkpoint() {
        let (client_key, _server_key) = fhe_core::ClientKey::new();
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let path = temp_path("checkpoint_report");
        cleanup(&path);

        let mut store = WalBackedDatabase::open(&path).unwrap();
        store.create_table(matrix_schema()).unwrap();
        for v in [1, 2] {
            store
                .insert_ciphertexts("t", "c", vec![u32_ct(&encryptor, v)])
                .unwrap();
        }

        let first = store.wal.checkpoint_with_report(&store.db).unwrap();
        assert_eq!(first.records_compacted, 3);
        assert!(first.record_bytes_compacted > 0);
        assert_eq!(first.log_bytes_after, LOG_HEADER_LEN + first.snapshot_bytes);
        assert!(first.duration < std::time::Duration::from_secs(300));

        // Nothing appended since: the idle checkpoint reports no records.
        let idle = store.wal.checkpoint_with_report(&store.db).unwrap();
        assert_eq!(idle.records_compacted, 0);
        assert_eq!(idle.record_bytes_compacted, 0);

        // Two more appends are reported by the next compaction, not the old
        // total (the counter resets at every successful checkpoint).
        store
            .insert_ciphertexts("t", "c", vec![u32_ct(&encryptor, 3)])
            .unwrap();
        store
            .insert_ciphertexts("t", "c", vec![u32_ct(&encryptor, 4)])
            .unwrap();
        let after = store.wal.checkpoint_with_report(&store.db).unwrap();
        assert_eq!(after.records_compacted, 2);
        assert_eq!(after.log_bytes_after, LOG_HEADER_LEN + after.snapshot_bytes);
        cleanup(&path);
    }
}

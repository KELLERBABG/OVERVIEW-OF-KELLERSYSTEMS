//! Live wire tap records for the Monitor app.
//!
//! Matched-route request and response metadata that crosses the loopback
//! boundary between the [`fhe_client`] SDK and the embedded [`fhe_server`] is
//! recorded here for the phone's Monitor app. The tap records only method,
//! path, and status: it does not capture bodies, sizes, or unmatched routes,
//! and `try_send` drops records under load. It is a traffic view, not proof
//! about payload contents.

use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use serde::Serialize;

/// Maximum number of wire records retained by the ring buffer.
pub const WIRE_CAPACITY: usize = 600;

/// A single observed HTTP event on the wire.
#[derive(Debug, Clone, Serialize)]
pub struct WireRecord {
    /// Monotonic-ish label (wall clock, seconds since epoch).
    pub at_ms: u128,
    /// `"req"` for a request line, `"res"` for a response line.
    pub direction: &'static str,
    /// For requests: HTTP method. For responses: status code.
    pub method: String,
    /// For requests: request path. For responses: status text.
    pub path: String,
}

impl WireRecord {
    /// Record a request line (`METHOD path`).
    pub fn request(method: &str, path: &str) -> Self {
        Self {
            at_ms: now_ms(),
            direction: "req",
            method: method.to_string(),
            path: path.to_string(),
        }
    }

    /// Record a response line (`HTTP status`).
    pub fn response(status: u16) -> Self {
        Self {
            at_ms: now_ms(),
            direction: "res",
            method: status.to_string(),
            path: status_text(status).to_string(),
        }
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        409 => "Conflict",
        415 => "Unsupported Media Type",
        _ => "HTTP",
    }
}

/// A shared, bounded ring buffer of wire records.
///
/// The embedded server's middleware appends records; the Monitor app reads
/// [`WireRing::snapshot`] over IPC.
#[derive(Clone, Default)]
pub struct WireRing {
    inner: Arc<Mutex<std::collections::VecDeque<WireRecord>>>,
}

impl WireRing {
    /// Create a new empty ring.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a record, trimming to [`WIRE_CAPACITY`].
    pub fn push(&self, record: WireRecord) {
        let mut inner = self.inner.lock().expect("wire ring lock");
        inner.push_back(record);
        while inner.len() > WIRE_CAPACITY {
            inner.pop_front();
        }
    }

    /// Snapshot the current records, oldest first.
    pub fn snapshot(&self) -> Vec<WireRecord> {
        self.inner
            .lock()
            .expect("wire ring lock")
            .iter()
            .cloned()
            .collect()
    }

    /// Whether the ring is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().expect("wire ring lock").is_empty()
    }

    /// Number of records currently retained.
    pub fn len(&self) -> usize {
        self.inner.lock().expect("wire ring lock").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_append_in_order() {
        let ring = WireRing::new();
        ring.push(WireRecord::request("POST", "/keys/server"));
        ring.push(WireRecord::response(204));
        ring.push(WireRecord::request("GET", "/tables"));

        let snap = ring.snapshot();
        assert_eq!(snap.len(), 3);
        assert_eq!(snap[0].direction, "req");
        assert_eq!(snap[0].method, "POST");
        assert_eq!(snap[0].path, "/keys/server");
        assert_eq!(snap[1].direction, "res");
        assert_eq!(snap[1].method, "204");
        assert_eq!(ring.len(), 3);
        assert!(!ring.is_empty());
    }

    #[test]
    fn ring_is_bounded() {
        let ring = WireRing::new();
        for i in 0..(WIRE_CAPACITY + 100) {
            ring.push(WireRecord::request("GET", &format!("/t/{i}")));
        }
        // Oldest records are evicted; the ring never exceeds the cap.
        assert_eq!(ring.len(), WIRE_CAPACITY);
        let snap = ring.snapshot();
        assert_eq!(snap.len(), WIRE_CAPACITY);
        // The oldest retained record is the one we inserted at index 100.
        assert_eq!(snap[0].path, "/t/100");
    }
}

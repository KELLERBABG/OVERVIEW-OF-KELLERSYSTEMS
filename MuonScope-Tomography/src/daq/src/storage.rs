//! Event Stream Persistence & Serialization

use std::fs::File;
use std::io::{BufWriter, Write};
use crate::CorrelatedEvent;

/// Writes JSON Lines stream of reconstructed events for logging and archival.
pub struct EventFileWriter {
    writer: BufWriter<File>,
}

impl EventFileWriter {
    pub fn create(path: &str) -> std::io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    pub fn write_event(&mut self, event: &CorrelatedEvent) -> std::io::Result<()> {
        let json = serde_json::to_string(event).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        writeln!(self.writer, "{}", json)?;
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

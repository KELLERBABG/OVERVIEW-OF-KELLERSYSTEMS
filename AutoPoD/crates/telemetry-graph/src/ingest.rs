use std::path::Path;

use csv::ReaderBuilder;
use pod_core::error::CoreError;
use pod_core::sensor::{SensorId, SensorReading, SensorType};

use crate::TelemetryGraph;

#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    #[error("Missing field: {0}")]
    MissingField(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

pub struct CsvIngestionConfig {
    pub sensor_id: SensorId,
    pub sensor_type: SensorType,
    pub timestamp_col: String,
    pub value_col: String,
    pub unit: String,
}

pub fn ingest_csv<P: AsRef<Path>>(
    path: P,
    config: CsvIngestionConfig,
    graph: &mut TelemetryGraph,
) -> Result<usize, IngestionError> {
    let mut reader = ReaderBuilder::new().has_headers(true).from_path(path)?;

    let headers = reader.headers()?.clone();
    let ts_idx = headers
        .iter()
        .position(|h| h == config.timestamp_col)
        .ok_or_else(|| IngestionError::MissingField(config.timestamp_col.clone()))?;
    let val_idx = headers
        .iter()
        .position(|h| h == config.value_col)
        .ok_or_else(|| IngestionError::MissingField(config.value_col.clone()))?;

    let mut count = 0;

    for result in reader.records() {
        let record = result?;
        let ts_str = record
            .get(ts_idx)
            .ok_or_else(|| IngestionError::MissingField("timestamp row".into()))?;
        let val_str = record
            .get(val_idx)
            .ok_or_else(|| IngestionError::MissingField("value row".into()))?;

        let timestamp: i64 = ts_str
            .parse()
            .map_err(|e| IngestionError::Parse(format!("timestamp: {e}")))?;
        let value: f64 = val_str
            .parse()
            .map_err(|e| IngestionError::Parse(format!("value: {e}")))?;

        let reading = SensorReading {
            sensor_id: config.sensor_id,
            sensor_type: config.sensor_type.clone(),
            timestamp,
            value,
            unit: config.unit.clone(),
        };

        graph.add_or_update_sensor(config.sensor_id, config.sensor_type.clone(), reading);
        count += 1;
    }

    Ok(count)
}

use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use pod_core::sensor::{SensorId, SensorReading, SensorType};
use serde::{Deserialize, Serialize};

pub type TelemetryNode = NodeIndex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEdge {
    pub relationship: &'static str,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct TelemetryGraph {
    graph: DiGraph<GraphNode, TelemetryEdge>,
    sensor_index: HashMap<SensorId, TelemetryNode>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GraphNode {
    sensor_id: SensorId,
    sensor_type: SensorType,
    readings: Vec<SensorReading>,
}

impl TelemetryGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            sensor_index: HashMap::new(),
        }
    }

    pub fn add_or_update_sensor(
        &mut self,
        sensor_id: SensorId,
        sensor_type: SensorType,
        reading: SensorReading,
    ) -> TelemetryNode {
        if let Some(&node) = self.sensor_index.get(&sensor_id) {
            if let Some(data) = self.graph.node_weight_mut(node) {
                data.readings.push(reading);
            }
            node
        } else {
            let node = self.graph.add_node(GraphNode {
                sensor_id,
                sensor_type,
                readings: vec![reading],
            });
            self.sensor_index.insert(sensor_id, node);
            node
        }
    }

    pub fn add_edge(
        &mut self,
        from: TelemetryNode,
        to: TelemetryNode,
        relationship: &'static str,
        weight: f64,
    ) {
        self.graph.add_edge(from, to, TelemetryEdge { relationship, weight });
    }

    pub fn get_readings(&self, sensor_id: &SensorId) -> Option<&[SensorReading]> {
        self.sensor_index
            .get(sensor_id)
            .and_then(|&node| self.graph.node_weight(node))
            .map(|data| data.readings.as_slice())
    }

    pub fn readings_in_window(
        &self,
        sensor_id: &SensorId,
        start_ts: i64,
        end_ts: i64,
    ) -> Vec<&SensorReading> {
        self.sensor_index
            .get(sensor_id)
            .and_then(|&node| self.graph.node_weight(node))
            .map(|data| {
                data.readings
                    .iter()
                    .filter(|r| r.timestamp >= start_ts && r.timestamp <= end_ts)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn aggregate_window(
        &self,
        sensor_id: &SensorId,
        start_ts: i64,
        end_ts: i64,
    ) -> Option<WindowAggregation> {
        let in_window = self.readings_in_window(sensor_id, start_ts, end_ts);
        if in_window.is_empty() {
            return None;
        }
        let values: Vec<f64> = in_window.iter().map(|r| r.value).collect();
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = values.iter().sum();
        let mean = sum / values.len() as f64;
        let count = values.len();

        Some(WindowAggregation {
            min,
            max,
            mean,
            count,
            duration_ms: end_ts - start_ts,
        })
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn all_sensor_ids(&self) -> Vec<SensorId> {
        self.sensor_index.keys().copied().collect()
    }
}

#[derive(Debug, Clone)]
pub struct WindowAggregation {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub count: usize,
    pub duration_ms: i64,
}

impl Default for TelemetryGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn reading(sid: SensorId, ts: i64, val: f64) -> SensorReading {
        SensorReading {
            sensor_id: sid,
            sensor_type: SensorType::Accelerometer,
            timestamp: ts,
            value: val,
            unit: "g".into(),
        }
    }

    #[test]
    fn test_add_sensor_creates_node() {
        let mut g = TelemetryGraph::new();
        let sid = Uuid::new_v4();
        let _node = g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 0, 1.0));
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.get_readings(&sid).unwrap().len(), 1);
    }

    #[test]
    fn test_add_reading_to_existing_sensor() {
        let mut g = TelemetryGraph::new();
        let sid = Uuid::nil();
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 0, 1.0));
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 100, 2.0));
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.get_readings(&sid).unwrap().len(), 2);
    }

    #[test]
    fn test_multiple_sensors_independent() {
        let mut g = TelemetryGraph::new();
        let sid1 = Uuid::new_v4();
        let sid2 = Uuid::new_v4();
        g.add_or_update_sensor(sid1, SensorType::Accelerometer, reading(sid1, 0, 1.0));
        g.add_or_update_sensor(sid2, SensorType::Speedometer, reading(sid2, 0, 50.0));
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.get_readings(&sid1).unwrap().len(), 1);
        assert_eq!(g.get_readings(&sid2).unwrap().len(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut g = TelemetryGraph::new();
        let sid1 = Uuid::new_v4();
        let sid2 = Uuid::new_v4();
        let n1 = g.add_or_update_sensor(sid1, SensorType::Accelerometer, reading(sid1, 0, 1.0));
        let n2 = g.add_or_update_sensor(sid2, SensorType::Accelerometer, reading(sid2, 0, 1.0));
        g.add_edge(n1, n2, "spatially_linked", 0.5);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn test_unknown_sensor_returns_none() {
        let g = TelemetryGraph::new();
        assert!(g.get_readings(&Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_readings_in_window() {
        let mut g = TelemetryGraph::new();
        let sid = Uuid::nil();
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 100, 1.0));
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 200, 2.0));
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 300, 3.0));
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 400, 4.0));

        let in_window = g.readings_in_window(&sid, 150, 350);
        assert_eq!(in_window.len(), 2);
        assert!((in_window[0].value - 2.0).abs() < 1e-6);
        assert!((in_window[1].value - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_aggregate_window() {
        let mut g = TelemetryGraph::new();
        let sid = Uuid::nil();
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 100, 1.0));
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 200, 3.0));
        g.add_or_update_sensor(sid, SensorType::Accelerometer, reading(sid, 300, 5.0));

        let agg = g.aggregate_window(&sid, 0, 500).unwrap();
        assert!((agg.min - 1.0).abs() < 1e-6);
        assert!((agg.max - 5.0).abs() < 1e-6);
        assert!((agg.mean - 3.0).abs() < 1e-6);
        assert_eq!(agg.count, 3);
    }

    #[test]
    fn test_aggregate_window_empty_returns_none() {
        let g = TelemetryGraph::new();
        assert!(g.aggregate_window(&Uuid::new_v4(), 0, 100).is_none());
    }

    #[test]
    fn test_all_sensor_ids() {
        let mut g = TelemetryGraph::new();
        let sid1 = Uuid::new_v4();
        let sid2 = Uuid::new_v4();
        g.add_or_update_sensor(sid1, SensorType::Accelerometer, reading(sid1, 0, 1.0));
        g.add_or_update_sensor(sid2, SensorType::Speedometer, reading(sid2, 0, 50.0));
        let ids = g.all_sensor_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&sid1));
        assert!(ids.contains(&sid2));
    }
}

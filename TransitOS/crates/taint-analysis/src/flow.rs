use crate::sensors::SensorSource;
use crate::taint::{TaintStatus, TaintedValue};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DataFlowNode {
    pub id: String,
    pub label: String,
    pub taint: TaintStatus,
    pub sources: Vec<SensorSource>,
}

#[derive(Debug, Clone)]
pub struct DataFlowEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub struct DataFlowGraph {
    pub nodes: HashMap<String, DataFlowNode>,
    pub edges: Vec<DataFlowEdge>,
}

impl DataFlowGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, id: &str, label: &str, taint: TaintStatus) {
        self.nodes.insert(
            id.to_string(),
            DataFlowNode {
                id: id.to_string(),
                label: label.to_string(),
                taint,
                sources: Vec::new(),
            },
        );
    }

    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges.push(DataFlowEdge {
            from: from.to_string(),
            to: to.to_string(),
        });
    }

    pub fn taint_paths(&self) -> Vec<Vec<String>> {
        let mut paths = Vec::new();
        for node in self.nodes.values() {
            if node.taint == TaintStatus::Tainted {
                let mut path = Vec::new();
                path.push(node.id.clone());
                self._trace_back(&node.id, &mut path, &mut paths);
            }
        }
        paths
    }

    fn _trace_back(&self, node_id: &str, path: &mut Vec<String>, paths: &mut Vec<Vec<String>>) {
        let predecessors: Vec<&String> = self
            .edges
            .iter()
            .filter(|e| e.to == node_id)
            .map(|e| &e.from)
            .collect();

        if predecessors.is_empty() {
            paths.push(path.clone());
        } else {
            for pred in predecessors {
                path.push(pred.clone());
                self._trace_back(pred, path, paths);
                path.pop();
            }
        }
    }
}

impl Default for DataFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_actuator_flow_graph<T>(
    sensor_inputs: &[(&str, &TaintedValue<T>)],
    actuator: &TaintedValue<T>,
) -> DataFlowGraph {
    let mut graph = DataFlowGraph::new();

    for (name, val) in sensor_inputs {
        graph.add_node(name, &format!("sensor:{}", name), val.taint);
    }

    graph.add_node("actuator", "actuator_command", actuator.taint);

    for (name, _) in sensor_inputs {
        graph.add_edge(name, "actuator");
    }

    graph
}

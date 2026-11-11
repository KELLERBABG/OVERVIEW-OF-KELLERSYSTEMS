use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mesh_graph_core::algorithms;
use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::*;
use rand::Rng;

fn build_graph(node_count: usize, edge_density: f64) -> SocialGraph {
    let mut graph = SocialGraph::new();
    let mut rng = rand::thread_rng();

    for i in 0..node_count {
        graph.add_node(NodeData {
            id: format!("user_{}", i),
            display_name: format!("User {}", i),
            created_at: i as i64,
        });
    }

    let ids: Vec<String> = (0..node_count).map(|i| format!("user_{}", i)).collect();
    for i in 0..node_count {
        for j in (i + 1)..node_count {
            if rng.gen::<f64>() < edge_density {
                let _ = graph.add_edge(
                    &ids[i],
                    &ids[j],
                    EdgeData {
                        relationship: RelationshipType::Friend,
                        trust_weight: rng.gen::<f64>() * 0.5 + 0.5,
                        created_at: i as i64,
                        expires_at: None,
                    },
                );
            }
        }
    }
    graph
}

fn bench_pagerank(c: &mut Criterion) {
    let graph = build_graph(100, 0.1);
    c.bench_function("pagerank_100", |b| {
        b.iter(|| algorithms::compute_pagerank(black_box(&graph), 0.85, 100, 1e-6))
    });
}

fn bench_clustering(c: &mut Criterion) {
    let graph = build_graph(50, 0.15);
    c.bench_function("avg_clustering_50", |b| {
        b.iter(|| algorithms::average_clustering(black_box(&graph)))
    });
}

fn bench_shortest_path(c: &mut Criterion) {
    let graph = build_graph(200, 0.05);
    c.bench_function("shortest_path_200", |b| {
        b.iter(|| graph.shortest_trust_path("user_0", "user_199"))
    });
}

fn bench_graph_entropy(c: &mut Criterion) {
    let graph = build_graph(100, 0.08);
    c.bench_function("graph_entropy_100", |b| {
        b.iter(|| mesh_graph_core::entropy::graph_entropy(black_box(&graph)))
    });
}

criterion_group!(
    graph_benches,
    bench_pagerank,
    bench_clustering,
    bench_shortest_path,
    bench_graph_entropy
);
criterion_main!(graph_benches);

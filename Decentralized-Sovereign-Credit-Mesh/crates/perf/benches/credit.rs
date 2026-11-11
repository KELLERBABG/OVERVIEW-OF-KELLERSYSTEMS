use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mesh_credit::constraints::{ConstraintEngine, Exposure};
use mesh_credit::loan::calculate_interest_rate;
use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::*;

fn build_test_graph(n: usize) -> SocialGraph {
    let mut graph = SocialGraph::new();
    for i in 0..n {
        let id = format!("u{}", i);
        graph.add_node(NodeData {
            id: id.clone(),
            display_name: id,
            created_at: i as i64,
        });
    }
    for i in 1..n {
        let _ = graph.add_edge(
            &format!("u{}", i - 1),
            &format!("u{}", i),
            EdgeData {
                relationship: RelationshipType::Friend,
                trust_weight: 0.8,
                created_at: i as i64,
                expires_at: None,
            },
        );
    }
    graph
}

fn bench_interest_rate(c: &mut Criterion) {
    let graph = build_test_graph(50);
    c.bench_function("interest_rate_50", |b| {
        b.iter(|| calculate_interest_rate(black_box(&graph), "u0", 0.05))
    });
}

fn bench_constraint_eval(c: &mut Criterion) {
    let graph = build_test_graph(50);
    let exposures = vec![Exposure {
        borrower: "u0".into(),
        outstanding: 100.0,
        active: true,
    }];
    let constraints = ConstraintEngine::generate_credit_constraints(&graph, "u0", 1000.0);
    c.bench_function("constraint_eval", |b| {
        b.iter(|| {
            for c in &constraints {
                let _ = c.evaluate(black_box(&graph), black_box(&exposures));
            }
        })
    });
}

criterion_group!(credit_benches, bench_interest_rate, bench_constraint_eval);
criterion_main!(credit_benches);

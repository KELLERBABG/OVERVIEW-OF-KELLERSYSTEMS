//! # dfip-tests
//!
//! Integration and property-based tests for the Decoupled Fluid IP Network.

#[cfg(test)]
mod integration_tests {
    use dfip_core::graph::PatentDag;
    use dfip_core::models::*;
    use uuid::Uuid;

    fn make_node(id: Uuid) -> PatentNode {
        PatentNode {
            id,
            title: "Integration Test Patent".into(),
            patent_type: PatentType::Software,
            authors: vec!["Alice".into()],
            created_at_unix: 0,
            content_hash: vec![],
            invariant: None,
            tags: vec![],
            license: License::Open,
            meta_commitment: vec![],
        }
    }

    fn make_edge(parent: Uuid, child: Uuid, weight: f64) -> Edge {
        Edge {
            id: Uuid::new_v4(),
            parent_id: parent,
            child_id: child,
            kind: EdgeKind::Derivative,
            delta: None,
            contribution_weight: weight,
            signature: vec![],
        }
    }

    #[test]
    fn test_full_pipeline_no_infringement() {
        let mut dag = PatentDag::new();

        // Original patent
        let orig = Uuid::new_v4();
        dag.add_node(make_node(orig)).unwrap();

        // Derivative
        let deriv = Uuid::new_v4();
        dag.add_node(make_node(deriv)).unwrap();
        dag.add_edge(&orig, &deriv, make_edge(orig, deriv, 0.3)).unwrap();

        // Verify graph invariants
        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 1);
        assert!(dag.has_path(&orig, &deriv).unwrap());

        // Ancestors
        let anc = dag.ancestor_ids(&deriv).unwrap();
        assert!(anc.contains(&orig));

        // Topological order
        let order = dag.topological_order_indices().unwrap();
        let pos_orig = order.iter().position(|&x| x == orig).unwrap();
        let pos_deriv = order.iter().position(|&x| x == deriv).unwrap();
        assert!(pos_orig < pos_deriv);

        // Distance
        assert_eq!(dag.distance(&orig, &deriv).unwrap(), Some(1));

        // Lifecycle events recorded
        assert_eq!(dag.lifecycle_log().len(), 3); // 2 nodes + 1 edge
    }

    #[test]
    fn test_three_generation_lineage() {
        let mut dag = PatentDag::new();

        let grandparent = Uuid::new_v4();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();

        dag.add_node(make_node(grandparent)).unwrap();
        dag.add_node(make_node(parent)).unwrap();
        dag.add_node(make_node(child)).unwrap();

        dag.add_edge(&grandparent, &parent, make_edge(grandparent, parent, 0.5)).unwrap();
        dag.add_edge(&parent, &child, make_edge(parent, child, 0.4)).unwrap();

        // LCA of child and parent should be parent
        let lcas = dag.lowest_common_ancestors(&parent, &child).unwrap();
        assert_eq!(lcas.len(), 1);
        assert_eq!(lcas[0].id, parent);

        // Distance
        assert_eq!(dag.distance(&grandparent, &child).unwrap(), Some(2));
    }

    #[test]
    fn test_invalid_weight_rejected() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();

        let err = dag.add_edge(&a, &b, make_edge(a, b, 1.5)).unwrap_err();
        assert!(matches!(err, dfip_core::error::CoreError::InvalidContributionWeight(_)));
    }
}
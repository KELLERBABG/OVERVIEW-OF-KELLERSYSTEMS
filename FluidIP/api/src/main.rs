//! # dfip-api — REST/HTTP Server for Decoupled Fluid IP Network
//!
//! Full API implementation with all endpoints from the spec.
//! Run with: `cargo run -p dfip-api`
//!
//! Endpoints:
//! - POST /patents           — Submit a new patent
//! - GET  /patents/{id}      — Get patent details
//! - GET  /patents           — List all patents
//! - POST /edges             — Add a derivation edge
//! - POST /verify            — Trigger SMT verification on an edge
//! - GET  /royalties/{id}    — Compute royalty shares for a patent
//! - GET  /derivatives/{id}  — Query derivative tree
//! - POST /simulate          — Run simulation examples
//! - GET  /health            — Health check

use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use dfip_core::delta::build_delta;
use dfip_core::explorer::compare_nodes;
use dfip_core::graph::PatentDag;
use dfip_core::ir::*;
use dfip_core::models::*;
use dfip_core::royalties::compute_royalty_shares;
use dfip_core::solver::{run_verification, BuiltInSolver, SolverConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

// ─── App State ───────────────────────────────────────────────────────────────

struct AppState {
    dag: Mutex<PatentDag>,
    ir_store: Mutex<HashMap<Uuid, InventionIr>>,
}

// ─── API Models ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SubmitPatentRequest {
    title: String,
    patent_type: String,
    authors: Vec<String>,
    preconditions: Vec<String>,
    postconditions: Vec<String>,
    invariants: Vec<String>,
}

#[derive(Serialize)]
struct PatentResponse {
    id: Uuid,
    title: String,
    patent_type: String,
    authors: Vec<String>,
    created_at: i64,
}

#[derive(Deserialize)]
struct AddEdgeRequest {
    parent_id: Uuid,
    child_id: Uuid,
    contribution_weight: f64,
}

#[derive(Serialize)]
struct EdgeResponse {
    id: Uuid,
    parent_id: Uuid,
    child_id: Uuid,
    weight: f64,
}

#[derive(Deserialize)]
struct VerifyRequest {
    edge_id: Uuid,
    original_id: Uuid,
    derivative_id: Uuid,
}

#[derive(Serialize)]
struct VerifyResponse {
    edge_id: Uuid,
    sat: String,
    severity: f64,
    solver_time_ms: f64,
    distance: Option<usize>,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "Decoupled Fluid IP Network API",
        "version": "0.1.0"
    }))
}

async fn submit_patent(
    data: web::Data<AppState>,
    req: web::Json<SubmitPatentRequest>,
) -> impl Responder {
    let patent_type = match req.patent_type.to_lowercase().as_str() {
        "software" => PatentType::Software,
        "chemical" => PatentType::Chemical,
        "mechanical" => PatentType::Mechanical,
        _ => return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "invalid patent type. Use: software, chemical, mechanical"
        })),
    };

    let id = Uuid::new_v4();
    let node = PatentNode {
        id,
        title: req.title.clone(),
        patent_type,
        authors: req.authors.clone(),
        created_at_unix: chrono::Utc::now().timestamp(),
        content_hash: vec![],
        invariant: None,
        tags: vec![],
        license: License::Open,
        meta_commitment: vec![],
    };

    let ir = InventionIr {
        kind: match node.patent_type {
            PatentType::Software => IrKind::Software(SsaTrace {
                entry_block: "entry".into(),
                blocks: HashMap::new(),
                inputs: vec![],
                outputs: vec![],
            }),
            PatentType::Chemical => IrKind::Chemical(MolecularGraph {
                atoms: vec![],
                bonds: vec![],
                constraints: vec![],
            }),
            PatentType::Mechanical => IrKind::Mechanical(MechanicalModel {
                entities: vec![],
                constraints: vec![],
                metadata: HashMap::new(),
            }),
        },
        preconditions: req.preconditions.clone(),
        postconditions: req.postconditions.clone(),
        invariants: req.invariants.clone(),
    };

    match data.dag.lock() {
        Ok(mut dag) => {
            if let Err(e) = dag.add_node(node) {
                return HttpResponse::BadRequest().json(serde_json::json!({"error": e.to_string()}));
            }
            if let Ok(mut store) = data.ir_store.lock() {
                store.insert(id, ir);
            }
            HttpResponse::Created().json(PatentResponse {
                id,
                title: req.title.clone(),
                patent_type: req.patent_type.clone(),
                authors: req.authors.clone(),
                created_at: chrono::Utc::now().timestamp(),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn get_patent(data: web::Data<AppState>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    match data.dag.lock() {
        Ok(dag) => match dag.get_node(&id) {
            Ok(node) => HttpResponse::Ok().json(PatentResponse {
                id: node.id,
                title: node.title.clone(),
                patent_type: format!("{:?}", node.patent_type).to_lowercase(),
                authors: node.authors.clone(),
                created_at: node.created_at_unix,
            }),
            Err(e) => HttpResponse::NotFound().json(serde_json::json!({"error": e.to_string()})),
        },
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn list_patents(data: web::Data<AppState>) -> impl Responder {
    match data.dag.lock() {
        Ok(dag) => {
            let state = dag.export_state();
            let patents: Vec<PatentResponse> = state
                .nodes
                .values()
                .map(|n| PatentResponse {
                    id: n.id,
                    title: n.title.clone(),
                    patent_type: format!("{:?}", n.patent_type).to_lowercase(),
                    authors: n.authors.clone(),
                    created_at: n.created_at_unix,
                })
                .collect();
            HttpResponse::Ok().json(patents)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn add_edge(data: web::Data<AppState>, req: web::Json<AddEdgeRequest>) -> impl Responder {
    let edge = Edge {
        id: Uuid::new_v4(),
        parent_id: req.parent_id,
        child_id: req.child_id,
        kind: EdgeKind::Derivative,
        delta: None,
        contribution_weight: req.contribution_weight,
        signature: vec![],
    };

    match data.dag.lock() {
        Ok(mut dag) => match dag.add_edge(&req.parent_id, &req.child_id, edge.clone()) {
            Ok(()) => HttpResponse::Created().json(EdgeResponse {
                id: edge.id,
                parent_id: req.parent_id,
                child_id: req.child_id,
                weight: req.contribution_weight,
            }),
            Err(e) => HttpResponse::BadRequest().json(serde_json::json!({"error": e.to_string()})),
        },
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn verify(data: web::Data<AppState>, req: web::Json<VerifyRequest>) -> impl Responder {
    let solver = BuiltInSolver;
    let config = SolverConfig::default();

    let (orig_ir, deriv_ir) = match data.ir_store.lock() {
        Ok(store) => (
            store.get(&req.original_id).cloned(),
            store.get(&req.derivative_id).cloned(),
        ),
        Err(_) => (None, None),
    };

    let (orig_ir, deriv_ir) = match (orig_ir, deriv_ir) {
        (Some(o), Some(d)) => (o, d),
        _ => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "IR not found for one or both nodes"
            }));
        }
    };

    match data.dag.lock() {
        Ok(dag) => {
            let result = compare_nodes(&dag, &req.original_id, &req.derivative_id, &orig_ir, &deriv_ir, &solver, &config);
            match result {
                Ok(sim) => HttpResponse::Ok().json(VerifyResponse {
                    edge_id: req.edge_id,
                    sat: format!("{:?}", sim.sat),
                    severity: sim.severity,
                    solver_time_ms: 0.0,
                    distance: sim.distance,
                }),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn get_royalties(data: web::Data<AppState>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    match data.dag.lock() {
        Ok(dag) => match compute_royalty_shares(&dag, &id) {
            Ok(shares) => {
                let result: Vec<serde_json::Value> = shares
                    .iter()
                    .map(|(nid, share)| serde_json::json!({"node_id": nid, "share": share}))
                    .collect();
                HttpResponse::Ok().json(result)
            }
            Err(e) => HttpResponse::BadRequest().json(serde_json::json!({"error": e.to_string()})),
        },
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn get_derivatives(data: web::Data<AppState>, path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    match data.dag.lock() {
        Ok(dag) => match dag.descendant_ids(&id) {
            Ok(desc) => {
                let result: Vec<serde_json::Value> = desc
                    .iter()
                    .map(|nid| serde_json::json!({"node_id": nid}))
                    .collect();
                HttpResponse::Ok().json(result)
            }
            Err(e) => HttpResponse::BadRequest().json(serde_json::json!({"error": e.to_string()})),
        },
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn simulate() -> impl Responder {
    let mut dag = PatentDag::new();
    let mut ir_store: HashMap<Uuid, InventionIr> = HashMap::new();

    let orig_id = Uuid::new_v4();
    let orig = PatentNode {
        id: orig_id,
        title: "QuickSort Algorithm v1".into(),
        patent_type: PatentType::Software,
        authors: vec!["Alice".into()],
        created_at_unix: chrono::Utc::now().timestamp(),
        content_hash: vec![],
        invariant: None,
        tags: vec!["sorting".into(), "algorithm".into()],
        license: License::Open,
        meta_commitment: vec![],
    };
    let orig_ir = InventionIr {
        kind: IrKind::Software(SsaTrace {
            entry_block: "entry".into(), blocks: HashMap::new(),
            inputs: vec![("arr".into(), IrType::Array(Box::new(IrType::Int(32)), Box::new(IrType::Int(32))))],
            outputs: vec![("sorted".into(), IrType::Array(Box::new(IrType::Int(32)), Box::new(IrType::Int(32))))],
        }),
        preconditions: vec!["length(arr) > 0".into()],
        postconditions: vec!["is_sorted(sorted)".into(), "permutation(arr, sorted)".into()],
        invariants: vec![],
    };
    dag.add_node(orig).unwrap();
    ir_store.insert(orig_id, orig_ir);

    let deriv_id = Uuid::new_v4();
    let deriv = PatentNode {
        id: deriv_id,
        title: "QuickSort Algorithm v2 (Optimized)".into(),
        patent_type: PatentType::Software,
        authors: vec!["Alice".into(), "Bob".into()],
        created_at_unix: chrono::Utc::now().timestamp(),
        content_hash: vec![],
        invariant: None,
        tags: vec!["sorting".into(), "optimized".into()],
        license: License::Protected,
        meta_commitment: vec![],
    };
    let deriv_ir = InventionIr {
        kind: IrKind::Software(SsaTrace {
            entry_block: "entry".into(),
            blocks: {
                let mut b = HashMap::new();
                b.insert("entry".into(), BasicBlock {
                    label: "entry".into(),
                    instructions: vec![SsaInstruction {
                        result: Some("pivot".into()),
                        op: SsaOp::Custom("median_of_three".into()),
                        operands: vec![IrExpr::Ref("arr".into())],
                    }],
                    terminator: Terminator::Return(Some(IrExpr::Ref("sorted".into()))),
                });
                b
            },
            inputs: vec![("arr".into(), IrType::Array(Box::new(IrType::Int(32)), Box::new(IrType::Int(32))))],
            outputs: vec![("sorted".into(), IrType::Array(Box::new(IrType::Int(32)), Box::new(IrType::Int(32))))],
        }),
        preconditions: vec!["length(arr) > 0".into()],
        postconditions: vec!["is_sorted(sorted)".into(), "permutation(arr, sorted)".into(), "time_complexity = O(n log n)".into()],
        invariants: vec![],
    };
    dag.add_node(deriv).unwrap();
    ir_store.insert(deriv_id, deriv_ir);

    let edge = Edge {
        id: Uuid::new_v4(), parent_id: orig_id, child_id: deriv_id,
        kind: EdgeKind::Improvement, delta: None,
        contribution_weight: 0.3, signature: vec![],
    };
    dag.add_edge(&orig_id, &deriv_id, edge).unwrap();

    let delta = build_delta(orig_id, deriv_id, ir_store.get(&orig_id).unwrap(),
        ir_store.get(&deriv_id).unwrap(), &[]).unwrap();
    let solver = BuiltInSolver;
    let config = SolverConfig::default();
    let verification = run_verification(Uuid::new_v4(), &delta.formula_smt_lib2, &solver, &config).unwrap();
    let royalties = compute_royalty_shares(&dag, &deriv_id).unwrap();

    HttpResponse::Ok().json(serde_json::json!({
        "simulation": "QuickSort Patent Pipeline",
        "original_node": orig_id,
        "derivative_node": deriv_id,
        "delta_formula_size": delta.formula_smt_lib2.len(),
        "verification_result": {
            "sat": format!("{:?}", verification.sat),
            "severity": verification.severity_score,
        },
        "royalty_shares": royalties.iter().map(|(k, v)| {
            serde_json::json!({"node": k, "share": v})
        }).collect::<Vec<_>>(),
        "node_count": dag.node_count(),
        "edge_count": dag.edge_count(),
    }))
}

// ─── Main ────────────────────────────────────────────────────────────────────

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Decoupled Fluid IP Network API");
    println!("Listening on http://127.0.0.1:8080");
    println!("Endpoints:");
    println!("  POST /patents         Submit a new patent");
    println!("  GET  /patents         List all patents");
    println!("  GET  /patents/{{id}}  Get patent details");
    println!("  POST /edges           Add a derivation edge");
    println!("  POST /verify          Verify edge (SMT)");
    println!("  GET  /royalties/{{id}} Compute royalties");
    println!("  GET  /derivatives/{{id}} List derivatives");
    println!("  POST /simulate        Run full simulation");
    println!("  GET  /health          Health check");

    let state = web::Data::new(AppState {
        dag: Mutex::new(PatentDag::new()),
        ir_store: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .route("/health", web::get().to(health))
            .route("/patents", web::post().to(submit_patent))
            .route("/patents", web::get().to(list_patents))
            .route("/patents/{id}", web::get().to(get_patent))
            .route("/edges", web::post().to(add_edge))
            .route("/verify", web::post().to(verify))
            .route("/royalties/{id}", web::get().to(get_royalties))
            .route("/derivatives/{id}", web::get().to(get_derivatives))
            .route("/simulate", web::post().to(simulate))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
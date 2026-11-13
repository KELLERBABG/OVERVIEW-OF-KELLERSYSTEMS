//! CIL Bytecode Analyzer.
//!
//! Analyzes CIL (Common Intermediate Language) bytecode to extract
//! control-flow and data-flow facts used by the verification passes.
//!
//! In a full implementation this would interface with a CIL disassembler
//! (e.g., ILSpy, Mono.Cecil, or dotnet-ildasm). Here we model the
//! essential CIL constructs for the verification pipeline.

use std::collections::{HashMap, HashSet};

use crate::csharp_engine::ast_ingest::{CSharpProject, MethodBody, Statement};
use crate::csharp_engine::EngineError;

/// Facts extracted from CIL analysis of a project.
#[derive(Debug)]
pub struct CilFacts {
    /// Per-method control-flow graph: method FQN → CFG.
    pub cfgs: HashMap<String, ControlFlowGraph>,
    /// Data-flow facts: variable → set of possible null states at each program point.
    pub null_flow: HashMap<String, Vec<NullState>>,
    /// Array/Span access patterns: method FQN → list of index accesses.
    pub index_accesses: HashMap<String, Vec<IndexAccessFact>>,
    /// Loop bounds information.
    pub loop_bounds: HashMap<String, LoopBoundInfo>,
}

/// A control-flow graph for a single method.
#[derive(Debug)]
pub struct ControlFlowGraph {
    /// Basic blocks in order.
    pub blocks: Vec<BasicBlock>,
    /// Entry block index.
    pub entry: usize,
    /// Exit block indices.
    pub exits: Vec<usize>,
}

/// A basic block in the CFG.
#[derive(Debug)]
pub struct BasicBlock {
    /// Block identifier.
    pub id: usize,
    /// Instructions in this block.
    pub instructions: Vec<CilInstruction>,
    /// Successor block indices.
    pub successors: Vec<usize>,
}

/// A single CIL instruction (simplified).
#[derive(Debug)]
pub enum CilInstruction {
    /// Load a local variable.
    Ldloc(String),
    /// Store to a local variable.
    Stloc(String),
    /// Load an argument.
    Ldarg(usize),
    /// Call a method.
    Call(String),
    /// Array/Span index load.
    Ldelem(String),
    /// Array/Span index store.
    Stelem(String),
    /// Branch (conditional).
    BrIf(String, usize),
    /// Unconditional branch.
    Br(usize),
    /// Return.
    Ret,
    /// Throw.
    Throw,
    /// Load a constant.
    LdcI4(i32),
    /// Binary operation.
    Op(string),
}

/// Null-state tracking for a variable at a program point.
#[derive(Debug)]
pub struct NullState {
    /// Variable name.
    pub variable: String,
    /// Program point (block ID, instruction index).
    pub block_id: usize,
    pub instr_idx: usize,
    /// Whether the variable is known to be null at this point.
    pub is_null: bool,
    /// Whether the variable is known to be non-null.
    pub is_non_null: bool,
    /// Whether the state is uncertain (may or may not be null).
    pub is_maybe: bool,
}

/// A fact about an index access into an array or Span.
#[derive(Debug)]
pub struct IndexAccessFact {
    /// The target variable name (array or span).
    pub target: String,
    /// The index expression description.
    pub index_expr: String,
    /// Whether the bounds check is present in CIL (ldlen + clt/clt.un).
    pub has_bounds_check: bool,
    /// The known length of the target (if statically determinable).
    pub known_length: Option<usize>,
    /// Block and instruction where this access occurs.
    pub block_id: usize,
    pub instr_idx: usize,
}

/// Loop bound information.
#[derive(Debug)]
pub struct LoopBoundInfo {
    /// Loop variable name.
    pub loop_var: String,
    /// Lower bound (inclusive).
    pub lower: i64,
    /// Upper bound (exclusive) — None if unbounded.
    pub upper: Option<i64>,
    /// Step.
    pub step: i64,
}

/// Analyze CIL for a project, extracting CFGs and data-flow facts.
pub fn analyze_cil(project: &CSharpProject) -> Result<CilFacts, EngineError> {
    let mut cfgs = HashMap::new();
    let mut null_flow = HashMap::new();
    let mut index_accesses = HashMap::new();
    let mut loop_bounds = HashMap::new();

    for unit in &project.compilation_units {
        for ns in &unit.namespaces {
            for ty in &ns.types {
                for member in &ty.members {
                    if let Some(body) = &member.body {
                        let method_fqn = format!("{}.{}::{}", ns.name, ty.name, member.name);

                        // Build CFG from statements
                        let cfg = build_cfg_from_body(body);
                        cfgs.insert(method_fqn.clone(), cfg);

                        // Extract null-flow facts
                        extract_null_flow(body, &method_fqn, &mut null_flow);

                        // Extract index access facts
                        extract_index_accesses(body, &method_fqn, &mut index_accesses);

                        // Extract loop bounds
                        extract_loop_bounds(body, &method_fqn, &mut loop_bounds);
                    }
                }
            }
        }
    }

    Ok(CilFacts {
        cfgs,
        null_flow,
        index_accesses,
        loop_bounds,
    })
}

/// Build a simplified CFG from a method body's statements.
fn build_cfg_from_body(body: &MethodBody) -> ControlFlowGraph {
    let mut blocks = Vec::new();
    let mut current_block_id = 0;
    let mut current_instrs: Vec<CilInstruction> = Vec::new();

    for stmt in &body.statements {
        match stmt {
            Statement::Assignment { target, .. } => {
                current_instrs.push(CilInstruction::Stloc(target.clone()));
            }
            Statement::Invocation { method_name, .. } => {
                current_instrs.push(CilInstruction::Call(method_name.clone()));
            }
            Statement::IndexAccess { target, is_span: _, .. } => {
                if *is_span {
                    current_instrs.push(CilInstruction::Ldelem(target.clone()));
                } else {
                    current_instrs.push(CilInstruction::Ldelem(target.clone()));
                }
            }
            Statement::If { then_branch, else_branch, .. } => {
                // Close current block, create branch
                if !current_instrs.is_empty() {
                    blocks.push(BasicBlock {
                        id: current_block_id,
                        instructions: std::mem::take(&mut current_instrs),
                        successors: vec![],
                    });
                    current_block_id += 1;
                }

                // Then branch
                let then_block_id = current_block_id;
                for s in then_branch {
                    push_stmt_instr(s, &mut current_instrs);
                }
                if !current_instrs.is_empty() {
                    blocks.push(BasicBlock {
                        id: then_block_id,
                        instructions: std::mem::take(&mut current_instrs),
                        successors: vec![],
                    });
                    current_block_id += 1;
                }

                // Else branch
                let else_block_id = current_block_id;
                for s in else_branch {
                    push_stmt_instr(s, &mut current_instrs);
                }
                if !current_instrs.is_empty() {
                    blocks.push(BasicBlock {
                        id: else_block_id,
                        instructions: std::mem::take(&mut current_instrs),
                        successors: vec![],
                    });
                    current_block_id += 1;
                }
            }
            Statement::ForLoop { body: loop_body, .. } => {
                // Model as a simple loop block
                let loop_block_id = current_block_id;
                for s in loop_body {
                    push_stmt_instr(s, &mut current_instrs);
                }
                if !current_instrs.is_empty() {
                    blocks.push(BasicBlock {
                        id: loop_block_id,
                        instructions: std::mem::take(&mut current_instrs),
                        successors: vec![],
                    });
                    current_block_id += 1;
                }
            }
            Statement::WhileLoop { body: loop_body, .. } => {
                let loop_block_id = current_block_id;
                for s in loop_body {
                    push_stmt_instr(s, &mut current_instrs);
                }
                if !current_instrs.is_empty() {
                    blocks.push(BasicBlock {
                        id: loop_block_id,
                        instructions: std::mem::take(&mut current_instrs),
                        successors: vec![],
                    });
                    current_block_id += 1;
                }
            }
            Statement::Return(_) => {
                current_instrs.push(CilInstruction::Ret);
            }
            Statement::Throw(_) => {
                current_instrs.push(CilInstruction::Throw);
            }
            _ => {}
        }
    }

    // Close any remaining instructions into the last block
    if !current_instrs.is_empty() {
        blocks.push(BasicBlock {
            id: current_block_id,
            instructions: std::mem::take(&mut current_instrs),
            successors: vec![],
        });
        current_block_id += 1;
    }

    // Wire up sequential successors (simplified linear CFG)
    for i in 0..blocks.len().saturating_sub(1) {
        blocks[i].successors.push(i + 1);
    }

    let entry = 0;
    let exits: Vec<usize> = blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.instructions.iter().any(|i| matches!(i, CilInstruction::Ret | CilInstruction::Throw)))
        .map(|(i, _)| i)
        .collect();

    ControlFlowGraph {
        blocks,
        entry,
        exits,
    }
}

fn push_stmt_instr(stmt: &Statement, instrs: &mut Vec<CilInstruction>) {
    match stmt {
        Statement::Assignment { target, .. } => {
            instrs.push(CilInstruction::Stloc(target.clone()));
        }
        Statement::Invocation { method_name, .. } => {
            instrs.push(CilInstruction::Call(method_name.clone()));
        }
        Statement::IndexAccess { target, .. } => {
            instrs.push(CilInstruction::Ldelem(target.clone()));
        }
        Statement::Return(_) => {
            instrs.push(CilInstruction::Ret);
        }
        Statement::Throw(_) => {
            instrs.push(CilInstruction::Throw);
        }
        _ => {}
    }
}

/// Extract null-flow facts from a method body.
fn extract_null_flow(
    body: &MethodBody,
    method_fqn: &str,
    null_flow: &mut HashMap<String, Vec<NullState>>,
) {
    let mut states = Vec::new();
    let mut block_id = 0;
    let mut instr_idx = 0;

    for local in &body.locals {
        // If a local is nullable and has no initializer, it starts as null
        if local.nullability == crate::csharp_engine::ast_ingest::NullabilityAnnotation::Nullable
            && !local.has_initializer
        {
            states.push(NullState {
                variable: local.name.clone(),
                block_id,
                instr_idx,
                is_null: true,
                is_non_null: false,
                is_maybe: false,
            });
        }
    }

    // Track assignments that may introduce null
    for stmt in &body.statements {
        if let crate::csharp_engine::ast_ingest::Statement::Assignment { target, expr } = stmt {
            if matches!(expr, crate::csharp_engine::ast_ingest::Expression::Null) {
                states.push(NullState {
                    variable: target.clone(),
                    block_id,
                    instr_idx,
                    is_null: true,
                    is_non_null: false,
                    is_maybe: false,
                });
            }
        }
        instr_idx += 1;
    }

    if !states.is_empty() {
        null_flow.insert(method_fqn.to_string(), states);
    }
}

/// Extract index access facts from a method body.
fn extract_index_accesses(
    body: &MethodBody,
    method_fqn: &str,
    index_accesses: &mut HashMap<String, Vec<IndexAccessFact>>,
) {
    let mut accesses = Vec::new();
    let mut block_id = 0;
    let mut instr_idx = 0;

    for stmt in &body.statements {
        if let crate::csharp_engine::ast_ingest::Statement::IndexAccess {
            target,
            index,
            is_span,
        } = stmt
        {
            // Determine if bounds check is present (heuristic: look for preceding if-check)
            let has_bounds_check = false; // Would require full CIL analysis

            accesses.push(IndexAccessFact {
                target: target.clone(),
                index_expr: expr_to_string(index),
                has_bounds_check,
                known_length: None,
                block_id,
                instr_idx,
            });
        }
        instr_idx += 1;
    }

    if !accesses.is_empty() {
        index_accesses.insert(method_fqn.to_string(), accesses);
    }
}

/// Extract loop bound information.
fn extract_loop_bounds(
    body: &MethodBody,
    method_fqn: &str,
    loop_bounds: &mut HashMap<String, LoopBoundInfo>,
) {
    for stmt in &body.statements {
        if let crate::csharp_engine::ast_ingest::Statement::ForLoop {
            init,
            condition,
            increment,
            ..
        } = stmt
        {
            // Try to extract: for (int i = lower; i < upper; i += step)
            if let Some(init_stmt) = init {
                if let crate::csharp_engine::ast_ingest::Statement::Assignment { target, expr } = init_stmt.as_ref() {
                    if let crate::csharp_engine::ast_ingest::Expression::Literal(val) = expr {
                        let lower: i64 = val.parse().unwrap_or(0);

                        // Extract upper from condition
                        let upper = if let Some(cond) = condition {
                            if let crate::csharp_engine::ast_ingest::Expression::BinaryOp { right, .. } = cond.as_ref() {
                                if let crate::csharp_engine::ast_ingest::Expression::Literal(ub) = right {
                                    ub.parse::<i64>().ok()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // Extract step from increment
                        let step = if let Some(incr) = increment {
                            if let crate::csharp_engine::ast_ingest::Statement::Assignment { expr, .. } = incr.as_ref() {
                                if let crate::csharp_engine::ast_ingest::Expression::BinaryOp { right, .. } = expr {
                                    if let crate::csharp_engine::ast_ingest::Expression::Literal(s) = right {
                                        s.parse::<i64>().unwrap_or(1)
                                    } else {
                                        1
                                    }
                                } else {
                                    1
                                }
                            } else {
                                1
                            }
                        } else {
                            1
                        };

                        loop_bounds.insert(
                            method_fqn.to_string(),
                            LoopBoundInfo {
                                loop_var: target.clone(),
                                lower,
                                upper,
                                step,
                            },
                        );
                    }
                }
            }
        }
    }
}

/// Convert an expression to a string representation.
fn expr_to_string(expr: &crate::csharp_engine::ast_ingest::Expression) -> String {
    match expr {
        crate::csharp_engine::ast_ingest::Expression::Literal(v) => v.clone(),
        crate::csharp_engine::ast_ingest::Expression::Variable(v) => v.clone(),
        crate::csharp_engine::ast_ingest::Expression::BinaryOp { op, left, right } => {
            format!("{} {} {}", expr_to_string(left), op, expr_to_string(right))
        }
        crate::csharp_engine::ast_ingest::Expression::Null => "null".to_string(),
        _ => String::new(),
    }
}

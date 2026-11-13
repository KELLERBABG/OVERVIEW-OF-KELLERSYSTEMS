//! Lowering of C++ function bodies into Keller-VIR SSA form.
//!
//! Each C++ function is lowered into a sequence of SSA operations
//! that become nodes in the hypergraph.

use crate::core::ssa::{SsaBuilder, SsaValue};
use super::ast::{FunctionDef, Statement, Expression};
use super::CppFrontendError;

/// Lower a C++ function definition into the SSA builder.
pub fn lower_function(
    ssa: &mut SsaBuilder,
    func: &FunctionDef,
) -> Result<(), CppFrontendError> {
    // Create a new basic block for the function entry.
    let entry_bb = ssa.new_block(format!("<{}>.entry", func.name));

    // Lower parameters as incoming SSA values.
    for param in &func.params {
        if let Some(name) = &param.name {
            ssa.define_incoming(entry_bb, name.clone(), param.type_name.clone());
        }
    }

    // Lower the body statements sequentially.
    let mut current_bb = entry_bb;
    for stmt in &func.body {
        current_bb = lower_statement(ssa, current_bb, stmt)?;
    }

    // Implicit return void if no explicit return was emitted.
    if !ssa.has_return(current_bb) {
        ssa.emit_return_void(current_bb);
    }

    Ok(())
}

/// Lower a single statement, returning the successor basic block ID.
fn lower_statement(
    ssa: &mut SsaBuilder,
    bb: u32,
    stmt: &Statement,
) -> Result<u32, CppFrontendError> {
    match stmt {
        Statement::Expr(expr) => {
            let _ = lower_expression(ssa, bb, expr)?;
            Ok(bb)
        }
        Statement::Block(statements) => {
            let mut current = bb;
            for s in statements {
                current = lower_statement(ssa, current, s)?;
            }
            Ok(current)
        }
        Statement::ForLoop { init, condition, increment, body } => {
            // Create loop header and body blocks.
            let header_bb = ssa.new_block("loop.header");
            let body_bb = ssa.new_block("loop.body");
            let exit_bb = ssa.new_block("loop.exit");

            // Lower init into the current block, branch to header.
            if let Some(init_stmt) = init {
                lower_statement(ssa, bb, init_stmt)?;
            }
            ssa.emit_branch(bb, header_bb);

            // Header: evaluate condition.
            if let Some(cond) = condition {
                let cond_val = lower_expression(ssa, header_bb, cond)?;
                ssa.emit_conditional_branch(header_bb, cond_val, body_bb, exit_bb);
            } else {
                // Infinite loop (condition is true).
                ssa.emit_branch(header_bb, body_bb);
            }

            // Body: lower the loop body.
            let body_end = lower_statement(ssa, body_bb, body)?;

            // Increment and back-edge.
            if let Some(increment) = increment {
                let inc_bb = ssa.new_block("loop.increment");
                ssa.emit_branch(body_end, inc_bb);
                lower_statement(ssa, inc_bb, increment)?;
                ssa.emit_branch(inc_bb, header_bb);
            } else {
                ssa.emit_branch(body_end, header_bb);
            }

            Ok(exit_bb)
        }
        Statement::RangeFor { var_type, var_name, iterable_expr, body } => {
            // Range-for desugars to:
            //   auto __begin = std::begin(iterable);
            //   auto __end = std::end(iterable);
            //   for (; __begin != __end; ++__begin) { *var = *__begin; body }
            let header_bb = ssa.new_block("range_for.header");
            let body_bb = ssa.new_block("range_for.body");
            let exit_bb = ssa.new_block("range_for.exit");

            // Materialize begin/end iterators.
            let _iterable = lower_expression(ssa, bb, iterable_expr)?;
            let begin_val = ssa.emit_call(bb, "std::begin", vec![]);
            let end_val = ssa.emit_call(bb, "std::end", vec![]);

            ssa.emit_branch(bb, header_bb);

            // Header: compare iterators.
            let cmp_val = ssa.emit_binary_op(header_bb, "ne", begin_val, end_val);
            ssa.emit_conditional_branch(header_bb, cmp_val, body_bb, exit_bb);

            // Body: dereference and assign.
            let deref_val = ssa.emit_call(body_bb, "operator*", vec![begin_val]);
            ssa.define_local(body_bb, var_name.clone(), var_type.clone(), deref_val);

            let body_end = lower_statement(ssa, body_bb, body)?;

            // Increment iterator.
            let inc_bb = ssa.new_block("range_for.increment");
            ssa.emit_branch(body_end, inc_bb);
            let _inc_val = ssa.emit_call(inc_bb, "operator++", vec![begin_val]);
            ssa.emit_branch(inc_bb, header_bb);

            Ok(exit_bb)
        }
        Statement::If { condition, then_branch, else_branch } => {
            let cond_bb = bb;
            let then_bb = ssa.new_block("if.then");
            let else_bb = ssa.new_block("if.else");
            let merge_bb = ssa.new_block("if.merge");

            let cond_val = lower_expression(ssa, cond_bb, condition)?;
            ssa.emit_conditional_branch(cond_bb, cond_val, then_bb, else_bb);

            let then_end = lower_statement(ssa, then_bb, then_branch)?;
            ssa.emit_branch(then_end, merge_bb);

            if let Some(else_stmt) = else_branch {
                let else_end = lower_statement(ssa, else_bb, else_stmt)?;
                ssa.emit_branch(else_end, merge_bb);
            } else {
                ssa.emit_branch(else_bb, merge_bb);
            }

            Ok(merge_bb)
        }
        Statement::Return(value) => {
            if let Some(expr) = value {
                let val = lower_expression(ssa, bb, expr)?;
                ssa.emit_return(bb, val);
            } else {
                ssa.emit_return_void(bb);
            }
            Ok(bb)
        }
    }
}

/// Lower an expression, returning the SSA value it produces.
fn lower_expression(
    ssa: &mut SsaBuilder,
    bb: u32,
    expr: &Expression,
) -> Result<SsaValue, CppFrontendError> {
    match expr {
        Expression::Literal(text) => {
            Ok(ssa.emit_constant(bb, text.clone()))
        }
        Expression::VarRef(name) => {
            Ok(ssa.lookup_local(bb, name))
        }
        Expression::Call { callee, args } => {
            let arg_values: Vec<SsaValue> = args
                .iter()
                .map(|a| lower_expression(ssa, bb, a))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ssa.emit_call(bb, callee, arg_values))
        }
        Expression::MemberAccess { base, member, is_arrow } => {
            let base_val = lower_expression(ssa, bb, base)?;
            if *is_arrow {
                // obj->member desugars to (*obj).member
                let deref = ssa.emit_call(bb, "operator*", vec![base_val]);
                Ok(ssa.emit_field_access(bb, deref, member.clone()))
            } else {
                Ok(ssa.emit_field_access(bb, base_val, member.clone()))
            }
        }
        Expression::Assign { target, value } => {
            let target_val = lower_expression(ssa, bb, target)?;
            let value_val = lower_expression(ssa, bb, value)?;
            ssa.emit_store(bb, target_val, value_val);
            Ok(value_val)
        }
        Expression::BinaryOp { op, lhs, rhs } => {
            let lhs_val = lower_expression(ssa, bb, lhs)?;
            let rhs_val = lower_expression(ssa, bb, rhs)?;
            Ok(ssa.emit_binary_op(bb, op, lhs_val, rhs_val))
        }
    }
}

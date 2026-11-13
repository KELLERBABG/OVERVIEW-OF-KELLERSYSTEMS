//! C-RMM (C Reduced Memory Model) Lowering & Representation.
//!
//! Bridges C AST functions into C-RMM memory models and tracks pointer lifecycles.

#![allow(dead_code)]

use crate::c::ast::*;
use crate::vir::crmm::{CrmmOp, CrmmProgram, CrmmVarKind};

pub struct CrmmLowering {
    pub program: CrmmProgram,
}

impl CrmmLowering {
    pub fn new() -> Self {
        Self {
            program: CrmmProgram::new(),
        }
    }

    pub fn lower_translation_unit(&mut self, tu: &TranslationUnit) {
        for g in &tu.globals {
            let kind = if g.ty.is_pointer() || g.ty.is_array() {
                CrmmVarKind::Pointer
            } else {
                CrmmVarKind::Scalar
            };
            self.program.add_global(&g.name, kind);
        }

        for func in &tu.functions {
            self.lower_function(func);
        }
    }

    pub fn lower_function(&mut self, func: &Function) -> usize {
        let fid = self.program.add_function(&func.name);

        for p in &func.params {
            let kind = if p.ty.is_pointer() || p.ty.is_array() {
                CrmmVarKind::Pointer
            } else {
                CrmmVarKind::Scalar
            };
            self.program.add_param(fid, &p.name, kind);
        }

        if let Some(body) = &func.body {
            for stmt in body {
                self.lower_stmt(fid, stmt);
            }
        }

        fid
    }

    fn lower_stmt(&mut self, fid: usize, stmt: &Stmt) {
        match stmt {
            Stmt::Decl(decls, _) => {
                for d in decls {
                    let kind = if d.ty.is_pointer() || d.ty.is_array() {
                        CrmmVarKind::Pointer
                    } else {
                        CrmmVarKind::Scalar
                    };
                    self.program.add_local(fid, &d.name, kind);

                    if let Some(init) = &d.init {
                        self.lower_assignment(fid, &d.name, init);
                    }
                }
            }
            Stmt::Expr(expr, _) => {
                self.lower_expr(fid, expr);
            }
            Stmt::Block(stmts, _) => {
                for s in stmts {
                    self.lower_stmt(fid, s);
                }
            }
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.lower_stmt(fid, then_branch);
                if let Some(eb) = else_branch {
                    self.lower_stmt(fid, eb);
                }
            }
            Stmt::While { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::For { body, .. }
            | Stmt::Switch { body, .. } => {
                self.lower_stmt(fid, body);
            }
            Stmt::Return(Some(expr), _) => {
                self.lower_expr(fid, expr);
            }
            _ => {}
        }
    }

    fn lower_assignment(&mut self, fid: usize, target: &str, rhs: &Expr) {
        match rhs {
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "malloc" || name == "calloc" || name == "realloc" {
                        let size = if let Some(arg0) = args.first() {
                            match arg0 {
                                Expr::IntLit(n, _) => *n as usize,
                                _ => 64, // symbolic / default
                            }
                        } else {
                            64
                        };
                        self.program.emit(
                            fid,
                            CrmmOp::Alloc {
                                target: target.to_string(),
                                size,
                            },
                        );
                        return;
                    }
                }
                self.program.emit(
                    fid,
                    CrmmOp::Assign {
                        target: target.to_string(),
                        source: format!("{:?}", rhs),
                    },
                );
            }
            Expr::Cast { expr, .. } => {
                self.lower_assignment(fid, target, expr);
            }
            Expr::Ident(src, _) => {
                self.program.emit(
                    fid,
                    CrmmOp::Assign {
                        target: target.to_string(),
                        source: src.clone(),
                    },
                );
            }
            _ => {
                self.program.emit(
                    fid,
                    CrmmOp::Assign {
                        target: target.to_string(),
                        source: format!("{:?}", rhs),
                    },
                );
            }
        }
    }

    fn lower_expr(&mut self, fid: usize, expr: &Expr) {
        match expr {
            Expr::Assign {
                op: AssignOp::Assign,
                lhs,
                rhs,
                ..
            } => {
                if let Expr::Ident(target, _) = lhs.as_ref() {
                    self.lower_assignment(fid, target, rhs);
                }
            }
            Expr::Assign {
                op: AssignOp::AddAssign,
                lhs,
                rhs,
                ..
            } => {
                if let Expr::Ident(target, _) = lhs.as_ref() {
                    if let Expr::IntLit(delta, _) = rhs.as_ref() {
                        self.program.emit(
                            fid,
                            CrmmOp::PtrArith {
                                target: target.clone(),
                                delta: *delta,
                            },
                        );
                    }
                }
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "free" {
                        if let Some(Expr::Ident(p, _)) = args.first() {
                            self.program.emit(
                                fid,
                                CrmmOp::Free {
                                    target: p.clone(),
                                },
                            );
                        }
                    }
                }
            }
            Expr::Unary {
                op: UnaryOp::Deref,
                expr,
                ..
            } => {
                if let Expr::Ident(target, _) = expr.as_ref() {
                    self.program.emit(
                        fid,
                        CrmmOp::Deref {
                            target: target.clone(),
                        },
                    );
                }
            }
            Expr::Index { base, index, .. } => {
                if let Expr::Ident(target, _) = base.as_ref() {
                    let idx = if let Expr::IntLit(n, _) = index.as_ref() {
                        *n as usize
                    } else {
                        0
                    };
                    self.program.emit(
                        fid,
                        CrmmOp::ArrayAccess {
                            target: target.clone(),
                            index: idx,
                        },
                    );
                }
            }
            _ => {}
        }
    }
}

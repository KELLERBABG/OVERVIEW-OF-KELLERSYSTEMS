//! SMT-LIB 2.6 Constraint Generation Engine for Keller-VIR.
//!
//! Emits bitvector (QF_BV), linear integer (QF_LIA), and memory/array constraints (QF_ABV).

use crate::vir::ast::*;

pub struct SmtEmitter {
    logic: &'static str,
    declarations: Vec<String>,
    assertions: Vec<String>,
}

impl SmtEmitter {
    pub fn new(logic: &'static str) -> Self {
        Self {
            logic,
            declarations: Vec::new(),
            assertions: Vec::new(),
        }
    }

    pub fn declare_const(&mut self, name: &str, sort: &str) {
        let decl = format!("(declare-const {} {})", name, sort);
        if !self.declarations.contains(&decl) {
            self.declarations.push(decl);
        }
    }

    pub fn assert_formula(&mut self, expr: &str) {
        self.assertions.push(format!("(assert {})", expr));
    }

    pub fn is_bitvector_logic(&self) -> bool {
        self.logic.contains("BV")
    }

    pub fn lower_expr(&mut self, expr: &VirExpr) -> String {
        let is_bv = self.is_bitvector_logic();
        match expr {
            VirExpr::Var(name) => name.clone(),
            VirExpr::ConstInt(val, bits) => {
                if is_bv && *bits > 0 {
                    format!("(_ bv{} {})", val, bits)
                } else if *val < 0 {
                    format!("(- {})", val.abs())
                } else {
                    val.to_string()
                }
            }
            VirExpr::ConstFloat(s) => {
                if s.starts_with('-') {
                    format!("(- {})", &s[1..])
                } else {
                    s.clone()
                }
            }
            VirExpr::ConstBool(b) => if *b { "true" } else { "false" }.to_string(),
            VirExpr::Binary(op, lhs, rhs) => {
                let l_smt = self.lower_expr(lhs);
                let r_smt = self.lower_expr(rhs);
                let op_smt = if is_bv {
                    match op {
                        VirBinaryOp::Add => "bvadd",
                        VirBinaryOp::Sub => "bvsub",
                        VirBinaryOp::Mul => "bvmul",
                        VirBinaryOp::Div => "bvudiv",
                        VirBinaryOp::Rem => "bvurem",
                        VirBinaryOp::BitAnd => "bvand",
                        VirBinaryOp::BitOr => "bvor",
                        VirBinaryOp::BitXor => "bvxor",
                        VirBinaryOp::Shl => "bvshl",
                        VirBinaryOp::Shr => "bvlshr",
                        VirBinaryOp::Eq => "=",
                        VirBinaryOp::Ne => "distinct",
                        VirBinaryOp::Lt => "bvult",
                        VirBinaryOp::Le => "bvule",
                        VirBinaryOp::Gt => "bvugt",
                        VirBinaryOp::Ge => "bvuge",
                        VirBinaryOp::And => "and",
                        VirBinaryOp::Or => "or",
                    }
                } else {
                    match op {
                        VirBinaryOp::Add => "+",
                        VirBinaryOp::Sub => "-",
                        VirBinaryOp::Mul => "*",
                        VirBinaryOp::Div => "div",
                        VirBinaryOp::Rem => "mod",
                        VirBinaryOp::BitAnd => "bvand",
                        VirBinaryOp::BitOr => "bvor",
                        VirBinaryOp::BitXor => "bvxor",
                        VirBinaryOp::Shl => "bvshl",
                        VirBinaryOp::Shr => "bvlshr",
                        VirBinaryOp::Eq => "=",
                        VirBinaryOp::Ne => "distinct",
                        VirBinaryOp::Lt => "<",
                        VirBinaryOp::Le => "<=",
                        VirBinaryOp::Gt => ">",
                        VirBinaryOp::Ge => ">=",
                        VirBinaryOp::And => "and",
                        VirBinaryOp::Or => "or",
                    }
                };
                format!("({} {} {})", op_smt, l_smt, r_smt)
            }
            VirExpr::Unary(VirUnaryOp::Neg, inner) => {
                let inner_smt = self.lower_expr(inner);
                if is_bv {
                    format!("(bvneg {inner_smt})")
                } else {
                    format!("(- {inner_smt})")
                }
            }
            VirExpr::Unary(VirUnaryOp::Not, inner) => {
                format!("(not {})", self.lower_expr(inner))
            }
            VirExpr::Ite(cond, then_e, else_e) => {
                let c = self.lower_expr(cond);
                let t = self.lower_expr(then_e);
                let e = self.lower_expr(else_e);
                format!("(ite {c} {t} {e})")
            }
            VirExpr::Load { mem_token, address, .. } => {
                let addr_smt = self.lower_expr(address);
                format!("(select {} {})", mem_token, addr_smt)
            }
            VirExpr::Call { target, args } => {
                let mut s = format!("({target}");
                for arg in args {
                    s.push(' ');
                    s.push_str(&self.lower_expr(arg));
                }
                s.push(')');
                s
            }
        }
    }

    pub fn emit_script(&self) -> String {
        let mut script = String::new();
        script.push_str(&format!("(set-logic {})\n", self.logic));
        script.push_str("(set-option :produce-models true)\n");
        for decl in &self.declarations {
            script.push_str(decl);
            script.push('\n');
        }
        for ass in &self.assertions {
            script.push_str(ass);
            script.push('\n');
        }
        script.push_str("(check-sat)\n(get-model)\n");
        script
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vir_smt_lowering() {
        let mut emitter = SmtEmitter::new("QF_BV");
        emitter.declare_const("x", "(_ BitVec 32)");
        emitter.declare_const("y", "(_ BitVec 32)");
        
        let expr = VirExpr::Binary(
            VirBinaryOp::Add,
            Box::new(VirExpr::Var("x".to_string())),
            Box::new(VirExpr::ConstInt(1, 32)),
        );
        let smt_expr = emitter.lower_expr(&expr);
        assert_eq!(smt_expr, "(bvadd x (_ bv1 32))");

        emitter.assert_formula(&format!("(= y {})", smt_expr));
        let script = emitter.emit_script();
        assert!(script.contains("(set-logic QF_BV)"));
        assert!(script.contains("(assert (= y (bvadd x (_ bv1 32))))"));
        assert!(script.contains("(check-sat)"));
    }
}

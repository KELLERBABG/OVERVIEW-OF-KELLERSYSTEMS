use crate::domain::Interval;
use crate::symbolic::SymbolicExpr;
use std::collections::HashMap;

pub struct AbstractInterpreter;

impl AbstractInterpreter {
    pub fn eval(expr: &SymbolicExpr, env: &HashMap<String, Interval>) -> Interval {
        match expr {
            SymbolicExpr::Variable(name) => *env.get(name).unwrap_or(&Interval::new(0.0, 0.0)),
            SymbolicExpr::Constant(c) => Interval::singleton(*c),
            SymbolicExpr::Add(l, r) => Self::eval(l, env).add(&Self::eval(r, env)),
            SymbolicExpr::Sub(l, r) => Self::eval(l, env).sub(&Self::eval(r, env)),
            SymbolicExpr::Mul(l, r) => Self::eval(l, env).mul(&Self::eval(r, env)),
            SymbolicExpr::Div(l, r) => Self::eval(l, env).div(&Self::eval(r, env)),
            SymbolicExpr::Neg(e) => {
                let i = Self::eval(e, env);
                Interval::new(-i.hi, -i.lo)
            }
            SymbolicExpr::Pow(e, n) => Self::eval(e, env).pow(*n),
            SymbolicExpr::Sin(e) => Self::eval(e, env).sin(),
            SymbolicExpr::Cos(e) => Self::eval(e, env).cos(),
            SymbolicExpr::Atan2(l, r) => Self::eval(l, env).atan2(&Self::eval(r, env)),
            SymbolicExpr::Sqrt(e) => Self::eval(e, env).sqrt(),
            SymbolicExpr::Abs(e) => {
                let i = Self::eval(e, env);
                Interval::new(0.0, i.abs())
            }
            SymbolicExpr::Tanh(e) => {
                let _ = Self::eval(e, env);
                Interval::new(-1.0, 1.0)
            }
        }
    }

    pub fn eval_bool(expr: &SymbolicExpr, env: &HashMap<String, Interval>) -> Option<bool> {
        let i = Self::eval(expr, env);
        if i.lo > 0.0 {
            Some(true)
        } else if i.hi <= 0.0 {
            Some(false)
        } else {
            None
        }
    }
}

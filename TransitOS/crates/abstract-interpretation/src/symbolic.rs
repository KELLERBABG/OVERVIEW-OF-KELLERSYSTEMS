use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolicExpr {
    Variable(String),
    Constant(f64),
    Add(Box<SymbolicExpr>, Box<SymbolicExpr>),
    Sub(Box<SymbolicExpr>, Box<SymbolicExpr>),
    Mul(Box<SymbolicExpr>, Box<SymbolicExpr>),
    Div(Box<SymbolicExpr>, Box<SymbolicExpr>),
    Neg(Box<SymbolicExpr>),
    Pow(Box<SymbolicExpr>, i32),
    Sin(Box<SymbolicExpr>),
    Cos(Box<SymbolicExpr>),
    Atan2(Box<SymbolicExpr>, Box<SymbolicExpr>),
    Sqrt(Box<SymbolicExpr>),
    Abs(Box<SymbolicExpr>),
    Tanh(Box<SymbolicExpr>),
}

pub type SymbolicVar = String;

pub fn var(name: &str) -> SymbolicExpr {
    SymbolicExpr::Variable(name.to_string())
}

pub fn constant(val: f64) -> SymbolicExpr {
    SymbolicExpr::Constant(val)
}

impl Add for SymbolicExpr {
    type Output = SymbolicExpr;
    fn add(self, other: SymbolicExpr) -> SymbolicExpr {
        SymbolicExpr::Add(Box::new(self), Box::new(other))
    }
}

impl Sub for SymbolicExpr {
    type Output = SymbolicExpr;
    fn sub(self, other: SymbolicExpr) -> SymbolicExpr {
        SymbolicExpr::Sub(Box::new(self), Box::new(other))
    }
}

impl Mul for SymbolicExpr {
    type Output = SymbolicExpr;
    fn mul(self, other: SymbolicExpr) -> SymbolicExpr {
        SymbolicExpr::Mul(Box::new(self), Box::new(other))
    }
}

impl Div for SymbolicExpr {
    type Output = SymbolicExpr;
    fn div(self, other: SymbolicExpr) -> SymbolicExpr {
        SymbolicExpr::Div(Box::new(self), Box::new(other))
    }
}

impl SymbolicExpr {
    pub fn pow(self, n: i32) -> SymbolicExpr {
        SymbolicExpr::Pow(Box::new(self), n)
    }

    pub fn substitute(&self, subst: &HashMap<String, SymbolicExpr>) -> SymbolicExpr {
        match self {
            SymbolicExpr::Variable(name) => {
                subst.get(name).cloned().unwrap_or_else(|| self.clone())
            }
            SymbolicExpr::Constant(_) => self.clone(),
            SymbolicExpr::Add(l, r) => {
                SymbolicExpr::Add(Box::new(l.substitute(subst)), Box::new(r.substitute(subst)))
            }
            SymbolicExpr::Sub(l, r) => {
                SymbolicExpr::Sub(Box::new(l.substitute(subst)), Box::new(r.substitute(subst)))
            }
            SymbolicExpr::Mul(l, r) => {
                SymbolicExpr::Mul(Box::new(l.substitute(subst)), Box::new(r.substitute(subst)))
            }
            SymbolicExpr::Div(l, r) => {
                SymbolicExpr::Div(Box::new(l.substitute(subst)), Box::new(r.substitute(subst)))
            }
            SymbolicExpr::Neg(e) => SymbolicExpr::Neg(Box::new(e.substitute(subst))),
            SymbolicExpr::Pow(e, n) => SymbolicExpr::Pow(Box::new(e.substitute(subst)), *n),
            SymbolicExpr::Sin(e) => SymbolicExpr::Sin(Box::new(e.substitute(subst))),
            SymbolicExpr::Cos(e) => SymbolicExpr::Cos(Box::new(e.substitute(subst))),
            SymbolicExpr::Atan2(l, r) => {
                SymbolicExpr::Atan2(Box::new(l.substitute(subst)), Box::new(r.substitute(subst)))
            }
            SymbolicExpr::Sqrt(e) => SymbolicExpr::Sqrt(Box::new(e.substitute(subst))),
            SymbolicExpr::Abs(e) => SymbolicExpr::Abs(Box::new(e.substitute(subst))),
            SymbolicExpr::Tanh(e) => SymbolicExpr::Tanh(Box::new(e.substitute(subst))),
        }
    }

    pub fn vars(&self) -> Vec<String> {
        let mut result = Vec::new();
        self._collect_vars(&mut result);
        result.sort();
        result.dedup();
        result
    }

    fn _collect_vars(&self, acc: &mut Vec<String>) {
        match self {
            SymbolicExpr::Variable(n) => acc.push(n.clone()),
            SymbolicExpr::Constant(_) => {}
            SymbolicExpr::Add(l, r)
            | SymbolicExpr::Sub(l, r)
            | SymbolicExpr::Mul(l, r)
            | SymbolicExpr::Div(l, r)
            | SymbolicExpr::Atan2(l, r) => {
                l._collect_vars(acc);
                r._collect_vars(acc);
            }
            SymbolicExpr::Neg(e)
            | SymbolicExpr::Sin(e)
            | SymbolicExpr::Cos(e)
            | SymbolicExpr::Sqrt(e)
            | SymbolicExpr::Abs(e)
            | SymbolicExpr::Tanh(e)
            | SymbolicExpr::Pow(e, _) => e._collect_vars(acc),
        }
    }
}

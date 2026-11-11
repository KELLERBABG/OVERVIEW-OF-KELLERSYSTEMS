//! Symbolic nonlinear-real-arithmetic (NRA) DSL.
//!
//! All flight-path conditions — envelope violations, stall margins, thrust
//! limits, terrain conflicts — are expressed as [`Formula`]s over canonical
//! control-cycle variables (see [`crate::state`]). A formula is a boolean
//! expression tree over symbolic [`Expr`]s, which can be:
//!
//! * **ground-evaluated** against a concrete control-cycle snapshot by the
//!   [`crate::solver::StubSolver`], or
//! * **asserted into Z3** as native NRA constraints by the optional
//!   `z3`-feature backend.
//!
//! This module is dependency-free and `no_std`-compatible by construction.

use std::collections::BTreeMap;
use std::fmt;

/// A single canonical variable name in the flight-path formula namespace.
/// Variables live in a module-scoped namespace to keep formulas auditable:
/// `state` (physical state), `cmd` (pending actuator command), `his`
/// (horizon-independent state sequence), `sym` (symbolic input), `mag`
/// (magic-number provenance).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VarNS {
    State,
    Cmd,
    Horizon,
    Symbolic,
    Magic,
}

impl VarNS {
    pub fn prefix(self) -> &'static str {
        match self {
            VarNS::State => "state",
            VarNS::Cmd => "cmd",
            VarNS::Horizon => "his",
            VarNS::Symbolic => "sym",
            VarNS::Magic => "mag",
        }
    }
}

/// An expression node in the NRA DSL.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A floating-point constant.
    Const(f64),
    /// A canonical runtime variable.
    Var(String),
    /// Negation: `-x`.
    Neg(Box<Expr>),
    /// Addition: `a + b`.
    Add(Box<Expr>, Box<Expr>),
    /// Subtraction: `a - b`.
    Sub(Box<Expr>, Box<Expr>),
    /// Multiplication: `a * b`.
    Mul(Box<Expr>, Box<Expr>),
    /// Division: `a / b`.
    Div(Box<Expr>, Box<Expr>),
    /// Power: `a ^ b` (exponent is a constant integer at evaluation time).
    Pow(Box<Expr>, Box<Expr>),
    /// Absolute value.
    Abs(Box<Expr>),
    /// Minimum of two expressions.
    Min(Box<Expr>, Box<Expr>),
    /// Maximum of two expressions.
    Max(Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Variable with the [`VarNS::State`] namespace.
    pub fn state(name: &str) -> Self {
        Self::var(VarNS::State, name)
    }

    /// Variable with the [`VarNS::Cmd`] namespace.
    pub fn cmd(name: &str) -> Self {
        Self::var(VarNS::Cmd, name)
    }

    /// Variable with the [`VarNS::Horizon`] namespace.
    pub fn horizon(name: &str) -> Self {
        Self::var(VarNS::Horizon, name)
    }

    /// Variable with the [`VarNS::Symbolic`] namespace.
    pub fn symbolic(name: &str) -> Self {
        Self::var(VarNS::Symbolic, name)
    }

    /// Variable with the [`VarNS::Magic`] namespace.
    pub fn magic(name: &str) -> Self {
        Self::var(VarNS::Magic, name)
    }

    /// Build a namespaced canonical variable.
    pub fn var(ns: VarNS, name: &str) -> Self {
        Expr::Var(format!("{}.{}", ns.prefix(), name))
    }

    /// Collect all variable names appearing in this expression tree.
    pub fn free_vars(&self, out: &mut Vec<String>) {
        fn rec(e: &Expr, out: &mut Vec<String>) {
            match e {
                Expr::Const(_) => {}
                Expr::Var(name) => out.push(name.clone()),
                Expr::Neg(a) | Expr::Abs(a) | Expr::Pow(a, _) => rec(a, out),
                Expr::Add(a, b)
                | Expr::Sub(a, b)
                | Expr::Mul(a, b)
                | Expr::Div(a, b)
                | Expr::Min(a, b)
                | Expr::Max(a, b) => {
                    rec(a, out);
                    rec(b, out);
                }
            }
        }
        rec(self, out);
    }

    /// Recursively substitute expressions. Returns the rewritten tree.
    pub fn substitute(&self, env: &BTreeMap<String, Expr>) -> Expr {
        match self {
            Expr::Const(_) => self.clone(),
            Expr::Var(name) => env
                .get(name)
                .cloned()
                .unwrap_or_else(|| Expr::Var(name.clone())),
            Expr::Neg(a) => Expr::Neg(Box::new(a.substitute(env))),
            Expr::Abs(a) => Expr::Abs(Box::new(a.substitute(env))),
            Expr::Add(a, b) => {
                Expr::Add(Box::new(a.substitute(env)), Box::new(b.substitute(env)))
            }
            Expr::Sub(a, b) => {
                Expr::Sub(Box::new(a.substitute(env)), Box::new(b.substitute(env)))
            }
            Expr::Mul(a, b) => {
                Expr::Mul(Box::new(a.substitute(env)), Box::new(b.substitute(env)))
            }
            Expr::Div(a, b) => {
                Expr::Div(Box::new(a.substitute(env)), Box::new(b.substitute(env)))
            }
            Expr::Pow(a, b) => {
                Expr::Pow(Box::new(a.substitute(env)), Box::new(b.substitute(env)))
            }
            Expr::Min(a, b) => {
                Expr::Min(Box::new(a.substitute(env)), Box::new(b.substitute(env)))
            }
            Expr::Max(a, b) => {
                Expr::Max(Box::new(a.substitute(env)), Box::new(b.substitute(env)))
            }
        }
    }

    /// Ground-evaluate this expression against a variable environment.
    /// Returns `None` if a variable is missing or a division/domain error
    /// occurs (e.g. `Pow(x, 0.5)` with a negative base).
    pub fn eval(&self, env: &BTreeMap<String, f64>) -> Option<f64> {
        match self {
            Expr::Const(v) => Some(*v),
            Expr::Var(name) => env.get(name).copied(),
            Expr::Neg(a) => a.eval(env).map(|v| -v),
            Expr::Abs(a) => a.eval(env).map(f64::abs),
            Expr::Add(a, b) => Some(a.eval(env)? + b.eval(env)?),
            Expr::Sub(a, b) => Some(a.eval(env)? - b.eval(env)?),
            Expr::Mul(a, b) => Some(a.eval(env)? * b.eval(env)?),
            Expr::Div(a, b) => {
                let den = b.eval(env)?;
                if den.abs() < f64::EPSILON {
                    None
                } else {
                    Some(a.eval(env)? / den)
                }
            }
            Expr::Pow(a, b) => {
                let base = a.eval(env)?;
                let exp = b.eval(env)?;
                // Only constant-integer exponents are sound in ground eval.
                if exp.fract() == 0.0 && exp >= -64.0 && exp <= 64.0 {
                    Some(base.powi(exp as i32))
                } else if base >= 0.0 {
                    Some(base.powf(exp))
                } else {
                    None
                }
            }
            Expr::Min(a, b) => Some(a.eval(env)?.min(b.eval(env)?)),
            Expr::Max(a, b) => Some(a.eval(env)?.max(b.eval(env)?)),
        }
    }

    /// Human-readable canonical form for specs/recorders.
    pub fn to_canonical(&self) -> String {
        match self {
            Expr::Const(v) => {
                if v.fract() == 0.0 && v.abs() < 1e15 {
                    format!("{}", *v as i64)
                } else {
                    format!("{:.6}", v)
                }
            }
            Expr::Var(name) => name.clone(),
            Expr::Neg(a) => format!("(- {})", a.to_canonical()),
            Expr::Abs(a) => format!("(abs {})", a.to_canonical()),
            Expr::Add(a, b) => format!("(+ {} {})", a.to_canonical(), b.to_canonical()),
            Expr::Sub(a, b) => format!("(- {} {})", a.to_canonical(), b.to_canonical()),
            Expr::Mul(a, b) => format!("(* {} {})", a.to_canonical(), b.to_canonical()),
            Expr::Div(a, b) => format!("(/ {} {})", a.to_canonical(), b.to_canonical()),
            Expr::Pow(a, b) => format!("(pow {} {})", a.to_canonical(), b.to_canonical()),
            Expr::Min(a, b) => format!("(min {} {})", a.to_canonical(), b.to_canonical()),
            Expr::Max(a, b) => format!("(max {} {})", a.to_canonical(), b.to_canonical()),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

/// Comparison operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl CmpOp {
    pub fn symbol(self) -> &'static str {
        match self {
            CmpOp::Lt => "<",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Ge => ">=",
            CmpOp::Eq => "=",
            CmpOp::Ne => "!=",
        }
    }

    pub fn eval(self, a: f64, b: f64) -> bool {
        match self {
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
        }
    }
}

/// A comparison node `lhs OP rhs`.
#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    pub lhs: Expr,
    pub op: CmpOp,
    pub rhs: Expr,
}

impl Comparison {
    pub fn new(lhs: Expr, op: CmpOp, rhs: Expr) -> Self {
        Comparison { lhs, op, rhs }
    }

    pub fn free_vars(&self, out: &mut Vec<String>) {
        self.lhs.free_vars(out);
        self.rhs.free_vars(out);
    }

    pub fn substitute(&self, env: &BTreeMap<String, Expr>) -> Comparison {
        Comparison {
            lhs: self.lhs.substitute(env),
            op: self.op,
            rhs: self.rhs.substitute(env),
        }
    }

    pub fn eval(&self, env: &BTreeMap<String, f64>) -> Option<bool> {
        Some(self.op.eval(self.lhs.eval(env)?, self.rhs.eval(env)?))
    }

    pub fn to_canonical(&self) -> String {
        format!(
            "({} {} {})",
            self.op.symbol(),
            self.lhs.to_canonical(),
            self.rhs.to_canonical()
        )
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

/// A boolean formula over symbolic expressions: atoms are comparisons,
/// combined with AND/OR/NOT.
#[derive(Clone, Debug, PartialEq)]
pub enum Formula {
    /// Logical TRUE.
    True,
    /// Logical FALSE.
    False,
    /// An atomic comparison.
    Atom(Comparison),
    /// Negation.
    Not(Box<Formula>),
    /// Conjunction.
    And(Box<Formula>, Box<Formula>),
    /// Disjunction.
    Or(Box<Formula>, Box<Formula>),
}

impl Formula {
    pub fn atom(lhs: Expr, op: CmpOp, rhs: Expr) -> Self {
        Formula::Atom(Comparison::new(lhs, op, rhs))
    }

    pub fn not(f: Formula) -> Self {
        Formula::Not(Box::new(f))
    }

    pub fn and(a: Formula, b: Formula) -> Self {
        match (a, b) {
            (Formula::False, _) | (_, Formula::False) => Formula::False,
            (Formula::True, x) | (x, Formula::True) => x,
            (x, y) => Formula::And(Box::new(x), Box::new(y)),
        }
    }

    pub fn or(a: Formula, b: Formula) -> Self {
        match (a, b) {
            (Formula::True, _) | (_, Formula::True) => Formula::True,
            (Formula::False, x) | (x, Formula::False) => x,
            (x, y) => Formula::Or(Box::new(x), Box::new(y)),
        }
    }

    pub fn all(forms: impl IntoIterator<Item = Formula>) -> Formula {
        forms.into_iter().fold(Formula::True, Formula::and)
    }

    pub fn any(forms: impl IntoIterator<Item = Formula>) -> Formula {
        forms.into_iter().fold(Formula::False, Formula::or)
    }

    /// Flatten a conjunction (or the whole tree) into a clause vector,
    /// expanding nested ANDs. Useful for reporting.
    pub fn clauses<'a>(&'a self, out: &mut Vec<&'a Comparison>) {
        fn rec<'a>(f: &'a Formula, out: &mut Vec<&'a Comparison>) {
            match f {
                Formula::True | Formula::False => {}
                Formula::Atom(c) => out.push(c),
                Formula::Not(inner) => rec(inner, out),
                Formula::And(a, b) => {
                    rec(a, out);
                    rec(b, out);
                }
                Formula::Or(a, b) => {
                    rec(a, out);
                    rec(b, out);
                }
            }
        }
        rec(self, out);
    }

    pub fn free_vars(&self, out: &mut Vec<String>) {
        match self {
            Formula::True | Formula::False => {}
            Formula::Atom(c) => c.free_vars(out),
            Formula::Not(inner) => inner.free_vars(out),
            Formula::And(a, b) | Formula::Or(a, b) => {
                a.free_vars(out);
                b.free_vars(out);
            }
        }
    }

    pub fn substitute(&self, env: &BTreeMap<String, Expr>) -> Formula {
        match self {
            Formula::True | Formula::False => self.clone(),
            Formula::Atom(c) => Formula::Atom(c.substitute(env)),
            Formula::Not(inner) => Formula::Not(Box::new(inner.substitute(env))),
            Formula::And(a, b) => Formula::and(a.substitute(env), b.substitute(env)),
            Formula::Or(a, b) => Formula::or(a.substitute(env), b.substitute(env)),
        }
    }

    /// Ground-evaluate this formula. Variables missing from `env` or
    /// domain errors yield `None` (conservative "unknown").
    pub fn eval(&self, env: &BTreeMap<String, f64>) -> Option<bool> {
        match self {
            Formula::True => Some(true),
            Formula::False => Some(false),
            Formula::Atom(c) => c.eval(env),
            Formula::Not(inner) => inner.eval(env).map(|v| !v),
            Formula::And(a, b) => match (a.eval(env), b.eval(env)) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            },
            Formula::Or(a, b) => match (a.eval(env), b.eval(env)) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            },
        }
    }

    pub fn to_canonical(&self) -> String {
        match self {
            Formula::True => "true".to_string(),
            Formula::False => "false".to_string(),
            Formula::Atom(c) => c.to_canonical(),
            Formula::Not(inner) => format!("(not {})", inner.to_canonical()),
            Formula::And(a, b) => format!("(and {} {})", a.to_canonical(), b.to_canonical()),
            Formula::Or(a, b) => format!("(or {} {})", a.to_canonical(), b.to_canonical()),
        }
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

/// Convenience DSL for building formulas.
///
/// Note: the `ne` method is intentionally NOT provided. `Expr` implements
/// `PartialEq`, which provides `ne(&self, &Expr) -> bool`; Rust's method
/// resolution would prefer that over a consuming `Formula`-returning
/// `ne`, silently producing wrong types. Use `.gt(Expr::Const(0.0))` or
/// `.lt(...)` for non-zero comparisons instead.
pub trait FormulaBuilder {
    fn lt(self, rhs: Expr) -> Formula;
    fn le(self, rhs: Expr) -> Formula;
    fn gt(self, rhs: Expr) -> Formula;
    fn ge(self, rhs: Expr) -> Formula;
    fn eq(self, rhs: Expr) -> Formula;
}

impl FormulaBuilder for Expr {
    fn lt(self, rhs: Expr) -> Formula {
        Formula::atom(self, CmpOp::Lt, rhs)
    }
    fn le(self, rhs: Expr) -> Formula {
        Formula::atom(self, CmpOp::Le, rhs)
    }
    fn gt(self, rhs: Expr) -> Formula {
        Formula::atom(self, CmpOp::Gt, rhs)
    }
    fn ge(self, rhs: Expr) -> Formula {
        Formula::atom(self, CmpOp::Ge, rhs)
    }
    fn eq(self, rhs: Expr) -> Formula {
        Formula::atom(self, CmpOp::Eq, rhs)
    }
}

// Arithmetic operator impls over `Expr` keep formula construction readable.
impl std::ops::Add for Expr {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::Add(Box::new(self), Box::new(rhs))
    }
}

impl std::ops::Sub for Expr {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::Sub(Box::new(self), Box::new(rhs))
    }
}

impl std::ops::Mul for Expr {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::Mul(Box::new(self), Box::new(rhs))
    }
}

impl std::ops::Div for Expr {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::Div(Box::new(self), Box::new(rhs))
    }
}

impl std::ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::Neg(Box::new(self))
    }
}

// F64 constants can be combined with Expr directly (`Expr + 2.0`).
impl std::ops::Add<f64> for Expr {
    type Output = Expr;
    fn add(self, rhs: f64) -> Expr {
        self + Expr::Const(rhs)
    }
}

impl std::ops::Sub<f64> for Expr {
    type Output = Expr;
    fn sub(self, rhs: f64) -> Expr {
        self - Expr::Const(rhs)
    }
}

impl std::ops::Mul<f64> for Expr {
    type Output = Expr;
    fn mul(self, rhs: f64) -> Expr {
        self * Expr::Const(rhs)
    }
}

impl std::ops::Div<f64> for Expr {
    type Output = Expr;
    fn div(self, rhs: f64) -> Expr {
        self / Expr::Const(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_eval_arithmetic() {
        let x = Expr::state("q_deg");
        let y = Expr::state("nz");
        let inner = Expr::Max(
            Box::new(x.clone() * x.clone()),
            Box::new(Expr::Const(2.0)),
        );
        let e = Expr::Abs(Box::new(inner)) + y.clone() * 3.0;
        let mut env = BTreeMap::new();
        env.insert("state.q_deg".to_string(), 15.0);
        env.insert("state.nz".to_string(), -1.2);
        assert_eq!(e.eval(&env), Some(225.0 + (-3.6)));
    }

    #[test]
    fn formula_eval_and_or_not() {
        let x = Expr::state("q_deg");
        let f = Formula::and(
            x.clone().lt(Expr::Const(8.0)),
            Formula::or(
                Expr::state("nz").gt(Expr::Const(2.5)),
                Formula::not(Expr::magic("stall_margin").lt(Expr::Const(0.0))),
            ),
        );
        let mut env = BTreeMap::new();
        env.insert("state.q_deg".to_string(), 11.0);
        env.insert("state.nz".to_string(), 3.2);
        env.insert("mag.stall_margin".to_string(), 2.5);
        assert_eq!(f.eval(&env), Some(false));
    }

    #[test]
    fn canonical_forms_are_stable() {
        let f = (Expr::state("ias") * Expr::Const(1.94384)).ge(Expr::magic("envelope.vne_knots"));
        let s = f.to_canonical();
        assert!(s.contains("(>= (* state.ias"));
        assert_eq!(f.to_canonical(), s);
    }

    #[test]
    fn substitution_rewrites_vars() {
        let f = Expr::state("ias").gt(Expr::Const(50.0));
        let mut env = BTreeMap::new();
        env.insert("state.ias".to_string(), Expr::horizon("ias_1"));
        let sub = f.substitute(&env);
        let mut vars = Vec::new();
        sub.free_vars(&mut vars);
        assert!(vars.contains(&"his.ias_1".to_string()));
    }

    #[test]
    fn f64_ops_work() {
        let e = Expr::state("ias_knots") - 120.0;
        let mut env = BTreeMap::new();
        env.insert("state.ias_knots".to_string(), 250.0);
        assert_eq!(e.eval(&env), Some(130.0));
    }
}
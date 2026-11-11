//! SMT verification backends.
//!
//! The verification chain checks hazard formulas (see [`crate::formula`])
//! for satisfiability on the actual control-cycle state. Two backends
//! implement the [`SMTBackend`] trait:
//!
//! * [`StubSolver`] — ground evaluation of the concrete state. Sound for
//!   the runtime snapshot, dependency-free, always available.
//! * `Z3Backend` (feature `z3`) — full SMT-NRA through the Z3 solver with
//!   the concrete ground state asserted, so Z3 checks the nonlinear
//!   constraint *symbolically* on the exact snapshot.

use crate::formula::{Expr, Formula, VarNS};

/// Result of a satisfiability check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckResult {
    /// The formula is satisfiable — a hazard condition may hold.
    Sat,
    /// The formula is unsatisfiable — the hazard is excluded.
    Unsat,
    /// The backend could not produce a verdict (missing variable, domain
    /// error, solver failure). Treat as unsafe (conservative).
    Unknown,
}

impl CheckResult {
    pub fn is_sat(self) -> bool {
        self == CheckResult::Sat
    }

    pub fn is_unsat(self) -> bool {
        self == CheckResult::Unsat
    }
}

impl std::fmt::Display for CheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckResult::Sat => write!(f, "SAT"),
            CheckResult::Unsat => write!(f, "UNSAT"),
            CheckResult::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// A verification backend able to decide formulas over a variable
/// environment.
///
/// The trait requires `Debug` and a [`SMTBackend::clone_box`] so that the
/// trait object can be stored in the [`crate::gate::ExecutionGate`], which
/// derives `Clone`/`Debug` (the simulator holds the gate by value).
pub trait SMTBackend: std::fmt::Debug {
    /// Check a formula under a ground environment.
    ///
    /// The returned [`CheckResult`] must be **conservative**: a real
    /// satisfiability must never be reported as `Unsat`.
    fn check(
        &self,
        formula: &Formula,
        env: &std::collections::BTreeMap<String, f64>,
    ) -> CheckResult;

    /// Clone this backend as a boxed trait object.
    fn clone_box(&self) -> Box<dyn SMTBackend>;
}

/// Ground-evaluation backend (default).
///
/// Evaluates every comparison directly on the concrete control-cycle state.
/// Because the environment is complete by construction of
/// [`crate::state::ControlSnapshot`], evaluation is total in practice; any
/// missing variable or domain error yields `Unknown` (unsafe).
#[derive(Clone, Copy, Debug, Default)]
pub struct StubSolver;

impl StubSolver {
    pub fn new() -> Self {
        StubSolver
    }
}

impl SMTBackend for StubSolver {
    fn check(
        &self,
        formula: &Formula,
        env: &std::collections::BTreeMap<String, f64>,
    ) -> CheckResult {
        match formula.eval(env) {
            Some(true) => CheckResult::Sat,
            Some(false) => CheckResult::Unsat,
            None => CheckResult::Unknown,
        }
    }

    fn clone_box(&self) -> Box<dyn SMTBackend> {
        Box::new(*self)
    }
}

/// Helper to build a ground environment from a slice of `(name, value)`
/// pairs, mapping human-readable names into canonical namespaced variables.
pub fn namespaced_env(
    ns: VarNS,
    pairs: &[(&str, f64)],
) -> std::collections::BTreeMap<String, f64> {
    let mut env = std::collections::BTreeMap::new();
    for (name, value) in pairs {
        env.insert(format!("{}.{}", ns.prefix(), name), *value);
    }
    env
}

/// Gather free variables of a formula into a sorted unique list.
pub fn free_variables(formula: &Formula) -> Vec<String> {
    let mut vars = Vec::new();
    formula.free_vars(&mut vars);
    vars.sort();
    vars.dedup();
    vars
}

/// Report builder for a satisfiability verdict.
#[derive(Clone, Debug, PartialEq)]
pub struct VerdictReport {
    pub formula: Formula,
    pub free_vars: Vec<String>,
    pub result: CheckResult,
    pub backend: &'static str,
}

impl VerdictReport {
    pub fn new(
        formula: Formula,
        env: &std::collections::BTreeMap<String, f64>,
        backend: &'static str,
    ) -> Self {
        let result = StubSolver::new().check(&formula, env);
        let free_vars = free_variables(&formula);
        VerdictReport {
            formula,
            free_vars,
            result,
            backend,
        }
    }
}

/// Helper: substitute a ground env into a formula, returning a purely
/// boolean constant formula (useful for debugging/specs).
pub fn ground_substitute(
    formula: &Formula,
    env: &std::collections::BTreeMap<String, f64>,
) -> Formula {
    let sub = env
        .iter()
        .map(|(k, v)| (k.clone(), Expr::Const(*v)))
        .collect();
    formula.substitute(&sub)
}

#[cfg(feature = "z3")]
pub mod z3 {
    //! Real Z3 backend (feature `z3`).
    //!
    //! The concrete ground state is asserted first, so the solver operates
    //! on the exact control-cycle snapshot and cannot wander the whole
    //! state space.
    //!
    //! **Experimental**: requires the Z3 native library. Not exercised by
    //! the default test suite.

    use super::*;
    use crate::formula::{CmpOp, Expr, Formula};
    use z3::{ast::Bool, ast::Real, Config, Context, SatResult, Solver};

    /// Z3-backed SMT-NRA checker.
    pub struct Z3Backend {
        ctx: Context,
    }

    impl Z3Backend {
        pub fn new() -> Result<Self, String> {
            let cfg = Config::new();
            let ctx = Context::new(&cfg);
            Ok(Z3Backend { ctx })
        }
    }

    impl std::fmt::Debug for Z3Backend {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Z3Backend")
        }
    }

    fn real_expr<'ctx>(ctx: &'ctx Context, e: &Expr) -> Result<Real<'ctx>, String> {
        Ok(match e {
            Expr::Const(v) => Real::from_real(ctx, *v as i64, 1),
            Expr::Var(name) => Real::new_const(ctx, name),
            Expr::Neg(a) => real_expr(ctx, a)?.neg(),
            Expr::Abs(a) => real_expr(ctx, a)?.abs(),
            Expr::Add(a, b) => real_expr(ctx, a)?.add(&real_expr(ctx, b)?),
            Expr::Sub(a, b) => real_expr(ctx, a)?.sub(&real_expr(ctx, b)?),
            Expr::Mul(a, b) => real_expr(ctx, a)?.mul(&real_expr(ctx, b)?),
            Expr::Div(a, b) => real_expr(ctx, a)?.div(&real_expr(ctx, b)?),
            Expr::Pow(a, b) => {
                let base = real_expr(ctx, a)?;
                let exp = real_expr(ctx, b)?;
                match b {
                    Expr::Const(v) if v.fract() == 0.0 && v.abs() <= 8.0 => {
                        base.powi(v as i32)
                    }
                    _ => {
                        let zero = Real::from_real(ctx, 0, 1);
                        let one = Real::from_real(ctx, 1, 1);
                        base.ge(&zero)
                            .ite(&exp.mul(&base.log().ok_or("log domain")?).exp(), &one)
                    }
                }
            }
            Expr::Min(a, b) => {
                let la = real_expr(ctx, a)?;
                let lb = real_expr(ctx, b)?;
                la.le(&lb).ite(&la, &lb)
            }
            Expr::Max(a, b) => {
                let la = real_expr(ctx, a)?;
                let lb = real_expr(ctx, b)?;
                la.ge(&lb).ite(&la, &lb)
            }
        })
    }

    fn bool_expr<'ctx>(ctx: &'ctx Context, f: &Formula) -> Result<Bool<'ctx>, String> {
        Ok(match f {
            Formula::True => Bool::from_bool(ctx, true),
            Formula::False => Bool::from_bool(ctx, false),
            Formula::Atom(c) => {
                let lhs = real_expr(ctx, &c.lhs)?;
                let rhs = real_expr(ctx, &c.rhs)?;
                match c.op {
                    CmpOp::Lt => lhs.lt(&rhs),
                    CmpOp::Le => lhs.le(&rhs),
                    CmpOp::Gt => lhs.gt(&rhs),
                    CmpOp::Ge => lhs.ge(&rhs),
                    CmpOp::Eq => lhs._eq(&rhs),
                    CmpOp::Ne => lhs._eq(&rhs).not(),
                }
            }
            Formula::Not(inner) => bool_expr(ctx, inner)?.not(),
            Formula::And(a, b) => bool_expr(ctx, a)?.and(&bool_expr(ctx, b)?),
            Formula::Or(a, b) => bool_expr(ctx, a)?.or(&bool_expr(ctx, b)?),
        })
    }

    impl SMTBackend for Z3Backend {
        fn check(
            &self,
            formula: &Formula,
            env: &std::collections::BTreeMap<String, f64>,
        ) -> CheckResult {
            let solver = Solver::new(&self.ctx);
            for (name, value) in env {
                let var = Real::new_const(&self.ctx, name);
                solver.assert(&var._eq(&Real::from_real(&self.ctx, *value as i64, 1)));
            }
            let f_as_bool = match bool_expr(&self.ctx, formula) {
                Ok(b) => b,
                Err(_) => return CheckResult::Unknown,
            };
            solver.assert(&f_as_bool);
            match solver.check() {
                SatResult::Sat => CheckResult::Sat,
                SatResult::Unsat => CheckResult::Unsat,
                _ => CheckResult::Unknown,
            }
        }

        fn clone_box(&self) -> Box<dyn SMTBackend> {
            Box::new(Z3Backend::new().expect("z3 backend re-initialization failed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::FormulaBuilder;

    #[test]
    fn stub_verdicts() {
        let f = Expr::state("ias").gt(Expr::magic("envelope.vne_knots"));

        // SAT: 305 > 300.
        let mut sat_env = namespaced_env(VarNS::State, &[("ias", 305.0)]);
        sat_env.insert("mag.envelope.vne_knots".to_string(), 300.0);
        assert_eq!(StubSolver::new().check(&f, &sat_env), CheckResult::Sat);

        // UNSAT: 295 < 300.
        let mut unsat_env = namespaced_env(VarNS::State, &[("ias", 295.0)]);
        unsat_env.insert("mag.envelope.vne_knots".to_string(), 300.0);
        assert_eq!(StubSolver::new().check(&f, &unsat_env), CheckResult::Unsat);
    }

    #[test]
    fn missing_var_is_conservative() {
        let f = Expr::state("ias").gt(Expr::magic("envelope.vne_knots"));
        let env = namespaced_env(VarNS::State, &[("ias", 305.0)]);
        assert_eq!(StubSolver::new().check(&f, &env), CheckResult::Unknown);
    }

    #[test]
    fn free_vars_is_sorted_unique() {
        let f = Formula::and(
            Expr::state("b").gt(Expr::magic("x")),
            Expr::state("a").lt(Expr::magic("x")),
        );
        assert_eq!(
            free_variables(&f),
            vec![
                "mag.x".to_string(),
                "state.a".to_string(),
                "state.b".to_string()
            ]
        );
    }

    #[test]
    fn clone_box_works() {
        let s = StubSolver::new();
        let b: Box<dyn SMTBackend> = s.clone_box();
        assert!(format!("{:?}", b).contains("StubSolver"));
    }
}
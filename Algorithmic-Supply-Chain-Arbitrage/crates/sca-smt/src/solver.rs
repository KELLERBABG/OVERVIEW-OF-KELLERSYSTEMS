//! Solver trait and the in-process exact solver.
//!
//! Defines the common interface for SMT solvers and provides `ExactSolver`,
//! a sound and complete decision procedure for the `ConstraintExpr` language
//! that requires no external dependencies (no Z3, no cmake).
//!
//! ## Why this is exact
//!
//! Every leaf of `sca_types::ConstraintExpr` is a *unary* atom `var <op> const`.
//! Constraints never relate two variables to each other, so each variable is
//! an independent real. That makes the theory decidable by a complete method:
//!
//! 1. Group the atoms by variable.
//! 2. For each variable, enumerate the atom truth-vectors that some real value
//!    can actually produce. The truth of every atom on a variable is constant
//!    across each region of the real line cut by that variable's constants, so
//!    it suffices to sample every open region and every constant.
//! 3. Search the (small) product of per-variable truth-vectors for one that
//!    satisfies the boolean structure.
//!
//! The previous implementation assigned the *same* value to every variable and
//! tried seven fixed starting points, so a genuinely feasible path such as
//! `capacity >= 100 AND temperature <= 8` was reported unsatisfiable. Path
//! verification therefore produced false negatives.

use sca_types::{ComparisonOp, ConstraintExpr, ConstraintVariable};
use std::collections::HashMap;

/// Upper bound on the product of per-variable truth-vectors.
///
/// Real logistic constraint sets are tiny (a handful of variables, a few atoms
/// each), so this is never approached in practice. It exists only so a
/// pathological input degrades into an explicit "cannot decide" answer instead
/// of hanging or exhausting memory.
const MAX_COMBINATIONS: usize = 5_000_000;

/// Result of a solver query.
#[derive(Debug, Clone, PartialEq)]
pub struct SolverResult {
    /// Whether the constraint set is satisfiable.
    pub satisfiable: bool,
    /// A model (variable assignments) if satisfiable, or explanation if not.
    pub model: HashMap<ConstraintVariable, f64>,
    /// Human-readable explanation (useful for unsatisfiable cases).
    pub explanation: Option<String>,
}

/// Abstract SMT solver interface.
pub trait Solver: Send + Sync {
    /// Check a set of constraints for satisfiability.
    /// Returns a model (variable assignments) if satisfiable,
    /// or an explanation of unsatisfiability.
    fn check(&self, constraints: &[ConstraintExpr]) -> SolverResult;

    /// Find the minimum value of a variable subject to constraints.
    fn minimize(&self, constraints: &[ConstraintExpr], variable: &ConstraintVariable) -> SolverResult;

    /// Find the maximum value of a variable subject to constraints.
    fn maximize(&self, constraints: &[ConstraintExpr], variable: &ConstraintVariable) -> SolverResult;
}

/// An atomic comparison `variable <op> value`.
#[derive(Debug, Clone, Copy)]
struct Atom {
    /// Index into `Problem::variables`.
    variable: usize,
    op: ComparisonOp,
    value: f64,
}

impl Atom {
    fn holds(&self, x: f64) -> bool {
        self.op.evaluate(x, self.value)
    }
}

/// Boolean structure over atom indices.
#[derive(Debug, Clone)]
enum Formula {
    Literal(bool),
    Atom(usize),
    And(Box<Formula>, Box<Formula>),
    Or(Box<Formula>, Box<Formula>),
    Not(Box<Formula>),
}

impl Formula {
    fn eval(&self, truths: &[bool]) -> bool {
        match self {
            Formula::Literal(b) => *b,
            Formula::Atom(i) => truths[*i],
            Formula::And(a, b) => a.eval(truths) && b.eval(truths),
            Formula::Or(a, b) => a.eval(truths) || b.eval(truths),
            Formula::Not(a) => !a.eval(truths),
        }
    }
}

/// A compiled constraint set: atoms, the variable table, and the boolean form.
#[derive(Debug, Clone)]
struct Problem {
    atoms: Vec<Atom>,
    variables: Vec<ConstraintVariable>,
    formula: Formula,
    /// Atom indices grouped by variable, in a stable order.
    atoms_by_variable: Vec<Vec<usize>>,
}

impl Problem {
    fn compile(constraints: &[ConstraintExpr]) -> Self {
        let mut atoms = Vec::new();
        let mut variables: Vec<ConstraintVariable> = Vec::new();

        let mut formula = Formula::Literal(true);
        for constraint in constraints {
            let compiled = compile_expr(constraint, &mut atoms, &mut variables);
            formula = Formula::And(Box::new(formula), Box::new(compiled));
        }

        let mut atoms_by_variable = vec![Vec::new(); variables.len()];
        for (idx, atom) in atoms.iter().enumerate() {
            atoms_by_variable[atom.variable].push(idx);
        }

        Self { atoms, variables, formula, atoms_by_variable }
    }

    fn variable_index(&self, variable: &ConstraintVariable) -> Option<usize> {
        self.variables.iter().position(|v| v == variable)
    }

    /// Total number of candidate combinations, or `None` on overflow.
    fn combination_count(&self, fixed: &HashMap<usize, f64>) -> Option<usize> {
        let mut total: usize = 1;
        for (vidx, atom_indices) in self.atoms_by_variable.iter().enumerate() {
            if fixed.contains_key(&vidx) {
                continue;
            }
            total = total.checked_mul(domain_size(self, atom_indices))?;
        }
        Some(total)
    }
}

fn compile_expr(expr: &ConstraintExpr, atoms: &mut Vec<Atom>, variables: &mut Vec<ConstraintVariable>) -> Formula {
    match expr {
        ConstraintExpr::Literal(b) => Formula::Literal(*b),
        ConstraintExpr::Comparison { variable, op, value } => {
            let vidx = match variables.iter().position(|v| v == variable) {
                Some(i) => i,
                None => {
                    variables.push(variable.clone());
                    variables.len() - 1
                }
            };
            let idx = atoms.len();
            atoms.push(Atom { variable: vidx, op: *op, value: *value });
            Formula::Atom(idx)
        }
        ConstraintExpr::And(a, b) => {
            Formula::And(Box::new(compile_expr(a, atoms, variables)), Box::new(compile_expr(b, atoms, variables)))
        }
        ConstraintExpr::Or(a, b) => {
            Formula::Or(Box::new(compile_expr(a, atoms, variables)), Box::new(compile_expr(b, atoms, variables)))
        }
        ConstraintExpr::Not(a) => Formula::Not(Box::new(compile_expr(a, atoms, variables))),
    }
}

/// A representative point of one region of a variable's domain.
#[derive(Debug, Clone)]
struct Sample {
    /// A concrete value inside the region (used as the witness in the model).
    witness: f64,
    /// Atom truth values at that value.
    truths: Vec<bool>,
    /// Infimum of the region (`-inf` when unbounded below).
    lower: f64,
    /// Supremum of the region (`+inf` when unbounded above).
    upper: f64,
}

/// Number of distinct regions a variable's domain is cut into.
fn domain_size(problem: &Problem, atom_indices: &[usize]) -> usize {
    2 * distinct_constants(problem, atom_indices).len() + 1
}

fn distinct_constants(problem: &Problem, atom_indices: &[usize]) -> Vec<f64> {
    let mut constants: Vec<f64> = atom_indices.iter().map(|&i| problem.atoms[i].value).collect();
    constants.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    constants.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    constants
}

/// Enumerate the distinct regions of a variable's domain in ascending order.
///
/// Between two consecutive constants the truth of every atom is constant, and
/// at a constant it is evaluated exactly (which is what makes `Eq`/`Ne` sound).
fn samples_for(problem: &Problem, atom_indices: &[usize]) -> Vec<Sample> {
    let constants = distinct_constants(problem, atom_indices);

    // Only the ordering relative to the constants matters, so stepping by 1
    // past either end always lands in the unbounded region.
    let mut points: Vec<(f64, f64, f64)> = Vec::new(); // (witness, lower, upper)
    if constants.is_empty() {
        points.push((0.0, f64::NEG_INFINITY, f64::INFINITY));
    } else {
        let first = constants[0];
        let last = constants[constants.len() - 1];
        points.push((first - 1.0, f64::NEG_INFINITY, first));
        points.push((first, first, first));
        for pair in constants.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            points.push(((a + b) / 2.0, a, b));
            points.push((b, b, b));
        }
        points.push((last + 1.0, last, f64::INFINITY));
    }

    let mut samples = Vec::with_capacity(points.len());
    for (witness, lower, upper) in points {
        let truths = atom_indices.iter().map(|&i| problem.atoms[i].holds(witness)).collect();
        samples.push(Sample { witness, truths, lower, upper });
    }

    // Drop duplicate truth-vectors; keep the first (lowest) representative.
    let mut unique: Vec<Sample> = Vec::with_capacity(samples.len());
    for sample in samples {
        if !unique.iter().any(|s| s.truths == sample.truths) {
            unique.push(sample);
        }
    }
    unique
}

/// The in-process exact solver.
///
/// Implements the `Solver` trait with no external dependencies and no
/// heuristic search: it decides the constraint language exactly, and reports
/// exact optima (infimum/supremum) for `minimize`/`maximize`.
#[derive(Debug, Clone, Default)]
pub struct ExactSolver {
    /// Variable values that are fixed rather than solved for.
    pinned: HashMap<ConstraintVariable, f64>,
}

impl ExactSolver {
    pub fn new() -> Self {
        Self { pinned: HashMap::new() }
    }

    /// Pin a variable to a concrete value (e.g. an observed sensor reading).
    pub fn with_var(mut self, var: ConstraintVariable, value: f64) -> Self {
        self.pinned.insert(var, value);
        self
    }

    /// The currently pinned assignments.
    pub fn pinned(&self) -> &HashMap<ConstraintVariable, f64> {
        &self.pinned
    }

    fn pinned_indices(&self, problem: &Problem) -> HashMap<usize, f64> {
        let mut fixed = HashMap::new();
        for (variable, &value) in &self.pinned {
            if let Some(idx) = problem.variable_index(variable) {
                fixed.insert(idx, value);
            }
        }
        fixed
    }

    /// Decide satisfiability and return a witness model, if one exists.
    fn solve(&self, problem: &Problem, extra_fixed: &HashMap<usize, f64>) -> Option<HashMap<ConstraintVariable, f64>> {
        let mut fixed = self.pinned_indices(problem);
        for (&idx, &value) in extra_fixed {
            fixed.insert(idx, value);
        }

        match problem.combination_count(&fixed) {
            Some(n) if n <= MAX_COMBINATIONS => {}
            _ => return None,
        }

        // Regions per variable, in the same order as `problem.variables`.
        let domains: Vec<Vec<Sample>> = problem
            .atoms_by_variable
            .iter()
            .enumerate()
            .map(|(vidx, atom_indices)| match fixed.get(&vidx) {
                // A pinned variable has exactly one reachable truth-vector.
                Some(&value) => vec![Sample {
                    witness: value,
                    truths: atom_indices.iter().map(|&i| problem.atoms[i].holds(value)).collect(),
                    lower: value,
                    upper: value,
                }],
                None => samples_for(problem, atom_indices),
            })
            .collect();

        let mut truths = vec![false; problem.atoms.len()];
        let mut model: HashMap<ConstraintVariable, f64> = HashMap::new();
        // Seed the model with the pinned values so callers always see them.
        for (variable, &value) in &self.pinned {
            model.insert(variable.clone(), value);
        }

        if search(problem, &domains, 0, &mut truths, &mut model) {
            for (&idx, &value) in extra_fixed {
                model.insert(problem.variables[idx].clone(), value);
            }
            Some(model)
        } else {
            None
        }
    }

    /// Ordered regions of one variable, collapsing to a single region when pinned.
    fn regions_for(&self, problem: &Problem, vidx: usize) -> Vec<Sample> {
        match self.pinned_indices(problem).get(&vidx) {
            Some(&value) => vec![Sample { witness: value, truths: Vec::new(), lower: value, upper: value }],
            None => samples_for(problem, &problem.atoms_by_variable[vidx]),
        }
    }
}

/// Depth-first product search over per-variable regions.
fn search(
    problem: &Problem,
    domains: &[Vec<Sample>],
    idx: usize,
    truths: &mut Vec<bool>,
    model: &mut HashMap<ConstraintVariable, f64>,
) -> bool {
    if idx == domains.len() {
        return problem.formula.eval(truths);
    }

    for sample in &domains[idx] {
        let atom_indices = &problem.atoms_by_variable[idx];
        for (slot, truth) in sample.truths.iter().enumerate() {
            truths[atom_indices[slot]] = *truth;
        }
        let previous = model.insert(problem.variables[idx].clone(), sample.witness);
        if search(problem, domains, idx + 1, truths, model) {
            return true;
        }
        match previous {
            Some(value) => {
                model.insert(problem.variables[idx].clone(), value);
            }
            None => {
                model.remove(&problem.variables[idx]);
            }
        }
    }

    false
}

impl Solver for ExactSolver {
    fn check(&self, constraints: &[ConstraintExpr]) -> SolverResult {
        let problem = Problem::compile(constraints);

        let combinations = problem.combination_count(&self.pinned_indices(&problem));
        if combinations.is_none_or(|n| n > MAX_COMBINATIONS) {
            return SolverResult {
                satisfiable: false,
                model: HashMap::new(),
                explanation: Some(format!(
                    "Constraint space exceeds {MAX_COMBINATIONS} combinations; the exact in-process solver cannot decide it"
                )),
            };
        }

        match self.solve(&problem, &HashMap::new()) {
            Some(model) => SolverResult { satisfiable: true, model, explanation: None },
            None => SolverResult {
                satisfiable: false,
                model: self.pinned.clone(),
                explanation: Some(describe_unsatisfiable(constraints)),
            },
        }
    }

    fn minimize(&self, constraints: &[ConstraintExpr], variable: &ConstraintVariable) -> SolverResult {
        optimize(self, constraints, variable, Optimization::Minimize)
    }

    fn maximize(&self, constraints: &[ConstraintExpr], variable: &ConstraintVariable) -> SolverResult {
        optimize(self, constraints, variable, Optimization::Maximize)
    }
}

enum Optimization {
    Minimize,
    Maximize,
}

fn optimize(
    solver: &ExactSolver,
    constraints: &[ConstraintExpr],
    variable: &ConstraintVariable,
    direction: Optimization,
) -> SolverResult {
    let problem = Problem::compile(constraints);

    // Reject outright-infeasible constraint sets before optimising.
    if solver.solve(&problem, &HashMap::new()).is_none() {
        return SolverResult {
            satisfiable: false,
            model: solver.pinned.clone(),
            explanation: Some(describe_unsatisfiable(constraints)),
        };
    }

    let Some(vidx) = problem.variable_index(variable) else {
        return SolverResult {
            satisfiable: true,
            model: solver.pinned.clone(),
            explanation: Some(format!("{variable:?} does not appear in the constraints and is therefore unbounded")),
        };
    };

    let regions = solver.regions_for(&problem, vidx);

    // A pinned variable has a single region: its optimum is the pinned value.
    if regions.len() == 1 {
        let sample = &regions[0];
        let mut model = solver.solve(&problem, &HashMap::new()).unwrap_or_default();
        model.insert(variable.clone(), sample.witness);
        return SolverResult {
            satisfiable: true,
            model,
            explanation: Some(format!("{variable:?} is pinned to {}", sample.witness)),
        };
    }

    // Regions are produced in ascending order and the truth of the constraint
    // set is constant across each, so the first feasible region carries the
    // optimum at the relevant edge.
    let ordered: Vec<&Sample> = match direction {
        Optimization::Minimize => regions.iter().collect(),
        Optimization::Maximize => regions.iter().rev().collect(),
    };

    for sample in ordered {
        let mut extra = HashMap::new();
        extra.insert(vidx, sample.witness);

        if let Some(mut model) = solver.solve(&problem, &extra) {
            let optimum = match direction {
                Optimization::Minimize => sample.lower,
                Optimization::Maximize => sample.upper,
            };
            let is_unbounded = match direction {
                Optimization::Minimize => optimum == f64::NEG_INFINITY,
                Optimization::Maximize => optimum == f64::INFINITY,
            };

            let value = if is_unbounded { sample.witness } else { optimum };
            model.insert(variable.clone(), value);

            let explanation = if is_unbounded {
                format!(
                    "{variable:?} is unbounded in the {} direction; {} is a witness",
                    direction_name(&direction),
                    sample.witness
                )
            } else {
                let bound = match direction {
                    Optimization::Minimize => "lower bound",
                    Optimization::Maximize => "upper bound",
                };
                format!("{variable:?} {bound} = {value}")
            };

            return SolverResult { satisfiable: true, model, explanation: Some(explanation) };
        }
    }

    SolverResult {
        satisfiable: false,
        model: solver.pinned.clone(),
        explanation: Some(describe_unsatisfiable(constraints)),
    }
}

fn direction_name(direction: &Optimization) -> &'static str {
    match direction {
        Optimization::Minimize => "minimize",
        Optimization::Maximize => "maximize",
    }
}

fn describe_unsatisfiable(constraints: &[ConstraintExpr]) -> String {
    if constraints.is_empty() {
        "No constraints given".to_string()
    } else {
        format!("Constraint set is unsatisfiable ({} expressions)", constraints.len())
    }
}

/// Retained name for the in-process solver.
///
/// Earlier revisions described this as a "mock" even though it was the tier-1
/// solver behind every path verification. It is now exact, so the alias exists
/// only so existing call sites keep compiling.
pub type MockSolver = ExactSolver;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trivially_satisfiable() {
        let solver = ExactSolver::new();
        let result = solver.check(&[ConstraintExpr::always_true()]);
        assert!(result.satisfiable);
    }

    #[test]
    fn test_trivially_unsatisfiable() {
        let solver = ExactSolver::new();
        let result = solver.check(&[ConstraintExpr::always_false()]);
        assert!(!result.satisfiable);
    }

    #[test]
    fn test_capacity_constraint_satisfied() {
        let solver = ExactSolver::new().with_var(ConstraintVariable::Capacity, 500.0);
        let constraint = ConstraintExpr::capacity(ComparisonOp::Ge, 100.0);
        let result = solver.check(&[constraint]);
        assert!(result.satisfiable);
    }

    #[test]
    fn test_capacity_constraint_violated() {
        let solver = ExactSolver::new().with_var(ConstraintVariable::Capacity, 50.0);
        let constraint = ConstraintExpr::capacity(ComparisonOp::Ge, 100.0);
        let result = solver.check(&[constraint]);
        assert!(!result.satisfiable);
    }

    #[test]
    fn test_composite_constraint() {
        let solver = ExactSolver::new()
            .with_var(ConstraintVariable::Capacity, 200.0)
            .with_var(ConstraintVariable::Temperature, 5.0);

        let constraint = ConstraintExpr::And(
            Box::new(ConstraintExpr::capacity(ComparisonOp::Ge, 100.0)),
            Box::new(ConstraintExpr::Comparison {
                variable: ConstraintVariable::Temperature,
                op: ComparisonOp::Le,
                value: 8.0,
            }),
        );
        let result = solver.check(&[constraint]);
        assert!(result.satisfiable);
    }

    /// Regression: the previous solver gave every variable the same value, so
    /// two independent constraints were wrongly reported unsatisfiable.
    #[test]
    fn test_independent_variables_get_independent_values() {
        let solver = ExactSolver::new();
        let constraints = vec![
            ConstraintExpr::capacity(ComparisonOp::Ge, 100.0),
            ConstraintExpr::Comparison { variable: ConstraintVariable::Temperature, op: ComparisonOp::Le, value: 8.0 },
        ];

        let result = solver.check(&constraints);
        assert!(result.satisfiable, "independent constraints are jointly satisfiable");

        let capacity = result.model[&ConstraintVariable::Capacity];
        let temperature = result.model[&ConstraintVariable::Temperature];
        assert!(capacity >= 100.0, "capacity {capacity} must satisfy >= 100");
        assert!(temperature <= 8.0, "temperature {temperature} must satisfy <= 8");
    }

    /// Regression: contradictions on one variable must still be detected.
    #[test]
    fn test_contradiction_on_a_single_variable() {
        let solver = ExactSolver::new();
        let constraints =
            vec![ConstraintExpr::capacity(ComparisonOp::Ge, 500.0), ConstraintExpr::capacity(ComparisonOp::Le, 100.0)];
        assert!(!solver.check(&constraints).satisfiable);
    }

    #[test]
    fn test_or_is_handled() {
        let solver = ExactSolver::new();
        // capacity >= 500 OR temperature <= 8 is satisfiable: temperature
        // can be 8 even though capacity alone is not pinned favourably.
        let constraints = vec![ConstraintExpr::Or(
            Box::new(ConstraintExpr::capacity(ComparisonOp::Ge, 500.0)),
            Box::new(ConstraintExpr::Comparison {
                variable: ConstraintVariable::Temperature,
                op: ComparisonOp::Le,
                value: 8.0,
            }),
        )];
        assert!(solver.check(&constraints).satisfiable);
    }

    #[test]
    fn test_not_is_handled() {
        let solver = ExactSolver::new();
        let constraints = vec![ConstraintExpr::Not(Box::new(ConstraintExpr::Comparison {
            variable: ConstraintVariable::Temperature,
            op: ComparisonOp::Le,
            value: 8.0,
        }))];
        let result = solver.check(&constraints);
        assert!(result.satisfiable);
        assert!(result.model[&ConstraintVariable::Temperature] > 8.0);
    }

    #[test]
    fn test_eq_is_exact() {
        let solver = ExactSolver::new();
        let constraints = vec![ConstraintExpr::Comparison {
            variable: ConstraintVariable::Temperature,
            op: ComparisonOp::Eq,
            value: 4.0,
        }];
        let result = solver.check(&constraints);
        assert!(result.satisfiable);
        assert!((result.model[&ConstraintVariable::Temperature] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_minimize() {
        let solver = ExactSolver::new();
        let constraint =
            ConstraintExpr::Comparison { variable: ConstraintVariable::Temperature, op: ComparisonOp::Ge, value: 5.0 };
        let result = solver.minimize(&[constraint], &ConstraintVariable::Temperature);
        assert!(result.satisfiable);
        let temp = result.model.get(&ConstraintVariable::Temperature).unwrap();
        assert!(*temp >= 5.0);
    }

    /// The optimum is a real bound, not a binary-search approximation.
    #[test]
    fn test_minimize_returns_the_exact_bound() {
        let solver = ExactSolver::new();
        let constraint = ConstraintExpr::capacity(ComparisonOp::Ge, 250.0);
        let result = solver.minimize(&[constraint], &ConstraintVariable::Capacity);
        assert!(result.satisfiable);
        assert_eq!(result.model[&ConstraintVariable::Capacity], 250.0);
    }

    #[test]
    fn test_maximize_returns_the_exact_bound() {
        let solver = ExactSolver::new();
        let constraint = ConstraintExpr::capacity(ComparisonOp::Le, 900.0);
        let result = solver.maximize(&[constraint], &ConstraintVariable::Capacity);
        assert!(result.satisfiable);
        assert_eq!(result.model[&ConstraintVariable::Capacity], 900.0);
    }

    /// A strictly bounded-below variable has an infimum, not a minimum.
    #[test]
    fn test_minimize_strict_inequality_returns_infimum() {
        let solver = ExactSolver::new();
        let constraint =
            ConstraintExpr::Comparison { variable: ConstraintVariable::Temperature, op: ComparisonOp::Gt, value: 5.0 };
        let result = solver.minimize(&[constraint], &ConstraintVariable::Temperature);
        assert!(result.satisfiable);
        assert_eq!(result.model[&ConstraintVariable::Temperature], 5.0);
    }

    #[test]
    fn test_maximize() {
        let solver = ExactSolver::new();
        let constraint =
            ConstraintExpr::Comparison { variable: ConstraintVariable::Capacity, op: ComparisonOp::Le, value: 1000.0 };
        let result = solver.maximize(&[constraint], &ConstraintVariable::Capacity);
        assert!(result.satisfiable);
    }

    #[test]
    fn test_minimize_unsatisfiable_constraints() {
        let solver = ExactSolver::new();
        let constraints =
            vec![ConstraintExpr::capacity(ComparisonOp::Ge, 500.0), ConstraintExpr::capacity(ComparisonOp::Le, 100.0)];
        let result = solver.minimize(&constraints, &ConstraintVariable::Capacity);
        assert!(!result.satisfiable);
    }

    #[test]
    fn test_mock_solver_alias_still_works() {
        let solver = MockSolver::new().with_var(ConstraintVariable::Capacity, 500.0);
        assert!(solver.check(&[ConstraintExpr::capacity(ComparisonOp::Ge, 100.0)]).satisfiable);
    }
}

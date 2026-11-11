use crate::query::{CollisionQuery, SatModel, SolverResult};

pub trait SolverBackend {
    fn check(&self, query: &CollisionQuery) -> SolverResult;
    fn get_model(&self) -> Option<SatModel>;
}

pub struct GeometricSolver;

impl SolverBackend for GeometricSolver {
    fn check(&self, query: &CollisionQuery) -> SolverResult {
        if query.will_collide() {
            SolverResult::Sat
        } else {
            SolverResult::Unsat
        }
    }

    fn get_model(&self) -> Option<SatModel> {
        None
    }
}

pub struct SmtSolver {
    backend: Box<dyn SolverBackend>,
}

impl SmtSolver {
    /// Create a solver with the default geometric backend.
    pub fn new() -> Self {
        Self {
            backend: Box::new(GeometricSolver),
        }
    }

    /// Create a solver with a custom backend (e.g., Z3Backend).
    pub fn with_backend(backend: Box<dyn SolverBackend>) -> Self {
        Self { backend }
    }

    /// Create a solver using the Z3 SMT backend for formal verification.
    pub fn z3() -> Self {
        Self {
            backend: Box::new(crate::z3_backend::Z3Backend::new()),
        }
    }

    pub fn check_collision(&self, query: &CollisionQuery) -> SolverResult {
        self.backend.check(query)
    }
}

impl Default for SmtSolver {
    fn default() -> Self {
        Self::new()
    }
}
